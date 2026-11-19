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

/// After a save to a vault whose vault.toml lists kinds but does not refuse new ones (OPS-405):
/// tell the writer its kind is new, with the nearest listed kind, so a typo or a one-off kind is
/// caught by the agent that wrote it. Empty when the kind is listed, no kinds are listed, or the
/// vault.toml cannot be read (the save path reports that itself).
pub(super) fn new_kind_note(kind: &str) -> String {
    let Ok(vault_dir) = crate::paths::resolve_vault_dir() else { return String::new() };
    let Ok(t) = VaultTaxonomy::load_strict(&vault_dir) else { return String::new() };
    kind_note_for(&t, kind)
}

/// The note for one taxonomy: empty when the kind is listed, nothing is listed, or new kinds are refused anyway.
pub(super) fn kind_note_for(t: &VaultTaxonomy, kind: &str) -> String {
    if t.kinds().is_empty() || t.kinds().contains_key(kind) || t.policy().closed_kinds {
        return String::new();
    }
    let listed: Vec<&str> = t.kinds().keys().map(|k| k.as_str()).collect();
    let hint = cv_vault::nearest(kind, &listed).map(|k| format!(" Nearest listed kind: `{k}`.")).unwrap_or_default();
    format!("\nNote: kind `{kind}` is new to this vault (not in vault.toml), so it was filed under the default rule.{hint} vault_schema lists the kinds.")
}
