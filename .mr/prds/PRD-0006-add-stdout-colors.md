---
id: PRD-0006
title: Add Stdout Colors
status: draft
owner: twitchax
created: 2026-01-24
updated: 2026-01-24
tasks:
  - id: T-001
    title: Add owo-colors dependency and create color utilities module
    priority: 1
    status: todo
  - id: T-002
    title: Colorize success messages with green and emoji prefixes
    priority: 1
    status: todo
  - id: T-003
    title: Colorize error and warning messages with red/yellow and emoji prefixes
    priority: 1
    status: todo
  - id: T-004
    title: Style question prompts with blue color, bold text, and emoji prefix
    priority: 2
    status: todo
  - id: T-005
    title: Add color to informational and status messages
    priority: 2
    status: todo
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

