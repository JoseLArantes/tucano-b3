use std::{
    fs,
    io::{Cursor, Write},
    path::Path,
};
use tucano_b3::services::artifacts::{ArtifactLimits, ArtifactService};
use zip::write::SimpleFileOptions;

fn archive(files: &[(&str, Vec<u8>)]) -> Vec<u8> {
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    for (name, bytes) in files {
        zip.start_file(*name, SimpleFileOptions::default()).unwrap();
        zip.write_all(bytes).unwrap();
    }
    zip.finish().unwrap().into_inner()
}
fn setup(limits: ArtifactLimits) -> (tempfile::TempDir, ArtifactService) {
    let dir = tempfile::tempdir().unwrap();
    let service =
        ArtifactService::new(dir.path().into(), vec!["www.b3.com.br".into()], limits).unwrap();
    (dir, service)
}

#[tokio::test]
async fn nested_inventory_and_content_reuse_are_deterministic() {
    let (dir, service) = setup(ArtifactLimits::default());
    let nested = archive(&[("prices.txt", b"official example".to_vec())]);
    fs::write(
        dir.path().join("imports/input.zip"),
        archive(&[
            ("a.zip", nested.clone()),
            ("b.zip", nested),
            ("notes.bin", b"ignored".to_vec()),
            ("notes.html", b"<html>readme</html>".to_vec()),
            ("empty.bin", Vec::new()),
        ]),
    )
    .unwrap();
    let raw = service.import_local(Path::new("input.zip")).await.unwrap();
    let again = service.import_local(Path::new("input.zip")).await.unwrap();
    assert_eq!(raw.sha256, again.sha256);
    assert_eq!(raw.path, again.path);
    let members = service.inventory(&raw).unwrap();
    assert_eq!(members.len(), 7);
    assert_eq!(members[1].name, "a.zip!/prices.txt");
    assert_eq!(members[3].name, "b.zip!/prices.txt");
    assert_eq!(members[1].parent_index, Some(0));
    assert_eq!(members[3].parent_index, Some(2));
    assert_eq!(members[1].path, members[3].path);
    assert!(members[1].selected);
    assert!(!members[4].selected);
    assert!(!members[5].selected);
    assert_eq!(members[6].byte_size, 0);
    assert_eq!(
        members.iter().map(|m| m.name.clone()).collect::<Vec<_>>(),
        service
            .inventory(&raw)
            .unwrap()
            .iter()
            .map(|m| m.name.clone())
            .collect::<Vec<_>>()
    );
}

#[tokio::test]
async fn traversal_depth_bytes_and_raw_tampering_fail_visibly() {
    let limits = ArtifactLimits {
        max_depth: 0,
        ..ArtifactLimits::default()
    };
    let (dir, service) = setup(limits);
    for (name, bytes) in [
        ("traversal.zip", archive(&[("../bad.txt", b"no".to_vec())])),
        (
            "ambiguous.zip",
            archive(&[
                ("a.zip", archive(&[("b.txt", b"nested".to_vec())])),
                ("a.zip!/b.txt", b"literal".to_vec()),
            ]),
        ),
        (
            "deep.zip",
            archive(&[
                ("good.txt", b"valid".to_vec()),
                (
                    "nested.zip",
                    archive(&[("inner.txt", b"too deep".to_vec())]),
                ),
            ]),
        ),
    ] {
        fs::write(dir.path().join("imports").join(name), bytes).unwrap();
        let raw = service.import_local(Path::new(name)).await.unwrap();
        assert!(service.inventory(&raw).is_err());
    }
    assert!(service.import_local(Path::new("../outside")).await.is_err());
    fs::write(dir.path().join("imports/plain.txt"), b"raw").unwrap();
    let raw = service.import_local(Path::new("plain.txt")).await.unwrap();
    fs::write(&raw.path, b"tampered").unwrap();
    assert!(service.inventory(&raw).is_err());
    let (small_dir, small) = setup(ArtifactLimits {
        download_bytes: 2,
        ..ArtifactLimits::default()
    });
    fs::write(
        small_dir.path().join("imports/plain.txt"),
        b"larger than limit",
    )
    .unwrap();
    assert!(small
        .import_local(Path::new("plain.txt"))
        .await
        .unwrap_err()
        .to_string()
        .contains("limite"));
    assert_eq!(
        fs::read_dir(small_dir.path().join("tmp")).unwrap().count(),
        0
    );
    let (inventory_dir, inventory_limited) = setup(ArtifactLimits {
        max_inventory_bytes: 1,
        ..ArtifactLimits::default()
    });
    fs::write(
        inventory_dir.path().join("imports/inventory.zip"),
        archive(&[("one.txt", b"bounded metadata".to_vec())]),
    )
    .unwrap();
    let inventory_raw = inventory_limited
        .import_local(Path::new("inventory.zip"))
        .await
        .unwrap();
    assert!(inventory_limited.inventory(&inventory_raw).is_err());
    fs::remove_file(&raw.path).unwrap();
    assert!(service.inventory(&raw).is_err());
}

#[tokio::test]
async fn unsafe_remote_urls_are_rejected_before_transport() {
    let (_, service) = setup(ArtifactLimits::default());
    for url in [
        "http://www.b3.com.br/data",
        "https://www.b3.com.br:444/data",
        "https://bad.example/data",
        "https://user:secret@www.b3.com.br/data",
        "https://127.0.0.1/data",
    ] {
        assert!(service.download(url).await.is_err());
        assert!(service.probe(url).await.is_err());
    }
    assert!(!service.two_step_supported());
}
