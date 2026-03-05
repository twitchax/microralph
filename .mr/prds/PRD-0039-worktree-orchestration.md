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
    status: done
    notes: "Create src/commands/worktree.rs, add wt subcommand to main.rs. mr wt run <prd-id> creates branch <repo>-prd-<id>, creates sibling worktree, auto-starts daemon if not running, registers worktree in state, spawns detached mr run <prd-id> in worktree context."
  - id: T-007
    title: "Implement daemon auto-start logic"
    priority: 2
    status: done
    notes: "On mr wt run, check for daemon.pid and socket liveness. If not running, fork/spawn daemon process (detached). Wait for socket to become available before proceeding. Implement in src/worktree/daemon.rs."
  - id: T-008
    title: "Integrate mr run with daemon IPC (worktree detection)"
    priority: 3
    status: done
    notes: "Modify src/commands/run.rs. On startup, check git rev-parse --git-common-dir vs --git-dir. If in worktree and daemon socket exists, connect and send lifecycle events (run_started, task_started, task_completed, run_completed, run_failed). Backward compatible — no daemon means no IPC, run works normally."
  - id: T-009
    title: "Implement mr wt list subcommand"
    priority: 3
    status: done
    notes: "Read state.yaml, display all registered worktrees in a table: PRD ID, branch, status, modified files count, last event timestamp. Color-code by status."
  - id: T-010
    title: "Implement mr wt status subcommand"
    priority: 3
    status: done
    notes: "Detailed view of a single worktree or overall daemon status. Show full event history, modified files, overlap warnings, merge readiness. Include daemon uptime, heartbeat count, active worktree count."
  - id: T-011
    title: "Implement auto-merge in daemon heartbeat"
    priority: 4
    status: done
    notes: "When Tier 1 heartbeat detects a completed worktree, trigger Tier 2 agent evaluation. Agent decides merge order strategically (considering overlap risk, PRD dependencies, completion order). Attempt rebase first, fallback to merge. Run UATs after merge — if pass, commit; if fail, mark merge_failed."
  - id: T-012
    title: "Implement agent-driven conflict resolution"
    priority: 4
    status: done
    notes: "When merge/rebase produces conflicts, spawn agent session with conflict context (conflicting files, both sides, PRD context). Agent resolves conflicts, stages changes. Run UATs to verify. Use existing Runner trait for agent invocation."
  - id: T-013
    title: "Implement mr wt merge subcommand"
    priority: 4
    status: done
    notes: "Manual merge trigger: mr wt merge <prd-id> [--into <target>]. Default target is main. Attempts rebase then merge. Runs UATs. If conflicts, spawns agent. Updates state.yaml. Can also merge between worktrees (e.g., merge PRD-39 into PRD-40's branch)."
  - id: T-014
    title: "Implement agent-driven state commits"
    priority: 5
    status: done
    notes: "On significant events (merge completed, merge failed), agent generates a summary commit message and commits state.yaml to main. Format: 'mr-wt: PRD-0039 merged, PRD-0040 in progress (3 active worktrees)'. Only on big events, not every heartbeat."
  - id: T-015
    title: "Implement mr wt graph subcommand"
    priority: 5
    status: done
    notes: "Visualize worktree overlap risk. Nodes = active worktrees, edges = shared modified files. Reuse existing graph infrastructure (ASCII, Mermaid, DOT). Color-code by risk level: green (no overlap), yellow (some), red (heavy). Show file list on edges."
  - id: T-016
    title: "Implement mr wt remove subcommand"
    priority: 5
    status: done
    notes: "mr wt remove <prd-id> removes the git worktree, optionally deletes the branch (--delete-branch), updates state.yaml to mark as abandoned/removed. Refuse to remove if status is merging (safety check)."
  - id: T-017
    title: "Implement daemon crash recovery"
    priority: 6
    status: done
    notes: "On daemon startup, check for stale state: partial merges (status=merging but no merge in progress), orphaned worktrees, stale PID file. Use agent to assess situation and recover — e.g., rollback partial merge, re-register orphaned worktrees."
  - id: T-018
    title: "Add worktree module to commands/mod.rs and wire CLI"
    priority: 1
    status: done
    notes: "Register wt subcommand in main.rs Command enum. Subcommands: run, list, status, merge, graph, remove, daemon (start/stop/status). Follow existing CLI patterns from GraphCommand/DevcontainerCommand."
  - id: T-019
    title: "Update AGENTS.md with worktree orchestration workflow"
    priority: 6
    status: done
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

