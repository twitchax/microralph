---
id: PRD-0020
title: Add No-Commit Flag
status: done
owner: twitchax
created: 2026-01-26
updated: 2026-01-26
principles:
- CLI flag supersedes config.toml option
- Default behavior unchanged (commit by default)
- Prompt instructions are inverted, not removed, when flag is active
- No behavioral enforcement; only affects prompt text
references:
- name: Handlebars Templating
  url: https://handlebarsjs.com/guide/
acceptance_tests:
- id: uat-001
  name: CLI flag --no-commit is accepted by mr run
  command: cargo run -- run --help | grep -q "no-commit"
  uat_status: verified
- id: uat-002
  name: Config option no_commit is parsed from config.toml
  command: cargo make test
  uat_status: verified
- id: uat-003
  name: Prompts include "Do NOT commit" when flag is active
  command: cargo make uat
  uat_status: verified
- id: uat-004
  name: Default behavior still instructs to commit
  command: cargo make uat
  uat_status: verified
tasks:
- id: T-001
  title: Add no_commit option to config.rs and CLI args
  priority: 1
  status: done
  notes: 'Add `no_commit: Option<bool>` to Config struct with parsing and an `effective_no_commit()` method. Add `--no-commit` flag to Run command in main.rs.'
- id: T-002
  title: Add commit conditional to prompt templates
  priority: 2
  status: done
  notes: Update run_task.md and run_task_finalize.md with `{{#if commit}}` blocks. When commit=true, show existing commit instructions. When commit=false, show "Do NOT commit" instructions.
- id: T-003
  title: Thread no_commit flag through run module
  priority: 3
  status: done
  notes: Add no_commit field to RunConfig, pass through to prompt expansion as `commit` variable (inverted logic).
- id: T-004
  title: Update init.rs embedded prompts
  priority: 4
  status: done
  notes: Per constitution rule 7, update the embedded prompt constants in init.rs to match the new conditional template syntax.
- id: T-005
  title: Add tests for no_commit functionality
  priority: 5
  status: done
  notes: Unit tests for config parsing, effective_no_commit precedence, and prompt expansion with commit variable.
---

# Summary

Add a `--no-commit` CLI flag and corresponding `no_commit` config option that instructs agents to NOT commit changes, allowing users to review edits before manual commit. Default behavior remains unchanged (commit by default).

---

# Problem

Currently, `mr run` and `mr finalize` prompts instruct the agent to commit changes automatically. Users who want to review changes before committing have no way to prevent this instruction. This makes it difficult to audit agent work before it becomes part of git history.

---

# Goals

1. Add `--no-commit` flag to `mr run` command that prevents commit instructions in prompts.
2. Add `no_commit` option to `.mr/config.toml` for persistent configuration.
3. CLI flag supersedes config option (explicit flag wins).
4. When active, prompts say "Do NOT commit" instead of commit instructions.
5. Default behavior unchanged: commit instructions present when flag is not set.

---

# Non-Goals (MVP)

- No rollback or undo functionality
- No automatic staging summary (per user Q/A)
- No enforcement mechanism—flag only affects prompt text, not agent behavior
- Does not affect `mr finalize` initially (though same pattern applies)

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-26 — T-001 Completed
- **Task**: Add no_commit option to config.rs and CLI args
- **Status**: ✅ Done
- **Changes**:
  - Added `no_commit: Option<bool>` field to `Config` struct in `src/config.rs`
  - Added `effective_no_commit()` method to `Config` for precedence logic (CLI > config > default)
  - Updated `DEFAULT_CONFIG` constant to include commented `no_commit` option
  - Added `--no-commit` flag to Run command in `src/main.rs`
  - Updated `cmd_run()` function to accept and compute effective no_commit value
  - Added unit tests for config parsing and effective_no_commit precedence
  - UAT passed: `cargo make uat` succeeded

- **Opportunistic UAT Verification**:
  - uat-001: Verified (`cargo run -- run --help | grep -q "no-commit"` passed)
  - uat-002: Verified (config parsing tests pass, including `test_config_load_with_no_commit`)

---

## 2026-01-26 — T-002 Completed
- **Task**: Add commit conditional to prompt templates
- **Status**: ✅ Done
- **Changes**:
  - Updated `.mr/prompts/run_task.md` with `{{#if commit}}` blocks:
    - Step 9 (commit instruction) now conditionally shows commit or "Do NOT commit" instruction
    - On Success section step 4 conditionally shows commit message format or "Do NOT commit"
    - On Failure section step 4 updated with conditional wording
  - Updated `.mr/prompts/run_task_finalize.md` with `{{#if commit}}` blocks:
    - Step 5 summary format shows commit status or "Skipped" message
    - Step 6 conditionally shows "Commit All Changes" or "Do NOT Commit Changes" section
    - Output format at end shows commit status or "Skipped" message
  - Updated `PROMPT_RUN_TASK` constant in `src/init.rs` to match prompts file
  - Updated `PROMPT_RUN_TASK_FINALIZE` constant in `src/init.rs` to match prompts file
  - UAT passed: `cargo make uat` succeeded (331 tests passed)

