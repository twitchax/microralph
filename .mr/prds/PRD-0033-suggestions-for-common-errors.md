---
id: PRD-0033
title: "Suggestions for Common Errors"
status: active
owner: twitchax
created: 2026-02-06
updated: 2026-02-06
principles:
  - Improve user experience by adding actionable suggestions to common error messages
  - Keep changes minimal — enhance existing bail!/context messages inline
  - Do not introduce new error types or abstractions; use simple string improvements
  - Follow existing color/formatting patterns (colors::warning, colors::error)
references:
  - name: "anyhow error handling"
    url: "https://docs.rs/anyhow/latest/anyhow/"
acceptance_tests:
  - id: uat-001
    name: "Constitution missing produces suggestion to run mr restore or mr init"
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: "Malformed config.toml produces suggestion to run mr restore"
    command: cargo make uat
    uat_status: unverified
  - id: uat-003
    name: "PRD not found produces suggestion to run mr status"
    command: cargo make uat
    uat_status: unverified
  - id: uat-004
    name: "Invalid PRD format produces suggestion about correct format"
    command: cargo make uat
    uat_status: unverified
  - id: uat-005
    name: "Interactive session failure produces suggestion to retry"
    command: cargo make uat
    uat_status: unverified
  - id: uat-006
    name: "Partial init state produces suggestion to run mr init or mr restore"
    command: cargo make uat
    uat_status: unverified
  - id: uat-007
    name: "All existing tests continue to pass"
    command: cargo make ci
    uat_status: unverified
tasks:
  - id: T-001
    title: "Improve constitution-missing error with restore/init suggestion"
    priority: 1
    status: done
    notes: "In src/config/constitution.rs, when constitution file is not found, add suggestion to run `mr restore` or `mr init`"
  - id: T-002
    title: "Add suggestion for malformed config.toml parse errors"
    priority: 1
    status: done
    notes: "In src/config/mod.rs, wrap TOML parse errors with a suggestion to run `mr restore` to reset config"
  - id: T-003
    title: "Add suggestion to PRD-not-found errors"
    priority: 1
    status: todo
    notes: "In src/prd/edit.rs, src/prd/finalize.rs, and src/commands/run.rs — add suggestion to run `mr status` to list available PRDs"
  - id: T-004
    title: "Add suggestion to invalid PRD format errors"
    priority: 2
    status: todo
    notes: "In src/prd/parser.rs, when frontmatter parsing fails, suggest checking PRD format or recreating with `mr new`"
  - id: T-005
    title: "Improve interactive session failure messages"
    priority: 2
    status: todo
    notes: "In src/prd/new.rs, improve Ctrl+C and runner crash messages with clearer retry guidance"
  - id: T-006
    title: "Improve partial init state detection and messaging"
    priority: 2
    status: todo
    notes: "In src/commands/init.rs ensure_initialized(), when .mr/ exists but subdirectories are missing, suggest `mr init` or `mr restore` instead of generic init message"
  - id: T-007
    title: "Add unit tests for improved error messages"
    priority: 3
    status: todo
    notes: "Add tests verifying that error messages contain the expected suggestion text for each improved error path"
---

# Summary

Audit and improve error messages across the microralph CLI so that common "weird state" errors include actionable `Suggestion: ...` text guiding users toward resolution. Currently, many error paths produce raw or unhelpful messages when users hit states like missing constitution files, corrupt configs, or missing PRDs. This PRD adds clear, contextual suggestions to the most impactful error paths.

# Problem

When users encounter errors caused by partial initialization, missing files after upgrades, corrupt configuration, or invalid PRD state, the CLI often provides only a raw error message without guidance on how to fix the issue. This forces users to search documentation or guess at the correct recovery command. For example, if a user upgrades microralph and their constitution file is missing, they see `"Constitution file not found at .mr/constitution.md"` with no hint that `mr restore` would fix it.

# Goals

1. Add actionable `Suggestion: ...` text to the most common user-facing error paths
2. Cover at least these categories: missing constitution, malformed config, PRD not found, invalid PRD format, interactive session failures, and partial init state
3. Keep changes minimal — enhance existing `bail!()` and `.context()` messages inline without introducing new error types
4. Ensure all existing tests continue to pass

# Technical Approach

This is a straightforward text-improvement pass across existing error paths. No new abstractions or error types are needed.

For each targeted error path:
1. Locate the `bail!()`, `anyhow!()`, or `.context()` call
2. Append a `\nSuggestion: <actionable command or guidance>` line to the error message
3. Add a unit test verifying the suggestion text is present

Example transformation:
```rust
// Before
bail!("Constitution file not found at {path}");

// After
bail!("Constitution file not found at {path}.\n  Suggestion: Run `mr restore` to regenerate default files, or `mr init` to reinitialize.");
```

The changes are localized to individual error sites — no cross-cutting changes needed.

# Assumptions

- Users encountering these errors are familiar with basic `mr` commands (`init`, `restore`, `status`, `new`)
- The existing `anyhow` error chain display is sufficient — no custom error rendering is needed
- Prompt loading already has a silent fallback mechanism, so prompt-missing errors are lower priority

# Constraints

- Must not change any public API signatures (constitution rule 5)
- Must not introduce new dependencies
- Must follow existing error handling patterns (anyhow + bail!)
- Changes should be limited to error message strings and associated tests (constitution rule 3)

# References to Code

- `src/config/constitution.rs` — constitution file-not-found error (~line 72)
- `src/config/mod.rs` — config.toml loading and TOML parse errors (~line 76-81)
- `src/prd/edit.rs` — PRD-not-found error (~line 166)
- `src/prd/finalize.rs` — PRD-not-found in finalization (~line 89)
- `src/prd/parser.rs` — frontmatter parsing errors
- `src/prd/new.rs` — interactive session failure messages (~line 130-146)
- `src/commands/init.rs` — `ensure_initialized()` and `is_initialized()` (~line 2116-2133)
- `src/commands/run.rs` — PRD-not-found during run
- `src/util/colors.rs` — existing color/formatting helpers

# Non-Goals (MVP)

- Creating a centralized error type or error code system
- Adding structured error output (JSON error format)
- Dynamic suggestions based on OS or environment detection
- Improving runner install suggestions with platform-specific URLs
- Adding a `mr doctor` or `mr diagnose` command

# History

## 2026-02-06 — T-001 Completed
- **Task**: Improve constitution-missing error with restore/init suggestion
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/config/constitution.rs` line 72-75: enhanced `bail!` message for missing constitution file to include `Suggestion: Run \`mr restore\` to regenerate default files, or \`mr init\` to reinitialize.`
  - UAT passed: 497 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-06 — T-002 Completed
- **Task**: Add suggestion for malformed config.toml parse errors
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/config/mod.rs` line 81: wrapped `toml::from_str` with `.with_context()` adding `Suggestion: Run \`mr restore\` to reset config to defaults.`
  - Added `test_config_load_malformed_includes_suggestion` unit test verifying the suggestion text is present in parse error messages
  - UAT passed: 498 tests, 0 failures
- **Constitution Compliance**: No violations.
