use super::*;

#[test]
fn test_archive_dir_path() {
    let path = archive_dir("/vault");
    assert!(path.to_string_lossy().contains("_archive"));
}

#[test]
fn test_valid_tiers_and_categories() {
    assert!(VALID_TIERS.contains(&"ephemeral"));
    assert!(VALID_TIERS.contains(&"working"));
    assert!(VALID_TIERS.contains(&"durable"));
    assert!(VALID_CATEGORIES.contains(&"knowledge"));
}

#[test]
fn test_count_archived_entries_empty() {
    let tmpdir = tempfile::TempDir::new().unwrap();
    let vault_dir = tmpdir.path().to_string_lossy().to_string();
    let count = count_archived_entries(&vault_dir);
    assert_eq!(count, 0);
}