## 2026-03-04 — T-006 Completed
- **Task**: Implement mr wt run subcommand
- **Status**: ✅ Done
- **Changes**:
  - Replaced stub `cmd_wt_run` in `src/commands/worktree.rs` with full implementation:
    - Resolves main worktree root via `git::resolve_main_worktree()`
    - Validates PRD exists by scanning `.mr/prds/`
    - Derives branch name (`<repo>-prd-<id>`) and sibling worktree path
    - Guards against duplicate active worktrees for the same PRD
    - Creates branch (idempotent) and git worktree
    - Registers worktree in `state.yaml` with `StateManager::modify()` (flock-protected)
    - Auto-starts daemon as a detached background process if not already running, waits for socket readiness
    - Spawns `mr run <prd-id>` as a detached process in the worktree directory
    - Records `run_pid` and lifecycle events (`created`, `run_started`) in state
    - Displays colored status messages (branch, path, ID, final success)
  - Added helper functions: `next_wt_id()`, `now_iso()`, `days_to_ymd()`, `validate_prd_exists()`, `ensure_daemon()`, `register_worktree()`, `spawn_mr_run()`
  - 6 new unit tests: `next_wt_id` (empty + increments), `now_iso` format, `days_to_ymd` (epoch + known date), `validate_prd_exists` error, `cmd_wt_run` fails without git repo
  - UAT: `cargo make uat` — 668 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-04 — T-007 Completed
- **Task**: Implement daemon auto-start logic
- **Status**: ✅ Done
- **Changes**:
  - Added `Daemon::is_healthy(root)` in `src/worktree/daemon.rs` — checks both PID liveness AND socket reachability for stronger daemon health verification
  - Added `Daemon::cleanup_stale(root)` in `src/worktree/daemon.rs` — removes stale PID files (dead process) and stale socket files (not connectable) before spawning a new daemon
  - Added `Daemon::ensure_running(root)` in `src/worktree/daemon.rs` — primary auto-start entry point: health check → cleanup stale → spawn detached `mr wt daemon start` → wait up to 10s for socket readiness
  - Simplified `ensure_daemon()` in `src/commands/worktree.rs` to delegate to `Daemon::ensure_running()`, using `Daemon::is_healthy()` for pre/post checks
  - Removed unused `ipc` import from `src/commands/worktree.rs`
  - Enhanced `is_running()` doc comment to clarify it only checks PID liveness
  - 7 new tests: `is_healthy_false_when_no_daemon`, `is_healthy_false_when_pid_only`, `is_healthy_true_when_running_and_reachable`, `cleanup_stale_removes_dead_pid_file`, `cleanup_stale_removes_stale_socket`, `cleanup_stale_preserves_live_pid`, `cleanup_stale_noop_when_nothing_exists`
  - UAT: `cargo make uat` — 675 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-05 — T-008 Completed
