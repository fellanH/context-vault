//! HTML tag → Markdown formatting passes used by `html_to_markdown`.

use regex::Regex;

use super::{decode_entities, strip_tags};

pub(super) fn remove_container(md: &str, tag: &str) -> String {
    let pattern = format!(r"(?i)<{0}[\s\S]*?</{0}>", tag);
    Regex::new(&pattern)
        .unwrap()
        .replace_all(md, "")
        .to_string()
}

pub(super) fn apply_heading_levels(mut md: String) -> String {
    for level in 1..=6 {
        let pattern = format!(r"(?i)<h{0}[^>]*>([\s\S]*?)</h{0}>", level);
        let hashes = "#".repeat(level);
        md = Regex::new(&pattern)
            .unwrap()
            .replace_all(&md, |caps: &regex::Captures| {
                format!("\n{} {}\n", hashes, strip_tags(&caps[1]).trim())
            })
            .to_string();
    }
    md
}

pub(super) fn apply_link_formatting(md: &str) -> String {
    Regex::new(r#"(?i)<a[^>]*href="([^"]*)"[^>]*>([\s\S]*?)</a>"#)
        .unwrap()
        .replace_all(md, |caps: &regex::Captures| {
            let text_str = strip_tags(&caps[2]);
            let text = text_str.trim();
            if text.is_empty() {
                String::new()
            } else {
                format!("[{}]({})", text, &caps[1])
            }
        })
        .to_string()
}

pub(super) fn apply_code_formatting(md: &str) -> String {
    let md = Regex::new(r"(?i)<pre[^>]*><code[^>]*>([\s\S]*?)</code></pre>")
        .unwrap()
        .replace_all(md, |caps: &regex::Captures| {
            format!("\n```\n{}\n```\n", decode_entities(&caps[1]).trim())
        })
        .to_string();

    let md = Regex::new(r"(?i)<pre[^>]*>([\s\S]*?)</pre>")
        .unwrap()
        .replace_all(&md, |caps: &regex::Captures| {
            format!(
                "\n```\n{}\n```\n",
                decode_entities(&strip_tags(&caps[1])).trim()
            )
        })
        .to_string();

    Regex::new(r"(?i)<code[^>]*>([\s\S]*?)</code>")
        .unwrap()
        .replace_all(&md, |caps: &regex::Captures| {
            format!("`{}`", decode_entities(&caps[1]).trim())
        })
        .to_string()
}

pub(super) fn apply_emphasis_formatting(md: &str) -> String {
    let md = Regex::new(r"(?i)<strong[^>]*>([\s\S]*?)</strong>")
        .unwrap()
        .replace_all(md, |caps: &regex::Captures| {
            format!("**{}**", strip_tags(&caps[1]).trim())
        })
        .to_string();

    let md = Regex::new(r"(?i)<b[^>]*>([\s\S]*?)</b>")
        .unwrap()
        .replace_all(&md, |caps: &regex::Captures| {
            format!("**{}**", strip_tags(&caps[1]).trim())
        })
        .to_string();

    let md = Regex::new(r"(?i)<em[^>]*>([\s\S]*?)</em>")
        .unwrap()
        .replace_all(&md, |caps: &regex::Captures| {
            format!("*{}*", strip_tags(&caps[1]).trim())
        })
        .to_string();

    Regex::new(r"(?i)<i[^>]*>([\s\S]*?)</i>")
        .unwrap()
        .replace_all(&md, |caps: &regex::Captures| {
            format!("*{}*", strip_tags(&caps[1]).trim())
        })
        .to_string()
}

pub(super) fn apply_block_formatting(md: &str) -> String {
    let md = Regex::new(r"(?i)<li[^>]*>([\s\S]*?)</li>")
        .unwrap()
        .replace_all(md, |caps: &regex::Captures| {
            format!("- {}\n", strip_tags(&caps[1]).trim())
        })
        .to_string();

    let md = Regex::new(r"(?i)<br\s*/?>\s*")
        .unwrap()
        .replace_all(&md, "\n")
        .to_string();

    let md = Regex::new(r"(?i)<p[^>]*>([\s\S]*?)</p>")
        .unwrap()
        .replace_all(&md, |caps: &regex::Captures| {
            format!("\n{}\n", strip_tags(&caps[1]).trim())
        })
        .to_string();

    Regex::new(r"(?i)<blockquote[^>]*>([\s\S]*?)</blockquote>")
        .unwrap()
        .replace_all(&md, |caps: &regex::Captures| {
            let content = strip_tags(&caps[1]);
            let quoted = content
                .lines()
                .map(|line| format!("> {}", line))
                .collect::<Vec<_>>()
                .join("\n");
            format!("\n{}\n", quoted)
        })
        .to_string()
}
