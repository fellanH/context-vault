use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IndexingConfig {
    pub exclude_kinds: Vec<String>,
    pub exclude_categories: Vec<String>,
    pub max_body_size: usize,
    pub auto_index_events: bool,
}

impl Default for IndexingConfig {
    fn default() -> Self {
        IndexingConfig {
            exclude_kinds: vec![],
            exclude_categories: vec![],
            max_body_size: 0,
            auto_index_events: true,
        }
    }
}

pub fn should_index(
    kind: &str,
    category: &str,
    body_length: usize,
    explicit_indexed: Option<bool>,
    config: Option<&IndexingConfig>,
) -> bool {
    if let Some(true) = explicit_indexed {
        return true;
    }

    if let Some(false) = explicit_indexed {
        return false;
    }

    let default = IndexingConfig::default();
    let c = config.unwrap_or(&default);

    if !c.exclude_kinds.is_empty() && c.exclude_kinds.contains(&kind.to_string()) {
        return false;
    }

    if !c.exclude_categories.is_empty() && c.exclude_categories.contains(&category.to_string()) {
        return false;
    }

    if category == "event" && !c.auto_index_events {
        return false;
    }

    if c.max_body_size > 0 && body_length > c.max_body_size {
        return false;
    }

    true
}

#[cfg(test)]
#[path = "indexing_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "indexing_more_tests.rs"]
mod more_tests;
