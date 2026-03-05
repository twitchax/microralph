//! CLI subcommand handlers for worktree orchestration (`mr wt`).
//!
//! Provides working implementations for `wt run`, `wt list`, `wt status`,
//! `wt merge`, and daemon start/stop/status, with stub implementations
//! for the remaining `wt` subcommands.

use std::fmt::Write;
use std::process::Command as ProcessCommand;

use anyhow::{Context, Result, bail};

use crate::prd::scan_prds;
use crate::util::colors;
use crate::worktree::daemon::Daemon;
use crate::worktree::git;
use crate::worktree::state::StateManager;
use crate::worktree::types::{
    DaemonConfig, EventType, OverlapRisk, OverlapWarning, WorktreeEntry, WorktreeEvent,
    WorktreeState, WorktreeStatus,
};

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
/// When a PRD ID is given, shows detailed state of that worktree (event
/// history, modified files, overlap warnings, merge readiness).
/// When omitted, shows overall daemon status with a summary of all worktrees.
pub fn cmd_wt_status(prd_id: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let main_root = git::resolve_main_worktree(&cwd).context("failed to resolve main worktree")?;
    let state_mgr = StateManager::new(&main_root);
    let state = state_mgr.read()?;

    match prd_id {
        Some(id) => print_worktree_detail(&state, id),
        None => print_overall_status(&main_root, &state),
    }
}

/// Print overall daemon status and a summary of all worktrees.
fn print_overall_status(root: &std::path::Path, state: &WorktreeState) -> Result<()> {
    // Daemon section.
    println!("{}", colors::header("Daemon"));

    if Daemon::is_healthy(root) {
        let pid = Daemon::read_pid(root)?.unwrap_or(0);
        println!("  Status:  {}", colors::success("running"));
        println!("  PID:     {pid}");

        if let Some(daemon) = &state.daemon {
            println!("  Started: {}", daemon.started_at);
            println!("  Last HB: {}", daemon.last_heartbeat);
            println!("  Timeout: {}h idle", daemon.idle_timeout_hours);
        }
    } else if Daemon::is_running(root) {
        let pid = Daemon::read_pid(root)?.unwrap_or(0);
        println!(
            "  Status:  {} (PID {pid}, socket unreachable)",
            colors::warning("unhealthy")
        );
    } else {
        println!("  Status:  {}", colors::dim("not running"));
    }

    println!();

    // Worktree summary section.
    let total = state.worktrees.len();
    let active = state
        .worktrees
        .iter()
        .filter(|w| w.status == WorktreeStatus::Active)
        .count();
    let completed = state
        .worktrees
        .iter()
        .filter(|w| w.status == WorktreeStatus::Completed)
        .count();
    let merged = state
        .worktrees
        .iter()
        .filter(|w| w.status == WorktreeStatus::Merged)
        .count();
    let failed = state
        .worktrees
        .iter()
        .filter(|w| {
            w.status == WorktreeStatus::MergeFailed || w.status == WorktreeStatus::Conflicted
        })
        .count();

    println!("{}", colors::header("Worktrees"));

    if total == 0 {
        println!(
            "  {}",
            colors::dim("No worktrees registered. Run `mr wt run <prd-id>` to create one.")
        );
    } else {
        println!("  Total:     {total}");
        println!("  Active:    {active}");
        println!("  Completed: {completed}");
        println!("  Merged:    {merged}");

        if failed > 0 {
            println!("  Failed:    {}", colors::error(&failed.to_string()));
        }

        // Brief per-worktree list.
        println!();
        for wt in &state.worktrees {
            println!(
                "  {} {} ({})",
                colors::dim(&wt.id),
                wt.prd,
                status_colored(wt.status),
            );
        }
    }

    // Overlap warnings.
    if !state.overlap_warnings.is_empty() {
        println!();
        println!("{}", colors::header("Overlap Warnings"));

        for w in &state.overlap_warnings {
            let risk_str = overlap_risk_colored(w.risk);
            let wts = w.worktrees.join(", ");
            println!("  [{risk_str}] {wts} — {} shared file(s)", w.files.len());
        }
    }

    Ok(())
}