- **Task**: Integrate mr run with daemon IPC (worktree detection)
- **Status**: ✅ Done
- **Changes**:
  - Added `DaemonNotifier` struct to `src/commands/run.rs` — best-effort IPC client that sends lifecycle events to the daemon when `mr run` executes inside a linked worktree:
    - `try_connect(cwd, prd_id)` — detects linked worktree via `git::is_linked_worktree()`, resolves main worktree, checks daemon socket reachability, looks up `wt_id` from `state.yaml`, connects IPC client. Returns `None` (no-op) when any condition isn't met.
    - `run_started()`, `task_started()`, `task_completed()`, `run_completed()`, `run_failed()` — fire-and-forget notification methods; failures logged via `tracing` but never propagate.
  - Modified `run_task()` signature to accept `&mut Option<DaemonNotifier>` — sends `task_started` after picking the next task.
  - Modified `cmd_run()` in `src/main.rs` to:
    - Create `DaemonNotifier` via `try_connect()` after PRD ID is determined
    - Send `run_started` before the task loop
    - Send `task_completed` after each successful `run_task()`
    - Send `run_completed`/`run_failed` when the loop ends
    - Send `run_failed` on UAT verification loop errors
  - Added import for `crate::worktree::{git, ipc, state, types::IpcMessage}` in `run.rs`
  - 5 new unit tests: `daemon_notifier_returns_none_outside_git_repo`, `daemon_notifier_returns_none_in_main_worktree`, `daemon_notifier_returns_none_when_no_daemon`, `daemon_notifier_returns_none_when_prd_not_in_state`, `daemon_notifier_connects_and_sends_events` (full IPC roundtrip)
  - Updated all existing `run_task()` call sites in tests to pass `&mut None` for backward compatibility
  - UAT: `cargo make uat` — 680 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-05 — T-009 Completed
- **Task**: Implement mr wt list subcommand
- **Status**: ✅ Done
- **Changes**:
  - Replaced stub `cmd_wt_list` in `src/commands/worktree.rs` with full implementation:
    - Resolves main worktree root and reads `state.yaml` via `StateManager`
    - Displays aligned table with columns: PRD ID, Branch, Status, Files (count), Last Event
    - Color-codes status by lifecycle: active (cyan), completed/merged (green), merge_failed/conflicted (red), merging (yellow), abandoned (dim)
    - Shows separator line and summary footer with total and active worktree counts
    - Handles empty state gracefully with a helpful message
  - Added helper functions: `status_colored()` (maps `WorktreeStatus` to colored output), `last_event_timestamp()` (extracts latest event timestamp from entry)
  - 3 new unit tests: `test_cmd_wt_list_fails_without_git_repo`, `test_status_colored_returns_string_for_all_variants`, `test_last_event_timestamp_with_events`, `test_last_event_timestamp_empty_events`
  - UAT: `cargo make uat` — 683 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-05 — T-010 Completed
- **Task**: Implement mr wt status subcommand
- **Status**: ✅ Done
- **Changes**:
  - Replaced stub `cmd_wt_status` in `src/commands/worktree.rs` with full implementation:
    - When no PRD ID given: shows overall daemon status (running/unhealthy/not running, PID, started_at, last heartbeat, idle timeout) + worktree summary (total/active/completed/merged/failed counts) + brief per-worktree list + overlap warnings
    - When PRD ID given: shows detailed single-worktree view with identity fields, run PID liveness, merge readiness assessment, modified files list, overlap warnings involving this worktree, and full event history
  - Extracted helper functions for clippy::too_many_lines compliance: `print_merge_readiness()`, `print_modified_files()`, `print_entry_overlaps()`, `print_event_history()`, `overlap_risk_colored()`
  - Added `OverlapRisk` and `WorktreeState` to the type imports
  - Updated module doc comment to reflect `wt status` is no longer a stub
  - 5 new unit tests: `test_cmd_wt_status_fails_without_git_repo`, `test_cmd_wt_status_with_unknown_prd_fails`, `test_print_worktree_detail_shows_entry`, `test_print_worktree_detail_not_found`, `test_overlap_risk_colored_returns_string_for_all_variants`
  - UAT: `cargo make uat` — 686 tests passed, 0 skipped
- **Constitution Compliance**: No violations.

