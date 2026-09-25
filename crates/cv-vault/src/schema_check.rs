//! Save-time refusers declared in vault.toml: closed kind registry, per-kind typed
//! `meta` fields, and the personal-free policy. Called by `write_entry` before any
//! file is written, so a refused save leaves nothing on disk or in the index.
//!
//! Every violation is collected and returned in one error, so an agent can fix a
//! record in one retry instead of discovering problems one at a time.

use anyhow::{bail, Result};
use serde_json::{Map, Value};

use cv_engine::SaveInput;

use crate::kind_suggest::nearest;
use crate::schema_personal::check_personal_free;
use crate::schema_types::{describe, FieldSpec, FieldType};
use crate::taxonomy::VaultTaxonomy;

pub use crate::schema_personal::SENSITIVITY_VALUES;

pub fn check_save(taxonomy: &VaultTaxonomy, input: &SaveInput) -> Result<()> {
    let policy = taxonomy.policy();
    if policy.closed_kinds {
        check_kind(taxonomy, &input.kind)?;
    }

    let fields = &taxonomy.rule_for(&input.kind).fields;
    if fields.is_empty() && !policy.personal_free {
        return Ok(());
    }

    let meta = meta_object(input)?;
    let mut problems = Vec::new();
    for (name, spec) in fields {
        check_field(name, spec, meta.get(name), &mut problems);
    }
    if policy.personal_free {
        check_personal_free(&meta, input.tags.as_deref().unwrap_or(&[]), &mut problems);
    }
    refusal(&input.kind, &problems)
}

fn check_kind(taxonomy: &VaultTaxonomy, kind: &str) -> Result<()> {
    if taxonomy.kinds().contains_key(kind) {
        return Ok(());
    }
    let known: Vec<&str> = taxonomy.kinds().keys().map(|k| k.as_str()).collect();
    let hint = nearest(kind, &known)
        .map(|k| format!(" Did you mean `{k}`?"))
        .unwrap_or_default();
    bail!(
        "save refused: kind `{kind}` is not in this vault's closed kind registry.{hint} Known kinds: {}",
        known.join(", ")
    )
}

fn meta_object(input: &SaveInput) -> Result<Map<String, Value>> {
    match &input.meta {
        None | Some(Value::Null) => Ok(Map::new()),
        Some(Value::Object(m)) => Ok(m.clone()),
        Some(other) => bail!(
            "save refused: meta must be a JSON object, got {}",
            describe(other)
        ),
    }
}

fn refusal(kind: &str, problems: &[String]) -> Result<()> {
    if problems.is_empty() {
        return Ok(());
    }
    let plural = if problems.len() == 1 { "" } else { "s" };
    bail!(
        "save refused for kind `{kind}` ({} problem{plural}):\n- {}",
        problems.len(),
        problems.join("\n- ")
    )
}

fn check_field(name: &str, spec: &FieldSpec, value: Option<&Value>, problems: &mut Vec<String>) {
    let Some(value) = value.filter(|v| !v.is_null()) else {
        if spec.required {
            problems.push(format!("meta.{name} is required ({})", spec.ty.as_str()));
        }
        return;
    };

    if !spec.ty.matches(value) {
        let expected = match spec.ty {
            FieldType::Date => "date (YYYY-MM-DD or RFC 3339)",
            other => other.as_str(),
        };
        problems.push(format!("meta.{name} must be {expected}, got {}", describe(value)));
        return;
    }

    let (Some(allowed), Some(s)) = (&spec.allowed, value.as_str()) else { return };
    if !allowed.iter().any(|a| a == s) {
        problems.push(format!("meta.{name} = {s:?} is not one of: {}", allowed.join(", ")));
    }
}

#[cfg(test)]
#[path = "schema_check_tests.rs"]
mod tests;
