# microralph — Agents Guide

This document provides detailed workflows and troubleshooting for AI coding agents working in this repository.

## Workspace Overview

- `src/`: Main Rust source code
  - `commands/`: CLI command implementations (bootstrap, devcontainer, graph, init, refactor, reindex, run, status, suggest, validate)
  - `config/`: Configuration loading and constitution editing
  - `prd/`: PRD types, parsing, indexing, and operations (edit, new, finalize)
  - `prompt/`: Prompt loading and expansion
  - `runner/`: Runner implementations (copilot, claude, mock)
  - `util/`: Shared utilities (colors, spinner, qa_workflow)
  - `main.rs`: CLI entry point
  - `changelog.rs`: Changelog generation
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

# Dev container (start and exec into container)
cargo make devcontainer
```

## Suggest Command Workflow

The `mr suggest` command uses AI to analyze the codebase and generate PRD suggestions:

1. **Analysis Phase**: Scans repository structure, existing PRDs, git history, TODO comments, and dependency versions
2. **Generation**: Produces exactly 5 suggestions with title, description, category, effort estimate, and rationale
3. **Selection**: Displays numbered picker (1-5 or 'q' to quit) for user to choose a suggestion
4. **Integration**: Selected suggestion flows into `mr new` with pre-filled context

Suggestions balance strategic features with quick wins. The command follows existing CLI patterns from PRD-0009.

## Bootstrap Workflow

The `mr bootstrap` command reconstructs PRDs from git history by default:

1. **Git Analysis**: LLM analyzes commits, tags, and major changes to identify historical milestones
2. **PRD Creation**: Creates PRDs for each milestone with `status: done` and `reconstructed: true`
3. **Dependency Inference**: Infers `depends_on` relationships from temporal order
4. **Idempotency**: Skips existing PRDs to avoid duplication

### Usage

```bash
# Reconstruct PRDs from git history (default behavior)
mr bootstrap

# With specific runner and model
mr bootstrap --runner claude --model claude-sonnet-4.5

# With language hint
mr bootstrap --language rust

# Scaffold mode: skip git history analysis and create an initial PRD for bootstrapping
mr bootstrap --scaffold
```

### Flags Reference

| Flag         | Default | Description                                                                      |
| ------------ | ------- | -------------------------------------------------------------------------------- |
| `--scaffold` | false   | Skip git history analysis; create an initial PRD for bootstrapping new projects |
| `--runner`   | copilot | Runner to use (copilot, claude)                                                  |
| `--model`    | None    | Model override for the runner                                                    |
| `--language` | None    | Target language hint (rust, python, node, go, java)                              |
| `--stream`   | false   | Stream runner output in real-time                                                |

### Important Notes

- **Default is reconstruct**: Running `mr bootstrap` without flags analyzes git history to create PRDs for completed work
- **Scaffold mode**: Use `--scaffold` for new projects or when you want to skip git history analysis and create a fresh starting PRD
- **Reconstructed PRDs**: Created with `reconstructed: true` in frontmatter to distinguish from manually created PRDs
- **Dependency inference**: Uses temporal ordering of commits/tags to infer `depends_on` relationships
- **Existing PRD awareness**: Scans existing PRDs and avoids creating duplicates for already-documented work

## Graph Command Workflow

The `mr graph` command visualizes PRD dependencies in multiple output formats:

1. **Graph Building**: Scans all PRDs and builds a dependency graph from `depends_on` fields
2. **Missing Reference Handling**: References to non-existent PRDs are shown as dashed/special nodes with warnings
3. **Format Rendering**: Outputs graph in chosen format (ASCII, Mermaid, or DOT)

### Usage

```bash
# Render ASCII art graph in terminal
mr graph ascii

# Render Mermaid flowchart syntax (for GitHub/GitLab rendering)
mr graph mermaid

# Render Graphviz DOT format
mr graph dot

# Left-to-right layout (Mermaid and DOT only)
mr graph mermaid --lr
mr graph dot --lr

# Hide titles, show only PRD IDs
mr graph ascii --no-titles

# Limit title length
mr graph ascii --max-title-len 20
```

### Subcommands

| Subcommand | Description                                       |
| ---------- | ------------------------------------------------- |
| `ascii`    | Render as ASCII art for terminal viewing          |
| `mermaid`  | Output Mermaid flowchart syntax for GitHub/GitLab |
| `dot`      | Output Graphviz DOT format for external tools     |

### Flags Reference

| Flag              | Default | Subcommands      | Description                                     |
| ----------------- | ------- | ---------------- | ----------------------------------------------- |
| `--no-titles`     | false   | all              | Show only PRD IDs, hide titles                  |
| `--max-title-len` | 40      | all              | Maximum title length before truncation          |
| `--lr`            | false   | mermaid, dot     | Render left-to-right instead of top-to-bottom   |

### Output Examples

**ASCII output** shows nodes with dependencies indented:
```
PRD Dependency Graph
====================
[PRD-0001] Initial Setup (done)
├── [PRD-0002] Add Auth (done)
└── [PRD-0003] Add API (active)
    └── [PRD-0004] Add Tests (todo)
