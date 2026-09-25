//! Render a `VaultTaxonomy` back to editable vault.toml — used by `schema init` / `schema show`.

use std::collections::BTreeMap;

use super::{FieldSpec, VaultTaxonomy};

impl VaultTaxonomy {
    /// Render as an editable `vault.toml` — used by `context-vault schema init`.
    pub fn to_toml_string(&self) -> String {
        let mut out = String::new();
        out.push_str("# Context Vault taxonomy — kind -> category/tier/identity/folder.\n");
        out.push_str("# Kinds not listed under [kinds.*] fall back to [default].\n");
        out.push_str("# identity=true: filename + upsert are keyed by identity_key instead of title+id.\n");
        out.push_str("# folder: optional explicit path override (default is \"{category}/{kind}\").\n\n");
        out.push_str("schema_version = 1\n\n");
        if !self.policy.is_default() {
            out.push_str("[vault]\n");
            out.push_str(&format!("closed_kinds = {}\n", self.policy.closed_kinds));
            out.push_str(&format!("personal_free = {}\n\n", self.policy.personal_free));
        }
        out.push_str("[default]\n");
        out.push_str(&format!("category = \"{}\"\n", self.default.category));
        out.push_str(&format!("tier = \"{}\"\n", self.default.tier));
        out.push_str(&format!("identity = {}\n\n", self.default.identity));
        push_fields(&mut out, "default", &self.default.fields);

        for (kind, rule) in &self.kinds {
            out.push_str(&format!("[kinds.{}]\n", kind));
            out.push_str(&format!("category = \"{}\"\n", rule.category));
            out.push_str(&format!("tier = \"{}\"\n", rule.tier));
            if rule.identity {
                out.push_str("identity = true\n");
            }
            if let Some(folder) = &rule.folder {
                out.push_str(&format!("folder = \"{}\"\n", folder));
            }
            out.push('\n');
            if rule.fields != self.default.fields {
                push_fields(&mut out, &format!("kinds.{}", kind), &rule.fields);
            }
        }

        out
    }
}

fn push_fields(out: &mut String, table: &str, fields: &BTreeMap<String, FieldSpec>) {
    if fields.is_empty() {
        return;
    }
    out.push_str(&format!("[{}.fields]\n", table));
    for (name, spec) in fields {
        let mut parts = vec![format!("type = \"{}\"", spec.ty.as_str())];
        if spec.required {
            parts.push("required = true".to_string());
        }
        if let Some(allowed) = &spec.allowed {
            let quoted: Vec<String> = allowed.iter().map(|v| format!("{:?}", v)).collect();
            parts.push(format!("enum = [{}]", quoted.join(", ")));
        }
        out.push_str(&format!("{} = {{ {} }}\n", name, parts.join(", ")));
    }
    out.push('\n');
}
