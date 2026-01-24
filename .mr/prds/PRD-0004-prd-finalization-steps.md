---
id: PRD-0004
title: PRD Finalization Steps
status: draft
owner: Aaron Roney
created: 2026-01-24
updated: 2026-01-24

tasks:
  - id: T-001
    title: "Add `mr prd finalize <id>` CLI command"
    priority: 1
    status: done
  - id: T-002
    title: "Implement task completion validation (all tasks must be done)"
    priority: 1
    status: done
  - id: T-003
    title: "Run acceptance test verification via finalization prompt"
    priority: 2
    status: done
  - id: T-004
    title: "Create CHANGELOG.md at project root with Keep a Changelog format"
    priority: 2
    status: done
  - id: T-005
    title: "Add changelog entry generation to finalization prompt"
    priority: 2
    status: done
  - id: T-006
    title: "Generate summary report (append to PRD + stdout)"
    priority: 2
    status: done
  - id: T-007
    title: "Update PRD status to done and refresh PRDS.md index"
    priority: 2
    status: done
  - id: T-008
    title: "Update inter-PRD links in index during finalization"
    priority: 3
    status: todo
  - id: T-009
    title: "Add cleanup tasks to finalization prompt (temp files, comments)"
    priority: 3
    status: todo
  - id: T-010
    title: "Append finalization history entry to PRD"
    priority: 2
    status: todo
  - id: T-011
    title: "Update run_task_finalize.md and init.rs default prompt with comprehensive instructions"
    priority: 2
    status: todo
---

# PRD-0004: PRD Finalization Steps

## Summary

Add an explicit `mr prd finalize <id>` command that validates PRD completion, runs final acceptance tests, generates artifacts (changelog entry, summary report), updates the index, and marks the PRD as done.

## Problem

Currently, there is no formal process to finalize a PRD. When all tasks are complete, users must manually update the status, ensure the index reflects completion, and verify that all acceptance criteria have been met. This creates inconsistency and risks leaving PRDs in limbo.

## Goals

1. **Explicit finalization command**: Add `mr prd finalize <id>` as the entry point.
2. **Task completion validation**: Block finalization if any task is not `done`.
3. **Acceptance test verification**: Re-run acceptance criteria as part of finalization.
4. **Changelog generation**: Create/update `CHANGELOG.md` at project root using Keep a Changelog format.
5. **Summary report**: Generate a completion summary, output to stdout and append to PRD history.
6. **Index update**: Mark PRD as `done` in `PRDS.md` and update any inter-PRD links.
7. **Cleanup guidance**: Prompt-driven cleanup of temporary files and extraneous comments.
8. **History entry**: Append a final history entry documenting when and how the PRD was finalized.

## Non-Goals

- Automatic finalization when all tasks are done (future enhancement).
- Integration with external notification systems.
- Rollback or undo of finalization.

## Acceptance Tests

- [ ] `mr prd finalize PRD-XXXX` fails if any task is not `done`.
- [ ] `mr prd finalize PRD-XXXX` fails if any task is `parked` or `wontfix` (must be explicitly resolved/removed via `mr prd edit`).
- [ ] Finalization runs acceptance criteria verification via the prompt.
- [ ] `CHANGELOG.md` is created at project root if it doesn't exist, following Keep a Changelog format.
- [ ] A changelog entry is appended under `[Unreleased]` with the PRD title and summary of completed tasks.
- [ ] Summary report is printed to stdout.
- [ ] Summary report is appended to the PRD as the final history entry.
- [ ] PRD status is updated to `done`.
- [ ] `PRDS.md` index is refreshed to show the PRD as done.
- [ ] Inter-PRD links in the index are updated if applicable.
- [ ] Cleanup instructions are included in the finalization prompt (temp files, comments).
- [ ] A history entry is appended documenting finalization timestamp and outcome.

## Design Notes

### Changelog Format (Keep a Changelog)

