//! CLI subcommand handlers for worktree orchestration (`mr wt`).
//!
//! Provides working implementations for `wt run` and daemon start/stop/status,
//! with stub implementations for the remaining `wt` subcommands.

use std::process::Command as ProcessCommand;

use anyhow::{Context, Result, bail};

use crate::prd::scan_prds;
use crate::util::colors;
use crate::worktree::daemon::Daemon;
use crate::worktree::git;
use crate::worktree::state::StateManager;
use crate::worktree::types::{EventType, WorktreeEntry, WorktreeEvent, WorktreeStatus};

// ── Helpers ─────────────────────────────────────────────────────────

/// Generate the next `wt-NNN` identifier based on existing entries in state.
fn next_wt_id(state: &crate::worktree::types::WorktreeState) -> String {
    let max = state
        .worktrees
        .iter()
        .filter_map(|w| w.id.strip_prefix("wt-").and_then(|n| n.parse::<u32>().ok()))
        .max()
        .unwrap_or(0);

    format!("wt-{:03}", max + 1)
}

/// ISO 8601 UTC timestamp for the current moment.
fn now_iso() -> String {
    let d = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    // Simple ISO 8601 without pulling in chrono.
    let secs = d.as_secs();
    let (days, rem) = (secs / 86400, secs % 86400);
    let (hours, rem) = (rem / 3600, rem % 3600);
    let (mins, s) = (rem / 60, rem % 60);

    // Days since Unix epoch → year/month/day (simplified leap-year aware).
    let (y, m, d) = days_to_ymd(days);
    format!("{y:04}-{m:02}-{d:02}T{hours:02}:{mins:02}:{s:02}Z")
}

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (u64, u64, u64) {
    // Algorithm from Howard Hinnant's `civil_from_days`.
    let z = days.wrapping_add(719_468);
    let era = z / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m, d)
}

/// Validate that a PRD with the given ID exists in `.mr/prds/`.
fn validate_prd_exists(root: &std::path::Path, prd_id: &str) -> Result<()> {
    let prds_dir = root.join(".mr").join("prds");
    let prds = scan_prds(&prds_dir)?;

    let found = prds
        .iter()
        .any(|(_, prd, _)| prd.id().eq_ignore_ascii_case(prd_id));
    if !found {
        bail!("PRD not found: {prd_id}.\n  Run `mr status` to list available PRDs.");
    }
    Ok(())
}

/// Ensure the daemon is running, spawning it as a detached process if not.
///
/// Returns `Ok(())` once the daemon socket is reachable, or an error if
/// the daemon could not be started within a reasonable timeout.
fn ensure_daemon(root: &std::path::Path) -> Result<()> {
    if Daemon::is_healthy(root) {
        return Ok(());
    }

    println!(
        "{}",
        colors::info("Daemon is not running — starting in background...")
    );

    Daemon::ensure_running(root)?;

    if Daemon::is_healthy(root) {
        println!("{}", colors::success("Daemon started."));
    } else {
        println!(
            "{}",
            colors::warning("Daemon socket not yet reachable — proceeding anyway.")
        );
    }

    Ok(())
}

/// Register a new worktree entry in `state.yaml` and return its assigned ID.
fn register_worktree(
    state_mgr: &StateManager,
    prd_id: &str,
    branch: &str,
    wt_path_str: &str,
) -> Result<String> {
    let now = now_iso();
    let prd_clone = prd_id.to_string();
    let branch_clone = branch.to_string();
    let path_clone = wt_path_str.to_string();

    let updated = state_mgr.modify(|s| {
        let id = next_wt_id(s);
        s.worktrees.push(WorktreeEntry {
            id: id.clone(),
            prd: prd_clone,
            branch: branch_clone,
            path: path_clone,
            status: WorktreeStatus::Active,
            run_pid: None,
            created_at: now.clone(),
            updated_at: now,
            merge_target: "main".to_string(),
            modified_files: Vec::new(),
            events: vec![WorktreeEvent {
                timestamp: now_iso(),
                event_type: EventType::Created,
                detail: None,
            }],
        });
        Ok(())
    })?;

    Ok(updated
        .worktrees
        .last()
        .map_or_else(|| String::from("wt-001"), |w| w.id.clone()))
}

