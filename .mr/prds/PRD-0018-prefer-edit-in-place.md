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
    uat_status: unverified
  - id: uat-002
    name: Verify YAML frontmatter validation emits warnings on malformed PRDs
    command: cargo test -- --nocapture yaml_validation
    uat_status: unverified
tasks:
  - id: T-001
    title: Audit all runner commands (run, edit, etc.) for Rust-side file writes
    priority: 1
    status: done
    notes: Check run.rs, edit.rs, constitution.rs, etc. for any append_history or similar writes
  - id: T-002
    title: Remove Rust code that appends History entries or updates task status
    priority: 2
    status: todo
    notes: Except index regeneration (keep) and init/bootstrap (keep)
  - id: T-003
    title: Add YAML frontmatter validation after agent edits
    priority: 3
    status: todo
    notes: Parse frontmatter, emit warnings if malformed; applies to PRDs and Constitution
  - id: T-004
    title: Update prompts if needed to ensure agents edit files correctly
    priority: 4
    status: todo
    notes: Review run_task.md, run_uat_verify.md, edit_prd.md, edit_constitution.md, and init.rs
  - id: T-005
    title: Run full UAT suite and verify no regressions
    priority: 5
    status: todo
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