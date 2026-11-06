#!/usr/bin/env python3
"""Generate a clearly synthetic, fixed-width COTAHIST-shaped benchmark file."""
import argparse
from datetime import date
import json
from pathlib import Path

RECORD_WIDTH = 245


def record(kind: bytes, fields: tuple[tuple[int, bytes], ...] = ()) -> bytes:
    row = bytearray(b" " * RECORD_WIDTH)
    row[: len(kind)] = kind
    for offset, value in fields:
        if offset + len(value) > RECORD_WIDTH:
            raise ValueError("Field exceeds the COTAHIST-shaped record width.")
        row[offset : offset + len(value)] = value
    return bytes(row)


def generate(output: Path, records: int, trade_date: date) -> int:
    if not 1 <= records <= 99_999_999_997:
        raise ValueError("Record count does not fit the benchmark trailer.")
    output.parent.mkdir(parents=True, exist_ok=True)
    iso_date = trade_date.strftime("%Y%m%d").encode("ascii")
    count_bytes = f"{records + 2:011d}".encode("ascii")
    total_bytes = 0
    with output.open("wb") as target:
        for row in (record(b"00"),):
            target.write(row + b"\n")
            total_bytes += RECORD_WIDTH + 1
        for number in range(records):
            ticker = f"S{number:011d}".encode("ascii")
            row = record(
                b"01",
                (
                    (2, iso_date),
                    (10, b"02"),
                    (12, ticker),
                    (24, b"010"),
                    (52, b"BRL"),
                    (56, b"0000000100000"),
                    (202, b"99991231"),
                    (210, b"0000001"),
                    (242, b"001"),
                ),
            )
            target.write(row + b"\n")
            total_bytes += RECORD_WIDTH + 1
        trailer = record(b"99", ((31, count_bytes),))
        target.write(trailer + b"\n")
        total_bytes += RECORD_WIDTH + 1
    return total_bytes


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--records", type=int, default=20_000)
    parser.add_argument("--trade-date", type=date.fromisoformat, default=date(2025, 1, 2))
    parser.add_argument("--output", type=Path, required=True)
    parser.add_argument("--command-file", type=Path)
    args = parser.parse_args()
    size = generate(args.output, args.records, args.trade_date)
    if args.command_file:
        args.command_file.parent.mkdir(parents=True, exist_ok=True)
        args.command_file.write_text(
            json.dumps(
                {
                    "operation": "ingest",
                    "source": "COTAHIST_YEARLY",
                    "from": "2025-01-01",
                    "to": "2025-12-31",
                }
            )
            + "\n",
            encoding="utf-8",
        )
    print(
        f"SYNTHETIC ONLY; records={args.records}; bytes={size}; "
        f"trade_date={args.trade_date.isoformat()}; path={args.output}"
    )


if __name__ == "__main__":
    main()
