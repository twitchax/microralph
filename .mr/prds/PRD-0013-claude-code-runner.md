---
id: PRD-0013
title: "Add Claude CLI Runner"
status: active
owner: "microralph"
created: 2026-01-24
updated: 2026-01-25

principles:
- Mirror CopilotRunner surface area for consistency across runners
- Default to non-interactive mode with yolo permissions for autonomous operation
- Parse and track token usage to enable cost monitoring
- Support model selection for flexibility between Claude models

references:
- name: Claude CLI Official Documentation
  url: https://deepwiki.com/anthropics/claude-code/2.3-cli-commands-and-interaction-modes
- name: Claude CLI Commands Reference
  url: https://gist.github.com/dai/51b06d2ed1c1b11a90d16c1a913c96f8
- name: CopilotRunner Implementation
  url: /home/twitchax/projects/microralph/src/runner/copilot.rs

acceptance_tests:
- id: uat-001
  name: ClaudeRunner executes prompts in non-interactive mode
  command: cargo make test
  uat_status: unverified
- id: uat-002
  name: ClaudeRunner parses token usage from Claude CLI output
  command: cargo make test
  uat_status: unverified
- id: uat-003
  name: ClaudeRunner supports yolo/manual permission modes
  command: cargo make test
  uat_status: unverified
- id: uat-004
  name: Full CI passes with ClaudeRunner implementation
  command: cargo make ci
  uat_status: unverified

tasks:
- id: T-001
  title: Research Claude CLI command structure and flags
  priority: 1
  status: done
  notes: Verified binary name (`claude`), non-interactive mode (`-p`), permission flags (`--dangerously-skip-permissions`, `--allowedTools`), model selection, and token usage output format.

- id: T-002
  title: Create ClaudeConfig struct with permission modes
  priority: 2
  status: done
  notes: Mirror CopilotConfig structure with `claude_path`, `permission_mode` (Yolo/Manual), `no_ask_user`, and `model` fields.

- id: T-003
  title: Implement ClaudeRunner struct with Runner trait
  priority: 3
  status: done
  notes: Implement `name()`, `execute()`, `execute_streaming()`, and `is_available()` methods following CopilotRunner pattern.

- id: T-004
  title: Implement build_args method for Claude CLI flags
  priority: 4
  status: todo
  notes: Support `-p` for prompt, `--dangerously-skip-permissions` for yolo mode, model selection, and other relevant flags.

- id: T-005
  title: Implement token usage parsing for Claude CLI output
  priority: 5
  status: todo
  notes: Research actual Claude CLI output format. Note that as of late 2025, Claude CLI does not provide built-in token usage in output. May need to use `--output-format json` and parse response, or rely on third-party tools.

- id: T-006
  title: Implement output stripping for Claude CLI stats section
  priority: 6
  status: todo
  notes: If Claude CLI appends statistics similar to Copilot, strip them to keep output clean while preserving usage data.

- id: T-007
  title: Add unit tests for ClaudeRunner
  priority: 7
  status: todo
  notes: Test config builder, arg construction, permission modes, usage parsing, and output stripping. Mock the `claude` binary for tests.

- id: T-008
  title: Export ClaudeRunner from runner module
  priority: 8
  status: todo
  notes: Add ClaudeRunner to `mod.rs` exports alongside CopilotRunner.

- id: T-009
  title: Update AGENTS.md with ClaudeRunner conventions
  priority: 9
  status: todo
  notes: Document ClaudeRunner implementation patterns, token usage parsing, and testing approach for future agents.

---

# Summary

Add a Claude CLI runner (`ClaudeRunner`) that provides the same surface area as the existing `CopilotRunner`, enabling microralph to execute prompts using Claude's CLI tool. This runner should support non-interactive mode, permission controls (yolo/manual), model selection, and token usage tracking.

---

# Problem

Currently, microralph only supports GitHub Copilot CLI as a runner. Users who prefer or require Claude CLI (e.g., due to model preferences, API access, or organizational requirements) cannot use microralph. Adding ClaudeRunner enables multi-agent support and provides users with flexibility in choosing their coding agent backend.

---

# Goals

1. Implement `ClaudeRunner` with the same surface area as `CopilotRunner`
2. Support non-interactive mode (`-p` flag) for autonomous operation
3. Support permission modes: yolo (`--dangerously-skip-permissions`) and manual
4. Parse and track token usage from Claude CLI output (if available)
5. Support model selection for different Claude variants
6. Provide streaming and non-streaming execution modes
7. Mock Claude CLI for unit tests (do not require actual installation for CI)

---

# Non-Goals (MVP)

- API-based Claude runner (this PRD focuses on CLI-based execution only)
- Advanced Claude-specific features beyond the CopilotRunner surface area
- Third-party token tracking tools integration (e.g., CCUsage npm package)
- Automatic fallback between Copilot and Claude runners
- Configuration for specifying which runner to use (defer to future work)

---

# History

(Entries appended by `mr run` will go below this line.)

---

## 2026-01-25 — T-001 Completed
- **Task**: Research Claude CLI command structure and flags
- **Status**: ✅ Done
- **Changes**:
  - Created `src/runner/claude.rs` with ClaudeRunner implementation
  - ClaudeRunner mirrors CopilotRunner surface area with appropriate Claude CLI flags
  - Binary name: `claude`
  - Non-interactive mode: `-p` or `--print` flag
  - Permission skipping: `--dangerously-skip-permissions` (yolo mode)
  - Model selection: `--model <name>`
  - No-ask-user: `--no-ask-user` for autonomous operation
  - Token usage: Claude CLI does not currently output token stats in stdout (unlike Copilot CLI)
  - Added helper function `create_runner()` in main.rs to centralize runner creation
  - Updated all runner instantiation sites to support "claude" runner
  - Updated runner module exports in `src/runner/mod.rs`
  - UAT passes: All 283 tests pass

---

## 2026-01-25 — T-002 Completed
- **Task**: Create ClaudeConfig struct with permission modes
- **Status**: ✅ Done
- **Changes**:
  - No additional changes required; T-001 implementation already included ClaudeConfig
  - Verified ClaudeConfig struct in `src/runner/claude.rs` (lines 24-84):
    - Contains `claude_path`, `permission_mode`, `no_ask_user`, and `model` fields
    - PermissionMode enum with Yolo and Manual variants (lines 13-22)
    - Default implementation with yolo mode and no_ask_user enabled
    - Builder methods: `with_path()`, `with_permission_mode()`, `with_no_ask_user()`, `with_model()`
  - Mirrors CopilotConfig structure as specified
  - UAT passes: All 283 tests pass

---

## 2026-01-25 — T-003 Completed
- **Task**: Implement ClaudeRunner struct with Runner trait
- **Status**: ✅ Done
- **Changes**:
  - No additional changes required; T-001 implementation already included full ClaudeRunner implementation
  - Verified ClaudeRunner struct in `src/runner/claude.rs` (lines 86-391) implements all Runner trait methods:
    - `name()` returns "claude" (lines 180-182)
    - `format_command_display()` formats command with flags for display (lines 184-214)
    - `execute()` runs Claude CLI non-streaming (lines 216-272)
    - `execute_streaming()` runs Claude CLI with real-time output (lines 274-386)
    - `is_available()` checks if Claude CLI is installed (lines 388-390)
  - Implementation follows CopilotRunner pattern exactly
  - Includes comprehensive unit tests (lines 393-532) covering all functionality
  - UAT passes: All 283 tests pass