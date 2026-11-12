//! User-defined vault taxonomy: kind → category/tier/identity/folder.
//!
//! This crate ships mechanism, not ontology: there is no built-in "knowledge /
//! entities / events" taxonomy baked into the code. A vault's actual taxonomy lives
//! entirely in its own `vault.toml`, authored by whoever owns that vault. Without one,
//! `VaultTaxonomy::minimal()` applies a zero-assumption fallback: every kind gets its
//! own flat folder named after itself, tier `working`, no identity-based upsert. See
//! specs/vault-taxonomy.md.
//!
//! A vault.toml may also declare refusers: per-kind `fields` (typed, required meta
//! fields) and a `[vault]` policy (`closed_kinds`, `personal_free`). Those are checked
//! on every save by `schema_check`; this module only parses and holds them.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

pub use crate::schema_types::{FieldSpec, FieldType, VaultPolicy};

pub const VAULT_TOML_FILENAME: &str = "vault.toml";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct KindRule {
    pub category: String,
    pub tier: String,
    /// When true, this kind upserts by `identity_key` (slugified filename, overwrite on
    /// resave) instead of always creating a new randomly-named file. This is a fixed
    /// behavioral flag rather than part of the free-form `category` string because it's
    /// load-bearing engine behavior (upsert semantics), not a folder-naming preference —
    /// folding it into `category` would mean renaming a folder silently changes save
    /// semantics.
    pub identity: bool,
    /// Explicit path override, relative to vault_dir. Defaults to "{category}/{kind}".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// Typed `meta` fields for this kind. Empty = no schema, any meta accepted.
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, FieldSpec>,
    /// Days an entry stays fresh after it was last verified or updated (`stale` lists the rest).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fresh_days: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct RawKindRule {
    category: Option<String>,
    tier: Option<String>,
    identity: Option<bool>,
    folder: Option<String>,
    fields: Option<BTreeMap<String, FieldSpec>>,
    fresh_days: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct TaxonomyFile {
    #[allow(dead_code)]
    schema_version: u32,
    #[serde(default)]
    vault: VaultPolicy,
    #[serde(default)]
    default: RawKindRule,
    #[serde(default)]
    kinds: BTreeMap<String, RawKindRule>,
}

#[derive(Clone, Debug)]
pub struct VaultTaxonomy {
    policy: VaultPolicy,
    default: KindRule,
    kinds: BTreeMap<String, KindRule>,
}

const NO_CONFIG_DEFAULT_TIER: &str = "working";

impl VaultTaxonomy {
    /// Zero-assumption fallback: no ontology, no grouping. Every kind gets a flat
    /// folder named after itself ("{kind}/"), tier `working`, no identity-based
    /// upsert. This is what a vault gets before anyone has written a `vault.toml` --
    /// deliberately not pre-populated with any particular taxonomy's kinds, since
    /// which kinds exist and how they're grouped is the vault owner's call, not this
    /// crate's.
    pub fn minimal() -> Self {
        Self {
            policy: VaultPolicy::default(),
            default: KindRule {
                category: String::new(),
                tier: NO_CONFIG_DEFAULT_TIER.to_string(),
                identity: false,
                folder: None,
                fields: BTreeMap::new(),
                fresh_days: None,
            },
            kinds: BTreeMap::new(),
        }
    }

    /// Load `<vault_dir>/vault.toml`. Falls back to `minimal()` when the file is absent
    /// (silently — this is the common case for a fresh vault) or fails to parse (with a
    /// stderr warning, so a typo in the config never breaks saves/search).
    pub fn load(vault_dir: &str) -> Self {
        let path = Path::new(vault_dir).join(VAULT_TOML_FILENAME);
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(_) => return Self::minimal(),
        };
        match toml::from_str::<TaxonomyFile>(&raw) {
            Ok(file) => Self::from_file(file),
            Err(e) => {
                eprintln!(
                    "warning: failed to parse {} ({e}) — falling back to the zero-config default",
                    path.display()
                );
                Self::minimal()
            }
        }
    }

    fn from_file(file: TaxonomyFile) -> Self {
        let default = KindRule {
            category: file.default.category.unwrap_or_default(),
            tier: file.default.tier.unwrap_or_else(|| NO_CONFIG_DEFAULT_TIER.to_string()),
            identity: file.default.identity.unwrap_or(false),
            folder: file.default.folder,
            fields: file.default.fields.unwrap_or_default(),
            fresh_days: file.default.fresh_days,
        };

        let kinds = file
            .kinds
            .into_iter()
            .map(|(kind, raw)| {
                let rule = KindRule {
                    category: raw.category.unwrap_or_else(|| default.category.clone()),
                    tier: raw.tier.unwrap_or_else(|| default.tier.clone()),
                    identity: raw.identity.unwrap_or(default.identity),
                    folder: raw.folder,
                    fields: raw.fields.unwrap_or_else(|| default.fields.clone()),
                    fresh_days: raw.fresh_days.or(default.fresh_days),
                };
                (kind, rule)
            })
            .collect();

        Self { policy: file.vault, default, kinds }
    }

    pub fn policy(&self) -> &VaultPolicy {
        &self.policy
    }

    /// Kinds with their own `[kinds.<kind>]` entry (not the `[default]` fallback).
    pub fn kinds(&self) -> &BTreeMap<String, KindRule> {
        &self.kinds
    }

    pub fn rule_for(&self, kind: &str) -> &KindRule {
        self.kinds.get(kind).unwrap_or(&self.default)
    }

    pub fn category_for(&self, kind: &str) -> &str {
        &self.rule_for(kind).category
    }

    pub fn tier_for(&self, kind: &str) -> &str {
        &self.rule_for(kind).tier
    }

    pub fn is_identity(&self, kind: &str) -> bool {
        self.rule_for(kind).identity
    }

    /// Directory for this kind, relative to vault_dir: the rule's explicit `folder`
    /// override; else "{category}/{kind}"; else -- if category is unset/empty, as in
    /// `minimal()` -- just "{kind}" flat, since there's no category to group under.
    pub fn folder_for(&self, kind: &str) -> String {
        let rule = self.rule_for(kind);
        if let Some(folder) = &rule.folder {
            return folder.clone();
        }
        if rule.category.is_empty() {
            kind.to_string()
        } else {
            format!("{}/{}", rule.category, kind)
        }
    }
}

#[path = "taxonomy_strict.rs"]
mod strict;

#[path = "taxonomy_render.rs"]
mod render;

#[cfg(test)]
#[path = "taxonomy_tests.rs"]
mod tests;