```

**Mermaid output** can be embedded in GitHub READMEs:
```mermaid
flowchart TD
    PRD0001["PRD-0001: Initial Setup (done)"]
    PRD0002["PRD-0002: Add Auth (done)"]
    PRD0002 --> PRD0001
```

**DOT output** can be rendered with Graphviz:
```dot
digraph PRDs {
    rankdir=TB;
    PRD0001 [label="PRD-0001: Initial Setup (done)"]
    PRD0002 [label="PRD-0002: Add Auth (done)"]
    PRD0002 -> PRD0001
}
```

### Important Notes

- **Missing references**: If a PRD references a non-existent `depends_on` ID, it appears as a dashed node with a warning
- **PRDS.md integration**: The index now includes a Dependencies section showing `depends_on` relationships
- **Reindex enhancement**: Running `mr reindex` will auto-fix `depends_on` relationships using LLM analysis

## Refactor Command Workflow

The `mr refactor` command runs an iterative AI-driven loop to improve code quality:

1. **Iteration Loop**: Runs up to N iterations (default: 3, configurable with `--max`)
2. **Analysis**: Agent analyzes codebase against constitution principles
3. **Application**: Agent applies one focused refactor per iteration
4. **Verification**: Runs `cargo make uat` to ensure no regressions
5. **Commit**: Each iteration commits separately (unless `--no-commit`)

### Usage

```bash
# Basic usage (3 iterations)
mr refactor

# Custom iteration count
mr refactor --max 5

# Focus on specific improvements
mr refactor --context "improve error handling"

# Constrain to specific path
mr refactor --path src/runner/

# Preview without applying (dry-run)
mr refactor --dry-run

# Skip commits (for manual review)
mr refactor --no-commit

# Use specific runner and model
mr refactor --runner claude --model claude-sonnet-4.5

# Stream output in real-time
mr refactor --stream
```

### Flags Reference

| Flag          | Default | Description                                                 |
| ------------- | ------- | ----------------------------------------------------------- |
| `--max`       | 3       | Maximum number of refactor iterations                       |
| `--context`   | None    | Focus hint for the agent (takes priority over constitution) |
| `--path`      | None    | Constrain scope to specific directory/file pattern          |
| `--dry-run`   | false   | Preview suggestions without applying changes                |
| `--no-commit` | false   | Leave changes uncommitted for manual review                 |
| `--runner`    | copilot | Runner to use (copilot, claude)                             |
| `--model`     | None    | Model override for the runner                               |
| `--stream`    | false   | Stream runner output in real-time                           |

### Termination Signals

- **`NO-MORE-REFACTORS`**: Agent signals no more impactful refactors remain (early termination)
- **`PREVIEW-COMPLETE`**: Agent completed dry-run suggestion (in preview mode)

### Constitution Integration

When no `--context` is provided, the agent uses the project's constitution (`.mr/constitution.md`) to guide refactor selection. Common constitution-driven improvements include:
- DRY (Don't Repeat Yourself) violations
- Separation of Concerns improvements
- Consistency with existing patterns
- Root cause fixes over workarounds

### Important Notes

- **Self-contained iterations**: Each iteration is independent (no cross-iteration memory)
- **UAT gating**: Refactors are only committed if UATs pass
- **Minimal changes**: Each iteration makes one focused change
- **Git safety**: Always use `git diff` to review changes before pushing

## Restore Command Workflow

The `mr restore` command overwrites `.mr/prompts/`, `.mr/templates/`, `constitution.md`, and `config.toml` with built-in defaults:

1. **Pre-flight check**: Verifies that `mr init` has been run (`.mr/` directory exists)
2. **Deletion phase**: Removes `.mr/prompts/` and `.mr/templates/` directories using `std::fs::remove_dir_all`
3. **Recreation phase**: Calls `init::init_prompts_and_templates()` to recreate directories with built-in defaults
4. **Config restoration**: Calls `init::init_constitution_and_config()` to overwrite `constitution.md` and `config.toml`
5. **No auto-commit**: Leaves changes uncommitted so users can review via Git workflow

### Use Cases

- **Reset customizations**: Agents who have modified prompts/templates/constitution/config can restore defaults to test baseline behavior
- **Update after upgrade**: After microralph version upgrades, restore to get latest built-in prompts/templates
- **Compare customizations**: Use Git diff to see differences between custom and default prompts

### Important Notes

- **Destructive**: Overwrites existing files with no backup (Git is the safety net)
- **Git workflow**: Always use `git diff` to review changes before committing
- **Scope**: Affects `.mr/prompts/`, `.mr/templates/`, `constitution.md`, and `config.toml` (not PRDs or PRDS.md)
- **Idempotent**: Running multiple times produces the same result (all files overwritten with built-in defaults)

### Implementation Pattern

The restore command follows the DRY principle by reusing `init::init_prompts_and_templates()`, which is also used by `mr init`. This ensures consistent file-writing logic and reduces code duplication.

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

## Prompt vs Constitution Philosophy

microralph separates agent guidance into two distinct concerns:

- **Prompts** (`.mr/prompts/`): Define **what to do** — workflow steps, required actions, and task-specific constraints. Prompts are operational and workflow-focused.
- **Constitution** (`.mr/constitution.md`): Defines **how to behave** — behavioral rules, coding principles, and project-wide constraints like DRY, minimal changes, and consistency.

This separation provides a single source of truth for agent behavior. To change how agents behave across all workflows, edit the constitution. Individual prompts remain focused on their specific workflow without duplicating behavioral guidance.

### Customization Guide

- **Change agent behavior globally**: Edit `.mr/constitution.md` only
- **Change a specific workflow**: Edit the relevant prompt in `.mr/prompts/`
- **Reset to defaults**: Run `mr restore` to overwrite prompts and templates with built-in defaults

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
- `depends_on`: Optional list of PRD IDs this PRD depends on (e.g., `["PRD-0001", "PRD-0003"]`)
- `reconstructed`: Optional boolean, true if PRD was created via `mr bootstrap` (git history reconstruction)
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

# Dev container
cargo make devcontainer
```

