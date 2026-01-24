---
id: PRD-0006
title: Add Stdout Colors
status: active
owner: twitchax
created: 2026-01-24
updated: 2026-01-24
tasks:
  - id: T-001
    title: Add owo-colors dependency and create color utilities module
    priority: 1
    status: done
  - id: T-002
    title: Colorize success messages with green and emoji prefixes
    priority: 1
    status: done
  - id: T-003
    title: Colorize error and warning messages with red/yellow and emoji prefixes
    priority: 1
    status: done
  - id: T-004
    title: Style question prompts with blue color, bold text, and emoji prefix
    priority: 2
    status: done
  - id: T-005
    title: Add color to informational and status messages
    priority: 2
    status: done
  - id: T-006
    title: Colorize finalization summary box and separators
    priority: 3
    status: todo

acceptance_tests:
  - id: uat-001
    name: Success messages display in green with emoji prefix
    command: cargo run -- prd list 2>&1 | grep -E '✅|Created|success'
    uat_status: unverified
  - id: uat-002
    name: Error messages display in red with emoji prefix
    command: cargo run -- run --prd PRD-NONEXISTENT 2>&1 | grep -E '❌|Error|error'
    uat_status: unverified
  - id: uat-003
    name: Question prompts display in blue with bold text
    command: echo "manual verification required"
    uat_status: unverified
  - id: uat-004
    name: Colors disabled when stdout is piped
    command: cargo run -- prd list 2>&1 | cat | grep -v $'\033'
    uat_status: unverified
  - id: uat-005
    name: Existing emoji usage preserved and enhanced
    command: cargo run -- prd list 2>&1 | grep -E '✅|📋|🧪|⚠️'
    uat_status: unverified
  - id: uat-006
    name: Finalization summary box renders with styling
    command: echo "manual verification required"
    uat_status: unverified
  - id: uat-007
    name: NO_COLOR environment variable disables colors
    command: NO_COLOR=1 cargo run -- prd list 2>&1 | grep -v $'\033'
    uat_status: unverified
---

## Summary

Add terminal colorization and emoji enhancements to microralph's CLI output to improve readability and user experience. Colors should automatically degrade to plain text when output is piped or the terminal doesn't support colors.

## Problem

Currently, microralph's CLI output is plain monochrome text, making it difficult for users to quickly scan and identify important information like success/failure states, questions requiring input, and status updates. The output lacks visual hierarchy and doesn't leverage terminal capabilities to improve UX.

## Goals

1. **Improve visual hierarchy**: Use colors and emojis to distinguish between success, error, warning, and informational messages
2. **Enhance Q/A readability**: Make question prompts visually distinct with blue color, bold formatting, and question emoji (❓)
3. **Maintain terminal compatibility**: Auto-detect TTY support and gracefully degrade to plain text for piped output
4. **Keep it tasteful**: Use colors and emojis sparingly to add clarity, not clutter
5. **Leverage existing patterns**: Build on the existing emoji usage in `prd list` command

## Non-Goals

- **Colorizing tracing output**: Tracing/logging is out of scope; focus on main.rs CLI output only
- **Configuration options**: No user settings for color preferences in this iteration
- **Windows-specific handling**: Rely on library defaults for cross-platform support

## Relevant References

### Code Entry Points
- `src/main.rs` — All CLI output (~109 `println!` calls to update)
- `src/init.rs` — Minor output (3 `println!` calls)

### Existing Emoji Patterns
- `src/main.rs:668-686` — Task status emojis (✅, 📋) and UAT status emojis (🧪, ⚠️)

### Message Categories to Colorize

**Success messages (green + ✅):**
- `src/main.rs:305` — "Initialized microralph!"
- `src/main.rs:548` — "PRD created successfully!"
- `src/main.rs:608` — "PRD edited successfully!"
- `src/main.rs:906` — "Task {} completed successfully!"
- `src/main.rs:949` — "PRD {} is complete!"

**Error/Warning messages (red + ❌ or yellow + ⚠️):**
- `src/main.rs:908` — "Task {} did not complete successfully."
- `src/main.rs:938` — "All tasks done but UAT(s) need verification."

