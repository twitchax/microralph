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
The T-003 task completed adds acceptance test verification via finalization prompt. Looking at the current auto-managed section, it's incomplete/malformed - it contains descriptive text but no actual structured documentation. Let me check the current source to understand the patterns properly:

Now let me check the current CLI commands to document them properly:

Now I have a complete picture. The auto-managed section is currently malformed (contains descriptive text but not structured content). Based on the T-003 completion and the full codebase view, I should update it with proper documentation. The key patterns are:

1. **CLI Commands** - now includes `mr prd finalize`
2. **Error Handling** - `thiserror` for domain-specific errors (from T-002)
3. **Prompt system** - `PromptKind`, `PlaceholderContext`, `expand_placeholders`, `load_prompt_with_fallback`
4. **Runner pattern** - for LLM command execution with streaming support

Updated the auto-managed section with:
- **CLI Commands** table including the new `mr prd finalize` command
- **Error Handling** documenting both `anyhow` and `thiserror` patterns
- **Prompt System** explaining the placeholder expansion workflow
- **Module Organization** covering the Config/Result struct pattern and Runner abstraction
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