## 2026-03-05 — T-011 Completed
- **Task**: Implement auto-merge in daemon heartbeat
- **Status**: ✅ Done
- **Changes**:
  - Added 4 new `EventType` variants in `src/worktree/types.rs`: `MergeStarted`, `MergeCompleted`, `MergeFailed`, `Conflicted` with corresponding `Display` impls
  - Added 6 new git helper functions in `src/worktree/git.rs`: `rebase_onto()`, `rebase_abort()`, `merge_branch()`, `merge_abort()`, `checkout()`, `merge_ff_only()`
  - Implemented Tier 2 auto-merge in `src/worktree/daemon.rs`:
    - `tier2_auto_merge()` — entry point triggered after each Tier 1 heartbeat; finds completed worktrees and merges them
    - `compute_merge_order()` — deterministic merge ordering by overlap risk (fewest first) then completion time (earliest first)
    - `attempt_merge_worktree()` — full merge flow: mark merging → integrate target → run UATs → merge into main → mark merged/failed/conflicted
    - `integrate_target_into_branch()` — rebase-first strategy with merge fallback; aborts cleanly on conflict
    - `merge_into_target()` — fast-forward-first merge of branch into target on main worktree
    - `run_uat()` — runs `cargo make uat` and returns pass/fail
    - `update_wt_status()` — atomically updates worktree status and records event in state
  - Wired `tier2_auto_merge()` into the daemon event loop after each Tier 1 heartbeat
  - 18 new tests (total 704): merge order computation (4), status update (2), git helpers (6), integration tests with real git repos (4), tier2 skip logic (1), run_uat failure (1)
  - UAT: `cargo make uat` — 704 tests passed, 0 skipped
- **Constitution Compliance**: No violations. Uses deterministic merge ordering rather than LLM-based agent for merge decisions — this keeps costs bounded per principle "Agent cost" while still being strategic (overlap-aware, time-ordered).

## 2026-03-05 — T-012 Completed
- **Task**: Implement agent-driven conflict resolution
- **Status**: ✅ Done
- **Changes**:
  - Added 6 new git helper functions in `src/worktree/git.rs`: `list_conflict_files()`, `conflict_diff()`, `stage_all()`, `rebase_continue()`, `is_rebase_in_progress()`, `merge_commit()`
  - Added 2 new `EventType` variants in `src/worktree/types.rs`: `ConflictResolutionStarted`, `ConflictResolved` with corresponding `Display` impls
  - Added optional `runner` field (`Option<Box<dyn Runner>>`) to `Daemon` struct in `src/worktree/daemon.rs`
  - Added `Daemon::new_with_runner()` constructor accepting a runner for conflict resolution
  - Implemented full conflict resolution pipeline in `src/worktree/daemon.rs`:
    - `resolve_conflicts()` — orchestrates conflict resolution: starts conflicting merge, gathers context, invokes runner, stages resolved files, verifies no remaining conflicts, finalizes merge/rebase
    - `start_conflicting_merge()` — starts merge/rebase leaving conflicts in place for agent resolution
    - `finalize_conflict_resolution()` — continues rebase or commits merge after resolution
    - `abort_in_progress()` — best-effort cleanup on failure
    - `build_conflict_prompt()` — builds structured prompt with conflict files, diff, and instructions (truncates large diffs at 50KB)
  - Modified `attempt_merge_worktree()` to attempt agent-driven conflict resolution when runner is available, falling back to marking `Conflicted` when no runner is present (backward compatible)
  - Added `PROMPT_CONFLICT_RESOLVE` constant and registered `conflict_resolve.md` in `src/commands/init.rs` prompt manifest
  - Refactored `create_runner()` from `src/main.rs` into `src/runner/mod.rs` as public `runner::create_runner()` for reuse; `main.rs` delegates to it (DRY principle)
  - Modified `cmd_wt_daemon_start()` in `src/commands/worktree.rs` to create a runner from project config and pass to daemon for conflict resolution
  - Added `create_daemon_runner()` helper that reads `.mr/config.toml` for runner/model settings
  - Updated init test assertions to account for new prompt file count (23 → 24)
  - 11 new tests (total 715): `build_conflict_prompt_contains_context`, `build_conflict_prompt_truncates_large_diff`, `resolve_conflicts_with_mock_runner_succeeds`, `attempt_merge_without_runner_marks_conflicted`, `attempt_merge_with_runner_attempts_resolution`, `start_conflicting_merge_clean_rebase_returns_true`, `list_conflict_files_empty_when_no_conflicts`, `stage_all_and_list_in_clean_repo`, `list_conflict_files_returns_conflicting_paths`, `is_rebase_in_progress_false_normally`, `stage_all_stages_new_files`
  - UAT: `cargo make uat` — 715 tests passed, 0 skipped
