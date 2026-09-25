//! Tracks paths this process just wrote, so its own file-watch events can be
//! distinguished from external edits and ignored for a short TTL.

use std::collections::HashMap;

use super::{current_time_ms, SELF_WRITE_TTL_MS};

#[derive(Debug)]
pub(super) struct SelfWriteTracker {
    paths: HashMap<String, u64>,
}

impl SelfWriteTracker {
    pub(super) fn new() -> Self {
        SelfWriteTracker {
            paths: HashMap::new(),
        }
    }

    pub(super) fn mark(&mut self, path: String) {
        let expiry = current_time_ms() + SELF_WRITE_TTL_MS;
        self.paths.insert(path, expiry);
    }

    pub(super) fn is_self_write(&mut self, path: &str) -> bool {
        let now = current_time_ms();
        let expiry = self.paths.get(path).copied();

        if let Some(exp) = expiry {
            if now < exp {
                self.paths.remove(path);
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_self_write_tracker() {
        let mut tracker = SelfWriteTracker::new();
        tracker.mark("test.md".to_string());
        assert!(tracker.is_self_write("test.md"));
        assert!(!tracker.is_self_write("other.md"));
    }
}
