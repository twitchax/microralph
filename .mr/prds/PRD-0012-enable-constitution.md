---
id: PRD-0012
title: Enable Constitution
status: active
owner: twitchax
created: 2026-01-24
updated: 2026-01-25

principles:
- Constitution is version-controlled and part of project governance
- Constitution enforcement is informational, not blocking
- Constitution can be updated intelligently via LLM

references:
- name: PRD-0001 (bootstrap implementation)
  url: ./PRD-0001-build-micro-ralph-mvp.md
- name: PRD-0003 (upfront context pattern)
  url: ./PRD-0003-prd-new-allows-upfront-context.md
- name: PRD-0004 (finalization workflow)
  url: ./PRD-0004-prd-finalization-steps.md

acceptance_tests:
- id: uat-001
  name: Bootstrap creates initial constitution file
  command: cargo make uat constitution_bootstrap
  uat_status: unverified
- id: uat-002
  name: Constitution contains numbered example rules
  command: cargo make uat constitution_template
  uat_status: unverified
- id: uat-003
  name: prd new reads and respects constitution
  command: cargo make uat constitution_prd_new
  uat_status: unverified
- id: uat-004
  name: prd finalize reads and respects constitution
  command: cargo make uat constitution_prd_finalize
  uat_status: unverified
- id: uat-005
  name: constitution edit command updates via LLM
  command: cargo make uat constitution_edit
  uat_status: unverified
- id: uat-006
  name: Runner logs violations in PRD history
  command: cargo make uat constitution_violation_logging
  uat_status: unverified

tasks:
- id: T-001
  title: Create constitution template with numbered examples
  priority: 1
  status: done
  notes: Create a markdown template with commented-out numbered example rules (e.g., "1. One-off acceptance tests are unacceptable..."). Should be mostly empty but provide clear guidance.
- id: T-002
  title: Emit constitution during bootstrap
  priority: 1
  status: done
  notes: Update bootstrap.rs to create .mr/constitution.md from template. Constitution should be placed alongside configs in .mr/ directory.
- id: T-003
  title: Add constitution edit subcommand
  priority: 2
  status: done
  notes: Add `mr constitution edit <request>` command that invokes the runner/LLM to intelligently update the constitution based on natural language request.
- id: T-004
  title: Load constitution in runner context
  priority: 2
  status: todo
  notes: Add constitution reading function in runner.rs or config.rs. Constitution should be loaded and available to runner prompts.
- id: T-005
  title: Include constitution in prd new prompts
  priority: 2
  status: todo
  notes: Update prd_new_round1_questions.md and prd_new_roundN_questions.md to include constitution content in prompt context.
- id: T-006
  title: Include constitution in prd finalize prompts
  priority: 2
  status: todo
  notes: Update prd_finalize prompts to include constitution content, enabling finalization to respect project governance rules.
- id: T-007
  title: Update runner prompts to log constitution violations
  priority: 3
  status: todo
  notes: Modify runner prompts to instruct LLM to mention and reason about constitution violations in PRD history entries. No programmatic enforcement needed.
- id: T-008
  title: Document constitution feature in README
  priority: 3
  status: todo
  notes: Add section explaining constitution purpose, location, editing workflow, and how it influences PRD creation and execution.

---

# Summary

Add a `.mr/constitution.md` file that defines project-specific governance rules and constraints. The constitution is emitted during `mr bootstrap`, version-controlled, and user-editable. Commands like `mr prd new` and `mr prd finalize` read and respect the constitution, and a new `mr constitution edit <request>` command allows intelligent updates via LLM. Constitution violations are logged in PRD history with reasoning but do not block execution.

---

# Problem

microralph currently has no mechanism to encode project-specific constraints, best practices, or governance rules that should influence PRD creation and execution. For example, a project might require that all acceptance tests be codified (not one-offs), or that certain architectural patterns be followed. Without a constitution, these rules must be manually enforced or repeatedly mentioned, and there's no single source of truth for project governance.

---

# Goals