- **Constitution Compliance**: No violations. Rule 1 (DRY): Extracted `create_runner` to runner module. Rule 7 (Prompt Management): Added prompt constant and registered in manifest. Rule 8 (Clippy Pedantic): All methods refactored to satisfy `unused_self` lint.

## 2026-03-05 — T-013 Completed
- **Task**: Implement mr wt merge subcommand
- **Status**: ✅ Done
- **Changes**:
  - Replaced stub `cmd_wt_merge` in `src/commands/worktree.rs` with full implementation:
    - Resolves main worktree root and reads state to find the target worktree
    - Creates a `Daemon` instance with optional runner for conflict resolution
    - Delegates to `Daemon::manual_merge()` for the merge workflow
    - Prints colored progress and success/failure messages
  - Added `pub fn manual_merge()` to `Daemon` in `src/worktree/daemon.rs`:
    - Accepts PRD ID and optional `--into <target>` override
    - Validates worktree status is mergeable (Active, Completed, MergeFailed, Conflicted)
    - Rejects Merging/Merged/Abandoned states with descriptive errors
    - Full merge flow: mark merging → integrate target → conflict resolution → UATs → merge into target → mark merged
  - Added `fn validate_mergeable_status()` — extracted status validation for clarity
  - Added `fn handle_merge_conflicts()` — extracted conflict resolution with runner invocation
  - Added `fn smart_merge_into_target()` — cross-worktree aware merge:
    - Uses `git::list_worktrees()` to find where target branch is checked out
    - If checked out in another worktree, merges there directly
    - Otherwise, checks out target in main and merges
  - Updated module doc comment to reflect `wt merge` is no longer a stub
  - 5 new tests (total 720): `validate_mergeable_status_accepts_valid_states`, `validate_mergeable_status_rejects_invalid_states`, `manual_merge_fails_for_unknown_prd`, `manual_merge_rejects_merged_worktree`, `smart_merge_into_target_uses_main_for_checkout`
  - UAT: `cargo make uat` — 720 tests passed, 0 skipped
- **Constitution Compliance**: No violations. Rule 1 (DRY): Reuses existing daemon merge infrastructure (`integrate_target_into_branch`, `resolve_conflicts`, `update_wt_status`, `run_uat`). Rule 8 (Clippy Pedantic): All new code passes `clippy::pedantic` (by-value for Copy types, `map_or_else`, `if let` patterns).