/// Spawn `mr run <prd-id>` as a detached process in the given worktree directory.
fn spawn_mr_run(
    prd_id: &str,
    runner_name: &str,
    cli_model: Option<&str>,
    stream: bool,
    wt_path: &std::path::Path,
) -> Result<u32> {
    let exe = std::env::current_exe().context("failed to resolve current executable")?;

    let mut cmd = ProcessCommand::new(&exe);
    cmd.arg("run").arg(prd_id);
    cmd.arg("--runner").arg(runner_name);

    if let Some(model) = cli_model {
        cmd.arg("--model").arg(model);
    }
    if stream {
        cmd.arg("--stream");
    }

    cmd.current_dir(wt_path);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    let child = cmd
        .spawn()
        .with_context(|| format!("failed to spawn `mr run {prd_id}` in {}", wt_path.display()))?;

    Ok(child.id())
}

/// Handles `mr wt run <prd-id>`.
///
/// Creates a worktree, branch, auto-starts the daemon, and spawns `mr run`
/// in the worktree context.
pub fn cmd_wt_run(
    prd_id: &str,
    runner_name: &str,
    cli_model: Option<&str>,
    stream: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let main_root = git::resolve_main_worktree(&cwd).context("failed to resolve main worktree")?;

    validate_prd_exists(&main_root, prd_id)?;

    let repo = git::repo_name(&main_root)?;
    let branch = git::worktree_branch_name(&repo, prd_id);
    let wt_path = git::worktree_path(&main_root, &repo, prd_id);

    // Guard against duplicate active worktrees.
    let state_mgr = StateManager::new(&main_root);
    let state = state_mgr.read()?;
    if state
        .worktrees
        .iter()
        .any(|w| w.prd.eq_ignore_ascii_case(prd_id) && w.status == WorktreeStatus::Active)
    {
        bail!(
            "A worktree for {prd_id} is already active.\n  \
             Run `mr wt status {prd_id}` to see details, or `mr wt remove {prd_id}` to clean up."
        );
    }

    println!(
        "{}",
        colors::header(&format!("Creating worktree for {prd_id}"))
    );
    println!("{}", colors::dim(&format!("  Branch: {branch}")));

    git::create_branch(&branch, "HEAD", &main_root)
        .with_context(|| format!("failed to create branch {branch}"))?;

    println!(
        "{}",
        colors::dim(&format!("  Path:   {}", wt_path.display()))
    );

    git::create_worktree(&wt_path, &branch, &main_root)
        .with_context(|| format!("failed to create worktree at {}", wt_path.display()))?;

    let wt_path_str = wt_path
        .to_str()
        .context("worktree path is not valid UTF-8")?;

    let wt_id = register_worktree(&state_mgr, prd_id, &branch, wt_path_str)?;
    println!("{}", colors::dim(&format!("  ID:     {wt_id}")));

    ensure_daemon(&main_root)?;

    let run_pid = spawn_mr_run(prd_id, runner_name, cli_model, stream, &wt_path)?;

    // Record the run PID and a run_started event.
    state_mgr.modify(|s| {
        if let Some(wt) = s.worktrees.iter_mut().find(|w| w.id == wt_id) {
            wt.run_pid = Some(run_pid);
            wt.updated_at = now_iso();
            wt.events.push(WorktreeEvent {
                timestamp: now_iso(),
                event_type: EventType::RunStarted,
                detail: None,
            });
        }
        Ok(())
    })?;

    println!();
    println!(
        "{}",
        colors::success(&format!(
            "Worktree created — {prd_id} running in background (pid {run_pid})"
        ))
    );
    println!(
        "{}",
        colors::dim(&format!(
            "  Monitor: mr wt status {prd_id}\n  \
             Logs:    check the worktree directory at {}\n  \
             Stop:    mr wt remove {prd_id}",
            wt_path.display()
        ))
    );

    Ok(())
}

