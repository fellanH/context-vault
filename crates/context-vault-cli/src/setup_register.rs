//! Writing the MCP server entry into each detected client's config file.

use anyhow::{anyhow, Result};
use serde_json::Value;
use std::fs;

use crate::cmd_setup::{get_server_entry, merge_config, ClientConfig};

pub(super) fn register_client(
    client: &ClientConfig,
    dry_run: bool,
) -> Result<(bool, Option<String>)> {
    let config_path = &client.config_path;
    let config_dir = config_path.parent().ok_or_else(|| anyhow!("invalid path"))?;

    let entry = get_server_entry();

    // Read existing config or create empty
    let mut config: Value = if config_path.exists() {
        let raw = fs::read_to_string(config_path)?;
        serde_json::from_str(&raw).unwrap_or_else(|_| Value::Object(Default::default()))
    } else {
        Value::Object(Default::default())
    };

    // Merge or replace
    config = merge_config(config, entry);

    if dry_run {
        return Ok((true, Some(format!(
            "Would write {} bytes to {}",
            serde_json::to_string(&config)?.len(),
            config_path.display()
        ))));
    }

    // Create parent dirs
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }

    // Write config
    let formatted = serde_json::to_string_pretty(&config)? + "\n";
    fs::write(config_path, formatted)?;

    Ok((true, Some(format!("Registered with {}", config_path.display()))))
}

pub(super) fn print_entry_and_paths(target_clients: &[&ClientConfig]) -> Result<()> {
    println!("  MCP server entry:\n");
    let entry = get_server_entry();
    println!("  {}\n", serde_json::to_string_pretty(&entry)?);
    println!("  Paths:\n");
    for client in target_clients {
        println!("    {} → {}", client.name, client.config_path.display());
    }
    println!();
    Ok(())
}

pub(super) fn register_all(target_clients: &[&ClientConfig], dry_run: bool) {
    for client in target_clients {
        match register_client(client, dry_run) {
            Ok((_, Some(msg))) => println!("  ✓ {} — {}", client.name, msg),
            Ok(_) => println!("  ✓ {}", client.name),
            Err(e) => println!("  ✗ {} — {}", client.name, e),
        }
    }
}