## Release Workflow

microralph uses a fully automated release process powered by cargo-make tasks.

### Automated Release (Recommended)

```bash
# Single command: CI → changelog → bump → commit → push → wait for artifacts
cargo make release

# After artifacts are downloaded, publish:
cargo make publish-all vX.Y.Z
```

The `release` task automatically:
1. Runs full CI pipeline (fmt, clippy, test)
2. Generates changelog with git-cliff
3. Bumps version using cargo-release
4. Commits and pushes changes (including tags)
5. Waits for GitHub Actions to complete
6. Downloads artifacts to `target/release-artifacts/`

### Manual Release Workflow (Advanced)

For more control, run individual tasks:

```bash
# Generate/update changelog from conventional commits
cargo make changelog

# Version bump with cargo-release (dry-run to preview)
cargo make release-bump

# Build platform-specific binaries (done automatically by CI on main)
cargo make build-linux
cargo make build-macos
cargo make build-windows
cargo make build-wasm

# Publish to crates.io (dry-run recommended first)
cargo make publish-crates -- --dry-run
cargo make publish-crates

# Create GitHub release with binaries
cargo make github-release v0.1.0
cargo make github-release v0.1.0 --draft  # Create as draft
```

### Release Tasks Reference

| Task                   | Description                                                            |
| ---------------------- | ---------------------------------------------------------------------- |
| `release`              | Fully automated: CI → changelog → bump → commit → push → wait/download |
| `release-bump`         | Bumps version using cargo-release                                      |
| `publish-all <tag>`    | Publishes to crates.io and creates GitHub release in one go            |
| `changelog`            | Generates CHANGELOG.md from conventional commits                       |
| `publish-crates`       | Publishes to crates.io with pre-publish checks                         |
| `github-release <tag>` | Creates GitHub release with artifacts                                  |
| `build-linux`          | Builds Linux x86_64 binary                                             |
| `build-macos`          | Builds macOS ARM binary                                                |
| `build-windows`        | Builds Windows x86_64 binary                                           |
| `build-wasm`           | Builds WASM32-WASIP2 binary                                            |
| `build-oci`            | Alias for build-wasm (for OCI publishing)                              |
| `publish-oci`          | Publishes WASM binary to GitHub Container Registry                     |

## Dev Container Workflow

microralph supports development containers for consistent, sandboxed environments. The `devcontainer` cargo-make task automates the entire setup:

```bash
# One command to install dependencies, start container, and exec into it
cargo make devcontainer
```

This task:
1. Checks for `@devcontainers/cli` and installs it via npm if missing
2. Verifies `.devcontainer/devcontainer.json` exists (prompts to generate if not)
3. Starts the dev container with `--remove-existing-container` (equivalent to `--rm`)
4. Execs into the container with bash

### Prerequisites

- **Node.js/npm**: Required to install `@devcontainers/cli`
- **Docker**: Must be installed and running
- **Dev container config**: Generate with `cargo run -- devcontainer generate`

### Manual Steps (if needed)

```bash
# Generate config
cargo run -- devcontainer generate

# Then use the automated task
cargo make devcontainer

# Or manual approach:
npm install -g @devcontainers/cli
devcontainer up --workspace-folder . --remove-existing-container
devcontainer exec --workspace-folder . bash
```

## Troubleshooting

- If `cargo-make` is missing: `cargo install cargo-make`
- If `cargo-nextest` is missing: `cargo binstall cargo-nextest --no-confirm`
- For faster tool installation, use cargo-binstall:
  ```bash
  curl -L --proto '=https' --tlsv1.2 -sSf https://raw.githubusercontent.com/cargo-bins/cargo-binstall/main/install-from-binstall-release.sh | bash
  ```

---