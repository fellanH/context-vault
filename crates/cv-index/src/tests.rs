use super::*;
use crate::UpdateOptions;
use std::time::Duration;

use crate::fixtures::{fixture, markdown, update};
#[test]
fn incremental_edits_and_deletes_survive_reopen_without_duplicates() {
    let (temp, root, index) = fixture(VaultType::Knowledge);
    let path = markdown(&root, "one", "webhook retry", "{}");
    assert_eq!(update(&index).indexed, 1);
    assert_eq!(update(&index).indexed, 0);
    assert_eq!(
        index
            .search("webhook", &SearchOptions::default())
            .unwrap()
            .len(),
        1
    );
    markdown(&root, "one", "portal cancellation", "{}");
    assert_eq!(update(&index).indexed, 1);
    assert_eq!(index.count().unwrap(), 1);
    assert!(index
        .search("webhook", &SearchOptions::default())
        .unwrap()
        .is_empty());
    let reopened =
        VaultIndex::open(&root, VaultType::Knowledge, &temp.path().join("index")).unwrap();
    assert_eq!(update(&reopened).indexed, 0);
    assert_eq!(
        reopened
            .search("cancellation", &SearchOptions::default())
            .unwrap()
            .len(),
        1
    );
    std::fs::remove_file(path).unwrap();
    assert_eq!(update(&reopened).removed, 1);
    assert_eq!(reopened.count().unwrap(), 0);
}
#[test]
fn exact_meta_filters_distinguish_strings_numbers_and_query_syntax() {
    let (_temp, root, index) = fixture(VaultType::Knowledge);
    markdown(
        &root,
        "one",
        "alpha beta",
        r#"{"amount": 9, "system": "A OR B"}"#,
    );
    markdown(
        &root,
        "two",
        "alpha gamma",
        r#"{"amount": "9", "system": "other"}"#,
    );
    update(&index);
    let mut opts = SearchOptions::default();
    opts.meta.insert("amount".into(), serde_json::json!(9));
    let hits = index.search("alpha", &opts).unwrap();
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].record.id, "one");
    opts.meta
        .insert("system".into(), serde_json::json!("A OR B"));
    assert_eq!(index.search("alpha", &opts).unwrap().len(), 1);
    opts.tag = Some("absent".into());
    assert!(index.search("alpha", &opts).unwrap().is_empty());
}
#[test]
fn phrases_and_created_filters_are_indexed() {
    let (_temp, root, index) = fixture(VaultType::Knowledge);
    markdown(&root, "one", "alpha beta gamma", "{}");
    markdown(&root, "two", "beta unrelated alpha", "{}");
    update(&index);
    assert_eq!(
        index
            .search("\"alpha beta\"", &SearchOptions::default())
            .unwrap()
            .len(),
        1
    );
    let opts = SearchOptions {
        since: Some("2026-10-13T00:00:00Z".parse().unwrap()),
        ..Default::default()
    };
    assert!(index.search("alpha", &opts).unwrap().is_empty());
}
#[test]
fn busy_writer_keeps_readers_available_and_retries_after_unlock() {
    let (_temp, root, index) = fixture(VaultType::Knowledge);
    markdown(&root, "one", "alpha", "{}");
    update(&index);
    let held: tantivy::IndexWriter = index.index.writer_with_num_threads(1, 20_000_000).unwrap();
    markdown(&root, "two", "beta", "{}");
    assert!(index.update(&UpdateOptions::default()).unwrap().busy);
    assert_eq!(
        index
            .search("alpha", &SearchOptions::default())
            .unwrap()
            .len(),
        1
    );
    drop(held);
    assert_eq!(update(&index).indexed, 1);
    assert_eq!(index.count().unwrap(), 2);
}
#[test]
fn aborted_update_never_advances_checkpoint_or_publishes_partial_records() {
    let (_temp, root, index) = fixture(VaultType::Knowledge);
    markdown(&root, "a", "alpha", "{}");
    let dup = markdown(&root, "b", "beta", "{}");
    std::fs::copy(dup, root.join("z.md")).unwrap();
    assert!(index
        .update(&UpdateOptions {
            budget: None,
            ..Default::default()
        })
        .is_err());
    assert_eq!(index.count().unwrap(), 0);
    std::fs::remove_file(root.join("z.md")).unwrap();
    assert_eq!(update(&index).indexed, 2);
}
#[test]
fn zero_budget_never_deletes_unvisited_files() {
    let (_temp, root, index) = fixture(VaultType::Knowledge);
    markdown(&root, "one", "alpha", "{}");
    update(&index);
    markdown(&root, "one", "beta", "{}");
    let report = index
        .update(&UpdateOptions {
            budget: Some(Duration::ZERO),
            ..Default::default()
        })
        .unwrap();
    assert!(report.budget_reached);
    assert_eq!(index.count().unwrap(), 1);
    assert_eq!(update(&index).indexed, 1);
}
#[test]
fn segment_checkpoint_survives_sealing_and_appends_without_duplicate_hits() {
    let (_temp, root, index) = fixture(VaultType::Data);
    let mut rec = Record::new("chat-message", "alpha first");
    rec.created = "2026-10-10T08:00:00Z".parse().unwrap();
    cv_core::append(&root, "chat", &rec).unwrap();
    let next = Record::new("chat-message", "alpha second");
    cv_core::append(&root, "chat", &next).unwrap();
    assert_eq!(update(&index).indexed, 2);
    cv_core::seal_before(&root, "2026-10-11".parse().unwrap()).unwrap();
    assert_eq!(update(&index).indexed, 0);
    let opts = SearchOptions {
        stream: Some("chat".into()),
        ..Default::default()
    };
    assert_eq!(index.search("alpha", &opts).unwrap().len(), 2);
    cv_core::append(&root, "chat", &Record::new("chat-message", "alpha third")).unwrap();
    assert_eq!(update(&index).indexed, 1);
    assert_eq!(index.count().unwrap(), 3);
}
#[test]
fn index_cannot_be_reused_for_another_vault_or_nested_in_source() {
    let (temp, root, index) = fixture(VaultType::Knowledge);
    markdown(&root, "one", "alpha", "{}");
    update(&index);
    let other = temp.path().join("other");
    std::fs::create_dir(&other).unwrap();
    assert!(VaultIndex::open(&other, VaultType::Knowledge, &temp.path().join("index")).is_err());
    assert!(VaultIndex::open(&root, VaultType::Data, &temp.path().join("index")).is_err());
    assert!(VaultIndex::open(&root, VaultType::Knowledge, &root.join("index")).is_err());
}

