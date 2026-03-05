# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### 📋 PRD Tasks

- Prd(PRD-0039)feat(T-001): define worktree state schema and types

Add src/worktree/types.rs with WorktreeState, WorktreeEntry,
WorktreeEvent, OverlapWarning, DaemonConfig, IPC message types.
All types are YAML/JSON-serializable with serde. Includes version
field for future schema migration. 9 unit tests covering roundtrip
serialization, display impls, and defaults.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-002): implement state file read/write with advisory locking

Add src/worktree/state.rs with StateManager providing flock-based
advisory locking, atomic writes via temp+rename, and read-modify-write
cycle for .mr/worktrees/state.yaml. 11 unit tests included.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-003): Implement worktree path resolution and git helpers

Add src/worktree/git.rs with functions for main worktree resolution,
branch/worktree create/remove, modified file detection, linked worktree
detection, and sibling directory path convention. 17 unit tests.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-018): wire wt subcommand with CLI skeleton

Add 'mr wt' command group with subcommands: run, list, status, merge,
graph, remove, and daemon (start/stop/status). All handlers are stubs
returning not-implemented errors, to be filled in by subsequent tasks.

- Create src/commands/worktree.rs with 9 stub handlers and 10 tests
- Add WtCommand and DaemonCommand enums to main.rs
- Wire full dispatch with normalize_prd_id for PRD ID arguments

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-004): implement IPC protocol over Unix domain socket

Add src/worktree/ipc.rs with newline-delimited JSON protocol for
daemon-worktree communication. Includes IpcClient (connect + send),
IpcServer (bind + accept + dispatch), socket path resolution, daemon
reachability check, stale socket cleanup, and non-blocking mode support.
10 unit tests covering full client-server roundtrip, all message types,
lifecycle, and error paths.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-005): implement daemon core with heartbeat loop

- Create src/worktree/daemon.rs with full daemon implementation:
  - PID file management, process liveness checks, SIGTERM stop
  - Single-threaded event loop with non-blocking IPC accept
  - Tier 1 heartbeat: polls worktree PIDs, updates modified files, computes overlap
  - Auto-exit on configurable idle timeout (default 3h)
  - Signal handling (SIGTERM/SIGINT) + per-instance shutdown handle for testing
  - IPC message processing for all lifecycle events
- Add try_accept_stream() and handle_stream() to src/worktree/ipc.rs
- Implement cmd_wt_daemon_start/stop/status in src/commands/worktree.rs
- 21 new tests (662 total), all passing

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-006): implement mr wt run subcommand

Implement the `mr wt run <prd-id>` command that:
- Resolves main worktree and validates PRD exists
- Creates branch and sibling git worktree
- Registers worktree in state.yaml with advisory locking
- Auto-starts daemon as detached process if not running
- Spawns `mr run` as detached process in worktree context
- Guards against duplicate active worktrees for same PRD
- Records lifecycle events (created, run_started) in state

Added helpers: next_wt_id, now_iso, days_to_ymd, validate_prd_exists,
ensure_daemon, register_worktree, spawn_mr_run.

6 new unit tests (668 total, all passing).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-007): implement daemon auto-start logic

Add Daemon::is_healthy(), cleanup_stale(), and ensure_running() to
src/worktree/daemon.rs for robust daemon auto-start with PID + socket
liveness checks, stale state cleanup, and readiness wait. Simplify
ensure_daemon() in commands/worktree.rs to delegate to the new API.

7 new tests (675 total, all passing).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-008): integrate mr run with daemon IPC for worktree detection

Add DaemonNotifier to src/commands/run.rs that sends lifecycle events
(run_started, task_started, task_completed, run_completed, run_failed)
to the worktree daemon when mr run executes inside a linked worktree.

Backward compatible: no daemon means no IPC, run works identically.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-009): implement mr wt list subcommand

Add cmd_wt_list implementation that reads state.yaml and displays all
registered worktrees in an aligned table with PRD ID, branch, status,
modified files count, and last event timestamp. Status is color-coded
by lifecycle phase. Includes 3 new unit tests (683 total, all passing).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-010): implement mr wt status subcommand

Add detailed worktree status display with two modes:
- Overall: daemon health, worktree summary counts, overlap warnings
- Per-PRD: identity, merge readiness, modified files, event history

5 new unit tests, 686 total passing.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-011): implement auto-merge in daemon heartbeat

Add Tier 2 auto-merge logic to the daemon heartbeat loop. When completed
worktrees are detected, the daemon:
- Computes merge order (fewest overlapping files first, then by completion time)
- Attempts rebase onto target (falls back to merge on failure)
- Runs UATs in the worktree after integration
- Merges branch into target on success (fast-forward preferred)
- Marks conflicted/merge_failed on failure for T-012/agent resolution

New EventType variants: MergeStarted, MergeCompleted, MergeFailed, Conflicted
New git helpers: rebase_onto, rebase_abort, merge_branch, merge_abort, checkout, merge_ff_only
18 new tests (704 total, all passing)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-012): implement agent-driven conflict resolution

Add conflict resolution pipeline to worktree daemon. When merge/rebase
produces conflicts, the daemon now invokes an agent (via Runner trait)
to resolve them automatically before marking the worktree as conflicted.

Key changes:
- Add git helpers: list_conflict_files, conflict_diff, stage_all,
  rebase_continue, is_rebase_in_progress, merge_commit
- Add ConflictResolutionStarted/ConflictResolved event types
- Add optional Runner to Daemon for agent-driven resolution
- Modify attempt_merge_worktree to try resolution before fallback
- Extract create_runner to runner module for reuse (DRY)
- Add conflict_resolve.md prompt template
- Wire runner from config into daemon start
- 11 new tests (715 total, all passing)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-013): implement mr wt merge subcommand

Manual merge trigger with cross-worktree support:
- mr wt merge <prd-id> [--into <target>]
- Rebase-first strategy with merge fallback
- Agent-driven conflict resolution via runner
- UAT gating before merge completion
- Smart target detection for cross-worktree merges
- 5 new tests (720 total, all passing)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-014): implement agent-driven state commits

Add state commit infrastructure to the daemon that commits state.yaml
to the main worktree repository on significant events (merge completed,
merge failed, conflicted). Generates human-readable summary commit
messages in the format: mr-wt: PRD-0039 merged, PRD-0040 in progress
(3 active worktrees).

- Add git helpers: add_file(), commit(), has_staged_changes()
- Add EventType::StateCommitted variant
- Add build_state_summary() and commit_state() to Daemon
- Wire commit_state() into attempt_merge_worktree() and manual_merge()
- 9 new tests (720 → 729)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-015): implement mr wt graph subcommand

Add worktree overlap risk visualization in ASCII, Mermaid, and DOT formats.
Nodes represent active worktrees, edges represent shared modified files.
Risk is color-coded: green (low), yellow (medium), red (high).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-016): implement mr wt remove subcommand

Replaces the stub cmd_wt_remove with a full implementation that:
- Resolves main worktree and finds the target worktree in state
- Refuses removal of worktrees in Merging status (safety check)
- Removes git worktree (best-effort, handles missing dirs)
- Optionally deletes the branch with --delete-branch
- Updates state.yaml: marks Abandoned, clears run_pid, records event
- Cleans up overlap warnings referencing the removed worktree
- Adds EventType::Removed variant to types
- 4 new tests (742 total, all passing)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-017): implement daemon crash recovery

Add crash recovery logic that runs on daemon startup before the event
loop. Detects and corrects stale state from previous crashes:
- Orphaned worktrees (missing paths) → marked Abandoned
- Partial merges (stuck Merging status) → aborted and reset to Completed
- Dead run processes (Active with dead PID) → marked Completed

New helpers: detect_recovery_actions(), apply_recovery_actions(),
git::is_merge_in_progress(). New EventType::RecoveryPerformed variant.

7 new tests (749 total), all passing.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)feat(T-019): document worktree orchestration in AGENTS.md

Add comprehensive worktree orchestration section to AGENTS.md covering
architecture, CLI usage, daemon lifecycle, IPC protocol, state file schema,
merge strategy, and troubleshooting. Update workspace overview to include
worktree module and .mr/worktrees/ directory.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-001): verify wt run creates branch, worktree, and registers state

Add test wt_run_creates_branch_worktree_and_registers_state that exercises
the core cmd_wt_run flow: PRD validation, branch/worktree creation, and state
registration with correct fields and events.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-002): verify mr wt list shows worktrees with status

Added test_cmd_wt_list_shows_registered_worktrees_with_status and
test_cmd_wt_list_empty_state_shows_hint tests. Extracted make_test_repo
helper for worktree test setup.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-003): verified — wt status shows detailed worktree state

Existing tests cover the acceptance criterion:
- test_print_worktree_detail_shows_entry (happy path)
- test_cmd_wt_status_with_unknown_prd_fails (error path)
- test_print_worktree_detail_not_found (not-found path)

All 752 tests pass.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-004): verified daemon auto-start and unix socket listening

Existing tests cover this UAT:
- daemon_accepts_ipc_heartbeat: proves daemon binds and responds on unix socket
- is_healthy_true_when_running_and_reachable: validates health check used by auto-start

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-005): verify daemon heartbeat updates liveness and overlaps

Add tier1_heartbeat_updates_liveness_and_overlaps integration test that
starts a daemon with a 1-second heartbeat, pre-populates state with a
dead-PID worktree and overlapping active worktrees, then verifies the
daemon detects dead processes and computes overlap warnings.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-006): verified lifecycle events via IPC with existing tests

Verified by existing tests:
- daemon_notifier_connects_and_sends_events (full lifecycle flow)
- daemon_notifier_returns_none_* (4 graceful fallback tests)
- client_server_all_message_types (IPC protocol layer)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-007): verify auto-merge UAT gating with new test

Add attempt_merge_gates_on_uat_failure test that exercises the full
auto-merge flow: completed worktree → integration → UAT gate → MergeFailed.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-008): verify agent resolves merge conflicts

Add end-to-end test that verifies the daemon detects merge conflicts,
invokes the agent runner with conflict context, and records both
ConflictResolutionStarted and ConflictResolved events after successful
resolution.

Added set_execute_side_effect() to MockRunner to enable simulating
agent file edits during execute().

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-009): verified worktree graph overlap risk visualization

Existing tests in src/commands/worktree.rs cover all three graph formats
(ASCII, Mermaid, DOT) with overlap risk visualization, risk level
computation, empty state handling, and error paths (11 tests total).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-010): verify manual merge triggers merge pipeline

Add manual_merge_triggers_merge_pipeline_for_prd test that creates a real
git repo with a worktree, registers it as Completed, calls manual_merge,
and verifies MergeStarted event and UAT gating (MergeFailed status).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-011): verify daemon crash recovery with existing tests

Six tests in src/worktree/daemon.rs cover all crash recovery scenarios
including partial merge state detection (recover_stale_state_resets_partial_merge_no_operation).
Also fix pre-existing clippy warning in mock.rs (underscore-prefixed used binding).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-012): verify daemon auto-exits on idle timeout

Existing test daemon_exits_on_idle_timeout in src/worktree/daemon.rs
covers this acceptance criterion by setting idle_timeout_hours=0 and
verifying the daemon exits cleanly with PID and state cleanup.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-013): verify wt remove cleans up worktree, branch, and state

