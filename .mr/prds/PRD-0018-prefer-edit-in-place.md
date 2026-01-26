---
id: PRD-0018
title: Prefer Edit-in-Place for Agent Actions
status: active
owner: microralph
created: 2026-01-26
updated: 2026-01-26
principles:
  - Agents should own file content; Rust code orchestrates workflows
  - Index regeneration and init/bootstrap remain Rust-controlled
  - Validation should warn, not block execution
  - Existing prompts (run_task.md, run_uat_verify.md) already work correctly
references:
  - name: Constitution Rule on Minimal Changes
    url: .mr/constitution.md
  - name: PRD-0012 Enable Constitution
    url: .mr/prds/enable-constitution.md
acceptance_tests:
  - id: uat-001
    name: Verify all existing UATs pass after refactoring
    command: cargo make uat
    uat_status: verified
  - id: uat-002
    name: Verify YAML frontmatter validation emits warnings on malformed PRDs
    command: cargo test -- --nocapture yaml_validation
    uat_status: verified
tasks:
  - id: T-001
    title: Audit all runner commands (run, edit, etc.) for Rust-side file writes
    priority: 1
    status: done
    notes: Check run.rs, edit.rs, constitution.rs, etc. for any append_history or similar writes
  - id: T-002
    title: Remove Rust code that appends History entries or updates task status
    priority: 2
    status: done
    notes: Except index regeneration (keep) and init/bootstrap (keep)
  - id: T-003
    title: Add YAML frontmatter validation after agent edits
    priority: 3
    status: done
    notes: Parse frontmatter, emit warnings if malformed; applies to PRDs and Constitution
  - id: T-004
    title: Update prompts if needed to ensure agents edit files correctly
    priority: 4
    status: done
    notes: Review run_task.md, run_uat_verify.md, edit_prd.md, edit_constitution.md, and init.rs
  - id: T-005
    title: Run full UAT suite and verify no regressions
    priority: 5
    status: done
    notes: Ensure History entries, task status updates, and UAT verification still work
---

# Summary

Shift file-writing responsibilities from Rust code to agent prompts, ensuring agents edit PRD and Constitution files in place rather than Rust appending content programmatically. This aligns with separation of concerns and gives agents full ownership of file content.

# Problem

Currently, Rust code directly appends History entries, updates task status, and writes other content to PRD files. This violates separation of concerns, reduces flexibility, and makes it harder for agents to maintain consistent formatting. The Rust layer should orchestrate workflows, not manipulate file content beyond initialization and index regeneration.

# Goals

1. Audit all runner commands (run, edit, constitution, etc.) to identify Rust-side file writes
2. Remove Rust code that appends to PRD files (except index regeneration and init/bootstrap)
3. Add YAML frontmatter validation after agent edits to catch malformed files
4. Emit warnings (not errors) when frontmatter is invalid
5. Ensure all existing UATs pass without modification to core prompts

# Non-Goals (MVP)

- Changing index regeneration logic (remains in Rust via `cargo run -- list`)
- Modifying init/bootstrap file copying (remains in Rust via `init.rs`)
- Rewriting run_task.md or run_uat_verify.md prompts (they already work correctly)
- Enforcing strict validation that blocks execution on malformed files

# History

## 2026-01-26 — T-001 Completed
- **Task**: Audit all runner commands (run, edit, etc.) for Rust-side file writes
- **Status**: ✅ Done
- **Changes**:
  - Conducted comprehensive audit of all Rust source files for file write operations
  - Identified two functions in `src/run.rs` that directly manipulate PRD files:
    - `append_opt_out_history()` (lines 474-497): Appends opt-out History entries to PRDs
    - `update_uat_status()` (lines 541-564): Updates UAT status in PRD frontmatter
  - Confirmed `src/prd_edit.rs` writes are acceptable (orchestration, not content generation)
  - Confirmed `src/constitution_edit.rs` already follows desired pattern (agent edits directly)
  - Verified init/bootstrap, index regeneration, and other file writes are out-of-scope (per PRD)
  - Created detailed audit report in session workspace: `/home/twitchax/.copilot/session-state/26032f88-bdae-4d1f-919d-bbb5839248f3/audit-findings.md`
  - All UATs pass: `cargo make uat` exits with code 0

- **Constitution Compliance**: No violations. This is an audit task that only reads and documents existing code structure without making changes.

