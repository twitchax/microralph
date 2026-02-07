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
    uat_status: verified
  - id: uat-002
    name: "Malformed config.toml produces suggestion to run mr restore"
    command: cargo make uat
    uat_status: verified
  - id: uat-003
    name: "PRD not found produces suggestion to run mr status"
    command: cargo make uat
    uat_status: verified
  - id: uat-004
    name: "Invalid PRD format produces suggestion about correct format"
    command: cargo make uat
    uat_status: verified
  - id: uat-005
    name: "Interactive session failure produces suggestion to retry"
    command: cargo make uat
    uat_status: verified
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
    status: done
    notes: "In src/prd/edit.rs, src/prd/finalize.rs, and src/commands/run.rs — add suggestion to run `mr status` to list available PRDs"
  - id: T-004
    title: "Add suggestion to invalid PRD format errors"
    priority: 2
    status: done
    notes: "In src/prd/parser.rs, when frontmatter parsing fails, suggest checking PRD format or recreating with `mr new`"
  - id: T-005
    title: "Improve interactive session failure messages"
    priority: 2
    status: done
    notes: "In src/prd/new.rs, improve Ctrl+C and runner crash messages with clearer retry guidance"
  - id: T-006
    title: "Improve partial init state detection and messaging"
    priority: 2
    status: done
    notes: "In src/commands/init.rs ensure_initialized(), when .mr/ exists but subdirectories are missing, suggest `mr init` or `mr restore` instead of generic init message"
  - id: T-007
    title: "Add unit tests for improved error messages"
    priority: 3
    status: done
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

## 2026-02-06 — T-003 Completed
- **Task**: Add suggestion to PRD-not-found errors
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/prd/edit.rs` line 227: enhanced `bail!` for `find_prd()` to include `Suggestion: Run \`mr status\` to list available PRDs.`
  - Updated `src/prd/finalize.rs` line 51: enhanced `FinalizeError::PrdNotFound` `#[error]` message with the same suggestion
  - Updated `src/commands/run.rs`: enhanced all 5 `ok_or_else` PRD-not-found closures (`run_task`, `process_uat_iteration_outcome`, `run_uat_verification_loop` ×3) with the suggestion
  - UAT passed: 498 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-06 — T-004 Completed
- **Task**: Add suggestion to invalid PRD format errors
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/prd/parser.rs` line 87: enhanced `bail!` for missing opening `---` delimiter to include `Suggestion: Check the PRD format or recreate with \`mr new\`.`
  - Updated `src/prd/parser.rs` line 100: enhanced `anyhow!` for missing closing `---` delimiter with the same suggestion
  - Updated `src/prd/parser.rs` line 40: enhanced `map_err` for YAML parse failure with the same suggestion
  - Added 3 unit tests (`test_missing_frontmatter_includes_suggestion`, `test_unclosed_frontmatter_includes_suggestion`, `test_invalid_yaml_frontmatter_includes_suggestion`) verifying suggestion text is present
  - UAT passed: 501 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-07 — T-005 Completed
- **Task**: Improve interactive session failure messages
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/prd/new.rs` line 121: enhanced interrupted session `bail!` to include `Suggestion: Re-run \`mr new\` to start a fresh PRD creation session.`
  - Updated `src/prd/new.rs` line 125: enhanced failed session `bail!` to include `Suggestion: Re-run \`mr new\` to retry. If the problem persists, check that your runner (e.g., \`copilot\` or \`claude\` CLI) is installed and working.`
  - Existing tests (`test_create_prd_aborts_on_interrupted_signal`, `test_create_prd_aborts_on_process_failure`) continue to pass with new messages
  - UAT passed: 501 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-07 — T-006 Completed