Existing tests in src/commands/worktree.rs cover the full remove flow:
- test_cmd_wt_remove_succeeds_for_active_worktree (happy path)
- test_cmd_wt_remove_rejects_merging_worktree (safety guard)
- test_cmd_wt_remove_unknown_prd_returns_error
- test_cmd_wt_remove_fails_without_git_repo

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-014): verify state commits on significant events with summaries

Existing tests cover commit_state() behavior:
- Only called after significant events (merged, merge failed, conflicted)
- build_state_summary() generates descriptive commit messages
- 6 tests pass: summary format, mixed states, no-op when unchanged

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0039)uat(uat-015): verified strategic merge order via existing tests

4 existing compute_merge_order tests cover overlap-aware ordering
and tie-breaking by completion time.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-001): workspace setup with mr-ui crate and feature gate

- Create crates/mr-ui/ with Cargo.toml (leptos 0.8, axum 0.8, thaw 0.5-beta, tracing)
- Add workspace config to root Cargo.toml with default-members
- Feature-gate mr-ui behind 'ui' feature in root crate
- Add cargo-leptos install task to Makefile.toml
- Update AGENTS.md with workspace structure docs

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-002): Leptos + Axum app scaffold with cargo-leptos

Set up the Leptos 0.8 app entrypoint with Axum router, SSR + hydration,
and cargo-leptos config. Created app.rs (root component, router, shell),
main.rs (server entrypoint), updated lib.rs (hydration), and added
leptos_meta, wasm-bindgen, console_error_panic_hook dependencies.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-003): add StateService for reading and watching state.yaml

Create server-side StateService in mr-ui crate that polls .mr/worktrees/state.yaml
and .mr/prds/ every 2 seconds, exposing combined worktree + PRD state via
Arc<RwLock<AppState>> shared across Axum handlers. Includes broadcast channel for
future WebSocket push, UI-specific types matching the YAML schemas, and 9 async
tests covering parsing, polling, and broadcast notification.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-004): WebSocket server function for real-time state push

Add Axum WebSocket handler at /ws/state that sends initial AppState
snapshot on connection and streams subsequent broadcast updates as JSON.
Client-side hook connects via web_sys::WebSocket after hydration and
updates a reactive RwSignal<Option<AppState>> provided as Leptos context.
HomePage now reactively displays daemon status, worktree count, and PRD count.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-005): redirect mr run output to log files

Modify worktree run flow to redirect stdout/stderr of the detached
mr run process to .mr/worktrees/<wt-id>/run.log. This enables the UI
to tail and stream logs for active worktrees.

- Add log_file_path() helper for computing log paths
- spawn_mr_run() creates log directory, opens log file, redirects output
- WorktreeEntry gains log_file: Option<String> field in state.yaml
- wt status detail view shows log file path
- CLI output updated to show actual log file location
- 757 tests pass (756 existing + 1 new)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-015): add mr ui CLI command with host, port, open flags

Add the 'mr ui' subcommand to the root CLI, feature-gated behind
#[cfg(feature = "ui")]. Flags: --host (default 127.0.0.1), --port
(default 3939), --open (auto-open browser).

Create crates/mr-ui/src/serve.rs with serve_blocking() that constructs
LeptosOptions via typed builder, starts StateService, and runs the
Axum/Leptos server. Cross-platform open_browser() helper for --open.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-006): app shell layout with sidebar, dark theme toggle

- Add components module: layout, sidebar, theme
- AppShell with Thaw Layout/LayoutHeader/LayoutSider
- NavDrawer sidebar with Dashboard, Worktrees, PRDs nav items
- Dark theme by default via Thaw ConfigProvider + Theme
- Light/dark toggle with Switch component
- Daemon status badge in top bar
- Responsive CSS with sidebar collapse at 768px
- Placeholder routes for /worktrees and /prds

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-007): Dashboard home page with overview cards and daemon health

Add DashboardHome component with:
- Worktree status breakdown cards (total/active/completed/merged/failed)
- Daemon health card (online/offline badge, PID, started_at, heartbeat)
- Overlap warnings card with risk-colored tags (high/medium/low)
- Recent events timeline (last 10 events across all worktrees)

Uses Thaw Card, Badge, Tag components. Custom CSS timeline since
Thaw 0.5.0-beta does not include a Timeline component.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-008): worktree list view with sortable table and status filter

Add WorktreeList component with real-time status table using Thaw Table,
Badge, Tag, and Select components. Includes status filter dropdown,
sortable column headers, color-coded status badges, current task
derivation from event history, and responsive CSS styling.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-016): add cargo-make tasks for UI dev, build, test, and CI integration

Add ui-dev (cargo-leptos watch), ui-build (production build), ui-test
(nextest with all features), ui-clippy, and ui-ci tasks to Makefile.toml.
Update ci task to include UI lint and test. Fix pre-existing clippy
warnings in mr-ui and add recursion_limit for --all-features compilation.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-009): worktree detail view with event timeline, task progress, files, and merge info

Add the worktree detail page at /worktrees/:id with five sections:
- Status header with PRD title, branch, status badge, and PID
- Event timeline (Temporal-style, reverse chronological)
- Task progress list with status indicators and progress bar
- Modified files list
- Merge info with target branch and resolution summary

Extended PrdSummary with TaskSummary for individual task display.
Made worktree list rows link to their detail page.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-010): add PRD list page with status, deps, and completion

Implement the PRD list page component for the UI dashboard with:
- Sortable table (ID, Title, Status, Dependencies, Tasks, Completion)
- Status filter dropdown (All/Draft/Active/Done/Parked)
- Color-coded status badges and dependency tags
- Completion progress bars with percentage display
- New PRD link placeholder for future T-011

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-017): add tracing integration for UI server

Add tower-http TraceLayer for HTTP request/response logging, WebSocket
connection events, and enhanced state service reload tracing with
structured fields (worktree/PRD counts, change flags).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-011): add PRD creation form with server-side mr new invocation

Add a PRD creation form at /prds/new with slug, context, runner, and model
fields. On submit, invokes mr new server-side via tokio::process::Command
through a Leptos #[server] function. Shows spinner during creation and
displays success/error results with navigation links.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-012): worktree kickoff from UI

Add worktree kickoff functionality to the dashboard UI:
- New wt_kickoff.rs component with server function, dialog, and action button
- Action button in PRD list table (Actions column) and worktree detail header
- Modal dialog with runner/model selection, spinner, and success/error display
- CSS for kickoff button, overlay dialog, form fields, and result messages

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-013): log streaming view with real-time WebSocket tail

Add dedicated log viewer page at /worktrees/:id/logs that streams
worktree run.log content via WebSocket. Server-side tails the log file
polling every 200ms, client renders in terminal-styled container with
auto-scroll, pause/resume, clear, and error line highlighting.

- Add log_ws_handler in ws.rs for /ws/logs/{id} WebSocket endpoint
- Create log_viewer.rs component with terminal display and controls
- Add View Logs link on worktree detail when log_file is present
- Add routes to app.rs, serve.rs, and main.rs (dev)
- Add terminal-styled CSS with monospace font stack and error colors

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-014): overlap risk visualization page

Add dedicated overlap risk visualization page at /overlap with:
- Summary cards showing total/high/medium/low warning counts
- Table of overlap warnings sorted by risk (high first)
- Risk-colored badges and row borders (red/yellow/blue)
- Worktree IDs as clickable links to detail pages
- Monospaced shared file path lists
- Sidebar nav item with warning icon
- Empty state when no warnings exist

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)feat(T-018): add UI documentation to AGENTS.md and README.md

Add comprehensive Control UI Dashboard section to AGENTS.md covering
architecture, state flow, dev workflow, crate structure, routes, and
feature gates. Update README.md commands table with mr ui entries and
add UI Dashboard subsection to Development section.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)uat(uat-001): skipped — Leptos SSR rendering requires WASM/browser context

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)uat(uat-002): verify dashboard renders worktree state from state.yaml

Add load_app_state_combines_worktrees_and_prds_for_dashboard test in
crates/mr-ui/src/state.rs. Validates the full data pipeline from a
realistic state.yaml (multi-status worktrees, daemon, overlap warnings,
events) and PRDs through to AppState — the exact data consumed by
dashboard components.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)uat(uat-003): verify WebSocket pushes real-time state updates

Add 3 integration tests in crates/mr-ui/src/ws.rs that spin up an Axum
server with the /ws/state WebSocket route, connect a tokio-tungstenite
client, and verify initial snapshot delivery, broadcast-triggered update
push, and ordered multi-update delivery.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)uat(uat-004): skipped — thin subprocess wrapper, core logic tested in main crate

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)uat(uat-005): skipped — requires full integration environment

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)uat(uat-006): verify log streaming with WebSocket tests

Added three tests for the log streaming WebSocket handler:
- log_ws_streams_existing_content_on_connect
- log_ws_streams_new_content_in_realtime
- log_ws_reports_missing_worktree

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)uat(uat-007): skipped — requires browser/WASM E2E testing not available in CI

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0040)uat(uat-008): verified UI compiles and passes clippy with pedantic lints

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>

## [0.9.0] - 2026-02-23

### 📋 PRD Tasks

- Prd(PRD-0037)finalize: Fix run loop to re-enter task execution after UAT-added tasks

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)feat(T-001): create .mr/skills/ directory and SKILLS.md during mr init

Add SKILLS_TEMPLATE constant and create .mr/skills/ directory with
SKILLS.md manifest file during mr init. The skills directory provides
persistent storage for agent-learned techniques across runs.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)feat(T-002): handle skills directory in mr restore

Extract init_skills() helper from init() for DRY reuse. Call it from
restore_impl() to create .mr/skills/ and SKILLS.md if missing while
preserving existing learned skills. Add tests for both preservation
and recreation scenarios.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)feat(T-003): load skills manifest and inject into build_prompt()

Read .mr/skills/SKILLS.md in build_prompt() and insert as skills_manifest
placeholder into PlaceholderContext. Compares against default SKILLS_TEMPLATE
to avoid injecting boilerplate when no real skills exist. Missing/empty/default
files result in no placeholder, so {{#if skills_manifest}} evaluates to false.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)feat(T-004): add skills manifest and saving instructions to run_task.md

Add two new sections to the run_task.md prompt:
- Skills manifest context section ({{#if skills_manifest}}) after Constitution,
  showing available skills with a note to read full details on demand
- Saving Skills (End-of-Task) section with instructions for creating skill
  directories, writing skill.md files, updating SKILLS.md, and guidance
  to bias toward saving only genuinely reusable techniques

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)feat(T-005): sync PROMPT_RUN_TASK constant with run_task.md skills sections

Add skills_manifest conditional block and Saving Skills end-of-task
section to the PROMPT_RUN_TASK constant in init.rs, matching the
changes made to .mr/prompts/run_task.md in T-004. Resolves the
transient Constitution Rule 7 violation.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)feat(T-006): add unit tests for skills init and build_prompt expansion

Add 6 unit tests covering the skills system:
- init_skills(): creation, preservation, idempotency (3 tests in init.rs)
- build_prompt(): skills_manifest injection, default omission, missing omission (3 tests in run.rs)

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)uat(uat-001): verify mr init creates skills dir and SKILLS.md

Existing tests test_init_creates_structure and test_init_skills_creates_dir_and_manifest
already cover this acceptance criterion. All 583 tests pass.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)uat(uat-002): verify restore creates/preserves skills

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)uat(uat-003): verified build_prompt skills_manifest injection

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)uat(uat-004): add test verifying run_task prompt includes skills sections

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)uat(uat-005): verify full CI passes

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0038)finalize: persistent skills system with manifest-pattern prompt injection

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>

