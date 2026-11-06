#!/usr/bin/env python3
"""Consistent PostgreSQL/raw snapshots using the Compose PostgreSQL client tools.

No network database credentials are accepted or printed. Run from the repository root;
the PostgreSQL container's local administrative connection performs backup/restore.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import shutil
import subprocess
import sys


REFERENCE_SQL = """SELECT jsonb_build_object('fact',to_jsonb(f),'version',to_jsonb(v),
 'instrument_id',q.instrument_id,'resolution_status',q.resolution_status)::text
 FROM b3_history_revision_quotes q JOIN b3_cotahist_facts f ON f.id=q.fact_id
 JOIN b3_cotahist_versions v ON v.id=q.version_id
 WHERE q.revision_id=(SELECT revision_id FROM b3_history_active WHERE singleton)
 ORDER BY f.id"""
RAW_SQL = """SELECT COALESCE(jsonb_agg(to_jsonb(r)), '[]'::jsonb)::text FROM (
 SELECT DISTINCT local_path,sha256,byte_size FROM b3_source_files WHERE local_path IS NOT NULL
 UNION SELECT DISTINCT local_path,sha256,byte_size FROM b3_source_file_members
 WHERE local_path IS NOT NULL AND local_path<>''
 ) r"""


def checked_name(value):
    if not re.fullmatch(r"[a-z][a-z0-9_]{2,62}", value):
        raise ValueError("Identificador de banco/serviço inválido.")
    return value


def docker(args, command):
    return ["docker", "compose", "--project-directory", str(Path(__file__).resolve().parents[1]),
            "exec", "-T", args.service, *command]


def psql(args, database, sql):
    command = docker(args, ["psql", "-XqAt", "-v", "ON_ERROR_STOP=1", "-U", args.user,
                            "-d", database, "-c", sql])
    result = subprocess.run(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=False)
    if result.returncode:
        raise RuntimeError("Operação PostgreSQL falhou; detalhes sensíveis não são exibidos.")
    return result.stdout.decode().strip()


def snapshot_sql(snapshot, sql):
    if not re.fullmatch(r"[0-9A-Fa-f-]+", snapshot):
        raise ValueError("Snapshot PostgreSQL inválido.")
    return ("BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY; "
            f"SET TRANSACTION SNAPSHOT '{snapshot}'; SET TIME ZONE 'UTC'; {sql}; COMMIT;")


def digest(path):
    sha = hashlib.sha256()
    size = 0
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(64 * 1024), b""):
            sha.update(chunk)
            size += len(chunk)
    return sha.hexdigest(), size


def reference(args, database, snapshot=None):
    sql = snapshot_sql(snapshot, REFERENCE_SQL) if snapshot else "SET TIME ZONE 'UTC'; " + REFERENCE_SQL
    process = subprocess.Popen(docker(args, ["psql", "-XqAt", "-v", "ON_ERROR_STOP=1", "-U", args.user,
                                              "-d", database, "-c", sql]),
                               stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    sha = hashlib.sha256()
    records = 0
    for line in process.stdout:
        sha.update(line)
        records += 1
    if process.wait():
        raise RuntimeError("Falha ao verificar revisão de referência.")
    return {"sha256": sha.hexdigest(), "records": records}


def backup(args):
    database = checked_name(args.database)
    root = args.raw_root.resolve(strict=True)
    destination = args.destination.resolve()
    if destination == root or root in destination.parents:
        raise ValueError("O backup deve ficar fora do diretório raw.")
    destination.mkdir(mode=0o700, parents=True, exist_ok=False)
    incomplete = destination / "INCOMPLETE"
    incomplete.write_text("Snapshot em construção; não restaurar.\n")
    keeper = subprocess.Popen(docker(args, ["psql", "-XqAt", "-v", "ON_ERROR_STOP=1", "-U", args.user,
                                            "-d", database]),
                              stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL)
    try:
        keeper.stdin.write(b"BEGIN ISOLATION LEVEL REPEATABLE READ READ ONLY; SELECT pg_export_snapshot();\n")
        keeper.stdin.flush()
        snapshot = keeper.stdout.readline().decode().strip()
        if not re.fullmatch(r"[0-9A-Fa-f-]+", snapshot):
            raise RuntimeError("Não foi possível exportar snapshot PostgreSQL.")
        rows = json.loads(psql(args, database, snapshot_sql(snapshot, RAW_SQL)))
        active = psql(args, database, snapshot_sql(snapshot,
                      "SELECT COALESCE(revision_id::text,'') FROM b3_history_active WHERE singleton"))
        expected = reference(args, database, snapshot)
        dump = destination / "database.dump"
        with dump.open("xb") as output:
            result = subprocess.run(docker(args, ["pg_dump", "-U", args.user, "-d", database,
                                                  "--format=custom", "--no-owner", "--no-acl",
                                                  "--snapshot", snapshot]),
                                    stdout=output, stderr=subprocess.DEVNULL, check=False)
            output.flush()
            os.fsync(output.fileno())
        if result.returncode:
            raise RuntimeError("Falha no pg_dump consistente.")
        files = []
        for row in sorted(rows, key=lambda item: item["local_path"]):
            path = Path(row["local_path"]).resolve(strict=True)
            if root not in path.parents or not path.is_file():
                raise ValueError("Banco referencia raw fora do diretório permitido.")
            sha, size = digest(path)
            if sha != row["sha256"] or size != row["byte_size"]:
                raise ValueError("Integridade raw diverge do manifesto persistido.")
            relative = path.relative_to(root)
            target = destination / "raw" / relative
            target.parent.mkdir(parents=True, exist_ok=True)
            if not target.exists():
                shutil.copyfile(path, target)
                with target.open("rb") as copied:
                    os.fsync(copied.fileno())
            if digest(target) != (sha, size):
                raise ValueError("Cópia raw não reproduz o hash de referência.")
            files.append({"original_path": row["local_path"], "relative_path": str(relative),
                          "sha256": sha, "byte_size": size})
        manifest = {"format": 1, "created_at": datetime.datetime.now(datetime.timezone.utc).isoformat(),
                    "database": database, "postgres_version": psql(args, database, "SHOW server_version"),
                    "active_revision": active or None, "reference": expected,
                    "dump_sha256": digest(dump)[0], "raw_files": files}
        with (destination / "manifest.json.part").open("x") as output:
            output.write(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
            output.flush()
            os.fsync(output.fileno())
        (destination / "manifest.json.part").rename(destination / "manifest.json")
        incomplete.unlink()
        print(json.dumps({"backup": str(destination), "active_revision": manifest["active_revision"],
                          "raw_files": len(files), "reference": expected}, ensure_ascii=False))
    finally:
        if keeper.poll() is None:
            try:
                keeper.stdin.write(b"ROLLBACK;\n\\q\n")
                keeper.stdin.flush()
            except BrokenPipeError:
                pass
            keeper.stdin.close()
            try:
                keeper.wait(timeout=10)
            except subprocess.TimeoutExpired:
                keeper.terminate()
                keeper.wait(timeout=10)


def restore(args):
    database = checked_name(args.database)
    source = args.backup.resolve(strict=True)
    if (source / "INCOMPLETE").exists():
        raise ValueError("Backup incompleto não pode ser restaurado.")
    manifest = json.loads((source / "manifest.json").read_text())
    if manifest.get("format") != 1:
        raise ValueError("Formato de manifesto não suportado.")
    dump = source / "database.dump"
    if digest(dump)[0] != manifest["dump_sha256"]:
        raise ValueError("Checksum do dump inválido.")
    root = args.raw_root.resolve()
    if root.exists() and any(root.iterdir()):
        raise ValueError("O destino raw deve estar vazio ou ausente.")
    exists = psql(args, "postgres", f"SELECT EXISTS(SELECT 1 FROM pg_database WHERE datname='{database}')")
    if exists != "f":
        raise ValueError("Restore exige nome de banco novo; nenhum banco existente é alterado.")
    paths = []
    for item in manifest["raw_files"]:
        relative = Path(item["relative_path"])
        if relative.is_absolute() or not relative.parts or any(p in ("..", ".") for p in relative.parts):
            raise ValueError("Localizador raw inválido no manifesto.")
        raw = (source / "raw" / relative).resolve(strict=True)
        if (source / "raw").resolve() not in raw.parents or digest(raw) != (item["sha256"], item["byte_size"]):
            raise ValueError("Integridade raw do backup inválida.")
        paths.append((item, relative, raw))
    root.mkdir(mode=0o700, parents=True, exist_ok=True)
    for item, relative, raw in paths:
        target = root / relative
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copyfile(raw, target)
    psql(args, "postgres", f"CREATE DATABASE {database}")
    with dump.open("rb") as input_file:
        result = subprocess.run(docker(args, ["pg_restore", "-U", args.user, "-d", database,
                                              "--no-owner", "--no-acl", "--exit-on-error"]),
                                stdin=input_file, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
                                check=False)
    if result.returncode:
        raise RuntimeError("Restore falhou; banco novo foi preservado para diagnóstico.")
    mapping = json.dumps([{"old": item["original_path"], "new": str(root / relative)}
                          for item, relative, _ in paths]).replace("'", "''")
    psql(args, database, "BEGIN; " + "; ".join(
        f"UPDATE {table} t SET local_path=m.new FROM jsonb_to_recordset('{mapping}'::jsonb) AS m(old text,new text) WHERE t.local_path=m.old"
        for table in ("b3_source_files", "b3_source_file_members")) + "; COMMIT;")
    active = psql(args, database, "SELECT COALESCE(revision_id::text,'') FROM b3_history_active WHERE singleton")
    actual = reference(args, database)
    if (active or None) != manifest["active_revision"] or actual != manifest["reference"]:
        raise RuntimeError("Restore não reproduz a revisão de referência.")
    print(json.dumps({"database": database, "active_revision": active or None,
                      "reference": actual, "verified": True}, ensure_ascii=False))


def main():
    parser = argparse.ArgumentParser(description="Backup/restore consistente de SQL + raw imutável.")
    parser.add_argument("--service", default="b3_postgres")
    parser.add_argument("--user", default="postgres")
    subcommands = parser.add_subparsers(dest="operation", required=True)
    save = subcommands.add_parser("backup")
    save.add_argument("--database", required=True)
    save.add_argument("--raw-root", type=Path, required=True)
    save.add_argument("--destination", type=Path, required=True)
    load = subcommands.add_parser("restore")
    load.add_argument("--database", required=True)
    load.add_argument("--raw-root", type=Path, required=True)
    load.add_argument("--backup", type=Path, required=True)
    args = parser.parse_args()
    checked_name(args.service)
    checked_name(args.user)
    try:
        (backup if args.operation == "backup" else restore)(args)
    except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
        print(str(error), file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
