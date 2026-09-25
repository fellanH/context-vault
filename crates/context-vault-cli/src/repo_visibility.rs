//! Public-repo guard: determine write-target repo visibility before session-log writes.
//! Canon: `~/.claude/rules/behavior/public-repo-boundary.md` § clause 3.
//!
//! Fail-closed on public or unknown visibility: refuses writes loudly (stderr + vault row).
//! Per-run cache only — never persisted.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Mutex;

use anyhow::{Context, Result};
use once_cell::sync::Lazy;

/// Per-process cache of repo visibility verdicts: repo path -> is_public
static VISIBILITY_CACHE: Lazy<Mutex<HashMap<String, bool>>> = Lazy::new(|| Mutex::new(HashMap::new()));

/// Determines if the target repo (resolved from `cwd` git root) has a public remote.
/// Returns `true` if ANY remote is public, `false` if no remotes or all private,
/// and Err if visibility is UNKNOWN (gh error, network, no auth, etc.) — which
/// fail-closes to refuse the write.
///
/// Caches results per repo per process run (HashMap, never persisted).
pub fn repo_visibility_is_public(cwd: &str) -> Result<bool> {
    // Normalize the cwd path for caching
    let repo_path = std::fs::canonicalize(cwd)
        .unwrap_or_else(|_| PathBuf::from(cwd))
        .to_string_lossy()
        .to_string();

    // Check cache first
    {
        let cache = VISIBILITY_CACHE.lock().unwrap();
        if let Some(&is_public) = cache.get(&repo_path) {
            return Ok(is_public);
        }
    }

    // Resolve git root and fetch remotes
    let git_remotes = fetch_git_remotes(cwd)?;

    // No remotes = private by definition
    if git_remotes.is_empty() {
        let cache = VISIBILITY_CACHE.lock().unwrap();
        // SAFETY: we already checked cache above, and we're in a single-threaded
        // context relative to this repo, so no race on insertion.
        drop(cache); // explicitly drop read guard before write
        VISIBILITY_CACHE.lock().unwrap().insert(repo_path, false);
        return Ok(false);
    }

    // Check each remote for public visibility
    for remote in &git_remotes {
        match check_remote_visibility(remote) {
            Ok(true) => {
                // Public remote found — cache and return true
                VISIBILITY_CACHE.lock().unwrap().insert(repo_path.clone(), true);
                return Ok(true);
            }
            Ok(false) => {
                // This remote is private, continue checking others
                continue;
            }
            Err(e) => {
                // UNKNOWN (gh error, no auth, network failure, etc.) — fail closed
                eprintln!("[context-vault] public-repo-guard: checking visibility of {remote}: {e:#}");
                anyhow::bail!("public-repo-guard: unable to determine visibility (UNKNOWN = refuse)")
            }
        }
    }

    // All remotes checked, all private
    VISIBILITY_CACHE.lock().unwrap().insert(repo_path, false);
    Ok(false)
}

/// Fetches all remotes from `git -C <cwd> remote -v`, returning their URLs.
fn fetch_git_remotes(cwd: &str) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["-C", cwd, "remote", "-v"])
        .output()
        .context("spawning git remote -v")?;

    if !output.status.success() {
        // No .git or git error — treat as private (no remotes)
        return Ok(Vec::new());
    }

    let remotes_text = String::from_utf8_lossy(&output.stdout);
    let mut urls = Vec::new();
    for line in remotes_text.lines() {
        if let Some(url) = line.split_whitespace().nth(1) {
            if !urls.contains(&url.to_string()) {
                urls.push(url.to_string());
            }
        }
    }
    Ok(urls)
}

/// Checks a single remote's visibility using `gh repo view <owner/repo> --json visibility`.
/// For github.com URLs, parses owner/repo and calls gh. For non-GitHub URLs, returns false
/// (private, fail-open on unknown forges). Returns Err if gh command fails (UNKNOWN).
fn check_remote_visibility(remote_url: &str) -> Result<bool> {
    // Parse github.com owner/repo from remote URL
    let Some(owner_repo) = parse_github_owner_repo(remote_url) else {
        // Not a GitHub remote — treat as private (unknown forges fail open, not closed)
        return Ok(false);
    };

    // Call `gh repo view <owner/repo> --json visibility`
    let output = Command::new("gh")
        .args(["repo", "view", &owner_repo, "--json", "visibility"])
        .output()
        .context("spawning gh repo view")?;

    if !output.status.success() {
        // gh error: auth failed, network error, repo not found, etc. — UNKNOWN
        let stderr = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("gh repo view exited {}: {}", output.status, stderr.trim());
    }

    // Parse visibility field from JSON
    let json_text = String::from_utf8_lossy(&output.stdout);
    match serde_json::from_str::<serde_json::Value>(&json_text) {
        Ok(json) => {
            if let Some(visibility) = json.get("visibility").and_then(|v| v.as_str()) {
                Ok(visibility == "PUBLIC")
            } else {
                anyhow::bail!("gh response missing visibility field: {json_text}")
            }
        }
        Err(e) => {
            anyhow::bail!("parsing gh response JSON: {e}")
        }
    }
}

/// Parses a git remote URL and returns "owner/repo" for github.com remotes.
/// Returns None for non-GitHub URLs.
fn parse_github_owner_repo(remote_url: &str) -> Option<String> {
    // Handles both HTTPS (https://github.com/owner/repo.git) and SSH (git@github.com:owner/repo.git)
    if remote_url.contains("github.com") {
        // Extract owner/repo part
        if let Some(after_colon) = remote_url.split(':').nth(1) {
            // SSH format: git@github.com:owner/repo.git
            let repo_part = after_colon
                .trim_end_matches(".git")
                .trim_end_matches('/');
            return Some(repo_part.to_string());
        } else if let Some(after_slash) = remote_url.rfind("github.com/") {
            // HTTPS format: https://github.com/owner/repo.git
            let repo_part = &remote_url[after_slash + "github.com/".len()..]
                .trim_end_matches(".git")
                .trim_end_matches('/');
            return Some(repo_part.to_string());
        }
    }
    None
}
