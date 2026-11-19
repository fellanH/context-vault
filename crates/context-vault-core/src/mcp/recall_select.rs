//! Which search hits become prompt hints, and in what order (OPS-402). Split from
//! `handlers_recall.rs`, which fetches the hits.

/// Hints are passive context injection, so weak matches cost more than missing
/// ones. RRF scores are rank-based (rank-1 single-lane ≈ 1.67 × tier/heat/recency
/// boosts), so this floor drops stale, low-ranked tail results — it cannot judge
/// whether a rank-1 match is semantically relevant; project scoping handles the
/// cross-project case.
const MIN_HINT_SCORE: f32 = 0.4;

/// Machine logs, not knowledge: never offered as a passive hint (OPS-402). They stay in search,
/// so a prompt that wants a session log still finds it with get_context or `search`.
const HINT_SKIP_KINDS: &[&str] = &["session", "event", "log", "tool-bypass", "rule-candidate"];

/// Folder names too common to say where a session works.
const GENERIC_DIRS: &[&str] = &["users", "admin", "home", "omni", "app", "src", "tools", "company", "projects", "code"];

/// The words naming the area a session works in, from its cwd below the home folder:
/// `~/work/builder/app` gives `builder`, `~/.config/fleet` gives `fleet`. The first folder
/// left is the workspace every session shares, so it counts only when it is the only one.
/// Hints whose source, path or tags name one come first (OPS-402: on a 30-prompt bench such
/// hints were judged useful 11 of 25 times, others 10 of 56).
pub(super) fn area_words(cwd: &str) -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let below = cwd.strip_prefix(home.as_str()).filter(|_| !home.is_empty()).unwrap_or(cwd);
    let mut words: Vec<String> = below
        .split('/')
        .map(|p| p.trim_start_matches('.').to_lowercase())
        .filter(|p| p.len() >= 3 && !GENERIC_DIRS.contains(&p.as_str()))
        .collect();
    if words.len() > 1 {
        words.remove(0);
    }
    words
}

/// Whether the entry's source, file path or tags name one of the area words as a whole word.
fn is_local(entry: &crate::engine::Entry, area: &[String]) -> bool {
    if area.is_empty() {
        return false;
    }
    let text = format!("{} {} {}", entry.source.as_deref().unwrap_or(""), entry.file_path.as_deref().unwrap_or(""), entry.tags.join(" ")).to_lowercase();
    text.split(|c: char| !c.is_alphanumeric() && c != '-').any(|w| area.iter().any(|a| a == w))
}

/// Words too common to say what a prompt or a title is about.
const TITLE_STOPWORDS: &[&str] = &[
    "the", "are", "was", "were", "been", "being", "have", "has", "had", "does", "did", "will", "would",
    "could", "should", "may", "might", "shall", "can", "need", "must", "and", "not", "for", "with",
    "from", "when", "done", "its", "that", "this",
];

/// The content words of a prompt or a title: lowercased, three letters or more, common words out.
pub(super) fn title_words(text: &str) -> std::collections::HashSet<String> {
    text.to_lowercase()
        .split(|c: char| !c.is_alphanumeric() && c != '_')
        .filter(|w| w.chars().count() >= 3 && !TITLE_STOPWORDS.contains(w))
        .map(|w| w.to_string())
        .collect()
}

/// How many of the prompt's content words the entry's title holds, counted up to two.
/// OPS-402: on a blind-judged pool of 1,036 candidates for 30 prompts, a title holding two
/// or more prompt words was useful 21 % of the time (65 of 299) and one holding none 3 %.
fn title_overlap(entry: &crate::engine::Entry, prompt: &std::collections::HashSet<String>) -> usize {
    let title = title_words(entry.title.as_deref().unwrap_or(""));
    title.iter().filter(|w| prompt.contains(*w)).count().min(2)
}

