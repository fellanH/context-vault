//! Frontmatter emission (format side). Parsing lives in `fm_parse.rs`.
//! Emission matches the TS `formatFrontmatter` byte-for-byte: field order is
//! caller-controlled (ordered slice), arrays render as `[json, json]`, and scalar
//! strings are JSON-quoted only when they contain YAML-significant characters.

use serde_json::Value;

pub fn format_frontmatter(fields: &[(String, Value)]) -> String {
    let mut lines = vec!["---".to_string()];
    for (k, v) in fields {
        if let Some(line) = format_fm_field(k, v) {
            lines.push(line);
        }
    }
    lines.push("---".to_string());
    lines.join("\n")
}

/// One `key: value` line, or None for null values (which TS skips).
fn format_fm_field(key: &str, value: &Value) -> Option<String> {
    if value.is_null() {
        return None;
    }
    Some(format!("{}: {}", key, format_fm_value(value)))
}

/// The right-hand side of a frontmatter line.
fn format_fm_value(value: &Value) -> String {
    if let Some(arr) = value.as_array() {
        let items: Vec<String> = arr
            .iter()
            .map(|item| serde_json::to_string(item).unwrap_or_else(|_| "null".to_string()))
            .collect();
        return format!("[{}]", items.join(", "));
    }
    if let Some(s) = value.as_str() {
        if needs_quoting(s) {
            return serde_json::to_string(s).unwrap_or_else(|_| s.to_string());
        }
        return s.to_string();
    }
    match value {
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        _ => value.to_string(),
    }
}

fn needs_quoting(s: &str) -> bool {
    s.chars().any(|c| {
        matches!(
            c,
            ':' | '#' | '\'' | '"' | '{' | '}' | '[' | ']' | ',' | '>' | '|' | '&' | '*' | '?'
                | '!' | '@' | '`'
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_frontmatter_exact() {
        let fields = vec![
            ("id".to_string(), Value::String("test123".to_string())),
            ("title".to_string(), Value::String("My Title".to_string())),
            ("kind".to_string(), Value::String("insight".to_string())),
            (
                "tags".to_string(),
                Value::Array(vec![
                    Value::String("tag1".to_string()),
                    Value::String("tag2".to_string()),
                ]),
            ),
        ];
        let result = format_frontmatter(&fields);
        assert!(result.starts_with("---\n"));
        assert!(result.ends_with("\n---"));
        assert!(result.contains("id: test123"));
        assert!(result.contains("kind: insight"));
        assert!(result.contains("tags: [\"tag1\", \"tag2\"]"));
    }

    #[test]
    fn test_format_quotes_special_chars() {
        let fields = vec![("title".to_string(), Value::String("a: b".to_string()))];
        assert!(format_frontmatter(&fields).contains("title: \"a: b\""));
    }
}
