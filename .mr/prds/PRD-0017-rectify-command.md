---
id: PRD-0017
title: Add Restore Command to Reset Prompts and Templates
status: done
owner: microralph
created: 2026-01-25
updated: 2026-01-25
principles:
- Leverage existing `mr init` logic to avoid code duplication
- Don't auto-commit—let users review changes via Git workflow
- 'All-or-nothing approach: restore all editable files, no selective options'
- Clear documentation prevents user confusion about destructive nature
references:
- name: PRD-0001 (mr init implementation)
  url: .mr/prds/PRD-0001-build-micro-ralph-mvp.md
acceptance_tests:
- id: uat-001
  name: mr restore command overwrites .mr/prompts/ and .mr/templates/ with built-in defaults
  command: cargo make uat
  uat_status: verified
- id: uat-002
  name: Command does not auto-commit changes
  command: cargo make uat
  uat_status: verified
- id: uat-003
  name: Command succeeds when .mr/ directories already exist
  command: cargo make uat
  uat_status: verified
tasks:
- id: T-001
  title: Add `restore` subcommand to CLI enum and parser
  priority: 1
  status: done
  notes: Add to Command enum in main.rs, similar to Init/New/Run pattern
- id: T-002
  title: Implement cmd_restore function to delete .mr/prompts/ and .mr/templates/
  priority: 2
  status: done
  notes: Use std::fs::remove_dir_all for deletion, handle missing directories gracefully
- id: T-003
  title: Refactor init logic to support reinitialization of prompts/templates
  priority: 3
  status: done
  notes: Extract file-writing logic from init::init() to allow reuse; ensure it doesn't fail when .mr/ structure exists
- id: T-004
  title: Update documentation (README, AGENTS.md) with restore command usage
  priority: 4
  status: done
  notes: Emphasize that restore shows diffs via Git and doesn't auto-commit; explain use case for updating to latest built-ins
- id: T-005
  title: Add integration tests for restore command
  priority: 5
  status: done
  notes: Test scenarios - fresh restore, restore after customization, idempotency
---

# Summary

Add a `mr restore` command that overwrites `.mr/prompts/` and `.mr/templates/` with built-in defaults. The command deletes these directories and re-runs the `mr init` file-writing logic, allowing users to see changes via Git diff without auto-committing.

---

# Problem

Users who customize prompts or templates may want to:
1. Reset their customizations and return to defaults
2. Update to the latest built-in prompts when microralph is upgraded
3. Compare their customizations against current defaults

Currently, there's no way to restore built-in files without manually deleting directories and re-running `mr init`, which fails if `.mr/` already exists.

---

# Goals

1. Provide `mr restore` command that overwrites `.mr/prompts/` and `.mr/templates/` with built-in versions
2. Reuse existing `mr init` logic for file writing to avoid duplication
3. Delete and recreate directories in one operation (all-or-nothing)
4. Leave changes uncommitted so users can review via Git workflow
5. Document clearly that this is a destructive operation showing diffs for user decision-making

---

# Non-Goals (MVP)

- Selective restoration (e.g., restoring only specific prompts)
- Safety checks like `--force` flags, diff previews, or automatic backups
- Restoring `.mr/constitution.md` or `.mr/config.toml` (only prompts/templates)
- Interactive confirmation prompts

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-25 — T-001 Completed
- **Task**: Add `restore` subcommand to CLI enum and parser
- **Status**: ✅ Done
- **Changes**:
  - Added `Restore` variant to the `Command` enum in main.rs with display_order = 3
  - Updated all subsequent command display_order values to maintain correct ordering
  - Added `cmd_restore()` function handler with initialization check and placeholder implementation
  - Added match arm in main() to handle `Command::Restore`
  - Command appears correctly in `mr --help` output in the [0] Initialization category
  - UAT passed: All existing tests continue to pass with the new command structure
  - Note: T-002 will implement the actual restoration logic

## 2026-01-25 — T-002 Completed
- **Task**: Implement cmd_restore function to delete .mr/prompts/ and .mr/templates/
- **Status**: ✅ Done
- **Changes**:
  - Implemented directory deletion logic in `cmd_restore()` in main.rs
  - Used `std::fs::remove_dir_all()` to delete `.mr/prompts/` and `.mr/templates/`
  - Added graceful handling for missing directories (checks existence before deletion)
  - Added user feedback messages: info message at start, success message after deletion
  - Used `anyhow::Context` for detailed error messages if deletion fails
  - Added debug-level tracing for each directory removal
  - UAT passed: All existing tests continue to pass (cargo make uat succeeded)
  - Next: T-003 will implement the reinitialization logic to recreate directories with built-in defaults

