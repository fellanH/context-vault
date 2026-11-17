//! Turn arbitrary user text into safe FTS5 MATCH expressions: an all-words query first and
//! an any-word fallback, both with filler words left out.

/// Words a plain-language question carries that say nothing about which entry is meant.
/// FTS5 ANDs every token, so "what is our staging bucket called?" found nothing while
/// "staging bucket" found the entry (OPS-288). They are dropped only when at least one
/// other word remains, so a search for "the" alone still searches.
const STOP_WORDS: &[&str] = &[
    "a", "about", "an", "and", "any", "are", "as", "at", "be", "been", "but", "by", "called",
    "can", "could", "did", "do", "does", "for", "from", "had", "has", "have", "how", "i", "in",
    "into", "is", "it", "its", "me", "my", "named", "of", "on", "or", "our", "please", "should",
    "tell", "that", "the", "their", "there", "these", "this", "those", "to", "us", "was", "we",
    "were", "what", "whats", "when", "where", "which", "who", "why", "will", "with", "would",
    "you", "your",
];

fn is_stop_word(tok: &str) -> bool {
    let word: String = tok.chars().filter(|c| c.is_alphanumeric()).collect::<String>().to_lowercase();
    STOP_WORDS.contains(&word.as_str())
}

/// The searchable tokens of arbitrary user text, each wrapped as a literal FTS5 phrase.
/// FTS5 parses `-`, `:`, `"`, `*`, `(`, and `AND/OR/NOT/NEAR` as operators, so a bareword
/// like `session-end` or a stray `:` raises `no such column` / syntax errors. Wrapping each
/// whitespace token in a double-quoted phrase (internal quotes doubled) makes FTS5 treat it
/// as a literal string. Tokens with no alphanumeric char are dropped, then stop words, unless
/// that would leave nothing. Empty when nothing searchable remains.
fn fts_phrases(query: &str) -> Vec<String> {
    let tokens: Vec<&str> =
        query.split_whitespace().filter(|tok| tok.chars().any(|c| c.is_alphanumeric())).collect();
    let content: Vec<&str> = tokens.iter().copied().filter(|t| !is_stop_word(t)).collect();
    let kept = if content.is_empty() { tokens } else { content };
    kept.iter().map(|tok| format!("\"{}\"", tok.replace('"', "\"\""))).collect()
}

/// Every searchable word must match (FTS5's implicit AND). `None` when nothing searchable remains.
pub(crate) fn all_words(query: &str) -> Option<String> {
    let phrases = fts_phrases(query);
    (!phrases.is_empty()).then(|| phrases.join(" "))
}

/// Any searchable word may match, ranked by bm25 so entries matching more words come first.
/// The fallback when the all-words query finds nothing; `None` when it would be the same query.
pub(crate) fn any_word(query: &str) -> Option<String> {
    let phrases = fts_phrases(query);
    (phrases.len() > 1).then(|| phrases.join(" OR "))
}

#[cfg(test)]
mod tests {
    use super::{all_words, any_word};

    #[test]
    fn quotes_tokens_and_neutralizes_fts_operators() {
        // The pre-fix crash case: `session-end` was parsed as an FTS operator expression.
        assert_eq!(
            all_words("session-end transcript parser").as_deref(),
            Some(r#""session-end" "transcript" "parser""#)
        );
        // A stray colon (column-filter syntax) is neutralized too.
        assert_eq!(all_words("bucket:omni foo").as_deref(), Some(r#""bucket:omni" "foo""#));
    }

    #[test]
    fn drops_punctuation_only_tokens_and_empties() {
        assert_eq!(all_words("   ").as_deref(), None);
        assert_eq!(all_words("- : *").as_deref(), None);
        // Embedded double-quote is doubled so the phrase stays well-formed.
        assert_eq!(all_words(r#"a"b"#).as_deref(), Some(r#""a""b""#));
    }

    #[test]
    fn a_plain_question_keeps_only_its_content_words() {
        // OPS-288: the README's demo question ANDed "what", "is", "our" and "called" too.
        assert_eq!(all_words("what is our staging bucket called?").as_deref(), Some(r#""staging" "bucket""#));
        assert_eq!(any_word("what is our staging bucket called?").as_deref(), Some(r#""staging" OR "bucket""#));
    }

    #[test]
    fn stop_words_alone_still_search() {
        assert_eq!(all_words("what is it").as_deref(), Some(r#""what" "is" "it""#));
        // One word has no any-word fallback: it would be the same query.
        assert_eq!(any_word("bucket?").as_deref(), None);
    }
}
