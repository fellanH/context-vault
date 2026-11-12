//! Files this process just wrote or deleted are indexed first, whatever the budget: an
//! in-place edit changes no directory, so neither the changed-directory pass nor a lap
//! would see it soon (card 38).
use super::Scan;
use anyhow::Result;
use std::io::ErrorKind;

impl Scan<'_> {
    pub(super) fn refresh_written(&mut self) -> Result<()> {
        for path in &self.options.paths {
            let Ok(rel) = path.strip_prefix(&self.index.root) else { continue };
            let key = rel.to_string_lossy().into_owned();
            match std::fs::metadata(path) {
                Ok(metadata) if metadata.is_file() => self.process(path, &metadata, key)?,
                Ok(_) => {}
                Err(error) if error.kind() == ErrorKind::NotFound => self.forget(&key),
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }
}