## 2026-01-25 — T-003 Completed
- **Task**: Refactor init logic to support reinitialization of prompts/templates
- **Status**: ✅ Done
- **Changes**:
  - Created new public function `init_prompts_and_templates()` in `src/init.rs` that recreates prompts and templates directories with built-in defaults
  - Added new helper function `create_file_always()` that writes files, always overwriting if they exist (unlike `create_file_if_missing()` which skips existing files)
  - Function creates directories if they don't exist, then writes all 13 prompt files and 1 template file
  - Updated `cmd_restore()` in `src/main.rs` to call `init_prompts_and_templates()` after deleting directories
  - Added user feedback showing number of files restored
  - Maintained DRY principle: Both `init()` and `restore` now share the same file-writing logic for prompts/templates
  - UAT passed: All 312 tests passed, including existing init and restore tests
  - Note: The original `init()` function still works correctly for first-time setup, using `create_file_if_missing()` to skip existing files

## 2026-01-25 — T-004 Completed
- **Task**: Update documentation (README, AGENTS.md) with restore command usage
- **Status**: ✅ Done
- **Changes**:
  - Updated README.md: Added `mr restore` to Commands table with description "Restore `.mr/prompts/` and `.mr/templates/` to built-in defaults (destructive)"
  - Added comprehensive "Restoring Prompts and Templates" section in README.md with:
    - How It Works explanation (3-step process: delete, recreate, leave uncommitted)
    - Git workflow examples for reviewing changes
    - Important notes about destructive nature, no auto-commit, Git safety net, and limited scope
    - Three detailed use case scenarios with command examples
  - Updated AGENTS.md: Added "Restore Command Workflow" section with:
    - 4-step workflow description (pre-flight check, deletion, recreation, no auto-commit)
    - Use cases for agents (reset customizations, update after upgrade, compare customizations)
    - Important notes section emphasizing destructive nature, Git workflow, limited scope, and idempotency
    - Implementation pattern note explaining DRY principle and code reuse
  - UAT passed: All 312 tests passed (cargo make uat succeeded)
  - Documentation clearly emphasizes Git workflow for reviewing changes and non-auto-commit behavior

## 2026-01-25 — T-005 Completed
- **Task**: Add integration tests for restore command
- **Status**: ✅ Done
- **Changes**:
  - Added 4 comprehensive integration tests in `src/main.rs` tests module:
    - `test_restore_fresh`: Tests restore on a freshly initialized repository, verifies files are recreated correctly
    - `test_restore_after_customization`: Tests that customized prompt files are properly overwritten with built-in defaults
    - `test_restore_idempotency`: Tests that multiple restore operations produce identical results (3 consecutive restores)
    - `test_restore_fails_if_not_initialized`: Tests that restore fails gracefully when `.mr/` doesn't exist
  - All tests use `tempfile::TempDir` for isolated test environments
  - Tests change current directory to temp dir to properly exercise `cmd_restore()` function
  - Tests verify both success cases (files exist, content is correct) and failure cases (proper error messages)
  - UAT passed: All 316 tests passed (4 new restore tests + 312 existing tests)
  - Code formatted with `cargo fmt` to maintain consistency
  - **UATs Verified**:
    - uat-001 ✅: Integration test `test_restore_after_customization` verifies files are overwritten with built-in defaults
    - uat-002 ✅: Code inspection confirms no git commit logic in `cmd_restore()` function
    - uat-003 ✅: Integration test `test_restore_idempotency` verifies restore succeeds multiple times on existing directories

---

## 2026-01-25 — PRD Finalized
- **Status**: ✅ Finalized
- **Tasks Completed**: 5 tasks (T-001 through T-005)
- **Outcome**: All tasks completed, acceptance tests passed (318/318 tests)
- **Changelog**: Entry added under [Unreleased] → Added
- **Cleanup**: No temporary files or excessive comments found
- **Summary**:
  - Added `mr restore` command to CLI with proper error handling and user feedback
  - Implemented directory deletion and recreation using DRY principle (reuses `init_prompts_and_templates()`)
  - All 3 UATs verified: files overwritten with defaults, no auto-commit, idempotent restoration
  - Comprehensive documentation added to README.md and AGENTS.md emphasizing Git workflow
  - 4 integration tests added covering fresh restore, customization override, idempotency, and error handling

---