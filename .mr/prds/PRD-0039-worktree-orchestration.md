---
id: PRD-0039
title: "Worktree Orchestration: Daemon-Driven Parallel PRD Execution"
status: active
owner: twitchax
created: 2026-03-04
updated: 2026-03-04
depends_on: []
principles:
  - "File-state on main is the single source of truth, queryable by a future Web UI"
  - "Non-agent operations (branch, worktree, list) are pure git — no LLM cost"
  - "Agent involvement only for conflict resolution, merge ordering, and state summarization"
  - "Daemon-ready from day one — heartbeat drives automation"
  - "Backward compatible — mr run without a daemon works identically to today"
  - "Worktrees are always tied to a PRD"
references:
  - name: "git-worktree documentation"
    url: "https://git-scm.com/docs/git-worktree"
  - name: "Unix domain sockets in Rust"
    url: "https://doc.rust-lang.org/std/os/unix/net/struct.UnixListener.html"
  - name: "flock advisory locking"
    url: "https://man7.org/linux/man-pages/man2/flock.2.html"
acceptance_tests:
  - id: uat-001
    name: "mr wt run creates worktree, branch, and starts mr run in worktree"
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: "mr wt list shows all registered worktrees with status"
    command: cargo make uat
    uat_status: unverified
  - id: uat-003
    name: "mr wt status shows detailed state of a specific worktree"
    command: cargo make uat
    uat_status: unverified
  - id: uat-004
    name: "Daemon auto-starts on first mr wt run and listens on unix socket"
    command: cargo make uat
    uat_status: unverified
  - id: uat-005
    name: "Daemon heartbeat updates state.yaml with worktree liveness and overlap warnings"
    command: cargo make uat
    uat_status: unverified
  - id: uat-006
    name: "mr run in a worktree detects daemon and sends lifecycle events via IPC"
    command: cargo make uat
    uat_status: unverified
  - id: uat-007
    name: "Daemon auto-merges completed worktree into main with UAT gating"
    command: cargo make uat
    uat_status: unverified
  - id: uat-008
    name: "Agent resolves merge conflicts when auto-merge fails"
    command: cargo make uat
    uat_status: unverified
  - id: uat-009
    name: "mr wt graph shows worktree overlap risk visualization"
    command: cargo make uat
    uat_status: unverified
  - id: uat-010
    name: "mr wt merge manually triggers merge of a specific worktree into target"
    command: cargo make uat
    uat_status: unverified
  - id: uat-011
    name: "Daemon recovers gracefully from crash, detecting partial merge state"
    command: cargo make uat
    uat_status: unverified
  - id: uat-012
    name: "Daemon auto-exits after 3 hours with no active worktrees"
    command: cargo make uat
    uat_status: unverified
  - id: uat-013
    name: "mr wt remove cleans up worktree, branch, and state entry"
    command: cargo make uat
    uat_status: unverified
  - id: uat-014
    name: "State commits to main only on significant events with agent-generated summaries"
    command: cargo make uat
    uat_status: unverified
  - id: uat-015
    name: "Agent strategically decides merge order when multiple worktrees complete"
    command: cargo make uat
    uat_status: unverified
