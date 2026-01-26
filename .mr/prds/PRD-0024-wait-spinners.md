---
id: PRD-0024
title: "Add Progress Indicators for Long-Running Operations"
status: draft
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
    status: todo
    notes: Add indicatif crate with default features.
  - id: T-002
    title: Create spinner utility module
    priority: 2
    status: todo
    notes: "Create src/spinner.rs with helper functions: start_spinner, update_message, finish_and_clear. Include TTY detection logic using std::io::stdout().is_terminal()."
  - id: T-003
    title: Integrate spinner into mr run command
    priority: 3
    status: todo
    notes: Add spinner that shows task progress (e.g., "Running task 2/5..."). Only display when stream=false. Clear before showing output.
  - id: T-004
    title: Integrate spinner into mr run --loop mode
    priority: 4
    status: todo
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

---