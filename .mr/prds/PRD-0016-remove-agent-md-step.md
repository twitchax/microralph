---
id: PRD-0016
title: Remove Automatic AGENTS.md Update Step
status: active
owner: twitchax
created: 2026-01-25
updated: 2026-01-25
principles:
  - Simplify the task execution pipeline by removing redundant automation
  - Empower agents to make holistic documentation updates without artificial constraints
  - Reduce code complexity by eliminating single-purpose modules
references: []
acceptance_tests:
  - id: uat-001
    name: All existing UATs pass after changes
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: Task execution completes without calling update_agents_md
    command: cargo run -- run PRD-0001 --task T-001
    uat_status: unverified
tasks:
  - id: T-001
    title: Remove update_agents_md() call from src/run.rs
    priority: 1
    status: done
    notes: Remove the call after task completion
  - id: T-002
    title: Remove update_agents_md() call from src/prd_new.rs
    priority: 1
    status: done
    notes: Remove the call after PRD creation
  - id: T-003
    title: Remove update_agents_md() call from src/bootstrap.rs
    priority: 1
    status: done
    notes: Remove the call after bootstrapping
  - id: T-004
    title: Delete the agents.rs module if unused
    priority: 2
    status: done
    notes: Remove src/agents.rs and its mod declaration
  - id: T-005
    title: Remove update_agents.md prompt creation from init.rs
    priority: 2
    status: done
    notes: Line 1469 in init.rs creates this prompt - remove it
  - id: T-006
    title: Delete .mr/prompts/update_agents.md file
    priority: 2
    status: done
    notes: Remove the prompt file from the repository
  - id: T-007
    title: Add AGENTS.md update reminder to run_task.md prompt
    priority: 1
    status: done
    notes: Add to numbered task list - "Update AGENTS.md if needed based on changes made"
  - id: T-008
    title: Add AGENTS.md update reminder to run_new_prd.md prompt in init.rs
    priority: 1
    status: done
    notes: Add reminder to the PRD creation prompt
  - id: T-009
    title: Add AGENTS.md update reminder to bootstrap_all.md prompt in init.rs
    priority: 1
    status: done
    notes: Add reminder to the bootstrap prompt
  - id: T-010
    title: Verify all prompts are updated in both init.rs and .md files
    priority: 3
    status: done
    notes: Ensure consistency between init.rs embedded prompts and .mr/prompts/*.md files
---

# Summary

Remove the automatic `update_agents_md()` step that runs after task completion, PRD creation, and bootstrapping. Replace it with a simple reminder in the relevant prompts instructing agents to update AGENTS.md if their changes warrant documentation updates. This simplifies the codebase while giving agents more flexibility to update any part of AGENTS.md, not just an auto-managed section.

# Problem

The current implementation automatically calls `update_agents_md()` after three operations: task execution, PRD creation, and bootstrapping. This approach has several issues:

1. It constrains updates to only an "auto-managed" section of AGENTS.md, preventing holistic documentation updates
2. It adds unnecessary complexity with a dedicated module and prompt file
3. It's unclear why this automation was added as a separate step rather than being part of the agent's general responsibilities
4. It creates a rigid pattern that doesn't adapt well to different types of changes

# Goals

1. Remove all three invocations of `update_agents_md()` from the codebase
2. Delete the `agents.rs` module and associated prompt file
3. Add clear reminders to task execution, PRD creation, and bootstrap prompts instructing agents to update AGENTS.md if needed
4. Give agents the freedom to update any section of AGENTS.md, not just an auto-managed section
5. Maintain all existing UAT pass rates

# Non-Goals (MVP)

- Changing how AGENTS.md is structured or organized
- Adding new documentation automation features
- Modifying other auto-update mechanisms in the codebase

# History

## 2026-01-25 — T-001 Completed
- **Task**: Remove update_agents_md() call from src/run.rs
- **Status**: ✅ Done
- **Changes**:
  - Removed `update_agents_md()` call and associated logic from src/run.rs (lines 439-468)
  - Removed unused import `use crate::agents::{RecentChange, update_agents_md};` from src/run.rs
  - Fixed pre-existing dead code warnings in src/prd_finalize.rs by adding `#[cfg(test)]` to test-only functions
  - Fixed pre-existing unused import warnings in src/prd_finalize.rs by marking test-only imports with `#[cfg(test)]`
  - UAT pass: All tests pass (`cargo make uat` exits with code 0)

## 2026-01-25 — T-002 Completed
- **Task**: Remove update_agents_md() call from src/prd_new.rs
- **Status**: ✅ Done
- **Changes**:
  - Removed `update_agents_md()` call and associated logic from src/prd_new.rs (lines 315-335)
  - Removed unused import `use crate::agents::{RecentChange, update_agents_md};` from src/prd_new.rs (line 12)
  - Added `#[allow(dead_code)]` to `new_content` field in src/agents.rs to suppress dead code warning (field is still used by src/bootstrap.rs, will be cleaned up in T-004)
  - UAT pass: All tests pass (`cargo make uat` exits with code 0)

## 2026-01-25 — T-003 Completed
- **Task**: Remove update_agents_md() call from src/bootstrap.rs
- **Status**: ✅ Done
- **Changes**:
  - Removed `update_agents_md()` call and associated logic from src/bootstrap.rs (lines 154-172)
  - Removed unused import `use crate::agents::{RecentChange, update_agents_md};` from src/bootstrap.rs (line 14)
  - Updated function documentation to remove references to AGENTS.md patching
  - Updated all MockRunner test fixtures to expect 2 runner calls instead of 3 (removed the third call for agents update)
  - Added `#![allow(dead_code)]` to entire src/agents.rs module since all functions are now unused (will be deleted in T-004)
  - UAT pass: All tests pass (`cargo make uat` exits with code 0)

## 2026-01-25 — T-007 Completed
- **Task**: Add AGENTS.md update reminder to run_task.md prompt
- **Status**: ✅ Done
- **Changes**:
  - Added step 7 to "Required Actions" section in `.mr/prompts/run_task.md`: "Update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about"
  - This gives agents explicit guidance to update AGENTS.md when appropriate, replacing the removed automatic update mechanism
  - UAT pass: All tests pass (`cargo make uat` exits with code 0)

## 2026-01-25 — T-008 Completed
- **Task**: Add AGENTS.md update reminder to run_new_prd.md prompt in init.rs
- **Status**: ✅ Done
- **Changes**:
  - Added step 5 to "Required Actions" section in both `src/init.rs` (PROMPT_PRD_NEW_SYNTHESIZE constant, line 530) and `.mr/prompts/prd_new_synthesize_prd.md`: "Update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about"
  - This reminder is shown during PRD creation/synthesis phase, ensuring agents document new patterns introduced by new PRDs
  - UAT pass: All tests pass (`cargo make uat` exits with code 0)

## 2026-01-25 — T-009 Completed
- **Task**: Add AGENTS.md update reminder to bootstrap_all.md prompt in init.rs
- **Status**: ✅ Done
- **Changes**:
  - Added step 4 to "Required Actions" section in both `src/init.rs` (PROMPT_BOOTSTRAP_GENERATE_PRDS constant, lines 332-338) and `.mr/prompts/bootstrap_generate_prds.md`: "Update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about"
  - This reminder is shown during bootstrap PRD generation phase, ensuring agents document new patterns discovered during repository analysis
  - UAT pass: All tests pass (`cargo make uat` exits with code 0)

## 2026-01-25 — T-004 Completed
- **Task**: Delete the agents.rs module if unused
- **Status**: ✅ Done
- **Changes**:
  - Removed `mod agents;` declaration from `src/main.rs` (line 5)
  - Deleted `src/agents.rs` file (531 lines including comprehensive tests)
  - The module was completely unused after T-001, T-002, and T-003 removed all calls to `update_agents_md()`
  - UAT pass: All tests pass (`cargo make uat` exits with code 0)


## 2026-01-25 — T-005 Completed
- **Task**: Remove update_agents.md prompt creation from init.rs
- **Status**: ✅ Done
- **Changes**:
  - Removed `PROMPT_UPDATE_AGENTS` constant definition from `src/init.rs` (lines 1018-1063)
  - Removed `create_file_if_missing()` call for `update_agents.md` from `src/init.rs` (lines 1597-1601)
  - Removed `PromptKind::UpdateAgents` enum variant from `src/prompt/types.rs`
  - Removed `UpdateAgents` references from `PromptKind::filename()`, `PromptKind::all()`, and loader mapping
  - Updated `STARTER_AGENTS` constant to replace auto-managed section markers with a "Manual Updates by Agents" section that explains when agents should update AGENTS.md
  - Removed `test_starter_agents_has_auto_managed_section` test from `src/init.rs`
  - Removed test assertion checking `PROMPT_UPDATE_AGENTS` placeholder from `src/init.rs`
  - Removed test assertion checking for `update_agents.md` file existence from `src/init.rs`
  - Updated `test_init_creates_structure` test: changed expected file count from 19 to 18 (13 prompts instead of 14)
  - Updated `test_init_is_idempotent` test: changed expected file counts from 19 to 18
  - Updated `test_prompt_loader_missing_prompts` test: changed expected prompt counts from 15 to 14 (initially) and 14 to 13 (after creating one)
  - UAT pass: All 267 tests pass (`cargo make uat` exits with code 0)

## 2026-01-25 — T-006 Completed
- **Task**: Delete .mr/prompts/update_agents.md file
- **Status**: ✅ Done
- **Changes**:
  - Deleted `.mr/prompts/update_agents.md` file (941 bytes)
  - This file was previously created during `mr init` but is no longer needed after removal of automatic AGENTS.md update mechanism
  - File contained the prompt template that instructed agents on how to update the auto-managed section of AGENTS.md
  - UAT pass: All 267 tests pass (`cargo make uat` exits with code 0)


## 2026-01-25 — T-010 Completed
- **Task**: Verify all prompts are updated in both init.rs and .md files
- **Status**: ✅ Done
- **Changes**:
  - Updated `PROMPT_RUN_TASK` constant in `src/init.rs` (lines 593-601) to add step 7 about updating AGENTS.md
  - Verified all 14 prompts are consistent between init.rs constants and .mr/prompts/*.md files:
    - init, bootstrap_plan, bootstrap_generate_prds
    - prd_new_round1_questions, prd_new_roundN_questions, prd_new_synthesize_prd
    - run_task, run_task_finalize, run_uat_verify
    - prd_edit, constitution_edit
    - adapt_language, reindex, pick_prd
  - Confirmed AGENTS.md update reminders are present in all three modified prompts (T-007, T-008, T-009)
  - UAT pass: All 267 tests pass (`cargo make uat` exits with code 0)