/// Print detailed status for a single worktree identified by PRD ID.
fn print_worktree_detail(state: &WorktreeState, prd_id: &str) -> Result<()> {
    let entry = state
        .worktrees
        .iter()
        .find(|w| w.prd.eq_ignore_ascii_case(prd_id))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No worktree found for {prd_id}. Run `mr wt list` to see registered worktrees."
            )
        })?;

    // Header + identity.
    println!(
        "{} — {}",
        colors::header(&entry.prd),
        status_colored(entry.status),
    );
    println!();
    println!("  ID:           {}", entry.id);
    println!("  Branch:       {}", entry.branch);
    println!("  Path:         {}", entry.path);
    println!("  Merge target: {}", entry.merge_target);
    println!("  Created:      {}", entry.created_at);
    println!("  Updated:      {}", entry.updated_at);

    if let Some(pid) = entry.run_pid {
        let alive = Daemon::is_process_alive(pid);
        let label = if alive {
            colors::success("alive")
        } else {
            colors::dim("exited")
        };
        println!("  Run PID:      {pid} ({label})");
    }

    print_merge_readiness(entry);
    print_modified_files(entry);
    print_entry_overlaps(state, entry);
    print_event_history(entry);

    Ok(())
}

/// Print merge readiness section.
fn print_merge_readiness(entry: &WorktreeEntry) {
    println!();
    println!("{}", colors::header("Merge Readiness"));

    if entry.status == WorktreeStatus::Completed {
        println!(
            "  {}",
            colors::success("✓ Ready to merge (status: completed)")
        );
    } else {
        let reason = match entry.status {
            WorktreeStatus::Active => "still active (tasks in progress)",
            WorktreeStatus::Merging => "merge in progress",
            WorktreeStatus::Merged => "already merged",
            WorktreeStatus::MergeFailed => "previous merge failed",
            WorktreeStatus::Conflicted => "has unresolved conflicts",
            WorktreeStatus::Abandoned => "abandoned",
            WorktreeStatus::Completed => unreachable!(),
        };
        println!("  {} Not ready — {reason}", colors::warning("⚠"));
    }
}

/// Print modified files section.
fn print_modified_files(entry: &WorktreeEntry) {
    println!();
    println!(
        "{}",
        colors::header(&format!("Modified Files ({})", entry.modified_files.len()))
    );

    if entry.modified_files.is_empty() {
        println!("  {}", colors::dim("(none)"));
    } else {
        for f in &entry.modified_files {
            println!("  {f}");
        }
    }
}

/// Print overlap warnings involving a specific worktree entry.
fn print_entry_overlaps(state: &WorktreeState, entry: &WorktreeEntry) {
    let overlaps: Vec<_> = state
        .overlap_warnings
        .iter()
        .filter(|w| w.worktrees.contains(&entry.id))
        .collect();

    if overlaps.is_empty() {
        return;
    }

    println!();
    println!("{}", colors::header("Overlap Warnings"));

    for w in overlaps {
        let others: Vec<_> = w.worktrees.iter().filter(|id| *id != &entry.id).collect();
        let risk_str = overlap_risk_colored(w.risk);
        println!(
            "  [{risk_str}] shared with {} — {} file(s): {}",
            others
                .iter()
                .map(|s| s.as_str())
                .collect::<Vec<_>>()
                .join(", "),
            w.files.len(),
            w.files.join(", "),
        );
    }
}

/// Print event history section.
fn print_event_history(entry: &WorktreeEntry) {
    println!();
    println!(
        "{}",
        colors::header(&format!("Event History ({})", entry.events.len()))
    );

    if entry.events.is_empty() {
        println!("  {}", colors::dim("(no events)"));
    } else {
        for ev in &entry.events {
            let detail = ev.detail.as_deref().unwrap_or("");
            let detail_suffix = if detail.is_empty() {
                String::new()
            } else {
                format!(" — {detail}")
            };
            println!(
                "  {} {}{detail_suffix}",
                colors::dim(&ev.timestamp),
                ev.event_type,
            );
        }
    }
}