## [0.8.7] - 2026-02-23

## [0.8.6] - 2026-02-23

### 📋 PRD Tasks

- Prd(PRD-0037)feat(T-001): Add has_incomplete_tasks() public method to Prd

Add has_incomplete_tasks() -> bool method to Prd struct that delegates
to next_task().is_some(). This enables production code (T-002/T-003)
to check for incomplete tasks after UAT verification.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0037)feat(T-002): UAT loop detects new incomplete tasks and breaks early

Add has_new_tasks field to UatVerificationLoopResult. After each UAT
verification iteration, reload the PRD and check has_incomplete_tasks().
If new tasks exist, break out of the UAT loop early so the outer loop
can re-enter task execution.

Also add require_prd_by_id() helper to reduce repeated error-handling
boilerplate (DRY), and display a warning in print_uat_result() when
new tasks are detected.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0037)feat(T-003): outer loop re-enters task execution after UAT adds new tasks

After UAT verification loop completes, check has_new_tasks flag on
the result. If true, continue the outer loop to execute newly added
tasks instead of breaking. A safety counter (max 10 cycles) prevents
infinite task→UAT cycling.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0037)feat(T-004): add unit tests for UAT loop early-break and re-entry

Add 8 tests covering:
- UAT loop breaks early when new incomplete tasks detected (SideEffectRunner)
- UAT loop reports has_new_tasks=false when no tasks added
- UAT loop convergence (terminates with 0 iterations when all verified)
- UatVerificationLoopResult has_new_tasks field
- has_incomplete_tasks() for all_done, todo, in_progress, and no_tasks cases
- Remove stale #[allow(dead_code)] on has_incomplete_tasks()

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0037)uat(uat-001): verified run loop re-entry on new tasks

Existing test test_uat_loop_breaks_early_when_new_tasks_detected covers
this acceptance criterion by simulating agent task addition during UAT
verification and asserting has_new_tasks=true with early loop break.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0037)uat(uat-002): verified normal termination when no new tasks added

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0037)uat(uat-003): verify UAT loop breaks early on new tasks

Verified by existing test test_uat_loop_breaks_early_when_new_tasks_detected
which uses SideEffectRunner to simulate task addition during UAT and asserts
has_new_tasks=true with early break after 1 iteration. 575/575 tests pass.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0037)uat(uat-004): verify run loop convergence via existing tests

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>

## [0.8.5] - 2026-02-21

### 📋 PRD Tasks

- Prd(PRD-0036)feat(T-001): add Skipped variant to UatStatus enum

Add UatStatus::Skipped with serde rename 'skipped' and Display impl.
This is a purely additive change that introduces a new terminal UAT
state for intentionally skipped acceptance tests.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)feat(T-002): Update finalize to accept skipped UATs

Updated validate_all_uats_verified to explicitly document that both
Verified and Skipped are acceptable terminal states. Added three new
tests covering all-skipped, mixed verified/skipped, and skipped-with-
unverified scenarios.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)feat(T-003): Add --disallow-skip-uat and --disallow-add-task CLI flags

Add two new flags to the 'mr run' command:
- --disallow-skip-uat: prevents the agent from skipping UATs
- --disallow-add-task: prevents the agent from adding tasks

Thread these through RunConfig/UatVerificationConfig into prompt building
functions as placeholder context values for conditional prompt sections.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)feat(T-004): Update run task prompt to allow dynamic task addition

Add a conditional {{#if allow_add_task}} section to PROMPT_RUN_TASK in
src/commands/init.rs that instructs the agent it may add new tasks to the
PRD frontmatter during execution. Includes guidelines for task ID
assignment, status, and guidance to prefer adding a task over skipping a
UAT when the task would unblock verification.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)feat(T-005): update UAT verification prompt to allow skipping

Add conditional sections to PROMPT_RUN_UAT_VERIFY for marking UATs as
skipped (Option D) and adding tasks to unblock verification (Option E).
Both sections are conditioned on allow_skip_uat and allow_add_task
placeholders respectively.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)feat(T-006): Thread new flags through prompt building

Add tests verifying allow_add_task and allow_skip_uat conditional
sections in build_prompt and build_uat_verify_prompt work correctly
when flags are true and false. 4 new tests added (562 total).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)feat(T-007): restore .mr/prompts with updated prompt files

Sync .mr/prompts/ with src/commands/init.rs per constitution rule 7.
Updated run_task.md and run_uat_verify.md with new conditional sections
for UAT skipping and dynamic task addition. Removed obsolete prompts
(prd_edit.md, prd_new_discovery.md, prd_new_synthesize_prd.md) and added
new interactive prompts (prd_edit_interactive.md, prd_new_interactive.md).

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)feat(T-008): add unit tests for UatStatus::Skipped serde and Display

Add 5 new unit tests in src/prd/types.rs:
- UatStatus Display impl for all variants
- UatStatus serde_yaml round-trip for all variants
- UatStatus::Skipped deserialization from YAML string
- UatStatus default is Unverified
- AcceptanceTest struct round-trip with Skipped status

567 tests run, 567 passed, 0 skipped.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)uat(uat-001): verify UatStatus::Skipped serde round-trip

Existing tests in src/prd/types.rs already cover deserialization
from YAML and serialization back for UatStatus::Skipped.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)uat(uat-002): verify finalize accepts verified or skipped UATs

Existing tests in src/prd/finalize.rs cover this criterion:
- test_validate_all_uats_verified_with_all_skipped
- test_validate_uats_mixed_verified_and_skipped

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)uat(uat-003): verify finalize rejects unverified UATs

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)uat(uat-004): verify --disallow-skip-uat and --disallow-add-task CLI flags

Verified by existing tests:
- test_args_parse_run_with_disallow_skip_uat
- test_args_parse_run_with_disallow_add_task
- test_args_parse_run_default_disallow_flags_off

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)uat(uat-005): verify agent can add tasks during execution

Existing tests test_build_prompt_allow_add_task_true and
test_build_prompt_allow_add_task_false in src/commands/run.rs verify
that the task execution prompt correctly includes/excludes the dynamic
task addition section based on the allow_add_task flag.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)uat(uat-006): verify agent can mark UAT as skipped

Verified by existing test test_build_uat_verify_prompt_allow_skip_uat_true
which confirms the UAT verification prompt includes "Option D: Mark as
Skipped" when allow_skip_uat=true.

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)uat(uat-007): verify --disallow-skip-uat excludes skip section from prompt

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)uat(uat-008): verify --disallow-add-task prevents add-task in prompt

Verified by existing tests:
- test_build_prompt_allow_add_task_false
- test_build_uat_verify_prompt_allow_skip_uat_false

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>
- Prd(PRD-0036)finalize: UAT skipping and dynamic task addition during run

Co-authored-by: Copilot <223556219+Copilot@users.noreply.github.com>

## [0.8.4] - 2026-02-12

## [0.8.3] - 2026-02-10

## [0.8.2] - 2026-02-10

### 🚜 Refactor

- Extract shared JSON parsing helpers from Claude/Codex runners
- Remove redundant delegation methods in Claude/Codex runners
- Extract PrdSummary::to_placeholder_list to eliminate DRY violation

## [0.8.1] - 2026-02-08

### 📋 PRD Tasks

- Prd(PRD-0035)feat(T-001): add CodexRunner with full CliRunnerConfig implementation
- Prd(PRD-0035)feat(T-010): add comprehensive unit tests for CodexRunner
- Prd(PRD-0035)feat(T-009): update --runner help text and AGENTS.md for codex runner
- Prd(PRD-0035)uat(uat-001): verify project builds cleanly with codex runner module
- Prd(PRD-0035)uat(uat-002): verify all 542 tests pass with codex runner
- Prd(PRD-0035)uat(uat-003): verify clippy pedantic passes with no warnings
- Prd(PRD-0035)uat(uat-004): verify runner selectable via --runner codex flag
- Prd(PRD-0035)uat(uat-005): verify build_args produces correct CLI invocation
- Prd(PRD-0035)uat(uat-006): verify token usage parsing with existing tests
- Prd(PRD-0035)uat(uat-007): verify interactive mode args via existing tests
- Prd(PRD-0035)uat(uat-008): verify full CI pipeline passes
- Prd(PRD-0035)finalize: Codex runner support with full test coverage and documentation

## [0.8.0] - 2026-02-07

### 📋 PRD Tasks

- Prd(PRD-0034)feat(T-001): add PROMPT_PRD_EDIT_INTERACTIVE template and PrdEditInteractive prompt kind
- Prd(PRD-0034)feat(T-002): rewrite edit_prd() to use execute_interactive()
- Prd(PRD-0034)feat(T-003): replace required request with optional --context in prd edit
- Prd(PRD-0034)feat(T-004): remove --stream flag from prd new CLI
- Prd(PRD-0034)feat(T-005): register PrdEditInteractive prompt (no-op, already done in T-001)
- Prd(PRD-0034)feat(T-006): remove old PrdEdit prompt variant and PROMPT_PRD_EDIT constant
- Prd(PRD-0034)feat(T-007): add comprehensive tests for interactive edit flow
- Prd(PRD-0034)feat(T-008): document prd edit interactive workflow in AGENTS.md
- Prd(PRD-0034)uat(uat-001): verify interactive session with existing PRD context
- Prd(PRD-0034)uat(uat-002): verify --context passes upfront context to interactive prompt
- Prd(PRD-0034)uat(uat-003): verify Ctrl+C abort preserves PRD integrity
- Prd(PRD-0034)uat(uat-004): add test for post-edit validation and index regeneration
- Prd(PRD-0034)uat(uat-005): verify stream flag removed from prd new CLI
- Prd(PRD-0034)uat(uat-006): verify existing edit and new unit tests pass
- Prd(PRD-0031)finalize: cross-platform CLI checks with which crate
- Prd(PRD-0034)finalize: Switch prd edit to interactive mode with all 8 tasks complete

## [0.7.2] - 2026-02-07

## [0.7.1] - 2026-02-07

## [0.7.0] - 2026-02-07

### 📋 PRD Tasks

- Prd(PRD-0033)feat(T-001): add suggestion to constitution-missing error
- Prd(PRD-0033)feat(T-002): add suggestion for malformed config.toml parse errors
- Prd(PRD-0033)feat(T-003): add suggestion to PRD-not-found errors
- Prd(PRD-0033)feat(T-004): add suggestions to invalid PRD format errors
- Prd(PRD-0033)feat(T-005): improve interactive session failure messages with retry suggestions
- Prd(PRD-0033)feat(T-006): improve partial init state detection and messaging
- Prd(PRD-0033)feat(T-007): add unit tests for improved error messages
- Prd(PRD-0033)uat(uat-001): verify constitution missing suggestion test
- Prd(PRD-0033)uat(uat-002): verify malformed config.toml suggests mr restore
- Prd(PRD-0033)uat(uat-003): verify PRD-not-found suggests mr status
- Prd(PRD-0033)uat(uat-004): verify invalid PRD format suggestion tests
- Prd(PRD-0033)uat(uat-005): verify interactive session failure retry suggestion
- Prd(PRD-0033)uat(uat-006): verify partial init state suggestion test
- Prd(PRD-0033)uat(uat-007): verify all existing tests pass via cargo make ci
- Prd(PRD-0033)finalize: actionable suggestion text for common CLI errors