tasks:
  - id: T-001
    title: "Define worktree state schema and types"
    priority: 1
    status: done
    notes: "Create src/worktree/types.rs with WorktreeState, WorktreeEntry, WorktreeEvent, OverlapWarning, DaemonConfig structs. YAML-serializable. Version field for future schema migration."
  - id: T-002
    title: "Implement state file read/write with advisory locking"
    priority: 1
    status: done
    notes: "Create src/worktree/state.rs. Read/write .mr/worktrees/state.yaml on main worktree. Use flock-based advisory locking via .mr/worktrees/state.lock. Atomic read-modify-write cycle."
  - id: T-003
    title: "Implement worktree path resolution and git helpers"
    priority: 1
    status: done
    notes: "Create src/worktree/git.rs. Resolve main worktree path via git rev-parse --git-common-dir. Create/remove worktrees. Compute modified files via git diff --name-only. Sibling directory convention: ../<repo>-prd-<id>/."
  - id: T-004
    title: "Implement IPC protocol over Unix domain socket"
    priority: 2
    status: done
    notes: "Create src/worktree/ipc.rs. JSON-over-Unix-socket protocol. Message types: run_started, run_completed, run_failed, task_started, task_completed, heartbeat_request. Daemon listens on .mr/worktrees/daemon.sock. Async-compatible."
  - id: T-005
    title: "Implement daemon core with heartbeat loop"
    priority: 2
    status: done
    notes: "Create src/worktree/daemon.rs. Two-tier heartbeat: Tier 1 (every 30s, mechanical) polls worktree liveness via kill -0, updates modified_files, recomputes overlap. Tier 2 (on significant events) uses agent for merge decisions and state commits. PID file at .mr/worktrees/daemon.pid. Auto-exit after 3h idle."
  - id: T-006
    title: "Implement mr wt run subcommand"
    priority: 2
    status: todo
    notes: "Create src/commands/worktree.rs, add wt subcommand to main.rs. mr wt run <prd-id> creates branch <repo>-prd-<id>, creates sibling worktree, auto-starts daemon if not running, registers worktree in state, spawns detached mr run <prd-id> in worktree context."
  - id: T-007
    title: "Implement daemon auto-start logic"
    priority: 2
    status: todo
    notes: "On mr wt run, check for daemon.pid and socket liveness. If not running, fork/spawn daemon process (detached). Wait for socket to become available before proceeding. Implement in src/worktree/daemon.rs."
  - id: T-008
    title: "Integrate mr run with daemon IPC (worktree detection)"
    priority: 3
    status: todo
    notes: "Modify src/commands/run.rs. On startup, check git rev-parse --git-common-dir vs --git-dir. If in worktree and daemon socket exists, connect and send lifecycle events (run_started, task_started, task_completed, run_completed, run_failed). Backward compatible — no daemon means no IPC, run works normally."
  - id: T-009
    title: "Implement mr wt list subcommand"
    priority: 3
    status: todo
    notes: "Read state.yaml, display all registered worktrees in a table: PRD ID, branch, status, modified files count, last event timestamp. Color-code by status."
  - id: T-010
    title: "Implement mr wt status subcommand"
    priority: 3
    status: todo
    notes: "Detailed view of a single worktree or overall daemon status. Show full event history, modified files, overlap warnings, merge readiness. Include daemon uptime, heartbeat count, active worktree count."
  - id: T-011
    title: "Implement auto-merge in daemon heartbeat"
    priority: 4
    status: todo
    notes: "When Tier 1 heartbeat detects a completed worktree, trigger Tier 2 agent evaluation. Agent decides merge order strategically (considering overlap risk, PRD dependencies, completion order). Attempt rebase first, fallback to merge. Run UATs after merge — if pass, commit; if fail, mark merge_failed."
  - id: T-012
    title: "Implement agent-driven conflict resolution"
    priority: 4
    status: todo
    notes: "When merge/rebase produces conflicts, spawn agent session with conflict context (conflicting files, both sides, PRD context). Agent resolves conflicts, stages changes. Run UATs to verify. Use existing Runner trait for agent invocation."
  - id: T-013
    title: "Implement mr wt merge subcommand"
    priority: 4
    status: todo
    notes: "Manual merge trigger: mr wt merge <prd-id> [--into <target>]. Default target is main. Attempts rebase then merge. Runs UATs. If conflicts, spawns agent. Updates state.yaml. Can also merge between worktrees (e.g., merge PRD-39 into PRD-40's branch)."
  - id: T-014
    title: "Implement agent-driven state commits"
    priority: 5
    status: todo
    notes: "On significant events (merge completed, merge failed), agent generates a summary commit message and commits state.yaml to main. Format: 'mr-wt: PRD-0039 merged, PRD-0040 in progress (3 active worktrees)'. Only on big events, not every heartbeat."
  - id: T-015
    title: "Implement mr wt graph subcommand"
    priority: 5
    status: todo
    notes: "Visualize worktree overlap risk. Nodes = active worktrees, edges = shared modified files. Reuse existing graph infrastructure (ASCII, Mermaid, DOT). Color-code by risk level: green (no overlap), yellow (some), red (heavy). Show file list on edges."
  - id: T-016
    title: "Implement mr wt remove subcommand"
    priority: 5
    status: todo
    notes: "mr wt remove <prd-id> removes the git worktree, optionally deletes the branch (--delete-branch), updates state.yaml to mark as abandoned/removed. Refuse to remove if status is merging (safety check)."
  - id: T-017
    title: "Implement daemon crash recovery"
    priority: 6
    status: todo
    notes: "On daemon startup, check for stale state: partial merges (status=merging but no merge in progress), orphaned worktrees, stale PID file. Use agent to assess situation and recover — e.g., rollback partial merge, re-register orphaned worktrees."
  - id: T-018
    title: "Add worktree module to commands/mod.rs and wire CLI"
    priority: 1
    status: done
    notes: "Register wt subcommand in main.rs Command enum. Subcommands: run, list, status, merge, graph, remove, daemon (start/stop/status). Follow existing CLI patterns from GraphCommand/DevcontainerCommand."
  - id: T-019
    title: "Update AGENTS.md with worktree orchestration workflow"
    priority: 6
    status: todo
    notes: "Document the full wt workflow, daemon lifecycle, IPC protocol, state file schema, and troubleshooting in AGENTS.md for future agent reference."