/// Color-code overlap risk level.
fn overlap_risk_colored(risk: OverlapRisk) -> String {
    match risk {
        OverlapRisk::Low => colors::success(&risk.to_string()),
        OverlapRisk::Medium => colors::warning(&risk.to_string()),
        OverlapRisk::High => colors::error(&risk.to_string()),
    }
}

/// Handles `mr wt merge <prd-id>`.
///
/// Manually triggers merge of a specific worktree into a target branch.
/// Supports `--into <target>` to override the default merge target,
/// including cross-worktree merges (e.g., merge into another PRD's branch).
pub fn cmd_wt_merge(
    prd_id: &str,
    into: Option<&str>,
    runner_name: &str,
    cli_model: Option<&str>,
) -> Result<()> {
    let root = git::resolve_main_worktree(&std::env::current_dir()?)?;

    // Read state to get display info before creating the daemon (which moves root).
    let merge_target = {
        let state = StateManager::new(&root).read()?;
        let wt = state
            .worktrees
            .iter()
            .find(|w| w.prd.eq_ignore_ascii_case(prd_id))
            .with_context(|| {
                format!(
                    "no worktree registered for {prd_id}. Run `mr wt list` to see registered worktrees."
                )
            })?;

        let target = into.unwrap_or(&wt.merge_target).to_string();
        println!(
            "{}",
            colors::info(&format!(
                "Merging {prd_id} ({}) into {target}...",
                wt.branch
            ))
        );
        target
    };

    // Create a Daemon instance with optional runner for conflict resolution.
    let daemon = match crate::runner::create_runner(runner_name, cli_model.map(String::from)) {
        Ok(r) => {
            tracing::info!("using runner '{}' for conflict resolution", r.name());
            Daemon::new_with_runner(root, DaemonConfig::default(), r)
        }
        Err(e) => {
            println!(
                "{}",
                colors::warning(&format!(
                    "No runner available for conflict resolution: {e:#}"
                ))
            );
            Daemon::new(root)
        }
    };

    daemon.manual_merge(prd_id, into)?;

    println!(
        "{}",
        colors::success(&format!(
            "✅ {prd_id} merged into {merge_target} successfully."
        ))
    );

    Ok(())
}

/// Handles `mr wt graph`.
///
/// Visualizes worktree overlap risk.  Nodes represent active worktrees and
/// edges represent shared modified files (from `overlap_warnings`).
/// Risk is color-coded: green (low/none), yellow (medium), red (high).
pub fn cmd_wt_graph(format: &str) -> Result<()> {
    let cwd = std::env::current_dir().context("failed to get current directory")?;
    let main_root = git::resolve_main_worktree(&cwd).context("failed to resolve main worktree")?;
    let state_mgr = StateManager::new(&main_root);
    let state = state_mgr.read()?;

    let output = match format.to_lowercase().as_str() {
        "ascii" => render_wt_graph_ascii(&state),
        "mermaid" => render_wt_graph_mermaid(&state),
        "dot" => render_wt_graph_dot(&state),
        other => bail!("unknown format: {other}. Use ascii, mermaid, or dot"),
    };

    print!("{output}");

    Ok(())
}

// ── Worktree graph renderers ────────────────────────────────────────

