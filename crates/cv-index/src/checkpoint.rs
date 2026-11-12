use crate::VaultIndex;
use anyhow::{bail, Result};
use cv_core::VaultType;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct FileState {
    pub signature: String,
    pub id: String,
}

/// A directory's own change signature: a file saved, renamed or deleted in it changes this.
#[derive(Clone, Default, Serialize, Deserialize)]
pub(crate) struct DirState {
    pub signature: String,
    /// Markdown files seen at the last listing; big directories are left to the lap.
    pub files: u64,
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Checkpoint {
    pub version: u32,
    pub root: String,
    pub kind: VaultType,
    #[serde(default)]
    pub scan_after: Option<String>,
    #[serde(default)]
    pub sealed: std::collections::BTreeSet<String>,
    pub files: BTreeMap<String, FileState>,
    /// Files that are not records (no id, bad fields), by signature: not re-read until they change.
    #[serde(default)]
    pub invalid: BTreeMap<String, String>,
    #[serde(default)]
    pub dirs: BTreeMap<String, DirState>,
    pub segments: BTreeMap<String, u64>,
}

impl VaultIndex {
    pub(crate) fn checkpoint(&self) -> Result<Checkpoint> {
        let payload = self.index.load_metas()?.payload;
        let state = match payload {
            Some(raw) => serde_json::from_str::<Checkpoint>(&raw)?,
            None => Checkpoint {
                version: 1,
                root: self.root.to_string_lossy().into_owned(),
                kind: self.kind,
                scan_after: None,
                sealed: Default::default(),
                files: BTreeMap::new(),
                invalid: BTreeMap::new(),
                dirs: BTreeMap::new(),
                segments: BTreeMap::new(),
            },
        };
        if state.version != 1
            || state.root != self.root.to_string_lossy()
            || state.kind != self.kind
        {
            bail!("checkpoint belongs to another vault or version; use a separate index directory");
        }
        Ok(state)
    }
}
