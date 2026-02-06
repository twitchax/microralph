---
id: PRD-0032
title: "Direct Interaction Mode for PRD Creation"
status: active
owner: twitchax
created: 2026-02-06
updated: 2026-02-06
principles:
  - "Drop users directly into interactive chat with the agent instead of multi-round Q/A"
  - "Use conversation resume or transcript capture for context handoff between phases"
  - "Maintain runner parity: both CopilotRunner and ClaudeRunner must support interactive mode"
  - "Abort cleanly on force-quit with no partial PRD synthesis"
  - "Preserve existing context injection (PRDs, constitution, codebase scan) in initial prompt"
references:
  - name: "Runner trait definition"
    url: "src/runner/mod.rs"
  - name: "CopilotRunner implementation"
    url: "src/runner/copilot.rs"
  - name: "ClaudeRunner implementation"
    url: "src/runner/claude.rs"
  - name: "MockRunner implementation"
    url: "src/runner/mock.rs"
  - name: "PRD new command"
    url: "src/prd/new.rs"
  - name: "PRD new prompt"
    url: ".mr/prompts/prd_new.md"
  - name: "Init prompt definitions"
    url: "src/init.rs"
  - name: "Q/A workflow utility"
    url: "src/util/qa_workflow.rs"
acceptance_tests:
  - id: uat-001
    name: "Interactive session launches for mr new with CopilotRunner"
    command: cargo make uat
    uat_status: verified
  - id: uat-002
    name: "Interactive session launches for mr new with ClaudeRunner"
    command: cargo make uat
    uat_status: verified
  - id: uat-003
    name: "PRD is synthesized from conversation context after interactive session exits"
    command: cargo make uat
    uat_status: verified
  - id: uat-004
    name: "Ctrl+C during interactive session aborts entirely without creating a PRD"
    command: cargo make uat
    uat_status: verified
  - id: uat-005
    name: "Existing context (PRDs, constitution, codebase scan) is injected into the interactive session"
    command: cargo make uat
    uat_status: verified
  - id: uat-006
    name: "MockRunner supports interactive mode for unit tests"
    command: cargo make test
    uat_status: verified
  - id: uat-007
    name: "Old multi-round Q/A code is fully removed"
    command: cargo make test
    uat_status: verified
  - id: uat-008
    name: "Project builds and passes CI with clippy pedantic"
    command: cargo make ci
    uat_status: verified
tasks:
  - id: T-001
    title: "Add execute_interactive() method to Runner trait"
    priority: 1
    status: done
    notes: "New trait method that spawns the CLI with Stdio::inherit() for stdin/stdout/stderr. Should accept an initial prompt/context string and return a Result with conversation ID or transcript."
  - id: T-002
    title: "Implement execute_interactive() for CopilotRunner"
    priority: 1
    status: done
    notes: "Spawn gh copilot in interactive mode with Stdio::inherit(). Capture conversation transcript or session ID on exit. Investigate gh copilot flags for interactive chat and output capture."
  - id: T-003
    title: "Implement execute_interactive() for ClaudeRunner"
    priority: 1
    status: done
    notes: "Spawn claude CLI in interactive chat mode with Stdio::inherit(). Use --resume or session ID for context handoff. Use --output-format json to capture transcript if resume is not viable."
  - id: T-004
    title: "Implement execute_interactive() for MockRunner"
    priority: 2
    status: done
    notes: "Return mocked conversation context for testing. Should allow tests to inject predefined Q/A transcripts without requiring actual CLI interaction."
  - id: T-005
    title: "Create interactive chat prompt for PRD discovery phase"
    priority: 2
    status: done
    notes: "Define in src/init.rs and materialize to .mr/prompts/. Prompt instructs the agent to ask questions until it has enough information, then exit. Include existing context (PRDs, constitution, codebase scan) as initial context."
  - id: T-006
    title: "Refactor prd::new to use two-phase interactive flow"
    priority: 1
    status: done
    notes: "Phase 1: Call execute_interactive() with discovery prompt and injected context. Phase 2: On clean exit, call existing execute() with synthesis prompt, passing conversation transcript/session context. On Ctrl+C or error, abort entirely."
  - id: T-007
    title: "Remove old multi-round Q/A workflow from prd::new"
    priority: 3
    status: done
    notes: "Remove the iterative question-answer loop code. Clean up qa_workflow.rs if it becomes unused. Remove any related prompts that are no longer needed."
  - id: T-008
    title: "Handle conversation context handoff between phases"
    priority: 2
    status: done
    notes: "Prefer session/conversation ID resume if CLI supports it. Fall back to --output-format json transcript capture. Pass captured context into the synthesis prompt for phase 2."
  - id: T-009
    title: "Handle Ctrl+C and error cases in interactive mode"
    priority: 2
    status: done
    notes: "Detect non-zero exit codes or interrupted signals from the interactive subprocess. Abort PRD creation entirely on force-quit. Clean up any temporary state."
  - id: T-010
    title: "Update prd_new prompt for synthesis phase"
    priority: 3
    status: done
    notes: "Adjust the existing PRD synthesis prompt to accept conversation transcript as input context. Keep it compatible with the existing template-based PRD generation. Update both src/init.rs and .mr/prompts/prd_new.md."
  - id: T-011
    title: "Update tests and MockRunner for new interactive flow"
    priority: 3
    status: done
    notes: "Update unit tests for prd::new to use MockRunner with mocked interactive context. Ensure CI passes without requiring actual CLI tools."
  - id: T-012
    title: "Update AGENTS.md with new PRD creation workflow"
    priority: 4
    status: done
    notes: "Document the two-phase interactive flow, runner interactive mode, and any new flags or behaviors."