---

# Summary

Worktree Orchestration adds a daemon-driven parallel execution layer to microralph. The `mr wt` command group manages git worktrees tied to PRDs, enabling multiple PRDs to be worked on simultaneously in isolated worktrees. A lightweight daemon on the main branch coordinates state, detects completions, auto-merges results, and uses agents for conflict resolution and strategic merge ordering. All state is tracked in a YAML file on main, designed to be queryable by a future Sentry-esque Web UI.

# Problem

Today, microralph executes PRDs sequentially — one `mr run` at a time on a single branch. For projects with many independent PRDs, this is a bottleneck. Git worktrees provide isolated working directories that share the same repository, enabling true parallel execution. However, coordinating multiple concurrent agent sessions — tracking their state, merging their results, detecting conflicts, and maintaining a coherent project state — requires orchestration infrastructure that doesn't exist today.

Additionally, the long-term vision includes a Web UI that visualizes project state (active worktrees, merge status, conflict risk). This requires a well-structured, queryable state format maintained on the main branch.

# Goals

1. Enable parallel PRD execution via git worktrees, each tied to a specific PRD.
2. Provide a daemon that automates worktree lifecycle management (heartbeat, auto-merge, conflict resolution).
3. Maintain all orchestration state in a YAML file on main, suitable for Web UI consumption.
4. Detect and visualize parallelization risk (file overlap between worktrees).
5. Use agents strategically — only for conflict resolution, merge ordering, and state summarization.
6. Keep non-agent operations (create, list, remove) as pure git commands with zero LLM cost.
7. Ensure backward compatibility — `mr run` without a daemon works identically to today.

# Technical Approach

## Architecture Overview

