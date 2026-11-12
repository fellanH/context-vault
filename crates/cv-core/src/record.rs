//! One record, the unit both vault types store. In a data vault it is one JSONL line;
//! in a knowledge vault it is one markdown file (frontmatter + body = `text`).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub kind: String,
    pub created: DateTime<Utc>,
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Map::is_empty")]
    pub meta: Map<String, Value>,
    /// Ids this record replaces. Data vaults never edit in place; a correction is a newer
    /// record that supersedes the old one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub supersedes: Vec<String>,
}

impl Record {
    /// A new record with a fresh ULID and `created` = now.
    pub fn new(kind: &str, text: &str) -> Self {
        Self {
            id: ulid::Ulid::new().to_string(),
            kind: kind.to_string(),
            created: Utc::now(),
            text: text.to_string(),
            title: None,
            source: None,
            tags: Vec::new(),
            meta: Map::new(),
            supersedes: Vec::new(),
        }
    }
}