1. **Bootstrap emits constitution**: `mr bootstrap` creates `.mr/constitution.md` with numbered, commented-out example rules.
2. **Version-controlled governance**: Constitution lives in `.mr/` alongside configs and is committed to version control.
3. **Intelligent editing**: `mr constitution edit <request>` invokes LLM to update constitution based on natural language request.
4. **PRD workflow integration**: `mr prd new` and `mr prd finalize` read constitution and incorporate it into LLM prompts.
5. **Violation logging**: Runner logs constitution violations with reasoning in PRD history; no blocking enforcement.

---

# Non-Goals (MVP)

- Programmatic enforcement (e.g., failing builds on violation)
- Constitution validation or schema beyond markdown format
- Multi-file constitution or imports
- Constitution versioning or rollback beyond git
- Constitution diff or approval workflow
- Support for other constitution formats (TOML, YAML)

---

# History

(Entries appended by `mr run` will go below this line.)

---

## 2026-01-25 — T-001 Completed
- **Task**: Create constitution template with numbered examples
- **Status**: ✅ Done
- **Changes**:
  - Added `CONSTITUTION_TEMPLATE` constant in `src/init.rs` with numbered example rules
  - Template includes commented-out examples (8 numbered rules) covering common governance topics
  - Updated `init()` function to create `.mr/constitution.md` during initialization
  - Added test `test_constitution_template_content` to verify template structure
  - Updated test expectations from 18 to 19 files created
  - Verified that `mr init` creates constitution.md alongside other `.mr/` files
  - UAT pass: All 274 tests passed

---

## 2026-01-25 — T-002 Completed
- **Task**: Emit constitution during bootstrap
- **Status**: ✅ Done
- **Changes**:
  - Constitution already emitted via existing `init()` call in `bootstrap()` (lines 86-92 of bootstrap.rs)
  - Added test `test_bootstrap_creates_constitution` in `src/bootstrap.rs` to explicitly verify constitution creation
  - Test confirms constitution.md is created when bootstrap initializes .mr/ structure
  - Test validates constitution content includes "# Constitution", "## Purpose", and "## Rules" sections
  - UAT pass: All 275 tests passed (1 new test added)
- **UAT Verification Note**: UAT-001 (bootstrap creates constitution) and UAT-002 (constitution template) are now functionally complete and verified by unit tests (`test_bootstrap_creates_constitution` and `test_constitution_template_content`). However, the PRD specifies custom Makefile.toml targets (`cargo make uat constitution_bootstrap` and `cargo make uat constitution_template`) which don't exist yet. These UATs remain `unverified` status until those specific test targets are added to Makefile.toml.

---

## 2026-01-25 — T-003 Completed
- **Task**: Add constitution edit subcommand
- **Status**: ✅ Done
- **Changes**:
  - Created `src/constitution_edit.rs` module implementing `mr constitution edit <request>` command
  - Added `ConstitutionEditConfig` and `ConstitutionEditResult` structs for configuration and results
  - Implemented `edit_constitution()` function with Q/A flow similar to prd_edit
  - Created `.mr/prompts/constitution_edit.md` prompt template with placeholders for request, content, and Q/A history
  - Added `PROMPT_CONSTITUTION_EDIT` constant to `src/init.rs` with embedded default prompt
  - Added `ConstitutionEdit` variant to `PromptKind` enum in `src/prompt/types.rs`
  - Updated `get_default_prompt()` in `src/prompt/loader.rs` to include ConstitutionEdit case
  - Added `Constitution` command with nested `Edit` subcommand to CLI in `src/main.rs`
  - Added `ConstitutionCommand` enum with Edit variant
  - Implemented `cmd_constitution_edit()` function in `src/main.rs` following same pattern as prd_edit
  - Updated `test_prompt_loader_missing_prompts` to expect 15 prompts instead of 14
  - UAT pass: All 277 tests passed
- **Implementation Details**:
  - Command signature: `mr constitution edit "<request>"`
  - Supports optional `--runner` and `--model` flags
  - Uses Q/A flow with max 3 rounds before forcing application
  - Runner signals readiness with `READY_TO_APPLY` marker
  - Extracts constitution content from markdown code blocks
  - Writes updated constitution directly to `.mr/constitution.md`
- **Notes**: UAT-005 (constitution edit command) is functionally implemented but remains `unverified` until custom Makefile.toml target `cargo make uat constitution_edit` is added.
