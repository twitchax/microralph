---
id: PRD-0022
title: "Refactor Command for AI-Driven Code Improvements"
status: active
owner: "twitchax"
created: 2026-01-26
updated: 2026-01-26

principles:
- "Each iteration is self-contained: identify one refactor, apply it, verify UATs, commit"
- "Agent autonomy within iteration: agent decides how to handle UAT failures"
- Context overrides constitution focus when provided, but constitution still informs decisions
- Respects `--no-commit` flag from PRD-0020 for commit behavior
- Early termination allowed if agent reports no more impactful refactors

references:
- name: PRD-0020 No-Commit Flag
  url: .mr/prds/PRD-0020-add-no-commit-flag.md
- name: PRD-0015 Suggest Command
  url: .mr/prds/PRD-0015-suggest-command.md

acceptance_tests:
- id: uat-001
  name: Refactor command runs with default 3 iterations
  command: cargo run -- refactor --help | grep -q "max"
  uat_status: verified
- id: uat-002
  name: --context flag passes focus hint to agent
  command: cargo run -- refactor --help | grep -q "context"
  uat_status: verified
- id: uat-003
  name: --path flag constrains scope to specified directory
  command: cargo run -- refactor --help | grep -q "path"
  uat_status: verified
- id: uat-004
  name: --dry-run shows suggested refactors without applying
  command: cargo run -- refactor --help | grep -q "dry-run"
  uat_status: verified
- id: uat-005
  name: Loop stops early when agent reports no more refactors
  command: cargo make uat
  uat_status: verified
- id: uat-006
  name: Each iteration commits separately (respecting --no-commit)
  command: cargo make uat
  uat_status: unverified
- id: uat-007
  name: CI passes after refactor command implementation
  command: cargo make ci
  uat_status: unverified

tasks:
- id: T-001
  title: Add Refactor subcommand to main.rs CLI
  priority: 1
  status: done
  notes: Add Refactor variant to Command enum with --max (default 3), --context, --path, --dry-run, --no-commit, --runner, --model flags.
- id: T-002
  title: Create refactor.rs module with loop logic
  priority: 2
  status: done
  notes: Implement refactor() function that loops up to max iterations, invoking runner each time. Handle early termination when agent signals "no more refactors".
- id: T-003
  title: Create refactor prompt templates
  priority: 3
  status: done
  notes: Add refactor.md prompt to .mr/prompts/ and init.rs. Prompt should instruct agent to identify one impactful refactor, apply it, run UATs, and commit (unless --no-commit). Include constitution context and optional --context override.
- id: T-004
  title: Implement --dry-run mode
  priority: 4
  status: done
  notes: In dry-run mode, runner should analyze and suggest refactors without applying changes. Output suggestions in a readable format.
- id: T-005
  title: Implement --path scope constraint
  priority: 5
  status: done
  notes: When --path is provided, include it in the prompt to constrain agent focus to that directory/file pattern.
- id: T-006
  title: Add tests for refactor command
  priority: 6
  status: done
  notes: Unit tests for CLI parsing, iteration logic, early termination detection, and prompt expansion.
- id: T-007
  title: Update AGENTS.md with refactor workflow documentation
  priority: 7
  status: done
  notes: Document refactor command usage, flags, and expected behavior for future agents.

---

# Summary

Add an `mr refactor` command that runs in a loop (default 3 iterations), instructing the underlying agent to identify one impactful refactor per iteration that improves adherence to the constitution, apply the change, verify UATs pass, and commit. Supports `--context` for focus hints, `--path` for scope constraints, `--dry-run` for previewing suggestions, and respects the `--no-commit` flag.

---

# Problem

Currently, improving code quality requires manual identification of refactoring opportunities or creating full PRDs for cleanup work. There's no automated way to have an AI agent iteratively improve the codebase against established principles (DRY, SOC, etc.) defined in the constitution. Users want a hands-off loop that makes incremental, verified improvements.

---

# Goals

1. Implement `mr refactor` command with iterative loop (default 3 max iterations).
2. Each iteration: identify one refactor → apply it → verify UATs → commit.
3. Support `--context` flag for user-provided focus hints (prioritized over constitution).
4. Support `--path` flag to constrain scope (default: repo-wide).
5. Support `--dry-run` flag to preview suggestions without applying.
6. Allow agent to signal "no more impactful refactors" for early termination.
7. Respect `--no-commit` flag (from [PRD-0020](./PRD-0020-add-no-commit-flag.md)) for commit behavior.
8. Leave UAT failure handling to agent's discretion per iteration.

