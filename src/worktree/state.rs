//! State file read/write with advisory locking.
//!
//! Manages `.mr/worktrees/state.yaml` persistence with flock-based
//! advisory locking via `.mr/worktrees/state.lock` for safe concurrent access.

// State module is defined now but consumed by later tasks (T-003 .. T-018).
#![allow(dead_code)]

use std::fs::{self, File, OpenOptions};
use std::os::unix::io::AsRawFd;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use super::types::WorktreeState;

// ── Constants ───────────────────────────────────────────────────────

/// Directory name under `.mr/` for worktree state.
const WORKTREE_DIR: &str = "worktrees";

/// State file name.
const STATE_FILE: &str = "state.yaml";

/// Lock file name.
const LOCK_FILE: &str = "state.lock";

/// Temporary file used for atomic writes.
const STATE_TMP_FILE: &str = "state.yaml.tmp";

// ── Lock guard ──────────────────────────────────────────────────────

/// RAII guard that holds an advisory flock.
///
/// The lock is released automatically when the guard is dropped
/// (the underlying file descriptor is closed).
struct LockGuard {
    _file: File,
}

// ── State manager ───────────────────────────────────────────────────

/// Manages reading and writing of worktree orchestration state with
/// flock-based advisory locking.
///
/// All mutations to `state.yaml` should go through [`Self::modify`] to
/// ensure atomicity and mutual exclusion with other processes (daemon,
/// concurrent `mr wt` invocations).
pub struct StateManager {
    /// Root path to the `.mr/worktrees/` directory.
    dir: PathBuf,
}

impl StateManager {
    /// Create a new `StateManager` rooted at the given project root.
    ///
    /// The state directory is `<root>/.mr/worktrees/`.
    #[must_use]
    pub fn new(root: &Path) -> Self {
        Self {
            dir: root.join(".mr").join(WORKTREE_DIR),
        }
    }

    /// Create a `StateManager` from an explicit worktrees directory path.
    ///
    /// Useful for testing where the directory layout doesn't match `.mr/`.
    #[cfg(test)]
    #[must_use]
    pub fn from_dir(dir: PathBuf) -> Self {
        Self { dir }
    }

    /// Path to the worktrees directory.
    #[must_use]
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Path to the state YAML file.
    #[must_use]
    pub fn state_path(&self) -> PathBuf {
        self.dir.join(STATE_FILE)
    }

    /// Path to the advisory lock file.
    fn lock_path(&self) -> PathBuf {
        self.dir.join(LOCK_FILE)
    }

    /// Path to the temporary file used for atomic writes.
    fn tmp_path(&self) -> PathBuf {
        self.dir.join(STATE_TMP_FILE)
    }

    /// Ensure the worktrees directory exists.
    pub fn ensure_dir(&self) -> Result<()> {
        fs::create_dir_all(&self.dir).with_context(|| {
            format!(
                "failed to create worktrees directory: {}",
                self.dir.display()
            )
        })
    }

    // ── Read / Write (unlocked) ─────────────────────────────────────

