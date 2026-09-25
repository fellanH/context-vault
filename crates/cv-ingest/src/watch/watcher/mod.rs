use anyhow::Result;
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

mod lock;
mod self_write;

use lock::WatcherLock;
use self_write::SelfWriteTracker;

const SELF_WRITE_TTL_MS: u64 = 2_000;

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn is_excluded(rel_path: &str) -> bool {
    if rel_path.starts_with('.') {
        return true;
    }

    let excluded_dirs = ["_archive", "projects"];
    rel_path
        .split('/')
        .any(|part| part.starts_with('.') || excluded_dirs.contains(&part))
}

pub struct VaultWatcher {
    _lock: WatcherLock,
    self_writes: Arc<Mutex<SelfWriteTracker>>,
}

impl VaultWatcher {
    pub fn mark_self_write(&self, file_path: String) {
        if let Ok(mut tracker) = self.self_writes.lock() {
            tracker.mark(file_path);
        }
    }

    pub fn should_process_event(&self, rel_path: &str) -> bool {
        if is_excluded(rel_path) {
            return false;
        }

        if let Ok(mut tracker) = self.self_writes.lock() {
            if tracker.is_self_write(rel_path) {
                return false;
            }
        }

        true
    }
}

pub fn start_watcher(_vault_dir: &str, _debounce_ms: u64, data_dir: &str) -> Result<VaultWatcher> {
    let lock = WatcherLock::acquire(data_dir)?;

    Ok(VaultWatcher {
        _lock: lock,
        self_writes: Arc::new(Mutex::new(SelfWriteTracker::new())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_excluded() {
        assert!(is_excluded("_archive/file.md"));
        assert!(is_excluded(".hidden/file.md"));
        assert!(is_excluded("projects/file.md"));
        assert!(!is_excluded("reference/file.md"));
    }

    #[test]
    fn test_current_time_ms() {
        let t1 = current_time_ms();
        let t2 = current_time_ms();
        assert!(t2 >= t1);
    }
}