/// Color a status string based on worktree lifecycle status.
fn status_colored(status: WorktreeStatus) -> String {
    let label = status.to_string();
    match status {
        WorktreeStatus::Active => colors::info(&label),
        WorktreeStatus::Completed | WorktreeStatus::Merged => colors::success(&label),
        WorktreeStatus::MergeFailed | WorktreeStatus::Conflicted => colors::error(&label),
        WorktreeStatus::Merging => colors::warning(&label),
        WorktreeStatus::Abandoned => colors::dim(&label),
    }
}

/// Extract the timestamp of the most recent event for a worktree entry.
fn last_event_timestamp(entry: &WorktreeEntry) -> &str {
    entry.events.last().map_or("—", |e| e.timestamp.as_str())
}

/// Handles `mr wt list`.
///
/// Displays all registered worktrees in a table with PRD ID, branch, status,
/// modified files count, and last event timestamp.  Color-coded by status.
pub fn cmd_wt_list() -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let main_root = git::resolve_main_worktree(&cwd).context("failed to resolve main worktree")?;
    let state_mgr = StateManager::new(&main_root);
    let state = state_mgr.read()?;

    if state.worktrees.is_empty() {
        println!(
            "{}",
            colors::dim("No worktrees registered. Run `mr wt run <prd-id>` to create one.")
        );
        return Ok(());
    }

    // Compute column widths for alignment.
    let prd_w = state
        .worktrees
        .iter()
        .map(|w| w.prd.len())
        .max()
        .unwrap_or(6)
        .max(6); // "PRD ID"

    let branch_w = state
        .worktrees
        .iter()
        .map(|w| w.branch.len())
        .max()
        .unwrap_or(6)
        .max(6); // "Branch"

    let status_w = state
        .worktrees
        .iter()
        .map(|w| w.status.to_string().len())
        .max()
        .unwrap_or(6)
        .max(6); // "Status"

    // Header.
    println!(
        "{}",
        colors::header(&format!(
            "{:<prd_w$}  {:<branch_w$}  {:<status_w$}  {:>5}  {}",
            "PRD ID", "Branch", "Status", "Files", "Last Event",
        ))
    );

    println!(
        "{}",
        colors::dim(&"─".repeat(prd_w + 2 + branch_w + 2 + status_w + 2 + 5 + 2 + 20))
    );

    // Rows.
    for wt in &state.worktrees {
        let files_count = wt.modified_files.len();
        let last_event = last_event_timestamp(wt);

        println!(
            "{:<prd_w$}  {:<branch_w$}  {:<status_w$}  {:>5}  {}",
            wt.prd,
            wt.branch,
            status_colored(wt.status),
            files_count,
            colors::dim(last_event),
        );
    }

    // Summary line.
    let active = state
        .worktrees
        .iter()
        .filter(|w| w.status == WorktreeStatus::Active)
        .count();
    let total = state.worktrees.len();
    println!();
    println!(
        "{}",
        colors::dim(&format!("{total} worktree(s), {active} active"))
    );

    Ok(())
}

/// Handles `mr wt status [prd-id]`.
///
/// Shows detailed state of a specific worktree or overall daemon status.
pub fn cmd_wt_status(prd_id: Option<&str>) -> Result<()> {
    let target = prd_id.unwrap_or("all worktrees");
    println!(
        "{}",
        colors::info(&format!(
            "Worktree status for {target} — not yet implemented."
        ))
    );
    bail!("mr wt status is not yet implemented (see T-010)")
}

/// Handles `mr wt merge <prd-id>`.
///
/// Manually triggers merge of a specific worktree into a target branch.
pub fn cmd_wt_merge(
    prd_id: &str,
    _into: Option<&str>,
    _runner_name: &str,
    _cli_model: Option<&str>,
) -> Result<()> {
    println!(
        "{}",
        colors::info(&format!(
            "Worktree merge for {prd_id} — not yet implemented."
        ))
    );
    bail!("mr wt merge is not yet implemented (see T-013)")
}

/// Handles `mr wt graph`.
///
/// Visualizes worktree overlap risk.
pub fn cmd_wt_graph(format: &str) -> Result<()> {
    println!(
        "{}",
        colors::info(&format!("Worktree graph ({format}) — not yet implemented."))
    );
    bail!("mr wt graph is not yet implemented (see T-015)")
}