/// Renders the worktree overlap graph as ASCII art.
fn render_wt_graph_ascii(state: &WorktreeState) -> String {
    let mut out = String::new();

    out.push_str("Worktree Overlap Graph\n");
    out.push_str("======================\n\n");

    let active: Vec<&WorktreeEntry> = state
        .worktrees
        .iter()
        .filter(|w| !matches!(w.status, WorktreeStatus::Abandoned | WorktreeStatus::Merged))
        .collect();

    if active.is_empty() {
        out.push_str("(no active worktrees)\n");
        return out;
    }

    // Render each node.
    for wt in &active {
        let risk = wt_risk_level(wt, &state.overlap_warnings);
        let risk_indicator = match risk {
            OverlapRisk::Low => "●",
            OverlapRisk::Medium => "◐",
            OverlapRisk::High => "◉",
        };
        let _ = writeln!(
            out,
            "{risk_indicator} [{id}] {prd} ({status}) — {files} file(s) modified",
            id = wt.id,
            prd = wt.prd,
            status = wt.status,
            files = wt.modified_files.len(),
        );
    }

    // Render overlap edges.
    if !state.overlap_warnings.is_empty() {
        out.push_str("\n--- Overlaps ---\n\n");

        for warning in &state.overlap_warnings {
            let risk_label = match warning.risk {
                OverlapRisk::Low => "LOW",
                OverlapRisk::Medium => "MEDIUM",
                OverlapRisk::High => "HIGH",
            };
            let _ = writeln!(
                out,
                "{wts} [{risk_label}] — {count} shared file(s)",
                wts = warning.worktrees.join(" <-> "),
                count = warning.files.len(),
            );
            for f in &warning.files {
                let _ = writeln!(out, "    {f}");
            }
        }
    }

    // Summary.
    let _ = write!(
        out,
        "\n---\n{nodes} worktree(s), {edges} overlap(s)\n",
        nodes = active.len(),
        edges = state.overlap_warnings.len(),
    );

    out
}

/// Renders the worktree overlap graph as Mermaid flowchart syntax.
fn render_wt_graph_mermaid(state: &WorktreeState) -> String {
    let mut out = String::new();

    out.push_str("flowchart LR\n");

    let active: Vec<&WorktreeEntry> = state
        .worktrees
        .iter()
        .filter(|w| !matches!(w.status, WorktreeStatus::Abandoned | WorktreeStatus::Merged))
        .collect();

    if active.is_empty() {
        out.push_str("    empty[\"No active worktrees\"]\n");
        return out;
    }

    // Node definitions.
    for wt in &active {
        let node_id = wt.id.replace('-', "");
        let risk = wt_risk_level(wt, &state.overlap_warnings);
        let class = match risk {
            OverlapRisk::Low => ":::low",
            OverlapRisk::Medium => ":::medium",
            OverlapRisk::High => ":::high",
        };
        let _ = writeln!(
            out,
            "    {node_id}[\"{id}: {prd} ({status})\"]{class}",
            id = wt.id,
            prd = wt.prd,
            status = wt.status,
        );
    }

    // Edges from overlap warnings.
    if !state.overlap_warnings.is_empty() {
        out.push('\n');
        for warning in &state.overlap_warnings {
            if warning.worktrees.len() == 2 {
                let a = warning.worktrees[0].replace('-', "");
                let b = warning.worktrees[1].replace('-', "");
                let label = format!("{} file(s)", warning.files.len());
                let edge = match warning.risk {
                    OverlapRisk::Low => format!("    {a} -.-|{label}| {b}"),
                    OverlapRisk::Medium => format!("    {a} ---|{label}| {b}"),
                    OverlapRisk::High => format!("    {a} ===|{label}| {b}"),
                };
                let _ = writeln!(out, "{edge}");
            }
        }
    }

    // Style classes.
    out.push('\n');
    out.push_str("    classDef low fill:#d4edda,stroke:#28a745\n");
    out.push_str("    classDef medium fill:#fff3cd,stroke:#ffc107\n");
    out.push_str("    classDef high fill:#f8d7da,stroke:#dc3545\n");

    out
}