/// Post-select fused search results into hint-worthy entries: apply the score
/// floor, drop untitled entries (a hint rendered as "(untitled)" carries no
/// information), dedup near-identical entries by (title, kind) — duplicate saves
/// are common and two hints with the same title carry no extra signal — and
/// truncate to `max_hints`. Results arrive sorted by score descending, so the
/// kept duplicate is always the strongest one. Then entries whose title holds more
/// of the prompt's words come first, and within those the session's own area,
/// each group keeping its score order.
pub(super) fn select_hints(
    results: Vec<crate::engine::ScoredEntry>,
    prompt: &std::collections::HashSet<String>,
    area: &[String],
    max_hints: usize,
) -> Vec<crate::engine::ScoredEntry> {
    let mut seen = std::collections::HashSet::new();
    let mut kept: Vec<_> = results
        .into_iter()
        .filter(|r| r.score >= MIN_HINT_SCORE)
        .filter(|r| !HINT_SKIP_KINDS.contains(&r.entry.kind.as_str()))
        .filter(|r| r.entry.title.as_deref().is_some_and(|t| !t.trim().is_empty()))
        .filter(|r| seen.insert((r.entry.title.clone(), r.entry.kind.clone())))
        .collect();
    kept.sort_by_key(|r| (std::cmp::Reverse(title_overlap(&r.entry, prompt)), !is_local(&r.entry, area)));
    kept.truncate(max_hints);
    kept
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{Entry, ScoredEntry};

    fn scored(title: &str, kind: &str, score: f32) -> ScoredEntry {
        ScoredEntry {
            entry: Entry {
                id: format!("{title}-{score}"),
                kind: kind.to_string(),
                category: "events".to_string(),
                title: (!title.is_empty()).then(|| title.to_string()),
                body: String::new(),
                tags: vec![],
                meta: None,
                source: None,
                identity_key: None,
                tier: "working".to_string(),
                indexed: true,
                created_at: String::new(),
                updated_at: String::new(),
                expires_at: None,
                superseded_by: None,
                supersedes: None,
                related_to: None,
                hit_count: 0,
                recall_count: 0,
                recall_sessions: 0,
                last_recalled_at: None,
                heat_tier: None,
                summary_condensed: None,
                summary_keypoint: None,
                file_path: None,
                last_accessed_at: None,
                source_files: None,
            },
            score,
            fts_rank: None,
            vec_rank: None,
        }
    }

    #[test]
    fn select_hints_drops_below_floor_dedups_and_truncates() {
        let results = vec![
            scored("charter plan", "decision", 1.6),
            scored("charter plan", "decision", 1.5), // duplicate title+kind — dropped
            scored("charter plan", "insight", 1.4), // same title, different kind — kept
            scored("", "browser-screenshot", 1.3), // untitled — dropped
            scored("charter plan tail", "decision", MIN_HINT_SCORE - 0.01), // below floor — dropped
            scored("third charter plan", "decision", 0.9),
            scored("fourth charter plan", "decision", 0.8), // beyond max_hints — truncated
        ];
        let hints = select_hints(results, &Default::default(), &[], 3);
        let ids: Vec<_> = hints.iter().map(|h| (h.entry.title.clone().unwrap(), h.entry.kind.clone())).collect();
        assert_eq!(
            ids,
            vec![
                ("charter plan".to_string(), "decision".to_string()),
                ("charter plan".to_string(), "insight".to_string()),
                ("third charter plan".to_string(), "decision".to_string()),
            ]
        );
    }

    #[test]
    fn session_logs_are_never_hints_even_when_they_match() {
        let results = vec![
            scored("Session deploy runner notes", "session", 1.6),
            scored("Deploy runner retries", "insight", 1.0),
        ];
        let hints = select_hints(results, &Default::default(), &[], 3);
        assert_eq!(hints.iter().map(|h| h.entry.kind.as_str()).collect::<Vec<_>>(), vec!["insight"]);
    }

    #[test]
    fn hints_from_the_sessions_area_come_first() {
        assert_eq!(area_words("/home/me/projects/acme/builder/app"), vec!["builder"]);
        assert_eq!(area_words("/home/me/.config/fleet"), vec!["fleet"]);
        let home = std::env::var("HOME").unwrap_or_default();
        if !home.is_empty() {
            assert_eq!(area_words(&format!("{home}/work/builder/app")), vec!["builder"]);
            assert_eq!(area_words(&format!("{home}/notes")), vec!["notes"]);
        }
        let mut far = scored("Traffic idea for press outlets", "opportunity", 1.6);
        far.entry.source = Some("Marketing seat, ideas.md".to_string());
        let mut near = scored("Builder hover prefetch rule", "decision", 1.0);
        near.entry.source = Some("Klar Builder release session, releases.md".to_string());
        let mut tagged = scored("Builder import fidelity", "insight", 0.9);
        tagged.entry.tags = vec!["product:builder".to_string()];
        let hints = select_hints(vec![far, near, tagged], &Default::default(), &area_words("/x/acme/builder/app"), 2);
        let titles: Vec<_> = hints.iter().map(|h| h.entry.title.clone().unwrap()).collect();
        assert_eq!(titles, vec!["Builder hover prefetch rule", "Builder import fidelity"]);
        // No area: score order stands.
        let plain = select_hints(vec![scored("a b", "insight", 1.6), scored("c d", "insight", 1.0)], &Default::default(), &[], 3);
        assert_eq!(plain[0].entry.title.as_deref(), Some("a b"));
    }

    #[test]
    fn titles_holding_the_prompts_words_come_before_the_area_and_the_score() {
        let prompt = title_words("Make the Builder pass 30 briefs in the fidelity bench");
        assert!(prompt.contains("fidelity") && !prompt.contains("the"));
        let mut local = scored("Release train notes", "insight", 1.6);
        local.entry.source = Some("builder/app releases.md".to_string());
        let one = scored("Fidelity scorer weights", "insight", 1.2);
        let two = scored("Builder fidelity bench, run 17", "insight", 0.8);
        let hints = select_hints(vec![local, one, two], &prompt, &area_words("/x/acme/builder/app"), 3);
        let titles: Vec<_> = hints.iter().map(|h| h.entry.title.clone().unwrap()).collect();
        assert_eq!(titles, vec!["Builder fidelity bench, run 17", "Fidelity scorer weights", "Release train notes"]);
    }
}