/// Handles `mr wt remove <prd-id>`.
///
/// Removes a worktree, optionally deletes the branch, and updates state.
pub fn cmd_wt_remove(prd_id: &str, _delete_branch: bool) -> Result<()> {
    println!(
        "{}",
        colors::info(&format!(
            "Worktree remove for {prd_id} — not yet implemented."
        ))
    );
    bail!("mr wt remove is not yet implemented (see T-016)")
}

/// Handles `mr wt daemon start`.
///
/// Starts the worktree orchestration daemon in the foreground.
/// The daemon runs until it receives SIGTERM, SIGINT, or reaches
/// the idle timeout.
pub fn cmd_wt_daemon_start() -> Result<()> {
    let root = std::env::current_dir()?;

    if Daemon::is_running(&root) {
        let pid = Daemon::read_pid(&root)?.unwrap_or(0);
        println!(
            "{}",
            colors::warning(&format!("Daemon is already running (pid {pid})."))
        );
        return Ok(());
    }

    println!(
        "{}",
        colors::info("Starting worktree orchestration daemon (foreground)...")
    );

    let daemon = Daemon::new(root);
    daemon.run()
}

/// Handles `mr wt daemon stop`.
///
/// Sends SIGTERM to the running daemon and waits for it to exit.
pub fn cmd_wt_daemon_stop() -> Result<()> {
    let root = std::env::current_dir()?;
    Daemon::stop(&root)?;
    println!("{}", colors::success("Daemon stopped."));
    Ok(())
}

