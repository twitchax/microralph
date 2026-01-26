---
id: PRD-0024
title: "Add Progress Indicators for Long-Running Operations"
status: active
owner: "twitchax"
created: 2026-01-26
updated: 2026-01-26

principles:
  - Spinners display only when --stream is false (streaming already provides feedback)
  - Automatically disable spinners when stdout is not a TTY (CI, redirected output)
  - Clear spinner before displaying accumulated runner output
  - Reset spinner with iteration count for multi-step operations
  - Use indicatif crate for spinner implementation

references:
  - name: indicatif crate
    url: https://docs.rs/indicatif/latest/indicatif/
  - name: owo-colors supports-colors feature
    url: https://docs.rs/owo-colors/latest/owo_colors/

acceptance_tests:
  - id: uat-001
    name: Spinner displays during mr run when not streaming
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: Spinner displays during mr refactor when not streaming
    command: cargo make uat
    uat_status: unverified
  - id: uat-003
    name: Spinner displays during mr suggest when not streaming
    command: cargo make uat
    uat_status: unverified
  - id: uat-004
    name: Spinner is hidden when stdout is not a TTY
    command: cargo make uat
    uat_status: unverified
  - id: uat-005
    name: Spinner clears before output is displayed
    command: cargo make uat
    uat_status: unverified

tasks:
  - id: T-001
    title: Add indicatif dependency to Cargo.toml
    priority: 1
    status: done
    notes: Add indicatif crate with default features.
  - id: T-002
    title: Create spinner utility module
    priority: 2
    status: done
    notes: "Create src/spinner.rs with helper functions: start_spinner, update_message, finish_and_clear. Include TTY detection logic using std::io::stdout().is_terminal()."
  - id: T-003
    title: Integrate spinner into mr run command
    priority: 3
    status: done
    notes: Add spinner that shows task progress (e.g., "Running task 2/5..."). Only display when stream=false. Clear before showing output.
  - id: T-004
    title: Integrate spinner into mr run --loop mode
    priority: 4
    status: done
    notes: Reset spinner between iterations with message like "Task 2/5...".
  - id: T-005
    title: Integrate spinner into mr refactor command
    priority: 5
    status: todo
    notes: Show iteration progress (e.g., "Refactor iteration 2/5..."). Reset between iterations.
  - id: T-006
    title: Integrate spinner into mr suggest command
    priority: 6
    status: todo
    notes: Show analyzing spinner during AI generation phase.
  - id: T-007
    title: Integrate spinner into mr finalize command
    priority: 7
    status: todo
    notes: Show spinner during agent execution phase.
  - id: T-008
    title: Integrate spinner into mr reindex command
    priority: 8
    status: todo
    notes: Show spinner during link verification agent call.
  - id: T-009
    title: Add integration tests for spinner behavior
    priority: 9
    status: todo
    notes: Test that spinner is disabled in non-TTY environments and clears properly.

---

# Summary

Add visual feedback during long-running operations using spinners from the `indicatif` crate. Commands like `mr run`, `mr refactor`, `mr suggest`, `mr finalize`, and `mr reindex` can take minutes without visible feedback unless `--stream` is enabled. This feature adds spinners that display during agent execution, reset between iterations with progress counts, and automatically disable when stdout is not a TTY.

---

# Problem

Currently, when running commands like `mr run`, `mr refactor`, or `mr suggest` without the `--stream` flag, users see no visual feedback while waiting for potentially long agent operations. This creates uncertainty about whether the command is still running or has stalled. Users must either enable streaming (which produces verbose output) or wait without any progress indication.

---

# Goals

1. Show a spinner during agent execution when `--stream` is false
2. Display iteration progress for multi-step operations (e.g., "Task 2/5...", "Refactor iteration 2/5...")
3. Clear the spinner cleanly before displaying accumulated output
4. Automatically disable spinners when stdout is not a TTY (CI, redirected output)
5. Apply consistent spinner behavior across all long-running commands

---

# Non-Goals (MVP)

- Progress bars with percentage completion (spinners with text updates are sufficient)
- Spinners during streaming mode (streaming already provides visual feedback)
- Custom spinner styles or themes
- Spinner behavior customization via config or flags

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-26 — T-001 Completed
- **Task**: Add indicatif dependency to Cargo.toml
- **Status**: ✅ Done
- **Changes**:
  - Added `indicatif = "0.17"` to dependencies in Cargo.toml (alphabetically between clap and owo-colors)
  - UAT passed: 344 tests run, 344 passed

---

## 2026-01-26 — T-002 Completed
- **Task**: Create spinner utility module
- **Status**: ✅ Done
- **Changes**:
  - Created `src/spinner.rs` with `Spinner` struct and `start_spinner()` function
  - `Spinner` wraps indicatif's `ProgressBar` with automatic TTY detection via `std::io::stdout().is_terminal()`
  - Provides `set_message()` and `finish_and_clear()` methods
  - When stdout is not a TTY (CI, piped output), spinner operations become no-ops
  - Added module declaration to `main.rs`
  - Includes 4 unit tests covering disabled/enabled spinners and static/dynamic messages
  - UAT passed: 348 tests run, 348 passed

---

## 2026-01-26 — T-003 Completed
- **Task**: Integrate spinner into mr run command
- **Status**: ✅ Done
- **Changes**:
  - Added `use crate::spinner::start_spinner;` import to `src/run.rs`
  - Integrated spinner in `run_task()` function that displays "Running task N/M..." during agent execution
  - Spinner starts when `stream=false` (enabled via `!config.stream`)
  - Spinner clears before output is displayed via `finish_and_clear()`
  - Spinner automatically disabled in non-TTY environments (handled by spinner module's TTY detection)
  - UAT passed: 348 tests run, 348 passed

---

## 2026-01-26 — T-004 Completed
- **Task**: Integrate spinner into mr run --loop mode
- **Status**: ✅ Done
- **Changes**:
  - Verified that T-003's implementation already satisfies T-004's requirements
  - The spinner in `run_task()` (src/run.rs:408-428) already handles loop mode correctly:
    - Each call to `run_task()` creates a fresh spinner with updated task count ("Running task N/M...")
    - `finish_and_clear()` is called before output display, resetting the spinner for the next iteration
    - The loop in main.rs (line 1262) calls `run_task()` repeatedly, each time getting correct task numbers
  - No additional code changes required - the spinner automatically resets between iterations due to the architecture of one spinner per `run_task()` invocation
  - UAT passed: 348 tests run, 348 passed

---