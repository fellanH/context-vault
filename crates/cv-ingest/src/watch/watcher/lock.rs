//! Single-instance file lock for the vault watcher (PID + timestamp, stale-checked).

use anyhow::{anyhow, Result};
use std::fs;
use std::path::PathBuf;

use super::current_time_ms;

pub(super) struct WatcherLock {
    path: PathBuf,
}

impl WatcherLock {
    pub(super) fn acquire(data_dir: &str) -> Result<Self> {
        fs::create_dir_all(data_dir).ok();
        let lock_path = PathBuf::from(data_dir).join("watcher.lock");

        if check_existing_lock(&lock_path)? {
            return Err(anyhow!(
                "Another instance is already watching this vault. Skipping watcher."
            ));
        }

        write_lock(&lock_path)?;
        Ok(WatcherLock { path: lock_path })
    }
}

impl Drop for WatcherLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

fn check_existing_lock(lock_path: &PathBuf) -> Result<bool> {
    let content = match fs::read_to_string(lock_path) {
        Ok(c) => c,
        Err(_) => return Ok(false),
    };

    let parts: Vec<&str> = content.trim().split(':').collect();
    if parts.len() != 2 {
        return Ok(false);
    }

    let (Ok(pid), Ok(ts)) = (parts[0].parse::<u32>(), parts[1].parse::<u64>()) else {
        return Ok(false);
    };

    if is_lock_stale(ts) {
        return Ok(false);
    }

    if is_process_alive(pid) {
        return Ok(true);
    }

    Ok(false)
}

fn is_lock_stale(ts: u64) -> bool {
    current_time_ms() - ts > 60_000
}

fn is_process_alive(pid: u32) -> bool {
    #[cfg(unix)]
    {
        let pid_str = pid.to_string();
        std::process::Command::new("kill")
            .args(["-0", &pid_str])
            .output()
            .map(|status| status.status.success())
            .unwrap_or(false)
    }

    #[cfg(not(unix))]
    {
        false
    }
}

fn write_lock(lock_path: &PathBuf) -> Result<()> {
    let pid = std::process::id();
    let ts = current_time_ms();
    fs::write(lock_path, format!("{}:{}", pid, ts))?;
    Ok(())
}
