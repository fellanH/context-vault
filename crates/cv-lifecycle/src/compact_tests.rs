use super::*;
use tempfile::TempDir;
use std::io::Write;

#[test]
fn test_archive_gz_path() {
    let path = archive_gz_path("/vault", "entry-123");
    assert!(path.to_string_lossy().contains("_archive"));
    assert!(path.to_string_lossy().contains("entry-123"));
    assert!(path.to_string_lossy().contains(".md.gz"));
}

#[test]
fn test_ensure_archive_dir() {
    let tmpdir = TempDir::new().unwrap();
    let vault_dir = tmpdir.path().to_string_lossy().to_string();
    ensure_archive_dir(&vault_dir).unwrap();
    assert!(Path::new(&vault_dir).join("_archive").exists());
}

#[test]
fn test_summarize_body_short() {
    let body = "Short body";
    let summary = summarize_body(body);
    assert_eq!(summary, body);
}

#[test]
fn test_summarize_body_long() {
    let body = "x".repeat(600);
    let summary = summarize_body(&body);
    assert!(summary.contains("[compacted:"));
    assert!(summary.len() < body.len());
}

#[test]
fn test_restore_compacted_body_not_found() {
    let tmpdir = TempDir::new().unwrap();
    let vault_dir = tmpdir.path().to_string_lossy().to_string();
    let result = restore_compacted_body(&vault_dir, "nonexistent");
    assert!(result.is_ok());
    assert!(result.unwrap().is_none());
}

#[test]
fn test_restore_compacted_body_success() {
    let tmpdir = TempDir::new().unwrap();
    let vault_dir = tmpdir.path().to_string_lossy().to_string();
    ensure_archive_dir(&vault_dir).unwrap();

    let original = "This is my archived content";
    let gz_path = archive_gz_path(&vault_dir, "test-123");

    // Manually compress and write
    let mut encoder =
        flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
    encoder.write_all(original.as_bytes()).unwrap();
    let compressed = encoder.finish().unwrap();
    fs::write(&gz_path, &compressed).unwrap();

    // Restore and verify
    let restored = restore_compacted_body(&vault_dir, "test-123").unwrap();
    assert_eq!(restored, Some(original.to_string()));
}
