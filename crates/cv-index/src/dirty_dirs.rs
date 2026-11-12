//! Directories whose signature changed since the last cycle are re-listed before the lap, so a
//! file saved, renamed or deleted there is searchable on the next catch-up, not a lap later (card 37).
use super::Scan;
use crate::{checkpoint::DirState, source_file::signature};
use anyhow::Result;
use std::{
    collections::{btree_map::Entry, HashSet},
    io::ErrorKind,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// Event-log directories hold tens of thousands of files and change all the time; re-listing
/// them every cycle would eat the budget, so the lap covers them.
const WATCH_MAX_FILES: u64 = 5_000;

#[derive(Default)]
struct Listing {
    files: Vec<(String, PathBuf)>,
    dirs: Vec<String>,
}

impl Scan<'_> {
    /// Returns the time spent; the lap's budget is reduced by it.
    pub(super) fn refresh_dirty_dirs(&mut self) -> Result<Duration> {
        let start = Instant::now();
        let mut queue: Vec<String> = self
            .state
            .dirs
            .iter()
            .filter(|(_, dir)| dir.files <= WATCH_MAX_FILES)
            .map(|(key, _)| key.clone())
            .collect();
        while let Some(key) = queue.pop() {
            if self.stop_for_budget(start) {
                break;
            }
            self.refresh_dir(&key, &mut queue, start)?;
        }
        Ok(start.elapsed())
    }

    /// The lap records each directory it enters, once; later changes are this pass's job.
    pub(super) fn note_dir(&mut self, entry: &walkdir::DirEntry) -> Result<()> {
        let key = self.key(entry.path())?;
        if let Entry::Vacant(slot) = self.state.dirs.entry(key) {
            slot.insert(DirState {
                signature: signature(&entry.metadata()?)?,
                files: 0,
            });
        }
        Ok(())
    }

    fn key(&self, path: &Path) -> Result<String> {
        Ok(path
            .strip_prefix(&self.index.root)?
            .to_string_lossy()
            .into_owned())
    }

    fn refresh_dir(&mut self, key: &str, queue: &mut Vec<String>, start: Instant) -> Result<()> {
        let path = self.index.root.join(key);
        // Read the signature before listing: a file added during the listing changes it again.
        let Some(metadata) = dir_metadata(&path)? else {
            self.forget_dir(key);
            return Ok(());
        };
        let signature = signature(&metadata)?;
        if self
            .state
            .dirs
            .get(key)
            .is_some_and(|dir| dir.signature == signature)
        {
            return Ok(());
        }
        let listing = match list(&path, key) {
            Ok(listing) => listing,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                self.forget_dir(key);
                return Ok(());
            }
            Err(error) => return Err(error.into()),
        };
        self.report.dirs_relisted += 1;
        let files = listing.files.len() as u64;
        if files <= WATCH_MAX_FILES && !self.apply(key, listing, queue, start)? {
            // Out of budget part-way: keep the old signature so the next cycle finishes it.
            return Ok(());
        }
        self.state
            .dirs
            .insert(key.to_owned(), DirState { signature, files });
        Ok(())
    }

    fn apply(
        &mut self,
        key: &str,
        listing: Listing,
        queue: &mut Vec<String>,
        start: Instant,
    ) -> Result<bool> {
        for dir in listing.dirs {
            if let Entry::Vacant(slot) = self.state.dirs.entry(dir.clone()) {
                slot.insert(DirState::default());
                queue.push(dir);
            }
        }
        let present: HashSet<&str> = listing
            .files
            .iter()
            .map(|(file, _)| file.as_str())
            .collect();
        let gone: Vec<String> = children(&self.state.files, key)
            .chain(children(&self.state.invalid, key))
            .filter(|file| !present.contains(file.as_str()))
            .collect();
        for file in gone {
            self.forget(&file);
            self.state.invalid.remove(&file);
        }
        for (file, path) in listing.files {
            if self.stop_for_budget(start) {
                return Ok(false);
            }
            match std::fs::symlink_metadata(&path) {
                Ok(metadata) => self.process(&path, &metadata, file)?,
                Err(error) if error.kind() == ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        Ok(true)
    }

    fn forget_dir(&mut self, key: &str) {
        let prefix = format!("{key}/");
        let under = |path: &String| path.starts_with(&prefix);
        let files: Vec<String> = self
            .state
            .files
            .keys()
            .filter(|k| under(k))
            .cloned()
            .collect();
        for file in files {
            self.forget(&file);
        }
        self.state.invalid.retain(|path, _| !under(path));
        self.state
            .dirs
            .retain(|path, _| path != key && !under(path));
    }
}

/// Known files directly inside `dir` (not in its subdirectories).
fn children<'a, V>(
    known: &'a std::collections::BTreeMap<String, V>,
    dir: &'a str,
) -> impl Iterator<Item = String> + 'a {
    let prefix = if dir.is_empty() {
        String::new()
    } else {
        format!("{dir}/")
    };
    known
        .range(prefix.clone()..)
        .map(|(path, _)| path)
        .take_while(move |path| path.starts_with(&prefix))
        .filter(move |path| Path::new(path.as_str()).parent() == Some(Path::new(dir)))
        .cloned()
}

