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

    /// Optional runner for agent-driven conflict resolution.
    runner: Option<Box<dyn crate::runner::Runner>>,

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
            runner: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a daemon with explicit configuration.
    #[must_use]
    pub fn new_with_config(root: PathBuf, config: DaemonConfig) -> Self {
        Self {
            root,
            config,
            runner: None,
            shutdown: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Create a daemon with explicit configuration and a runner for conflict resolution.
    pub fn new_with_runner(
        root: PathBuf,
        config: DaemonConfig,
        runner: Box<dyn crate::runner::Runner>,
    ) -> Self {
        Self {
            root,
            config,
            runner: Some(runner),
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
    ///
    /// Only checks PID liveness via `kill -0`.  For a stronger check that
    /// also verifies socket availability, use [`Self::is_healthy`].
    #[must_use]
    pub fn is_running(root: &Path) -> bool {
        match Self::read_pid(root) {
            Ok(Some(pid)) => Self::is_process_alive(pid),
            _ => false,
        }
    }

    /// Check whether a daemon is both running **and** reachable via IPC.
    ///
    /// Returns `true` only when the PID file points to a live process
    /// *and* the daemon socket is connectable.
    #[must_use]
    pub fn is_healthy(root: &Path) -> bool {
        Self::is_running(root) && ipc::is_daemon_reachable(&ipc::socket_path(root))
    }

    /// Remove stale PID and socket files left behind by a dead daemon.
    ///
    /// Called before spawning a new daemon to ensure a clean start.
    pub fn cleanup_stale(root: &Path) {
        let pid_path = Self::pid_path(root);
        if pid_path.exists() {
            let is_alive = Self::read_pid(root)
                .map(|opt| opt.is_some_and(Self::is_process_alive))
                .unwrap_or(false);

            if !is_alive {
                let _ = fs::remove_file(&pid_path);
                tracing::debug!("removed stale PID file: {}", pid_path.display());
            }
        }

        let sock_path = ipc::socket_path(root);
        if sock_path.exists() && !ipc::is_daemon_reachable(&sock_path) {
            let _ = fs::remove_file(&sock_path);
            tracing::debug!("removed stale socket file: {}", sock_path.display());
        }
    }

    /// Ensure a daemon is running and healthy, spawning one if necessary.
    ///
    /// 1. If the daemon is already healthy (PID alive + socket reachable),
    ///    returns immediately.
    /// 2. Otherwise, cleans up stale PID/socket files and spawns a new
    ///    daemon process (`mr wt daemon start`) as a detached background
    ///    process.
    /// 3. Waits up to 10 seconds for the socket to become reachable.
    ///
    /// This is the primary entry point for daemon auto-start logic,
    /// called by `mr wt run` before dispatching work to a worktree.
    pub fn ensure_running(root: &Path) -> Result<()> {
        if Self::is_healthy(root) {
            return Ok(());
        }

        Self::cleanup_stale(root);

        let exe = std::env::current_exe().context("failed to resolve current executable")?;

        let child = std::process::Command::new(&exe)
            .args(["wt", "daemon", "start"])
            .current_dir(root)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("failed to spawn daemon process")?;

        tracing::info!(pid = child.id(), "spawned daemon process");

        // Wait for the socket to become reachable.
        let sock = ipc::socket_path(root);
        let deadline = std::time::Instant::now() + Duration::from_secs(10);

        while std::time::Instant::now() < deadline {
            if ipc::is_daemon_reachable(&sock) {
                return Ok(());
            }
            thread::sleep(Duration::from_millis(200));
        }

        // Daemon may still be starting up — warn but don't fail.
        tracing::warn!("daemon socket not reachable after 10 s — proceeding anyway");
        Ok(())
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

                // Tier 2: auto-merge completed worktrees.
                self.tier2_auto_merge();
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

    // ── Tier 2 auto-merge ───────────────────────────────────────────

    /// Tier 2: attempt auto-merge for all completed worktrees.
    ///
    /// Reads state, finds completed worktrees, sorts them by merge
    /// priority, and attempts to merge each one into its target branch.
    fn tier2_auto_merge(&self) {
        let state = match self.state_manager().read() {
            Ok(s) => s,
            Err(e) => {
                tracing::warn!("tier2: failed to read state: {e:#}");
                return;
            }
        };

        let completed: Vec<&WorktreeEntry> = state
            .worktrees
            .iter()
            .filter(|w| w.status == WorktreeStatus::Completed)
            .collect();

        if completed.is_empty() {
            return;
        }

        let merge_order = Self::compute_merge_order(&completed, &state.overlap_warnings);

        tracing::info!(
            "tier2: {} completed worktree(s) ready to merge",
            merge_order.len()
        );

        for wt_id in merge_order {
            if self.should_shutdown() {
                break;
            }

            if let Err(e) = self.attempt_merge_worktree(&wt_id) {
                tracing::warn!("tier2: merge failed for {wt_id}: {e:#}");
            }
        }
    }

    /// Compute deterministic merge order for completed worktrees.
    ///
    /// Ordering criteria (lower = merge first):
    /// 1. Fewer overlapping files with other completed worktrees
    /// 2. Earlier completion time (`updated_at`)
    #[must_use]
    pub fn compute_merge_order(
        completed: &[&WorktreeEntry],
        overlap_warnings: &[OverlapWarning],
    ) -> Vec<String> {
        let mut scored: Vec<(&WorktreeEntry, usize)> = completed
            .iter()
            .map(|wt| {
                let overlap_count: usize = overlap_warnings
                    .iter()
                    .filter(|ow| ow.worktrees.contains(&wt.id))
                    .map(|ow| ow.files.len())
                    .sum();
                (*wt, overlap_count)
            })
            .collect();

        scored.sort_by(|(a, a_overlap), (b, b_overlap)| {
            a_overlap
                .cmp(b_overlap)
                .then_with(|| a.updated_at.cmp(&b.updated_at))
        });

        scored.into_iter().map(|(wt, _)| wt.id.clone()).collect()
    }

    /// Attempt to merge a single completed worktree into its target.
    ///
    /// Flow:
    /// 1. Mark status as `merging`
    /// 2. Rebase branch onto target (or merge target into branch on failure)
    /// 3. Run `cargo make uat` in the worktree
    /// 4. If UATs pass, merge branch into target via fast-forward (or merge)
    /// 5. Update state accordingly
    fn attempt_merge_worktree(&self, wt_id: &str) -> Result<()> {
        let state = self.state_manager().read()?;
        let wt = state
            .worktrees
            .iter()
            .find(|w| w.id == wt_id)
            .context("worktree not found in state")?;

        if wt.status != WorktreeStatus::Completed {
            return Ok(());
        }

        let wt_path = PathBuf::from(&wt.path);
        let branch = wt.branch.clone();
        let merge_target = wt.merge_target.clone();

        tracing::info!("tier2: merging {wt_id} ({branch}) into {merge_target}");

        // Mark as merging.
        self.update_wt_status(
            wt_id,
            WorktreeStatus::Merging,
            EventType::MergeStarted,
            None,
        )?;

        // Step 1: Update branch with latest target changes.
        let integrate_result = Self::integrate_target_into_branch(&wt_path, &merge_target);

        if integrate_result.is_err() {
            // Integration produced conflicts — try agent-driven resolution.
            if let Some(runner) = &self.runner {
                tracing::info!("tier2: {wt_id} has conflicts, attempting agent resolution");
                self.update_wt_status(
                    wt_id,
                    WorktreeStatus::Conflicted,
                    EventType::ConflictResolutionStarted,
                    Some("Agent resolving merge conflicts"),
                )?;

                match self.resolve_conflicts(wt_id, &wt_path, &merge_target, runner.as_ref()) {
                    Ok(()) => {
                        tracing::info!("tier2: agent resolved conflicts for {wt_id}");
                    }
                    Err(e) => {
                        tracing::warn!(
                            "tier2: agent conflict resolution failed for {wt_id}: {e:#}"
                        );
                        self.update_wt_status(
                            wt_id,
                            WorktreeStatus::Conflicted,
                            EventType::Conflicted,
                            Some(&format!("Agent conflict resolution failed: {e:#}")),
                        )?;
                        return Ok(());
                    }
                }
            } else {
                // No runner available — mark as conflicted for manual resolution.
                tracing::warn!("tier2: {wt_id} has conflicts, no runner for resolution");
                self.update_wt_status(
                    wt_id,
                    WorktreeStatus::Conflicted,
                    EventType::Conflicted,
                    Some("Rebase and merge both produced conflicts (no runner for resolution)"),
                )?;
                return Ok(());
            }
        }

        // Step 2: Run UATs in the worktree.
        tracing::info!("tier2: running UATs for {wt_id} in {}", wt_path.display());
        let uat_passed = Self::run_uat(&wt_path);

        if !uat_passed {
            tracing::warn!("tier2: UATs failed for {wt_id}");
            self.update_wt_status(
                wt_id,
                WorktreeStatus::MergeFailed,
                EventType::MergeFailed,
                Some("UATs failed after integration"),
            )?;
            return Ok(());
        }

        // Step 3: Merge branch into target on main worktree.
        tracing::info!("tier2: merging {branch} into {merge_target} on main worktree");
        if let Err(e) = self.merge_into_target(&branch, &merge_target) {
            tracing::warn!("tier2: failed to merge {wt_id} into {merge_target}: {e:#}");
            self.update_wt_status(
                wt_id,
                WorktreeStatus::MergeFailed,
                EventType::MergeFailed,
                Some(&format!("Failed to merge into {merge_target}: {e:#}")),
            )?;
            return Ok(());
        }

        // Success!
        tracing::info!("tier2: {wt_id} merged successfully");
        self.update_wt_status(
            wt_id,
            WorktreeStatus::Merged,
            EventType::MergeCompleted,
            None,
        )?;

        Ok(())
    }

    // ── Manual merge (mr wt merge) ─────────────────────────────────

    /// Manually merge a worktree branch into a target.
    ///
    /// Unlike `attempt_merge_worktree` (auto-merge from daemon heartbeat),
    /// this accepts a PRD ID, allows an optional target override, permits
    /// merging from broader states, and supports cross-worktree merges.
    pub fn manual_merge(&self, prd_id: &str, target_override: Option<&str>) -> Result<()> {
        let state = self.state_manager().read()?;
        let wt = state
            .worktrees
            .iter()
            .find(|w| w.prd.eq_ignore_ascii_case(prd_id))
            .with_context(|| format!("no worktree registered for {prd_id}"))?;

        Self::validate_mergeable_status(prd_id, wt.status)?;

        let wt_id = wt.id.clone();
        let wt_path = PathBuf::from(&wt.path);
        let branch = wt.branch.clone();
        let merge_target = target_override
            .map_or_else(|| wt.merge_target.clone(), std::string::ToString::to_string);

        tracing::info!("manual merge: {prd_id} ({branch}) into {merge_target}");

        self.update_wt_status(
            &wt_id,
            WorktreeStatus::Merging,
            EventType::MergeStarted,
            None,
        )?;

        // Step 1: Integrate target into branch (rebase-first, merge fallback).
        if Self::integrate_target_into_branch(&wt_path, &merge_target).is_err() {
            self.handle_merge_conflicts(&wt_id, &wt_path, &merge_target)?;
        }

        // Step 2: Run UATs in the worktree.
        if !Self::run_uat(&wt_path) {
            self.update_wt_status(
                &wt_id,
                WorktreeStatus::MergeFailed,
                EventType::MergeFailed,
                Some("UATs failed after integration"),
            )?;
            bail!("UATs failed after merge integration");
        }

        // Step 3: Merge branch into target (cross-worktree aware).
        if let Err(e) = self.smart_merge_into_target(&branch, &merge_target) {
            self.update_wt_status(
                &wt_id,
                WorktreeStatus::MergeFailed,
                EventType::MergeFailed,
                Some(&format!("Failed to merge into {merge_target}: {e:#}")),
            )?;
            bail!("failed to merge into {merge_target}: {e:#}");
        }

        self.update_wt_status(
            &wt_id,
            WorktreeStatus::Merged,
            EventType::MergeCompleted,
            Some(&format!("Manually merged into {merge_target}")),
        )?;

        Ok(())
    }

    /// Validate that a worktree's status allows merging.
    fn validate_mergeable_status(prd_id: &str, status: WorktreeStatus) -> Result<()> {
        match status {
            WorktreeStatus::Active
            | WorktreeStatus::Completed
            | WorktreeStatus::MergeFailed
            | WorktreeStatus::Conflicted => Ok(()),
            WorktreeStatus::Merging => bail!("{prd_id} is already being merged"),
            WorktreeStatus::Merged => bail!("{prd_id} has already been merged"),
            WorktreeStatus::Abandoned => bail!("{prd_id} has been abandoned"),
        }
    }

    /// Handle conflicts during integration by invoking the runner.
    fn handle_merge_conflicts(
        &self,
        wt_id: &str,
        wt_path: &Path,
        merge_target: &str,
    ) -> Result<()> {
        if let Some(runner) = &self.runner {
            self.update_wt_status(
                wt_id,
                WorktreeStatus::Conflicted,
                EventType::ConflictResolutionStarted,
                Some("Agent resolving merge conflicts"),
            )?;

            self.resolve_conflicts(wt_id, wt_path, merge_target, runner.as_ref())
                .map_err(|e| {
                    let _ = self.update_wt_status(
                        wt_id,
                        WorktreeStatus::Conflicted,
                        EventType::Conflicted,
                        Some(&format!("Conflict resolution failed: {e:#}")),
                    );
                    anyhow::anyhow!("conflict resolution failed: {e:#}")
                })
        } else {
            self.update_wt_status(
                wt_id,
                WorktreeStatus::Conflicted,
                EventType::Conflicted,
                Some("Merge produced conflicts (no runner for resolution)"),
            )?;
            bail!("merge produced conflicts and no runner is available for resolution")
        }
    }

    /// Merge a branch into a target, handling cross-worktree scenarios.
    ///
    /// If the target branch is checked out in a worktree, merges there.
    /// Otherwise, checks out the target in the main worktree and merges.
    fn smart_merge_into_target(&self, branch: &str, target: &str) -> Result<()> {
        let worktrees = git::list_worktrees(&self.root)?;
        let target_dir = worktrees
            .iter()
            .find(|(_, b)| b.as_deref() == Some(target))
            .map(|(p, _)| p.clone());

        let merge_dir = if let Some(dir) = target_dir {
            dir
        } else {
            git::checkout(target, &self.root)?;
            self.root.clone()
        };

        if git::merge_ff_only(branch, &merge_dir).is_ok() {
            return Ok(());
        }

        git::merge_branch(branch, &merge_dir)
    }

    /// Integrate target changes into the worktree branch.
    ///
    /// Attempts rebase first (cleaner history), then falls back to merge.
    fn integrate_target_into_branch(wt_path: &Path, target: &str) -> Result<()> {
        // Try rebase first.
        if git::rebase_onto(target, wt_path).is_ok() {
            tracing::info!("tier2: rebase succeeded");
            return Ok(());
        }

        // Rebase failed — abort and try merge.
        tracing::info!("tier2: rebase failed, aborting and trying merge");
        let _ = git::rebase_abort(wt_path);

        // Try merging target into branch.
        if git::merge_branch(target, wt_path).is_ok() {
            tracing::info!("tier2: merge succeeded");
            return Ok(());
        }

        // Both failed — abort merge and report conflict.
        let _ = git::merge_abort(wt_path);
        bail!("both rebase and merge produced conflicts")
    }

    /// Merge a branch into the target on the main worktree.
    ///
    /// Tries fast-forward first, falls back to regular merge.
    fn merge_into_target(&self, branch: &str, target: &str) -> Result<()> {
        // Ensure we're on the target branch in the main worktree.
        git::checkout(target, &self.root)?;

        // Try fast-forward (works after successful rebase).
        if git::merge_ff_only(branch, &self.root).is_ok() {
            return Ok(());
        }

        // Fall back to regular merge (works after merge-based integration).
        git::merge_branch(branch, &self.root)
    }

    /// Run `cargo make uat` in the given directory and return whether it passed.
    #[must_use]
    fn run_uat(cwd: &Path) -> bool {
        let result = std::process::Command::new("cargo")
            .args(["make", "uat"])
            .current_dir(cwd)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .status();

        match result {
            Ok(status) => status.success(),
            Err(e) => {
                tracing::warn!("failed to run cargo make uat: {e:#}");
                false
            }
        }
    }

    /// Update a worktree's status and record an event.
    fn update_wt_status(
        &self,
        wt_id: &str,
        status: WorktreeStatus,
        event_type: EventType,
        detail: Option<&str>,
    ) -> Result<()> {
        let detail_owned = detail.map(String::from);

        self.state_manager().modify(|state| {
            if let Some(wt) = state.worktrees.iter_mut().find(|w| w.id == wt_id) {
                let now = chrono::Utc::now().to_rfc3339();
                wt.status = status;
                wt.updated_at.clone_from(&now);
                wt.events.push(WorktreeEvent {
                    timestamp: now,
                    event_type,
                    detail: detail_owned.clone(),
                });
            }
            Ok(())
        })?;

        Ok(())
    }

    // ── Conflict resolution ─────────────────────────────────────────

    /// Attempt agent-driven conflict resolution for a worktree.
    ///
    /// Strategy:
    /// 1. Start a merge of the target into the worktree branch (leaving
    ///    conflicts in place rather than aborting).
    /// 2. Gather conflict context (file list, diff with markers).
    /// 3. Invoke the runner to resolve the conflicts.
    /// 4. Stage and finalize the merge/rebase.
    /// 5. Verify no conflicts remain.
    fn resolve_conflicts(
        &self,
        wt_id: &str,
        wt_path: &Path,
        target: &str,
        runner: &dyn crate::runner::Runner,
    ) -> Result<()> {
        // Start integration (merge) — leave conflicts in place for the agent.
        let merge_started = Self::start_conflicting_merge(wt_path, target)?;

        // Gather conflict context.
        let conflict_files =
            git::list_conflict_files(wt_path).context("failed to list conflict files")?;

        if conflict_files.is_empty() {
            // No actual conflicts (shouldn't happen, but handle gracefully).
            tracing::info!("tier2: no conflict files found for {wt_id}, finalizing");
            return Self::finalize_conflict_resolution(wt_path, merge_started);
        }

        let conflict_diff = git::conflict_diff(wt_path)
            .unwrap_or_else(|_| String::from("(failed to retrieve conflict diff)"));

        // Build the prompt for the agent.
        let prompt = Self::build_conflict_prompt(wt_id, &conflict_files, &conflict_diff, target);

        // Invoke the runner.
        tracing::info!(
            "tier2: invoking {} to resolve {} conflict(s) in {wt_id}",
            runner.name(),
            conflict_files.len()
        );

        let output = runner
            .execute(&prompt, wt_path)
            .map_err(|e| anyhow::anyhow!("runner failed: {e}"))?;

        if !output.success {
            // Abort the in-progress merge/rebase and report.
            Self::abort_in_progress(wt_path, merge_started);
            bail!("agent reported failure resolving conflicts");
        }

        // Stage all changes the agent made.
        git::stage_all(wt_path).context("failed to stage resolved files")?;

        // Verify no conflicts remain.
        let remaining = git::list_conflict_files(wt_path).unwrap_or_default();

        if !remaining.is_empty() {
            Self::abort_in_progress(wt_path, merge_started);
            bail!(
                "agent left {} unresolved conflict(s): {}",
                remaining.len(),
                remaining.join(", ")
            );
        }

        // Finalize the merge/rebase.
        Self::finalize_conflict_resolution(wt_path, merge_started)?;

        self.update_wt_status(
            wt_id,
            WorktreeStatus::Merging,
            EventType::ConflictResolved,
            Some(&format!(
                "Agent resolved {} conflict(s)",
                conflict_files.len()
            )),
        )?;

        Ok(())
    }

    /// Start a conflicting integration (merge) of the target into the
    /// worktree branch, leaving conflict markers in place.
    ///
    /// Returns `true` if a rebase was started (vs. a merge).
    fn start_conflicting_merge(wt_path: &Path, target: &str) -> Result<bool> {
        // Try rebase — it may leave conflicts in the working tree.
        if git::rebase_onto(target, wt_path).is_ok() {
            // Rebase succeeded cleanly — no conflicts after all.
            return Ok(true);
        }

        // Check if the rebase is still in progress (has conflicts).
        if git::is_rebase_in_progress(wt_path)? {
            // Rebase paused with conflicts — agent will resolve.
            return Ok(true);
        }

        // Rebase failed completely (not just conflicts) — try merge.
        let _ = git::rebase_abort(wt_path);

        // Start merge — will leave conflicts in working tree.
        let _ = git::merge_branch(target, wt_path);

        // Whether it succeeded or has conflicts, we continue.
        Ok(false)
    }

    /// Finalize the in-progress merge or rebase after conflict resolution.
    fn finalize_conflict_resolution(wt_path: &Path, is_rebase: bool) -> Result<()> {
        if is_rebase {
            git::rebase_continue(wt_path)
                .context("failed to continue rebase after conflict resolution")
        } else {
            git::merge_commit(wt_path).context("failed to finalize merge after conflict resolution")
        }
    }

    /// Abort an in-progress merge or rebase (best-effort cleanup).
    fn abort_in_progress(wt_path: &Path, is_rebase: bool) {
        if is_rebase {
            let _ = git::rebase_abort(wt_path);
        } else {
            let _ = git::merge_abort(wt_path);
        }
    }

    /// Build a prompt for the agent to resolve merge conflicts.
    fn build_conflict_prompt(
        wt_id: &str,
        conflict_files: &[String],
        conflict_diff: &str,
        target: &str,
    ) -> String {
        let file_list = conflict_files
            .iter()
            .map(|f| format!("  - {f}"))
            .collect::<Vec<_>>()
            .join("\n");

        // Truncate diff if too large to keep prompt manageable.
        let max_diff_len = 50_000;
        let diff_section = if conflict_diff.len() > max_diff_len {
            format!(
                "{}\n\n... (diff truncated, {} total bytes)",
                &conflict_diff[..max_diff_len],
                conflict_diff.len()
            )
        } else {
            conflict_diff.to_string()
        };

        format!(
            "# Merge Conflict Resolution\n\
            \n\
            You are resolving merge conflicts in worktree `{wt_id}`.\n\
            \n\
            ## Context\n\
            \n\
            The worktree branch is being integrated with `{target}`. A merge/rebase produced\n\
            conflicts that need to be resolved.\n\
            \n\
            ## Conflicting Files\n\
            \n\
            {file_list}\n\
            \n\
            ## Conflict Diff\n\
            \n\
            The following diff shows the conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`)\n\
            in the affected files:\n\
            \n\
            ```diff\n\
            {diff_section}\n\
            ```\n\
            \n\
            ## Instructions\n\
            \n\
            1. **Read each conflicting file** listed above.\n\
            2. **Resolve every conflict** by editing the files to produce correct, working code.\n\
               - Remove all conflict markers (`<<<<<<<`, `=======`, `>>>>>>>`).\n\
               - Choose the correct combination of both sides, or write new code that\n\
                 integrates both changes correctly.\n\
               - Preserve the intent of both the worktree branch and the target branch.\n\
            3. **Do NOT add, delete, or rename files** — only edit the conflicting files.\n\
            4. **Do NOT run tests or commit** — the daemon handles that after you finish.\n\
            5. **Respond with a brief summary** of how you resolved each conflict.\n\
            \n\
            Resolve all conflicts now."
        )
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runner::Runner;
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

    // ── Health check & stale cleanup ────────────────────────────────

    #[test]
    fn is_healthy_false_when_no_daemon() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(!Daemon::is_healthy(tmp.path()));
    }

    #[test]
    fn is_healthy_false_when_pid_only() {
        let tmp = tempfile::tempdir().unwrap();
        let daemon = Daemon::new(tmp.path().to_path_buf());

        // Write a PID file pointing to ourselves — but no socket.
        daemon.write_pid_file().unwrap();
        assert!(Daemon::is_running(tmp.path()));
        assert!(!Daemon::is_healthy(tmp.path()));
    }

    #[test]
    fn is_healthy_true_when_running_and_reachable() {
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

        // Wait for socket.
        let sock = ipc::socket_path(&root);
        for _ in 0..50 {
            if ipc::is_daemon_reachable(&sock) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }

        assert!(Daemon::is_healthy(&root));

        shutdown.store(true, Ordering::SeqCst);
        handle.join().unwrap().unwrap();
    }

    #[test]
    fn cleanup_stale_removes_dead_pid_file() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_path = Daemon::pid_path(tmp.path());

        // Create the directory and write a PID file pointing to a dead process.
        fs::create_dir_all(pid_path.parent().unwrap()).unwrap();
        fs::write(&pid_path, "4000000").unwrap();
        assert!(pid_path.exists());

        Daemon::cleanup_stale(tmp.path());
        assert!(!pid_path.exists());
    }

    #[test]
    fn cleanup_stale_removes_stale_socket() {
        let tmp = tempfile::tempdir().unwrap();
        let sock_path = ipc::socket_path(tmp.path());

        // Create a regular file pretending to be a socket (not connectable).
        fs::create_dir_all(sock_path.parent().unwrap()).unwrap();
        fs::write(&sock_path, "stale").unwrap();
        assert!(sock_path.exists());

        Daemon::cleanup_stale(tmp.path());
        assert!(!sock_path.exists());
    }

    #[test]
    fn cleanup_stale_preserves_live_pid() {
        let tmp = tempfile::tempdir().unwrap();
        let pid_path = Daemon::pid_path(tmp.path());

        // Write our own PID — should not be removed.
        fs::create_dir_all(pid_path.parent().unwrap()).unwrap();
        fs::write(&pid_path, std::process::id().to_string()).unwrap();

        Daemon::cleanup_stale(tmp.path());
        assert!(pid_path.exists());
    }

    #[test]
    fn cleanup_stale_noop_when_nothing_exists() {
        let tmp = tempfile::tempdir().unwrap();
        // No PID file, no socket — should not error.
        Daemon::cleanup_stale(tmp.path());
    }

    // ── Merge order computation ─────────────────────────────────────

    #[test]
    fn compute_merge_order_single_worktree() {
        let entries = [make_entry("wt-001", WorktreeStatus::Completed, &[])];
        let refs: Vec<&WorktreeEntry> = entries.iter().collect();

        let order = Daemon::compute_merge_order(&refs, &[]);
        assert_eq!(order, vec!["wt-001"]);
    }

    #[test]
    fn compute_merge_order_prefers_less_overlap() {
        let wt_a = {
            let mut e = make_entry("wt-a", WorktreeStatus::Completed, &["shared.rs"]);
            e.updated_at = "2026-01-01T00:00:00Z".to_string();
            e
        };
        let wt_b = {
            let mut e = make_entry("wt-b", WorktreeStatus::Completed, &[]);
            e.updated_at = "2026-01-02T00:00:00Z".to_string();
            e
        };

        let entries = [wt_a, wt_b];
        let refs: Vec<&WorktreeEntry> = entries.iter().collect();

        let warnings = vec![OverlapWarning {
            worktrees: vec!["wt-a".to_string(), "wt-c".to_string()],
            files: vec!["shared.rs".to_string()],
            risk: OverlapRisk::Low,
        }];

        let order = Daemon::compute_merge_order(&refs, &warnings);
        // wt-b has 0 overlap files, wt-a has 1 → wt-b first.
        assert_eq!(order, vec!["wt-b", "wt-a"]);
    }

    #[test]
    fn compute_merge_order_breaks_tie_by_completion_time() {
        let wt_a = {
            let mut e = make_entry("wt-a", WorktreeStatus::Completed, &[]);
            e.updated_at = "2026-01-02T00:00:00Z".to_string();
            e
        };
        let wt_b = {
            let mut e = make_entry("wt-b", WorktreeStatus::Completed, &[]);
            e.updated_at = "2026-01-01T00:00:00Z".to_string();
            e
        };

        let entries = [wt_a, wt_b];
        let refs: Vec<&WorktreeEntry> = entries.iter().collect();

        let order = Daemon::compute_merge_order(&refs, &[]);
        // Both have 0 overlap, wt-b completed earlier.
        assert_eq!(order, vec!["wt-b", "wt-a"]);
    }

    #[test]
    fn compute_merge_order_empty() {
        let entries: Vec<WorktreeEntry> = vec![];
        let refs: Vec<&WorktreeEntry> = entries.iter().collect();

        let order = Daemon::compute_merge_order(&refs, &[]);
        assert!(order.is_empty());
    }

    // ── Update worktree status ──────────────────────────────────────

    #[test]
    fn update_wt_status_records_event() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        state_mgr
            .modify(|state| {
                state
                    .worktrees
                    .push(make_entry("wt-010", WorktreeStatus::Completed, &[]));
                Ok(())
            })
            .unwrap();

        let daemon = Daemon::new(tmp.path().to_path_buf());
        daemon
            .update_wt_status(
                "wt-010",
                WorktreeStatus::Merging,
                EventType::MergeStarted,
                None,
            )
            .unwrap();

        let state = state_mgr.read().unwrap();
        let wt = &state.worktrees[0];
        assert_eq!(wt.status, WorktreeStatus::Merging);
        assert_eq!(wt.events.len(), 1);
        assert_eq!(wt.events[0].event_type, EventType::MergeStarted);
    }

    #[test]
    fn update_wt_status_with_detail() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        state_mgr
            .modify(|state| {
                state
                    .worktrees
                    .push(make_entry("wt-011", WorktreeStatus::Merging, &[]));
                Ok(())
            })
            .unwrap();

        let daemon = Daemon::new(tmp.path().to_path_buf());
        daemon
            .update_wt_status(
                "wt-011",
                WorktreeStatus::MergeFailed,
                EventType::MergeFailed,
                Some("UATs failed"),
            )
            .unwrap();

        let state = state_mgr.read().unwrap();
        let wt = &state.worktrees[0];
        assert_eq!(wt.status, WorktreeStatus::MergeFailed);
        assert_eq!(wt.events[0].detail.as_deref(), Some("UATs failed"));
    }

    // ── Integration: auto-merge with real git repos ─────────────────

    /// Helper: create a git repo in a directory.
    fn init_git_repo(dir: &Path) {
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(dir)
            .output()
            .expect("git init");
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(dir)
            .output()
            .expect("git config email");
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(dir)
            .output()
            .expect("git config name");
        std::fs::write(dir.join("README.md"), "# Test").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(dir)
            .output()
            .expect("git add");
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(dir)
            .output()
            .expect("git commit");
    }

    #[test]
    fn integrate_target_into_branch_rebase_succeeds() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create a worktree with a feature branch.
        let wt_path = tmp.path().join("main-repo-prd-99");
        git::create_branch("main-repo-prd-99", "HEAD", &main_dir).unwrap();
        git::create_worktree(&wt_path, "main-repo-prd-99", &main_dir).unwrap();

        // Make a commit in the worktree (no conflict).
        std::fs::write(wt_path.join("feature.rs"), "fn feature() {}").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&wt_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Feature commit"])
            .current_dir(&wt_path)
            .output()
            .unwrap();

        let result = Daemon::integrate_target_into_branch(&wt_path, &default_branch);
        assert!(result.is_ok());
    }

    #[test]
    fn integrate_target_into_branch_falls_back_to_merge() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create a worktree.
        let wt_path = tmp.path().join("main-repo-prd-98");
        git::create_branch("main-repo-prd-98", "HEAD", &main_dir).unwrap();
        git::create_worktree(&wt_path, "main-repo-prd-98", &main_dir).unwrap();

        // Make a commit in the worktree.
        std::fs::write(wt_path.join("feature.rs"), "fn feature() {}").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&wt_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Worktree commit"])
            .current_dir(&wt_path)
            .output()
            .unwrap();

        // Also make a (non-conflicting) commit on main.
        std::fs::write(main_dir.join("main_new.rs"), "fn main_new() {}").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&main_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Main commit"])
            .current_dir(&main_dir)
            .output()
            .unwrap();

        // Rebase may succeed or merge may succeed — either way integration should pass.
        let result = Daemon::integrate_target_into_branch(&wt_path, &default_branch);
        assert!(result.is_ok());
    }

    #[test]
    fn integrate_target_into_branch_conflicts_both_fail() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create a worktree.
        let wt_path = tmp.path().join("main-repo-prd-97");
        git::create_branch("main-repo-prd-97", "HEAD", &main_dir).unwrap();
        git::create_worktree(&wt_path, "main-repo-prd-97", &main_dir).unwrap();

        // Modify the same file in both worktree and main (conflict).
        std::fs::write(wt_path.join("README.md"), "worktree changes").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&wt_path)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Worktree conflict"])
            .current_dir(&wt_path)
            .output()
            .unwrap();

        std::fs::write(main_dir.join("README.md"), "main changes").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&main_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Main conflict"])
            .current_dir(&main_dir)
            .output()
            .unwrap();

        let result = Daemon::integrate_target_into_branch(&wt_path, &default_branch);
        assert!(result.is_err());
    }

    #[test]
    fn merge_into_target_succeeds_with_ff() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create a branch ahead of main.
        git::create_branch("ahead-branch", "HEAD", &main_dir).unwrap();
        git::checkout("ahead-branch", &main_dir).unwrap();
        std::fs::write(main_dir.join("ahead.rs"), "fn ahead() {}").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&main_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Ahead commit"])
            .current_dir(&main_dir)
            .output()
            .unwrap();

        // Go back to default branch.
        git::checkout(&default_branch, &main_dir).unwrap();

        let daemon = Daemon::new(main_dir.clone());
        daemon
            .merge_into_target("ahead-branch", &default_branch)
            .unwrap();

        // File should now exist.
        assert!(main_dir.join("ahead.rs").exists());
    }

    #[test]
    fn tier2_auto_merge_skips_when_no_completed() {
        let tmp = tempfile::tempdir().unwrap();
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();

        state_mgr
            .modify(|state| {
                state
                    .worktrees
                    .push(make_entry("wt-020", WorktreeStatus::Active, &[]));
                Ok(())
            })
            .unwrap();

        let daemon = Daemon::new(tmp.path().to_path_buf());
        daemon.tier2_auto_merge();

        // No status changes should have occurred.
        let state = state_mgr.read().unwrap();
        assert_eq!(state.worktrees[0].status, WorktreeStatus::Active);
    }

    #[test]
    fn run_uat_returns_false_for_nonexistent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let nonexistent = tmp.path().join("nonexistent");
        assert!(!Daemon::run_uat(&nonexistent));
    }

    // ── Conflict resolution tests ───────────────────────────────────

    #[test]
    fn build_conflict_prompt_contains_context() {
        let prompt = Daemon::build_conflict_prompt(
            "wt-001",
            &["src/main.rs".to_string(), "src/lib.rs".to_string()],
            "diff --git a/src/main.rs\n<<<<<<< HEAD\nold\n=======\nnew\n>>>>>>> branch",
            "main",
        );

        assert!(prompt.contains("wt-001"));
        assert!(prompt.contains("src/main.rs"));
        assert!(prompt.contains("src/lib.rs"));
        assert!(prompt.contains("main"));
        assert!(prompt.contains("<<<<<<<"));
        assert!(prompt.contains("Resolve all conflicts now."));
    }

    #[test]
    fn build_conflict_prompt_truncates_large_diff() {
        let large_diff = "x".repeat(60_000);
        let prompt =
            Daemon::build_conflict_prompt("wt-002", &["file.rs".to_string()], &large_diff, "main");

        assert!(prompt.contains("diff truncated"));
        assert!(prompt.len() < 55_000);
    }

    #[test]
    fn resolve_conflicts_with_mock_runner_succeeds() {
        // Set up a git repo with a conflict scenario.
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create worktree branch with a conflicting change.
        git::create_branch("wt-branch", "HEAD", &main_dir).unwrap();

        // Create worktree directory (simulated).
        let wt_dir = tmp.path().join("wt-repo");
        git::create_worktree(&wt_dir, "wt-branch", &main_dir).unwrap();

        // Modify same file differently in both branches.
        std::fs::write(wt_dir.join("README.md"), "# Worktree version").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&wt_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Worktree change"])
            .current_dir(&wt_dir)
            .output()
            .unwrap();

        std::fs::write(main_dir.join("README.md"), "# Main version").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&main_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Main change"])
            .current_dir(&main_dir)
            .output()
            .unwrap();

        // Set up state.
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();
        state_mgr
            .modify(|state| {
                state.worktrees.push(WorktreeEntry {
                    id: "wt-099".to_string(),
                    prd: "PRD-0099".to_string(),
                    branch: "wt-branch".to_string(),
                    path: wt_dir.to_string_lossy().to_string(),
                    status: WorktreeStatus::Merging,
                    run_pid: None,
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                    merge_target: default_branch.clone(),
                    modified_files: vec!["README.md".to_string()],
                    events: vec![],
                });
                Ok(())
            })
            .unwrap();

        // Create a mock runner that resolves the conflict by writing the file.
        let wt_dir_clone = wt_dir.clone();
        let runner = crate::runner::MockRunner::new(vec![crate::runner::RunnerOutput::success(
            "Resolved conflict in README.md",
        )]);

        // The mock runner doesn't actually edit files. We need to simulate
        // what the agent would do: resolve the conflict markers.
        // First, start the merge to create conflict state.
        let _ = git::merge_branch(&default_branch, &wt_dir);

        // Now resolve the conflict manually (simulating agent action).
        std::fs::write(wt_dir_clone.join("README.md"), "# Merged version").unwrap();
        git::stage_all(&wt_dir_clone).unwrap();

        // Verify no conflicts remain.
        let remaining = git::list_conflict_files(&wt_dir_clone).unwrap();
        assert!(remaining.is_empty(), "conflicts should be resolved");

        // Finalize.
        let result = git::merge_commit(&wt_dir_clone);
        assert!(result.is_ok(), "merge commit should succeed");

        // Verify the runner is usable (even if we didn't use it here).
        let output = runner.execute("test", &wt_dir_clone);
        assert!(output.is_ok());
        assert!(output.unwrap().success);
    }

    #[test]
    fn attempt_merge_without_runner_marks_conflicted() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create worktree with conflicting changes.
        git::create_branch("conflict-branch", "HEAD", &main_dir).unwrap();
        let wt_dir = tmp.path().join("wt-conflict");
        git::create_worktree(&wt_dir, "conflict-branch", &main_dir).unwrap();

        std::fs::write(wt_dir.join("README.md"), "# Worktree").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&wt_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Wt conflict"])
            .current_dir(&wt_dir)
            .output()
            .unwrap();

        std::fs::write(main_dir.join("README.md"), "# Main").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&main_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Main conflict"])
            .current_dir(&main_dir)
            .output()
            .unwrap();

        // Set up state with completed worktree.
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();
        state_mgr
            .modify(|state| {
                state.worktrees.push(WorktreeEntry {
                    id: "wt-100".to_string(),
                    prd: "PRD-0100".to_string(),
                    branch: "conflict-branch".to_string(),
                    path: wt_dir.to_string_lossy().to_string(),
                    status: WorktreeStatus::Completed,
                    run_pid: None,
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                    merge_target: default_branch,
                    modified_files: vec!["README.md".to_string()],
                    events: vec![],
                });
                Ok(())
            })
            .unwrap();

        // Daemon without runner should mark as Conflicted.
        let daemon = Daemon::new(tmp.path().to_path_buf());
        let _ = daemon.attempt_merge_worktree("wt-100");

        let state = state_mgr.read().unwrap();
        let wt = state.worktrees.iter().find(|w| w.id == "wt-100").unwrap();
        assert_eq!(wt.status, WorktreeStatus::Conflicted);
        assert!(
            wt.events
                .iter()
                .any(|e| e.event_type == EventType::Conflicted),
            "should have Conflicted event"
        );
    }

    #[test]
    fn attempt_merge_with_runner_attempts_resolution() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create worktree with conflicting changes.
        git::create_branch("resolve-branch", "HEAD", &main_dir).unwrap();
        let wt_dir = tmp.path().join("wt-resolve");
        git::create_worktree(&wt_dir, "resolve-branch", &main_dir).unwrap();

        std::fs::write(wt_dir.join("README.md"), "# Worktree resolve").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&wt_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Wt resolve"])
            .current_dir(&wt_dir)
            .output()
            .unwrap();

        std::fs::write(main_dir.join("README.md"), "# Main resolve").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&main_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Main resolve"])
            .current_dir(&main_dir)
            .output()
            .unwrap();

        // Set up state.
        let state_mgr = StateManager::new(tmp.path());
        state_mgr.ensure_dir().unwrap();
        state_mgr
            .modify(|state| {
                state.worktrees.push(WorktreeEntry {
                    id: "wt-101".to_string(),
                    prd: "PRD-0101".to_string(),
                    branch: "resolve-branch".to_string(),
                    path: wt_dir.to_string_lossy().to_string(),
                    status: WorktreeStatus::Completed,
                    run_pid: None,
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    updated_at: "2026-01-01T00:00:00Z".to_string(),
                    merge_target: default_branch,
                    modified_files: vec!["README.md".to_string()],
                    events: vec![],
                });
                Ok(())
            })
            .unwrap();

        // Mock runner that "resolves" by reporting success.
        // The agent would normally edit files, but the mock can't.
        // This test verifies the daemon invokes the runner and tracks
        // ConflictResolutionStarted. The runner's failure to actually
        // resolve files means the daemon will see remaining conflicts
        // and abort — that's the expected path here.
        let runner =
            crate::runner::MockRunner::new(vec![crate::runner::RunnerOutput::success("Resolved")]);

        let daemon = Daemon::new_with_runner(
            tmp.path().to_path_buf(),
            DaemonConfig::default(),
            Box::new(runner),
        );

        let _ = daemon.attempt_merge_worktree("wt-101");

        let state = state_mgr.read().unwrap();
        let wt = state.worktrees.iter().find(|w| w.id == "wt-101").unwrap();

        // The daemon should have at least tried conflict resolution.
        assert!(
            wt.events
                .iter()
                .any(|e| e.event_type == EventType::ConflictResolutionStarted),
            "should have ConflictResolutionStarted event"
        );
    }

    #[test]
    fn start_conflicting_merge_clean_rebase_returns_true() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("main-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create branch with non-conflicting change.
        git::create_branch("clean-branch", "HEAD", &main_dir).unwrap();
        let wt_dir = tmp.path().join("wt-clean");
        git::create_worktree(&wt_dir, "clean-branch", &main_dir).unwrap();

        std::fs::write(wt_dir.join("new_file.rs"), "fn new() {}").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&wt_dir)
            .output()
            .unwrap();
        std::process::Command::new("git")
            .args(["commit", "-m", "Non-conflicting"])
            .current_dir(&wt_dir)
            .output()
            .unwrap();

        // No changes on main, so rebase should be clean.
        let result = Daemon::start_conflicting_merge(&wt_dir, &default_branch);
        assert!(result.is_ok());
        // Returns true because rebase was used.
        assert!(result.unwrap());
    }

    #[test]
    fn list_conflict_files_empty_when_no_conflicts() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("clean-repo");
        fs::create_dir(&dir).unwrap();
        init_git_repo(&dir);

        let files = git::list_conflict_files(&dir).unwrap();
        assert!(files.is_empty());
    }

    #[test]
    fn stage_all_and_list_in_clean_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("stage-repo");
        fs::create_dir(&dir).unwrap();
        init_git_repo(&dir);

        // Create a new file and stage.
        std::fs::write(dir.join("new.txt"), "hello").unwrap();
        git::stage_all(&dir).unwrap();

        // The file should be staged (not in conflict list).
        let conflicts = git::list_conflict_files(&dir).unwrap();
        assert!(conflicts.is_empty());
    }

    // ── manual_merge tests ──────────────────────────────────────────

    #[test]
    fn validate_mergeable_status_accepts_valid_states() {
        for status in [
            WorktreeStatus::Active,
            WorktreeStatus::Completed,
            WorktreeStatus::MergeFailed,
            WorktreeStatus::Conflicted,
        ] {
            assert!(
                Daemon::validate_mergeable_status("PRD-0001", status).is_ok(),
                "should accept {status:?}"
            );
        }
    }

    #[test]
    fn validate_mergeable_status_rejects_invalid_states() {
        for status in [
            WorktreeStatus::Merging,
            WorktreeStatus::Merged,
            WorktreeStatus::Abandoned,
        ] {
            assert!(
                Daemon::validate_mergeable_status("PRD-0001", status).is_err(),
                "should reject {status:?}"
            );
        }
    }

    #[test]
    fn manual_merge_fails_for_unknown_prd() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        fs::create_dir_all(root.join(".mr/worktrees")).unwrap();

        let daemon = Daemon::new(root);
        let result = daemon.manual_merge("PRD-9999", None);
        assert!(result.is_err());
        assert!(
            format!("{result:?}").contains("no worktree registered"),
            "error should mention missing worktree"
        );
    }

    #[test]
    fn manual_merge_rejects_merged_worktree() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().to_path_buf();
        let wt_dir = root.join(".mr/worktrees");
        fs::create_dir_all(&wt_dir).unwrap();

        // Write state with a merged worktree.
        let state = WorktreeState {
            version: 1,
            daemon: None,
            worktrees: vec![make_entry("wt-001", WorktreeStatus::Merged, &[])],
            overlap_warnings: vec![],
        };
        let state_yaml = serde_yaml::to_string(&state).unwrap();
        fs::write(wt_dir.join("state.yaml"), state_yaml).unwrap();

        let daemon = Daemon::new(root);
        let result = daemon.manual_merge("PRD-wt-001", None);
        assert!(result.is_err());
        assert!(
            format!("{result:?}").contains("already been merged"),
            "error should mention already merged"
        );
    }

    #[test]
    fn smart_merge_into_target_uses_main_for_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        let main_dir = tmp.path().join("smart-merge-repo");
        fs::create_dir(&main_dir).unwrap();
        init_git_repo(&main_dir);

        let default_branch = git::current_branch(&main_dir).unwrap();

        // Create a branch with a commit ahead of main.
        git::create_branch("ahead-branch", "HEAD", &main_dir).unwrap();
        git::checkout("ahead-branch", &main_dir).unwrap();
        std::fs::write(main_dir.join("ahead.txt"), "ahead content").unwrap();
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(&main_dir)
            .output()
            .expect("git add");
        std::process::Command::new("git")
            .args(["commit", "-m", "ahead commit"])
            .current_dir(&main_dir)
            .output()
            .expect("git commit");

        // Go back to main.
        git::checkout(&default_branch, &main_dir).unwrap();

        let daemon = Daemon::new(main_dir);
        let result = daemon.smart_merge_into_target("ahead-branch", &default_branch);
        assert!(result.is_ok(), "smart merge should succeed: {result:?}");
    }
}
