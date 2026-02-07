---
id: PRD-0034
title: "Fix PRD Edit: Switch to Interactive Mode"
status: active
owner: twitchax
created: 2026-02-07
updated: 2026-02-07
principles:
  - Follow the same interactive pattern established by PRD-0032 for prd new
  - Minimize divergence between prd new and prd edit code paths
  - Remove the multi-round Q/A loop and READY_TO_APPLY signal machinery
  - Remove the unused --stream flag from prd new CLI (dead code since interactive mode inherits stdio)
  - Do not add --stream to prd edit since interactive mode does not use it
acceptance_tests:
  - id: uat-001
    name: "prd edit drops user into interactive session with existing PRD context"
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: "prd edit with --context passes upfront context to the interactive prompt"
    command: cargo make uat
    uat_status: unverified
  - id: uat-003
    name: "prd edit aborts cleanly on Ctrl+C (SIGINT) without corrupting the PRD"
    command: cargo make uat
    uat_status: unverified
  - id: uat-004
    name: "prd edit validates the modified PRD and regenerates the index after interactive session"
    command: cargo make uat
    uat_status: unverified
  - id: uat-005
    name: "stream flag is removed from prd new CLI definition"
    command: cargo make uat
    uat_status: unverified
  - id: uat-006
    name: "existing unit tests for edit and new continue to pass"
    command: cargo make test
    uat_status: unverified
tasks:
  - id: T-001
    title: "Create new interactive prompt template for prd edit (PROMPT_PRD_EDIT_INTERACTIVE)"
    priority: 1
    status: done
    notes: "Define in src/commands/init.rs as an embedded constant. Template should inject existing PRD content, constitution, existing PRDs list, prd_path, and optional user context. Agent reads the PRD, chats with user, and writes updated PRD directly to disk."
  - id: T-002
    title: "Rewrite edit_prd() to use execute_interactive() instead of the Q/A loop"
    priority: 1
    status: done
    notes: "Mirror the pattern from create_prd() in new.rs. Remove the multi-round loop, READY_TO_APPLY signal parsing, qa_history, and collect_singleline_answers. Add interrupt/signal handling like new.rs."
  - id: T-003
    title: "Update PrdEditConfig to replace required request with optional context"
    priority: 1
    status: done
    notes: "Change request field to context: Option<&'a str>. Remove BufRead generic (no longer reading stdin in Rust)."
  - id: T-004
    title: "Update Edit CLI variant in main.rs"
    priority: 1
    status: done
    notes: "Change request from positional required to --context optional. Remove stream flag from New variant."
  - id: T-005
    title: "Register new prompt in PromptKind enum and init logic"
    priority: 2
    status: done
    notes: "Add PrdEditInteractive variant to PromptKind, add file name mapping, and ensure it is materialized to .mr/prompts/ during init."
  - id: T-006
    title: "Remove or update the old prd_edit.md prompt"
    priority: 2
    status: todo
    notes: "The old Q/A-style prompt becomes dead code. Either remove the old PromptKind::PrdEdit variant or repurpose it."
  - id: T-007
    title: "Update tests for the new interactive edit flow"
    priority: 2
    status: todo
    notes: "Update MockRunner tests to use set_interactive_error() for error paths. Add tests for interrupt handling, missing file after interactive session, and successful edit flow."
  - id: T-008
    title: "Update AGENTS.md with new prd edit workflow documentation"
    priority: 3
    status: todo
    notes: "Document that prd edit now uses interactive mode, matching prd new."
---

# Summary

Replace the current multi-round Q/A loop in `mr prd edit` with a single interactive session, matching the pattern established by `mr prd new` in PRD-0032. The agent will be dropped into an interactive chat with the user, read the existing PRD, discuss changes, and write the updated PRD directly to disk. The `request` argument becomes optional `--context`. The unused `--stream` flag is also removed from `prd new`.

# Problem

The current `prd edit` command uses a clunky multi-round Q/A workflow where:

1. The runner executes non-interactively, returning either numbered questions or a `READY_TO_APPLY` signal.
2. The Rust code parses questions, collects single-line answers from stdin, and loops (max 3 rounds).
3. The user experience is poor — single-line answers are limiting, the conversation feels robotic, and there's a hard cap of 3 rounds.

Meanwhile, `prd new` was already migrated to a fully interactive model (PRD-0032) where the user chats naturally with the agent. The edit command should follow the same pattern for consistency and a better user experience.

Additionally, the `--stream` flag on `prd new` is dead code — the parameter is prefixed with `_` and never used since interactive mode inherits stdio directly.

# Goals

1. Replace the Q/A loop in `prd edit` with `execute_interactive()` for a natural conversational experience.
2. Make the edit request optional upfront context (`--context`) instead of a required positional argument.
3. Handle Ctrl+C / SIGINT gracefully, aborting without corrupting the PRD file.
4. Validate the modified PRD and regenerate the index after the interactive session completes.
5. Remove the unused `--stream` flag from `prd new`.
6. Keep the implementation minimal and consistent with `prd new`.

