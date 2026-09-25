use super::*;

#[test]
fn test_load_registry_missing_file() {
    let temp = tempfile::TempDir::new().unwrap();
    let temp_path = temp.path();
    let registry_file = temp_path.join("vaults.json");

    if registry_file.exists() {
        std::fs::remove_file(&registry_file).unwrap();
    }

    let registry_content = r#"{"active_vault":"default","vaults":{}}"#;
    std::fs::write(&registry_file, registry_content).unwrap();

    let loaded: VaultRegistry = serde_json::from_str(registry_content).unwrap();
    assert_eq!(loaded.active_vault, "default");
    assert_eq!(loaded.vaults.len(), 0);
}

#[test]
fn test_create_vault() {
    let mut registry = VaultRegistry {
        active_vault: "default".to_string(),
        vaults: BTreeMap::new(),
    };

    create_vault(
        &mut registry,
        "test".to_string(),
        "/tmp/vault".to_string(),
        "/tmp/data".to_string(),
    )
    .unwrap();

    assert!(registry.vaults.contains_key("test"));
    assert_eq!(registry.vaults["test"].vault_dir, "/tmp/vault");
    assert_eq!(registry.vaults["test"].data_dir, "/tmp/data");
}

#[test]
fn test_create_vault_duplicate() {
    let mut registry = VaultRegistry {
        active_vault: "default".to_string(),
        vaults: BTreeMap::new(),
    };

    create_vault(
        &mut registry,
        "test".to_string(),
        "/tmp/vault".to_string(),
        "/tmp/data".to_string(),
    )
    .unwrap();

    let result = create_vault(
        &mut registry,
        "test".to_string(),
        "/tmp/vault2".to_string(),
        "/tmp/data2".to_string(),
    );

    assert!(result.is_err());
}

#[test]
fn test_use_vault() {
    let mut registry = VaultRegistry {
        active_vault: "default".to_string(),
        vaults: BTreeMap::new(),
    };

    let entry = VaultEntry {
        vault_dir: "/tmp/vault".to_string(),
        data_dir: "/tmp/data".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    registry.vaults.insert("test".to_string(), entry);
    use_vault(&mut registry, "test".to_string()).unwrap();
    assert_eq!(registry.active_vault, "test");
}

#[test]
fn test_use_vault_nonexistent() {
    let mut registry = VaultRegistry {
        active_vault: "default".to_string(),
        vaults: BTreeMap::new(),
    };

    let result = use_vault(&mut registry, "nonexistent".to_string());
    assert!(result.is_err());
}

#[test]
fn test_remove_vault() {
    let mut registry = VaultRegistry {
        active_vault: "test".to_string(),
        vaults: BTreeMap::new(),
    };

    let entry = VaultEntry {
        vault_dir: "/tmp/vault".to_string(),
        data_dir: "/tmp/data".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    registry.vaults.insert("test".to_string(), entry);
    registry.vaults.insert("default".to_string(), VaultEntry {
        vault_dir: "/tmp/vault2".to_string(),
        data_dir: "/tmp/data2".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    });

    remove_vault(&mut registry, "test".to_string()).unwrap();
    assert!(!registry.vaults.contains_key("test"));
    assert_eq!(registry.active_vault, "default");
}

#[test]
fn test_remove_vault_nonexistent() {
    let mut registry = VaultRegistry {
        active_vault: "default".to_string(),
        vaults: BTreeMap::new(),
    };

    let result = remove_vault(&mut registry, "nonexistent".to_string());
    assert!(result.is_err());
}

#[test]
fn test_list_vaults() {
    let mut registry = VaultRegistry {
        active_vault: "default".to_string(),
        vaults: BTreeMap::new(),
    };

    let entry1 = VaultEntry {
        vault_dir: "/tmp/vault1".to_string(),
        data_dir: "/tmp/data1".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let entry2 = VaultEntry {
        vault_dir: "/tmp/vault2".to_string(),
        data_dir: "/tmp/data2".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    registry.vaults.insert("vault1".to_string(), entry1);
    registry.vaults.insert("vault2".to_string(), entry2);

    let vaults = list_vaults(&registry);
    assert_eq!(vaults.len(), 2);
}

#[test]
fn test_resolve_active_vault_from_registry() {
    let entry = VaultEntry {
        vault_dir: "/tmp/vault".to_string(),
        data_dir: "/tmp/data".to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    let mut registry = VaultRegistry {
        active_vault: "test".to_string(),
        vaults: BTreeMap::new(),
    };

    registry.vaults.insert("test".to_string(), entry);

    let resolved = resolve_active_vault(&registry).unwrap();
    assert_eq!(resolved.vault_dir, "/tmp/vault");
    assert_eq!(resolved.data_dir, "/tmp/data");
}

#[test]
fn test_save_load_registry_roundtrip() {
    let temp = tempfile::TempDir::new().unwrap();
    let temp_path = temp.path().to_str().unwrap();

    let mut registry = VaultRegistry {
        active_vault: "test".to_string(),
        vaults: BTreeMap::new(),
    };

    let entry = VaultEntry {
        vault_dir: "/tmp/vault".to_string(),
        data_dir: "/tmp/data".to_string(),
        created_at: "2024-01-01T00:00:00Z".to_string(),
    };

    registry.vaults.insert("test".to_string(), entry);

    let registry_file = Path::new(temp_path).join("vaults.json");
    let content = serde_json::to_string_pretty(&registry).unwrap();
    std::fs::create_dir_all(temp_path).unwrap();
    std::fs::write(&registry_file, content).unwrap();

    let loaded: VaultRegistry = serde_json::from_str(
        &std::fs::read_to_string(&registry_file).unwrap()
    ).unwrap();

    assert_eq!(loaded.active_vault, "test");
    assert!(loaded.vaults.contains_key("test"));
}
