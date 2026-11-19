use super::*;

/// OPS-400: a real user's registry, written by the TS CLI on 2 July 2026, made 4.0.0's
/// `vault list` fail with "missing field `vault_dir`". Both shapes must keep reading.
#[test]
fn test_reads_both_registry_shapes_and_writes_camel_case() {
    let camel = r#"{
  "activeVault": "default",
  "vaults": {
    "default": {
      "vaultDir": "/Users/x/omni/vault",
      "dataDir": "/Users/x/.context-mcp",
      "createdAt": "2026-07-02T08:11:56.050Z"
    }
  }
}"#;
    let snake = r#"{"active_vault":"b","vaults":{"b":{"vault_dir":"/v","data_dir":"/d","created_at":"x"}}}"#;

    let a: VaultRegistry = serde_json::from_str(camel).unwrap();
    assert_eq!(a.active_vault, "default");
    assert_eq!(a.vaults["default"].vault_dir, "/Users/x/omni/vault");
    let b: VaultRegistry = serde_json::from_str(snake).unwrap();
    assert_eq!(b.vaults["b"].data_dir, "/d");

    let written = serde_json::to_string(&b).unwrap();
    assert!(written.contains("\"activeVault\"") && written.contains("\"vaultDir\""), "{written}");
}
