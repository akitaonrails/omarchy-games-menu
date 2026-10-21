//! Single-instance guard for `ogm refresh`.
//!
//! The QML overlay hands refresh off to a detached process, so a second
//! overlay launch can race the first one's refresh. Two refreshes writing
//! state.json would not corrupt it (writes are atomic) but the loser's
//! covers would be dropped by last-writer-wins, so only one may run.

use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use crate::paths::Paths;
use crate::StoreError;

/// Held for the lifetime of a refresh; removes the lock file on drop.
#[derive(Debug)]
pub struct RefreshLock {
    path: PathBuf,
}

/// A lock file is stale when nothing is running under the pid it records —
/// a refresh killed before it could clean up. Unreadable contents count as
/// stale; the worst case is one refresh skipped, which the next run repairs.
fn is_stale(recorded: Option<u32>, holder_alive: bool) -> bool {
    match recorded {
        None => true,
        Some(pid) => pid != std::process::id() && !holder_alive,
    }
}

fn process_exists(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}

impl RefreshLock {
    /// `Ok(None)` when another live refresh holds the lock. A lock left by a
    /// dead process is taken over.
    pub fn acquire(paths: &Paths) -> Result<Option<RefreshLock>, StoreError> {
        let path = paths.refresh_lock_file();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| StoreError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        match Self::try_create(&path) {
            Ok(lock) => Ok(Some(lock)),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => {
                let recorded = fs::read_to_string(&path)
                    .ok()
                    .and_then(|s| s.trim().parse::<u32>().ok());
                if !is_stale(recorded, recorded.is_some_and(process_exists)) {
                    return Ok(None);
                }
                // Stale: drop it and take the lock. Losing a second race here
                // just means the other refresh runs instead of this one.
                let _ = fs::remove_file(&path);
                match Self::try_create(&path) {
                    Ok(lock) => Ok(Some(lock)),
                    Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(None),
                    Err(e) => Err(StoreError::Io { path, source: e }),
                }
            }
            Err(e) => Err(StoreError::Io { path, source: e }),
        }
    }

    fn try_create(path: &Path) -> std::io::Result<RefreshLock> {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)?;
        write!(f, "{}", std::process::id())?;
        f.sync_all()?;
        Ok(RefreshLock {
            path: path.to_path_buf(),
        })
    }
}

impl Drop for RefreshLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_when_no_pid_recorded_or_holder_gone() {
        assert!(is_stale(None, false));
        assert!(is_stale(Some(4242), false));
    }

    #[test]
    fn live_holder_is_not_stale() {
        assert!(!is_stale(Some(4242), true));
        // Our own pid holds it, whether or not /proc was consulted.
        assert!(!is_stale(Some(std::process::id()), false));
    }

    #[test]
    fn this_process_is_detected_as_alive() {
        assert!(process_exists(std::process::id()));
        assert!(!process_exists(0));
    }

    #[test]
    fn acquire_creates_lock_and_drop_removes_it() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        let lock = RefreshLock::acquire(&paths).unwrap();
        assert!(lock.is_some());
        assert!(paths.refresh_lock_file().exists());
        drop(lock);
        assert!(!paths.refresh_lock_file().exists());
    }

    #[test]
    fn second_acquire_by_this_live_process_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        let _held = RefreshLock::acquire(&paths).unwrap().unwrap();
        // The file records our own pid, which is alive, so the lock stands.
        assert!(RefreshLock::acquire(&paths).unwrap().is_none());
    }

    #[test]
    fn lock_left_by_a_dead_process_is_taken_over() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        let path = paths.refresh_lock_file();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        // A pid that cannot be running: /proc/0 never exists.
        fs::write(&path, "0").unwrap();
        let lock = RefreshLock::acquire(&paths).unwrap();
        assert!(lock.is_some());
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            std::process::id().to_string()
        );
    }

    #[test]
    fn garbage_lock_contents_are_treated_as_stale() {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths::from_root(dir.path());
        let path = paths.refresh_lock_file();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, "not-a-pid").unwrap();
        assert!(RefreshLock::acquire(&paths).unwrap().is_some());
    }
}