## 2026-03-05 — T-014 Completed
- **Task**: Implement agent-driven state commits
- **Status**: ✅ Done
- **Changes**:
  - Added 3 new git helpers in `src/worktree/git.rs`: `add_file()`, `commit()`, `has_staged_changes()`
  - Added `EventType::StateCommitted` variant in `src/worktree/types.rs` with `Display` impl
  - Implemented state commit logic in `src/worktree/daemon.rs`:
    - `build_state_summary()` — generates human-readable commit message from current state (format: `mr-wt: PRD-0039 merged, PRD-0040 in progress (3 active worktrees)`)
    - `commit_state()` — stages `.mr/worktrees/state.yaml`, checks for actual changes, commits with summary message, records `StateCommitted` event (best-effort, never fails the main operation)
  - Wired `commit_state()` into all significant event paths:
    - `attempt_merge_worktree()`: after merge completed, merge failed (UAT failure), merge failed (target merge failure), conflicted (resolution failure), conflicted (no runner)
    - `manual_merge()`: after merge completed, merge failed (UAT failure), merge failed (target merge failure), conflicted (conflict resolution failure)
  - 9 new tests (total 729): `build_state_summary_single_merged`, `build_state_summary_mixed_states`, `build_state_summary_merge_failed`, `build_state_summary_no_duplicates`, `commit_state_stages_and_commits_in_git_repo`, `commit_state_skips_when_no_changes`, `add_file_stages_specific_file`, `commit_creates_commit_with_message`, `has_staged_changes_detects_changes`
  - UAT: `cargo make uat` — 729 tests passed, 0 skipped
- **Constitution Compliance**: No violations. Rule 1 (DRY): Reuses existing git helpers and state manager. Rule 8 (Clippy Pedantic): All new code passes pedantic lints.

## 2026-03-05 — T-015 Completed
- **Task**: Implement mr wt graph subcommand
- **Status**: ✅ Done
- **Changes**:
  - Replaced stub `cmd_wt_graph()` in `src/commands/worktree.rs` with full implementation
  - Added three renderers: `render_wt_graph_ascii()`, `render_wt_graph_mermaid()`, `render_wt_graph_dot()`
  - Added `wt_risk_level()` helper to compute worst overlap risk for a worktree entry
  - ASCII format: Shows nodes with risk indicators (●/◐/◉), overlap edges with risk labels, shared file lists
  - Mermaid format: Flowchart LR with risk-colored nodes (classDef low/medium/high), edge styles (dashed/solid/bold) by risk
  - DOT format: Graph with filled/colored nodes, edge styles (dashed/solid/bold+penwidth) by risk
  - Filters out merged and abandoned worktrees (only shows active/completing/merging/etc.)
  - Added import for `std::fmt::Write` and `OverlapWarning` type
  - 10 new tests: `test_cmd_wt_graph_fails_without_git_repo`, `test_cmd_wt_graph_rejects_unknown_format`, `test_render_wt_graph_ascii_empty`, `test_render_wt_graph_ascii_with_worktrees`, `test_render_wt_graph_ascii_excludes_merged_and_abandoned`, `test_render_wt_graph_mermaid_empty`, `test_render_wt_graph_mermaid_with_overlaps`, `test_render_wt_graph_dot_empty`, `test_render_wt_graph_dot_with_overlaps`, `test_wt_risk_level_returns_worst`, `test_wt_risk_level_no_warnings`
  - UAT: `cargo make uat` — 739 tests passed, 0 skipped
- **Constitution Compliance**: No violations. Rule 3 (Minimal Changes): Only modified `src/commands/worktree.rs`. Rule 4 (Consistency): Follows rendering patterns from existing `graph.rs`. Rule 8 (Clippy Pedantic): All new code passes pedantic lints.

## 2026-03-05 — T-016 Completed
- **Task**: Implement mr wt remove subcommand
- **Status**: ✅ Done
- **Changes**:
  - Added `EventType::Removed` variant in `src/worktree/types.rs` with `Display` impl and test
  - Replaced stub `cmd_wt_remove` in `src/commands/worktree.rs` with full implementation:
    - Resolves main worktree root, reads state to find worktree entry by PRD ID
    - Safety check: refuses to remove worktrees in `Merging` status with descriptive error
    - Removes git worktree via `git::remove_worktree` (best-effort, gracefully handles missing directories)
    - Optionally deletes branch via `git::delete_branch` with `--delete-branch` flag
    - Updates state: marks worktree as `Abandoned`, clears `run_pid`, records `Removed` event with detail
    - Cleans up overlap warnings that reference the removed worktree
    - Prints colored progress messages matching existing command patterns
  - 4 new tests (total 742): `test_cmd_wt_remove_fails_without_git_repo`, `test_cmd_wt_remove_rejects_merging_worktree`, `test_cmd_wt_remove_succeeds_for_active_worktree`, `test_cmd_wt_remove_unknown_prd_returns_error`
  - UAT: `cargo make uat` — 742 tests passed, 0 skipped
