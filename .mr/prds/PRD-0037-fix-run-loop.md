---
id: PRD-0037
title: "Fix Run Loop: Return to Task Execution After UAT-Added Tasks"
status: active
owner: twitchax
created: 2026-02-23
updated: 2026-02-23
principles:
  - "Convergence over infinite loops: the loop must always terminate"
  - "Minimal changes to existing run loop structure"
  - "Reuse existing PRD reload and next_task() logic"
  - "No breaking changes to CLI flags or public API"
references:
  - name: "PRD-0036: UAT Skipping and Dynamic Task Addition"
    url: ".mr/prds/PRD-0036-uat-skipping.md"
  - name: "Run loop orchestration"
    url: "src/main.rs"
  - name: "Run task and UAT verification"
    url: "src/commands/run.rs"
acceptance_tests:
  - id: uat-001
    name: "Run loop re-enters task execution when UAT verification adds a new task"
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: "Run loop terminates normally when no new tasks are added during UAT"
    command: cargo make uat
    uat_status: unverified
  - id: uat-003
    name: "UAT verification loop breaks early when new incomplete tasks are detected"
    command: cargo make uat
    uat_status: unverified
  - id: uat-004
    name: "Run loop converges and does not loop infinitely"
    command: cargo make uat
    uat_status: unverified
tasks:
  - id: T-001
    title: "Add has_incomplete_tasks() public method to Prd"
    priority: 1
    status: done
    notes: "The method already exists as #[cfg(test)]-only (incomplete_tasks at types.rs:429). Either remove the cfg(test) gate or add a new has_incomplete_tasks() -> bool helper. next_task().is_some() could also work."
  - id: T-002
    title: "Make UAT verification loop detect new incomplete tasks and break early"
    priority: 2
    status: done
    notes: "In run_uat_verification_loop (run.rs:765), after each iteration reload the PRD and check next_task(). If a new incomplete task exists, break out of the UAT loop with a new result field (e.g., has_new_tasks: bool on UatVerificationLoopResult)."
  - id: T-003
    title: "Update cmd_run outer loop to continue when new tasks exist after UAT"
    priority: 3
    status: todo
    notes: "In cmd_run (main.rs:1555-1585), after run_uat_verification_loop completes, check if new incomplete tasks exist (via the result flag or by re-reading the PRD). If yes, continue the outer loop instead of break. Add a safety counter to prevent infinite cycling."
  - id: T-004
    title: "Add unit tests for new UAT loop early-break and outer loop re-entry"
    priority: 4
    status: todo
    notes: "Test that UatVerificationLoopResult correctly reports has_new_tasks. Test that the outer loop re-enters task execution when new tasks are found. Test convergence (loop terminates when no new tasks are added)."
---

# Summary

The `mr run` loop has a two-phase structure: task execution followed by UAT verification. PRD-0036 added the ability for agents to dynamically add new tasks during UAT verification. However, after the UAT verification loop finishes, the outer loop unconditionally `break`s — meaning newly added tasks are never executed. The user must manually run `mr run` again to pick them up.

This PRD fixes the run loop so that after UAT verification, it checks for new incomplete tasks and re-enters the task execution phase if any exist, creating a natural task→UAT→task cycle that converges when all tasks are done and all UATs are verified.

# Problem

When the agent adds a new task during UAT verification (Option E in the UAT prompt), the task is written to the PRD file on disk. However, the control flow in `cmd_run()` (main.rs:1585) unconditionally breaks out of the outer loop after UAT verification completes. This means:

1. The new task is saved in the PRD but never executed in the current session.
2. The user must run `mr run` again to execute the newly added task.
3. This defeats the purpose of dynamic task addition — the agent identifies work needed to unblock a UAT but can't act on it in the same session.

The root cause is that the outer loop treats UAT verification as a terminal phase with no path back to task execution.

# Goals

1. After UAT verification completes, the run loop checks for new incomplete tasks and re-enters task execution if any exist.
2. The UAT verification loop detects new incomplete tasks mid-loop and breaks early to return control to the outer loop sooner.
3. The loop converges naturally and has a safety mechanism to prevent infinite cycling.
4. No changes to CLI flags or public API surface.

# Technical Approach

The fix modifies two control flow points:

## 1. UAT Verification Loop Early Break (run.rs)

After each UAT iteration, the loop reloads the PRD (line 766). Add a check: if `current_prd.next_task().is_some()`, a new task was added. Set a flag on the result and break out of the UAT loop early.