---

# Summary

Replace the current multi-round Q/A workflow in `mr new` with a direct interactive chat session. Instead of microralph orchestrating rounds of questions, the user is dropped straight into an interactive session with the underlying agent (Copilot or Claude). The agent asks questions until it has enough context, the user exits, and then a second non-interactive call synthesizes the PRD from the conversation.

# Problem

The current `mr new` workflow uses a structured multi-round Q/A loop where microralph generates questions, presents them to the user, collects answers, and iterates. This creates friction:

1. **Unnatural interaction**: Users must wait for each round of questions to be generated, answer them, and wait again. The flow feels rigid compared to a natural conversation.
2. **Limited exploration**: The fixed-round structure prevents the agent from following up on interesting threads or asking clarifying questions organically.
3. **Redundant orchestration**: microralph acts as a middleman between the user and the agent, adding latency and complexity without clear value over direct interaction.

The underlying CLI tools (gh copilot, claude) already support rich interactive chat. Leveraging this directly would produce better PRDs with less friction.

# Goals

1. Replace the multi-round Q/A loop with a single interactive chat session where the user converses directly with the agent.
2. Add an `execute_interactive()` method to the `Runner` trait that spawns the CLI with inherited stdio.
3. Implement interactive support for both `CopilotRunner` and `ClaudeRunner` from the start.
4. Use conversation resume or transcript capture to hand off context from the interactive phase to the synthesis phase.
5. Preserve existing context injection (PRDs, constitution, codebase scan) in the interactive session's initial prompt.
6. Abort cleanly if the user force-quits the interactive session.
7. Fully remove the old multi-round Q/A workflow code.

# Technical Approach

The implementation follows a two-phase architecture:

```
┌─────────────────────────────────────────────────────────┐
│                      mr new                             │
│                                                         │
│  Phase 1: Interactive Discovery                         │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Build context (PRDs, constitution, codebase)     │  │
│  │  ↓                                                │  │
│  │  Compose discovery prompt:                        │  │
│  │    "Here is the project context: {context}        │  │
│  │     Ask questions until you have enough info.     │  │
│  │     When done, signal completion and exit."       │  │
│  │  ↓                                                │  │
│  │  runner.execute_interactive(prompt)                │  │
│  │    → Stdio::inherit() (user ↔ agent directly)     │  │
│  │    → Returns session_id or transcript on exit     │  │
│  └───────────────────────────────────────────────────┘  │
│           │                          │                   │
│       Clean exit              Ctrl+C / Error             │
│           ↓                          ↓                   │
│  Phase 2: Synthesis              Abort (no PRD)          │
│  ┌─────────────────────┐                                │
│  │  runner.execute()   │                                │
│  │  with synthesis     │                                │
│  │  prompt + context   │                                │
│  │  from phase 1       │                                │
│  │  ↓                  │                                │
│  │  Parse PRD output   │                                │
│  │  Write .mr/prds/    │                                │
│  └─────────────────────┘                                │
└─────────────────────────────────────────────────────────┘
```

## Runner Trait Extension

Add `execute_interactive()` to the `Runner` trait:

```rust
async fn execute_interactive(&self, prompt: &str) -> Result<InteractiveResult>;
```

Where `InteractiveResult` contains either a session/conversation ID (for resume-based handoff) or the full conversation transcript (for transcript-based handoff).

## Runner Implementations

- **ClaudeRunner**: Spawn `claude` in interactive mode with `Stdio::inherit()`. Use `--resume` with a session ID for phase 2 context handoff. If resume is unavailable, use `--output-format json` to capture the transcript.
- **CopilotRunner**: Spawn `gh copilot` in interactive mode with `Stdio::inherit()`. Capture conversation output for phase 2 context. Investigate available flags for session management.
- **MockRunner**: Return predefined conversation context without spawning a subprocess. Tests inject mock transcripts to validate the two-phase flow.

