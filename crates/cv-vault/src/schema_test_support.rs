//! Shared fixtures for the vault.toml refuser tests.

use cv_engine::SaveInput;
use serde_json::Value;

use crate::capture::write_entry;
use crate::taxonomy::VAULT_TOML_FILENAME;

/// A data vault shaped like a real first consumer, trimmed to what the tests need.
pub const DATA_VAULT: &str = r#"
schema_version = 1

[vault]
closed_kinds = true
personal_free = true

[default]
category = "structured"
tier = "durable"

[kinds.price-point]
[kinds.price-point.fields]
amount = { type = "number", required = true }
currency = { type = "string", required = true, enum = ["SEK", "EUR", "USD"] }
observed_on = { type = "date", required = true }
hours = { type = "integer" }

[kinds.gotcha]
"#;

pub fn vault(toml: &str) -> tempfile::TempDir {
    let dir = tempfile::TempDir::new().unwrap();
    std::fs::write(dir.path().join(VAULT_TOML_FILENAME), toml).unwrap();
    dir
}

pub fn input(kind: &str, meta: Value, tags: &[&str]) -> SaveInput {
    SaveInput {
        kind: kind.to_string(),
        title: Some("t".to_string()),
        body: "b".to_string(),
        meta: Some(meta),
        tags: Some(tags.iter().map(|t| t.to_string()).collect()),
        ..Default::default()
    }
}

pub fn err(dir: &tempfile::TempDir, input: &SaveInput) -> String {
    write_entry(dir.path().to_str().unwrap(), input)
        .expect_err("save should have been refused")
        .to_string()
}
