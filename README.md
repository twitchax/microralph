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

## Prompt Placeholders

microralph uses static prompt files in `.mr/prompts/` that support placeholder expansion. If you want to customize prompts, here are the available placeholder variables for each prompt type.

### Placeholder Syntax

- `{{variable}}` — Simple string substitution
- `{{#if variable}}...{{/if}}` — Conditional block (renders if variable is truthy/non-empty)
- `{{#each list}}...{{/each}}` — List iteration (use `{{@index}}` for 0-based index)

### run_task.md

Used when executing a task via `mr run`.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{prd_path}}` | string | Absolute path to the PRD file |
| `{{prd_id}}` | string | PRD identifier (e.g., `PRD-0001`) |
| `{{prd_title}}` | string | PRD title |
| `{{next_task_id}}` | string | Task identifier (e.g., `T-001`) |
| `{{task_title}}` | string | Task title |
| `{{task_priority}}` | string | Task priority number |
| `{{task_notes}}` | string | Optional task notes (may be empty) |

### run_task_finalize.md

Used for the final wrap-up task of a PRD.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{prd_id}}` | string | PRD identifier |
| `{{prd_summary}}` | string | Summary of the PRD |

### prd_new_round1_questions.md

Used for the first round of questions when creating a new PRD.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{slug}}` | string | The slug for the new PRD |
| `{{user_description}}` | string | Optional initial description from user |
| `{{#each existing_prds}}` | list | Existing PRDs for context |
| ↳ `{{id}}` | string | PRD identifier |
| ↳ `{{title}}` | string | PRD title |
| ↳ `{{status}}` | string | PRD status (draft/active/done/parked) |

### prd_new_roundN_questions.md

Used for follow-up rounds of questions during PRD creation.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{slug}}` | string | The slug for the new PRD |
| `{{#each qa_history}}` | list | Previous Q/A pairs |
| ↳ `{{question}}` | string | The question that was asked |
| ↳ `{{answer}}` | string | The user's answer |
| ↳ `{{@index}}` | number | 0-based index of the Q/A pair |

### prd_new_synthesize_prd.md

Used to synthesize the final PRD from collected Q/A.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{slug}}` | string | The slug for the new PRD |
| `{{#each qa_history}}` | list | All Q/A pairs from the session |
| ↳ `{{question}}` | string | The question |
| ↳ `{{answer}}` | string | The answer |
| `{{#each existing_prds}}` | list | Existing PRDs for context |
| ↳ `{{id}}` | string | PRD identifier |
| ↳ `{{title}}` | string | PRD title |

### prd_edit.md

Used when editing an existing PRD via `mr prd edit`.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{prd_path}}` | string | Path to the PRD file |
| `{{user_request}}` | string | The user's edit request |
| `{{prd_content}}` | string | Current PRD file content |
| `{{#each qa_history}}` | list | Follow-up Q/A pairs (if any) |
| ↳ `{{question}}` | string | The question |
| ↳ `{{answer}}` | string | The answer |

### bootstrap_plan.md

Used during `mr bootstrap` to analyze the repository.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{prd_budget}}` | string | Maximum number of PRDs to generate |
| `{{#each heuristics}}` | list | Analysis heuristics |
| ↳ `{{description}}` | string | Heuristic description |

### bootstrap_generate_prds.md

Used to generate PRDs from the bootstrap plan.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{plan}}` | string | The generated bootstrap plan |
| `{{prd_budget}}` | string | Maximum number of PRDs to generate |

### update_agents.md

Used to update the auto-managed section of AGENTS.md.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{agents_content}}` | string | Current AGENTS.md content |
| `{{#each recent_changes}}` | list | Recent file changes |
| ↳ `{{file}}` | string | File path that was changed |
| ↳ `{{description}}` | string | Description of the change |

### adapt_language.md

Used when initializing for a non-Rust language.

| Placeholder | Type | Description |
|-------------|------|-------------|
| `{{language}}` | string | Target language (e.g., `python`, `node`) |
| `{{#each build_commands}}` | list | Typical build/test commands |
| ↳ `{{command}}` | string | A build/test command |

### init.md

Used during `mr init`. This prompt has no placeholders.

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
