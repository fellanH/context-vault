use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use anyhow::Result;

// vaults.json is camelCase (written by the JS side); snake_case is still read for
// files an older Rust build wrote.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultEntry {
    #[serde(alias = "vault_dir")]
    pub vault_dir: String,
    #[serde(alias = "data_dir")]
    pub data_dir: String,
    #[serde(alias = "created_at")]
    pub created_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VaultRegistry {
    #[serde(alias = "active_vault")]
    pub active_vault: String,
    pub vaults: BTreeMap<String, VaultEntry>,
}

pub fn registry_path() -> PathBuf {
    let data_dir = std::env::var("CONTEXT_MCP_DATA_DIR")
        .or_else(|_| std::env::var("CONTEXT_VAULT_DATA_DIR"))
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".context-mcp")
                .to_string_lossy()
                .to_string()
        });

    Path::new(&data_dir).join("vaults.json")
}

pub fn load_registry() -> Result<VaultRegistry> {
    let path = registry_path();

    if !path.exists() {
        return Ok(VaultRegistry {
            active_vault: "default".to_string(),
            vaults: BTreeMap::new(),
        });
    }

    let content = std::fs::read_to_string(&path)?;
    let registry = serde_json::from_str(&content)?;
    Ok(registry)
}

pub fn save_registry(registry: &VaultRegistry) -> Result<()> {
    let path = registry_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    let content = serde_json::to_string_pretty(registry)?;
    std::fs::write(path, content)?;
    Ok(())
}

pub fn resolve_active_vault(registry: &VaultRegistry) -> Result<VaultEntry> {
    if let Ok(env_dir) = std::env::var("CONTEXT_VAULT_DATA_DIR") {
        if let Ok(vault_dir) = std::env::var("CONTEXT_VAULT_DIR") {
            return Ok(VaultEntry {
                vault_dir,
                data_dir: env_dir.clone(),
                created_at: chrono::Utc::now().to_rfc3339(),
            });
        }
    }

    if let Some(entry) = registry.vaults.get(&registry.active_vault) {
        return Ok(entry.clone());
    }

    if let Some((_, entry)) = registry.vaults.iter().next() {
        return Ok(entry.clone());
    }

    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let default_vault = home.join(".context-mcp");
    let default_data = home.join("vault");

    Ok(VaultEntry {
        vault_dir: default_vault.to_string_lossy().to_string(),
        data_dir: default_data.to_string_lossy().to_string(),
        created_at: chrono::Utc::now().to_rfc3339(),
    })
}

pub fn list_vaults(registry: &VaultRegistry) -> Vec<(String, VaultEntry)> {
    registry
        .vaults
        .iter()
        .map(|(name, entry)| (name.clone(), entry.clone()))
        .collect()
}

pub fn create_vault(
    registry: &mut VaultRegistry,
    name: String,
    vault_dir: String,
    data_dir: String,
) -> Result<()> {
    if registry.vaults.contains_key(&name) {
        anyhow::bail!("Vault '{}' already exists", name);
    }

    let entry = VaultEntry {
        vault_dir,
        data_dir,
        created_at: chrono::Utc::now().to_rfc3339(),
    };

    registry.vaults.insert(name, entry);
    Ok(())
}

pub fn use_vault(registry: &mut VaultRegistry, name: String) -> Result<()> {
    if !registry.vaults.contains_key(&name) {
        anyhow::bail!("Vault '{}' not found", name);
    }

    registry.active_vault = name;
    Ok(())
}

pub fn remove_vault(registry: &mut VaultRegistry, name: String) -> Result<()> {
    if registry.vaults.remove(&name).is_none() {
        anyhow::bail!("Vault '{}' not found", name);
    }

    if registry.active_vault == name {
        registry.active_vault = registry
            .vaults
            .keys()
            .next()
            .cloned()
            .unwrap_or_else(|| "default".to_string());
    }

    Ok(())
}

pub fn ensure_default_vault_registered() -> Result<()> {
    let mut registry = load_registry()?;

    if !registry.vaults.contains_key("default") {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let default_vault = home.join(".context-mcp");
        let default_data = home.join("vault");

        let entry = VaultEntry {
            vault_dir: default_vault.to_string_lossy().to_string(),
            data_dir: default_data.to_string_lossy().to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        registry.vaults.insert("default".to_string(), entry);
        save_registry(&registry)?;
    }

    Ok(())
}

#[cfg(test)]
#[path = "vaults_tests.rs"]
mod tests;

#[cfg(test)]
mod casing_tests {
    use super::*;

    #[test]
    fn reads_camel_case_and_snake_case_registries() {
        let camel = r#"{"activeVault":"default","vaults":{"default":{"vaultDir":"/v","dataDir":"/d","createdAt":"2026-07-02T08:11:56.050Z"}}}"#;
        let snake = r#"{"active_vault":"default","vaults":{"default":{"vault_dir":"/v","data_dir":"/d","created_at":"2026-07-02"}}}"#;
        for text in [camel, snake] {
            let r: VaultRegistry = serde_json::from_str(text).unwrap();
            assert_eq!(r.active_vault, "default");
            assert_eq!(r.vaults["default"].vault_dir, "/v");
        }
        let written = serde_json::to_string(&serde_json::from_str::<VaultRegistry>(camel).unwrap()).unwrap();
        assert!(written.contains("\"vaultDir\"") && written.contains("\"activeVault\""), "{written}");
    }
}
