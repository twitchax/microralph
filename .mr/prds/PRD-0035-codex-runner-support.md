---
id: PRD-0035
title: "Codex Runner Support"
status: active
owner: twitchax
created: 2026-02-08
updated: 2026-02-08
principles:
  - "Mirror existing runner patterns (CopilotRunner, ClaudeRunner) for consistency"
  - "Implement full CliRunnerConfig trait for automatic Runner blanket implementation"
  - "Default to autonomous mode (--full-auto) for non-interactive execution"
  - "Parse token usage from JSON output when available"
  - "No new dependencies required — reuse existing serde_json, regex, and which crates"
references:
  - name: "OpenAI Codex CLI Reference"
    url: "https://developers.openai.com/codex/cli/reference"
  - name: "Codex CLI npm package"
    url: "https://www.npmjs.com/package/@openai/codex"
  - name: "PRD-0013 Add Claude CLI Runner"
    url: ".mr/prds/PRD-0013-add-claude-cli-runner.md"
acceptance_tests:
  - id: uat-001
    name: "Project builds cleanly with codex runner module"
    command: cargo build
    uat_status: verified
  - id: uat-002
    name: "All existing tests pass with new runner added"
    command: cargo make test
    uat_status: verified
  - id: uat-003
    name: "Clippy pedantic passes with no new warnings"
    command: cargo make clippy
    uat_status: verified
  - id: uat-004
    name: "Runner is selectable via --runner codex flag"
    command: cargo run -- --help
    uat_status: verified
  - id: uat-005
    name: "CodexRunner build_args produces correct CLI invocation"
    command: cargo make test
    uat_status: verified
  - id: uat-006
    name: "Token usage parsing extracts input/output tokens from JSON"
    command: cargo make test
    uat_status: verified
  - id: uat-007
    name: "Interactive mode produces correct args for codex TUI"
    command: cargo make test
    uat_status: unverified
  - id: uat-008
    name: "Full CI pipeline passes"
    command: cargo make ci
    uat_status: unverified
tasks:
  - id: T-001
    title: "Create CodexPermissionMode enum and CodexConfig struct"
    priority: 1
    status: done
    notes: "Mirror ClaudePermissionMode pattern. Default Yolo uses --full-auto. Add model and codex_path fields."
  - id: T-002
    title: "Create CodexRunner struct with constructors"
    priority: 1
    status: done
    notes: "Implement new(), with_model(), and test-only constructors. Follow CopilotRunner/ClaudeRunner patterns."
  - id: T-003
    title: "Implement append_config_flags for CodexRunner"
    priority: 1
    status: done
    notes: "Handle --full-auto (Yolo mode), --model flag, and any codex-specific flags."
  - id: T-004
    title: "Implement CliRunnerConfig trait for CodexRunner"
    priority: 1
    status: done
    notes: "Implement binary_path, build_args (codex exec -p prompt --json --full-auto), parse_usage (JSON), post_process_output (extract result from JSON), format_display_parts, build_interactive_args (codex prompt, no exec subcommand)."
  - id: T-005
    title: "Implement token usage parsing from Codex JSON output"
    priority: 2
    status: done
    notes: "Parse JSON output for usage.input_tokens and usage.output_tokens fields. Return TokenUsageInfo."
  - id: T-006
    title: "Implement post_process_output to extract result from JSON"
    priority: 2
    status: done
    notes: "Similar to ClaudeRunner's extract_result_from_json. Extract meaningful text from Codex JSON response."
  - id: T-007
    title: "Export CodexRunner in src/runner/mod.rs"
    priority: 2
    status: done
    notes: "Add mod codex and pub use codex::CodexRunner."
  - id: T-008
    title: "Add codex arm to create_runner() in main.rs"
    priority: 2
    status: done
    notes: "Add match arm for 'codex' in create_runner(). Update error message to list codex as supported runner."
  - id: T-009
    title: "Update --runner CLI argument help text and defaults"
    priority: 3
    status: done
    notes: "Update help strings to mention codex as a supported runner option."
  - id: T-010
    title: "Write unit tests for build_args and parse_usage"
    priority: 2
    status: done
    notes: "Test non-interactive args, interactive args, model override, usage parsing from sample JSON, and post-processing."
  - id: T-011
    title: "Update AGENTS.md with CodexRunner documentation"
    priority: 3
    status: done
    notes: "Document Codex runner patterns, CLI flags, and any differences from existing runners."
---

# Summary

Add a Codex runner to microralph, enabling users to use OpenAI's Codex CLI as a backend agent alongside the existing Copilot and Claude runners. The Codex runner follows the same `CliRunnerConfig` pattern established by `CopilotRunner` and `ClaudeRunner`, providing full feature parity: non-interactive execution, streaming, and interactive mode.

# Problem

microralph currently supports two runners: Copilot CLI and Claude CLI. Users who prefer or have access to OpenAI's Codex CLI cannot use it with microralph. Adding Codex as a third runner option broadens the tool's flexibility and lets users choose the best agent for their workflow.

