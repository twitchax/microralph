//! CLI subcommand handlers for worktree orchestration (`mr wt`).
//!
//! Provides stub implementations for all `wt` subcommands.
//! Each handler will be filled in by subsequent tasks.

use anyhow::{Result, bail};

use crate::util::colors;

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
/// Starts the worktree orchestration daemon.
pub fn cmd_wt_daemon_start() -> Result<()> {
    println!("{}", colors::info("Daemon start — not yet implemented."));
    bail!("mr wt daemon start is not yet implemented (see T-005)")
}

/// Handles `mr wt daemon stop`.
///
/// Stops the running daemon.
pub fn cmd_wt_daemon_stop() -> Result<()> {
    println!("{}", colors::info("Daemon stop — not yet implemented."));
    bail!("mr wt daemon stop is not yet implemented (see T-005)")
}

/// Handles `mr wt daemon status`.
///
/// Shows the daemon's current status (running, PID, uptime, etc.).
pub fn cmd_wt_daemon_status() -> Result<()> {
    println!("{}", colors::info("Daemon status — not yet implemented."));
    bail!("mr wt daemon status is not yet implemented (see T-005)")
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
    fn test_cmd_wt_daemon_start_returns_not_implemented() {
        let result = cmd_wt_daemon_start();
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_daemon_stop_returns_not_implemented() {
        let result = cmd_wt_daemon_stop();
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_daemon_status_returns_not_implemented() {
        let result = cmd_wt_daemon_status();
        assert!(result.is_err());
    }
}