#[test]
fn duplicate_ids_are_refused_instead_of_silently_losing_a_record() {
    let (_temp, root, index) = fixture(VaultType::Knowledge);
    let first = markdown(&root, "same", "first", "{}");
    std::fs::copy(first, root.join("other.md")).unwrap();
    let error = index
        .update(&UpdateOptions {
            budget: None,
            ..Default::default()
        })
        .unwrap_err();
    assert!(error.to_string().contains("duplicate id"));
    assert_eq!(index.count().unwrap(), 0);
}

#[test]
fn a_file_without_an_id_is_skipped_and_named_not_fatal() {
    let (_temp, root, index) = fixture(VaultType::Knowledge);
    markdown(&root, "kept", "still searchable", "{}");
    std::fs::write(
        root.join("hand-written.md"),
        "---\ntitle: No id here\n---\nbody\n",
    )
    .unwrap();
    let report = update(&index);
    assert_eq!(index.count().unwrap(), 1);
    assert_eq!(report.invalid_records, 1);
    assert!(report.invalid_examples[0].starts_with("hand-written.md: "));
    let again = update(&index);
    assert_eq!(
        again.invalid_records, 1,
        "an unchanged invalid file is still counted"
    );
    assert!(again.invalid_examples.is_empty(), "but not re-read");
    std::fs::write(
        root.join("hand-written.md"),
        "---\nid: fixed\nkind: gotcha\ncreated: 2026-10-12T08:00:00Z\ntitle: Fixed\n---\nbody\n",
    )
    .unwrap();
    assert_eq!(update(&index).invalid_records, 0);
    assert_eq!(index.count().unwrap(), 2);
}