# Goals

1. Implement a `CodexRunner` that fully implements the `CliRunnerConfig` trait, gaining automatic `Runner` implementation via the blanket impl.
2. Support non-interactive execution via `codex exec "<prompt>"` with JSON output for token usage parsing.
3. Support interactive mode via `codex "<prompt>"` with stdio inheritance.
4. Support streaming execution with real-time output.
5. Parse token usage from Codex JSON output (`input_tokens`, `output_tokens`).
6. Make the runner selectable via `--runner codex` across all commands.
7. Follow all existing patterns and conventions — no new dependencies or architectural changes.

# Technical Approach

The implementation mirrors the `ClaudeRunner` most closely, since both use JSON output for structured data extraction.

## Architecture

```
┌─────────────────────────────────────────────────┐
│                  CliRunnerConfig                 │
│  (trait: binary_path, build_args, parse_usage,  │
│   post_process_output, build_interactive_args)  │
└──────────────────┬──────────────────────────────┘
                   │ blanket impl
                   ▼
┌──────────┐  ┌──────────┐  ┌──────────┐
│ Copilot  │  │  Claude  │  │  Codex   │  ← NEW
│  Runner  │  │  Runner  │  │  Runner  │
└──────────┘  └──────────┘  └──────────┘
```

## CLI Invocation Mapping

| Mode             | Codex CLI Command                                         |
|------------------|-----------------------------------------------------------|
| Non-interactive  | `codex exec --full-auto --json --model <m> "<prompt>"`    |
| Interactive      | `codex --full-auto --model <m> "<prompt>"`                |
| Streaming        | Same as non-interactive (handled by `CliRunnerConfig`)    |

## Key Design Decisions

- **Non-interactive uses `codex exec`**: The `exec` subcommand runs headlessly, suitable for programmatic use.
- **`--json` flag for structured output**: Enables token usage parsing from the `usage` object in JSON responses.
- **`--full-auto` for Yolo mode**: This is Codex's equivalent of Claude's `--dangerously-skip-permissions`. It auto-approves actions within the sandbox.
- **Interactive mode omits `exec`**: Running `codex "<prompt>"` launches the TUI for direct user interaction.
- **Post-process extracts result from JSON**: Similar to `ClaudeRunner`, the `post_process_output` method extracts the meaningful text result from the JSON envelope.

## Token Usage Parsing

Codex JSON output includes usage data in this format:

```json
{
  "usage": {
    "input_tokens": 26549,
    "output_tokens": 1590
  }
}
```

The parser will extract `input_tokens` and `output_tokens`, compute `total_tokens` as their sum, and return a `TokenUsageInfo`.

# Assumptions

- The Codex CLI binary is named `codex` and is available on the user's PATH.
- The `codex exec --json` output format includes a `usage` object with `input_tokens` and `output_tokens` fields.
- The `codex exec --json` output includes a `result` field (or similar) containing the text response, similar to Claude's JSON output.
- The `--full-auto` flag is sufficient for autonomous operation in microralph's use cases.

# Constraints

- Must not introduce new crate dependencies.
- Must pass `clippy::pedantic` linting.
- Must follow the `CliRunnerConfig` trait pattern — no custom `Runner` impl.
- Permission mode variants beyond `Yolo` are test-only (`#[cfg(test)]`), matching existing runners.

# References to Code

- `src/runner/cli_runner.rs` — `CliRunnerConfig` trait and blanket `Runner` impl
- `src/runner/claude.rs` — Primary pattern to follow (JSON output, structured parsing)
- `src/runner/copilot.rs` — Secondary pattern reference (text-based parsing)
- `src/runner/types.rs` — `TokenUsageInfo`, `RunnerOutput`, `Runner` trait
- `src/runner/mod.rs` — Module exports
- `src/main.rs` — `create_runner()` factory function (line ~583)

# Non-Goals (MVP)

- Supporting Codex-specific features like `--search`, `--image`, or `--add-dir`.
- Supporting Codex profiles (`--profile`).
- Supporting sandbox mode configuration beyond `--full-auto`.
- Auto-detection or installation of the Codex CLI.
- Codex-specific error messages or recovery (standard `CliRunnerConfig` error handling suffices).

# History

## 2026-02-08 — T-001 through T-008 Completed
- **Task**: Create CodexPermissionMode enum, CodexConfig struct, CodexRunner with full CliRunnerConfig impl, export, and integration
- **Status**: ✅ Done
- **Changes**:
  - Created `src/runner/codex.rs` with `CodexPermissionMode` enum (Yolo/Manual), `CodexConfig` struct, and `CodexRunner` struct
  - Implemented `CliRunnerConfig` trait: `build_args` (exec subcommand + --full-auto), `parse_usage` (JSON), `post_process_output`, `format_display_parts`, `build_interactive_args`
  - Added constructors: `new()`, `with_model()`, `with_config()` (test-only), `append_config_flags()`
  - Token usage parsing from JSON `usage.input_tokens`/`output_tokens` fields
  - Added `mod codex` and `pub use codex::CodexRunner` in `src/runner/mod.rs`
  - Added `"codex"` match arm in `create_runner()` in `src/main.rs`
  - Updated error message to list codex as supported runner
  - Added unit tests for config defaults, builder pattern, and with_config
  - UAT passed: `cargo make uat` ✅
