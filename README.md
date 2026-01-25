[![Build and Test](https://github.com/twitchax/microralph/actions/workflows/build.yml/badge.svg)](https://github.com/twitchax/microralph/actions/workflows/build.yml)
[![codecov](https://codecov.io/gh/twitchax/microralph/branch/main/graph/badge.svg)](https://codecov.io/gh/twitchax/microralph)
[![Version](https://img.shields.io/crates/v/microralph.svg)](https://crates.io/crates/microralph)
[![Downloads](https://img.shields.io/github/downloads/twitchax/microralph/total.svg)](https://github.com/twitchax/microralph/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

# microralph

> *A small ralph to help you ralph your ralphs.* 🦙

**microralph** is a tiny CLI that wraps your favorite AI coding agent (starting with [GitHub Copilot CLI](https://docs.github.com/en/copilot/using-github-copilot/using-github-copilot-chat-in-your-ide)) and turns it into a **PRD-driven task loop**. You write PRDs (Product Requirements Documents), and microralph repeatedly invokes the agent—one task at a time—until everything is done.

Oh, and yes: **microralph was entirely `ralph`'d into existence by microralph itself**. Dogfooding at its finest. 🐕

## What is a Ralph?

A project that is mostly **ralph'd into existence** by AI agents is itself called a **ralph**. microralph is a ralph—it was built almost entirely by running `mr run` in a loop, with a human steering via PRDs.

The name comes from [Ralph Wiggum](https://en.wikipedia.org/wiki/Ralph_Wiggum): loveable, earnest, occasionally brilliant, but needs guidance. AI agents are the same way.

### The Real Value: Locking Time for Artisanal Code

Here's the thing: you don't want to ralph *everything*. Some code deserves your full attention—the elegant algorithm, the nuanced architecture, the domain-specific logic that only you understand. That's **artisanal code**.

But most projects need a lot of *other* code: CLI scaffolding, config parsing, test harnesses, CI pipelines, documentation. Important, but not where you want to spend your creative energy.

**microralph lets you ralph the boring parts so you can lock time for the good stuff.**

Use it to:
- Build internal tools and utilities you need but don't want to hand-craft
- Scaffold new projects with all the boilerplate handled
- Implement features that are well-defined but tedious
- Free up your time for higher-value work

The goal isn't to replace you—it's to *give you time back*.

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
│  2. mr new my-feature          ← Create PRD via guided Q/A │
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
- **Guided PRD creation**: `mr new` runs an interactive Q/A to generate PRDs
- **Bootstrap existing repos**: `mr bootstrap` scans your repo and generates starter PRDs
- **Constitution-based governance**: Define project rules in `.mr/constitution.md` to guide PRD workflows
- **Multi-language support**: Works with Rust, Python, Node.js, Go, Java (auto-detected)
- **Streaming output**: `mr run --stream` shows agent output in real-time
- **Git-native state**: PRDs are versioned markdown; no databases or JSON blobs
- **Runner abstraction**: Pluggable adapters (Copilot, mock for testing, more to come)

## Installation

### Pre-built Binaries (Recommended)

Download pre-built binaries from [GitHub Releases](https://github.com/twitchax/microralph/releases). Available for:
- **Linux x86_64** (`mr-linux`)
- **macOS ARM** (`mr-macos`)
- **Windows x86_64** (`mr-windows.exe`)
- **WASM32-WASIP2** (`mr.wasm`)

#### Linux / macOS

```bash
# Download the latest release binary
curl -L https://github.com/twitchax/microralph/releases/latest/download/mr-linux -o mr
# or for macOS ARM:
# curl -L https://github.com/twitchax/microralph/releases/latest/download/mr-macos -o mr

# Make it executable
chmod +x mr

# Move to a directory in your PATH
sudo mv mr /usr/local/bin/

# Verify installation
mr --version
```

#### Windows

1. Download `mr-windows.exe` from the [releases page](https://github.com/twitchax/microralph/releases)
2. Rename to `mr.exe`
3. Move to a directory in your PATH (e.g., `C:\Program Files\microralph\`)
4. Open a new terminal and verify with `mr --version`

#### WebAssembly (via OCI Registry)

Run directly from GitHub Container Registry using any WASI-compatible runtime. This works well for sandboxed or cross-platform use cases.

With Wasmtime:

```bash
$ wasmtime run ghcr.io/twitchax/microralph:latest -- --version
```

With wkg (WebAssembly Package Manager):

```bash
$ wkg get ghcr.io/twitchax/microralph:latest
```

Or download and run manually:

```bash
# Download the WASM binary
curl -L https://github.com/twitchax/microralph/releases/latest/download/mr.wasm -o mr.wasm

# Run with wasmtime
wasmtime mr.wasm -- --version

# Optional: Create an alias
echo 'alias mr="wasmtime /path/to/mr.wasm --"' >> ~/.bashrc
```

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

# Get AI-generated PRD suggestions
mr suggest

# Create a new PRD via guided Q/A
mr new my-feature

# List all PRDs
mr list

# Run the next task from the active PRD
mr run

# Show status of PRDs and tasks
mr status
```

### Commands

| Command                            | Description                                                                            |
| ---------------------------------- | -------------------------------------------------------------------------------------- |
| `mr init`                          | Initialize a new repo with `.mr/` structure, templates, prompts, and starter AGENTS.md |
| `mr init --language <lang>`        | Initialize for a specific language (rust, python, node, go, java)                      |
| `mr bootstrap`                     | Ingest an existing repo into PRDs: generate `.mr/PRDS.md` and starter PRDs             |
| `mr suggest`                       | Generate 5 AI-powered PRD suggestions based on codebase analysis and research          |
| `mr new <slug>`                    | Create a new PRD via guided Q/A                                                        |
| `mr new <slug> --context`          | Create a new PRD with upfront context to guide initial questions                       |
| `mr edit <id> "<request>"`         | Edit an existing PRD via runner assistance                                             |
| `mr constitution edit "<request>"` | Edit the constitution via LLM assistance                                               |
| `mr list`                          | List all PRDs (regenerates `.mr/PRDS.md`)                                              |
| `mr finalize <id>`                 | Finalize a PRD (mark as done and close out)                                            |
| `mr run`                           | Run the next task from the highest-priority active PRD                                 |
| `mr run <id>`                      | Run the next task from a specific PRD                                                  |
| `mr run --stream`                  | Run with real-time streaming output                                                    |
| `mr reindex`                       | Regenerate index and verify/fix PRD interlinks                                         |
| `mr status`                        | Show status of PRDs and tasks                                                          |

### Flags

| Flag                | Description                                      |
| ------------------- | ------------------------------------------------ |
| `-v, --verbose`     | Enable verbose output                            |
| `-q, --quiet`       | Suppress non-essential output                    |
| `--runner <runner>` | Specify runner (default: `copilot`)              |
| `--model <model>`   | Specify model (passed through to runner)         |
| `--stream`          | Stream runner output in real-time (for `mr run`) |

### Configuration

Settings can be persisted in `.mr/config.toml`:

```toml
runner = "copilot"
model = "claude-sonnet-4-20250514"
permission_mode = "yolo"
timeout_minutes = 30
```

CLI flags override config file settings.

### Dev Containers

microralph supports **dev containers** for consistent, sandboxed development environments. Dev containers isolate your development environment from your host machine, ensuring all tools and dependencies are versioned and reproducible.

#### Why Use Dev Containers?

- **Consistency**: Every developer works in the same environment
- **Isolation**: Protects your host machine from experimental or potentially risky operations
- **Reproducibility**: Codify all dependencies and tools in version control
- **Onboarding**: New contributors can get started in seconds
- **Safety**: Run AI-generated code in a sandbox without risk to your local machine

#### Supported Workflows

microralph dev containers work with:
- **VSCode**: Install the [Dev Containers extension](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) and open the repo—VSCode will prompt you to reopen in a container
- **GitHub Codespaces**: Open the repo in Codespaces for a fully cloud-based dev environment
- **CLI**: Use the [Dev Container CLI](https://github.com/devcontainers/cli) to build and run containers from the terminal:
  ```bash
  # Install the CLI
  npm install -g @devcontainers/cli
  
  # Open a shell in the dev container
  devcontainer up --workspace-folder .
  devcontainer exec --workspace-folder . bash
  ```

#### Generating Dev Container Configs

microralph can automatically generate `.devcontainer/devcontainer.json` by analyzing your repository:

```bash
mr devcontainer generate
```

This command:
1. Scans your repository structure (languages, frameworks, dependencies)
2. Analyzes git history for recently added tools
3. Reads PRDs for tool references
4. Generates a `.devcontainer/devcontainer.json` with appropriate base image, extensions, and tool installations

The generated config includes:
- Base container image matching your primary language
- Pre-installed development tools (cargo-make, cargo-nextest, etc.)
- VSCode extensions relevant to your stack
- Forwarded ports for local services
- Initialization scripts to set up the environment

#### Dev Container Warnings

When running commands that invoke AI models (`mr run`, `mr new`, `mr devcontainer generate`), microralph will show a brief warning if you're not inside a dev container. This is informational only—commands will still execute normally.

To suppress the warning, either:
- Work inside a dev container (recommended)
- Run commands in an environment where dev container detection identifies container usage

#### Regenerating After Changes

As your project evolves, regenerate the dev container config to keep it in sync:

```bash
# Analyze current state and update .devcontainer/devcontainer.json
mr devcontainer generate
```

This is especially useful after:
- Adding new dependencies or tools
- Switching to a different language or framework
- Major architectural changes documented in PRDs

### Constitution

microralph supports project-specific governance rules via a **Constitution** file (`.mr/constitution.md`). The constitution defines constraints, best practices, and architectural rules that influence PRD creation and execution.

#### What's the Constitution For?

The constitution provides a single source of truth for project governance:
- Define acceptance test requirements (e.g., "All UATs must be codified in Makefile.toml")
- Enforce architectural patterns (e.g., "Use anyhow::Result for all fallible functions")
- Set coding standards (e.g., "Avoid XML/JSON state blobs; use human-readable Markdown")
- Document project-specific constraints

#### How It Works

1. **Bootstrap creates it**: When you run `mr init` or `mr bootstrap`, microralph creates `.mr/constitution.md` with commented-out example rules.
2. **Version controlled**: The constitution is committed to git alongside your PRDs.
3. **Influences workflows**: Commands like `mr new` and `mr finalize` read the constitution and pass it to the LLM, which respects the rules when creating or finalizing PRDs.
4. **Intelligent editing**: Use `mr constitution edit "<request>"` to update the constitution via natural language (e.g., "Add a rule that all tests must use nextest").
5. **Violation logging**: When executing tasks, the runner logs any constitution violations in the PRD History section with reasoning—but violations do not block execution.

#### Example Constitution

```markdown
# Constitution

## Purpose
This file defines project-specific governance rules that guide PRD creation and execution.

## Rules

1. All acceptance tests must be codified in Makefile.toml (no one-off commands).
2. Use `anyhow::Result` for all fallible functions.
3. Prefer functional programming techniques where appropriate.
4. All dev commands must route through `cargo make`.
```

#### Editing the Constitution

You can edit `.mr/constitution.md` directly, or use the LLM-assisted command:

```bash
# Add a new rule via natural language
mr constitution edit "Add a rule requiring tracing instead of println for diagnostics"

# The LLM will ask clarifying questions and update the constitution
```

#### Enforcement Model

Constitution violations are **informational, not blocking**:
- The runner mentions violations in PRD History entries with reasoning
- Violations provide feedback but don't fail builds or prevent commits
- This allows flexibility while maintaining visibility into governance compliance

## User Flows

### The Complete microralph Experience

Let's walk through every way you can use microralph, from "I just heard about this" to "I'm shipping features like a boss."

#### Starting Fresh: The New Project Flow

You've got a brilliant idea and zero code. Here's how microralph helps you ralph it into existence:

```bash
# 1. Initialize your project
mkdir my-awesome-project
cd my-awesome-project
git init
mr init                          # Creates .mr/ structure, templates, and AGENTS.md

# 2. (Optional) Specify a language if not Rust
mr init --language python        # or node, go, java

# 3. Create your first PRD
mr suggest                        # Get 5 AI-generated suggestions (pick one or ignore)
mr new add-cli-parser            # Guided Q/A creates .mr/prds/PRD-0001-add-cli-parser.md

# 4. Run the loop until it's done
mr run                           # Agent implements T-001, runs tests, commits
mr run                           # Agent implements T-002, runs tests, commits
mr run                           # ... repeat until all tasks done

# 5. Check progress anytime
mr status                        # Show what's done, what's left
mr list                          # Regenerate .mr/PRDS.md index

# 6. Finalize when complete
mr finalize PRD-0001             # Mark PRD as done, append summary
```

**Pro tip**: Let `mr run` loop until the PRD is finished. Each run does one task and exits—no context bloat, no forgotten instructions.

---

#### Adding to Existing: The Bootstrap Flow

You've got a mature codebase but want to ralph new features onto it:

```bash
# 1. Bootstrap existing repo
cd my-existing-project
mr bootstrap                     # Scans repo, creates .mr/, generates starter PRDs

# 2. Review what was generated
mr list                          # See auto-generated PRDs
cat .mr/PRDS.md                  # Human-readable index

# 3. Edit or create new PRDs
mr edit PRD-0001 "Split T-003 into two tasks"  # LLM helps you edit
mr new add-auth                  # Create new PRD for auth feature

# 4. Run tasks
mr run                           # Picks highest-priority task from active PRDs
mr run PRD-0002                  # Force run from specific PRD

# 5. Stream for long tasks (optional)
mr run --stream                  # Watch the agent work in real-time
```

**Pro tip**: Use `mr suggest` after bootstrapping to get fresh ideas based on your codebase—it's like pair programming with an overenthusiastic intern who actually reads your TODO comments.

---

#### Day-to-Day: The Task Loop Flow

You're in the flow. PRDs are planned, tasks are queued. Here's your daily routine:

```bash
# Morning: Check what's up
mr status                        # "3 active PRDs, 12 tasks remaining"

# Pick your battle
mr run PRD-0003                  # Work on specific PRD
mr run                           # Let microralph pick highest priority

# Agent does the work:
# - Reads PRD and task details
# - Implements changes
# - Runs `cargo make uat` (or equivalent)
# - Updates PRD status and History
# - Commits with standardized message

# Rinse and repeat
mr run && mr run && mr run       # Chain 'em if you're feeling spicy

# End of day: Survey the damage
mr status
git log --oneline -10            # See what the agent committed
```

**Pro tip**: Use `mr run --stream` when you're actively watching—you'll see the agent's thought process unfold in real-time. Use plain `mr run` when you're grabbing coffee.

---

#### Advanced: Constitution-Driven Development

You want to enforce project rules (e.g., "All tests must use nextest," "No XML config blobs"). Enter the Constitution:

```bash
# 1. Edit your constitution
vim .mr/constitution.md          # Or use the LLM assistant:
mr constitution edit "Add rule: all functions must have doc comments"

# 2. PRD creation respects the constitution
mr new add-logging               # Agent asks about tracing vs println because constitution says so

# 3. Task execution logs violations
mr run                           # If agent violates constitution, it notes it in History
                                 # (but doesn't block—violations are informational)

# 4. Check compliance
grep -r "Constitution" .mr/prds/ # See where violations were noted
```

**Pro tip**: The constitution is git-tracked, so it evolves with your project. Update it as you learn what patterns work.

---

#### Configuration: Making microralph Yours

Don't like the defaults? Tweak them:

```bash
# Global CLI flags (override everything)
mr run --runner copilot --model claude-sonnet-4-20250514 --stream

# Persistent config (set once, forget)
cat > .mr/config.toml <<EOF
runner = "copilot"
model = "gpt-5-mini"
permission_mode = "yolo"          # Auto-approve all permissions (YOLO mode)
timeout_minutes = 60
EOF

# Now `mr run` uses your config
mr run                           # Uses gpt-5-mini, auto-approves, times out after 60min
```

**Available runners**:
- `copilot` (default) — Uses `gh copilot` CLI (requires `gh` and Copilot subscription)
- More runners coming soon (Claude, Gemini, etc.)

**Permission modes**:
- `manual` (default) — Agent asks before dangerous operations
- `yolo` — Auto-approve everything (great for trusted PRDs, terrible for unknown code)

---

#### Editing PRDs: When Plans Change

PRDs aren't set in stone. Life happens. Scope creeps. Here's how to adapt:

```bash
# Light edits: Just open the file
vim .mr/prds/PRD-0005-add-tests.md   # Change task priorities, add notes

# Heavy edits: Let the LLM help
mr edit PRD-0005 "Split T-008 into three tasks: unit tests, integration tests, E2E tests"
# Agent asks clarifying questions, updates the PRD

# Context-heavy edits: Provide upfront context
mr edit PRD-0005 --context "The auth system changed, update all tasks to use JWT"
# Agent uses context to guide questions
```

**Pro tip**: Don't delete tasks—mark them `status: parked` instead. The History section is your audit trail.

---

#### Troubleshooting: When Things Go Sideways

Agents aren't perfect. Here's how to recover:

```bash
# Task failed? Check the History
cat .mr/prds/PRD-0003-add-auth.md  # Scroll to History section
# Look for "❌ Failed" entries with failure details

# Retry the same task
mr run PRD-0003                    # Agent reads History, tries a different approach

# Skip a task manually
vim .mr/prds/PRD-0003-add-auth.md  # Change task status from `todo` to `parked`
mr run                             # Moves to next task

# Check what the agent actually did
git log -1 --stat                  # See last commit
git diff HEAD~1                    # Review changes

# Revert if needed
git reset --hard HEAD~1            # Undo last commit (if not pushed)
mr run                             # Try again
```

**Pro tip**: The History section is gold. If a task keeps failing, read past attempts—the agent learns from its mistakes (kinda).

---

#### Finalizing: Closing the Loop

All tasks done? Time to wrap it up:

```bash
# 1. Verify all tasks complete
mr status                        # Should show "0 tasks remaining"

# 2. Run UAT verification
mr run PRD-0003                  # If UATs are unverified, agent enters verification loop
                                 # For each UAT: verify, create test, or opt-out

# 3. Finalize the PRD
mr finalize PRD-0003             # Marks as `done`, appends summary to History

# 4. Celebrate
git log --oneline --graph        # Admire your git history
mr list                          # All green checkmarks
```

**Pro tip**: UATs (User Acceptance Tests) are defined in PRD frontmatter. They must be verified before finalization. If you skip verification, microralph won't let you finalize.

---

#### Multi-PRD Juggling: Parallel Workflows

Got multiple PRDs? microralph handles it:

```bash
# Create multiple PRDs
mr new add-auth
mr new refactor-db
mr new fix-ui-bugs

# microralph picks highest-priority task across ALL active PRDs
mr run                           # Might pick T-001 from PRD-0005 (priority 1)
mr run                           # Might pick T-002 from PRD-0003 (priority 2)

# Force work on specific PRD
mr run PRD-0004                  # Only run tasks from this PRD

# Check progress across all PRDs
mr status                        # Shows status of all active PRDs
```

**Pro tip**: Use priority numbers to control execution order. Priority 1 = highest. If two tasks have the same priority, microralph picks the older PRD first.

---

#### Dev Containers: Sandboxed Ralph-ing

Worried about AI-generated code trashing your machine? Use dev containers:

```bash
# 1. Generate dev container config
mr devcontainer generate         # Analyzes repo, creates .devcontainer/devcontainer.json

# 2. Open in container (VSCode)
# VSCode will prompt: "Reopen in Container" → Click it

# 3. Or use CLI
devcontainer up --workspace-folder .
devcontainer exec --workspace-folder . bash

# 4. Ralph safely inside the sandbox
mr run                           # All changes isolated to container
```

**Pro tip**: Dev containers also ensure consistent tooling across your team. No more "works on my machine" excuses.

---

#### Flags and Options: Power User Mode

Combine flags for maximum control:

```bash
# Verbose mode (see all the internals)
mr run --verbose

# Quiet mode (just the facts)
mr run --quiet

# Custom model for expensive tasks
mr run --model claude-opus-4-20250514

# Stream + specific PRD + custom model
mr run PRD-0003 --stream --model gpt-5.2-codex --verbose

# List with custom output
mr list --verbose                # More details in PRDS.md
```

**Pro tip**: `--verbose` shows token usage, model calls, and timing info. Great for debugging or cost tracking.

---

### Quick Reference: Command Cheat Sheet

| **Scenario**            | **Command**                        | **What It Does**                         |
| ----------------------- | ---------------------------------- | ---------------------------------------- |
| Start new project       | `mr init`                          | Creates `.mr/` structure                 |
| Bootstrap existing      | `mr bootstrap`                     | Scans repo, generates PRDs               |
| Get AI suggestions      | `mr suggest`                       | Analyzes codebase, suggests 5 PRDs       |
| Create PRD              | `mr new <slug>`                    | Guided Q/A creates PRD                   |
| Create PRD with context | `mr new <slug> --context "..."`    | Skips some questions                     |
| Edit PRD                | `mr edit <id> "<request>"`         | LLM helps edit PRD                       |
| Edit constitution       | `mr constitution edit "<request>"` | LLM updates project rules                |
| Run next task           | `mr run`                           | Picks highest-priority task              |
| Run from specific PRD   | `mr run <id>`                      | Only this PRD's tasks                    |
| Stream output           | `mr run --stream`                  | Watch agent work live                    |
| Check progress          | `mr status`                        | Summary of all PRDs                      |
| List PRDs               | `mr list`                          | Regenerates index                        |
| Finalize PRD            | `mr finalize <id>`                 | Mark done, append summary                |
| Reindex                 | `mr reindex`                       | Fix PRD cross-links                      |
| Generate dev container  | `mr devcontainer generate`         | Create `.devcontainer/devcontainer.json` |

---

### Common Workflows: Real-World Scenarios

#### "I want to add a feature to my app"

```bash
mr new add-user-profiles
mr run && mr run && mr run       # Until done
mr finalize PRD-0007
```

#### "I bootstrapped and got too many PRDs"

```bash
mr list                          # See all PRDs
vim .mr/prds/PRD-0003-*.md       # Change status to `parked`
mr run                           # Only runs active PRDs
```

#### "A task keeps failing"

```bash
cat .mr/prds/PRD-0005-*.md       # Read History section
mr run PRD-0005 --verbose        # See detailed failure logs
# Manually fix the issue, then:
vim .mr/prds/PRD-0005-*.md       # Update task notes with hints
mr run PRD-0005                  # Try again with new context
```

#### "I want to see what the agent is thinking"

```bash
mr run --stream --verbose        # Full transparency
```

#### "I need to enforce a coding standard"

```bash
mr constitution edit "All functions must have type hints"
mr run                           # Agent respects constitution
```

#### "I want to ralph faster"

```bash
# Use faster model for simple tasks
mr run --model gpt-5-mini

# Use yolo mode (skip permission prompts)
echo 'permission_mode = "yolo"' >> .mr/config.toml
mr run
```

---

### Tips and Tricks

- **Don't fight the loop**: Let `mr run` do its thing. Each invocation is cheap (context-wise). Run it 100 times if needed.
- **Read the History**: Failed tasks leave breadcrumbs. The agent learns from past attempts.
- **Use priorities wisely**: Lower numbers = higher priority. Use priority 1 for blocking tasks.
- **Stream when debugging**: `--stream` lets you see failures as they happen.
- **Constitution is your friend**: Define rules once, enforce everywhere.
- **Dev containers for safety**: Sandbox AI changes until you trust them.
- **Commit often**: Each `mr run` commits on success. Use git branches for risky PRDs.
- **Park, don't delete**: Mark tasks as `parked` instead of deleting. You might need them later.

---

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

| Placeholder         | Type   | Description                        |
| ------------------- | ------ | ---------------------------------- |
| `{{prd_path}}`      | string | Absolute path to the PRD file      |
| `{{prd_id}}`        | string | PRD identifier (e.g., `PRD-0001`)  |
| `{{prd_title}}`     | string | PRD title                          |
| `{{next_task_id}}`  | string | Task identifier (e.g., `T-001`)    |
| `{{task_title}}`    | string | Task title                         |
| `{{task_priority}}` | string | Task priority number               |
| `{{task_notes}}`    | string | Optional task notes (may be empty) |

### run_task_finalize.md

Used for the final wrap-up task of a PRD.

| Placeholder       | Type   | Description        |
| ----------------- | ------ | ------------------ |
| `{{prd_id}}`      | string | PRD identifier     |
| `{{prd_summary}}` | string | Summary of the PRD |

### prd_new_round1_questions.md

Used for the first round of questions when creating a new PRD.

| Placeholder               | Type   | Description                               |
| ------------------------- | ------ | ----------------------------------------- |
| `{{slug}}`                | string | The slug for the new PRD                  |
| `{{user_description}}`    | string | Optional initial description from user    |
| `{{user_context}}`        | string | Optional upfront context provided by user |
| `{{#each existing_prds}}` | list   | Existing PRDs for context                 |
| ↳ `{{id}}`                | string | PRD identifier                            |
| ↳ `{{title}}`             | string | PRD title                                 |
| ↳ `{{status}}`            | string | PRD status (draft/active/done/parked)     |

### prd_new_roundN_questions.md

Used for follow-up rounds of questions during PRD creation.

| Placeholder            | Type   | Description                               |
| ---------------------- | ------ | ----------------------------------------- |
| `{{slug}}`             | string | The slug for the new PRD                  |
| `{{user_context}}`     | string | Optional upfront context provided by user |
| `{{#each qa_history}}` | list   | Previous Q/A pairs                        |
| ↳ `{{question}}`       | string | The question that was asked               |
| ↳ `{{answer}}`         | string | The user's answer                         |
| ↳ `{{@index}}`         | number | 0-based index of the Q/A pair             |

### prd_new_synthesize_prd.md

Used to synthesize the final PRD from collected Q/A.

| Placeholder               | Type   | Description                               |
| ------------------------- | ------ | ----------------------------------------- |
| `{{slug}}`                | string | The slug for the new PRD                  |
| `{{user_context}}`        | string | Optional upfront context provided by user |
| `{{#each qa_history}}`    | list   | All Q/A pairs from the session            |
| ↳ `{{question}}`          | string | The question                              |
| ↳ `{{answer}}`            | string | The answer                                |
| `{{#each existing_prds}}` | list   | Existing PRDs for context                 |
| ↳ `{{id}}`                | string | PRD identifier                            |
| ↳ `{{title}}`             | string | PRD title                                 |

### prd_edit.md

Used when editing an existing PRD via `mr edit`.

| Placeholder            | Type   | Description                  |
| ---------------------- | ------ | ---------------------------- |
| `{{prd_path}}`         | string | Path to the PRD file         |
| `{{user_request}}`     | string | The user's edit request      |
| `{{prd_content}}`      | string | Current PRD file content     |
| `{{#each qa_history}}` | list   | Follow-up Q/A pairs (if any) |
| ↳ `{{question}}`       | string | The question                 |
| ↳ `{{answer}}`         | string | The answer                   |

### bootstrap_plan.md

Used during `mr bootstrap` to analyze the repository.

| Placeholder            | Type   | Description                        |
| ---------------------- | ------ | ---------------------------------- |
| `{{prd_budget}}`       | string | Maximum number of PRDs to generate |
| `{{#each heuristics}}` | list   | Analysis heuristics                |
| ↳ `{{description}}`    | string | Heuristic description              |

### bootstrap_generate_prds.md

Used to generate PRDs from the bootstrap plan.

| Placeholder      | Type   | Description                        |
| ---------------- | ------ | ---------------------------------- |
| `{{plan}}`       | string | The generated bootstrap plan       |
| `{{prd_budget}}` | string | Maximum number of PRDs to generate |

### update_agents.md

Used to update the auto-managed section of AGENTS.md.

| Placeholder                | Type   | Description                |
| -------------------------- | ------ | -------------------------- |
| `{{agents_content}}`       | string | Current AGENTS.md content  |
| `{{#each recent_changes}}` | list   | Recent file changes        |
| ↳ `{{file}}`               | string | File path that was changed |
| ↳ `{{description}}`        | string | Description of the change  |

### adapt_language.md

Used when initializing for a non-Rust language.

| Placeholder                | Type   | Description                              |
| -------------------------- | ------ | ---------------------------------------- |
| `{{language}}`             | string | Target language (e.g., `python`, `node`) |
| `{{#each build_commands}}` | list   | Typical build/test commands              |
| ↳ `{{command}}`            | string | A build/test command                     |

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

### The Ralph Pattern

**Ralph** is a pattern where you repeatedly invoke an AI coding agent in a loop until a task is complete. The original concept emerged in the AI coding community as a way to overcome context window limitations by running fresh agent sessions iteratively.

A project that is predominantly built this way—a **ralph**—becomes a testament to the pattern's power: AI does the heavy lifting while you steer with PRDs and review results.

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
5. **Guided workflows**: `mr new` and `mr bootstrap` help structure work
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

| Feature                         |    microralph     |   Claude Code   |     Cursor      |      Aider      |      Cline      |
| ------------------------------- | :---------------: | :-------------: | :-------------: | :-------------: | :-------------: |
| **PRD-driven task breakdown**   |         ✅         |        ❌        |        ❌        |        ❌        |        ❌        |
| **One-task-per-run (no bloat)** |         ✅         |        ❌        |        ❌        |        ❌        |        ❌        |
| **Git-native state**            |         ✅         |        ❌        |        ❌        |        ✅        |        ❌        |
| **History/retry logging**       |         ✅         |        ❌        |        ❌        |   ⚠️ (partial)   |        ❌        |
| **Multi-runner abstraction**    |         ✅         | ❌ (Claude only) | ❌ (Cursor only) | ⚠️ (multi-model) | ❌ (VSCode only) |
| **Works in terminal**           |         ✅         |        ✅        |  ❌ (IDE only)   |        ✅        |  ❌ (IDE only)   |
| **No API keys required**        | ✅ (uses CLI auth) |        ✅        |        ✅        |        ❌        |        ✅        |
| **Customizable prompts**        |         ✅         |        ❌        |        ❌        |        ⚠️        |        ❌        |

### Why microralph is Different

Most AI coding tools are **session-based**: you start a conversation, describe what you want, and the agent tries to do everything in one go. This works for small tasks but breaks down for larger projects:

- **Context bloat**: Long sessions accumulate context until the agent gets confused
- **No persistence**: If you close the session, you start over
- **No structure**: There's no clear definition of "done" or progress tracking

microralph is **task-based**: you define discrete tasks upfront, and each `mr run` tackles exactly one task with fresh context. Progress is tracked in git, so you can close your terminal, reboot your machine, or come back weeks later—microralph picks up where it left off.

Think of it as the difference between "do everything in one meeting" vs. "complete one ticket per sprint" — the latter scales.

## License

MIT