## 2026-01-26 — T-002 Completed
- **Task**: Remove Rust code that appends History entries or updates task status
- **Status**: ✅ Done
- **Changes**:
  - Removed `append_opt_out_history()` function from `src/run.rs` (previously lines 474-506)
  - Removed `update_uat_status()` function from `src/run.rs` (previously lines 524-572)
  - Removed calls to these functions in UAT verification loop (lines 755 and 784)
  - Updated UAT verification logic to emit warnings instead of updating PRD files
  - Removed unused `std::io::Write` import
  - Updated 4 tests that expected automatic UAT status updates and opt-out history appending:
    - `test_append_opt_out_history` (removed)
    - `test_update_uat_status` (removed)
    - `test_update_uat_status_not_found` (removed)
    - `test_uat_verification_loop_opt_out` (updated expectations)
    - `test_uat_verification_loop_max_iterations` (updated expectations)
    - `test_uat_verification_integration_flow` (updated expectations)
    - `test_uat_verification_history_appending` (updated expectations)
  - All tests now reflect that agents are responsible for updating PRD files
  - All UATs pass: `cargo make uat` exits with code 0 (318 tests passed)

- **Constitution Compliance**: No violations. Changes follow Rule 4 (Minimal Changes) by only removing the identified file manipulation code and updating affected tests. Code follows DRY and SOC principles by separating orchestration (Rust) from content generation (agents).

## 2026-01-26 — T-003 Completed
- **Task**: Add YAML frontmatter validation after agent edits
- **Status**: ✅ Done
- **Changes**:
  - Created new validation module `src/validate.rs` with functions for PRD and Constitution frontmatter validation
  - Implemented `validate_prd_frontmatter()` to parse PRD files and emit warnings if YAML frontmatter is malformed
  - Implemented `validate_constitution_frontmatter()` to handle Constitution files (with or without frontmatter)
  - Added validation calls in 5 key locations where agents edit files:
    - `src/run.rs`: After task execution (line ~413)
    - `src/run.rs`: After UAT verification in loop (line ~642)
    - `src/prd_edit.rs`: After PRD edit with READY_SIGNAL (line ~131)
    - `src/prd_edit.rs`: After PRD edit without questions (line ~160)
    - `src/prd_edit.rs`: After final PRD edit attempt (line ~214)
    - `src/constitution_edit.rs`: After constitution edit completes (line ~120)
  - Added comprehensive unit tests for validation functions (6 test cases covering valid/invalid scenarios)
  - Validation emits warnings via `tracing::warn!()` and `eprintln!()` but does not block execution
  - All UATs pass: `cargo make uat` exits with code 0 (324 tests passed)

- **Constitution Compliance**: No violations. Changes follow Rule 4 (Minimal Changes) by adding only the necessary validation logic. Code follows Rule 2 (Single Source of Truth) by using existing `parse_prd_file()` function. Code follows Rule 3 (Separation of Concerns) by creating a dedicated validation module.

- **Opportunistic UAT Verification**:
  - ✅ **uat-001** (Verify all existing UATs pass after refactoring): Verified via `cargo make uat` - all 324 tests passed
  - ✅ **uat-002** (Verify YAML frontmatter validation emits warnings on malformed PRDs): Verified via `cargo test validate` - 6 validation tests pass and warnings are correctly emitted for malformed frontmatter (visible with `--nocapture`)

## 2026-01-26 — T-004 Completed
- **Task**: Update prompts if needed to ensure agents edit files correctly
- **Status**: ✅ Done
- **Changes**:
  - Reviewed all relevant prompt files: `run_task.md`, `run_uat_verify.md`, `prd_edit.md`, `constitution_edit.md`, `run_task_finalize.md`
  - Verified prompt constants in `src/init.rs` are synchronized with `.mr/prompts/` files
  - Confirmed all prompts correctly instruct agents to edit files in place:
    - `run_task.md`: Updates PRD status, task status, appends History entries
    - `run_uat_verify.md`: Updates UAT status in frontmatter, appends History
    - `prd_edit.md`: Instructs agents to edit PRD directly with READY_TO_APPLY pattern
    - `constitution_edit.md`: Instructs agents to edit constitution directly with EDIT_COMPLETE signal
    - `run_task_finalize.md`: Appends finalization history, updates changelog
  - **No changes needed**: All prompts already correctly implement edit-in-place pattern
  - This aligns with PRD Non-Goals: "Rewriting run_task.md or run_uat_verify.md prompts (they already work correctly)"
  - All UATs pass: `cargo make uat` exits with code 0 (324 tests passed)

- **Constitution Compliance**: No violations. This was an audit/verification task that confirmed existing prompts are correctly implemented. No code changes were made, which aligns with Rule 4 (Minimal Changes).

## 2026-01-26 — T-005 Completed
- **Task**: Run full UAT suite and verify no regressions
- **Status**: ✅ Done
- **Changes**:
  - Executed `cargo make uat` to run the full UAT suite
  - All 324 tests passed with 0 failures and 0 skipped
  - Verified History entries, task status updates, and UAT verification still work correctly
  - No code changes required - this was a verification task confirming the refactoring from T-001 through T-004 is complete and functional

- **Constitution Compliance**: No violations. This was a verification-only task with no code changes, which aligns with Rule 4 (Minimal Changes).