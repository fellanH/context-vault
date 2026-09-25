use regex::Regex;

#[path = "url_convert_html.rs"]
mod html_format;
use html_format::{
    apply_block_formatting, apply_code_formatting, apply_emphasis_formatting,
    apply_heading_levels, apply_link_formatting, remove_container,
};

pub fn strip_tags(html: &str) -> String {
    Regex::new(r"<[^>]+>")
        .unwrap()
        .replace_all(html, "")
        .to_string()
}

pub fn decode_entities(text: &str) -> String {
    let mut result = text
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ");

    if let Ok(re) = Regex::new(r"&#(\d+);") {
        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                caps[1]
                    .parse::<u32>()
                    .ok()
                    .and_then(char::from_u32)
                    .map(|ch| ch.to_string())
                    .unwrap_or_else(|| caps[0].to_string())
            })
            .to_string();
    }

    if let Ok(re) = Regex::new(r"&#x([0-9a-f]+);") {
        result = re
            .replace_all(&result, |caps: &regex::Captures| {
                u32::from_str_radix(&caps[1], 16)
                    .ok()
                    .and_then(char::from_u32)
                    .map(|ch| ch.to_string())
                    .unwrap_or_else(|| caps[0].to_string())
            })
            .to_string();
    }

    result
}

pub fn html_to_markdown(html: &str) -> String {
    let mut md = html.to_string();

    // Remove unwanted containers
    md = remove_container(&md, "script");
    md = remove_container(&md, "style");

    for tag in &["nav", "header", "footer", "aside"] {
        md = remove_container(&md, tag);
    }

    // Apply formatting in order
    md = apply_heading_levels(md);
    md = apply_link_formatting(&md);
    md = apply_code_formatting(&md);
    md = apply_emphasis_formatting(&md);
    md = apply_block_formatting(&md);

    // Final cleanup
    md = strip_tags(&md);
    md = decode_entities(&md);

    Regex::new(r"\n{3,}")
        .unwrap()
        .replace_all(&md, "\n\n")
        .to_string()
        .trim()
        .to_string()
}

pub fn extract_html_content(html: &str, _url: &str) -> (String, String) {
    let title = Regex::new(r"(?i)<title[^>]*>([\s\S]*?)</title>")
        .unwrap()
        .captures(html)
        .map(|caps| strip_tags(&decode_entities(&caps[1])).trim().to_string())
        .unwrap_or_default();

    let content_html = find_content_section(html).unwrap_or_else(|| html.to_string());
    let body = html_to_markdown(&content_html);

    (title, body)
}

fn find_content_section(html: &str) -> Option<String> {
    const PATTERNS: [&str; 3] = [
        r"(?i)<article[^>]*>([\s\S]*?)</article>",
        r"(?i)<main[^>]*>([\s\S]*?)</main>",
        r"(?i)<body[^>]*>([\s\S]*?)</body>",
    ];

    PATTERNS.iter().find_map(|pattern| {
        Regex::new(pattern)
            .ok()?
            .captures(html)
            .map(|caps| caps[1].to_string())
    })
}

#[cfg(test)]
mod url_convert_tests {
    use super::*;

    #[test]
    fn test_strip_tags() {
        assert_eq!(strip_tags("<p>hello</p>"), "hello");
    }

    #[test]
    fn test_decode_entities() {
        assert_eq!(decode_entities("&amp;"), "&");
        assert_eq!(decode_entities("&lt;"), "<");
    }

    #[test]
    fn test_html_to_markdown_basic() {
        let html = "<p>Hello <strong>world</strong></p>";
        let md = html_to_markdown(html);
        assert!(md.to_lowercase().contains("hello"));
        assert!(md.contains("**world**"));
    }

    #[test]
    fn test_html_to_markdown_headings() {
        let html = "<h1>Title</h1><p>Content</p>";
        let md = html_to_markdown(html);
        assert!(md.contains("# Title"));
        assert!(md.to_lowercase().contains("content"));
    }
}