```
┌──────────────────────────────────────────────────────────┐
│                      main branch                          │
│                                                           │
│  .mr/worktrees/state.yaml    ← YAML state (source of     │
│  .mr/worktrees/daemon.sock   ← Unix socket IPC  truth)   │
│  .mr/worktrees/daemon.pid    ← Daemon lifecycle           │
│  .mr/worktrees/state.lock    ← Advisory file lock         │
│                                                           │
│  ┌────────────────────────────┐                           │
│  │     mr wt daemon           │                           │
│  │     (supervisor process)   │                           │
│  │                            │                           │
│  │  Tier 1 heartbeat (30s)    │  ← mechanical: liveness,  │
│  │  - poll worktree PIDs      │     modified files,        │
│  │  - update modified_files   │     overlap detection      │
│  │  - recompute overlap       │                           │
│  │                            │                           │
│  │  Tier 2 heartbeat (event)  │  ← agent-driven: merge    │
│  │  - strategic merge order   │     decisions, conflict    │
│  │  - auto-merge + UAT gate   │     resolution, state      │
│  │  - conflict resolution     │     commits                │
│  │  - state commit to main    │                           │
│  └─────────────┬──────────────┘                           │
│                │ unix socket                              │
└────────────────┼──────────────────────────────────────────┘
        ┌────────┼────────┬────────────────┐
        │        │        │                │
   ┌────┴────┐ ┌─┴──────┐ ┌┴───────────┐   │
   │ ../repo │ │ ../repo│ │ ../repo    │   │
   │ -prd-39/│ │ -prd-40│ │ -prd-41/   │   │
   │         │ │ /      │ │            │   │
   │ mr run  │ │ mr run │ │ mr run     │   │
   │ PRD-039 │ │ PRD-040│ │ PRD-041    │   │
   │ (sends  │ │ (sends │ │ (sends     │   │
   │  IPC)   │ │  IPC)  │ │  IPC)      │   │
   └─────────┘ └────────┘ └────────────┘   │
                                            │
```

## State File Schema (`.mr/worktrees/state.yaml`)

```yaml
version: 1
daemon:
  pid: 12345
  started_at: "2026-03-04T22:00:00Z"
  idle_timeout_hours: 3
  last_heartbeat: "2026-03-04T22:30:00Z"
worktrees:
  - id: wt-001
    prd: PRD-0039
    branch: microralph-prd-39
    path: "/home/user/projects/microralph-prd-39"
    status: active        # active | completed | merging | merged | merge_failed | conflicted | abandoned
    run_pid: 54321        # PID of the mr run process
    created_at: "2026-03-04T22:00:00Z"
    updated_at: "2026-03-04T22:30:00Z"
    merge_target: main    # default target branch
    modified_files:
      - src/commands/worktree.rs
      - src/worktree/daemon.rs
    events:
      - timestamp: "2026-03-04T22:00:00Z"
        type: created
      - timestamp: "2026-03-04T22:01:00Z"
        type: run_started
        detail: "T-001"
      - timestamp: "2026-03-04T22:15:00Z"
        type: task_completed
        detail: "T-001"
      - timestamp: "2026-03-04T22:30:00Z"
        type: run_completed
overlap_warnings:
  - worktrees: ["wt-001", "wt-003"]
    files: ["src/main.rs", "src/commands/mod.rs"]
    risk: high            # low | medium | high
```

## IPC Protocol (JSON over Unix socket)

Messages from worktree → daemon:
```json
{"type": "run_started", "prd": "PRD-0039", "wt_id": "wt-001", "pid": 54321}
{"type": "task_started", "prd": "PRD-0039", "wt_id": "wt-001", "task": "T-001"}
{"type": "task_completed", "prd": "PRD-0039", "wt_id": "wt-001", "task": "T-001"}
{"type": "run_completed", "prd": "PRD-0039", "wt_id": "wt-001"}
{"type": "run_failed", "prd": "PRD-0039", "wt_id": "wt-001", "error": "UAT failed"}
```

Daemon responds with ack: `{"status": "ok"}` or `{"status": "error", "message": "..."}`.

## Command Flow: `mr wt run <prd-id>`

1. Resolve main worktree path via `git worktree list`
2. Check daemon: read `daemon.pid`, test socket connectivity
3. If daemon not running → spawn detached daemon process, wait for socket
4. Create branch `<repo-name>-prd-<id>` from current main HEAD
5. Create worktree at `../<repo-name>-prd-<id>/`
6. Register worktree in `state.yaml` (via flock + atomic write)
7. Spawn detached `mr run <prd-id>` in the worktree directory
8. Return immediately — user sees "Worktree created, PRD-0039 running in background"

## Daemon Heartbeat Logic