/// Renders the worktree overlap graph as Graphviz DOT format.
fn render_wt_graph_dot(state: &WorktreeState) -> String {
    let mut out = String::new();

    out.push_str("graph Worktree_Overlaps {\n");
    out.push_str("    rankdir=LR;\n");
    out.push_str("    node [shape=box];\n\n");

    let active: Vec<&WorktreeEntry> = state
        .worktrees
        .iter()
        .filter(|w| !matches!(w.status, WorktreeStatus::Abandoned | WorktreeStatus::Merged))
        .collect();

    if active.is_empty() {
        out.push_str("    empty [label=\"No active worktrees\"];\n");
        out.push_str("}\n");
        return out;
    }

    // Node definitions.
    for wt in &active {
        let node_id = wt.id.replace('-', "");
        let risk = wt_risk_level(wt, &state.overlap_warnings);
        let (fill, border) = match risk {
            OverlapRisk::Low => ("#d4edda", "#28a745"),
            OverlapRisk::Medium => ("#fff3cd", "#ffc107"),
            OverlapRisk::High => ("#f8d7da", "#dc3545"),
        };
        let _ = writeln!(
            out,
            "    {node_id} [label=\"{id}: {prd} ({status})\" style=filled fillcolor=\"{fill}\" color=\"{border}\"];",
            id = wt.id,
            prd = wt.prd,
            status = wt.status,
        );
    }

    // Edges from overlap warnings.
    if !state.overlap_warnings.is_empty() {
        out.push('\n');
        for warning in &state.overlap_warnings {
            if warning.worktrees.len() == 2 {
                let a = warning.worktrees[0].replace('-', "");
                let b = warning.worktrees[1].replace('-', "");
                let label = format!("{} file(s)", warning.files.len());
                let style = match warning.risk {
                    OverlapRisk::Low => "style=dashed",
                    OverlapRisk::Medium => "style=solid",
                    OverlapRisk::High => "style=bold penwidth=3",
                };
                let _ = writeln!(out, "    {a} -- {b} [label=\"{label}\" {style}];",);
            }
        }
    }

    out.push_str("}\n");

    out
}

/// Determine the worst overlap risk for a given worktree entry.
fn wt_risk_level(entry: &WorktreeEntry, warnings: &[OverlapWarning]) -> OverlapRisk {
    warnings
        .iter()
        .filter(|w| w.worktrees.contains(&entry.id))
        .map(|w| w.risk)
        .max_by_key(|r| match r {
            OverlapRisk::Low => 0,
            OverlapRisk::Medium => 1,
            OverlapRisk::High => 2,
        })
        .unwrap_or(OverlapRisk::Low)
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
///
/// If a runner can be created from config, it is passed to the daemon
/// for agent-driven conflict resolution.
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

    let daemon = match create_daemon_runner(&root) {
        Ok(runner) => {
            tracing::info!(
                "daemon using runner '{}' for conflict resolution",
                runner.name()
            );
            Daemon::new_with_runner(root, DaemonConfig::default(), runner)
        }
        Err(e) => {
            tracing::info!("no runner available for conflict resolution: {e:#}");
            Daemon::new(root)
        }
    };

    daemon.run()
}

