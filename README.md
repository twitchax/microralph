[![Build and Test](https://github.com/twitchax/microralph/actions/workflows/build.yml/badge.svg)](https://github.com/twitchax/microralph/actions/workflows/build.yml)
[![codecov](https://codecov.io/gh/twitchax/microralph/branch/main/graph/badge.svg)](https://codecov.io/gh/twitchax/microralph)
[![Version](https://img.shields.io/crates/v/microralph.svg)](https://crates.io/crates/microralph)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# microralph

> *A small ralph to help you ralph your ralphs.* 🦙

**microralph** is a tiny CLI that wraps your favorite AI coding agent (starting with [GitHub Copilot CLI](https://docs.github.com/en/copilot/using-github-copilot/using-github-copilot-chat-in-your-ide)) and turns it into a **PRD-driven task loop**. You write PRDs (Product Requirements Documents), and microralph repeatedly invokes the agent—one task at a time—until everything is done.

Oh, and yes: **microralph was entirely `ralph`'d into existence by microralph itself**. Dogfooding at its finest. 🐕

## Why microralph?

AI coding agents are powerful, but they have a fatal flaw: **context windows**. The more context an agent accumulates, the slower and more expensive it gets—and eventually it forgets what it was doing.

microralph solves this by:

1. **Breaking work into discrete tasks** via PRDs
2. **Running one task per invocation** so context never bloats
3. **Persisting state in git-tracked Markdown** so the agent can pick up where it left off
4. **Logging History** so failed attempts inform future runs

No more 200k-token conversations that go off the rails. Just focused, atomic task execution.

## The Normal Flow

```
┌─────────────────────────────────────────────────────────────┐
│  1. mr init / mr bootstrap     ← Set up .mr/ structure     │
│  2. mr prd new my-feature      ← Create PRD via guided Q/A │
│  3. mr run                     ← Execute one task          │
│  4. Agent implements, runs UAT, updates PRD, commits       │
│  5. Repeat step 3 until all tasks are done                 │
└─────────────────────────────────────────────────────────────┘
```

Each `mr run` invocation:
- Picks the highest-priority incomplete task
- Invokes the underlying agent with a focused prompt
- Expects the agent to: implement, verify with UAT, update PRD status/history, commit
- Exits—keeping context minimal for the next run

## Features

- **PRD-driven development**: Structure your work as markdown PRDs with YAML frontmatter
- **One-task-per-run loop**: Context stays small, agents stay focused
- **Guided PRD creation**: `mr prd new` runs an interactive Q/A to generate PRDs
- **Bootstrap existing repos**: `mr bootstrap` scans your repo and generates starter PRDs
- **Multi-language support**: Works with Rust, Python, Node.js, Go, Java (auto-detected)
- **Streaming output**: `mr run --stream` shows agent output in real-time
- **Git-native state**: PRDs are versioned markdown; no databases or JSON blobs
- **Runner abstraction**: Pluggable adapters (Copilot, mock for testing, more to come)

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

| Command                        | Description                                                                            |
| ------------------------------ | -------------------------------------------------------------------------------------- |
| `mr init`                      | Initialize a new repo with `.mr/` structure, templates, prompts, and starter AGENTS.md |
| `mr init --language <lang>`    | Initialize for a specific language (rust, python, node, go, java)                      |
| `mr bootstrap`                 | Ingest an existing repo into PRDs: generate `.mr/PRDS.md` and starter PRDs             |
| `mr prd new <slug>`            | Create a new PRD via guided Q/A                                                        |
| `mr prd edit <id> "<request>"` | Edit an existing PRD via runner assistance                                             |
| `mr prd list`                  | List all PRDs (regenerates `.mr/PRDS.md`)                                              |
| `mr run`                       | Run the next task from the highest-priority active PRD                                 |
| `mr run --prd <id>`            | Run the next task from a specific PRD                                                  |
| `mr run --stream`              | Run with real-time streaming output                                                    |
| `mr reindex`                   | Regenerate index and verify/fix PRD interlinks                                         |
| `mr status`                    | Show status of PRDs and tasks                                                          |

### Flags

| Flag                | Description                                        |
| ------------------- | -------------------------------------------------- |
| `-v, --verbose`     | Enable verbose output                              |
| `-q, --quiet`       | Suppress non-essential output                      |
| `--runner <runner>` | Specify runner (default: `copilot`)                |
| `--model <model>`   | Specify model (passed through to runner)           |
| `--stream`          | Stream runner output in real-time (for `mr run`)   |

### Configuration

Settings can be persisted in `.mr/config.toml`:

```toml
runner = "copilot"
model = "claude-sonnet-4-20250514"
permission_mode = "yolo"
timeout_minutes = 30
```

CLI flags override config file settings.

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
| `{{user_context}}` | string | Optional upfront context provided by user |
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

## Learn More

### What is Ralph?

**Ralph** (named after [Ralph Wiggum from The Simpsons](https://en.wikipedia.org/wiki/Ralph_Wiggum)) is a pattern where you repeatedly invoke an AI coding agent in a loop until a task is complete. The original concept emerged in the AI coding community as a way to overcome context window limitations by running fresh agent sessions iteratively.

Popular Ralph implementations include:
- [soderlind/ralph](https://github.com/soderlind/ralph) — Shell script wrapper for GitHub Copilot CLI
- [Ralph TUI](https://ralph-tui.com/) — Terminal UI for Ralph loops
- [Ralph Loop blog post](https://benjamin-abt.com/blog/2026/01/19/ralph-loop-github-copilot-cli-dotnet/) — Deep dive on the Ralph pattern
- [The Ralph Wiggum Approach](https://dev.to/sivarampg/the-ralph-wiggum-approach-running-ai-coding-agents-for-hours-not-minutes-57c1) — Long-form article on autonomous coding

### How microralph Differs from Basic Ralph

Traditional Ralph implementations are simple loop scripts: run the agent → check if done → repeat. They work well for small tasks but have limitations:

- **No structure**: They don't enforce task breakdown or planning upfront
- **No persistence**: Progress isn't tracked in a human-readable way
- **No history**: Failed attempts aren't logged for future context
- **One-shot scope**: Typically run until a single condition is met, not across multiple tasks

**microralph** takes the Ralph pattern and adds:

1. **PRD-driven structure**: Define all tasks upfront with priorities
2. **One-task-per-run**: Each `mr run` completes exactly one task (no bloat)
3. **Git-native state**: PRDs are markdown files that track progress and history
4. **Multi-task orchestration**: Automatically picks the next task from active PRDs
5. **Guided workflows**: `mr prd new` and `mr bootstrap` help structure work
6. **Runner abstraction**: Pluggable backends (Copilot, others to come)

Think of microralph as "Ralph with a project management system built in."

### What's a PRD?

A **Product Requirements Document** (PRD) defines what you want to build. In microralph, PRDs are enhanced with:
- **Tasks**: Atomic units of work with priority and status
- **History**: A log of what the agent attempted and what happened

See [Writing Good PRDs](https://www.atlassian.com/agile/product-management/requirements) for general guidance.

### Agent Loops & Context Limits

Modern AI agents suffer from the **context window problem**: as conversations grow, agents slow down, get expensive, and eventually "forget" earlier context.

microralph implements an **agentic loop** pattern:
1. Load minimal context (just the current task + PRD)
2. Execute the task
3. Persist results to disk (git-tracked markdown)
4. Exit—freeing context for the next task

This pattern is inspired by work on:
- [Agentic Design Patterns](https://www.deeplearning.ai/the-batch/agentic-design-patterns-part-1/) by Andrew Ng
- [ReAct: Reasoning and Acting in Language Models](https://arxiv.org/abs/2210.03629)
- [LangChain Agent Loops](https://python.langchain.com/docs/modules/agents/)

## Comparison with Other Tools

| Feature                          | microralph         | Claude Code       | Cursor             | Aider              | Cline              |
|----------------------------------|:------------------:|:-----------------:|:------------------:|:------------------:|:------------------:|
| **PRD-driven task breakdown**    | ✅                  | ❌                | ❌                 | ❌                 | ❌                 |
| **One-task-per-run (no bloat)**  | ✅                  | ❌                | ❌                 | ❌                 | ❌                 |
| **Git-native state**             | ✅                  | ❌                | ❌                 | ✅                 | ❌                 |
| **History/retry logging**        | ✅                  | ❌                | ❌                 | ⚠️ (partial)      | ❌                 |
| **Multi-runner abstraction**     | ✅                  | ❌ (Claude only)  | ❌ (Cursor only)   | ⚠️ (multi-model)  | ❌ (VSCode only)   |
| **Works in terminal**            | ✅                  | ✅                | ❌ (IDE only)      | ✅                 | ❌ (IDE only)      |
| **No API keys required**         | ✅ (uses CLI auth)  | ✅                | ✅                 | ❌                 | ✅                 |
| **Customizable prompts**         | ✅                  | ❌                | ❌                 | ⚠️                | ❌                 |

### Why microralph is Different

Most AI coding tools are **session-based**: you start a conversation, describe what you want, and the agent tries to do everything in one go. This works for small tasks but breaks down for larger projects:

- **Context bloat**: Long sessions accumulate context until the agent gets confused
- **No persistence**: If you close the session, you start over
- **No structure**: There's no clear definition of "done" or progress tracking

microralph is **task-based**: you define discrete tasks upfront, and each `mr run` tackles exactly one task with fresh context. Progress is tracked in git, so you can close your terminal, reboot your machine, or come back weeks later—microralph picks up where it left off.

Think of it as the difference between "do everything in one meeting" vs. "complete one ticket per sprint" — the latter scales.

## License

MIT
