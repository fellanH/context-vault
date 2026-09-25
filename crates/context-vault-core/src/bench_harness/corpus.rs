use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Context;
use serde::Serialize;

use super::DiskMode;

#[derive(Serialize)]
pub struct CorpusSummary {
    pub entries_written: u32,
    pub root: PathBuf,
    pub layout: &'static str,
    pub disk_mode: DiskMode,
}

pub fn generate_corpus(
    corpus_root: Option<PathBuf>,
    seed: u64,
    entries: u32,
    disk_mode: DiskMode,
) -> anyhow::Result<CorpusSummary> {
    let root = match corpus_root {
        Some(p) => {
            fs::create_dir_all(&p).with_context(|| format!("create_dir_all {}", p.display()))?;
            p
        }
        None => {
            let base = std::env::temp_dir().join("context-vault-bench-corpus");
            let p = base.join(format!("run-{seed}"));
            let _ = fs::remove_dir_all(&p);
            fs::create_dir_all(&p).with_context(|| format!("create_dir_all {}", p.display()))?;
            p
        }
    };

    let decisions = root.join("knowledge").join("decisions");
    fs::create_dir_all(&decisions).context("mkdir knowledge/decisions")?;

    for i in 0..entries {
        let (id, _title, body) = vault_seed_row(seed, i);
        let path = decisions.join(format!("{id}.md"));
        let mut f = fs::File::create(&path).with_context(|| format!("create {}", path.display()))?;
        f.write_all(body.as_bytes())
            .with_context(|| format!("write {}", path.display()))?;
        if matches!(disk_mode, DiskMode::B | DiskMode::C) {
            f.sync_all().ok();
        }
    }

    Ok(CorpusSummary {
        entries_written: entries,
        root,
        layout: "knowledge/decisions/<id>.md",
        disk_mode,
    })
}

fn deterministic_u64(seed: u64, i: u32) -> u64 {
    let mut x = seed ^ 0x9E37_79B9_7F4A_7C15;
    x ^= u64::from(i).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x ^= x >> 27;
    x.wrapping_mul(0xC2B2_AE3D_27D4_B4F3)
}

fn format_title(h: u64) -> String {
    let adj = ADJECTIVES[(h as usize) % ADJECTIVES.len()];
    let noun = NOUNS[(h >> 8) as usize % NOUNS.len()];
    format!("{adj} {noun} ({h:x}) hybrid")
}

fn format_body(seed: u64, i: u32, title: &str) -> String {
    format!(
        "---\nid: bench-{i:05}\nkind: decision\ntags: bench, generated\nseed: {seed}\n---\n\n# {title}\n\nStub corpus entry for repeatable bench runs.\n"
    )
}

static ADJECTIVES: &[&str] = &[
    "incremental",
    "durable",
    "hybrid",
    "scoped",
    "typed",
    "local",
    "semantic",
    "bounded",
];

static NOUNS: &[&str] = &[
    "retrieval",
    "indexing",
    "migration",
    "embedding",
    "session",
    "vault",
    "lane",
    "fixture",
];

pub(crate) fn vault_seed_row(seed: u64, i: u32) -> (String, String, String) {
    let id = format!("bench-{i:05}");
    let title = format_title(deterministic_u64(seed, i));
    let body = format_body(seed, i, &title);
    (id, title, body)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    fn collect_md(dir: &Path) -> Vec<(PathBuf, String)> {
        let mut v = Vec::new();
        fn walk(dir: &Path, v: &mut Vec<(PathBuf, String)>) {
            let Ok(rd) = fs::read_dir(dir) else {
                return;
            };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() {
                    walk(&p, v);
                } else if p.extension().and_then(|s| s.to_str()) == Some("md") {
                    v.push((p.clone(), fs::read_to_string(&p).unwrap()));
                }
            }
        }
        walk(dir, &mut v);
        v.sort_by(|a, b| a.0.cmp(&b.0));
        v
    }

    #[test]
    fn corpus_is_repeatable() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a");
        let b = tmp.path().join("b");
        generate_corpus(Some(a.clone()), 7, 10, DiskMode::A).unwrap();
        generate_corpus(Some(b.clone()), 7, 10, DiskMode::A).unwrap();
        let fa = collect_md(&a);
        let fb = collect_md(&b);
        assert_eq!(fa.len(), fb.len());
        for ((pa, ca), (pb, cb)) in fa.iter().zip(fb.iter()) {
            assert_eq!(pa.file_name(), pb.file_name());
            assert_eq!(ca, cb);
        }
    }
}
