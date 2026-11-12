//! vault_schema: what save_context will accept, so an agent can read a kind's
//! rules before saving instead of learning them from a refusal (card 30).

use cv_vault::VaultTaxonomy;
use serde_json::{json, Value};

/// Strict load, same as the write path: a vault.toml that would refuse every save is
/// reported as an error, not papered over with the zero-config default.
pub async fn handle_vault_schema(args: &Value) -> Result<String, String> {
    let vault_dir = crate::paths::resolve_vault_dir().map_err(|e| e.to_string())?;
    let taxonomy = VaultTaxonomy::load_strict(&vault_dir)
        .map_err(|e| format!("vault.toml is invalid, so every save is refused until it is fixed: {e}"))?;
    let kind = args.get("kind").and_then(|v| v.as_str()).filter(|k| !k.is_empty());
    serde_json::to_string_pretty(&describe(&taxonomy, kind)).map_err(|e| e.to_string())
}

pub(super) fn describe(t: &VaultTaxonomy, kind: Option<&str>) -> Value {
    let policy = t.policy();
    let mut rules = vec![];
    if policy.closed_kinds {
        rules.push("closed_kinds: only the kinds listed here can be saved".to_string());
    }
    if policy.personal_free {
        rules.push("personal_free: meta.sensitivity is required; personal data and person:* tags are refused".to_string());
    }
    let known: Vec<&String> = t.kinds().keys().collect();
    match kind {
        Some(k) => {
            let listed = t.kinds().contains_key(k);
            json!({
                "kind": k,
                "listed": listed,
                "allowed": listed || !policy.closed_kinds,
                "rule": t.rule_for(k),
                "vault_rules": rules,
                "kinds": known,
            })
        }
        None => json!({
            "vault_rules": rules,
            "default": t.rule_for(""),
            "kinds": t.kinds(),
        }),
    }
}
