use super::*;
use tempfile::TempDir;

fn write(vault_dir: &std::path::Path, rel: &str, contents: &str) {
    let path = vault_dir.join(rel);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, contents).unwrap();
}

#[test]
fn moves_a_whole_directory_preserving_structure() {
    let temp = TempDir::new().unwrap();
    let vault_dir = temp.path().to_str().unwrap();
    write(temp.path(), "events/tool-calls/a.md", "a");
    write(temp.path(), "events/tool-calls/sub/b.md", "b");

    let summary = plan_move(vault_dir, "events/tool-calls", "events/event/tool-calls", false).unwrap();

    assert_eq!(summary.moved.len(), 2);
    assert!(temp.path().join("events/event/tool-calls/a.md").exists());
    assert!(temp.path().join("events/event/tool-calls/sub/b.md").exists());
    assert!(!temp.path().join("events/tool-calls").exists(), "empty source pruned");
}

#[test]
fn dry_run_moves_nothing() {
    let temp = TempDir::new().unwrap();
    let vault_dir = temp.path().to_str().unwrap();
    write(temp.path(), "knowledge/note/a.md", "a");

    let summary = plan_move(vault_dir, "knowledge/note", "knowledge/notes", true).unwrap();

    assert_eq!(summary.moved.len(), 1);
    assert!(temp.path().join("knowledge/note/a.md").exists());
    assert!(!temp.path().join("knowledge/notes").exists());
}

#[test]
fn moving_into_an_existing_directory_merges_flat_not_nested() {
    let temp = TempDir::new().unwrap();
    let vault_dir = temp.path().to_str().unwrap();
    write(temp.path(), "entities/sprint/p4.md", "p4");
    fs::create_dir_all(temp.path().join("archive")).unwrap();

    // dest names the target directly -- contents of "sprint" land in "archive",
    // not nested as "archive/sprint" the way `mv sprint archive` would nest it.
    let summary = plan_move(vault_dir, "entities/sprint", "archive", false).unwrap();

    assert_eq!(summary.moved.len(), 1);
    assert!(temp.path().join("archive/p4.md").exists());
    assert!(!temp.path().join("archive/sprint").exists());
}

#[test]
fn preserving_the_source_name_is_explicit() {
    let temp = TempDir::new().unwrap();
    let vault_dir = temp.path().to_str().unwrap();
    write(temp.path(), "entities/sprint/p4.md", "p4");

    // Want it filed as a named subfolder instead? Spell it out in dest.
    let summary = plan_move(vault_dir, "entities/sprint", "archive/sprint", false).unwrap();

    assert_eq!(summary.moved.len(), 1);
    assert!(temp.path().join("archive/sprint/p4.md").exists());
}

#[test]
fn never_overwrites_a_conflicting_destination() {
    let temp = TempDir::new().unwrap();
    let vault_dir = temp.path().to_str().unwrap();
    write(temp.path(), "a/one.md", "original-a");
    write(temp.path(), "b/one.md", "original-b");

    let summary = plan_move(vault_dir, "a/one.md", "b/one.md", false).unwrap();

    assert_eq!(summary.moved.len(), 0);
    assert_eq!(summary.skipped.len(), 1);
    assert_eq!(fs::read_to_string(temp.path().join("b/one.md")).unwrap(), "original-b");
    assert!(temp.path().join("a/one.md").exists(), "unmoved file stays put, no data loss");
}

#[test]
fn merging_directories_skips_only_the_colliding_files() {
    let temp = TempDir::new().unwrap();
    let vault_dir = temp.path().to_str().unwrap();
    write(temp.path(), "events/log/keep-me.md", "new");
    write(temp.path(), "events/log/one.md", "new-one");
    write(temp.path(), "events/archive/one.md", "existing-one");

    let summary = plan_move(vault_dir, "events/log", "events/archive", false).unwrap();

    assert_eq!(summary.moved.len(), 1);
    assert_eq!(summary.skipped.len(), 1);
    assert!(temp.path().join("events/archive/keep-me.md").exists());
    assert_eq!(
        fs::read_to_string(temp.path().join("events/archive/one.md")).unwrap(),
        "existing-one",
        "colliding file untouched"
    );
    assert!(temp.path().join("events/log/one.md").exists(), "unmoved file stays put");
}

#[test]
fn moves_a_single_file() {
    let temp = TempDir::new().unwrap();
    let vault_dir = temp.path().to_str().unwrap();
    write(temp.path(), "knowledge/insight/a.md", "a");

    let summary = plan_move(vault_dir, "knowledge/insight/a.md", "knowledge/insight/renamed.md", false).unwrap();

    assert_eq!(summary.moved.len(), 1);
    assert!(temp.path().join("knowledge/insight/renamed.md").exists());
    assert!(!temp.path().join("knowledge/insight/a.md").exists());
}

#[test]
fn rejects_path_traversal() {
    let temp = TempDir::new().unwrap();
    let vault_dir = temp.path().to_str().unwrap();
    write(temp.path(), "knowledge/insight/a.md", "a");

    assert!(plan_move(vault_dir, "../outside", "knowledge", false).is_err());
    assert!(plan_move(vault_dir, "knowledge/insight", "../../etc", false).is_err());
}
