//! The personal-free policy: a data vault stores facts about organizations, products,
//! places and events, never about individuals. Fail closed: an unclassified record is
//! refused, not assumed non-personal. Checks the declared classification only; it
//! cannot see a person described in a body marked `public`.

use serde_json::{Map, Value};

/// Allowed `meta.sensitivity` values in a personal-free vault. `personal` is the
/// value the policy exists to refuse, so it is deliberately absent.
pub const SENSITIVITY_VALUES: &[&str] = &["public", "internal", "confidential"];

pub fn check_personal_free(meta: &Map<String, Value>, tags: &[String], problems: &mut Vec<String>) {
    check_sensitivity(meta.get("sensitivity").and_then(|v| v.as_str()), problems);

    let subject = meta.get("subject").and_then(|v| v.as_str());
    if subject.is_some_and(|s| s.eq_ignore_ascii_case("person")) {
        problems.push(
            "meta.subject = \"Person\": this vault is personal-free; record the \
             organization, product or event instead"
                .to_string(),
        );
    }

    for tag in tags.iter().filter(|t| t.to_ascii_lowercase().starts_with("person:")) {
        problems.push(format!("tag `{tag}` names a person: this vault is personal-free"));
    }
}

fn check_sensitivity(sensitivity: Option<&str>, problems: &mut Vec<String>) {
    let allowed = SENSITIVITY_VALUES.join(", ");
    match sensitivity {
        None => problems.push(format!(
            "meta.sensitivity is required in a personal-free vault (one of: {allowed})"
        )),
        Some(s) if s.eq_ignore_ascii_case("personal") => problems.push(
            "meta.sensitivity = \"personal\": this vault is personal-free; personal data is \
             not stored here"
                .to_string(),
        ),
        Some(s) if !SENSITIVITY_VALUES.contains(&s) => {
            problems.push(format!("meta.sensitivity = {s:?} is not one of: {allowed}"))
        }
        Some(_) => {}
    }
}
