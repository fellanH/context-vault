use super::handlers_schema::{describe, kind_note_for};
use super::handlers_write::handle_save_context;
use crate::backends::{NoOpEmbedder, SqliteBackend};
use crate::engine::VaultEngine;
use cv_vault::VaultTaxonomy;
use serde_json::json;
use std::sync::Arc;

const TOML: &str = r#"
schema_version = 1

[vault]
closed_kinds = true
personal_free = false

[default]
category = "knowledge"
tier = "working"
identity = false

[kinds.decision]
category = "knowledge"
tier = "durable"

[kinds.decision.fields]
owner = { type = "string", required = true }
status = { type = "string", enum = ["open", "made"] }
"#;

/// Card 30: an agent reads the kind's rules first, and a save built from them passes.
#[tokio::test]
async fn describe_then_first_save_succeeds() {
    let tmp = tempfile::tempdir().unwrap();
    let vault_dir = tmp.path().join("vault");
    std::fs::create_dir_all(&vault_dir).unwrap();
    std::fs::write(vault_dir.join("vault.toml"), TOML).unwrap();
    let taxonomy = VaultTaxonomy::load_strict(vault_dir.to_str().unwrap()).unwrap();

    let d = describe(&taxonomy, Some("decision"));
    assert_eq!(d["allowed"], true);
    assert_eq!(d["rule"]["fields"]["owner"]["required"], true);
    assert_eq!(d["rule"]["fields"]["status"]["enum"], json!(["open", "made"]));
    assert!(d["vault_rules"][0].as_str().unwrap().starts_with("closed_kinds"));

    let unknown = describe(&taxonomy, Some("musing"));
    assert_eq!(unknown["allowed"], false);
    assert_eq!(unknown["kinds"], json!(["decision"]));
    assert!(describe(&taxonomy, None)["kinds"]["decision"].is_object());

    let e = SqliteBackend::new(tmp.path().join("vault.db").to_str().unwrap(), vault_dir.to_str().unwrap(), Arc::new(NoOpEmbedder)).unwrap();
    e.init().await.unwrap();
    let engine: Arc<dyn VaultEngine> = Arc::new(e);
    let refused = handle_save_context(engine.clone(), &json!({"kind": "decision", "title": "Ship", "body": "ship it"})).await;
    assert!(refused.is_err(), "missing required owner is refused");
    handle_save_context(engine, &json!({"kind": "decision", "title": "Ship", "body": "ship it", "meta": {"owner": "seat", "status": "made"}}))
        .await
        .expect("a save built from the described rule passes");
}

/// OPS-405: an open vault that lists kinds tells the writer a kind is new, with the nearest listed one;
/// a listed kind, a vault listing no kinds and a closed vault (which refuses instead) say nothing.
#[test]
fn a_new_kind_on_an_open_vault_is_named_with_its_nearest() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().to_str().unwrap();
    std::fs::write(tmp.path().join("vault.toml"), TOML.replace("closed_kinds = true", "closed_kinds = false")).unwrap();
    let open = VaultTaxonomy::load_strict(dir).unwrap();
    let note = kind_note_for(&open, "decisoin");
    assert!(note.contains("kind `decisoin` is new") && note.contains("Nearest listed kind: `decision`"), "{note}");
    assert!(kind_note_for(&open, "bankgirot").contains("is new") && !kind_note_for(&open, "bankgirot").contains("Nearest"));
    assert_eq!(kind_note_for(&open, "decision"), "");
    std::fs::write(tmp.path().join("vault.toml"), TOML).unwrap();
    assert_eq!(kind_note_for(&VaultTaxonomy::load_strict(dir).unwrap(), "decisoin"), "");
    assert_eq!(kind_note_for(&VaultTaxonomy::minimal(), "anything"), "");
}