- **Task**: Improve partial init state detection and messaging
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/commands/init.rs` `ensure_initialized()`: expanded from a single `bail!` into a two-branch check — when `.mr/` exists but subdirectories/files are missing, reports "partially initialized" with a list of missing items and suggests `mr init` or `mr restore`; when `.mr/` doesn't exist at all, keeps the original generic message.
  - Added `test_ensure_initialized_partial_init_includes_suggestion` unit test verifying partial init detection lists missing items and includes both `mr init` and `mr restore` suggestions.
  - Added `test_ensure_initialized_no_mr_dir_generic_message` unit test verifying the generic message path when `.mr/` doesn't exist.
  - UAT passed: 503 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-07 — T-007 Completed
- **Task**: Add unit tests for improved error messages
- **Status**: ✅ Done
- **Changes**:
  - Added `test_constitution_missing_includes_suggestion` in `src/config/constitution.rs`: verifies missing constitution error contains "Suggestion:", "mr restore", and "mr init"
  - Added `test_find_prd_not_found_includes_suggestion` in `src/prd/edit.rs`: verifies PRD-not-found error contains "Suggestion:" and "mr status"
  - Added `test_prd_not_found_error_includes_suggestion` in `src/prd/finalize.rs`: verifies `FinalizeError::PrdNotFound` contains "Suggestion:" and "mr status"
  - Enhanced `test_create_prd_aborts_on_interrupted_signal` in `src/prd/new.rs`: added assertions for "Suggestion:" and "mr new" in interrupted session error
  - Enhanced `test_create_prd_aborts_on_process_failure` in `src/prd/new.rs`: added assertions for "Suggestion:" and "mr new" in failed session error
  - UAT passed: 506 tests, 0 failures (3 new tests added)
- **Constitution Compliance**: No violations.

## 2026-02-07 — uat-001 Verification
- **UAT**: Constitution missing produces suggestion to run mr restore or mr init
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Test: `config::constitution::tests::test_constitution_missing_includes_suggestion` in `src/config/constitution.rs`
  - Verifies that when constitution file is missing, error contains "Suggestion:", "mr restore", and "mr init"
  - All 506 tests passed

## 2026-02-07 — uat-002 Verification
- **UAT**: Malformed config.toml produces suggestion to run mr restore
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Test: `config::tests::test_config_load_malformed_includes_suggestion` in `src/config/mod.rs`
  - Creates a malformed `config.toml`, loads it, and asserts the error message contains `mr restore`
  - All 506 tests passed

## 2026-02-07 — uat-003 Verification
- **UAT**: PRD not found produces suggestion to run mr status
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Test: `prd::edit::tests::test_find_prd_not_found_includes_suggestion` in `src/prd/edit.rs`
  - Test: `prd::finalize::tests::test_prd_not_found_error_includes_suggestion` in `src/prd/finalize.rs`
  - Both tests verify that PRD-not-found errors contain "Suggestion:" and "mr status"
  - All 506 tests passed

## 2026-02-07 — uat-004 Verification
- **UAT**: Invalid PRD format produces suggestion about correct format
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Test: `prd::parser::tests::test_missing_frontmatter_includes_suggestion` in `src/prd/parser.rs`
  - Test: `prd::parser::tests::test_unclosed_frontmatter_includes_suggestion` in `src/prd/parser.rs`
  - Test: `prd::parser::tests::test_invalid_yaml_frontmatter_includes_suggestion` in `src/prd/parser.rs`
  - All three tests verify that invalid PRD format errors contain "Suggestion:" and "mr new"
  - All 506 tests passed

## 2026-02-07 — uat-005 Verification
- **UAT**: Interactive session failure produces suggestion to retry
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Test: `prd::new::tests::test_create_prd_aborts_on_interrupted_signal` in `src/prd/new.rs` — verifies interrupted (Ctrl+C) session error contains "Suggestion:" and "mr new"
  - Test: `prd::new::tests::test_create_prd_aborts_on_process_failure` in `src/prd/new.rs` — verifies process failure error contains "Suggestion:" and "mr new"
  - Both tests confirm interactive session failures produce actionable retry suggestions
  - All 506 tests passed