## [0.6.0] - 2026-02-06

### 📋 PRD Tasks

- Prd(PRD-0032)feat(T-001): add execute_interactive() method to Runner trait
- Prd(PRD-0032)feat(T-002): implement execute_interactive() for CopilotRunner
- Prd(PRD-0032)feat(T-003): implement execute_interactive() for ClaudeRunner
- Prd(PRD-0032)feat(T-006): refactor prd::new to two-phase interactive flow

Replace multi-round Q/A workflow with two-phase interactive flow:
- Phase 1: execute_interactive() for direct user-agent chat
- Phase 2: execute() for PRD synthesis from conversation context
- Abort on Ctrl+C/error without creating PRD

Also completes T-004 (MockRunner interactive support) and T-011 (test updates).

Files changed:
- src/prd/new.rs: core refactor, new prompt builders, updated tests
- src/runner/mock.rs: full execute_interactive() implementation
- src/runner/types.rs, cli_runner.rs, mod.rs: remove dead_code allows
- src/main.rs, src/commands/suggest.rs: updated callers
- src/util/qa_workflow.rs: dead_code allow for deferred cleanup
- Prd(PRD-0032)feat(T-005): add interactive discovery prompt for PRD creation
- Prd(PRD-0032)feat(T-008): handle conversation context handoff between phases

Add session-resume support for context handoff between interactive
discovery (phase 1) and PRD synthesis (phase 2):

- Add execute_continue() to Runner trait for session-resume execution
- Add build_continue_args() to CliRunnerConfig for CLI-specific resume args
- ClaudeRunner: implements --continue -p for session resume
- CopilotRunner: returns None (no session resume support, uses transcript)
- Update synthesis flow to prefer session resume, fall back to transcript
- Update synthesis prompt: replace qa_history with conversation_transcript
  and session_id placeholders for interactive flow context

508 tests pass (net +8 new tests).
- Prd(PRD-0032)feat(T-009): handle Ctrl+C and error cases in interactive mode

- Add RunnerError::Interrupted variant for signal-based interruptions
- Detect SIGINT/SIGTERM in execute_interactive_cli on Unix via ExitStatusExt::signal()
- Provide user-friendly abort message in create_prd for Ctrl+C
- Add set_interactive_error to MockRunner for testing error paths
- Add 9 new tests covering signal detection, error classification, and abort behavior
- UAT: 517 tests passed, 0 failures
- Prd(PRD-0032)feat(T-007): remove old multi-round Q/A workflow

Remove dead code from the old multi-round Q/A workflow that was replaced
by the two-phase interactive flow in T-006:

- Remove collect_multiline_answers() from qa_workflow.rs (dead code)
- Remove PrdNewRound1Questions and PrdNewRoundNQuestions prompt variants
- Remove PROMPT_PRD_NEW_ROUND1 and PROMPT_PRD_NEW_ROUNDN constants
- Remove prd_new_round1_questions and prd_new_round_n_questions config fields
- Delete materialized prompt files from .mr/prompts/
- Remove 6 old Q/A tests from prd::new (covered by qa_workflow.rs tests)
- Update test counts across prompt loader, init, and types

Functions still used by prd::edit and config/constitution (parse_questions,
collect_singleline_answers, extract_prd_content, etc.) are preserved.
- Prd(PRD-0032)feat(T-010): update synthesis prompt for conversation transcript context
- Prd(PRD-0032)feat(T-012): document two-phase interactive PRD creation workflow in AGENTS.md
- Prd(PRD-0032)uat(uat-001): verify interactive session launches for CopilotRunner
- Prd(PRD-0032)uat(uat-002): verify ClaudeRunner interactive session launch
- Prd(PRD-0032)uat(uat-003): verify synthesis from conversation context
- Prd(PRD-0032)uat(uat-004): verify Ctrl+C abort behavior with existing tests
- Prd(PRD-0032)uat(uat-005): verify context injection into interactive session
- Prd(PRD-0032)uat(uat-006): verify MockRunner interactive mode support
- Prd(PRD-0032)uat(uat-007): verify old Q/A loop code removed from prd::new
- Prd(PRD-0032)uat(uat-008): verify CI passes with clippy pedantic
- Prd(PRD-0032)finalize: complete interactive PRD creation with docs updates

## [0.5.3] - 2026-02-06

## [0.5.2] - 2026-02-06

## [0.5.1] - 2026-02-06

### 📋 PRD Tasks

- Prd(PRD-0031)feat(T-001): add which crate dependency
- Prd(PRD-0031)feat(T-002): replace Command::new("which") with which::which() in check_cli_available
- Prd(PRD-0031)feat(T-003): update unit tests for cross-platform check_cli_available
- Prd(PRD-0031)feat(T-004): replace command -v with duckscript which in Makefile.toml
- Prd(PRD-0031)feat(T-005): audit platform-specific shell assumptions
- Prd(PRD-0031)uat(uat-001): verify CI pipeline passes after cross-platform changes
- Prd(PRD-0031)uat(uat-002): verify check_cli_available uses which crate
- Prd(PRD-0031)uat(uat-003): verify Windows MSVC cross-compilation

## [0.5.0] - 2026-02-05

### 📋 PRD Tasks

- Prd(PRD-0029)feat(T-001): fix uninlined_format_args warnings

- Applied clippy --fix to inline 78 format string arguments
- Files modified: 16 source files
- UAT passed: 484 tests green
- Prd(PRD-0029)feat(T-003): Add backticks to doc comments for clippy::doc_markdown compliance

Fixed 43 instances of doc_markdown warnings across 18 files:
- colors.rs, config.rs, graph.rs, init.rs
- prd/index.rs, prd/parser.rs, prd/types.rs
- prd_new.rs, prompt/types.rs, qa_workflow.rs, reindex.rs
- run.rs, runner/claude.rs, runner/cli_runner.rs
- runner/copilot.rs, runner/types.rs, spinner.rs, validate.rs

Used intra-doc link syntax for type references (e.g., [`Prd`]) and
regular backticks for field names, variables, and signal names.
- Prd(PRD-0029)feat(T-004): Remove unnecessary raw string hashes
- Prd(PRD-0029)feat(T-005): Fix redundant closures (11 instances)
- Prd(PRD-0029)feat(T-006): Fix items_after_statements lint warnings
- Prd(PRD-0029)feat(T-007): Fix option_if_let_else warnings with map_or/map_or_else
- Prd(PRD-0029)feat(T-008): Fix map().unwrap_or() patterns with map_or/map_or_else
- Prd(PRD-0029)feat(T-009): Fix casting warnings with local allows

- Fixed 4 casting warnings (PRD estimated 3)
- runner/copilot.rs: Added allow for f64->u64 conversion (token counts)
- status.rs: Added allow for usize->f64 conversion (task counts)
- Both conversions are safe due to domain constraints

UAT: 484 tests passed
- Prd(PRD-0029)feat(T-010): fix miscellaneous pedantic lint warnings
- Prd(PRD-0029)feat(T-011): Address functions with too many lines

Added #[allow(clippy::too_many_lines)] to 9 functions that are inherently
sequential command handlers not practical to refactor:
- init.rs: init()
- prd_edit.rs: edit_prd()
- prd_new.rs: create_prd()
- run.rs: run_uat_verification_loop()
- status.rs: format_status()
- suggest.rs: suggest()
- main.rs: main(), cmd_prd_list(), cmd_run()

PRD-0029 is now complete with all 11 tasks done and all UATs verified.
- Prd(PRD-0030)feat(T-001): Create commands/ module for CLI command implementations
- Prd(PRD-0030)feat(T-002): consolidate prd_*.rs files into prd/ module

- Move prd_edit.rs to prd/edit.rs
- Move prd_new.rs to prd/new.rs
- Move prd_finalize.rs to prd/finalize.rs
- Update prd/mod.rs with new submodule declarations
- Update main.rs and commands/suggest.rs imports
- UAT passed: 484 tests
- Prd(PRD-0030)feat(T-003): create util/ module for utility functions

- Created src/util/ module with colors, spinner, qa_workflow
- Updated all imports to use crate::util:: paths
- All 484 tests pass
- Prd(PRD-0030)feat(T-004): Move config and constitution_edit into config/ module
- Prd(PRD-0030)feat(T-005): Update main.rs module declarations and imports

- Verified main.rs module declarations match new structure
- Module declarations: changelog, commands, config, prd, prompt, runner, util
- Use statements properly import from new module paths
- No code changes needed; imports were updated incrementally by T-001 through T-004
- UAT: cargo make uat passed with 484 tests
- Prd(PRD-0030)feat(T-006): update cross-module imports to use explicit paths
- Prd(PRD-0030)feat(T-007): Verify CI passes with new module structure

- Verified full CI pipeline passes after module reorganization
- All 484 tests pass successfully
- Both UATs (uat-001, uat-002) verified and marked as complete
- All 7 tasks in PRD-0030 are now done
- Prd(PRD-0030)finalize: source module reorganization complete

## [0.4.0] - 2026-01-28

### 📋 PRD Tasks

- Prd(PRD-0024)feat(T-001): Add indicatif dependency for spinners
- Prd(PRD-0024)feat(T-002): Add spinner utility module
- Prd(PRD-0024)feat(T-003): Integrate spinner into mr run command
- Prd(PRD-0024)feat(T-004): verify spinner resets between loop iterations
- Prd(PRD-0024)feat(T-005): integrate spinner into mr refactor command
- Prd(PRD-0024)feat(T-006): integrate spinner into mr suggest command
- Prd(PRD-0024)feat(T-007): Add spinner to finalize command
- Prd(PRD-0024)feat(T-008): integrate spinner into mr reindex command
- Prd(PRD-0024)feat(T-009): add integration tests for spinner behavior
- Prd(PRD-0024)uat(uat-001): verify spinner displays during mr run
- Prd(PRD-0024)uat(uat-002): verify refactor spinner test
- Prd(PRD-0024)uat(uat-003): verify spinner displays during mr suggest
- Prd(PRD-0024)uat(uat-004): verify spinner hidden in non-TTY
- Prd(PRD-0024)uat(uat-005): verify spinner clears before output
- Prd(PRD-0024)finalize: Add progress spinners for long-running operations
- Prd(PRD-0025)feat(T-001): Add clippy lint to deny unwrap_used in production code

- Add #[allow(clippy::unwrap_used)] to all test modules (28 files)
- Add temporary inline allows with TODO comments for production unwraps
  (T-002 through T-006 will remove these)
- All 360 tests pass
- Prd(PRD-0025)feat(T-002): Fix bootstrap.rs Regex::new unwrap with LazyLock
- Prd(PRD-0025)feat(T-003): use LazyLock for spinner template
- Prd(PRD-0025)feat(T-004): fix suggest.rs file_name unwrap

