//! Worktree orchestration daemon.
//!
//! Provides the daemon core with a two-tier heartbeat loop:
//! - **Tier 1** (mechanical, every 30 s): polls worktree liveness via
//!   `kill -0`, updates `modified_files`, recomputes overlap warnings.
//! - **Tier 2** (event-driven): placeholder for agent-driven merge
//!   decisions and state commits (T-011 / T-012 / T-014).
//!
//! PID file at `.mr/worktrees/daemon.pid`.  Auto-exits after a
//! configurable idle timeout (default 3 h).

// Daemon module is defined now but some helpers are consumed by later tasks.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use std::{fs, thread};

use anyhow::{Context, Result, bail};

use super::git;
use super::ipc;
use super::state::StateManager;
use super::types::{
    DaemonConfig, DaemonInfo, EventType, IpcMessage, IpcResponse, OverlapRisk, OverlapWarning,
    WorktreeEntry, WorktreeEvent, WorktreeStatus,
};

// ── Constants ───────────────────────────────────────────────────────

/// PID file name within `.mr/worktrees/`.
const PID_FILE: &str = "daemon.pid";

/// How often the main loop polls for IPC connections and checks timers.
const POLL_INTERVAL: Duration = Duration::from_secs(1);

/// Read timeout applied to accepted IPC connections so the daemon is
/// never blocked on a single client for too long.
const CONNECTION_READ_TIMEOUT: Duration = Duration::from_secs(1);

/// Maximum number of shared files to classify as *low* risk.
const OVERLAP_LOW_MAX: usize = 2;

/// Maximum number of shared files to classify as *medium* risk.
const OVERLAP_MEDIUM_MAX: usize = 5;

// ── Shutdown signal ─────────────────────────────────────────────────

/// Global shutdown flag, set by signal handlers.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

/// Install `SIGTERM` / `SIGINT` handlers that flip [`SHUTDOWN`].
fn install_signal_handlers() {
    // SAFETY: The handler only stores `true` into an `AtomicBool`, which
    // is async-signal-safe.
    unsafe {
        libc::signal(
            libc::SIGTERM,
            signal_handler as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            signal_handler as *const () as libc::sighandler_t,
        );
    }
}

/// Bare signal handler — sets the global [`SHUTDOWN`] flag.
extern "C" fn signal_handler(_sig: libc::c_int) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

// ── Daemon ──────────────────────────────────────────────────────────

/// Worktree orchestration daemon.
///
/// Runs a single-threaded event loop that:
/// 1. Accepts IPC messages from worktree `mr run` processes.
/// 2. Performs periodic Tier 1 heartbeats (liveness, modified files,
///    overlap).
/// 3. Auto-exits after an idle timeout.
pub struct Daemon {
    root: PathBuf,
    config: DaemonConfig,

    /// Per-instance shutdown flag for programmatic control (e.g., tests).
    shutdown: Arc<AtomicBool>,
}

impl Daemon {
    /// Create a daemon with default configuration.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            config: DaemonConfig::default(),
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a daemon with explicit configuration.
    #[must_use]
    pub fn new_with_config(root: PathBuf, config: DaemonConfig) -> Self {
        Self {
            root,
            config,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Obtain a handle to the per-instance shutdown flag.
    ///
    /// Setting this to `true` causes the event loop to exit cleanly.
    #[must_use]
    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.shutdown)
    }

    // ── PID file management ─────────────────────────────────────────

    /// Path to the daemon PID file for the given project root.
    #[must_use]
    pub fn pid_path(root: &Path) -> PathBuf {
        root.join(".mr").join("worktrees").join(PID_FILE)
    }

