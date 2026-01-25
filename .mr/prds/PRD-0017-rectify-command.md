---
id: PRD-0017
title: "Add Restore Command to Reset Prompts and Templates"
status: active
owner: "microralph"
created: 2026-01-25
updated: 2026-01-25

principles:
- Leverage existing `mr init` logic to avoid code duplication
- Don't auto-commit—let users review changes via Git workflow
- "All-or-nothing approach: restore all editable files, no selective options"
- Clear documentation prevents user confusion about destructive nature

references:
- name: PRD-0001 (mr init implementation)
  url: .mr/prds/PRD-0001-build-micro-ralph-mvp.md

acceptance_tests:
- id: uat-001
  name: mr restore command overwrites .mr/prompts/ and .mr/templates/ with built-in defaults
  command: cargo make uat
  uat_status: unverified
- id: uat-002
  name: Command does not auto-commit changes
  command: cargo make uat
  uat_status: unverified
- id: uat-003
  name: Command succeeds when .mr/ directories already exist
  command: cargo make uat
  uat_status: unverified

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
  status: todo
  notes: Extract file-writing logic from init::init() to allow reuse; ensure it doesn't fail when .mr/ structure exists
- id: T-004
  title: Update documentation (README, AGENTS.md) with restore command usage
  priority: 4
  status: todo
  notes: Emphasize that restore shows diffs via Git and doesn't auto-commit; explain use case for updating to latest built-ins
- id: T-005
  title: Add integration tests for restore command
  priority: 5
  status: todo
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

---