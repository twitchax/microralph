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
    uat_status: unverified
  - id: uat-002
    name: "Interactive session launches for mr new with ClaudeRunner"
    command: cargo make uat
    uat_status: unverified
  - id: uat-003
    name: "PRD is synthesized from conversation context after interactive session exits"
    command: cargo make uat
    uat_status: unverified
  - id: uat-004
    name: "Ctrl+C during interactive session aborts entirely without creating a PRD"
    command: cargo make uat
    uat_status: unverified
  - id: uat-005
    name: "Existing context (PRDs, constitution, codebase scan) is injected into the interactive session"
    command: cargo make uat
    uat_status: unverified
  - id: uat-006
    name: "MockRunner supports interactive mode for unit tests"
    command: cargo make test
    uat_status: unverified
  - id: uat-007
    name: "Old multi-round Q/A code is fully removed"
    command: cargo make test
    uat_status: unverified
  - id: uat-008
    name: "Project builds and passes CI with clippy pedantic"
    command: cargo make ci
    uat_status: unverified
tasks:
  - id: T-001
    title: "Add execute_interactive() method to Runner trait"
    priority: 1
    status: done
    notes: "New trait method that spawns the CLI with Stdio::inherit() for stdin/stdout/stderr. Should accept an initial prompt/context string and return a Result with conversation ID or transcript."
  - id: T-002
    title: "Implement execute_interactive() for CopilotRunner"
    priority: 1
    status: todo
    notes: "Spawn gh copilot in interactive mode with Stdio::inherit(). Capture conversation transcript or session ID on exit. Investigate gh copilot flags for interactive chat and output capture."
  - id: T-003
    title: "Implement execute_interactive() for ClaudeRunner"
    priority: 1
    status: todo
    notes: "Spawn claude CLI in interactive chat mode with Stdio::inherit(). Use --resume or session ID for context handoff. Use --output-format json to capture transcript if resume is not viable."
  - id: T-004
    title: "Implement execute_interactive() for MockRunner"
    priority: 2
    status: todo
    notes: "Return mocked conversation context for testing. Should allow tests to inject predefined Q/A transcripts without requiring actual CLI interaction."
  - id: T-005
    title: "Create interactive chat prompt for PRD discovery phase"
    priority: 2
    status: todo
    notes: "Define in src/init.rs and materialize to .mr/prompts/. Prompt instructs the agent to ask questions until it has enough information, then exit. Include existing context (PRDs, constitution, codebase scan) as initial context."
  - id: T-006
    title: "Refactor prd::new to use two-phase interactive flow"
    priority: 1
    status: todo
    notes: "Phase 1: Call execute_interactive() with discovery prompt and injected context. Phase 2: On clean exit, call existing execute() with synthesis prompt, passing conversation transcript/session context. On Ctrl+C or error, abort entirely."
  - id: T-007
    title: "Remove old multi-round Q/A workflow from prd::new"
    priority: 3
    status: todo
    notes: "Remove the iterative question-answer loop code. Clean up qa_workflow.rs if it becomes unused. Remove any related prompts that are no longer needed."
  - id: T-008
    title: "Handle conversation context handoff between phases"
    priority: 2
    status: todo
    notes: "Prefer session/conversation ID resume if CLI supports it. Fall back to --output-format json transcript capture. Pass captured context into the synthesis prompt for phase 2."
  - id: T-009
    title: "Handle Ctrl+C and error cases in interactive mode"
    priority: 2
    status: todo
    notes: "Detect non-zero exit codes or interrupted signals from the interactive subprocess. Abort PRD creation entirely on force-quit. Clean up any temporary state."
  - id: T-010
    title: "Update prd_new prompt for synthesis phase"
    priority: 3
    status: todo
    notes: "Adjust the existing PRD synthesis prompt to accept conversation transcript as input context. Keep it compatible with the existing template-based PRD generation. Update both src/init.rs and .mr/prompts/prd_new.md."
  - id: T-011
    title: "Update tests and MockRunner for new interactive flow"
    priority: 3
    status: todo
    notes: "Update unit tests for prd::new to use MockRunner with mocked interactive context. Ensure CI passes without requiring actual CLI tools."
  - id: T-012
    title: "Update AGENTS.md with new PRD creation workflow"
    priority: 4
    status: todo
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