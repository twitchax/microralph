---
id: PRD-0027
title: "Swap Bootstrap Default Behavior"
status: active
owner: "Agent"
created: 2026-01-27
updated: 2026-01-27

depends_on:
- PRD-0026

principles:
- Reconstruct mode becomes the default behavior for bootstrap
- New --scaffold flag enables the current default (non-reconstruct) behavior
- All documentation and prompts must be updated to reflect the change
- Tests should be updated to reflect the new default

references:
- name: Bootstrap Reconstruct Workflow (AGENTS.md)
  url: ./AGENTS.md#bootstrap-reconstruct-workflow

acceptance_tests:
- id: uat-001
  name: "Running mr bootstrap without flags uses reconstruct behavior"
  command: cargo make uat
  uat_status: unverified
- id: uat-002
  name: "Running mr bootstrap --scaffold uses scaffold (non-reconstruct) behavior"
  command: cargo make uat
  uat_status: unverified
- id: uat-003
  name: "Help text reflects new default and --scaffold flag"
  command: cargo run -- bootstrap --help
  uat_status: unverified
- id: uat-004
  name: "All prompts and documentation updated to reflect new behavior"
  command: cargo make uat
  uat_status: unverified

tasks:
- id: T-001
  title: "Update CLI flag definitions in main.rs"
  priority: 1
  status: done
  notes: "Remove --reconstruct flag, add --scaffold flag. The default behavior (no flags) should now run reconstruct workflow."

- id: T-002
  title: "Update BootstrapConfig and bootstrap logic"
  priority: 2
  status: done
  notes: "Invert the boolean logic in BootstrapConfig. Default should be reconstruct=true, --scaffold sets it to false."

- id: T-003
  title: "Update bootstrap_reconstruct.md prompt"
  priority: 3
  status: done
  notes: "Remove references to --reconstruct flag, update context to reflect reconstruct is now the default behavior."

- id: T-004
  title: "Update AGENTS.md Bootstrap documentation"
  priority: 4
  status: done
  notes: "Restructure Bootstrap Reconstruct Workflow section to show reconstruct as default, document new --scaffold flag."

- id: T-005
  title: "Update init.rs embedded prompts"
  priority: 5
  status: done
  notes: "Ensure embedded prompt constants match the updated prompt files for consistency per constitution."

- id: T-006
  title: "Update existing tests to reflect new default"
  priority: 6
  status: done
  notes: "Tests that explicitly set reconstruct=true should be updated; tests for scaffold behavior should use --scaffold."

- id: T-007
  title: "Verify all --reconstruct references are removed"
  priority: 7
  status: todo
  notes: "Grep codebase for any remaining references to --reconstruct and update or remove them."

---

# Summary

This PRD swaps the default behavior of `mr bootstrap`. Currently, reconstruct mode (analyzing git history to create PRDs for completed work) requires the `--reconstruct` flag. After this change, reconstruct becomes the default behavior, and a new `--scaffold` flag enables the current default behavior (standard bootstrap without git history analysis).

---

# Problem

The current `mr bootstrap` default behavior is designed for new projects with no history, but `mr init` already serves this purpose. The `--reconstruct` flag, which analyzes git history to create PRDs for existing work, is actually the more common use case for `bootstrap` since it targets existing repositories with development history. Users must currently remember to add `--reconstruct` every time they want the git history analysis, which is counterintuitive.

---

# Goals

1. Make `--reconstruct` behavior the default when running `mr bootstrap` without flags.
2. Introduce `--scaffold` flag to enable the current default (non-reconstruct) behavior.
3. Update all prompts, documentation, and tests to reflect the new behavior.
4. Maintain consistency between embedded prompts in `init.rs` and `.mr/prompts/` files.

---

# Non-Goals (MVP)

- Automatic detection of git history presence to suggest appropriate mode.
- Combined mode that runs both reconstruct and scaffold sequentially.
- Deprecation warnings or transition period for old behavior.

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-27 — T-001 Completed
- **Task**: Update CLI flag definitions in main.rs
- **Status**: ✅ Done
- **Changes**:
  - Replaced `--reconstruct` flag with `--scaffold` flag in Command::Bootstrap enum
  - Updated help text: scaffold is now documented as the flag to skip git history analysis
  - Updated cmd_bootstrap function to accept `scaffold: bool` and set `config.reconstruct = !scaffold`
  - Updated all 4 bootstrap-related tests to use `scaffold` instead of `reconstruct`
  - Renamed test `test_args_parse_bootstrap_with_reconstruct` to `test_args_parse_bootstrap_with_scaffold`
  - Verified help text shows correct new behavior via `cargo run -- bootstrap --help`

- **UAT**: ✅ All 451 tests passed via `cargo make uat`