Replace unwrap() with map().unwrap_or_default() pattern in
analyze_codebase() to gracefully handle edge case of path
without file name component.
- Prd(PRD-0025)feat(T-005): fix suggest.rs selection parsing unwraps
- Prd(PRD-0025)feat(T-006): fix prd/index.rs regex expect with LazyLock
- Prd(PRD-0025)feat(T-007): Verify function signatures already complete
- Prd(PRD-0025)finalize: Complete unwrap elimination with clippy lint
- Prd(PRD-0026)feat(T-001): Add depends_on field to PrdFrontmatter struct
- Prd(PRD-0026)feat(T-002): Add reconstructed field to PrdFrontmatter
- Prd(PRD-0026)feat(T-003): Add depends_on field to PRD template
- Prd(PRD-0026)feat(T-004): add --reconstruct flag to bootstrap command
- Prd(PRD-0026)feat(T-005): Create bootstrap_reconstruct.md prompt

- Add BootstrapReconstruct variant to PromptKind enum
- Add PROMPT_BOOTSTRAP_RECONSTRUCT constant in init.rs
- Add bootstrap_reconstruct.md to .mr/prompts/
- Update prompt count in tests (17 -> 18)
- Prd(PRD-0026)feat(T-006): Implement reconstruct logic in bootstrap.rs
- Prd(PRD-0026)feat(T-007): Handle idempotency for --reconstruct with existing PRDs

- Updated build_reconstruct_prompt() to scan for existing PRDs and include
  them in the prompt context using PlaceholderValue::List
