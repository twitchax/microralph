# microralph — Agents Guide

This document provides detailed workflows and troubleshooting for AI coding agents working in this repository.

## Workspace Overview

- `src/`: Main Rust source code
- `.mr/`: microralph state directory
  - `prds/`: PRD files
  - `templates/`: PRD templates
  - `prompts/`: Static prompt files for each stage
  - `PRDS.md`: Auto-generated PRD index

## Quick Start

```bash
# Build
cargo build

# Test
cargo make test

# Full CI (fmt, clippy, test)
cargo make ci

# UAT (the one true gate)
cargo make uat
```

## Suggest Command Workflow

The `mr suggest` command uses AI to analyze the codebase and generate PRD suggestions:

1. **Analysis Phase**: Scans repository structure, existing PRDs, git history, TODO comments, and dependency versions
2. **Generation**: Produces exactly 5 suggestions with title, description, category, effort estimate, and rationale
3. **Selection**: Displays numbered picker (1-5 or 'q' to quit) for user to choose a suggestion
4. **Integration**: Selected suggestion flows into `mr new` with pre-filled context

Suggestions balance strategic features with quick wins. The command follows existing CLI patterns from PRD-0009.

## Build & Test

### Prerequisites

```bash
# Install cargo-make (if not present)
cargo install cargo-make
```

### Commands

```bash
# Format code
cargo make fmt

# Run clippy
cargo make clippy

# Run tests with nextest
cargo make test

# Run full CI pipeline
cargo make ci
```

## Conventions for Agents

- Keep changes minimal and focused; avoid unrelated refactors.
- Follow existing style; don't add license headers.
- Use `anyhow::Result` for fallible functions.
- Prefer `tracing` over `println!` for diagnostics.
- All dev commands route through `cargo make`.

### Code Style

- Use vertical whitespace generously to separate logical sections.
- Prefer explicitness over implicitness.
- Reduce nesting by using guard clauses and early returns.
- Prefer functional programming techniques where appropriate.

### Runner Implementation Patterns

When implementing new runners (e.g., ClaudeRunner, CopilotRunner):

- **Mirror the CopilotRunner surface area**: New runners should provide the same API as existing runners for consistency.
- **Config struct with permission modes**: Each runner has a config struct (e.g., `ClaudeConfig`) with fields for binary path, permission mode (Yolo/Manual), no_ask_user flag, and optional model.
- **Build args method**: Implement `build_args()` to construct CLI flags based on config. Keep this private and unit-testable.
- **Token usage parsing**: Parse token usage from CLI output if available. For Claude, use `--output-format json` and extract from the `usage` object. Return `Option<UsageInfo>` with `input_tokens`, `output_tokens`, and `total_tokens`.
- **Output stripping**: Implement a public `strip_usage_stats()` method to remove metadata from CLI output. For JSON-based CLIs (like Claude), extract only the `result` field. For text-based CLIs (like Copilot), use regex to strip stats sections.
- **Mock for tests**: All runner tests should use mocked binaries (via test-only constructors) and never require actual CLI installation. This ensures CI can run without external dependencies.
- **Default to yolo mode**: Runners default to non-interactive mode with permissions auto-granted (`--dangerously-skip-permissions` for Claude, similar for others) to enable autonomous operation.
- **Streaming support**: Implement both `execute()` (non-streaming) and `execute_streaming()` (real-time output) methods from the `Runner` trait.

## PRD Format

PRDs are Markdown files with YAML frontmatter containing:

- `id`: Unique identifier (e.g., PRD-0001)
- `title`: Human-readable title
- `status`: draft | active | done | parked
- `tasks`: List of tasks with id, title, priority, status

History entries are appended by `mr run` at the bottom of the PRD.

## Quick Tasks Reference

```bash
# Format
cargo make fmt

# Lint
cargo make clippy

# Test
cargo make test

# Full CI
cargo make ci

# UAT
cargo make uat
```

## Troubleshooting

- If `cargo-make` is missing: `cargo install cargo-make`
- If `cargo-nextest` is missing: `cargo binstall cargo-nextest --no-confirm`
- For faster tool installation, use cargo-binstall:
  ```bash
  curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
  ```

---

<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
## Manual Updates by Agents

Automatic AGENTS.md updates have been removed to give agents more flexibility. Agents should update AGENTS.md manually when:

- Discovering new build/test commands or troubleshooting steps
- Identifying code patterns or conventions not already documented
- Adding new tools or dependencies that affect the workflow
- Finding solutions to common issues during implementation

Update any relevant section, not just this one. Keep additions concise and actionable.
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