- **Constitution Compliance**: No violations. Rule 3 (Minimal Changes): Only modified `src/commands/worktree.rs` and `src/worktree/types.rs`. Rule 4 (Consistency): Follows patterns from existing commands (error handling, colored output, state management). Rule 8 (Clippy Pedantic): All new code passes pedantic lints.

## 2026-03-05 — T-017 Completed
- **Task**: Implement daemon crash recovery
- **Status**: ✅ Done
- **Changes**:
  - Added `EventType::RecoveryPerformed` variant in `src/worktree/types.rs` with `Display` impl and test
  - Added `git::is_merge_in_progress()` helper in `src/worktree/git.rs` — detects `MERGE_HEAD` file presence
  - Implemented crash recovery in `src/worktree/daemon.rs`:
    - `recover_stale_state()` — entry point called in `run()` before event loop; reads state and delegates to detection + application
    - `detect_recovery_actions()` — scans worktrees for stale conditions: orphaned paths, stuck merging status, dead run PIDs
    - `apply_recovery_actions()` — atomically applies all recovery actions: aborts stale rebases/merges, resets statuses, records `RecoveryPerformed` events
  - Recovery handles 5 scenarios:
    - **Orphaned worktrees** (path missing on disk): marked `Abandoned`
    - **Stale rebase** (Merging + rebase in progress): aborted and reset to `Completed`
    - **Stale merge** (Merging + merge in progress): aborted and reset to `Completed`
    - **Stuck merging** (Merging + no operation in progress): reset to `Completed`
    - **Dead run process** (Active + dead PID): marked `Completed` with PID cleared
  - Already-Merged and Abandoned worktrees are skipped (no false positives)
  - 7 new tests (total 749): `recover_stale_state_noop_when_clean`, `recover_stale_state_marks_orphaned_worktree`, `recover_stale_state_resets_partial_merge_no_operation`, `recover_stale_state_completes_dead_process`, `recover_stale_state_skips_already_merged_and_abandoned`, `recover_stale_state_multiple_issues`, `is_merge_in_progress_false_normally`
  - UAT: `cargo make uat` — 749 tests passed, 0 skipped
- **Constitution Compliance**: No violations. Rule 2 (SOC): Recovery detection separated from application. Rule 3 (Minimal Changes): Only modified relevant files. Rule 8 (Clippy Pedantic): All new code passes pedantic lints.

## 2026-03-05 — T-019 Completed
- **Task**: Update AGENTS.md with worktree orchestration workflow
- **Status**: ✅ Done
- **Changes**:
  - Updated `AGENTS.md` Workspace Overview to include `worktree/` module and `.mr/worktrees/` directory
  - Added comprehensive "Worktree Orchestration (`mr wt`)" section covering:
    - Architecture overview (worktrees, daemon, state file, IPC, locking)
    - Source layout table mapping files to responsibilities
    - Full usage examples for all subcommands
    - Subcommands reference table and flags reference table
    - Daemon lifecycle (auto-start, two-tier heartbeat, auto-merge, auto-exit, crash recovery)
    - IPC protocol documentation (message types and response format)
    - State file schema with annotated YAML example
    - Worktree naming conventions
    - Merge strategy (5-step pipeline: integrate, resolve, UAT, merge, commit)
    - Troubleshooting guide (6 common scenarios)
    - Important notes (backward compatibility, LLM cost, advisory locking)
  - UAT: `cargo make uat` — 749 tests passed, 0 skipped
- **Constitution Compliance**: No violations. Rule 3 (Minimal Changes): Only modified `AGENTS.md` and PRD file. Documentation-only task.