```
run_uat_verification_loop:
  loop {
    reload PRD
    if no unverified UATs → break (done)
    if iterations >= max → break (limit)
    execute UAT verification
    reload PRD again (or use the same reload)
    if next_task().is_some() → set has_new_tasks=true, break  ← NEW
  }
```

## 2. Outer Loop Re-entry (main.rs)

After the UAT verification loop returns, check the result. If `has_new_tasks` is true, `continue` the outer loop instead of `break`. Add a safety counter to cap the total number of task→UAT cycles.

```
cmd_run outer loop:
  loop {
    result = run_task(...)
    match result {
      TaskExecuted → continue
      NeedsUatVerification → {
        uat_result = run_uat_verification_loop(...)
        if uat_result.has_new_tasks → continue  ← NEW
        else → break
      }
      PrdComplete → break
    }
  }
```

## Data Flow

```
┌─────────────────────────────────────────────────────┐
│                  cmd_run outer loop                  │
│                                                     │
│  ┌──────────────┐    all tasks done    ┌─────────┐  │
│  │  run_task()   │ ──────────────────► │  UAT    │  │
│  │  (execute     │                     │  verify │  │
│  │   next task)  │ ◄────────────────── │  loop   │  │
│  └──────────────┘   has_new_tasks=true └─────────┘  │
│         │                                   │       │
│         │ task failed / --one               │       │
│         ▼                                   ▼       │
│       break                          all verified   │
│                                      or max iters   │
│                                          break      │
└─────────────────────────────────────────────────────┘
```

# Assumptions

- Agents that add tasks during UAT verification set the new task status to `todo`, which `next_task()` will find.
- The PRD file on disk is the source of truth and is re-read on each iteration (already the case).
- The existing `max_iterations` on the UAT loop provides a per-UAT-cycle guard; the new outer-loop safety counter provides a cross-cycle guard.

# Constraints

- Must not change public CLI flags or API signatures (constitution rule 5).
- Must follow minimal changes principle (constitution rule 3) — only modify the control flow, not restructure the loop.
- The safety counter for the outer loop should have a reasonable default (e.g., 10 task→UAT cycles) to prevent runaway loops.

# References to Code

- `src/main.rs:1508-1593` — `cmd_run()` outer loop with the unconditional `break` at line 1585
- `src/commands/run.rs:360-487` — `run_task()` which re-reads the PRD and finds the next task
- `src/commands/run.rs:739-844` — `run_uat_verification_loop()` which needs early-break on new tasks
- `src/commands/run.rs` — `UatVerificationLoopResult` struct that needs a `has_new_tasks` field
- `src/prd/types.rs:419-426` — `next_task()` method used to detect incomplete tasks
- `src/prd/types.rs:429` — `incomplete_tasks()` (currently `#[cfg(test)]` only)

# Non-Goals (MVP)

- Changing the maximum UAT iterations default.
- Adding a CLI flag to control the outer-loop cycle limit.
- Changing how agents decide to add tasks (prompt changes).
- Making the UAT verification loop itself execute tasks (it should hand control back to the outer loop).

# History

## 2026-02-23 — T-001 Completed
- **Task**: Add has_incomplete_tasks() public method to Prd
- **Status**: ✅ Done
- **Changes**:
  - Added `has_incomplete_tasks() -> bool` public method to `Prd` in `src/prd/types.rs` (line 429)
  - Delegates to `self.next_task().is_some()` for consistency with existing logic
  - Added `#[allow(dead_code)]` since the method will be consumed by T-002/T-003
  - UAT passed: 567/567 tests pass
- **Constitution Compliance**: No violations. Minimal change (3 lines added), follows existing patterns.

## 2026-02-23 — T-002 Completed
- **Task**: Make UAT verification loop detect new incomplete tasks and break early
- **Status**: ✅ Done
- **Changes**:
  - Added `has_new_tasks: bool` field to `UatVerificationLoopResult` in `src/commands/run.rs`
  - After each UAT iteration, reload the PRD and check `has_incomplete_tasks()`; if true, break early with `has_new_tasks: true`
  - Added `require_prd_by_id()` helper to reduce repeated `find_prd_by_id()?.ok_or_else()` boilerplate (DRY principle)
  - Updated `print_uat_result()` in `src/main.rs` to display a warning when new tasks are detected
  - All three `UatVerificationLoopResult` construction sites include the new field
  - UAT passed: 567/567 tests pass
- **Constitution Compliance**: No violations. Minimal changes, follows existing patterns, `require_prd_by_id` helper follows DRY principle.