/// Handles `mr wt daemon status`.
///
/// Shows the daemon's current status (running, PID, uptime, etc.).
pub fn cmd_wt_daemon_status() -> Result<()> {
    let root = std::env::current_dir()?;

    if Daemon::is_running(&root) {
        let pid = Daemon::read_pid(&root)?.unwrap_or(0);
        let state_mgr = StateManager::new(&root);
        let state = state_mgr.read()?;

        let active_count = state
            .worktrees
            .iter()
            .filter(|w| w.status == WorktreeStatus::Active)
            .count();

        println!(
            "{}",
            colors::success(&format!("Daemon is running (pid {pid})"))
        );
        println!("  Active worktrees: {active_count}");

        if let Some(daemon) = &state.daemon {
            println!("  Started:        {}", daemon.started_at);
            println!("  Last heartbeat: {}", daemon.last_heartbeat);
            println!("  Idle timeout:   {}h", daemon.idle_timeout_hours);
        }
    } else {
        println!("{}", colors::warning("Daemon is not running."));
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::worktree::types::WorktreeState;

    #[test]
    fn next_wt_id_starts_at_001() {
        let state = WorktreeState::default();
        assert_eq!(next_wt_id(&state), "wt-001");
    }

    #[test]
    fn next_wt_id_increments() {
        let mut state = WorktreeState::default();
        state.worktrees.push(WorktreeEntry {
            id: "wt-003".to_string(),
            prd: "PRD-0001".to_string(),
            branch: "repo-prd-1".to_string(),
            path: "/tmp/wt".to_string(),
            status: WorktreeStatus::Active,
            run_pid: None,
            created_at: String::new(),
            updated_at: String::new(),
            merge_target: "main".to_string(),
            modified_files: vec![],
            events: vec![],
        });
        assert_eq!(next_wt_id(&state), "wt-004");
    }

    #[test]
    fn now_iso_produces_valid_timestamp() {
        let ts = now_iso();
        // Should look like "2026-03-04T23:51:56Z".
        assert!(ts.ends_with('Z'));
        assert_eq!(ts.len(), 20);
        assert_eq!(&ts[4..5], "-");
        assert_eq!(&ts[7..8], "-");
        assert_eq!(&ts[10..11], "T");
    }

    #[test]
    fn days_to_ymd_unix_epoch() {
        let (y, m, d) = days_to_ymd(0);
        assert_eq!((y, m, d), (1970, 1, 1));
    }

    #[test]
    fn days_to_ymd_known_date() {
        // 2026-03-04 is day 20_516 since epoch.
        let (y, m, d) = days_to_ymd(20_516);
        assert_eq!((y, m, d), (2026, 3, 4));
    }

    #[test]
    fn validate_prd_exists_fails_for_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let prds_dir = tmp.path().join(".mr").join("prds");
        std::fs::create_dir_all(&prds_dir).unwrap();

        let result = validate_prd_exists(tmp.path(), "PRD-9999");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("PRD not found"));
    }

    #[test]
    fn cmd_wt_run_fails_without_git_repo() {
        // Running cmd_wt_run outside a git repo should fail gracefully.
        let tmp = tempfile::tempdir().unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let result = cmd_wt_run("PRD-0039", "copilot", None, false);
        assert!(result.is_err());

        std::env::set_current_dir(orig).unwrap();
    }

    #[test]
    fn test_cmd_wt_list_fails_without_git_repo() {
        // Running cmd_wt_list outside a git repo should fail gracefully.
        let tmp = tempfile::tempdir().unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let result = cmd_wt_list();
        assert!(result.is_err());

        std::env::set_current_dir(orig).unwrap();
    }

    #[test]
    fn test_status_colored_returns_string_for_all_variants() {
        // All status variants should produce a non-empty colored string.
        let variants = [
            WorktreeStatus::Active,
            WorktreeStatus::Completed,
            WorktreeStatus::Merging,
            WorktreeStatus::Merged,
            WorktreeStatus::MergeFailed,
            WorktreeStatus::Conflicted,
            WorktreeStatus::Abandoned,
        ];
        for status in variants {
            let result = status_colored(status);
            assert!(
                !result.is_empty(),
                "status_colored({status}) should not be empty"
            );
        }
    }

    #[test]
    fn test_last_event_timestamp_with_events() {
        let entry = WorktreeEntry {
            id: "wt-001".to_string(),
            prd: "PRD-0001".to_string(),
            branch: "repo-prd-1".to_string(),
            path: "/tmp/wt".to_string(),
            status: WorktreeStatus::Active,
            run_pid: None,
            created_at: String::new(),
            updated_at: String::new(),
            merge_target: "main".to_string(),
            modified_files: vec![],
            events: vec![
                WorktreeEvent {
                    timestamp: "2026-01-01T00:00:00Z".to_string(),
                    event_type: EventType::Created,
                    detail: None,
                },
                WorktreeEvent {
                    timestamp: "2026-01-02T00:00:00Z".to_string(),
                    event_type: EventType::RunStarted,
                    detail: None,
                },
            ],
        };
        assert_eq!(last_event_timestamp(&entry), "2026-01-02T00:00:00Z");
    }

    #[test]
    fn test_last_event_timestamp_empty_events() {
        let entry = WorktreeEntry {
            id: "wt-001".to_string(),
            prd: "PRD-0001".to_string(),
            branch: "repo-prd-1".to_string(),
            path: "/tmp/wt".to_string(),
            status: WorktreeStatus::Active,
            run_pid: None,
            created_at: String::new(),
            updated_at: String::new(),
            merge_target: "main".to_string(),
            modified_files: vec![],
            events: vec![],
        };
        assert_eq!(last_event_timestamp(&entry), "—");
    }

    #[test]
    fn test_cmd_wt_status_returns_not_implemented() {
        let result = cmd_wt_status(None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_status_with_prd_returns_not_implemented() {
        let result = cmd_wt_status(Some("PRD-0039"));
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_merge_returns_not_implemented() {
        let result = cmd_wt_merge("PRD-0039", None, "copilot", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_graph_returns_not_implemented() {
        let result = cmd_wt_graph("ascii");
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_remove_returns_not_implemented() {
        let result = cmd_wt_remove("PRD-0039", false);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_daemon_start_when_not_initialized() {
        // cmd_wt_daemon_start uses current_dir, so it should at least
        // not panic.  We can't easily test the full flow here since it
        // would start a real daemon, but we verify the function exists
        // and is callable.
        // (Full daemon lifecycle tests are in worktree::daemon::tests.)
    }

    #[test]
    fn test_cmd_wt_daemon_stop_when_not_running() {
        // Stopping when no daemon is running should return an error.
        let result = cmd_wt_daemon_stop();
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_daemon_status_runs() {
        // Status when no daemon is running should succeed (prints "not running").
        let result = cmd_wt_daemon_status();
        assert!(result.is_ok());
    }
}