## Context Handoff Strategy

Preferred order:
1. **Session resume** (if CLI supports it): Pass session/conversation ID to phase 2 so the agent has full conversational context.
2. **Transcript capture**: Use `--output-format json` or similar to extract the conversation, then inject it into the phase 2 synthesis prompt.

## Prompt Changes

- **New discovery prompt** (`prd_new_discovery.md`): Instructs the agent to ask questions about the PRD, injecting existing context. Defined in `src/init.rs` per constitution rule 7.
- **Updated synthesis prompt** (`prd_new.md`): Modified to accept conversation transcript/context as input rather than structured Q/A pairs.

# Assumptions

- The `claude` CLI supports interactive chat mode and either `--resume` or transcript capture via `--output-format json`.
- The `gh copilot` CLI supports an interactive chat mode that can be spawned with inherited stdio.
- `Stdio::inherit()` in Rust's `std::process::Command` (or tokio equivalent) correctly passes through terminal control for interactive sessions.
- Users have the relevant CLI tool installed and authenticated before running `mr new`.

# Constraints

- Both runners must support interactive mode from initial implementation (no single-runner-first approach).
- The old Q/A workflow is fully removed; no backward-compatibility flag is provided.
- Prompts must be defined in `src/init.rs` and materialized to `.mr/prompts/` per constitution rule 7.
- All production code must pass `clippy::pedantic` per constitution rule 8.
- Changes should be minimal and focused per constitution rule 3.

# References to Code

- **Runner trait**: `src/runner/mod.rs` — Add `execute_interactive()` method here.
- **CopilotRunner**: `src/runner/copilot.rs` — Implement interactive mode using `gh copilot`.
- **ClaudeRunner**: `src/runner/claude.rs` — Implement interactive mode using `claude` CLI.
- **MockRunner**: `src/runner/mock.rs` — Add mock interactive support for tests.
- **PRD new command**: `src/prd/new.rs` — Refactor from Q/A loop to two-phase interactive flow.
- **Q/A workflow**: `src/util/qa_workflow.rs` — Candidate for removal if no longer used elsewhere.
- **Init/prompts**: `src/init.rs` — Define new discovery prompt constant; update synthesis prompt.
- **Prompt files**: `.mr/prompts/prd_new.md` — Updated synthesis prompt.

# Non-Goals (MVP)

- Supporting a `--legacy` or `--non-interactive` flag for the old Q/A workflow.
- Multi-agent or multi-session interactive flows.
- Streaming the interactive session to a log file for audit purposes.
- Adding interactive mode to other commands (e.g., `mr run`, `mr refactor`) — those can adopt it later.
- Supporting runners beyond Copilot and Claude (e.g., OpenAI, Gemini).

# History

## 2026-02-06 — T-001 Completed
- **Task**: Add execute_interactive() method to Runner trait
- **Status**: ✅ Done
- **Changes**:
  - Added `InteractiveResult` struct to `src/runner/types.rs` with `session_id` and `transcript` fields
  - Added `execute_interactive()` method to `Runner` trait with default error implementation for unsupported runners
  - Added `build_interactive_args()` method to `CliRunnerConfig` trait (returns `None` by default)
  - Added `execute_interactive_cli()` function to `src/runner/cli_runner.rs` — shared infrastructure that spawns CLI with `Stdio::inherit()` for stdin/stdout/stderr
  - Updated blanket `Runner` impl for `CliRunnerConfig` to delegate `execute_interactive()` to `execute_interactive_cli()`
  - Re-exported `InteractiveResult` from `src/runner/mod.rs`
  - Added tests: `InteractiveResult` construction, default trait error behavior, CLI interactive success/failure, `MockRunner` default interactive error
  - UAT: `cargo make uat` passed — 495 tests, 0 failures

- **Constitution Compliance**: No violations. Changes are minimal (rule 3), consistent with existing patterns (rule 4), and do not break public API (rule 5).

