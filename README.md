[![Build and Test](https://github.com/twitchax/microralph/actions/workflows/build.yml/badge.svg)](https://github.com/twitchax/microralph/actions/workflows/build.yml)
[![codecov](https://codecov.io/gh/twitchax/microralph/branch/main/graph/badge.svg)](https://codecov.io/gh/twitchax/microralph)
[![Version](https://img.shields.io/crates/v/microralph.svg)](https://crates.io/crates/microralph)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# microralph

A tiny CLI that helps you **create PRDs** and **execute PRDs** by repeatedly invoking an underlying coding-agent CLI (starting with GitHub Copilot CLI) and updating PRD state (tasks + History) after every run.

## MVP Promise

Minimal ceremony. You can:

- Bootstrap or init a repo
- Write PRDs via a guided Q/A
- Run an iterative "try → verify → log" loop
- Watch tasks flip to done when `cargo make uat` passes

## Installation

### Cargo

```bash
cargo install microralph
```

### From Source

```bash
git clone https://github.com/twitchax/microralph.git
cd microralph
cargo install --path .
```

## Usage

```bash
# Initialize a new repo with .mr/ structure
mr init

# Bootstrap an existing repo into PRDs
mr bootstrap

# Create a new PRD via guided Q/A
mr prd new my-feature

# List all PRDs
mr prd list

# Run the next task from the active PRD
mr run

# Show status of PRDs and tasks
mr status
```

### Commands

| Command             | Description                                                                            |
| ------------------- | -------------------------------------------------------------------------------------- |
| `mr init`           | Initialize a new repo with `.mr/` structure, templates, prompts, and starter AGENTS.md |
| `mr bootstrap`      | Ingest an existing repo into PRDs: generate `.mr/PRDS.md` and starter PRDs             |
| `mr prd new <slug>` | Create a new PRD via guided Q/A                                                        |
| `mr prd list`       | List all PRDs                                                                          |
| `mr run`            | Run the next task from the active PRD                                                  |
| `mr status`         | Show status of PRDs and tasks                                                          |

### Flags

| Flag                | Description                         |
| ------------------- | ----------------------------------- |
| `-v, --verbose`     | Enable verbose output               |
| `-q, --quiet`       | Suppress non-essential output       |
| `--runner <runner>` | Specify runner (default: `copilot`) |

## Development

Most dev workflows run via `cargo make`.

### Prerequisites

```bash
# Install cargo-make
cargo install cargo-make
```

### Commands

```bash
# Run tests
cargo make test

# Run full CI pipeline (fmt, clippy, test)
cargo make ci

# Format code
cargo make fmt

# Run clippy
cargo make clippy

# Build release
cargo make build-release

# UAT (User Acceptance Tests) — the one true gate
cargo make uat
```

## Principles

- **No direct API calls**: microralph shells out to runner CLIs only
- **State lives in git**: PRDs are Markdown files with YAML frontmatter + History section
- **One-or-zero tasks per `mr run`**: Each invocation attempts at most one task
- **Runner can fail**: History captures what happened and what to try next
- **Avoid XML/JSON state blobs**: Human-readable Markdown PRDs
- **cargo make everything**: Almost all dev workflows route through `cargo make`

## PRD Format

PRDs are Markdown files with YAML frontmatter:

```markdown
---
id: PRD-0001
title: My Feature
status: active
owner: Your Name
created: 2026-01-23
updated: 2026-01-23

tasks:
  - id: T-001
    title: "Implement the thing"
    priority: 1
    status: todo
---

# Summary

What this PRD is about...

---

# History

(Entries appended by `mr run` will go below this line.)
```

## License

MIT