- **Constitution Compliance**: No violations. Per rule 7, embedded prompts in `init.rs` were synchronized with `.mr/prompts/` files.

---

## 2026-01-26 — T-003 Completed
- **Task**: Thread no_commit flag through run module
- **Status**: ✅ Done
- **Changes**:
  - Added `no_commit: bool` field to `RunConfig` struct in `src/run.rs`
  - Updated `build_prompt()` function to accept `no_commit` parameter and insert `commit` variable (inverted: `commit = !no_commit`)
  - Updated `cmd_run()` in `src/main.rs` to pass `no_commit` through to `RunConfig`
  - Updated all test constructions of `RunConfig` to include `no_commit: false`
  - Fixed `expand_if_blocks()` in `src/prompt/expand.rs` to support `{{else}}` blocks (bug fix discovered during implementation — T-002 templates used `{{else}}` but the templating engine didn't support it)
  - Added tests for `{{else}}` block expansion in `src/prompt/expand.rs`
  - Added tests for commit variable behavior: `test_build_prompt_commit_true` and `test_build_prompt_commit_false` in `src/run.rs`
  - UAT passed: `cargo make uat` succeeded (335 tests passed)

- **Opportunistic UAT Verification**:
  - uat-003: Verified (`test_build_prompt_commit_false` confirms "Do NOT commit" appears when `no_commit=true`)
  - uat-004: Verified (`test_build_prompt_commit_true` confirms "Commit your work" appears when `no_commit=false`)

- **Constitution Compliance**: No violations. Bug fix to `expand_if_blocks` was necessary to make T-002's templates functional.

---

## 2026-01-26 — T-004 Completed
- **Task**: Update init.rs embedded prompts
- **Status**: ✅ Done
- **Changes**:
  - Verified that `PROMPT_RUN_TASK` constant in `src/init.rs` (lines 598-748) already contains the `{{#if commit}}` conditional blocks
  - Verified that `PROMPT_RUN_TASK_FINALIZE` constant in `src/init.rs` (lines 751-975) already contains the `{{#if commit}}` conditional blocks
  - Both embedded constants match their corresponding `.mr/prompts/` files exactly (synchronized during T-002)
  - No code changes required — T-002 proactively updated init.rs per constitution rule 7
  - UAT passed: `cargo make uat` succeeded (335 tests passed)

- **Constitution Compliance**: No violations. Constitution rule 7 (Prompt Management) was already satisfied by T-002's synchronization of embedded prompts.

---

## 2026-01-26 — T-005 Completed
- **Task**: Add tests for no_commit functionality
- **Status**: ✅ Done
- **Changes**:
  - Reviewed existing test coverage: config parsing (`test_config_load_with_no_commit`), precedence (`test_effective_no_commit_*` tests), and prompt expansion (`test_build_prompt_commit_true/false`) were already implemented in T-001 and T-003
  - Added `test_run_task_with_no_commit_sends_correct_prompt` integration test in `src/run.rs` — verifies that `run_task` with `no_commit=true` sends a prompt containing "Do NOT commit" to the runner
  - Added `test_run_task_without_no_commit_sends_commit_instructions` integration test in `src/run.rs` — verifies that `run_task` with `no_commit=false` sends a prompt containing "Commit your work" to the runner
  - These integration tests validate the complete flow: `RunConfig.no_commit` → `build_prompt()` → prompt expansion → runner receives correct commit instructions
  - UAT passed: `cargo make uat` succeeded (337 tests passed — 2 new tests added)

- **Constitution Compliance**: No violations.

---

## 2026-01-26 — PRD Finalized
- **Status**: ✅ Finalized
- **Tasks Completed**: 5 tasks (T-001 through T-005)
- **Outcome**: All tasks completed, acceptance tests passed (337/337 tests)
- **Changelog**: Entry added under "Changed" — Added `--no-commit` CLI flag and config option
- **Cleanup**: None required; no debug statements or temp files found
- **Summary**:
  - Added `--no-commit` CLI flag to `mr run` command
  - Added `no_commit` option to `.mr/config.toml` for persistent configuration
  - CLI flag supersedes config option with inverted prompt logic (`commit = !no_commit`)
  - Fixed `{{else}}` block support in template engine (bonus bug fix)
  - Updated README.md with `--no-commit` documentation