# Technical Approach

The approach mirrors how `prd new` was implemented in PRD-0032.

## Architecture

```
User runs: mr prd edit PRD-0001 --context "add a new task for logging"
                │
                ▼
    ┌───────────────────────┐
    │  Build interactive    │  Inject: existing PRD content, constitution,
    │  prompt               │  existing PRDs list, prd_path, optional context
    └───────────┬───────────┘
                │
                ▼
    ┌───────────────────────┐
    │  execute_interactive  │  Stdio::inherit() — agent chats with user,
    │  (runner)             │  reads PRD, asks questions, writes to disk
    └───────────┬───────────┘
                │
        ┌───────┴───────┐
        │               │
     Success        Interrupted
        │            (SIGINT)
        ▼               ▼
    ┌──────────┐   ┌──────────┐
    │ Validate │   │  Abort   │
    │ PRD file │   │ cleanly  │
    │ Regen    │   └──────────┘
    │ index    │
    └──────────┘
```

## Key Changes

1. **New prompt constant** (`PROMPT_PRD_EDIT_INTERACTIVE` in `src/commands/init.rs`): Instructs the agent to read the current PRD, engage the user in conversation about desired changes, and write the updated PRD directly to disk. Includes `{{prd_content}}`, `{{prd_path}}`, `{{context}}`, `{{constitution}}`, and `{{existing_prds}}` placeholders.

2. **Rewritten `edit_prd()`** (`src/prd/edit.rs`): Remove the loop, `MAX_QA_ROUNDS`, `READY_SIGNAL`, `qa_history`, `parse_questions`, `collect_singleline_answers`, and `extract_prd_content`. Replace with a single call to `runner.execute_interactive()` followed by PRD validation and index regeneration.

3. **Updated `PrdEditConfig`**: Replace `request: &'a str` with `context: Option<&'a str>`. Remove the `I: BufRead` generic from `edit_prd()` since stdin is no longer read by Rust.

4. **Updated CLI definition**: `Edit` variant changes `request` from required positional to `--context` optional flag. `New` variant drops the `stream` flag.

# Assumptions

- The interactive runner (Copilot CLI or Claude CLI) can read, understand, and rewrite an existing PRD file when given its content in the prompt.
- The agent will write the updated PRD to the same path as the original file (overwriting it).
- Users are comfortable with the interactive chat pattern established by `prd new`.

# Constraints

- Must not change the PRD ID during edit (enforced by prompt instructions and post-session validation).
- Must preserve existing History entries in the PRD (enforced by prompt instructions).
- The old Q/A-based prompt (`prd_edit.md`) becomes dead code and should be cleaned up.
- Per constitution rule 7, the new prompt must be defined in `src/commands/init.rs` and materialized to `.mr/prompts/`.

# References to Code

- `src/prd/edit.rs` — Current edit implementation with Q/A loop (primary file to rewrite)
- `src/prd/new.rs` — Interactive prd new implementation (pattern to follow)
- `src/commands/init.rs` — Prompt constants and `PromptKind` enum
- `src/main.rs` — CLI definition for `Edit` and `New` variants
- `src/util/qa_workflow.rs` — Q/A utilities that will no longer be needed by edit
- `src/runner/types.rs` — `Runner` trait with `execute_interactive()`

# Non-Goals (MVP)

- Diffing the PRD before/after the edit to show the user what changed.
- Adding a `--dry-run` mode for edit.
- Supporting non-interactive / batch edit mode as a fallback.
- Removing `qa_workflow.rs` entirely (it may still be used elsewhere or kept for future use).

# History

## 2026-02-07 — T-001 Completed
- **Task**: Create new interactive prompt template for prd edit (PROMPT_PRD_EDIT_INTERACTIVE)
- **Status**: ✅ Done
- **Changes**:
  - Added `PROMPT_PRD_EDIT_INTERACTIVE` constant in `src/commands/init.rs` — mirrors `PROMPT_PRD_NEW_INTERACTIVE` but adapted for editing existing PRDs with `{{prd_path}}`, `{{prd_content}}`, `{{context}}`, `{{constitution}}`, and `{{existing_prds}}` placeholders
  - Added `PrdEditInteractive` variant to `PromptKind` enum in `src/prompt/types.rs` with filename `prd_edit_interactive.md`
  - Added fallback mapping in `src/prompt/loader.rs` (`PromptKind::PrdEditInteractive => init::PROMPT_PRD_EDIT_INTERACTIVE`)
  - Added new prompt to `PROMPT_FILES` array and test arrays in `src/commands/init.rs`
  - Updated all hard-coded prompt count assertions (17→18 in types.rs, 22→23 in init.rs tests, 17→18/16→17 in loader.rs tests)
  - UAT passed: 506 tests, 506 passed, 0 skipped