- Added 'Existing PRDs (Do Not Duplicate)' section to bootstrap_reconstruct
  prompt with {{#if existing_prds}} and {{#each existing_prds}} placeholders
- Synchronized .mr/prompts/bootstrap_reconstruct.md with init.rs constant
- Added 2 unit tests for idempotency behavior

UAT: 367 tests passed
- Prd(PRD-0026)feat(T-008): Create graph module with shared graph-building logic
- Prd(PRD-0026)feat(T-009): implement ASCII graph rendering

- Add AsciiConfig struct for rendering configuration
- Implement render_ascii() function for dependency graph visualization
- Show nodes with [ID] Title (status) format
- Display dependencies with tree connectors (├── and └──)
- Render missing refs in separate section with warnings
- Add summary stats (PRD count, edges, missing count)
- Add 7 unit tests for ASCII rendering
- Prd(PRD-0026)feat(T-010): implement mermaid graph rendering
- Prd(PRD-0026)feat(T-011): Implement graph dot subcommand
- Prd(PRD-0026)feat(T-012): Add graph command to CLI with subcommands

- Added GraphCommand enum with ascii, mermaid, dot subcommands
- Each has --no-titles, --max-title-len flags; mermaid/dot have --lr
- Implemented cmd_graph_ascii, cmd_graph_mermaid, cmd_graph_dot functions
- Added 6 CLI parsing tests for graph command
- Removed module-level dead_code allow from graph.rs
- UAT passed: 408 tests run, 408 passed
- Prd(PRD-0026)feat(T-013,T-014): enhance reindex with LLM-driven depends_on auto-fix

- Add ReindexDependsOn prompt kind for analyzing PRD dependencies
- Update reindex.rs with run_depends_on_fix() for second phase analysis
- Add PROMPT_REINDEX_DEPENDS_ON constant and physical prompt file
- Update ReindexResult with depends_on_added/fixed fields
- Add helper functions: extract_summary(), parse_depends_on_counts()
- Update cmd_reindex() to display new depends_on statistics
- Add 8 unit tests for new functions
- All 415 tests pass
- Prd(PRD-0026)feat(T-015): add depends_on field to PRDS.md index

- Add depends_on field to PrdSummary struct
- Generate Dependencies section in PRDS.md showing PRD dependency relationships
- Add 5 unit tests for dependency extraction and section generation
- Update all files constructing PrdSummary to include new field
- Prd(PRD-0026)feat(T-016): Add unit tests for graph building and rendering
- Prd(PRD-0026)feat(T-017): Add integration tests for reconstruct and graph commands

- Added 4 integration tests to bootstrap.rs for reconstruct workflow
  - Test full workflow with mock git repo
  - Test idempotency with existing PRDs
  - Test depends_on inference guidance
  - Test end-to-end with index regeneration

- Added 8 integration tests to graph.rs for graph commands
  - Test graph building from disk PRD files
  - Test ASCII, Mermaid, DOT rendering with real PRDs
  - Test missing dependency warning handling
  - Test empty repository handling
  - Test complex dependency chains
  - Test with reconstructed PRDs

Test count: 438 → 450 (12 new tests)
- Prd(PRD-0026)feat(T-018): Add graph and reconstruct documentation to AGENTS.md
- Prd(PRD-0026)uat(uat-001): verify reconstruct creates PRDs with depends_on
- Prd(PRD-0026)uat(uat-002): verify graph ascii renders dependency graph
- Prd(PRD-0026)uat(uat-003): verify mermaid flowchart output tests
- Prd(PRD-0026)uat(uat-004): verify graph dot outputs valid Graphviz DOT format
- Prd(PRD-0026)uat(uat-005): add integration tests for reindex depends_on auto-fix
- Prd(PRD-0026)uat(uat-006): verify graph warns and renders dashed nodes for missing depends_on
- Prd(PRD-0026)finalize: Complete bootstrap reconstruct, depends_on, and graph command
- Prd(PRD-0027)feat(T-001): Replace --reconstruct with --scaffold flag
- Prd(PRD-0027)feat(T-002): Update BootstrapConfig default to reconstruct=true

- Changed BootstrapConfig::new() to default reconstruct to true
- Updated module and function doc comments to reflect new default
- Updated tests to explicitly set reconstruct=false for scaffold behavior
- Prd(PRD-0027)feat(T-003): Update bootstrap_reconstruct.md prompt for new default
- Prd(PRD-0027)feat(T-004): Update AGENTS.md Bootstrap documentation
- Prd(PRD-0027)feat(T-005): Update init.rs embedded prompts to match updated prompt files
- Prd(PRD-0027)feat(T-006): Update tests to reflect reconstruct as default

- Update src/prd/types.rs doc comment for reconstructed field
- Update src/graph.rs test comment to use 'mr bootstrap' without flag
- Remove redundant config.reconstruct=true from 7 bootstrap tests
- All 451 tests pass
- Prd(PRD-0027)feat(T-007): verify all --reconstruct references removed
- Prd(PRD-0027)uat(uat-001): verify default reconstruct behavior
- Prd(PRD-0027)uat(uat-002): verify scaffold behavior with existing tests
- Prd(PRD-0027)uat(uat-003): verify help text shows --scaffold flag
- Prd(PRD-0027)uat(uat-004): verify documentation updates via manual grep analysis
- Prd(PRD-0027)finalize: Swap bootstrap default to reconstruct mode
- Prd(PRD-0028)feat(T-001): Add Technical Approach section to PRD template
- Prd(PRD-0028)feat(T-002): Add Assumptions section to PRD template
- Prd(PRD-0028)feat(T-003): Add Constraints section to PRD template
- Prd(PRD-0028)feat(T-004): Add References to Code section to PRD template
- Prd(PRD-0028)feat(T-005): Update Round1 prompt to elicit technical approach details
- Prd(PRD-0028)feat(T-006): Update synthesize prompt with new PRD sections
- Prd(PRD-0028)feat(T-007): sync prompts and templates via mr restore
- Prd(PRD-0028)feat(T-008): Verify all UATs pass
- Prd(PRD-0028)finalize: Enhanced PRD exposition with Technical Approach, Assumptions, Constraints, and References sections

### 🚜 Refactor

- Extract add_optional helper to reduce DRY violation in aggregate_usage
- Move UsageInfo aggregation to impl block for DRY
- Fix off-by-one bug in output tokens regex capture group
- Rename PermissionMode to runner-specific names for clarity
- Extract shared NodeDisplayConfig for graph renderers
- Extract ensure_initialized helper to eliminate DRY violation
- Consolidate Runner impl via blanket impl for CliRunnerConfig
- Extract shared PRD link formatting helpers to eliminate DRY violation
- Embed NodeDisplayConfig in graph config structs to eliminate DRY violation
- Extract read_stream_lines helper to reduce duplication in CLI streaming
- Extract analyze_codebase into separate helper functions
- Extract helper functions from parse_suggestions
- Improve spinner tests to verify spinner state
- Remove redundant assertions from spinner simulation tests
- Improve colors.rs tests to validate actual behavior
- Improve spinner tests with meaningful assertions
- Improve devcontainer tests to actually verify behavior
- Improve tautological test_build_refactor_prompt_with_context test
- Improve test_spinner_cow_static_vs_owned with meaningful assertions
- Improve tautological test_spinner_run_task_simulation test
- Improve tautological spinner tests with meaningful assertions
- Improve tautological spinner tests to test actual contracts
- Extract common flag-building logic in CopilotRunner to eliminate duplication
- Extract to_placeholder_list() for QaPair formatting
- Extract common flag-building logic in ClaudeRunner to eliminate duplication
- Move aggregate_usage to UsageInfo::aggregate method
- Simplify RunnerOutput construction by directly assigning usage field

## [0.3.1] - 2026-01-26

## [0.3.0] - 2026-01-26

### 🚀 Features

- *prd_new*: Add hint about pressing Enter twice to complete answers

### 🐛 Bug Fixes

- *bootstrap*: Enforce numeric priority values in generated PRDs

### 📋 PRD Tasks

- Prd(PRD-0018)feat(T-001): Complete audit of Rust-side file writes

- Audited all source files for file write operations
- Identified append_opt_out_history() and update_uat_status() in run.rs as removal targets
- Verified prd_edit.rs and constitution_edit.rs patterns are acceptable
- Confirmed init/bootstrap and index regeneration remain out-of-scope
- Updated PRD-0018 status to active, marked T-001 as done
- All UATs pass with no regressions
- Prd(PRD-0018)feat(T-002): Remove Rust code that manipulates PRD files

- Removed append_opt_out_history() and update_uat_status() functions
- Agents now fully responsible for History entries and UAT status updates
- Updated 4 tests to reflect new agent-driven workflow
- All UATs pass (318/318 tests)
- Prd(PRD-0018)feat(T-003): Add YAML frontmatter validation after agent edits

- Created src/validate.rs module with validation functions for PRD and Constitution files
- Added validation calls in run.rs, prd_edit.rs, and constitution_edit.rs after agent edits
- Validation emits warnings (not errors) for malformed YAML frontmatter
- Added comprehensive unit tests for validation functions
- All 324 UATs pass
- Opportunistically verified both acceptance tests (uat-001 and uat-002)
- Prd(PRD-0018)feat(T-004): verify prompts correctly support edit-in-place
- Prd(PRD-0018)feat(T-005): Run full UAT suite and verify no regressions
- Prd(PRD-0018)finalize: Prefer Edit-in-Place for Agent Actions
- Prd(PRD-0019)feat(T-001): Audit prompts for behavioral guidance
- Prd(PRD-0019)feat(T-002): add public API stability and root cause resolution rules to constitution
- Prd(PRD-0019)feat(T-003): Update default constitution with 6 comprehensive rules
- Prd(PRD-0019)feat(T-004): Remove behavioral guidance from prompt templates
- Prd(PRD-0019)feat(T-005): Test updated prompts and constitution with mr restore
- Prd(PRD-0019)feat(T-006): Add prompt/constitution philosophy to AGENTS.md
- Prd(PRD-0019)uat(uat-001): verify no unrelated code guidance in prompts
- Prd(PRD-0019)uat(uat-002): add test verifying constitution has under 10 rules
- Prd(PRD-0019)uat(uat-003): Add test for workflow-focused prompts
- Prd(PRD-0019)finalize: Tighten prompts and consolidate behavior in constitution
- Prd(PRD-0020)feat(T-001): Add no_commit option to config and CLI args

- Add no_commit: Option<bool> field to Config struct
- Add effective_no_commit() method with CLI > config > default precedence
- Add --no-commit flag to Run command in main.rs
- Update DEFAULT_CONFIG with commented no_commit option
- Add unit tests for config parsing and precedence logic
- Prd(PRD-0020)feat(T-002): Add commit conditional to prompt templates
- Prd(PRD-0022)feat(T-001): Add Refactor subcommand to main.rs CLI

- Add Refactor variant to Command enum with flags:
  --max, --context, --path, --dry-run, --no-commit, --runner, --model, --stream
- Add match arm and stub cmd_refactor() function
- Add CLI parsing tests for refactor command
- Opportunistically verify UATs 1-4 (all CLI flag tests pass)
- Update PRD status to active, T-001 to done
- Prd(PRD-0022)feat(T-002): implement refactor command loop logic

- Create refactor.rs module with RefactorConfig, loop logic, and result types
- Add Refactor variant to PromptKind enum
- Add PROMPT_REFACTOR constant with constitution/context/path support
- Implement early termination via NO-MORE-REFACTORS signal
- Implement preview mode via PREVIEW-COMPLETE signal
- Add unit tests for signal detection and prompt building
- Wire up cmd_refactor in main.rs to use refactor::refactor()
- Update file count assertions for new prompt (21→22 files, 16→17 prompts)

Tasks completed: T-002, T-003, T-004, T-005, T-006
UAT: cargo make uat passes (340 tests)
- Prd(PRD-0022)feat(T-007): document refactor command workflow in AGENTS.md
- Prd(PRD-0022)uat(uat-005): test early termination loop behavior
- Prd(PRD-0022)uat(uat-006): verify --no-commit flag in refactor prompt

- Add test_build_refactor_prompt_no_commit_true/false tests
- Fix template bug: {{#unless}} not supported, use {{#if no_commit}}
- Add no_commit variable to prompt context
- All 343 tests pass
- Prd(PRD-0022)uat(uat-007): verify CI passes after refactor implementation
- Prd(PRD-0021)feat(T-001): Update README install to kord pattern with target triples
- Prd(PRD-0021)feat(T-002): verify github-release attaches zipped artifacts
- Prd(PRD-0021)feat(T-003): fix Windows PowerShell install instructions
- Prd(PRD-0021)finalize: Complete binary zip release workflow

## [0.2.0] - 2026-01-25

## [0.1.0] - 2026-01-25

### 🚀 Features

- Implement static prompt library + placeholder system (T-005)
- *T-006, T-007*: Implement `mr prd new` guided Q/A + Runner abstraction
- *T-008*: Implement CopilotRunner adapter
- *T-009*: Implement `mr run` command
- *T-010*: Implement `mr status` command
- *T-011*: AGENTS.md updater with safe, bounded patching
- *T-012*: Implement mr bootstrap command
- *T-013*: Remove allow flags and clean clippy lints
- *T-014*: Implement `mr prd edit` for quick PRD modifications
- *T-016*: Add --language flag to init and bootstrap commands
- *T-017*: Document placeholder variables for each prompt in README
- *T-018*: Add .mr/config.toml for persistent settings
- *T-019*: Stream runner output during mr run
- *T-020*: Add reindex command for regenerating PRD index and fixing links
- *T-099*: Revamp docs, add DEVELOPMENT.md, comparison table
- *T-001*: Add mr prd finalize CLI command
- *T-002*: Implement task completion validation for PRD finalization
- *T-003*: Run acceptance test verification via finalization prompt

### 🐛 Bug Fixes

- *finalize*: Enable finalization summary report output
- Add missing constitution_edit and devcontainer_generate prompt files to init/restore

### 📋 PRD Tasks

- Prd(PRD-0004)feat(T-004): add changelog.rs module for Keep a Changelog support
- Prd(PRD-0004)feat(T-005): Add changelog entry generation to finalization prompt
- Prd(PRD-0004)feat(T-006): Generate summary report (append to PRD + stdout)

- Added generate_summary_report() function for formatted finalization reports
- Added append_to_prd() function to append history entries to PRD files
- Extended PrdFinalizeResult with summary_report field
- Enhanced cmd_prd_finalize output with visual formatting
- Added unit tests for summary report generation and file append
- UAT passes: 217/217 tests
- Prd(PRD-0004)feat(T-007): Update PRD status to done and refresh PRDS.md index

- Made serialize_prd() public in prd/parser.rs
- Added update_prd_status_to_done() function to set PRD status to done
- Integrated status update and index regeneration into finalize_prd()
- Added output messages for status update and index regeneration
- Added unit tests for status update functionality
- UAT passes: 219/219 tests
- Prd(PRD-0004)feat(T-010): Verify finalization history entry appending

T-010 functionality was already implemented by T-006:
- generate_summary_report() creates formatted history entry
- append_to_prd() appends entry to PRD file
- Entry includes date, timestamp, PRD info, task list, outcome

No code changes required; verified complete with 219/219 tests passing.
- Prd(PRD-0004)feat(T-011): Comprehensive finalization prompt instructions

Updated run_task_finalize.md and init.rs PROMPT_RUN_TASK_FINALIZE with:
- 6 numbered sections matching Design Notes requirements
- Detailed acceptance test verification instructions
- Keep a Changelog format and category guidelines
- Summary report format template
- Cleanup guidance for temp files and excessive comments
- Inter-PRD link update instructions
- Finalization history entry format
- Documentation check section (README, AGENTS, inline docs)
- Example output format for completion confirmation
- Prd(PRD-0004)feat(T-008): Add inter-PRD cross-references to index

- Add extract_prd_references() to scan PRD body and task notes for PRD-XXXX patterns
- Add references field to PrdSummary struct
- Add Cross-References section to generated PRDS.md index
- Add 7 new unit tests for reference extraction
- Update existing tests for new references field
- Prd(PRD-0004)feat(T-009): Verify cleanup tasks in finalization prompt
- Prd(PRD-0004)finalize: PRD Finalization Steps complete
- Prd(PRD-0003)feat(T-001): add --context CLI flag to prd new command
- Prd(PRD-0003)feat(T-002): add interactive context prompt before question generation
- Prd(PRD-0003)feat(T-003): add user_context placeholder to round1 prompt
- Prd(PRD-0003)feat(T-004): persist context through all Q/A rounds

- Modified build_round_n_prompt to accept user_context parameter
- Updated create_prd to pass context to round N prompts
- Added {{#if user_context}} block to prd_new_roundN_questions.md
- Context now flows through all Q/A rounds, not just round 1
- All 227 tests pass
- Prd(PRD-0003)feat(T-005): Include context in final PRD synthesis prompt
- Prd(PRD-0003)feat(T-006): update help text and documentation for --context flag
- Prd(PRD-0004)finalize: PRD Finalization Steps complete
- Prd(PRD-0005)feat(T-001): Add unverified UAT check to run loop

- Add acceptance_tests(), all_tasks_done(), has_unverified_uats(), and
  unverified_uats() methods to Prd struct
- Change RunResult from struct to enum with TaskExecuted,
  NeedsUatVerification, and PrdComplete variants
- Update run_task() to detect when all tasks done but UATs unverified
- Update main.rs run loop to handle new RunResult variants
- Add 3 new unit tests for UAT verification detection
- All 230 tests passing
- Prd(PRD-0005)feat(T-002): Create UAT verification prompt template
- Prd(PRD-0005)feat(T-003): Implement UAT verification loop in run.rs

- Add run_uat_verification_loop() function that iterates over unverified UATs
- Add UatVerificationConfig and UatVerificationLoopResult structs
- Add build_uat_verify_prompt() helper for verification prompts
- Add parse_opt_out() to detect OPT-OUT responses from runner
- Wire up verification loop in main.rs when NeedsUatVerification returned
- Loop respects max_iterations from PRD config (default: 10)
- Add 4 new unit tests for verification loop functionality

UAT passed: 238 tests, all passed
- Prd(PRD-0005)feat(T-004): Add model opt-out mechanism with History entry
- Prd(PRD-0005)feat(T-005): Block finalization on unverified UATs

- Added UnverifiedUats variant to FinalizeError enum
- Added validate_all_uats_verified() check in finalize_prd()
- Added 5 unit tests for UAT validation
- UAT passed: 244 tests
- Prd(PRD-0005)feat(T-006): Update run_task.md prompt to reference UAT verification phase
- Prd(PRD-0005)feat(T-007): Add UAT status update logic

- Add update_uat_status() function to programmatically set UAT status to verified
- Modify run_uat_verification_loop() to auto-update UAT status when runner succeeds
- Update test assertions for new behavior (runner success → verified count)
- Add unit tests for update_uat_status() function

246 tests pass.
- Prd(PRD-0005)feat(T-008): Add integration test for UAT verification loop
- Prd(PRD-0005)uat(uat-001): verify UAT verification loop triggers after tasks done
- Prd(PRD-0005)uat(uat-002): Verified loop addresses unverified UATs
- Prd(PRD-0005)uat(uat-003): Verify opt-out mechanism with existing tests
- Prd(PRD-0005)uat(uat-004): verify max_iterations respected
- Prd(PRD-0005)uat(uat-005): Verify unverified UATs block finalization
- Prd(PRD-0005)uat(uat-006): Verify update_uat_status updates PRD frontmatter
- Prd(PRD-0005)uat(uat-007): Verify UAT verification results appended to History

Added test_uat_verification_history_appending to verify that:
- Opt-out History entries are automatically appended via append_opt_out_history()
- Successful verification History entries are manually appended by the runner
- This behavior is by design: runners have full context to write meaningful entries

Test: cargo make uat test_uat_verification_history_appending
Result: All 249 tests passed
- Prd(PRD-0005)finalize: Verify UATs at End of Run Loop
- Prd(PRD-0003)uat(uat-001): verify interactive context prompt flow
- Prd(PRD-0003)uat(uat-002): Verify context flag flow with new test
- Prd(PRD-0003)uat(uat-003): verify context influences question generation
- Prd(PRD-0003)uat(uat-004): verify context persists through Q/A rounds
- Prd(PRD-0003)uat(uat-005): verify context included in final synthesis
- Prd(PRD-0003)finalize: PRD New Allows Upfront Context
- Prd(PRD-0006)feat(T-001): Add owo-colors dependency and create color utilities module
- Prd(PRD-0006)feat(T-002): Colorize success messages with green and emoji prefixes
- Prd(PRD-0006)feat(T-003): Colorize error and warning messages with red/yellow and emoji prefixes
- Prd(PRD-0006)feat(T-004): Style question prompts with blue bold text and emoji prefix
- Prd(PRD-0006)feat(T-005): Add color to informational and status messages
- Prd(PRD-0006)feat(T-006): Colorize finalization summary box and separators
- Prd(PRD-0006)uat(uat-001): verify success messages display green with emoji
- Prd(PRD-0006)uat(uat-003): verify question prompts display in blue with bold text
- Prd(PRD-0006)uat(uat-004): verify colors disabled when piped
- Prd(PRD-0006)uat(uat-005): verify emoji preservation in prd list output
- Prd(PRD-0006)uat(uat-006): verify finalization summary box styling with unit test
- Prd(PRD-0006)uat(uat-007): verify NO_COLOR disables colors
- Prd(PRD-0006)finalize: Add Stdout Colors with terminal colorization
- Prd(PRD-0007)feat(T-001): Extend RunnerOutput with optional usage metadata

- Added UsageInfo struct with optional input_tokens, output_tokens, and total_tokens fields
- Extended RunnerOutput to include optional usage field
- Implemented token usage parsing in CopilotRunner using regex patterns
- Added display logic in main.rs to show usage info after runner output
- Usage information displayed in dim color when available
- All tests pass (256/256), CI passes
- Prd(PRD-0007)feat(T-002): Parse token usage from Copilot CLI output

- Investigated Copilot CLI output format and discovered stats are emitted in non-silent mode
- Updated parse_usage() to correctly parse Copilot format: '18.3k in, 38 out'
- Added support for k/M suffixes (18.3k = 18,300, 1.2M = 1,200,000)
- Changed default silent mode from true to false to enable usage tracking
- Added strip_stats() to remove statistics section while preserving response
- Updated both execute() and execute_streaming() to parse and strip stats
- Added comprehensive unit tests for parsing and stripping functions
- All 261 tests pass, CI pipeline passes
- Prd(PRD-0007)feat(T-003): Display usage info after LLM output
- Prd(PRD-0007)feat(T-004): ensure runners without usage info omit display

- Added test to verify MockRunner omits usage info
- Confirmed existing implementation already handles this correctly
- Updated PRD status and verified UAT-002
- Prd(PRD-0007)finalize: Output Underlying Agent Usage
- Prd(PRD-0009)feat(T-001): Remove --prd flag, add positional PRD arg to run command
- Prd(PRD-0009)feat(T-002): Flatten CLI command structure by removing Prd subcommand
- Prd(PRD-0009)feat(T-003): Update code references to use flattened CLI commands
- Prd(PRD-0009)feat(T-004): Add tracing info for runner invocations

Added tracing::info! calls at all runner invocation points across the
codebase to improve debugging visibility. Each invocation now logs:
- Runner name
- Relevant IDs (PRD, task, slug, etc.)
- Stream mode status
- Prompt length

Changed existing debug-level logging to info-level where appropriate.
All 262 tests passed.
- Prd(PRD-0009)feat(T-005): Show tail of LLM output instead of beginning for better debugging
- Prd(PRD-0009)feat(T-006): Apply tail output to UAT verification loop
- Prd(PRD-0009)feat(T-007): Update documentation for new CLI structure

- Updated README.md with flattened CLI commands (mr new/list/edit vs mr prd new/list/edit)
- Changed mr run --prd <id> to mr run <id> (positional argument)
- Updated AGENTS.md auto-managed section with comprehensive CLI reference
- All 263 tests passing
- Prd(PRD-0009)feat(T-008): Verify all tests pass after CLI ergonomics improvements

- Executed cargo make ci: All 263 tests passed (fmt, clippy, unit tests)
- Executed cargo make uat: All 263 tests passed (acceptance tests)
- No regressions detected from CLI structure changes (T-001 through T-007)
- All CLI ergonomics improvements verified and working correctly
- Prd(PRD-0009)uat(uat-001): verify run command accepts positional PRD argument
- Prd(PRD-0009)uat(uat-002): verify interactive mode test
- Prd(PRD-0009)uat(uat-003): verify list command with new test
- Prd(PRD-0009)uat(uat-004): verify top-level new command works
- Prd(PRD-0009)uat(uat-005): verify top-level edit command
- Prd(PRD-0001)finalize: Complete microralph MVP build with full feature set
- Prd(PRD-0009)uat(uat-006): verify finalize command with test
- Prd(PRD-0009)finalize: CLI Ergonomics Improvements - flattened command structure and improved output readability
- Prd(PRD-0009)finalize: Update prompts to reflect new CLI structure
- Prd(PRD-0009)finalize: Update hardcoded prompt constants in init.rs
- Prd(PRD-0009)finalize: Sync remaining prompts with new CLI structure
- Prd(PRD-0009)finalize: Update DEVELOPMENT.md with new CLI syntax
- Prd(PRD-0010)feat(T-001): detect and display multi-line questions
- Prd(PRD-0010)test(T-001): add multi-line question test and verify uat-001
- Prd(PRD-0010)feat(T-002): Mark multi-line answer input as done

Multi-line answer input was already implemented in collect_answers() function.
Users can now:
- Type multi-line answers by pressing Enter after each line
- Finish by pressing Enter on a blank line (double-enter pattern)
- All lines are joined with newlines and stored

Updated PRD status:
- T-002: marked as done
- UAT-002: marked as verified
- Added History entry documenting completion

All 273 UAT tests pass.
- Prd(PRD-0010)feat(T-003): Verify UAT scenarios pass - all 273 tests passing
- Prd(PRD-0010)finalize: Support multi-line Q/A during PRD creation
- Prd(PRD-0012)feat(T-001): Add constitution template with numbered examples
- Prd(PRD-0012)feat(T-002): emit constitution during bootstrap

- Constitution already emitted via existing init() call in bootstrap()
- Added test test_bootstrap_creates_constitution to verify constitution creation
- Test confirms constitution.md is created when bootstrap initializes .mr/ structure
- Updated PRD-0012 task T-002 status to done
- Regenerated PRD index
- All 275 tests pass
- *PRD-0012*: Document UAT verification status for T-002
- Prd(PRD-0012)feat(T-003): Add constitution edit subcommand

- Created src/constitution_edit.rs module with edit_constitution() function
- Added ConstitutionEdit prompt kind and template
- Implemented 'mr constitution edit <request>' CLI command
- Supports Q/A flow with max 3 rounds before forcing application
- Runner uses READY_TO_APPLY signal to indicate completion
- All 277 tests passed
- Prd(PRD-0010)finalize: Support Multi-line Q/A During PRD Creation
- Prd(PRD-0012)feat(T-004): Load constitution in runner context
- Prd(PRD-0012)feat(T-005): Include constitution in prd new prompts
- Prd(PRD-0012)feat(T-006): Include constitution in prd finalize prompts
- Prd(PRD-0012)feat(T-007): Add constitution violation logging to runner prompts
- Prd(PRD-0012)feat(T-008): Document constitution feature in README
- Prd(PRD-0012)uat(uat-001): Verify bootstrap creates constitution
- Prd(PRD-0012)uat(uat-002): Verified constitution template contains numbered example rules
- Prd(PRD-0012)uat(uat-003): verify prd new reads constitution
- Prd(PRD-0012)uat(uat-004): verify prd finalize reads constitution
- Prd(PRD-0012)uat(uat-005): verify constitution edit command with integration test
- Prd(PRD-0012)uat(uat-006): verify runner logs constitution violations in prompts
- Prd(PRD-0012)finalize: Enable Constitution feature with LLM-assisted editing
- Prd(PRD-0016)feat(T-001): Remove update_agents_md() call from run.rs
- Prd(PRD-0016)feat(T-002): Remove update_agents_md() call from prd_new.rs
- Prd(PRD-0016)feat(T-003): Remove update_agents_md() call from bootstrap.rs
- Prd(PRD-0016)feat(T-007): Add AGENTS.md update reminder to run_task.md prompt
- Prd(PRD-0016)feat(T-008): Add AGENTS.md update reminder to PRD synthesis prompt
- Prd(PRD-0016)feat(T-009): Add AGENTS.md update reminder to bootstrap prompt
- Prd(PRD-0016)feat(T-004): Delete unused agents.rs module

- Removed mod agents declaration from src/main.rs
- Deleted src/agents.rs file (531 lines)
- Module was completely unused after T-001, T-002, T-003 removed all update_agents_md() calls
- Updated PRD-0016 task T-004 status to done and added History entry
- All UATs pass
- Prd(PRD-0016)feat(T-005): Remove update_agents.md prompt creation

- Removed PROMPT_UPDATE_AGENTS constant and file creation logic from init.rs
- Removed PromptKind::UpdateAgents enum variant and all references
- Replaced auto-managed section in STARTER_AGENTS with manual update guidance
- Updated all related tests to reflect new prompt count (14 instead of 15)
- All 267 tests pass
- Prd(PRD-0016)feat(T-006): Delete update_agents.md prompt file
- Prd(PRD-0016)feat(T-010): Verify prompt consistency between init.rs and .md files

- Updated PROMPT_RUN_TASK constant in src/init.rs to add AGENTS.md update reminder (step 7)
- Verified all 14 prompts match between init.rs constants and .mr/prompts/*.md files
- Confirmed AGENTS.md update reminders are present in run_task, prd_new_synthesize_prd, and bootstrap_generate_prds prompts
- All 267 tests pass
- Prd(PRD-0016)uat(uat-001): Verified all UATs pass after AGENTS.md automation removal
- Prd(PRD-0016)uat(uat-002): verify task execution without update_agents_md call
- Prd(PRD-0016)finalize: Remove automatic AGENTS.md update step
- Prd(PRD-0011)feat(T-001): Add dev container documentation to README
- Prd(PRD-0011)feat(T-002): Implement dev container detection utility
- Prd(PRD-0011)feat(T-003): Add dev container warning to model-invoking commands
- Prd(PRD-0011)feat(T-004): Implement mr devcontainer generate command

- Added DevcontainerGenerate prompt kind and default template
- Created new Devcontainer CLI subcommand with Generate command
- Implemented cmd_devcontainer_generate() with repo analysis
- Analyzes repo files, git history, and tools for context
- Generates .devcontainer/devcontainer.json via runner
- Extracts JSON from markdown-wrapped responses
- Also completes T-005 (repo analysis), T-006 (prompt template)
- All 270 tests pass with cargo make uat
- Prd(PRD-0011)feat(T-005): Update PRD status for repo analysis module

T-005 was already implemented in T-004 via analyze_repo_for_devcontainer().
This commit updates the PRD to reflect the actual completion status.
- Prd(PRD-0011)feat(T-006): Verify and document dev container prompt template completion
- Prd(PRD-0011)feat(T-007): Add unit test for devcontainer generate with mock runner

- Added serde_json dependency for JSON validation
- Refactored cmd_devcontainer_generate to extract testable generate_devcontainer_config function
- Created comprehensive unit test with MockRunner and temporary directory
- Test verifies valid JSON generation and file creation
- Opportunistically verified UAT-001 (devcontainer generate test)
- All 271 tests pass
- Prd(PRD-0011)finalize: Dev Container Support and Generation
- Prd(PRD-0008)feat(T-001): Disable rust-cache bin caching to fix cargo-make availability
- Prd(PRD-0008)feat(T-002): Verify CI cargo-make fix complete
- Prd(PRD-0008)uat(uat-001): verified CI passes with all tests
- Prd(PRD-0008)finalize: Fix CI cargo-make availability with rust-cache
- Prd(PRD-0013)feat(T-001): Add ClaudeRunner with full CLI support

- Created src/runner/claude.rs implementing ClaudeRunner
- Mirrors CopilotRunner surface area with Claude-specific flags
- Supports -p for non-interactive mode
- Supports --dangerously-skip-permissions for yolo mode
- Supports --model for model selection
- Supports --no-ask-user for autonomous operation
- Token usage not available in Claude CLI stdout (noted in code)
- Added create_runner() helper in main.rs for centralized runner creation
- Updated all runner instantiation sites to support claude runner
- Updated runner module exports
- All 283 UAT tests pass
- Prd(PRD-0013)feat(T-002): ClaudeConfig struct verified complete
- Prd(PRD-0013)feat(T-003): Complete ClaudeRunner Runner trait implementation

- Verified ClaudeRunner struct fully implements Runner trait with all required methods
- name() returns 'claude'
- execute() runs Claude CLI non-streaming
- execute_streaming() provides real-time output
- is_available() checks Claude CLI installation
- format_command_display() formats command for user display
- Implementation mirrors CopilotRunner pattern exactly
- All 283 tests pass (cargo make uat)
- Prd(PRD-0013)feat(T-004): Implement build_args method for Claude CLI flags
- Prd(PRD-0013)feat(T-005): Implement token usage parsing for Claude CLI

- Implemented JSON output format support via --output-format json
- Fixed invalid --no-ask-user flag to use --permission-mode dontAsk
- Added parse_usage() to extract input/output tokens from JSON response
- Added extract_result_from_json() to extract response text from JSON
- Updated execute() and execute_streaming() to parse JSON and extract usage
- Added comprehensive tests for token usage parsing and JSON extraction
- All 288 tests pass, UAT successful
- *PRD-0013*: Verify all UATs opportunistically after T-005
- Prd(PRD-0013)feat(T-006): Implement output stripping for Claude CLI

- Added public strip_usage_stats() method to ClaudeRunner
- Mirrors CopilotRunner::strip_usage_stats() API for consistency
- Leverages Claude CLI's --output-format json for clean metadata separation
- Extracts only the 'result' field, automatically stripping usage/type/session metadata
- Added comprehensive unit tests for all edge cases
- All 293 tests pass, cargo make uat successful
- Prd(PRD-0013)feat(T-007): verify comprehensive unit test coverage for ClaudeRunner
- Prd(PRD-0013)feat(T-008): Export ClaudeRunner from runner module
- Prd(PRD-0013)feat(T-009): Document ClaudeRunner implementation patterns in AGENTS.md
- Prd(PRD-0013)finalize: Complete Claude CLI runner implementation
- Prd(PRD-0015)feat(T-001): Add SuggestGenerate PromptKind variant

- Added SuggestGenerate variant to PromptKind enum
- Created PROMPT_SUGGEST_GENERATE constant with comprehensive template
- Updated prompt loader to handle new variant
- Updated tests to reflect new prompt count (16 instead of 15)
- All 293 tests passing
- Prd(PRD-0015)feat(T-002): Create suggest_generate.md prompt template

- Added create_file_if_missing call in src/init.rs for suggest_generate.md
- Created .mr/prompts/suggest_generate.md with comprehensive prompt template
- Updated init tests to expect 19 files (14 prompts) instead of 18
- Added assertion for suggest_generate.md in test_init_creates_structure
- All 293 tests passing
- Prd(PRD-0015)feat(T-003): Add top-level suggest command to CLI
- Prd(PRD-0015)feat(T-004,T-005,T-006): Implement suggest module with codebase analysis, numbered picker, and PRD integration
- Prd(PRD-0015)feat(T-007): Add comprehensive UAT tests for suggest command
- Prd(PRD-0015)feat(T-008): Document mr suggest command in AGENTS.md and README.md
- Prd(PRD-0015)uat(uat-001): verify suggest parses exactly 5 suggestions
- Prd(PRD-0015)uat(uat-002): Add validate_selection test for numbered picker
- Prd(PRD-0015)uat(uat-003): verify suggestion flows to mr new with pre-filled context
- Prd(PRD-0015)uat(uat-004): Verify suggestions include strategic and quick-win categories
- Prd(PRD-0015)uat(uat-005): Verify codebase analysis covers tech debt and dependencies
- Prd(PRD-0015)finalize: suggest command for AI-generated PRD recommendations
- Prd(PRD-0002)feat(T-001): Verify code coverage infrastructure complete
- Prd(PRD-0002)feat(T-002): Add WASM build support to complete cross-platform CI coverage

- Added build-wasm task to Makefile.toml for wasm32-wasip2 target
- Added build_wasm CI job to build.yml with artifact upload
- All four target platforms now supported: Linux x86_64, macOS ARM, Windows x86_64, WASM32-WASIP2
- All build jobs conditional on main branch
- Verified UAT-002 (Linux builds) and UAT-005 (WASM builds) pass
- Updated PRD status: T-002 marked as done, UAT-005 marked as verified
- Prd(PRD-0002)feat(T-003): Verify and document cargo-make build tasks for all target platforms
- Prd(PRD-0002)feat(T-004): Set up cargo-release for version management

- Added install-cargo-release task following kord pattern
- Updated release task to depend on install-cargo-release
- Changed from script-based to command-based implementation
- Verified UAT-007: cargo release dry-run works successfully
- All 304 tests pass
- Prd(PRD-0002)feat(T-005): add changelog generation with git-cliff

- Created cliff.toml with conventional commit support and Keep a Changelog format
- Added install-git-cliff task to Makefile.toml
- Added changelog task to Makefile.toml for generating CHANGELOG.md
- Verified UAT-006: cargo make changelog generates proper changelog output
- All 304 tests pass
- Prd(PRD-0002)feat(T-006): add publish-crates task for crates.io publishing
- Prd(PRD-0002)feat(T-007): Add GitHub Release creation task

- Added github-release cargo-make task that uses gh CLI
- Task accepts version tag and optional --draft flag
- Uses CHANGELOG.md for release notes
- Supports attaching binaries from release-artifacts/ directory
- Includes helpful instructions for downloading CI artifacts
- All 304 tests pass
- Prd(PRD-0002)feat(T-008): Add unified release task orchestrating full pipeline

- Renamed original 'release' task to 'release-bump' to preserve version bumping
- Created 'release-builds' helper task for sequential changelog and builds
- Created unified 'release' task that runs CI + builds + provides next-steps
- Created 'publish-all' task for one-command crates.io + GitHub release
- Updated AGENTS.md with comprehensive release workflow documentation
- All tasks follow cargo-make patterns and support argument passing
- Tested orchestration flow, error handling, and version bump validation
- Prd(PRD-0002)feat(T-009): Add pre-built binary installation instructions to README
- Prd(PRD-0002)feat(T-010): Add exhaustive user flow documentation to README

- Added comprehensive 'User Flows' section with 10+ workflow scenarios
- Included quick reference command cheat sheet and real-world scenarios
- Added tips and tricks section with 10 actionable best practices
- Maintained light and funny tone consistent with existing README
- All examples are runnable and reference actual commands
- ~350 lines of exhaustive documentation covering complete user experience
- Prd(PRD-0002)finalize: complete release infrastructure with multi-platform builds
- Prd(PRD-0014)feat(T-001): Extract Q/A workflow patterns to shared module

- Created src/qa_workflow.rs with shared utilities for PRD operations
- Extracted QaPair, parse_questions, extract_prd_content, strip_ansi_escapes
- Unified duplicate code across prd_new.rs and prd_edit.rs
- Added multiline and singleline answer collection variants
- All 312 tests pass
- Prd(PRD-0014)feat(T-002): reduce unnecessary clones in run.rs by 37.5%
- Prd(PRD-0014)feat(T-003): Add inline comments to complex parsing logic

- Enhanced split_frontmatter() in prd/parser.rs with detailed comments
- Enhanced expand_simple_placeholders() with character parsing explanations
- Enhanced expand_if_blocks() with block processing logic comments
- Enhanced expand_each_blocks() with iteration and expansion details
- All 312 tests pass after documentation improvements
- Prd(PRD-0014)feat(T-004): Add comprehensive state machine comments to orchestration flows

- Enhanced run.rs UAT verification loop with 9-state machine documentation
- Enhanced prd_new.rs multi-round Q/A flow with 11-state machine documentation
- Documented critical design decisions, exit conditions, and state transitions
- All 312 tests passing
- Prd(PRD-0014)feat(T-005): Replace string concatenation in loops with iterator-based collection
- Prd(PRD-0014)feat(T-006): Optimize string allocations and reduce unnecessary clones

- Runner optimization: Remove redundant .to_string() after String::from_utf8_lossy and eliminate unnecessary .clone() calls
- Status report optimization: Replace format! + push_str pattern with direct push_str calls
- Index optimization: Use .into_owned() instead of .to_string() to avoid double conversion
- All 312 tests pass
- Prd(PRD-0014)finalize: Cleanup Pass - Refactoring and Documentation
- Prd(PRD-0017)feat(T-001): Add restore subcommand to CLI enum and parser

- Added Restore variant to Command enum with display_order = 3
- Updated subsequent command display_order values for correct ordering
- Added cmd_restore() function with initialization check
- Command appears in --help output in [0] Initialization category
- UAT passed: all existing tests continue to pass
- Prd(PRD-0017)feat(T-002): Implement directory deletion for restore command
- Prd(PRD-0017)feat(T-003): Refactor init logic to support prompts/templates reinitialization

- Created init_prompts_and_templates() function for reusable file-writing logic
- Added create_file_always() helper that overwrites existing files
- Updated cmd_restore() to call init_prompts_and_templates() after deletion
- Maintains DRY principle by sharing file-writing logic between init and restore
- All 312 UAT tests passed
- Prd(PRD-0017)feat(T-004): Add restore command documentation to README and AGENTS.md
- Prd(PRD-0017)feat(T-005): Add integration tests for restore command

- Added 4 comprehensive integration tests for cmd_restore:
  - test_restore_fresh: Fresh restore on initialized repository
  - test_restore_after_customization: Verify customized files are overwritten
  - test_restore_idempotency: Multiple restores produce identical results
  - test_restore_fails_if_not_initialized: Proper error handling
- All tests use tempfile::TempDir for isolated test environments
- All 316 tests passing (4 new + 312 existing)
- Updated PRD-0017 status: T-005 marked as done (all tasks complete: 5/5)
- Regenerated .mr/PRDS.md index
- *PRD-0017*: Verify all UATs for restore command
- Prd(PRD-0017)finalize: Add Restore Command to Reset Prompts and Templates

### 📚 Documentation

- Add release workflow section to AGENTS.md

### ⚙️ Miscellaneous Tasks

- *prd*: Add T-018 for config.toml and --model flag support
- *prd*: Add T-019 for streaming runner output during mr run
- Stylize as lowercase microralph everywhere

<!-- generated by git-cliff -->