**Tier 1 (every 30 seconds, no agent):**
- For each registered worktree with status `active`:
  - Check `run_pid` liveness via `kill -0`
  - If dead and status still `active` → mark `completed` (or `failed` based on exit code)
  - Run `git diff --name-only main...<branch>` to update `modified_files`
- Recompute `overlap_warnings` across all active worktrees
- Write updated state (flock-protected)

**Tier 2 (triggered by significant events):**
- **Completion detected**: Agent evaluates all completed worktrees, decides strategic merge order considering overlap risk, PRD `depends_on`, and modified file count
- **Merge attempt**: Rebase first (cleaner history), fallback to merge. Run `cargo make uat` after — pass → commit and mark `merged`; fail → mark `merge_failed`
- **Conflict detected**: Spawn agent with conflict context to resolve, then re-run UATs
- **State commit**: Agent generates summary message, commits `state.yaml` to main. Only on merges (successful or failed), not every heartbeat

## Merge Strategy

1. Attempt `git rebase main` on the worktree branch (fastest path to clean history)
2. If rebase fails with conflicts → attempt `git merge main` instead
3. If merge also conflicts → mark `conflicted`, spawn agent for resolution
4. After successful rebase/merge → run `cargo make uat` in the worktree
5. If UATs pass → fast-forward merge into main (or merge commit), update state
6. If UATs fail → mark `merge_failed`, agent investigates

## Daemon Crash Recovery

On daemon startup, scan for inconsistent state:
- `status: merging` with no active merge process → agent assesses: rollback or complete
- Stale `daemon.pid` pointing to dead process → clean up, take over
- Worktrees registered in state but not in `git worktree list` → mark `abandoned`
- Worktrees in `git worktree list` but not in state → re-register

## Module Structure

```
src/
  worktree/
    mod.rs          ← module root, re-exports
    types.rs        ← WorktreeState, WorktreeEntry, Event, OverlapWarning, etc.
    state.rs        ← state file read/write with flock
    git.rs          ← git worktree/branch helpers, modified file detection
    ipc.rs          ← Unix socket server (daemon) and client (mr run)
    daemon.rs       ← heartbeat loop, auto-merge, event handling
  commands/
    worktree.rs     ← CLI subcommand handlers (run, list, status, merge, graph, remove, daemon)
```

# Assumptions

- Git is available and supports `git worktree` (Git 2.5+).
- Unix domain sockets are available (Linux/macOS; Windows support deferred).
- The main worktree (original repo checkout) is always present and on the `main` branch.
- `cargo make uat` is the universal verification gate for merge safety.
- Existing `Runner` trait and agent infrastructure can be reused for conflict resolution.

# Constraints

- **Unix-only for MVP**: Unix domain sockets and `flock` are POSIX. Windows support would need named pipes and different locking — defer to a follow-up PRD.
- **No Web UI**: This PRD builds the state and orchestration layer only. The Web UI is a separate follow-up that consumes `state.yaml`.
- **Single repo**: Worktrees are within one git repository. Multi-repo orchestration is out of scope.
- **Agent cost**: Tier 2 heartbeat invokes agents. Ensure cost is bounded by only triggering on significant events (not every 30s heartbeat).

# References to Code

- `src/main.rs` — CLI entry point; add `Wt` subcommand enum variant following `Graph`/`Devcontainer` pattern
- `src/commands/mod.rs` — register new `worktree` module
- `src/commands/run.rs` — modify to detect worktree context and send IPC events to daemon
- `src/commands/graph.rs` — reuse graph rendering infrastructure for `mr wt graph`
- `src/runner/types.rs` — `Runner` trait for agent-driven conflict resolution
- `src/prd/types.rs` — PRD types for worktree-PRD association
- `src/util/colors.rs` — color utilities for status display
- `Cargo.toml` — may need `serde_json` (IPC), `tokio` or async runtime (daemon), `nix` (Unix signals/process)

# Non-Goals (MVP)