- **Constitution Compliance**: No violations. Followed existing ClaudeRunner patterns exactly (Rule 4), minimal changes (Rule 3), clippy::pedantic enabled (Rule 8).

## 2026-02-08 — T-010 Completed
- **Task**: Write unit tests for build_args and parse_usage
- **Status**: ✅ Done
- **Changes**:
  - Added 28 comprehensive unit tests to `src/runner/codex.rs` (replacing the initial 3 config-only tests)
  - Tests cover: `build_args` (yolo mode, manual mode, with/without model, `with_model` constructor), `parse_usage` (valid JSON, missing fields, no usage object, plain text), `extract_result_from_json` (valid, invalid, missing result), `post_process_output` (via trait, JSON and plain text), `format_command_display` (with and without model), `strip_usage_stats` (full JSON, plain text, multiline, empty, missing result), Runner trait (`name`, `default`), `build_interactive_args` (yolo, with model, manual mode)
  - Test patterns mirror ClaudeRunner and CopilotRunner tests for consistency
  - UAT passed: `cargo make uat` — 542 tests, 0 failures ✅
- **Constitution Compliance**: No violations. Tests follow existing patterns (Rule 4), minimal changes (Rule 3), clippy::pedantic enabled with test-only suppression (Rule 8).

## 2026-02-08 — T-009 and T-011 Completed
- **Task**: Update --runner CLI argument help text and AGENTS.md documentation
- **Status**: ✅ Done
- **Changes**:
  - Updated all 11 `--runner` CLI argument doc comments in `src/main.rs` to include `[copilot, claude, codex]` in help text
  - Updated `AGENTS.md` workspace overview to list codex in runner implementations
  - Updated `AGENTS.md` flag reference tables for Edit, Bootstrap, and Refactor commands to include codex
  - Updated `AGENTS.md` Runner Implementation Patterns section to document CodexRunner patterns (CodexConfig, `--json` for token parsing, `--full-auto` for yolo mode)
  - UAT passed: `cargo make uat` — 542 tests, 0 failures ✅
- **Constitution Compliance**: No violations. Minimal changes (Rule 3), consistent with existing patterns (Rule 4).

## 2026-02-08 — uat-001 Verification
- **UAT**: Project builds cleanly with codex runner module
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Ran `cargo build` which completed successfully with exit code 0
  - The codex runner module compiles without errors or warnings

## 2026-02-08 — uat-002 Verification
- **UAT**: All existing tests pass with new runner added
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Ran `cargo make test` which completed successfully (exit code 0)
  - All 542 tests passed, 0 skipped, 0 failures
  - Codex runner unit tests (28 tests) included in the passing suite

## 2026-02-08 — uat-003 Verification
- **UAT**: Clippy pedantic passes with no new warnings
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Ran `cargo make clippy` which completed successfully (exit code 0)
  - Clippy ran with `-D warnings` flag — zero warnings emitted
  - All targets and features checked (`--all-targets --all-features`)

## 2026-02-08 — uat-004 Verification
- **UAT**: Runner is selectable via --runner codex flag
- **Status**: ✅ Verified
- **Method**: New test
- **Details**:
  - Created `test_args_parse_run_with_runner_codex` in `src/main.rs` (tests module)
  - Test verifies `--runner codex` is accepted by CLI arg parser and parsed correctly
  - Also confirmed `cargo run -- run --help` lists `codex` in `--runner` options
  - UAT passed: `cargo make uat` — 543 tests, 0 failures ✅

## 2026-02-08 — uat-005 Verification
- **UAT**: CodexRunner build_args produces correct CLI invocation
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Five existing tests in `src/runner/codex.rs` cover `build_args` CLI invocation:
    - `test_build_args_yolo_mode` — verifies `exec`, prompt, `--full-auto`
    - `test_build_args_manual_mode` — verifies `exec`, prompt, no `--full-auto`
    - `test_build_args_with_model` — verifies `--model` flag present
    - `test_build_args_without_model` — verifies no `--model` when unset
    - `test_with_model_constructor` — verifies model + default flags together
  - All 543 tests passed with `cargo make test` ✅

## 2026-02-08 — uat-006 Verification
- **UAT**: Token usage parsing extracts input/output tokens from JSON
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Four existing tests in `src/runner/codex.rs` cover token usage parsing:
    - `test_parse_usage_from_json` — parses JSON with `input_tokens: 26549`, `output_tokens: 1590`, verifies extraction and total computation (28139)
    - `test_parse_usage_missing_fields` — handles partial usage (only `input_tokens`)
    - `test_parse_usage_no_usage_object` — returns None when no usage object present
    - `test_parse_usage_returns_none` — returns None for plain text output
  - All 543 tests passed with `cargo make test` ✅

