//! What a full index run would read (card 34): files and bytes, from a stat-only walk.
use crate::VaultIndex;
use anyhow::Result;
use cv_core::VaultType;
use serde::Serialize;

#[derive(Debug, Default, Serialize)]
pub struct Plan {
    pub files: u64,
    pub bytes: u64,
}

impl VaultIndex {
    /// Knowledge: the markdown files the index reads (exhaust pruned); data: segment files
    /// as stored (sealed ones compressed, which the measured rate already accounts for).
    pub fn plan(&self) -> Result<Plan> {
        let sizes: Vec<u64> = match self.kind {
            VaultType::Knowledge => self.markdown_sizes(),
            VaultType::Data => cv_core::segment_files(&self.root)?.iter().map(|s| file_len(&s.path)).collect(),
        };
        Ok(Plan { files: sizes.len() as u64, bytes: sizes.iter().sum() })
    }

    fn markdown_sizes(&self) -> Vec<u64> {
        let root = self.root.clone();
        let walk = walkdir::WalkDir::new(&self.root).into_iter().filter_entry(move |e| crate::scan_cursor::keep_walking(&root, e.path(), None));
        walk.filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x == "md"))
            .map(|e| file_len(e.path()))
            .collect()
    }
}

fn file_len(path: &std::path::Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}
