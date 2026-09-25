//! The refuser vocabulary a vault.toml can declare: typed `meta` fields per kind and
//! the vault-wide `[vault]` policy. Parsed by `taxonomy`, enforced by `schema_check`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FieldType {
    String,
    Number,
    Integer,
    Boolean,
    /// `YYYY-MM-DD` or RFC 3339.
    Date,
    Array,
    Object,
}

impl FieldType {
    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Number => "number",
            FieldType::Integer => "integer",
            FieldType::Boolean => "boolean",
            FieldType::Date => "date",
            FieldType::Array => "array",
            FieldType::Object => "object",
        }
    }

    pub fn matches(&self, value: &Value) -> bool {
        match self {
            FieldType::String => value.is_string(),
            FieldType::Number => value.is_number(),
            FieldType::Integer => value.is_i64() || value.is_u64(),
            FieldType::Boolean => value.is_boolean(),
            FieldType::Date => value.as_str().is_some_and(is_date),
            FieldType::Array => value.is_array(),
            FieldType::Object => value.is_object(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FieldSpec {
    #[serde(rename = "type")]
    pub ty: FieldType,
    #[serde(default)]
    pub required: bool,
    /// Allowed values (string fields only).
    #[serde(default, rename = "enum", skip_serializing_if = "Option::is_none")]
    pub allowed: Option<Vec<String>>,
}

/// Vault-wide refusal policy, `[vault]` in vault.toml. Both default off, so a vault
/// without the section behaves exactly as before.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultPolicy {
    /// Refuse any kind not listed under `[kinds.*]` instead of falling back to `[default]`.
    #[serde(default)]
    pub closed_kinds: bool,
    /// Refuse records classed personal: every save must carry `meta.sensitivity`
    /// (public/internal/confidential), and `subject = Person` or a `person:` tag is refused.
    #[serde(default)]
    pub personal_free: bool,
}

impl VaultPolicy {
    pub fn is_default(&self) -> bool {
        self == &VaultPolicy::default()
    }
}

fn is_date(s: &str) -> bool {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok()
        || chrono::DateTime::parse_from_rfc3339(s).is_ok()
}

/// JSON type name plus value, for refusal messages: `string "950"`, `number 1.5`.
pub fn describe(v: &Value) -> String {
    let ty = match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(s) => return format!("string {s:?}"),
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    };
    format!("{ty} {v}")
}