- **Constitution Compliance**: No violations. Changes were minimal and focused on the flag rename.

---

## 2026-01-27 — T-002 Completed
- **Task**: Update BootstrapConfig and bootstrap logic
- **Status**: ✅ Done
- **Changes**:
  - Updated `BootstrapConfig::new()` in `src/bootstrap.rs` to default `reconstruct: true`
  - Updated module-level doc comment to document reconstruct as the default behavior
  - Updated `reconstruct` field doc comment to note it defaults to true
  - Updated `bootstrap()` function doc comment to document both modes
  - Updated 5 tests to explicitly set `config.reconstruct = false` for scaffold behavior testing:
    - `test_bootstrap_plan_generated`
    - `test_bootstrap_prds_generated`
    - `test_bootstrap_runner_failure_plan`
    - `test_bootstrap_runner_failure_generate`
    - `test_full_bootstrap_flow`
    - `test_bootstrap_creates_constitution`

- **UAT**: ✅ All 451 tests passed via `cargo make uat`

- **Constitution Compliance**: No violations. Changes were minimal and focused on inverting the default boolean.

---

## 2026-01-27 — T-003 Completed
- **Task**: Update bootstrap_reconstruct.md prompt
- **Status**: ✅ Done
- **Changes**:
  - Updated `.mr/prompts/bootstrap_reconstruct.md` Context section (line 9): Changed `mr bootstrap --reconstruct` to `mr bootstrap` and added note that reconstruct mode is the default behavior
  - Updated History template example (line 115): Changed `mr bootstrap --reconstruct` to `mr bootstrap`

- **UAT**: ✅ All 451 tests passed via `cargo make uat`

- **Constitution Compliance**: No violations. Changes were minimal and focused on updating the prompt to reflect new default behavior.

---

## 2026-01-27 — T-004 Completed
- **Task**: Update AGENTS.md Bootstrap documentation
- **Status**: ✅ Done
- **Changes**:
  - Renamed section from "Bootstrap Reconstruct Workflow" to "Bootstrap Workflow"
  - Updated intro to state reconstruct is now the default behavior
  - Updated usage examples: `mr bootstrap` for default (reconstruct), `mr bootstrap --scaffold` for scaffold mode
  - Updated flags table: replaced `--reconstruct` with `--scaffold`, added `--stream` flag
  - Added new "Important Notes" entries explaining default behavior and scaffold mode
  - Updated PRD Format section: changed `bootstrap --reconstruct` reference to `mr bootstrap`

- **UAT**: ✅ All 451 tests passed via `cargo make uat`

- **Constitution Compliance**: No violations. Changes were minimal and focused on documentation updates.

---

## 2026-01-27 — T-005 Completed
- **Task**: Update init.rs embedded prompts
- **Status**: ✅ Done
- **Changes**:
  - Updated `PROMPT_BOOTSTRAP_RECONSTRUCT` constant in `src/init.rs` (line 1731): Changed `mr bootstrap --reconstruct` to `mr bootstrap` and added note that reconstruct mode is the default behavior
  - Updated History template example (line 1837): Changed `mr bootstrap --reconstruct` to `mr bootstrap`

- **UAT**: ✅ All 451 tests passed via `cargo make uat`

- **Constitution Compliance**: No violations. Per constitution rule #7 (Prompt Management), the embedded prompt constant now matches the updated `.mr/prompts/bootstrap_reconstruct.md` file.

---

## 2026-01-27 — T-006 Completed
- **Task**: Update existing tests to reflect new default
- **Status**: ✅ Done
- **Changes**:
  - Updated `src/prd/types.rs:376`: Changed doc comment from "via `--reconstruct`" to "via reconstruct mode"
  - Updated `src/graph.rs:2591`: Changed test comment from "mr bootstrap --reconstruct" to "mr bootstrap"
  - Updated `src/bootstrap.rs`: Removed 7 redundant `config.reconstruct = true` lines from tests since reconstruct is now the default:
    - `test_bootstrap_reconstruct_workflow` (line 708)
    - `test_bootstrap_reconstruct_skips_init_if_exists` (line 738)
    - `test_bootstrap_reconstruct_runner_failure` (line 757)
    - `test_reconstruct_integration_creates_mr_structure` (line 873)
    - `test_reconstruct_integration_idempotent_with_existing_prds` (line 934)
    - `test_reconstruct_integration_with_depends_on_inference` (line 999)
    - `test_reconstruct_integration_full_workflow_with_index_regeneration` (line 1025)
  - Changed `config` from mutable to immutable in these tests, added clarifying comments

- **UAT**: ✅ All 451 tests passed via `cargo make uat`

- **Constitution Compliance**: No violations. Changes were minimal and focused on updating tests to reflect the new default behavior.