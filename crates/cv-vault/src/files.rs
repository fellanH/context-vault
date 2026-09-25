use std::fs;
use anyhow::{anyhow, Result};

pub fn slugify(text: &str, max_len: usize) -> String {
    let mut slug = text
        .to_lowercase()
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() {
                c.to_string()
            } else {
                "-".to_string()
            }
        })
        .collect::<String>()
        .replace("--", "-")
        .trim_matches('-')
        .to_string();

    if slug.len() > max_len {
        slug = slug[..max_len].to_string();
        // Remove partial word at end
        if let Some(pos) = slug.rfind('-') {
            slug.truncate(pos);
        }
        if slug.is_empty() {
            slug = text.chars().take(max_len).collect();
        }
    }

    slug
}

pub fn safe_join(base: &str, parts: &[&str]) -> Result<String> {
    let base_path = std::path::Path::new(base).canonicalize()?;
    let mut result = base_path.clone();
    for part in parts {
        result.push(part);
    }
    let result = result.canonicalize()?;

    if !result.starts_with(&base_path) && result != base_path {
        return Err(anyhow!("Path traversal blocked: resolved path escapes base directory"));
    }

    Ok(result.to_string_lossy().to_string())
}

#[derive(Clone, Debug)]
pub struct WalkResult {
    pub file_path: String,
    pub rel_dir: String,
}

fn visit_dir(
    current_dir: &str,
    rel_dir: &str,
    results: &mut Vec<WalkResult>,
) -> Result<()> {
    let entries = fs::read_dir(current_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if name_str.starts_with('_') {
            continue;
        }

        if !path.is_dir() && !path.is_file() {
            continue;
        }

        if path.is_dir() {
            let next_rel_dir = if rel_dir.is_empty() {
                name_str.to_string()
            } else {
                format!("{}/{}", rel_dir, name_str)
            };
            visit_dir(path.to_str().unwrap(), &next_rel_dir, results)?;
            continue;
        }

        if !name_str.ends_with(".md") {
            continue;
        }

        let file_path = path.to_string_lossy().to_string();
        results.push(WalkResult {
            file_path,
            rel_dir: rel_dir.to_string(),
        });
    }

    Ok(())
}

pub fn walk_dir(dir: &str) -> Result<Vec<WalkResult>> {
    let mut results = Vec::new();
    visit_dir(dir, "", &mut results)?;
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("hello world", 60), "hello-world");
        assert_eq!(slugify("Hello, World!", 60), "hello-world");
        assert_eq!(slugify("Test!!!", 60), "test");
    }

    #[test]
    fn test_safe_join_escapes() {
        let temp = tempfile::TempDir::new().unwrap();
        let base = temp.path().to_str().unwrap();
        let result = safe_join(base, &["..", "..", "etc", "passwd"]);
        assert!(result.is_err());
    }
}
