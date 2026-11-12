//! Where a scan may skip: subtrees before the saved cursor, and exhaust (the data vault's).
use std::path::Path;

/// True when `path` and everything under it come before `cursor` in walk order
/// (component-wise, as walkdir's sort_by_file_name visits them).
pub(crate) fn before_cursor(path: &Path, cursor: &Path) -> bool {
    path < cursor && !cursor.starts_with(path)
}

/// Exhaust (events/, rule-candidate) belongs to the data vault migrate copies it to; the
/// originals that stay must not be searched twice, and pruning them keeps laps short.
pub(crate) fn keep_walking(root: &Path, path: &Path, resume: Option<&Path>) -> bool {
    let exhaust = path.strip_prefix(root).is_ok_and(|rel| rel.starts_with("events") || crate::migrate::is_exhaust(rel));
    !exhaust && !resume.is_some_and(|cursor| before_cursor(path, cursor))
}

#[cfg(test)]
mod cursor_tests {
    use super::before_cursor;
    use std::path::Path;

    #[test]
    fn subtrees_wholly_before_the_cursor_are_pruned_and_its_ancestors_are_not() {
        let cursor = Path::new("/v/knowledge/insight/m.md");
        assert!(before_cursor(Path::new("/v/entities"), cursor));
        assert!(before_cursor(Path::new("/v/knowledge/decision"), cursor));
        assert!(before_cursor(
            Path::new("/v/knowledge/insight/a.md"),
            cursor
        ));
        assert!(!before_cursor(Path::new("/v"), cursor));
        assert!(!before_cursor(Path::new("/v/knowledge"), cursor));
        assert!(!before_cursor(cursor, cursor));
        assert!(!before_cursor(
            Path::new("/v/knowledge/insight/z.md"),
            cursor
        ));
        assert!(!before_cursor(Path::new("/v/knowledge/pattern"), cursor));
        // Component order, not string order: as a string "a-b" < "a/x.md", but the walk
        // visits directory "a" before "a-b", so "a-b" is after the cursor.
        assert!(!before_cursor(Path::new("/v/a-b"), Path::new("/v/a/x.md")));
    }
}