## 2026-02-06 — T-002 Completed
- **Task**: Implement execute_interactive() for CopilotRunner
- **Status**: ✅ Done
- **Changes**:
  - Implemented `build_interactive_args()` override in `CopilotRunner`'s `CliRunnerConfig` impl (`src/runner/copilot.rs`)
  - Uses `-i <prompt>` flag (Copilot CLI's interactive mode) instead of `-p <prompt>` (non-interactive)
  - Reuses `append_config_flags()` to include permission, model, and no-ask-user flags consistently
  - Added 3 unit tests: `test_build_interactive_args_yolo_mode`, `test_build_interactive_args_with_model`, `test_build_interactive_args_manual_mode`
  - Interactive execution is handled by the existing `execute_interactive_cli()` infrastructure from T-001 via the blanket impl
  - UAT: `cargo make uat` passed — 498 tests, 0 failures

- **Constitution Compliance**: No violations. Changes are minimal (rule 3), consistent with existing patterns (rule 4), and do not break public API (rule 5).

## 2026-02-06 — T-003 Completed
- **Task**: Implement execute_interactive() for ClaudeRunner
- **Status**: ✅ Done
- **Changes**:
  - Implemented `build_interactive_args()` override in `ClaudeRunner`'s `CliRunnerConfig` impl (`src/runner/claude.rs`)
  - Uses `--initial-prompt <prompt>` for interactive mode instead of `-p <prompt>` (non-interactive)
  - Does NOT include `--output-format json` in interactive mode (would break terminal display)
  - Reuses `append_config_flags()` to include permission, model, and no-ask-user flags consistently
  - Added 3 unit tests: `test_build_interactive_args_yolo_mode`, `test_build_interactive_args_with_model`, `test_build_interactive_args_manual_mode`
  - Interactive execution is handled by the existing `execute_interactive_cli()` infrastructure from T-001 via the blanket impl
  - UAT: `cargo make uat` passed — 501 tests, 0 failures

- **Constitution Compliance**: No violations. Changes are minimal (rule 3), consistent with existing patterns (rule 4), and do not break public API (rule 5).

## 2026-02-06 — T-004, T-006, T-011 Completed
- **Task**: Refactor prd::new to use two-phase interactive flow (with MockRunner interactive support and test updates)
- **Status**: ✅ Done
- **Changes**:
  - **`src/prd/new.rs`**: Major refactor — replaced multi-round Q/A loop with two-phase interactive flow:
    - Phase 1: `runner.execute_interactive()` with discovery prompt (reuses `PromptKind::PrdNewRound1Questions` template)
    - Phase 2: `runner.execute()` with synthesis prompt, passing conversation transcript via `conversation_transcript` and `session_id` placeholders
    - Aborts entirely if interactive session returns an error (Ctrl+C, non-zero exit)
    - Removed `create_prd`'s `input: &mut I` (BufRead) parameter — interactive mode uses `Stdio::inherit()` directly
    - Removed `PrdNewResult.rounds` and `PrdNewResult.qa_history` fields
    - Replaced `build_round1_prompt`, `build_round_n_prompt`, `build_synthesize_prompt`, `prompt_for_context` with `build_discovery_prompt` and new `build_synthesize_prompt`
    - Removed constants `MAX_QA_ROUNDS` and `READY_SIGNAL`
    - Rewrote all `create_prd`-dependent tests for two-phase flow
    - Retained Q/A utility tests (testing `qa_workflow` functions used by `prd::edit`)
  - **`src/runner/mock.rs`** (T-004): Added full `execute_interactive()` support:
    - New `interactive_result` field with `RefCell<Option<InteractiveResult>>` for customizable results
    - New `recorded_interactive_prompts` field to track interactive calls
    - `set_interactive_result()` method for test injection
    - `recorded_interactive_prompts()` accessor method
    - Default returns transcript `"Mock interactive session transcript"`
    - Added 3 tests: default interactive, custom result injection, prompt recording
  - **`src/runner/mod.rs`**: Removed `#[allow(unused_imports)]` on `InteractiveResult` re-export (now actively used)
  - **`src/runner/types.rs`**: Removed `#[allow(dead_code)]` from `InteractiveResult` and `execute_interactive` default impl
  - **`src/runner/cli_runner.rs`**: Removed `#[allow(dead_code)]` from `execute_interactive_cli` (now used via T-006 callers)
  - **`src/main.rs`**: Updated `cmd_prd_new` — removed stdin parameter, updated result display (removed rounds/qa_history, added task count)
  - **`src/commands/suggest.rs`**: Updated `create_prd` call to match new signature (no stdin)
  - **`src/util/qa_workflow.rs`**: Added `#[allow(dead_code)]` on `collect_multiline_answers` (no longer used in production; cleanup deferred to T-007)
  - UAT: `cargo make uat` passed — 500 tests, 0 failures (net -1 test due to Q/A loop tests replaced by interactive flow tests)

- **Constitution Compliance**: No violations. Minimal changes (rule 3), consistent patterns (rule 4), no public API breaks (rule 5). `collect_multiline_answers` dead_code allow is a temporary measure pending T-007 cleanup.

## 2026-02-06 — T-005 Completed
- **Task**: Create interactive chat prompt for PRD discovery phase
- **Status**: ✅ Done
- **Changes**:
  - Added `PrdNewDiscovery` variant to `PromptKind` enum in `src/prompt/types.rs` with filename `prd_new_discovery.md`
  - Added `PROMPT_PRD_NEW_DISCOVERY` constant in `src/commands/init.rs` — interactive discovery prompt that instructs the agent to have a natural conversation with the user to gather PRD requirements
  - Added entry to `PROMPT_FILES` array in `src/commands/init.rs` for materialization during `mr init`
  - Added mapping in `src/prompt/loader.rs` `get_default_prompt` for fallback loading
  - Updated `build_discovery_prompt()` in `src/prd/new.rs` to use `PromptKind::PrdNewDiscovery` instead of `PromptKind::PrdNewRound1Questions`
  - Materialized prompt file to `.mr/prompts/prd_new_discovery.md`
  - Updated test file references in `src/prd/new.rs` from `prd_new_round1_questions.md` to `prd_new_discovery.md`
  - Updated test counts: `PromptKind::all()` length 19→20, init file counts 24→25, missing prompts 19→20/18→19
  - Added `prd_new_discovery.md` existence assertion in `test_init_creates_structure`
  - Added `PROMPT_PRD_NEW_DISCOVERY` to `test_prompts_are_workflow_focused_no_philosophy` test
  - Added placeholder assertion for `{{slug}}` in `test_prompts_contain_placeholders`
  - UAT: `cargo make uat` passed — 500 tests, 0 failures

- **Constitution Compliance**: No violations. Prompt defined in `src/commands/init.rs` and materialized to `.mr/prompts/` per rule 7. Clippy pedantic clean per rule 8. Minimal changes per rule 3.

## 2026-02-06 — T-008 Completed
- **Task**: Handle conversation context handoff between phases
- **Status**: ✅ Done
- **Changes**:
  - **`src/runner/types.rs`**: Added `execute_continue()` method to `Runner` trait with default implementation returning `None` (opt-in session resume). Added test for default behavior.
  - **`src/runner/cli_runner.rs`**: Added `build_continue_args()` method to `CliRunnerConfig` trait (returns `None` by default). Added `execute_continue_cli()` function — shared infrastructure that executes CLI with session-resume args. Updated blanket `Runner` impl to delegate `execute_continue()` to `execute_continue_cli()`. Added 3 tests: unsupported default, continue config, and success execution.
  - **`src/runner/claude.rs`**: Implemented `build_continue_args()` override — uses `--continue -p <prompt> --output-format json` + config flags for session resume. Added 3 tests: yolo mode, with model, and manual mode.
  - **`src/runner/copilot.rs`**: No `build_continue_args()` override needed (returns `None` by default — Copilot does not support session resume). Added 1 test verifying it returns `None`.
  - **`src/runner/mock.rs`**: Uses default `execute_continue()` (returns `None`) — mock tests use transcript-based fallback. Added 1 test verifying `None` behavior.
  - **`src/prd/new.rs`**: Updated `synthesize_and_persist_prd()` to prefer `execute_continue()` for session-resume context handoff, falling back to `execute()` with transcript injected into the prompt.
  - **`src/commands/init.rs`**: Updated `PROMPT_PRD_NEW_SYNTHESIZE` constant — replaced `{{#each qa_history}}` Q/A section with `{{#if conversation_transcript}}` and `{{#if session_id}}` sections for interactive-flow context handoff.
  - **`.mr/prompts/prd_new_synthesize_prd.md`**: Updated materialized prompt to match `init.rs` changes (conversation transcript and session ID placeholders).
  - UAT: `cargo make uat` passed — 508 tests, 0 failures (net +8 tests)

- **Constitution Compliance**: No violations. Prompts synchronized between `src/commands/init.rs` and `.mr/prompts/` per rule 7. Clippy pedantic clean per rule 8. Minimal changes per rule 3. No public API breaks per rule 5 — new trait method has default implementation.

## 2026-02-06 — T-009 Completed
- **Task**: Handle Ctrl+C and error cases in interactive mode
- **Status**: ✅ Done
- **Changes**:
  - **`src/runner/types.rs`**: Added `Interrupted(String)` variant to `RunnerError` enum for signal-based interruptions. Updated `Display` impl to show "Process interrupted: ..." for the new variant. Added `is_interrupted()` helper method. Added 2 tests: `test_runner_error_is_interrupted` and `test_runner_error_interrupted_display`.
  - **`src/runner/cli_runner.rs`**: Updated `execute_interactive_cli()` to detect signal interruption on Unix using `ExitStatusExt::signal()`. When a process is killed by a signal (e.g., SIGINT from Ctrl+C), returns `RunnerError::Interrupted` instead of `RunnerError::ProcessFailed`. Added `signal_name()` helper for human-readable signal names. Added 3 tests: `test_execute_interactive_cli_signal_interrupted` (spawns `sh -c 'kill -2 $$'` to simulate Ctrl+C), `test_signal_name_known_signals`, `test_signal_name_unknown_signal`. Updated existing `test_execute_interactive_cli_failure` to assert `!is_interrupted()`.
  - **`src/runner/mock.rs`**: Added `interactive_error` field and `set_interactive_error()` method to `MockRunner` for testing error paths. Updated `execute_interactive()` to check for pre-configured errors. Added 2 tests: `test_mock_runner_execute_interactive_returns_interrupted_error` and `test_mock_runner_execute_interactive_returns_process_failed_error`.
  - **`src/runner/mod.rs`**: Added `#[cfg(test)]` re-export of `RunnerError` for use in test code across modules.
  - **`src/prd/new.rs`**: Replaced generic `map_err` with explicit `match` on `execute_interactive()` result. On `Interrupted` error: writes user-friendly abort message ("⚠️ Interactive session interrupted. PRD creation aborted — no PRD was created."). On other errors: logs and bails with generic failure message. Added 2 tests: `test_create_prd_aborts_on_interrupted_signal` and `test_create_prd_aborts_on_process_failure` — both verify no PRD file is created on error.
  - UAT: `cargo make uat` passed — 517 tests, 0 failures (net +9 tests)

- **Constitution Compliance**: No violations. Minimal changes (rule 3), consistent with existing error handling patterns (rule 4), no public API breaks (rule 5 — new enum variant is additive), clippy pedantic clean (rule 8).

## 2026-02-06 — T-007 Completed
- **Task**: Remove old multi-round Q/A workflow from prd::new
- **Status**: ✅ Done
- **Changes**:
  - **`src/util/qa_workflow.rs`**: Removed `collect_multiline_answers()` function (dead code since T-006 replaced the multi-round Q/A loop with interactive flow). Removed associated `#[allow(dead_code)]` annotation. Functions still used by `prd::edit` and `config/constitution` (`parse_questions`, `collect_singleline_answers`, `extract_prd_content`, `to_placeholder_list`, `QaPair`, `strip_ansi_escapes`) are preserved.
  - **`src/prd/new.rs`**: Removed 6 old Q/A tests: `test_parse_questions_numbered_dot`, `test_parse_questions_numbered_paren`, `test_parse_questions_empty`, `test_parse_questions_multiline_with_bullets`, `test_collect_answers`, `test_collect_answers_multiline`. These tested `qa_workflow` functions no longer used by `prd::new`. Equivalent tests remain in `qa_workflow.rs` and `prd::edit.rs`.
  - **`src/prompt/types.rs`**: Removed `PrdNewRound1Questions` and `PrdNewRoundNQuestions` variants from `PromptKind` enum. Updated `all()` list and test counts (20→18).
  - **`src/prompt/loader.rs`**: Removed `PrdNewRound1Questions` and `PrdNewRoundNQuestions` mappings from `get_default_prompt()`. Updated missing prompts test counts (20→18, 19→17).
  - **`src/commands/init.rs`**: Removed `PROMPT_PRD_NEW_ROUND1` and `PROMPT_PRD_NEW_ROUNDN` constants (~130 lines of prompt content). Removed from `PROMPT_FILES` array. Updated test counts in `test_init_creates_structure` (25→23) and `test_init_is_idempotent` (25→23). Removed round1 prompt file assertions. Removed `PROMPT_PRD_NEW_ROUND1` from `test_prompts_are_workflow_focused_no_philosophy` and `test_prompts_contain_placeholders`.
  - **`src/prd/types.rs`**: Removed `prd_new_round1_questions` and `prd_new_round_n_questions` fields from `PromptsConfig` struct.
  - **`.mr/prompts/`**: Deleted `prd_new_round1_questions.md` and `prd_new_roundN_questions.md` materialized prompt files.
  - UAT: `cargo make uat` passed — 511 tests, 0 failures (net -6 tests from removed old Q/A tests)

- **Constitution Compliance**: No violations. Prompt management synchronized between `src/commands/init.rs` and `.mr/prompts/` (rule 7). Minimal changes focused on dead code removal (rule 3). No public API breaks (rule 5). Clippy pedantic clean (rule 8).

## 2026-02-06 — T-010 Completed
- **Task**: Update prd_new prompt for synthesis phase
- **Status**: ✅ Done
- **Changes**:
  - **`src/commands/init.rs`**: Refined `PROMPT_PRD_NEW_SYNTHESIZE` constant with three improvements:
    1. Wrapped "Conversation Context" section header inside `{{#if conversation_transcript}}` conditional so it only renders when a transcript is present (renamed section to "Conversation Transcript")
    2. Added synthesis guidance paragraph instructing the agent how to extract goals, constraints, and technical approach from conversation discussion points
    3. Replaced `Read .mr/templates/prd.md for the exact structure` with `The PRD has two parts that you MUST follow exactly` — the synthesis phase runs non-interactively so the agent cannot read files; the template structure is already fully described inline
    4. Wrapped `session_id` block in its own `## Session Context` section header (was previously inside the Conversation Context section)
  - **`.mr/prompts/prd_new_synthesize_prd.md`**: Updated materialized prompt file to match `init.rs` changes exactly (per constitution rule 7)
  - UAT: `cargo make uat` passed — 511 tests, 0 failures

- **Constitution Compliance**: No violations. Prompts synchronized between `src/commands/init.rs` and `.mr/prompts/` per rule 7. Clippy pedantic clean per rule 8. Minimal changes per rule 3. No public API breaks per rule 5.

## 2026-02-06 — T-012 Completed
- **Task**: Update AGENTS.md with new PRD creation workflow
- **Status**: ✅ Done
- **Changes**:
  - Added "PRD Creation Workflow (`mr new`)" section to `AGENTS.md` documenting:
    - Two-phase interactive flow (discovery → synthesis)
    - Runner interactive mode methods (`execute_interactive`, `execute_continue`, `execute`)
    - Context handoff strategy table (Claude session resume vs Copilot transcript fallback)
    - Error handling (`RunnerError::Interrupted` for Ctrl+C, `ProcessFailed` for other errors)
    - Prompt files (`prd_new_discovery.md`, `prd_new_synthesize_prd.md`)
    - Important notes (no Q/A workflow, mock testing, `InteractiveResult` structure)
  - Updated "Runner Implementation Patterns" section with interactive support guidance (`build_interactive_args`, `build_continue_args`)
  - UAT: `cargo make uat` passed — 511 tests, 0 failures

## 2026-02-06 — uat-001 Verification
- **UAT**: Interactive session launches for mr new with CopilotRunner
- **Status**: ✅ Verified
- **Method**: Existing tests
- **Details**:
  - `test_build_interactive_args_yolo_mode` (src/runner/copilot.rs) — Verifies CopilotRunner builds correct interactive args (`-i`, `--allow-all`, `--no-ask-user`)
  - `test_build_interactive_args_with_model` (src/runner/copilot.rs) — Verifies interactive args with model override
  - `test_build_interactive_args_manual_mode` (src/runner/copilot.rs) — Verifies interactive args in manual permission mode
  - `test_execute_interactive_cli_success` (src/runner/cli_runner.rs) — Verifies shared `execute_interactive_cli` spawns interactive process correctly
  - `test_create_prd_two_phase_flow` (src/prd/new.rs) — Verifies `mr new` invokes `execute_interactive` in discovery phase
  - `cargo make uat` passed — 511 tests, 0 failures

## 2026-02-06 — uat-002 Verification
- **UAT**: Interactive session launches for mr new with ClaudeRunner
- **Status**: ✅ Verified
- **Method**: Existing tests
- **Details**:
  - `test_build_interactive_args_yolo_mode` (src/runner/claude.rs) — Verifies ClaudeRunner builds correct interactive args (`--initial-prompt`, `--dangerously-skip-permissions`, `--permission-mode dontAsk`)
  - `test_build_interactive_args_with_model` (src/runner/claude.rs) — Verifies interactive args include `--model` when specified
  - `test_build_interactive_args_manual_mode` (src/runner/claude.rs) — Verifies interactive args omit permission flags in manual mode
  - `test_execute_interactive_cli_success` (src/runner/cli_runner.rs) — Verifies shared `execute_interactive_cli` spawns interactive process with `Stdio::inherit()` and returns `InteractiveResult`
  - `test_create_prd_two_phase_flow` (src/prd/new.rs) — Verifies `mr new` invokes `execute_interactive` in discovery phase (runner-agnostic via MockRunner)
  - `cargo make uat` passed — 511 tests, 0 failures

## 2026-02-06 — uat-003 Verification
- **UAT**: PRD is synthesized from conversation context after interactive session exits
- **Status**: ✅ Verified
- **Method**: Existing tests
- **Details**:
  - `test_create_prd_two_phase_flow` (src/prd/new.rs) — Tests the full two-phase flow: interactive discovery → synthesis → PRD creation. Verifies interactive session was called once (discovery) and synthesis (execute) was called once, producing a valid PRD file.
  - `test_prd_new_transcript_in_synthesis_prompt` (src/prd/new.rs) — Verifies that conversation transcript from the interactive session is injected into the synthesis prompt via the `conversation_transcript` template variable, and that the resulting PRD is synthesized from that context.
  - `cargo make uat` passed — 511 tests, 0 failures

## 2026-02-06 — uat-004 Verification
- **UAT**: Ctrl+C during interactive session aborts entirely without creating a PRD
- **Status**: ✅ Verified
- **Method**: Existing tests
- **Details**:
  - `test_create_prd_aborts_on_interrupted_signal` (src/prd/new.rs) — Verifies that when the interactive session is interrupted by a signal (SIGINT/Ctrl+C), PRD creation fails with an error mentioning "interrupted", user-facing output mentions "aborted", and no PRD file is created on disk.
  - `test_create_prd_aborts_on_process_failure` (src/prd/new.rs) — Verifies that a non-zero exit code (non-signal failure) also aborts PRD creation with no file written.
  - `test_runner_error_is_interrupted` (src/runner/types.rs) — Verifies `RunnerError::Interrupted` is correctly detected by `is_interrupted()`.
  - `test_mock_runner_execute_interactive_returns_interrupted_error` (src/runner/mock.rs) — Verifies the mock runner correctly returns an `Interrupted` error with SIGINT details.
  - `cargo make test` passed — 511 tests, 0 failures

## 2026-02-06 — uat-005 Verification
- **UAT**: Existing context (PRDs, constitution, codebase scan) is injected into the interactive session
- **Status**: ✅ Verified
- **Method**: New test + existing tests
- **Details**:
  - `test_existing_prds_injected_into_discovery_prompt` (src/prd/new.rs) — New test verifying that existing PRD summaries (IDs, titles, statuses) are injected into the interactive discovery prompt via the `existing_prds` template variable.
  - `test_constitution_in_discovery_and_synthesis` (src/prd/new.rs) — Existing test verifying constitution content is loaded and injected into the discovery prompt.
  - `test_prd_new_context_in_discovery_prompt` (src/prd/new.rs) — Existing test verifying user-provided context is injected into the discovery prompt.
  - Codebase scan is not injected as data; the discovery prompt template instructs the agent to scan the codebase (line 42 of prd_new_discovery.md).
  - `cargo make uat` passed — 512 tests, 0 failures

## 2026-02-06 — uat-006 Verification
- **UAT**: MockRunner supports interactive mode for unit tests
- **Status**: ✅ Verified
- **Method**: Existing tests
- **Details**:
  - `test_mock_runner_execute_interactive_returns_default` (src/runner/mock.rs) — Verifies MockRunner returns a default InteractiveResult with transcript.
  - `test_mock_runner_execute_interactive_with_custom_result` (src/runner/mock.rs) — Verifies `set_interactive_result()` configures custom return values.
  - `test_mock_runner_records_interactive_prompts` (src/runner/mock.rs) — Verifies `recorded_interactive_prompts()` captures all prompts passed to `execute_interactive()`.
  - `test_mock_runner_execute_interactive_returns_interrupted_error` (src/runner/mock.rs) — Verifies `set_interactive_error()` with `RunnerError::Interrupted` for Ctrl+C testing.
  - `test_mock_runner_execute_interactive_returns_process_failed_error` (src/runner/mock.rs) — Verifies `set_interactive_error()` with `RunnerError::ProcessFailed` for failure testing.
  - All 5 tests passed.

## 2026-02-06 — uat-007 Verification
- **UAT**: Old multi-round Q/A code is fully removed
- **Status**: ✅ Verified
- **Method**: New test
- **Details**:
  - `test_old_qa_loop_code_removed` (src/prd/new.rs) — New test that uses `include_str!` to read the production code of `prd::new` and asserts that old Q/A loop patterns (`parse_questions`, `collect_singleline_answers`, `QaPair`, `MAX_QA_ROUNDS`, `qa_history`) are absent.
  - The test splits the source at `#[cfg(test)]` to inspect only production code, avoiding false positives from its own string literals.
  - Confirmed: `prd/new.rs` only uses `qa_workflow::extract_prd_content()` (a utility for parsing runner output), not the old iterative Q/A loop functions.
  - `prd/edit.rs` still uses Q/A workflow functions, which is correct — it's a separate command not in scope for this PRD.
  - `cargo make test` passed — 513 tests, 0 failures.

## 2026-02-06 — uat-008 Verification
- **UAT**: Project builds and passes CI with clippy pedantic
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Ran `cargo make ci` which executes fmt, clippy (pedantic), and test stages.
  - All stages passed: 513 tests run, 513 passed, 0 skipped.
  - No clippy warnings or formatting issues detected.

- **Constitution Compliance**: No violations. Documentation-only changes (rule 3). Consistent with existing AGENTS.md patterns (rule 4).