- **Web UI**: This PRD builds the queryable state layer; the UI is a follow-up.
- **Windows support**: Unix sockets and flock are POSIX-only; Windows deferred.
- **Multi-repo orchestration**: Only single-repo worktrees.
- **Semantic dependency analysis**: File-level overlap only; no call-graph analysis.
- **Persistent daemon (systemd/launchd)**: Daemon is user-space, started by `mr wt run`, exits on idle. No system service integration.
- **Worktree-to-worktree streaming**: Worktrees don't communicate directly; all coordination goes through the daemon.

# History

## 2026-03-04 — T-001 Completed
- **Task**: Define worktree state schema and types
- **Status**: ✅ Done
- **Changes**:
  - Created `src/worktree/mod.rs` — module root with `pub mod types` re-export
  - Created `src/worktree/types.rs` — all worktree orchestration types:
    - `WorktreeState` (top-level, version=1 for migration), `DaemonInfo`, `DaemonConfig`
    - `WorktreeEntry`, `WorktreeStatus` enum (7 variants), `WorktreeEvent`, `EventType` enum (6 variants)
    - `OverlapWarning`, `OverlapRisk` enum (low/medium/high)
    - `IpcMessage` (tagged enum, JSON-serializable), `IpcResponse` with `ok()`/`error()` helpers
  - Registered `mod worktree` in `src/main.rs`
  - 9 unit tests: YAML roundtrip, IPC JSON serialization, Display impls, defaults, minimal YAML deserialization
  - UAT: `cargo make uat` — 593 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-04 — T-002 Completed
- **Task**: Implement state file read/write with advisory locking
- **Status**: ✅ Done
- **Changes**:
  - Created `src/worktree/state.rs` — `StateManager` struct with full state file lifecycle:
    - `read()` / `write()` for unlocked access (returns default when file missing)
    - `lock_exclusive()` / `try_lock_exclusive()` using `libc::flock` advisory locking
    - `read_locked()` for concurrent-safe reads
    - `modify()` for atomic read-modify-write under flock
    - `try_modify()` for non-blocking variant
    - Atomic writes via temp file + rename
  - Added `libc` as direct dependency in `Cargo.toml` (already transitive)
  - Updated `src/worktree/mod.rs` to export `pub mod state`
  - 11 unit tests: roundtrip, atomic write, modify with closure, error propagation, lock creation, path resolution, sequential modifies
  - UAT: `cargo make uat` — 604 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-04 — T-003 Completed
- **Task**: Implement worktree path resolution and git helpers
- **Status**: ✅ Done
- **Changes**:
  - Created `src/worktree/git.rs` — git helper functions for worktree orchestration:
    - `git_output()` / `git_run()` — internal helpers for running git commands with error handling
    - `resolve_main_worktree()` — resolves main worktree root via `git rev-parse --git-common-dir`
    - `repo_name()` — extracts repository name from path
    - `worktree_branch_name()` — derives branch name following `<repo>-prd-<numeric-id>` convention
    - `worktree_path()` — derives sibling directory path `../<repo>-prd-<id>/`
    - `create_branch()` / `delete_branch()` — branch management (idempotent create)
    - `create_worktree()` / `remove_worktree()` — git worktree lifecycle
    - `list_worktrees()` — parses `git worktree list --porcelain` output
    - `modified_files()` — computes changed files via `git diff --name-only` (three-dot merge-base)
    - `is_linked_worktree()` — detects if cwd is inside a linked worktree
    - `current_branch()` — gets current branch name
  - Updated `src/worktree/mod.rs` to export `pub mod git`
  - 17 unit tests covering: path resolution from main and linked worktrees, branch/worktree create/remove, modified file detection, linked worktree detection, edge cases
  - UAT: `cargo make uat` — 621 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-04 — T-018 Completed
