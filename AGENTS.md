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
The task T-014 implemented `mr prd edit` for quick PRD modifications. This is a new command that allows editing existing PRDs via a runner-mediated Q/A flow. The auto-managed section currently has notes about clippy policy but contains some raw thinking text that shouldn't be there. Let me clean this up and add relevant info about the new `prd edit` command if applicable.

The current auto-managed section has internal reasoning that should be cleaned up and replaced with proper documentation. The T-014 task is an internal implementation detail (a new CLI command), so it doesn't establish new conventions for agents. The section should be cleaned up to just show the Clippy Policy properly formatted.

Updated the auto-managed section. Cleaned up raw reasoning text and replaced it with a properly formatted **Clippy Policy** section. T-014 (`mr prd edit`) doesn't introduce new agent conventions—it's an internal CLI feature.
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