/// None when the directory is gone or is no longer a directory.
fn dir_metadata(path: &Path) -> std::io::Result<Option<std::fs::Metadata>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) => Ok(metadata.is_dir().then_some(metadata)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

fn list(path: &Path, key: &str) -> std::io::Result<Listing> {
    let mut listing = Listing::default();
    for entry in std::fs::read_dir(path)? {
        listing.add(key, &entry?)?;
    }
    Ok(listing)
}

impl Listing {
    fn add(&mut self, key: &str, entry: &std::fs::DirEntry) -> std::io::Result<()> {
        let kind = entry.file_type()?;
        let child = Path::new(key)
            .join(entry.file_name())
            .to_string_lossy()
            .into_owned();
        if kind.is_dir() {
            self.dirs.push(child);
        } else if kind.is_file() && entry.path().extension().is_some_and(|e| e == "md") {
            self.files.push((child, entry.path()));
        }
        Ok(())
    }
}

#[cfg(test)]
mod dirty_dir_tests {
    use crate::fixtures::{fixture, markdown, update};
    use crate::{SearchOptions, VaultIndex};
    use cv_core::VaultType;

    /// Put the lap's cursor at `key`, as if earlier budgeted cycles had walked that far.
    fn park_cursor_at(index: &VaultIndex, key: &str) {
        let mut state = index.checkpoint().unwrap();
        state.scan_after = Some(key.into());
        let mut writer: crate::update::Writer =
            index.index.writer_with_num_threads(1, 15_000_000).unwrap();
        let mut commit = writer.prepare_commit().unwrap();
        commit.set_payload(&serde_json::to_string(&state).unwrap());
        commit.commit().unwrap();
    }

    #[test]
    fn saves_behind_the_lap_cursor_are_searchable_on_the_next_cycle() {
        let (_temp, root, index) = fixture(VaultType::Knowledge);
        let notes = root.join("aaa");
        std::fs::create_dir(&notes).unwrap();
        markdown(&notes, "old", "alpha", "{}");
        markdown(&root, "zzz", "omega", "{}");
        update(&index);
        // The lap resumes after zzz.md and never walks aaa/ this cycle.
        park_cursor_at(&index, "zzz.md");
        markdown(&notes, "fresh", "webhook retry", "{}");
        std::fs::create_dir(notes.join("sub")).unwrap();
        markdown(&notes.join("sub"), "nested", "nested note", "{}");
        let report = update(&index);
        assert_eq!(report.indexed, 2);
        assert_eq!(report.dirs_relisted, 2, "aaa/ changed, aaa/sub is new");
        let found = |query: &str| {
            index
                .search(query, &SearchOptions::default())
                .unwrap()
                .len()
        };
        assert_eq!(found("webhook"), 1);
        assert_eq!(found("nested"), 1);
        park_cursor_at(&index, "zzz.md");
        std::fs::remove_file(notes.join("old.md")).unwrap();
        std::fs::remove_dir_all(notes.join("sub")).unwrap();
        assert_eq!(update(&index).removed, 2);
        assert_eq!(found("alpha"), 0);
        assert_eq!(found("nested"), 0);
        let unchanged = update(&index);
        assert_eq!((unchanged.indexed, unchanged.dirs_relisted), (0, 0));
    }
}