- **Constitution Compliance**: No violations. Prompt defined in `src/commands/init.rs` as embedded constant per rule 7. All changes minimal per rule 3.

---

## 2026-02-07 — T-002 Completed
- **Task**: Rewrite edit_prd() to use execute_interactive() instead of the Q/A loop
- **Status**: ✅ Done
- **Changes**:
  - Rewrote `src/prd/edit.rs`: removed Q/A loop (`MAX_QA_ROUNDS`, `READY_SIGNAL`, `qa_history`, `BufRead` generic), replaced with single `runner.execute_interactive()` call mirroring `create_prd()` pattern from `new.rs`
  - Updated `PrdEditResult`: removed `rounds` and `qa_history` fields (no longer applicable with interactive mode)
  - Updated `build_edit_prompt()`: now uses `PromptKind::PrdEditInteractive` prompt, injects `prd_path`, `prd_content`, `context`, `constitution`, and `existing_prds` placeholders
  - Added interrupt/signal handling: Ctrl+C aborts cleanly without corrupting the PRD file, matching `new.rs` error handling pattern
  - Updated `cmd_prd_edit()` in `src/main.rs`: removed stdin lock (no longer needed), removed Q/A stats output, added dev container warning
  - Added `#[allow(dead_code)]` to now-unused functions in `qa_workflow.rs` (`extract_prd_content`, `strip_ansi_escapes`, `collect_singleline_answers`), `colors.rs` (`question`), and `copilot.rs` (`strip_usage_stats`) — these are kept for potential future use per PRD non-goals
  - Updated tests: replaced old Q/A-based tests with interactive-mode tests covering success flow, interrupt handling, process failure, context injection, and old Q/A code removal verification
  - UAT passed: 504 tests, 504 passed, 0 skipped

- **Constitution Compliance**: No violations. All changes minimal per rule 3. Consistent with `new.rs` pattern per rule 4.

---

## 2026-02-07 — T-003 Completed
- **Task**: Update PrdEditConfig to replace required request with optional context
- **Status**: ✅ Done
- **Changes**:
  - Changed `PrdEditConfig.request: &'a str` to `context: Option<&'a str>` in `src/prd/edit.rs`
  - Updated `build_edit_prompt()` to unwrap optional context with `unwrap_or("")`
  - Changed CLI `Edit` variant in `src/main.rs`: `request` positional arg → `--context` optional flag
  - Updated `cmd_prd_edit()` signature: `request: &str` → `context: Option<&str>`
  - Updated match arm dispatch to pass `context.as_deref()`
  - Updated all test configs from `request: "..."` to `context: Some("...")`
  - Updated `test_args_parse_edit` to test without required positional arg and assert `context.is_none()`
  - UAT passed: 504 tests, 504 passed, 0 skipped

- **Constitution Compliance**: No violations. Changes minimal per rule 3. Consistent with `PrdNewConfig` pattern per rule 4.

---

## 2026-02-07 — T-004 Completed
- **Task**: Update Edit CLI variant in main.rs
- **Status**: ✅ Done
- **Changes**:
  - Removed `stream: bool` field from the `New` CLI variant in `src/main.rs` (dead code since interactive mode inherits stdio)
  - Removed `stream` from the `New` match arm destructuring and `tracing::info!` call
  - Removed `_stream: bool` parameter from `cmd_prd_new()` function signature
  - Updated `test_args_parse_prd_new` and `test_args_parse_prd_new_with_model` tests to no longer destructure or assert `stream`
  - Note: `Edit` variant already had `--context` as optional (completed in T-003)
  - UAT passed: 504 tests, 504 passed, 0 skipped

- **Constitution Compliance**: No violations. Changes minimal per rule 3. Dead code removed per rule 6 (root cause).

---

## 2026-02-07 — T-005 Completed
- **Task**: Register new prompt in PromptKind enum and init logic
- **Status**: ✅ Done (no-op — already completed as part of T-001)
- **Changes**:
  - Verified that all T-005 requirements were already implemented during T-001:
    - `PrdEditInteractive` variant exists in `PromptKind` enum (`src/prompt/types.rs:41`)
    - Filename mapping `"prd_edit_interactive.md"` exists (`src/prompt/types.rs:85`)
    - Included in `PromptKind::all()` (`src/prompt/types.rs:109`)
    - Fallback mapping in `src/prompt/loader.rs:110` (`PromptKind::PrdEditInteractive => init::PROMPT_PRD_EDIT_INTERACTIVE`)
    - Entry in `PROMPT_FILES` array in `src/commands/init.rs:1982`
    - `PROMPT_PRD_EDIT_INTERACTIVE` constant defined in `src/commands/init.rs:1061`
  - No code changes needed — task was a no-op
  - UAT passed: 504 tests, 504 passed, 0 skipped

- **Constitution Compliance**: No violations. No changes made (rule 3 — minimal changes).