**Headers/Sections (bold or cyan):**
- `src/main.rs:662` — "PRDs:"
- `src/main.rs:712-749` — Section headers ("Active:", "Draft:", "Done:", "Parked:")
- `src/main.rs:800-821` — Finalization summary box

**Next steps/hints (dim or standard):**
- `src/main.rs:335-338` — "Next steps:" numbered list
- `src/main.rs:291,943` — "Run `mr ...`" suggestions

### Library Choice
Recommend **owo-colors** based on research:
- Zero-allocation, actively maintained
- Automatic TTY detection and color disable for piped output
- Honors `NO_COLOR` and `FORCE_COLOR` environment variables
- Clean ergonomic API: `"text".green().bold()`

## Acceptance Tests

See frontmatter for UAT definitions.

## History

## 2026-01-24 — T-001 Completed
- **Task**: Add owo-colors dependency and create color utilities module
- **Status**: ✅ Done
- **Changes**:
  - Added `owo-colors` crate v4 with `supports-colors` feature to Cargo.toml
  - Created `src/colors.rs` module with utility functions for success, error, warning, info, question, header, and dim styling
  - Color utilities automatically detect TTY support and degrade gracefully for piped output
  - Module honors `NO_COLOR` and `FORCE_COLOR` environment variables via owo-colors
  - Added module declaration to main.rs
  - All 254 tests pass in `cargo make uat`

## 2026-01-24 — T-002 Completed
- **Task**: Colorize success messages with green and emoji prefixes
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/main.rs` to use `colors::success()` for all success messages
  - Applied green color with ✅ emoji prefix to:
    - "Initialized microralph!" (line 308)
    - "PRD created successfully!" (line 551)
    - "PRD edited successfully!" (line 611)
    - "Task {} completed successfully!" (line 933)
    - "PRD {} is complete!" (line 1014)
  - All success messages now display consistently with visual emphasis
  - All 254 tests pass in `cargo make uat`

## 2026-01-24 — T-003 Completed
- **Task**: Colorize error and warning messages with red/yellow and emoji prefixes
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/main.rs` to use `colors::error()` and `colors::warning()` for error and warning messages
  - Applied red color with ❌ emoji prefix to error messages:
    - "Task {} did not complete successfully." (line 940)
    - "UAT verification loop failed: {e}" (line 1015)
  - Applied yellow color with ⚠️ emoji prefix to warning message:
    - "All tasks done for {} but {} UAT(s) need verification." (lines 972-975)
  - All error and warning messages now display with consistent visual emphasis
  - All 254 tests pass in `cargo make uat`

## 2026-01-24 — T-004 Completed
- **Task**: Style question prompts with blue color, bold text, and emoji prefix
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/prd_new.rs` to use `colors::question()` for question prompts
  - Applied blue color, bold formatting, and ❓ emoji prefix to:
    - "Would you like to provide additional context..." prompt (line 522)
    - All numbered questions in `collect_answers()` (line 552)
  - Updated `src/prd_edit.rs` to use `colors::question()` for question prompts
  - Applied same styling to numbered questions in `collect_answers()` (line 292)
  - Question prompts now display consistently with blue bold text and emoji
  - All 254 tests pass in `cargo make uat`

## 2026-01-24 — T-005 Completed
- **Task**: Add color to informational and status messages
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/main.rs` to use `colors::info()`, `colors::header()`, and `colors::dim()` for informational and status messages
  - Applied cyan color to informational messages like "Bootstrapping repository...", "Detected language:", "Continuing to next task...", "All UATs verified or opted out"
  - Applied bold styling to headers like "PRDs:", "Active:", "Draft:", "Done:", "Parked:", "Next steps:", "Runner output:", "UAT verification loop completed:"
  - Applied dim styling to secondary/contextual information like file paths, PRD/task details, statistics, and help hints
  - Colorized messages in init, bootstrap, prd new/edit/list, run, and reindex commands
  - All 254 tests pass in `cargo make uat`


