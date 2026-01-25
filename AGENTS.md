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

## Restore Command Workflow

The `mr restore` command overwrites `.mr/prompts/` and `.mr/templates/` with built-in defaults:

1. **Pre-flight check**: Verifies that `mr init` has been run (`.mr/` directory exists)
2. **Deletion phase**: Removes `.mr/prompts/` and `.mr/templates/` directories using `std::fs::remove_dir_all`
3. **Recreation phase**: Calls `init::init_prompts_and_templates()` to recreate directories with built-in defaults
4. **No auto-commit**: Leaves changes uncommitted so users can review via Git workflow

### Use Cases

- **Reset customizations**: Agents who have modified prompts/templates can restore defaults to test baseline behavior
- **Update after upgrade**: After microralph version upgrades, restore to get latest built-in prompts/templates
- **Compare customizations**: Use Git diff to see differences between custom and default prompts

### Important Notes

- **Destructive**: Overwrites existing files with no backup (Git is the safety net)
- **Git workflow**: Always use `git diff` to review changes before committing
- **Scope limited**: Only affects `.mr/prompts/` and `.mr/templates/` (not constitution or config)
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

# Dev container
cargo make devcontainer
```

## Release Workflow

microralph uses a multi-step release process powered by cargo-make tasks. There are two main workflows: **unified** (recommended) and **manual** (for advanced scenarios).

### Unified Release Workflow (Recommended)

```bash
# Step 1: Prepare release (runs CI, builds all targets, generates changelog)
cargo make release

# Step 2: Review CHANGELOG.md, bump version, commit and push
cargo make release-bump   # Bumps version with cargo-release
git add .
git commit -m "chore: prepare release vX.Y.Z"
git push

# Step 3: Wait for CI to complete, then download artifacts
gh run list --branch main --workflow build.yml --limit 1 --json databaseId --jq '.[0].databaseId' | \
  xargs -I {} gh run download {} --dir release-artifacts

# Step 4: Publish to crates.io and create GitHub release in one command
cargo make publish-all vX.Y.Z
# Or create draft: cargo make publish-all vX.Y.Z --draft
```

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

| Task                   | Description                                                    |
| ---------------------- | -------------------------------------------------------------- |
| `release`              | Unified task: runs CI, builds all targets, generates changelog |
| `release-bump`         | Bumps version using cargo-release (formerly just `release`)    |
| `publish-all <tag>`    | Publishes to crates.io and creates GitHub release in one go    |
| `changelog`            | Generates CHANGELOG.md from conventional commits               |
| `publish-crates`       | Publishes to crates.io with pre-publish checks                 |
| `github-release <tag>` | Creates GitHub release with artifacts                          |
| `build-linux`          | Builds Linux x86_64 binary                                     |
| `build-macos`          | Builds macOS ARM binary                                        |
| `build-windows`        | Builds Windows x86_64 binary                                   |
| `build-wasm`           | Builds WASM32-WASIP2 binary                                    |
| `build-oci`            | Alias for build-wasm (for OCI publishing)                      |
| `publish-oci`          | Publishes WASM binary to GitHub Container Registry             |

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