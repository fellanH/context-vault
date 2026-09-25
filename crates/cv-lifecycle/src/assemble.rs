//! Role-based context assembly with token budgeting.
//! Retrieves entries from vault and assembles a markdown report for agents.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

const TOKEN_TO_CHAR_RATIO: usize = 4;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Role {
    #[serde(rename = "worker")]
    Worker,
    #[serde(rename = "pm")]
    PM,
    #[serde(rename = "ceo")]
    CEO,
    #[serde(rename = "steer")]
    Steer,
}

impl Role {
    fn as_str(&self) -> &'static str {
        match self {
            Role::Worker => "worker",
            Role::PM => "pm",
            Role::CEO => "ceo",
            Role::Steer => "steer",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RoleProfile {
    rules: Vec<String>,
    skills: Vec<String>,
    memory: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncludedEntryMeta {
    pub title: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleMetadata {
    pub tokens_used: usize,
    pub budget: usize,
    pub entries_included: usize,
    pub role: String,
    pub included_entries: Vec<IncludedEntryMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembleResult {
    pub markdown: String,
    pub metadata: AssembleMetadata,
}

fn default_role_profiles() -> HashMap<String, RoleProfile> {
    let mut map = HashMap::new();

    map.insert(
        "worker".to_string(),
        RoleProfile {
            rules: vec![
                "coding", "patterns", "standards", "error-handling", "debugging",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
            skills: vec!["git", "compile", "test", "local"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            memory: vec!["feedback", "user", "project", "reference"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        },
    );

    map.insert(
        "pm".to_string(),
        RoleProfile {
            rules: vec![
                "orchestration",
                "planning",
                "spec-writing",
                "review",
                "debugging-escalation",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
            skills: vec!["dispatch", "review", "triage", "report"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            memory: vec!["project", "team", "feedback"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        },
    );

    map.insert(
        "ceo".to_string(),
        RoleProfile {
            rules: vec!["strategy", "epistemic-honesty", "decision-making", "business-logic"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            skills: vec!["triage", "delegate", "report"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            memory: vec!["north-star", "project", "user"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        },
    );

    map.insert(
        "steer".to_string(),
        RoleProfile {
            rules: vec![
                "steer", "strategy", "feedback-loops", "analysis",
            ]
            .into_iter()
            .map(|s| s.to_string())
            .collect(),
            skills: vec!["triage", "report", "analyze"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            memory: vec!["north-star", "feedback"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
        },
    );

    map
}

/// Assemble context for a specific role.
/// Allocates token budget across task, vault retrieval, rules, and skills.
/// PARITY: Full version requires hybrid_search; this version is a dry-run stub
/// that returns an empty vault section.
pub fn assemble_context(
    role: Role,
    task: &str,
    budget: usize,
) -> AssembleResult {
    let profiles = default_role_profiles();
    let profile = profiles
        .get(role.as_str())
        .cloned()
        .unwrap_or_else(|| profiles.get("worker").unwrap().clone());

    let chars_budget = budget * TOKEN_TO_CHAR_RATIO;

    // Allocate budget across sections
    let _rules_chars = (chars_budget * 20) / 100;
    let _skills_chars = (chars_budget * 10) / 100;
    let _task_chars = (chars_budget * 20) / 100;
    let _vault_chars = (chars_budget * 30) / 100;

    let mut markdown = format!("# Assembled Context: {}\n\n", role.as_str().to_uppercase());
    let mut total_chars_used = markdown.len();

    // Add Task Spec
    let task_section = format!("## Active Task\n\n{}\n\n", task);
    markdown.push_str(&task_section);
    total_chars_used += task_section.len();

    // Add Role Directives (U-Curve: rules at top)
    let rules_section = format!(
        "## Role Directives\n\n### Rules\n{}\n\n### Skills\n{}\n\n",
        profile.rules.join(", "),
        profile.skills.join(", ")
    );
    markdown = format!("{}{}", rules_section, markdown);
    total_chars_used += rules_section.len();

    // PARITY: Full version would call hybrid_search here and populate vault context.
    // For now, we return the stub with just task + rules.

    AssembleResult {
        markdown,
        metadata: AssembleMetadata {
            tokens_used: total_chars_used.div_ceil(TOKEN_TO_CHAR_RATIO),
            budget,
            entries_included: 0,
            role: role.as_str().to_string(),
            included_entries: Vec::new(),
        },
    }
}

#[cfg(test)]
#[path = "assemble_tests.rs"]
mod tests;