    /// Read current state from disk.
    ///
    /// Returns [`WorktreeState::default()`] if the state file does not exist.
    /// For concurrent-safe reads, prefer [`Self::read_locked`].
    pub fn read(&self) -> Result<WorktreeState> {
        let path = self.state_path();

        if !path.exists() {
            return Ok(WorktreeState::default());
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read state file: {}", path.display()))?;

        serde_yaml::from_str(&contents)
            .with_context(|| format!("failed to parse state file: {}", path.display()))
    }

    /// Write state to disk atomically.
    ///
    /// Serializes to a temporary file, then renames into place so readers
    /// never see a partially-written file.  For concurrent-safe writes,
    /// prefer [`Self::modify`] which holds the advisory lock.
    pub fn write(&self, state: &WorktreeState) -> Result<()> {
        self.ensure_dir()?;

        let yaml = serde_yaml::to_string(state).context("failed to serialize worktree state")?;

        let tmp = self.tmp_path();
        let dst = self.state_path();

        fs::write(&tmp, yaml)
            .with_context(|| format!("failed to write temp state file: {}", tmp.display()))?;

        fs::rename(&tmp, &dst)
            .with_context(|| format!("failed to rename {} → {}", tmp.display(), dst.display()))
    }

    // ── Locking ─────────────────────────────────────────────────────

    /// Acquire an exclusive advisory lock (blocking).
    ///
    /// Returns an RAII guard that releases the lock on drop.
    fn lock_exclusive(&self) -> Result<LockGuard> {
        self.ensure_dir()?;

        let lock_path = self.lock_path();

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("failed to open lock file: {}", lock_path.display()))?;

        // SAFETY: `flock` operates on a valid file descriptor obtained from
        // `File::as_raw_fd()`.  `LOCK_EX` blocks until the lock is acquired.
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX) };

        if ret != 0 {
            return Err(std::io::Error::last_os_error())
                .context("failed to acquire exclusive advisory lock");
        }

        Ok(LockGuard { _file: file })
    }

    /// Try to acquire an exclusive advisory lock (non-blocking).
    ///
    /// Returns `Ok(Some(guard))` on success, `Ok(None)` if the lock is
    /// already held by another process.
    fn try_lock_exclusive(&self) -> Result<Option<LockGuard>> {
        self.ensure_dir()?;

        let lock_path = self.lock_path();

        let file = OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(&lock_path)
            .with_context(|| format!("failed to open lock file: {}", lock_path.display()))?;

        // SAFETY: `flock` with `LOCK_EX | LOCK_NB` is non-blocking.
        let ret = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };

        if ret != 0 {
            let err = std::io::Error::last_os_error();

            if err.kind() == std::io::ErrorKind::WouldBlock {
                return Ok(None);
            }

            return Err(err).context("failed to try-acquire exclusive advisory lock");
        }

        Ok(Some(LockGuard { _file: file }))
    }

    // ── Locked operations ───────────────────────────────────────────

    /// Read state under an advisory lock.
    ///
    /// Guarantees a consistent snapshot when the daemon or another
    /// process may be writing concurrently.
    pub fn read_locked(&self) -> Result<WorktreeState> {
        let _guard = self.lock_exclusive()?;
        self.read()
    }

    /// Atomic read-modify-write cycle with advisory locking.
    ///
    /// 1. Acquires the exclusive flock.
    /// 2. Reads the current state (or default if missing).
    /// 3. Passes a mutable reference to the closure for modification.
    /// 4. Writes the updated state atomically.
    /// 5. Releases the lock (guard drops).
    ///
    /// Returns the updated state on success.
    pub fn modify<F>(&self, f: F) -> Result<WorktreeState>
    where
        F: FnOnce(&mut WorktreeState) -> Result<()>,
    {
        let _guard = self.lock_exclusive()?;

        let mut state = self.read()?;
        f(&mut state)?;
        self.write(&state)?;

        Ok(state)
    }

    /// Try to perform an atomic read-modify-write without blocking.
    ///
    /// Returns `Ok(None)` if the lock is already held by another process.
    pub fn try_modify<F>(&self, f: F) -> Result<Option<WorktreeState>>
    where
        F: FnOnce(&mut WorktreeState) -> Result<()>,
    {
        let guard = self.try_lock_exclusive()?;

        let Some(_guard) = guard else {
            return Ok(None);
        };

        let mut state = self.read()?;
        f(&mut state)?;
        self.write(&state)?;

        Ok(Some(state))
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::worktree::types::{
        DaemonInfo, EventType, WorktreeEntry, WorktreeEvent, WorktreeStatus,
    };

    fn make_manager(tmp: &tempfile::TempDir) -> StateManager {
        StateManager::from_dir(tmp.path().to_path_buf())
    }

    #[test]
    fn read_returns_default_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        let state = mgr.read().unwrap();
        assert_eq!(state, WorktreeState::default());
    }

    #[test]
    fn write_then_read_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        let state = WorktreeState {
            version: 1,
            daemon: Some(DaemonInfo {
                pid: 1234,
                started_at: "2026-03-04T22:00:00Z".to_string(),
                idle_timeout_hours: 3,
                last_heartbeat: "2026-03-04T22:30:00Z".to_string(),
            }),
            worktrees: vec![WorktreeEntry {
                id: "wt-001".to_string(),
                prd: "PRD-0039".to_string(),
                branch: "microralph-prd-39".to_string(),
                path: "/tmp/test".to_string(),
                status: WorktreeStatus::Active,
                run_pid: Some(5678),
                created_at: "2026-03-04T22:00:00Z".to_string(),
                updated_at: "2026-03-04T22:00:00Z".to_string(),
                merge_target: "main".to_string(),
                modified_files: vec!["src/main.rs".to_string()],
                events: vec![WorktreeEvent {
                    timestamp: "2026-03-04T22:00:00Z".to_string(),
                    event_type: EventType::Created,
                    detail: None,
                }],
            }],
            overlap_warnings: vec![],
        };

        mgr.write(&state).unwrap();

        let loaded = mgr.read().unwrap();
        assert_eq!(state, loaded);
    }

    #[test]
    fn write_is_atomic_via_rename() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        mgr.write(&WorktreeState::default()).unwrap();

        // The temp file should not be left behind.
        assert!(!mgr.tmp_path().exists());
        assert!(mgr.state_path().exists());
    }

    #[test]
    fn modify_applies_closure() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        let result = mgr
            .modify(|state| {
                state.worktrees.push(WorktreeEntry {
                    id: "wt-001".to_string(),
                    prd: "PRD-0001".to_string(),
                    branch: "test-branch".to_string(),
                    path: "/tmp/wt".to_string(),
                    status: WorktreeStatus::Active,
                    run_pid: None,
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                    merge_target: "main".to_string(),
                    modified_files: vec![],
                    events: vec![],
                });
                Ok(())
            })
            .unwrap();

        assert_eq!(result.worktrees.len(), 1);
        assert_eq!(result.worktrees[0].id, "wt-001");

        // Verify persisted to disk.
        let loaded = mgr.read().unwrap();
        assert_eq!(loaded.worktrees.len(), 1);
    }

    #[test]
    fn modify_propagates_closure_error() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        let result = mgr.modify(|_state| {
            anyhow::bail!("intentional test error");
        });

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("intentional test error")
        );
    }

    #[test]
    fn read_locked_returns_consistent_state() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        mgr.write(&WorktreeState::default()).unwrap();

        let state = mgr.read_locked().unwrap();
        assert_eq!(state, WorktreeState::default());
    }

    #[test]
    fn try_modify_succeeds_when_not_locked() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        let result = mgr
            .try_modify(|state| {
                state.version = 42;
                Ok(())
            })
            .unwrap();

        assert!(result.is_some());
        assert_eq!(result.unwrap().version, 42);
    }

    #[test]
    fn ensure_dir_creates_nested_path() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = StateManager::new(tmp.path());

        mgr.ensure_dir().unwrap();
        assert!(mgr.dir().exists());
    }

    #[test]
    fn lock_file_created_on_lock() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        let _guard = mgr.lock_exclusive().unwrap();
        assert!(mgr.lock_path().exists());
    }

    #[test]
    fn new_resolves_correct_path() {
        let mgr = StateManager::new(Path::new("/tmp/myproject"));
        assert_eq!(mgr.dir(), Path::new("/tmp/myproject/.mr/worktrees"));
        assert_eq!(
            mgr.state_path(),
            PathBuf::from("/tmp/myproject/.mr/worktrees/state.yaml")
        );
    }

    #[test]
    fn multiple_sequential_modifies() {
        let tmp = tempfile::tempdir().unwrap();
        let mgr = make_manager(&tmp);

        for i in 0..5 {
            mgr.modify(|state| {
                state.worktrees.push(WorktreeEntry {
                    id: format!("wt-{i:03}"),
                    prd: format!("PRD-{i:04}"),
                    branch: format!("branch-{i}"),
                    path: format!("/tmp/wt-{i}"),
                    status: WorktreeStatus::Active,
                    run_pid: None,
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                    merge_target: "main".to_string(),
                    modified_files: vec![],
                    events: vec![],
                });
                Ok(())
            })
            .unwrap();
        }

        let state = mgr.read().unwrap();
        assert_eq!(state.worktrees.len(), 5);
    }
}
