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
## CLI Command Structure

As of PRD-0009, the CLI has been streamlined for better ergonomics:

### PRD Management Commands (Top-Level)
- `mr new <slug>` — Create new PRD (formerly `mr prd new`)
- `mr list` — List all PRDs (formerly `mr prd list`)
- `mr edit <id> "<request>"` — Edit PRD (formerly `mr prd edit`)
- `mr finalize <id>` — Finalize PRD (formerly `mr prd finalize`)

### Run Command
- `mr run` — Execute next task from active PRD
- `mr run <id>` — Execute next task from specific PRD (formerly `mr run --prd <id>`)
- `mr run --stream` — Stream output in real-time

### Other Commands
- `mr init` — Initialize .mr/ structure
- `mr bootstrap` — Generate PRDs from existing repo
- `mr status` — Show PRD/task status
- `mr reindex` — Regenerate index

### Output Behavior
- LLM output now shows **tail** (last 500 chars) instead of beginning
- Applies to both task execution and UAT verification loops
- Improves debugging by surfacing errors and completion status
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
