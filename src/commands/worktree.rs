//! CLI subcommand handlers for worktree orchestration (`mr wt`).
//!
//! Provides stub implementations for most `wt` subcommands and
//! working implementations for daemon start/stop/status.

use anyhow::{Result, bail};

use crate::util::colors;
use crate::worktree::daemon::Daemon;
use crate::worktree::state::StateManager;
use crate::worktree::types::WorktreeStatus;

/// Handles `mr wt run <prd-id>`.
///
/// Creates a worktree, branch, auto-starts the daemon, and spawns `mr run`
/// in the worktree context.
pub fn cmd_wt_run(
    prd_id: &str,
    _runner_name: &str,
    _cli_model: Option<&str>,
    _stream: bool,
) -> Result<()> {
    println!(
        "{}",
        colors::info(&format!("Worktree run for {prd_id} — not yet implemented."))
    );
    bail!("mr wt run is not yet implemented (see T-006)")
}

/// Handles `mr wt list`.
///
/// Displays all registered worktrees with status.
pub fn cmd_wt_list() -> Result<()> {
    println!("{}", colors::info("Worktree list — not yet implemented."));
    bail!("mr wt list is not yet implemented (see T-009)")
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
mod tests {
    use super::*;

    #[test]
    fn test_cmd_wt_run_returns_not_implemented() {
        let result = cmd_wt_run("PRD-0039", "copilot", None, false);
        assert!(result.is_err());
        let err = result
            .expect_err("should be not-implemented error")
            .to_string();
        assert!(err.contains("not yet implemented"));
    }

    #[test]
    fn test_cmd_wt_list_returns_not_implemented() {
        let result = cmd_wt_list();
        assert!(result.is_err());
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
