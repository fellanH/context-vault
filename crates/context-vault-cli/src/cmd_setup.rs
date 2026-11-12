use std::path::PathBuf;
use anyhow::{anyhow, Result};
use serde_json::{json, Value};

use crate::client_paths::{get_app_data_dir, get_vscode_data_dir};

#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub id: &'static str,
    pub name: &'static str,
    pub config_path: PathBuf,
}

pub struct SetupOpts {
    pub client: Option<String>,
    pub print: bool,
    pub dry_run: bool,
}

impl ClientConfig {
    fn detect(&self) -> bool {
        self.config_path.parent().is_some_and(|p| p.exists())
    }
}

pub fn get_all_clients() -> Vec<ClientConfig> {
    let home = dirs::home_dir().expect("home dir");

    vec![
        ClientConfig {
            id: "claude-desktop",
            name: "Claude Desktop",
            config_path: get_app_data_dir().join("Claude").join("claude_desktop_config.json"),
        },
        ClientConfig {
            id: "claude-code",
            name: "Claude Code",
            config_path: home.join(".claude.json"),
        },
        ClientConfig {
            id: "cursor",
            name: "Cursor",
            config_path: home.join(".cursor").join("mcp.json"),
        },
        ClientConfig {
            id: "windsurf",
            name: "Windsurf",
            config_path: if home.join(".windsurf").exists() {
                home.join(".windsurf").join("mcp.json")
            } else {
                home.join(".codeium").join("windsurf").join("mcp_config.json")
            },
        },
        ClientConfig {
            id: "cline",
            name: "Cline (VS Code)",
            config_path: get_vscode_data_dir()
                .join("saoudrizwan.claude-dev")
                .join("settings")
                .join("cline_mcp_settings.json"),
        },
        ClientConfig {
            id: "roo-code",
            name: "Roo Code (VS Code)",
            config_path: get_vscode_data_dir()
                .join("rooveterinaryinc.roo-cline")
                .join("settings")
                .join("cline_mcp_settings.json"),
        },
    ]
}

pub(crate) fn get_server_entry() -> Value {
    json!({
        "command": server_command(),
        "args": ["mcp"]
    })
}

/// The command a client config should start: this binary's own path, so the server starts even
/// when the client's PATH lacks the install folder (install.sh puts it in ~/.local/bin, which a
/// fresh macOS shell and every GUI-launched client leave off PATH). Prefers the PATH entry that
/// resolves to this binary, so a symlink like /opt/homebrew/bin survives upgrades. Falls back to
/// the bare name when run from an npm/npx cache, whose path is not stable.
pub(crate) fn server_command() -> String {
    let Ok(exe) = std::env::current_exe() else {
        return "context-vault".to_string();
    };
    command_for(&exe, std::env::var_os("PATH"))
}

pub(crate) fn command_for(exe: &std::path::Path, path_var: Option<std::ffi::OsString>) -> String {
    let exe_str = exe.to_string_lossy();
    if exe_str.contains("node_modules") || exe_str.contains("_npx") {
        return "context-vault".to_string();
    }
    let canonical = std::fs::canonicalize(exe).unwrap_or_else(|_| exe.to_path_buf());
    if let Some(path_var) = path_var {
        let name = exe.file_name().unwrap_or_default();
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(name);
            if candidate.is_absolute()
                && std::fs::canonicalize(&candidate).is_ok_and(|c| c == canonical)
            {
                return candidate.to_string_lossy().into_owned();
            }
        }
    }
    canonical.to_string_lossy().into_owned()
}

pub(crate) fn merge_config(mut existing: Value, new_entry: Value) -> Value {
    if let Value::Object(ref mut obj) = existing {
        if let Some(servers) = obj.get_mut("mcpServers") {
            if let Value::Object(ref mut srv) = servers {
                srv.insert("context-vault".to_string(), new_entry);
            }
        } else {
            obj.insert(
                "mcpServers".to_string(),
                json!({ "context-vault": new_entry }),
            );
        }
    }
    existing
}

/// Resolve the `--client` flag (or auto-detect) into the clients to act on.
fn select_target_clients<'a>(
    all_clients: &'a [ClientConfig],
    client_id: &Option<String>,
) -> Result<Vec<&'a ClientConfig>> {
    let Some(client_id) = client_id else {
        return Ok(all_clients.iter().filter(|c| c.detect()).collect());
    };
    if client_id == "all" {
        return Ok(all_clients.iter().filter(|c| c.detect()).collect());
    }
    let matching = all_clients
        .iter()
        .find(|c| c.id == client_id)
        .ok_or_else(|| anyhow!("Unknown client: {}", client_id))?;
    Ok(vec![matching])
}

pub async fn run_setup(opts: SetupOpts) -> Result<()> {
    let all_clients = get_all_clients();
    let target_clients = select_target_clients(&all_clients, &opts.client)?;

    if target_clients.is_empty() {
        println!("\n  No clients detected.\n");
        return Ok(());
    }

    println!("\n  context-vault setup\n");

    if opts.print {
        return crate::setup_register::print_entry_and_paths(&target_clients);
    }

    println!(
        "  {}",
        if opts.dry_run { "[dry-run] Would register with:\n" } else { "Registering with:\n" }
    );

    crate::setup_register::register_all(&target_clients, opts.dry_run);

    println!();
    if !opts.dry_run {
        println!("  Setup complete. Restart your AI tools to apply changes.\n");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merge_empty_config() {
        let empty = Value::Object(Default::default());
        let entry = get_server_entry();
        let result = merge_config(empty, entry);

        assert!(result.get("mcpServers").is_some());
        assert!(result["mcpServers"].get("context-vault").is_some());
        assert_eq!(result["mcpServers"]["context-vault"]["command"], server_command());
    }

    #[test]
    fn test_merge_existing_servers() {
        let existing = json!({
            "mcpServers": {
                "other-server": { "command": "other" }
            }
        });
        let entry = get_server_entry();
        let result = merge_config(existing, entry);

        assert!(result["mcpServers"].get("other-server").is_some());
        assert!(result["mcpServers"].get("context-vault").is_some());
        assert_eq!(result["mcpServers"]["other-server"]["command"], "other");
    }

    #[test]
    fn test_merge_replace_existing() {
        let existing = json!({
            "mcpServers": {
                "context-vault": { "command": "old-path" }
            }
        });
        let entry = get_server_entry();
        let result = merge_config(existing, entry);

        assert_eq!(result["mcpServers"]["context-vault"]["command"], server_command());
        assert_eq!(result["mcpServers"]["context-vault"]["args"][0], "mcp");
    }

    #[test]
    fn command_is_absolute_off_path() {
        let dir = std::env::temp_dir().join(format!("cv-setup-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("context-vault");
        std::fs::write(&exe, b"").unwrap();
        let cmd = command_for(&exe, Some("/usr/bin:/bin".into()));
        assert!(std::path::Path::new(&cmd).is_absolute(), "{cmd}");
        assert!(cmd.ends_with("context-vault"));
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn command_prefers_path_entry_and_bare_for_npx() {
        let dir = std::env::temp_dir().join(format!("cv-setup-test2-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join("context-vault");
        std::fs::write(&exe, b"").unwrap();
        let path = std::env::join_paths([dir.clone()]).unwrap();
        assert_eq!(command_for(&exe, Some(path)), dir.join("context-vault").to_string_lossy());
        let npx = std::path::Path::new("/home/u/.npm/_npx/abc/node_modules/.bin/context-vault");
        assert_eq!(command_for(npx, None), "context-vault");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