- **Task**: Add worktree module to commands/mod.rs and wire CLI
- **Status**: ✅ Done
- **Changes**:
  - Created `src/commands/worktree.rs` — stub command handlers for all `wt` subcommands:
    - `cmd_wt_run`, `cmd_wt_list`, `cmd_wt_status`, `cmd_wt_merge`, `cmd_wt_graph`, `cmd_wt_remove`
    - `cmd_wt_daemon_start`, `cmd_wt_daemon_stop`, `cmd_wt_daemon_status`
    - 10 unit tests verifying all stubs return not-implemented errors
  - Registered `pub mod worktree` in `src/commands/mod.rs`
  - Added `WtCommand` enum (7 variants: Run, List, Status, Merge, Graph, Remove, Daemon) in `src/main.rs`
  - Added `DaemonCommand` enum (3 variants: Start, Stop, Status) in `src/main.rs`
  - Added `Wt` variant to `Command` enum with `display_order = 16`
  - Wired full dispatch in `match args.command` with `normalize_prd_id` for all PRD ID arguments
  - Used `commands::worktree::` qualified paths to avoid name collision with `mod worktree` (src/worktree/)
  - UAT: `cargo make uat` — 631 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-04 — T-004 Completed
- **Task**: Implement IPC protocol over Unix domain socket
- **Status**: ✅ Done
- **Changes**:
  - Created `src/worktree/ipc.rs` — full IPC protocol implementation:
    - `socket_path()` — resolves daemon socket path (`<root>/.mr/worktrees/daemon.sock`)
    - `is_daemon_reachable()` — checks whether the daemon socket is connectable
    - `IpcClient` — connects to daemon socket, sends `IpcMessage`, receives `IpcResponse` via newline-delimited JSON
    - `IpcServer` — listens on Unix domain socket, accepts connections, dispatches messages to `FnMut` handler
    - Stale socket cleanup on bind, RAII socket cleanup on drop
    - Non-blocking mode support via `set_nonblocking()`
  - Updated `src/worktree/mod.rs` to export `pub mod ipc`
  - 10 unit tests: socket path resolution, bind/drop lifecycle, stale socket handling, client-server roundtrip, error responses, all message types, daemon reachability, non-blocking accept
  - UAT: `cargo make uat` — 641 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-04 — T-005 Completed
- **Task**: Implement daemon core with heartbeat loop
- **Status**: ✅ Done
- **Changes**:
  - Created `src/worktree/daemon.rs` — full daemon core implementation:
    - `Daemon` struct with configurable `DaemonConfig` and per-instance `shutdown` handle
    - PID file management: `pid_path()`, `write_pid_file()`, `remove_pid_file()`, `read_pid()`
    - Process liveness: `is_process_alive()` via `kill -0`, `is_running()`, `stop()` via SIGTERM
    - State management: `register_daemon()` / `unregister_daemon()` in `state.yaml`
    - Main event loop: non-blocking IPC accept, periodic Tier 1 heartbeat, idle timeout auto-exit
    - Signal handling: `SIGTERM`/`SIGINT` via global `AtomicBool` + per-instance shutdown handle for testing
    - IPC message processing: `HeartbeatRequest`, `RunStarted`, `TaskStarted`, `TaskCompleted`, `RunCompleted`, `RunFailed`
    - Tier 1 heartbeat: polls worktree PID liveness, updates modified files via `git diff`, recomputes overlap warnings
    - Overlap computation: `compute_overlaps()` classifies risk as low (≤2 files), medium (3–5), high (6+)
  - Updated `src/worktree/ipc.rs`:
    - Added `try_accept_stream()` to `IpcServer` for non-blocking accept
    - Added public `handle_stream()` function with timeout/WouldBlock support
  - Updated `src/worktree/mod.rs` to export `pub mod daemon`
  - Updated `src/commands/worktree.rs`:
    - Implemented `cmd_wt_daemon_start()` — runs daemon in foreground, checks for existing instance
    - Implemented `cmd_wt_daemon_stop()` — sends SIGTERM to running daemon
    - Implemented `cmd_wt_daemon_status()` — shows daemon running state, PID, worktree count
  - 21 new tests (total 662): PID file lifecycle, process liveness, overlap computation (5 cases), IPC message processing (5 cases), daemon lifecycle (4 integration tests including IPC heartbeat roundtrip)
  - UAT: `cargo make uat` — 662 tests passed, 0 skipped
- **Constitution Compliance**: No violations.