    /// Write the current process PID to disk.
    fn write_pid_file(&self) -> Result<()> {
        let path = Self::pid_path(&self.root);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create PID file directory: {}", parent.display())
            })?;
        }

        let pid = std::process::id();
        fs::write(&path, pid.to_string())
            .with_context(|| format!("failed to write PID file: {}", path.display()))
    }

    /// Remove the PID file (best-effort on missing file).
    fn remove_pid_file(&self) -> Result<()> {
        let path = Self::pid_path(&self.root);

        if path.exists() {
            fs::remove_file(&path)
                .with_context(|| format!("failed to remove PID file: {}", path.display()))?;
        }

        Ok(())
    }

    /// Read the daemon PID from the PID file, if it exists.
    pub fn read_pid(root: &Path) -> Result<Option<u32>> {
        let path = Self::pid_path(root);

        if !path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&path)
            .with_context(|| format!("failed to read PID file: {}", path.display()))?;

        let pid: u32 = contents
            .trim()
            .parse()
            .with_context(|| format!("invalid PID in {}: {contents}", path.display()))?;

        Ok(Some(pid))
    }

    // ── Process liveness ────────────────────────────────────────────

    /// Check whether a process with the given PID is alive.
    ///
    /// Uses `kill(pid, 0)` which checks existence without sending a signal.
    #[must_use]
    pub fn is_process_alive(pid: u32) -> bool {
        let Ok(pid_i32) = i32::try_from(pid) else {
            return false;
        };

        // SAFETY: `kill` with signal 0 only checks process existence.
        unsafe { libc::kill(pid_i32, 0) == 0 }
    }

    /// Check whether a daemon is currently running for the given root.
    #[must_use]
    pub fn is_running(root: &Path) -> bool {
        match Self::read_pid(root) {
            Ok(Some(pid)) => Self::is_process_alive(pid),
            _ => false,
        }
    }

    /// Send `SIGTERM` to a running daemon and wait for it to exit.
    pub fn stop(root: &Path) -> Result<()> {
        let pid = Self::read_pid(root)?
            .context("no daemon PID file found — daemon may not be running")?;

        if !Self::is_process_alive(pid) {
            let _ = fs::remove_file(Self::pid_path(root));
            bail!("daemon process {pid} is not running (stale PID file removed)");
        }

        let pid_i32 = i32::try_from(pid).context("PID value out of range")?;

        // SAFETY: Sends SIGTERM to the daemon process for clean shutdown.
        let ret = unsafe { libc::kill(pid_i32, libc::SIGTERM) };

        if ret != 0 {
            return Err(std::io::Error::last_os_error())
                .context("failed to send SIGTERM to daemon");
        }

        // Wait for the process to exit (up to 5 s).
        for _ in 0..50 {
            if !Self::is_process_alive(pid) {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(100));
        }

        bail!("daemon process {pid} did not exit within 5 seconds after SIGTERM")
    }

    // ── State helpers ───────────────────────────────────────────────

    fn state_manager(&self) -> StateManager {
        StateManager::new(&self.root)
    }

    /// Register this daemon instance in the state file.
    fn register_daemon(&self) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let pid = std::process::id();

        self.state_manager().modify(|state| {
            state.daemon = Some(DaemonInfo {
                pid,
                started_at: now.clone(),
                idle_timeout_hours: self.config.idle_timeout_hours,
                last_heartbeat: now,
            });
            Ok(())
        })?;

        Ok(())
    }

    /// Remove daemon information from the state file.
    fn unregister_daemon(&self) -> Result<()> {
        self.state_manager().modify(|state| {
            state.daemon = None;
            Ok(())
        })?;

        Ok(())
    }

    // ── Main event loop ─────────────────────────────────────────────

    /// Run the daemon (production entry point).
    ///
    /// Installs signal handlers, writes a PID file, binds the IPC socket,
    /// and enters the event loop.  Returns when the daemon shuts down
    /// (via signal, idle timeout, or programmatic shutdown handle).
    pub fn run(&self) -> Result<()> {
        SHUTDOWN.store(false, Ordering::SeqCst);
        self.shutdown.store(false, Ordering::SeqCst);
        install_signal_handlers();

        self.write_pid_file()?;
        self.register_daemon()?;

        let sock = ipc::socket_path(&self.root);
        let server = ipc::IpcServer::bind(&sock)?;
        server.set_nonblocking(true)?;

        tracing::info!("daemon started (pid {})", std::process::id());

        self.event_loop(&server);

        // Cleanup regardless of how the loop exited.
        if let Err(e) = self.unregister_daemon() {
            tracing::warn!("failed to unregister daemon: {e:#}");
        }
        if let Err(e) = self.remove_pid_file() {
            tracing::warn!("failed to remove PID file: {e:#}");
        }
        // `IpcServer::drop` cleans up the socket file.

        tracing::info!("daemon stopped");

        Ok(())
    }

    /// Core event loop: accept IPC, run heartbeats, check idle timeout.
    fn event_loop(&self, server: &ipc::IpcServer) {
        let mut last_heartbeat = Instant::now();
        let mut last_activity = Instant::now();

        loop {
            // Check shutdown signals.
            if self.should_shutdown() {
                tracing::info!("shutdown signal received");
                break;
            }

            // Try to accept and handle an IPC connection.
            match server.try_accept_stream() {
                Ok(Some(stream)) => {
                    if let Err(e) = Self::handle_connection(&self.root, stream) {
                        tracing::warn!("IPC connection error: {e:#}");
                    }
                    last_activity = Instant::now();
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!("IPC accept error: {e:#}");
                }
            }

            // Tier 1 heartbeat.
            let heartbeat_interval = Duration::from_secs(self.config.heartbeat_interval_secs);

            if last_heartbeat.elapsed() >= heartbeat_interval {
                match self.tier1_heartbeat() {
                    Ok(has_active) => {
                        if has_active {
                            last_activity = Instant::now();
                        }
                    }
                    Err(e) => {
                        tracing::warn!("heartbeat error: {e:#}");
                    }
                }
                last_heartbeat = Instant::now();
            }

            // Idle timeout.
            let idle_timeout =
                Duration::from_secs(u64::from(self.config.idle_timeout_hours) * 3600);

            if last_activity.elapsed() >= idle_timeout {
                tracing::info!(
                    "idle timeout ({}h) reached, shutting down",
                    self.config.idle_timeout_hours
                );
                break;
            }

            thread::sleep(POLL_INTERVAL);
        }
    }

    /// Returns `true` when the daemon should exit.
    fn should_shutdown(&self) -> bool {
        SHUTDOWN.load(Ordering::Relaxed) || self.shutdown.load(Ordering::Relaxed)
    }

    // ── IPC message handling ────────────────────────────────────────

    /// Handle all messages on an accepted IPC connection.
    fn handle_connection(root: &Path, stream: std::os::unix::net::UnixStream) -> Result<()> {
        stream
            .set_read_timeout(Some(CONNECTION_READ_TIMEOUT))
            .context("failed to set read timeout on IPC connection")?;

        let root = root.to_path_buf();

        ipc::handle_stream(stream, |msg| Self::process_message(&root, msg))
    }

    /// Dispatch an incoming IPC message and return a response.
    fn process_message(root: &Path, msg: IpcMessage) -> IpcResponse {
        match msg {
            IpcMessage::HeartbeatRequest => IpcResponse::ok(),

            IpcMessage::RunStarted { wt_id, pid, .. } => {
                Self::handle_run_started(root, &wt_id, pid)
            }
            IpcMessage::TaskStarted { wt_id, task, .. } => {
                Self::handle_event(root, &wt_id, EventType::TaskStarted, Some(&task))
            }
            IpcMessage::TaskCompleted { wt_id, task, .. } => {
                Self::handle_event(root, &wt_id, EventType::TaskCompleted, Some(&task))
            }
            IpcMessage::RunCompleted { wt_id, .. } => Self::handle_run_completed(root, &wt_id),
            IpcMessage::RunFailed { wt_id, error, .. } => {
                Self::handle_run_failed(root, &wt_id, &error)
            }
        }
    }

    fn handle_run_started(root: &Path, wt_id: &str, pid: u32) -> IpcResponse {
        let state_mgr = StateManager::new(root);

        match state_mgr.modify(|state| {
            if let Some(wt) = state.worktrees.iter_mut().find(|w| w.id == wt_id) {
                wt.run_pid = Some(pid);
                wt.status = WorktreeStatus::Active;

                let now = chrono::Utc::now().to_rfc3339();
                wt.updated_at.clone_from(&now);
                wt.events.push(WorktreeEvent {
                    timestamp: now,
                    event_type: EventType::RunStarted,
                    detail: Some(format!("pid {pid}")),
                });
            }
            Ok(())
        }) {
            Ok(_) => IpcResponse::ok(),
            Err(e) => IpcResponse::error(format!("{e:#}")),
        }
    }

    fn handle_event(
        root: &Path,
        wt_id: &str,
        event_type: EventType,
        task: Option<&str>,
    ) -> IpcResponse {
        let state_mgr = StateManager::new(root);
        let detail = task.map(String::from);

        match state_mgr.modify(|state| {
            if let Some(wt) = state.worktrees.iter_mut().find(|w| w.id == wt_id) {
                let now = chrono::Utc::now().to_rfc3339();
                wt.updated_at.clone_from(&now);
                wt.events.push(WorktreeEvent {
                    timestamp: now,
                    event_type,
                    detail: detail.clone(),
                });
            }
            Ok(())
        }) {
            Ok(_) => IpcResponse::ok(),
            Err(e) => IpcResponse::error(format!("{e:#}")),
        }
    }

    fn handle_run_completed(root: &Path, wt_id: &str) -> IpcResponse {
        let state_mgr = StateManager::new(root);

        match state_mgr.modify(|state| {
            if let Some(wt) = state.worktrees.iter_mut().find(|w| w.id == wt_id) {
                wt.status = WorktreeStatus::Completed;
                wt.run_pid = None;

                let now = chrono::Utc::now().to_rfc3339();
                wt.updated_at.clone_from(&now);
                wt.events.push(WorktreeEvent {
                    timestamp: now,
                    event_type: EventType::RunCompleted,
                    detail: None,
                });
            }
            Ok(())
        }) {
            Ok(_) => IpcResponse::ok(),
            Err(e) => IpcResponse::error(format!("{e:#}")),
        }
    }

    fn handle_run_failed(root: &Path, wt_id: &str, error: &str) -> IpcResponse {
        let state_mgr = StateManager::new(root);
        let err_msg = error.to_string();

        match state_mgr.modify(|state| {
            if let Some(wt) = state.worktrees.iter_mut().find(|w| w.id == wt_id) {
                wt.status = WorktreeStatus::Abandoned;
                wt.run_pid = None;

                let now = chrono::Utc::now().to_rfc3339();
                wt.updated_at.clone_from(&now);
                wt.events.push(WorktreeEvent {
                    timestamp: now,
                    event_type: EventType::RunFailed,
                    detail: Some(err_msg.clone()),
                });
            }
            Ok(())
        }) {
            Ok(_) => IpcResponse::ok(),
            Err(e) => IpcResponse::error(format!("{e:#}")),
        }
    }

    // ── Tier 1 heartbeat ────────────────────────────────────────────

    /// Run Tier 1 heartbeat: check liveness, update modified files,
    /// recompute overlap.
    ///
    /// Returns `true` if there are active worktrees.
    fn tier1_heartbeat(&self) -> Result<bool> {
        let state = self.state_manager().modify(|state| {
            let now = chrono::Utc::now().to_rfc3339();

            // Update daemon heartbeat timestamp.
            if let Some(ref mut daemon) = state.daemon {
                daemon.last_heartbeat.clone_from(&now);
            }

            for wt in &mut state.worktrees {
                if wt.status != WorktreeStatus::Active {
                    continue;
                }

                // Check process liveness.
                if let Some(pid) = wt.run_pid
                    && !Self::is_process_alive(pid)
                {
                    wt.status = WorktreeStatus::Completed;
                    wt.run_pid = None;
                    wt.updated_at.clone_from(&now);
                    wt.events.push(WorktreeEvent {
                        timestamp: now.clone(),
                        event_type: EventType::RunCompleted,
                        detail: Some("Process exited (detected by heartbeat)".to_string()),
                    });
                    continue;
                }

                // Update modified files (best-effort from main worktree).
                if let Ok(files) = git::modified_files(&wt.branch, &wt.merge_target, &self.root) {
                    wt.modified_files = files;
                }

                wt.updated_at.clone_from(&now);
            }

            // Recompute overlap warnings.
            state.overlap_warnings = Self::compute_overlaps(&state.worktrees);

            Ok(())
        })?;

        let has_active = state
            .worktrees
            .iter()
            .any(|w| w.status == WorktreeStatus::Active);

        Ok(has_active)
    }

    // ── Overlap computation ─────────────────────────────────────────

    /// Compute file-overlap warnings across active/completed worktrees.
    #[must_use]
    pub fn compute_overlaps(worktrees: &[WorktreeEntry]) -> Vec<OverlapWarning> {
        let active: Vec<&WorktreeEntry> = worktrees
            .iter()
            .filter(|w| matches!(w.status, WorktreeStatus::Active | WorktreeStatus::Completed))
            .collect();

        let mut warnings = Vec::new();

        for (i, wt_a) in active.iter().enumerate() {
            for wt_b in &active[i + 1..] {
                let shared: Vec<String> = wt_a
                    .modified_files
                    .iter()
                    .filter(|f| wt_b.modified_files.contains(f))
                    .cloned()
                    .collect();

                if shared.is_empty() {
                    continue;
                }

                let risk = if shared.len() <= OVERLAP_LOW_MAX {
                    OverlapRisk::Low
                } else if shared.len() <= OVERLAP_MEDIUM_MAX {
                    OverlapRisk::Medium
                } else {
                    OverlapRisk::High
                };

                warnings.push(OverlapWarning {
                    worktrees: vec![wt_a.id.clone(), wt_b.id.clone()],
                    files: shared,
                    risk,
                });
            }
        }

        warnings
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::worktree::ipc::IpcClient;
    use crate::worktree::types::WorktreeState;

    /// Helper: create a `WorktreeEntry` with given id, status, and modified files.
    fn make_entry(id: &str, status: WorktreeStatus, files: &[&str]) -> WorktreeEntry {
        WorktreeEntry {
            id: id.to_string(),
            prd: format!("PRD-{id}"),
            branch: format!("branch-{id}"),
            path: format!("/tmp/{id}"),
            status,
            run_pid: None,
            created_at: "2026-01-01T00:00:00Z".to_string(),
            updated_at: "2026-01-01T00:00:00Z".to_string(),
            merge_target: "main".to_string(),
            modified_files: files.iter().map(|f| (*f).to_string()).collect(),
            events: vec![],
        }
    }

    // ── Process liveness ────────────────────────────────────────────

    #[test]
    fn is_process_alive_returns_true_for_self() {
        let pid = std::process::id();
        assert!(Daemon::is_process_alive(pid));
    }

    #[test]
    fn is_process_alive_returns_false_for_invalid_pid() {
        // PID 4_000_000 is extremely unlikely to exist.
        assert!(!Daemon::is_process_alive(4_000_000));
    }

    // ── PID file ────────────────────────────────────────────────────

    #[test]
    fn pid_file_write_read_remove() {
        let tmp = tempfile::tempdir().unwrap();
        let daemon = Daemon::new(tmp.path().to_path_buf());

        // Write.
        daemon.write_pid_file().unwrap();
        assert!(Daemon::pid_path(tmp.path()).exists());

        // Read.
        let pid = Daemon::read_pid(tmp.path()).unwrap();
        assert_eq!(pid, Some(std::process::id()));

        // Remove.
        daemon.remove_pid_file().unwrap();
        assert!(!Daemon::pid_path(tmp.path()).exists());
    }

    #[test]
    fn read_pid_returns_none_when_no_file() {
        let tmp = tempfile::tempdir().unwrap();
        let pid = Daemon::read_pid(tmp.path()).unwrap();
        assert!(pid.is_none());
    }

    #[test]
    fn is_running_false_when_no_pid_file() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!Daemon::is_running(tmp.path()));
    }

    // ── Overlap computation ─────────────────────────────────────────

    #[test]
    fn compute_overlaps_no_overlap() {
        let entries = vec![
            make_entry("a", WorktreeStatus::Active, &["src/a.rs"]),
            make_entry("b", WorktreeStatus::Active, &["src/b.rs"]),
        ];

        let warnings = Daemon::compute_overlaps(&entries);
        assert!(warnings.is_empty());
    }

    #[test]
    fn compute_overlaps_low_risk() {
        let entries = vec![
            make_entry("a", WorktreeStatus::Active, &["src/shared.rs", "src/a.rs"]),
            make_entry("b", WorktreeStatus::Active, &["src/shared.rs", "src/b.rs"]),
        ];

        let warnings = Daemon::compute_overlaps(&entries);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].risk, OverlapRisk::Low);
        assert_eq!(warnings[0].files, vec!["src/shared.rs"]);
    }

    #[test]
    fn compute_overlaps_medium_risk() {
        let shared: Vec<&str> = vec!["a.rs", "b.rs", "c.rs"];
        let entries = vec![
            make_entry("a", WorktreeStatus::Active, &shared),
            make_entry("b", WorktreeStatus::Completed, &shared),
        ];

        let warnings = Daemon::compute_overlaps(&entries);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].risk, OverlapRisk::Medium);
    }

    #[test]
    fn compute_overlaps_high_risk() {
        let shared: Vec<&str> = vec!["a.rs", "b.rs", "c.rs", "d.rs", "e.rs", "f.rs"];
        let entries = vec![
            make_entry("a", WorktreeStatus::Active, &shared),
            make_entry("b", WorktreeStatus::Active, &shared),
        ];

        let warnings = Daemon::compute_overlaps(&entries);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].risk, OverlapRisk::High);
    }

    #[test]
    fn compute_overlaps_ignores_non_active() {
        let entries = vec![
            make_entry("a", WorktreeStatus::Active, &["shared.rs"]),
            make_entry("b", WorktreeStatus::Merged, &["shared.rs"]),
            make_entry("c", WorktreeStatus::Abandoned, &["shared.rs"]),
        ];

        let warnings = Daemon::compute_overlaps(&entries);
        assert!(warnings.is_empty());
    }

    #[test]
    fn compute_overlaps_multiple_pairs() {
        let entries = vec![
            make_entry("a", WorktreeStatus::Active, &["x.rs", "y.rs"]),
            make_entry("b", WorktreeStatus::Active, &["x.rs"]),
            make_entry("c", WorktreeStatus::Active, &["y.rs"]),
        ];

        let warnings = Daemon::compute_overlaps(&entries);
        assert_eq!(warnings.len(), 2); // a-b (x.rs), a-c (y.rs)
    }

    // ── IPC message processing ──────────────────────────────────────

    #[test]
    fn process_message_heartbeat_returns_ok() {
        let tmp = tempfile::tempdir().unwrap();
        let resp = Daemon::process_message(tmp.path(), IpcMessage::HeartbeatRequest);
        assert_eq!(resp.status, "ok");
    }

    #[test]
    fn process_message_run_started_updates_state() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        // Pre-populate with a worktree entry.
        state_mgr
            .modify(|state| {
                state
                    .worktrees
                    .push(make_entry("wt-001", WorktreeStatus::Active, &[]));
                Ok(())
            })
            .unwrap();

        let resp = Daemon::process_message(
            tmp.path(),
            IpcMessage::RunStarted {
                prd: "PRD-0001".to_string(),
                wt_id: "wt-001".to_string(),
                pid: 99999,
            },
        );

        assert_eq!(resp.status, "ok");

        let state = state_mgr.read().unwrap();
        let wt = &state.worktrees[0];
        assert_eq!(wt.run_pid, Some(99999));
        assert_eq!(wt.events.len(), 1);
        assert_eq!(wt.events[0].event_type, EventType::RunStarted);
    }

    #[test]
    fn process_message_run_completed_updates_status() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        state_mgr
            .modify(|state| {
                let mut entry = make_entry("wt-002", WorktreeStatus::Active, &[]);
                entry.run_pid = Some(12345);
                state.worktrees.push(entry);
                Ok(())
            })
            .unwrap();

        let resp = Daemon::process_message(
            tmp.path(),
            IpcMessage::RunCompleted {
                prd: "PRD-0002".to_string(),
                wt_id: "wt-002".to_string(),
            },
        );

        assert_eq!(resp.status, "ok");

        let state = state_mgr.read().unwrap();
        assert_eq!(state.worktrees[0].status, WorktreeStatus::Completed);
        assert!(state.worktrees[0].run_pid.is_none());
    }

    #[test]
    fn process_message_run_failed_marks_abandoned() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        state_mgr
            .modify(|state| {
                state
                    .worktrees
                    .push(make_entry("wt-003", WorktreeStatus::Active, &[]));
                Ok(())
            })
            .unwrap();

        let resp = Daemon::process_message(
            tmp.path(),
            IpcMessage::RunFailed {
                prd: "PRD-0003".to_string(),
                wt_id: "wt-003".to_string(),
                error: "UAT failed".to_string(),
            },
        );

        assert_eq!(resp.status, "ok");

        let state = state_mgr.read().unwrap();
        assert_eq!(state.worktrees[0].status, WorktreeStatus::Abandoned);
    }

    #[test]
    fn process_message_ignores_unknown_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();
        state_mgr.write(&WorktreeState::default()).unwrap();

        // Message for a worktree ID that doesn't exist — should succeed silently.
        let resp = Daemon::process_message(
            tmp.path(),
            IpcMessage::RunStarted {
                prd: "PRD-9999".to_string(),
                wt_id: "wt-nonexistent".to_string(),
                pid: 1,
            },
        );

        assert_eq!(resp.status, "ok");
    }

    // ── Daemon lifecycle ────────────────────────────────────────────

    #[test]
    fn daemon_exits_on_idle_timeout() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        let daemon = Daemon::new_with_config(
            tmp.path().to_path_buf(),
            DaemonConfig {
                heartbeat_interval_secs: 60, // won't trigger
                idle_timeout_hours: 0,       // immediate exit
                socket_name: "daemon.sock".to_string(),
            },
        );

        // Should exit quickly due to idle timeout = 0.
        daemon.run().unwrap();

        // After exit, PID file and daemon info should be cleaned up.
        assert!(!Daemon::pid_path(tmp.path()).exists());

        let state = state_mgr.read().unwrap();
        assert!(state.daemon.is_none());
    }

    #[test]
    fn daemon_creates_pid_and_socket_then_cleans_up() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        let daemon = Daemon::new_with_config(
            tmp.path().to_path_buf(),
            DaemonConfig {
                heartbeat_interval_secs: 60,
                idle_timeout_hours: 0,
                socket_name: "daemon.sock".to_string(),
            },
        );

        let shutdown = daemon.shutdown_handle();

        // Start daemon in a thread.
        let root = tmp.path().to_path_buf();
        let handle = thread::spawn(move || daemon.run());

        // Give it a moment to start.
        thread::sleep(Duration::from_millis(200));

        // It should have already exited due to idle_timeout=0.
        // Signal shutdown just in case.
        shutdown.store(true, Ordering::SeqCst);

        handle.join().unwrap().unwrap();

        // After exit, PID file should be removed.
        assert!(!Daemon::pid_path(&root).exists());
    }

    #[test]
    fn daemon_responds_to_programmatic_shutdown() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        let daemon = Daemon::new_with_config(
            tmp.path().to_path_buf(),
            DaemonConfig {
                heartbeat_interval_secs: 60,
                idle_timeout_hours: 24, // won't auto-exit
                socket_name: "daemon.sock".to_string(),
            },
        );

        let shutdown = daemon.shutdown_handle();
        let root = tmp.path().to_path_buf();

        let handle = thread::spawn(move || daemon.run());

        // Wait for daemon to be ready (PID file exists).
        for _ in 0..50 {
            if Daemon::pid_path(&root).exists() {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        assert!(Daemon::pid_path(&root).exists());

        // Signal shutdown.
        shutdown.store(true, Ordering::SeqCst);

        let result = handle.join().unwrap();
        assert!(result.is_ok());

        // Cleanup verified.
        assert!(!Daemon::pid_path(&root).exists());

        let state = state_mgr.read().unwrap();
        assert!(state.daemon.is_none());
    }

    #[test]
    fn daemon_accepts_ipc_heartbeat() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        let daemon = Daemon::new_with_config(
            tmp.path().to_path_buf(),
            DaemonConfig {
                heartbeat_interval_secs: 60,
                idle_timeout_hours: 24,
                socket_name: "daemon.sock".to_string(),
            },
        );

        let shutdown = daemon.shutdown_handle();
        let root = tmp.path().to_path_buf();

        let handle = thread::spawn(move || daemon.run());

        // Wait for socket to be available.
        let sock = ipc::socket_path(&root);
        for _ in 0..50 {
            if ipc::is_daemon_reachable(&sock) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
        assert!(ipc::is_daemon_reachable(&sock));

        // Send a heartbeat request via IPC.
        let mut client = IpcClient::connect(&sock).unwrap();
        let resp = client.send(&IpcMessage::HeartbeatRequest).unwrap();
        assert_eq!(resp.status, "ok");
        drop(client);

        // Shutdown.
        shutdown.store(true, Ordering::SeqCst);
        handle.join().unwrap().unwrap();
    }

    #[test]
    fn daemon_registers_and_unregisters_in_state() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        let daemon = Daemon::new_with_config(
            tmp.path().to_path_buf(),
            DaemonConfig {
                heartbeat_interval_secs: 60,
                idle_timeout_hours: 0,
                socket_name: "daemon.sock".to_string(),
            },
        );

        daemon.run().unwrap();

        // After exit, daemon should be unregistered.
        let state = state_mgr.read().unwrap();
        assert!(state.daemon.is_none());
    }
}