The `CHANGELOG.md` at project root should follow [Keep a Changelog](https://keepachangelog.com/en/1.0.0/):

```markdown
# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- PRD-0004: PRD Finalization Steps — Added explicit finalization workflow with validation, changelog generation, and summary reporting.
```

### Changelog Entry Content

Each finalization entry should include:
- PRD ID and title
- Brief summary of what was accomplished (derived from completed tasks or PRD summary)
- Categorized under `Added`, `Changed`, `Fixed`, etc. as appropriate

### Finalization Prompt Enhancement

Update `.mr/prompts/run_task_finalize.md` to instruct the LLM to:
1. Verify all acceptance tests pass
2. Generate changelog entry
3. Create summary report
4. Clean up temporary files and excessive comments
5. Update inter-PRD links in index
6. Append finalization history entry

## History

## 2026-01-24 — T-001 Completed
- **Task**: Add `mr prd finalize <id>` CLI command
- **Status**: ✅ Done
- **Changes**:
  - Added `Finalize` subcommand to `PrdCommand` enum in `src/main.rs`
  - Created new `src/prd_finalize.rs` module with:
    - `PrdFinalizeConfig` struct for configuration
    - `PrdFinalizeResult` struct for results
    - `finalize_prd()` function that finds PRD by ID and validates task completion
    - Unit tests for task completion validation
  - Added `cmd_prd_finalize` handler function in `src/main.rs`
  - Added CLI argument parsing tests for the new command
  - UAT passes: 203/203 tests pass

## 2026-01-24 — T-002 Completed
- **Task**: Implement task completion validation (all tasks must be done)
- **Status**: ✅ Done
- **Changes**:
  - Added `thiserror` dependency to `Cargo.toml` for structured error handling
  - Created `FinalizeError` enum in `src/prd_finalize.rs`:
    - `IncompleteTasks` variant with count and task details
    - `PrdNotFound` variant for missing PRDs
  - Replaced `all_tasks_done()` with `validate_all_tasks_done()` that returns `Result<(), FinalizeError>`
  - Added `get_incomplete_tasks()` helper function
  - Updated `finalize_prd()` to return an error (fail the command) when tasks are incomplete
  - Updated tests to verify error details (count, task IDs, statuses)
  - Added test for multiple incomplete tasks
  - Updated `cmd_prd_finalize` in `src/main.rs` to remove conditional output (now errors propagate)
  - UAT passes: 204/204 tests pass

## 2026-01-24 — T-003 Completed
- **Task**: Run acceptance test verification via finalization prompt
- **Status**: ✅ Done
- **Changes**:
  - Added import for prompt module (`PlaceholderContext`, `PromptKind`, `expand_placeholders`, `load_prompt_with_fallback`) in `src/prd_finalize.rs`
  - Created `build_finalize_prompt()` function that:
    - Loads the `RunTaskFinalize` prompt template
    - Expands `{{prd_id}}` and `{{prd_summary}}` placeholders
  - Updated `finalize_prd()` to actually use the runner:
    - Invokes the finalization prompt via runner after task validation
    - Supports streaming mode for real-time output
    - Returns error if runner reports failure
    - Added tracing for acceptance test verification step
  - Added test `test_build_finalize_prompt()` to verify placeholder expansion
  - UAT passes: 205/205 tests pass

## 2026-01-24 — T-004 Completed
- **Task**: Create CHANGELOG.md at project root with Keep a Changelog format
- **Status**: ✅ Done
- **Changes**:
  - Created new `src/changelog.rs` module with:
    - `CHANGELOG_TEMPLATE` constant following Keep a Changelog format
    - `EnsureChangelogResult` struct to track creation result
    - `ensure_changelog_exists()` function that creates CHANGELOG.md if absent
    - `read_changelog()` function for future changelog entry generation (T-005)
    - Comprehensive unit tests for creation, preservation, and format validation
  - Added `mod changelog` to `src/main.rs`
  - Updated `src/prd_finalize.rs`:
    - Added import for `ensure_changelog_exists`
    - Extended `PrdFinalizeResult` with `changelog_path` and `changelog_created` fields
    - Called `ensure_changelog_exists()` after acceptance test verification
  - Updated `cmd_prd_finalize` in `src/main.rs` to report changelog creation
  - UAT passes: 210/210 tests pass

## 2026-01-24 — T-005 Completed
- **Task**: Add changelog entry generation to finalization prompt
- **Status**: ✅ Done
- **Changes**:
  - Updated `.mr/prompts/run_task_finalize.md` with:
    - New `{{prd_title}}` placeholder in context section
    - New `{{completed_tasks}}` section listing all completed tasks
    - New `{{changelog_content}}` section showing current changelog
    - Detailed changelog entry generation instructions with Keep a Changelog format
    - Category guidelines (Added, Changed, Fixed, Deprecated, Removed, Security)
  - Updated `src/init.rs`:
    - Synced `PROMPT_RUN_TASK_FINALIZE` constant with new prompt content
  - Updated `src/prd_finalize.rs`:
    - Added import for `read_changelog` from changelog module
    - Added `format_completed_tasks()` function to generate bullet list of done tasks
    - Extended `build_finalize_prompt()` to populate new placeholders:
      - `prd_title`: PRD title
      - `completed_tasks`: Formatted list of completed tasks
      - `changelog_content`: Current changelog content (or fallback message)
    - Updated doc comment to reflect T-005 implementation
  - Updated `src/changelog.rs`:
    - Removed `#[allow(dead_code)]` from `read_changelog()` (now used)
  - Added new tests:
    - `test_build_finalize_prompt_with_changelog()`: Verifies changelog content inclusion
    - `test_format_completed_tasks()`: Verifies task formatting
    - `test_format_completed_tasks_empty()`: Verifies empty task handling
  - UAT passes: 213/213 tests pass
## 2026-01-24 — T-006 Completed
- **Task**: Generate summary report (append to PRD + stdout)
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/prd_finalize.rs`:
    - Added imports for `fs`, `io::Write`, `chrono::{Local, Utc}`
    - Added `generate_summary_report()` function that creates formatted report with:
      - Finalization date and timestamp
      - PRD ID and title
      - Count of completed tasks
      - Summary list of all completed tasks
      - Status confirmation
    - Added `append_to_prd()` function to append content to PRD file
    - Extended `PrdFinalizeResult` with `summary_report` field
    - Updated `finalize_prd()` to generate summary and append to PRD
    - Added `#[allow(dead_code)]` to `PrdFinalizeResult` for unused fields
    - Added unit tests:
      - `test_generate_summary_report()`: Verifies report format and content
      - `test_generate_summary_report_no_tasks()`: Verifies empty task handling
      - `test_append_to_prd()`: Verifies file append functionality
      - `test_append_to_prd_preserves_existing()`: Verifies existing content preserved
  - Updated `src/main.rs`:
    - Enhanced `cmd_prd_finalize` to output formatted summary report to stdout
    - Added visual separators and clear sections for finalization output
  - UAT passes: 217/217 tests pass

## 2026-01-24 — T-007 Completed
- **Task**: Update PRD status to done and refresh PRDS.md index
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/prd/parser.rs`:
    - Made `serialize_prd()` public (removed `#[cfg(test)]`)
  - Updated `src/prd/mod.rs`:
    - Exported `serialize_prd` as a public function
  - Updated `src/prd_finalize.rs`:
    - Added imports for `PrdStatus`, `generate_index_from_root`, `serialize_prd`
    - Added `update_prd_status_to_done()` function that:
      - Clones PRD and sets status to `PrdStatus::Done`
      - Serializes and writes the updated PRD file
      - Logs the status update
    - Updated `finalize_prd()` to:
      - Call `update_prd_status_to_done()` after appending summary report
      - Call `generate_index_from_root()` to refresh PRDS.md
    - Updated doc comment to reflect T-007 implementation (removed "Future:")
    - Added unit tests:
      - `test_update_prd_status_to_done()`: Verifies status update
      - `test_update_prd_status_preserves_tasks()`: Verifies tasks and body preserved
  - Updated `src/main.rs`:
    - Added output lines for "PRD Status: Updated to done" and "Index: PRDS.md regenerated"
  - UAT passes: 219/219 tests pass
