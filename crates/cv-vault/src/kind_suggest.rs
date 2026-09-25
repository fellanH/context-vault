//! "Did you mean" for kinds refused by a closed kind registry. Suggests, never
//! auto-corrects: a silent correction would file a record under a kind nobody chose.

/// Closest known kind by edit distance, if close enough to be a plausible typo.
pub fn nearest<'a>(kind: &str, known: &[&'a str]) -> Option<&'a str> {
    known
        .iter()
        .map(|k| (levenshtein(kind, k), *k))
        .filter(|(d, k)| *d <= 2.max(k.len() / 4))
        .min_by_key(|(d, _)| *d)
        .map(|(_, k)| k)
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut cur = vec![i + 1];
        for (j, cb) in b.iter().enumerate() {
            let cost = usize::from(ca != *cb);
            cur.push((prev[j] + cost).min(prev[j + 1] + 1).min(cur[j] + 1));
        }
        prev = cur;
    }
    prev[b.len()]
}

#[cfg(test)]
mod tests {
    use super::nearest;

    #[test]
    fn only_suggests_plausible_typos() {
        assert_eq!(nearest("reguest", &["request", "gotcha"]), Some("request"));
        assert_eq!(nearest("acme-corp", &["request", "gotcha"]), None);
    }
}
