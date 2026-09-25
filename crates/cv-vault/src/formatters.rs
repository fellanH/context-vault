use serde_json::Map;

pub struct FormatInput {
    pub title: Option<String>,
    pub body: String,
    pub meta: Option<Map<String, serde_json::Value>>,
}

pub fn format_body(kind: &str, input: FormatInput) -> String {
    match kind {
        "insight" => format!("\n{}\n", input.body),
        "decision" => {
            let t = input.title.unwrap_or_else(|| input.body.chars().take(80).collect());
            format!("\n## Decision\n\n{}\n\n## Rationale\n\n{}\n", t, input.body)
        }
        "pattern" => {
            let t = input.title.unwrap_or_else(|| input.body.chars().take(80).collect());
            let lang = input
                .meta
                .as_ref()
                .and_then(|m| m.get("language"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            format!("\n# {}\n\n```{}\n{}\n```\n", t, lang, input.body)
        }
        _ => {
            // default formatter
            match input.title {
                Some(title) => format!("\n# {}\n\n{}\n", title, input.body),
                None => format!("\n{}\n", input.body),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_body_default() {
        let input = FormatInput {
            title: Some("Test Title".to_string()),
            body: "Test body".to_string(),
            meta: None,
        };
        let result = format_body("note", input);
        assert!(result.contains("# Test Title"));
        assert!(result.contains("Test body"));
    }

    #[test]
    fn test_format_body_pattern() {
        let input = FormatInput {
            title: Some("Test Pattern".to_string()),
            body: "test code".to_string(),
            meta: None,
        };
        let result = format_body("pattern", input);
        assert!(result.contains("# Test Pattern"));
        assert!(result.contains("```"));
        assert!(result.contains("test code"));
    }
}
