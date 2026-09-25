use anyhow::{anyhow, Result};
use std::fs;
use std::path::Path;
use std::process::Command;

#[path = "project_tech_stack.rs"]
mod tech_stack;
use tech_stack::detect_tech_stack;

#[allow(dead_code)]
fn safe_read(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

#[allow(dead_code)]
fn safe_exec(cmd: &str, cwd: &str) -> Option<String> {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    let cmd_name = parts.first()?;

    Command::new(cmd_name)
        .args(&parts[1..])
        .current_dir(cwd)
        .output()
        .ok()
        .and_then(|output| {
            output.status.success().then(|| {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            })?
        })
}


#[allow(dead_code)]
fn extract_readme_description(project_path: &str) -> Option<String> {
    let readme_path = format!("{}/README.md", project_path);
    let readme_lower = format!("{}/readme.md", project_path);
    let content = safe_read(&readme_path).or_else(|| safe_read(&readme_lower))?;

    content
        .lines()
        .find_map(|line| {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                Some(trimmed.chars().take(200).collect())
            } else {
                None
            }
        })
}

fn slugify(text: &str) -> String {
    let slug = text
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

    if slug.is_empty() {
        text.chars().take(60).collect()
    } else {
        slug.chars().take(60).collect()
    }
}

#[derive(Debug, Clone)]
pub struct ProjectIngestResult {
    pub id: String,
    pub title: String,
    pub file_path: String,
    pub identity_key: String,
    pub body: String,
    pub tags: Vec<String>,
}

/// Assemble the markdown body for a project entry from the detected metadata.
fn build_project_body(
    description: Option<&str>,
    tech_stack: &[String],
    repo_url: Option<&str>,
    last_commit: Option<&str>,
    has_claude_md: bool,
) -> String {
    let mut body = String::new();
    if let Some(d) = description {
        body.push_str(d);
        body.push_str("\n\n");
    }
    if !tech_stack.is_empty() {
        body.push_str(&format!("**Tech stack:** {}\n\n", tech_stack.join(", ")));
    }
    if let Some(url) = repo_url {
        body.push_str(&format!("**Repo:** {}\n\n", url.trim()));
    }
    if let Some(lc) = last_commit {
        body.push_str(&format!("**Last commit:** {}\n\n", lc.trim()));
    }
    if has_claude_md {
        body.push_str("Has `CLAUDE.md`.\n");
    }
    if body.is_empty() {
        body.push_str("Project entry.\n");
    }
    body
}

pub fn ingest_project(
    project_path: &str,
    tags: Option<Vec<String>>,
    pillar: Option<String>,
) -> Result<ProjectIngestResult> {
    if project_path.trim().is_empty() {
        return Err(anyhow!(
            "Required: path (absolute path to project directory)"
        ));
    }

    if !Path::new(project_path).exists() {
        return Err(anyhow!("Directory not found: {}", project_path));
    }

    // Read package.json if present
    let pkg_json: Option<serde_json::Value> = {
        let pkg_path = format!("{}/package.json", project_path);
        safe_read(&pkg_path).and_then(|content| serde_json::from_str(&content).ok())
    };

    // Derive project name
    let mut project_name = Path::new(project_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project")
        .to_string();

    if let Some(pkg) = &pkg_json {
        if let Some(name) = pkg.get("name").and_then(|v| v.as_str()) {
            project_name = name
                .strip_prefix('@')
                .and_then(|s| s.split('/').nth(1))
                .unwrap_or(name)
                .to_string();
        }
    }

    // Identity key (slug-safe)
    let identity_key = slugify(&project_name);

    // Metadata: description (package.json > README), tech stack, git, CLAUDE.md.
    let pkg_desc = pkg_json
        .as_ref()
        .and_then(|p| p.get("description").and_then(|v| v.as_str()));
    let readme_desc = extract_readme_description(project_path);
    let description = pkg_desc.or(readme_desc.as_deref());
    let tech_stack = detect_tech_stack(project_path, pkg_json.as_ref());
    let is_git_repo = Path::new(project_path).join(".git").exists();
    let repo_url = is_git_repo
        .then(|| safe_exec("git remote get-url origin", project_path))
        .flatten();
    let last_commit = is_git_repo
        .then(|| safe_exec("git log -1 --format=%ci", project_path))
        .flatten();
    let has_claude_md = Path::new(project_path).join("CLAUDE.md").exists();
    let body = build_project_body(
        description,
        &tech_stack,
        repo_url.as_deref(),
        last_commit.as_deref(),
        has_claude_md,
    );

    // Build tags
    let mut all_tags = vec![format!("bucket:{}", identity_key)];
    if let Some(p) = pillar {
        all_tags.push(format!("bucket:{}", p));
    }
    if let Some(additional) = tags {
        all_tags.extend(additional);
    }
    all_tags.sort();
    all_tags.dedup();

    // PARITY: The TypeScript version calls captureAndIndex which integrates with
    // DB + file writing. Here we return the data; the caller (Phase 5 MCP tool)
    // persists it via write_entry, which assigns the real file_path.
    let ulid = ulid::Ulid::new();

    Ok(ProjectIngestResult {
        id: ulid.to_string(),
        title: project_name,
        file_path: format!("reference/project/{}.md", identity_key),
        identity_key,
        body,
        tags: all_tags,
    })
}

#[cfg(test)]
#[path = "project_tests.rs"]
mod tests;