---

# Non-Goals (MVP)

- No automatic rollback on failures (agent handles within iteration)
- No cross-iteration state or memory (each iteration is independent)
- No interactive approval between iterations
- No metrics or reporting on refactors applied
- No integration with `mr suggest` for refactor discovery

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-26 — T-001 Completed
- **Task**: Add Refactor subcommand to main.rs CLI
- **Status**: ✅ Done
- **Changes**:
  - Added `Refactor` variant to `Command` enum in `src/main.rs` with flags:
    - `--max` (default 3): Maximum refactor iterations
    - `--context`: Focus hint for the agent
    - `--path`: Constrain scope to specific directory/file
    - `--dry-run`: Preview without applying changes
    - `--no-commit`: Skip commit instructions
    - `--runner` (default copilot): Runner to use
    - `--model`: Model override
    - `--stream`: Real-time output streaming
  - Added match arm in `main()` to handle `Command::Refactor`
  - Added stub `cmd_refactor()` function that validates initialization and prints placeholder
  - Added CLI parsing tests: `test_args_parse_refactor_defaults` and `test_args_parse_refactor_with_all_flags`
  - UAT: `cargo make uat` passes (336 tests)
- **Opportunistic UAT Verification**:
  - uat-001 (max flag): ✅ Verified
  - uat-002 (context flag): ✅ Verified
  - uat-003 (path flag): ✅ Verified
  - uat-004 (dry-run flag): ✅ Verified
- **Constitution Compliance**: No violations. Changes are minimal and focused on the task.

---

## 2026-01-26 — T-002, T-003, T-004, T-005, T-006 Completed
- **Task**: Create refactor.rs module with loop logic (and related tasks)
- **Status**: ✅ Done
- **Changes**:
  - Created `src/refactor.rs` module with:
    - `RefactorConfig` struct for configuration
    - `RefactorIterationResult` and `RefactorLoopResult` enums for results
    - `refactor()` function that loops up to max iterations
    - `run_iteration()` for single iteration execution
    - `aggregate_usage()` for token tracking across iterations
    - Early termination detection via `NO-MORE-REFACTORS` signal
    - Preview mode detection via `PREVIEW-COMPLETE` signal
  - Added `Refactor` variant to `PromptKind` enum in `src/prompt/types.rs`
  - Added `PROMPT_REFACTOR` constant to `src/init.rs` with:
    - Constitution context injection
    - User-provided `--context` focus hint support
    - `--path` scope constraint support
    - Preview mode with structured suggestion format
    - `--no-commit` mode support
  - Updated `init()` and `init_prompts_and_templates()` to write `refactor.md`
  - Updated `get_default_prompt()` in loader.rs to handle Refactor
  - Wired up `cmd_refactor()` in main.rs to call `refactor::refactor()`
  - Added unit tests for signal detection and prompt building
  - Updated test assertions for new file counts (21→22 files, 16→17 prompts)
  - UAT: `cargo make uat` passes (340 tests)
- **Constitution Compliance**: No violations. All changes are focused on implementing the refactor command functionality.

---

## 2026-01-26 — T-007 Completed
- **Task**: Update AGENTS.md with refactor workflow documentation
- **Status**: ✅ Done
- **Changes**:
  - Added comprehensive "Refactor Command Workflow" section to AGENTS.md
  - Documented all CLI flags with usage examples and reference table
  - Explained termination signals (NO-MORE-REFACTORS, PREVIEW-COMPLETE)
  - Covered constitution integration and guidance
  - Added important notes on iteration behavior and UAT gating
  - UAT: `cargo make uat` passes (340 tests)
- **Constitution Compliance**: No violations. Documentation-only change.

---

## 2026-01-26 — uat-005 Verification
- **UAT**: Loop stops early when agent reports no more refactors
- **Status**: ✅ Verified
- **Method**: New test
- **Details**:
  - Created `test_refactor_loop_early_termination` in `src/refactor.rs`
  - Test uses MockRunner to return "NO-MORE-REFACTORS" on second iteration
  - Verifies: iterations=2, applied_count=1, early_termination=true
  - UAT: `cargo make uat` passes (341 tests)