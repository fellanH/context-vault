//! Filesystem record detection and change signatures.
use anyhow::Result;
pub(crate) fn is_markdown(entry: &walkdir::DirEntry) -> bool {
    entry.file_type().is_file() && entry.path().extension().is_some_and(|e| e == "md")
}
pub(crate) fn has_frontmatter(raw: &str) -> bool {
    raw.starts_with("---\n") || raw.starts_with("---\r\n")
}
pub(crate) fn signature(metadata: &std::fs::Metadata) -> Result<String> {
    let stamp = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        Ok(format!(
            "{stamp}:{}:{}:{}",
            metadata.len(),
            metadata.ctime(),
            metadata.ctime_nsec()
        ))
    }
    #[cfg(not(unix))]
    {
        Ok(format!("{stamp}:{}", metadata.len()))
    }
}