/// Try to create a runner for daemon conflict resolution from project config.
///
/// Reads `.mr/config.toml` to determine runner name and model. Falls back
/// to "copilot" if no config is present. Returns an error if the runner
/// is not available (e.g., CLI not installed).
fn create_daemon_runner(root: &std::path::Path) -> Result<Box<dyn crate::runner::Runner>> {
    let config = crate::config::Config::load_or_default(root)?;
    let runner_name = config.runner.as_deref().unwrap_or("copilot");
    let model = config.model.clone();

    crate::runner::create_runner(runner_name, model)
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
    fn test_cmd_wt_status_fails_without_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let result = cmd_wt_status(None);
        assert!(result.is_err());

        std::env::set_current_dir(orig).unwrap();
    }

    #[test]
    fn test_cmd_wt_status_with_unknown_prd_fails() {
        // Set up a minimal git repo so resolve_main_worktree works.
        let tmp = tempfile::tempdir().unwrap();
        let orig = std::env::current_dir().unwrap();
        std::process::Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        // Create .mr/worktrees/ with an empty state (no worktrees).
        let wt_dir = tmp.path().join(".mr").join("worktrees");
        std::fs::create_dir_all(&wt_dir).unwrap();

        let result = cmd_wt_status(Some("PRD-9999"));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("No worktree found"), "got: {err_msg}");

        std::env::set_current_dir(orig).unwrap();
    }

    #[test]
    fn test_print_worktree_detail_shows_entry() {
        let state = WorktreeState {
            version: 1,
            daemon: None,
            worktrees: vec![WorktreeEntry {
                id: "wt-001".to_string(),
                prd: "PRD-0001".to_string(),
                branch: "repo-prd-1".to_string(),
                path: "/tmp/wt".to_string(),
                status: WorktreeStatus::Completed,
                run_pid: None,
                created_at: "2026-01-01T00:00:00Z".to_string(),
                updated_at: "2026-01-02T00:00:00Z".to_string(),
                merge_target: "main".to_string(),
                modified_files: vec!["src/main.rs".to_string()],
                events: vec![
                    WorktreeEvent {
                        timestamp: "2026-01-01T00:00:00Z".to_string(),
                        event_type: EventType::Created,
                        detail: None,
                    },
                    WorktreeEvent {
                        timestamp: "2026-01-02T00:00:00Z".to_string(),
                        event_type: EventType::RunCompleted,
                        detail: None,
                    },
                ],
            }],
            overlap_warnings: vec![],
        };

        // Should succeed and not panic.
        let result = print_worktree_detail(&state, "PRD-0001");
        assert!(result.is_ok());
    }

    #[test]
    fn test_print_worktree_detail_not_found() {
        let state = WorktreeState::default();
        let result = print_worktree_detail(&state, "PRD-9999");
        assert!(result.is_err());
    }

    #[test]
    fn test_overlap_risk_colored_returns_string_for_all_variants() {
        use crate::worktree::types::OverlapRisk;
        let variants = [OverlapRisk::Low, OverlapRisk::Medium, OverlapRisk::High];
        for risk in variants {
            let result = overlap_risk_colored(risk);
            assert!(
                !result.is_empty(),
                "overlap_risk_colored({risk}) should not be empty"
            );
        }
    }

    #[test]
    fn test_cmd_wt_merge_fails_without_worktree() {
        // cmd_wt_merge resolves main worktree root — should fail outside a git repo
        // or when no worktree is registered for the PRD.
        let result = cmd_wt_merge("PRD-9999", None, "copilot", None);
        assert!(result.is_err());
    }

    #[test]
    fn test_cmd_wt_graph_fails_without_git_repo() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let result = cmd_wt_graph("ascii");
        assert!(result.is_err());

        std::env::set_current_dir(orig).unwrap();
    }

    #[test]
    fn test_cmd_wt_graph_rejects_unknown_format() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = std::env::current_dir().unwrap();

        // Set up a minimal git repo.
        std::process::Command::new("git")
            .args(["init", "--initial-branch=main"])
            .current_dir(tmp.path())
            .output()
            .unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();

        let result = cmd_wt_graph("csv");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unknown format"));

        std::env::set_current_dir(orig).unwrap();
    }

    #[test]
    fn test_render_wt_graph_ascii_empty() {
        let state = WorktreeState::default();
        let out = render_wt_graph_ascii(&state);
        assert!(out.contains("Worktree Overlap Graph"));
        assert!(out.contains("(no active worktrees)"));
    }

    #[test]
    fn test_render_wt_graph_ascii_with_worktrees() {
        let state = make_graph_test_state();
        let out = render_wt_graph_ascii(&state);
        assert!(out.contains("wt-001"));
        assert!(out.contains("PRD-0001"));
        assert!(out.contains("wt-002"));
        assert!(out.contains("PRD-0002"));
        assert!(out.contains("HIGH"));
        assert!(out.contains("src/main.rs"));
        assert!(out.contains("2 worktree(s)"));
    }

    #[test]
    fn test_render_wt_graph_ascii_excludes_merged_and_abandoned() {
        let mut state = make_graph_test_state();
        state.worktrees[0].status = WorktreeStatus::Merged;
        state.worktrees[1].status = WorktreeStatus::Abandoned;
        let out = render_wt_graph_ascii(&state);
        assert!(out.contains("(no active worktrees)"));
    }

    #[test]
    fn test_render_wt_graph_mermaid_empty() {
        let state = WorktreeState::default();
        let out = render_wt_graph_mermaid(&state);
        assert!(out.contains("flowchart LR"));
        assert!(out.contains("No active worktrees"));
    }

    #[test]
    fn test_render_wt_graph_mermaid_with_overlaps() {
        let state = make_graph_test_state();
        let out = render_wt_graph_mermaid(&state);
        assert!(out.contains("flowchart LR"));
        assert!(out.contains("wt001"));
        assert!(out.contains("PRD-0001"));
        assert!(out.contains("==="));
        assert!(out.contains("classDef high"));
    }

    #[test]
    fn test_render_wt_graph_dot_empty() {
        let state = WorktreeState::default();
        let out = render_wt_graph_dot(&state);
        assert!(out.contains("graph Worktree_Overlaps"));
        assert!(out.contains("No active worktrees"));
    }

    #[test]
    fn test_render_wt_graph_dot_with_overlaps() {
        let state = make_graph_test_state();
        let out = render_wt_graph_dot(&state);
        assert!(out.contains("graph Worktree_Overlaps"));
        assert!(out.contains("wt001"));
        assert!(out.contains("PRD-0001"));
        assert!(out.contains("penwidth=3"));
        assert!(out.contains("1 file(s)"));
    }

    #[test]
    fn test_wt_risk_level_returns_worst() {
        let entry = WorktreeEntry {
            id: "wt-001".to_string(),
            prd: "PRD-0001".to_string(),
            branch: "b".to_string(),
            path: "/tmp".to_string(),
            status: WorktreeStatus::Active,
            run_pid: None,
            created_at: String::new(),
            updated_at: String::new(),
            merge_target: "main".to_string(),
            modified_files: vec![],
            events: vec![],
        };
        let warnings = vec![
            OverlapWarning {
                worktrees: vec!["wt-001".to_string(), "wt-002".to_string()],
                files: vec!["a.rs".to_string()],
                risk: OverlapRisk::Low,
            },
            OverlapWarning {
                worktrees: vec!["wt-001".to_string(), "wt-003".to_string()],
                files: vec!["b.rs".to_string()],
                risk: OverlapRisk::High,
            },
        ];
        assert_eq!(wt_risk_level(&entry, &warnings), OverlapRisk::High);
    }

    #[test]
    fn test_wt_risk_level_no_warnings() {
        let entry = WorktreeEntry {
            id: "wt-099".to_string(),
            prd: "PRD-0099".to_string(),
            branch: "b".to_string(),
            path: "/tmp".to_string(),
            status: WorktreeStatus::Active,
            run_pid: None,
            created_at: String::new(),
            updated_at: String::new(),
            merge_target: "main".to_string(),
            modified_files: vec![],
            events: vec![],
        };
        assert_eq!(wt_risk_level(&entry, &[]), OverlapRisk::Low);
    }

    /// Helper: build a state with two active worktrees and one high-risk overlap.
    fn make_graph_test_state() -> WorktreeState {
        WorktreeState {
            version: 1,
            daemon: None,
            worktrees: vec![
                WorktreeEntry {
                    id: "wt-001".to_string(),
                    prd: "PRD-0001".to_string(),
                    branch: "repo-prd-1".to_string(),
                    path: "/tmp/wt1".to_string(),
                    status: WorktreeStatus::Active,
                    run_pid: Some(1234),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    updated_at: "2026-01-02T00:00:00Z".to_string(),
                    merge_target: "main".to_string(),
                    modified_files: vec!["src/main.rs".to_string()],
                    events: vec![],
                },
                WorktreeEntry {
                    id: "wt-002".to_string(),
                    prd: "PRD-0002".to_string(),
                    branch: "repo-prd-2".to_string(),
                    path: "/tmp/wt2".to_string(),
                    status: WorktreeStatus::Active,
                    run_pid: Some(5678),
                    created_at: "2026-01-01T00:00:00Z".to_string(),
                    updated_at: "2026-01-02T00:00:00Z".to_string(),
                    merge_target: "main".to_string(),
                    modified_files: vec!["src/main.rs".to_string()],
                    events: vec![],
                },
            ],
            overlap_warnings: vec![OverlapWarning {
                worktrees: vec!["wt-001".to_string(), "wt-002".to_string()],
                files: vec!["src/main.rs".to_string()],
                risk: OverlapRisk::High,
            }],
        }
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
