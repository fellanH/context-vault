use super::*;
use crate::{UpdateOptions, UpdateReport};
pub(crate) fn fixture(kind: VaultType) -> (tempfile::TempDir, PathBuf, VaultIndex) {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("source");
    std::fs::create_dir(&root).unwrap();
    let index = VaultIndex::open(&root, kind, &temp.path().join("index")).unwrap();
    (temp, root, index)
}
pub(crate) fn update(index: &VaultIndex) -> UpdateReport {
    index
        .update(&UpdateOptions {
            budget: None,
            ..Default::default()
        })
        .unwrap()
}
pub(crate) fn markdown(root: &Path, id: &str, text: &str, meta: &str) -> PathBuf {
    let path = root.join(format!("{id}.md"));
    let metadata: serde_json::Map<String, serde_json::Value> = serde_json::from_str(meta).unwrap();
    let metadata = metadata
        .iter()
        .map(|(key, value)| format!("{key}: {value}"))
        .collect::<Vec<_>>()
        .join("\n");
    std::fs::write(&path, format!("---\nid: {id}\nkind: gotcha\ncreated: 2026-10-12T08:00:00Z\ntitle: Record {id}\ntags: [integration]\n{metadata}\n---\n{text}\n")).unwrap();
    path
}
