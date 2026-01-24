# Development Guide

This document covers development workflows, architecture, and contribution guidelines for microralph.

## Prerequisites

```bash
# Install Rust (if not present)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install cargo-make (required for dev workflows)
cargo install cargo-make

# Install cargo-nextest (faster test runner, optional but recommended)
cargo binstall cargo-nextest --no-confirm
# or: cargo install cargo-nextest
```

## Development Commands

All dev workflows route through `cargo make`:

```bash
# Run tests (uses nextest)
cargo make test

# Run full CI pipeline (fmt, clippy, test)
cargo make ci

# Format code
cargo make fmt

# Run clippy lints
cargo make clippy

# Build release binary
cargo make build-release

# UAT — the one true gate
cargo make uat
```

### Running Specific Tests

```bash
# Run a specific test
cargo nextest run test_name

# Run tests matching a pattern
cargo nextest run parser

# Run tests with output
cargo nextest run --no-capture test_name
```

## Project Structure

```
microralph/
├── src/
│   ├── main.rs           # CLI entrypoint, command routing
│   ├── init.rs           # `mr init` implementation
│   ├── bootstrap.rs      # `mr bootstrap` implementation
│   ├── run.rs            # `mr run` task execution
│   ├── status.rs         # `mr status` implementation
│   ├── reindex.rs        # `mr reindex` implementation
│   ├── config.rs         # Config file loading (.mr/config.toml)
│   ├── agents.rs         # AGENTS.md updater
│   ├── prd_new.rs        # `mr prd new` Q/A flow
│   ├── prd_edit.rs       # `mr prd edit` flow
│   ├── prd/              # PRD parsing and manipulation
│   │   ├── mod.rs
│   │   ├── parser.rs     # YAML frontmatter + markdown body parsing
│   │   ├── index.rs      # PRD index generation
│   │   └── types.rs      # PRD data structures
│   ├── prompt/           # Prompt system
│   │   ├── mod.rs
│   │   ├── types.rs      # PromptKind enum
│   │   ├── loader.rs     # Load prompts from .mr/prompts/
│   │   └── expand.rs     # Placeholder expansion (Handlebars-lite)
│   └── runner/           # Runner abstraction
│       ├── mod.rs
│       ├── types.rs      # Runner trait
│       ├── copilot.rs    # GitHub Copilot CLI adapter
│       └── mock.rs       # Mock runner for tests
├── .mr/                  # microralph state (dogfooding)
│   ├── config.toml       # Configuration
│   ├── PRDS.md           # Auto-generated PRD index
│   ├── prds/             # PRD files
│   ├── prompts/          # Static prompt files
│   └── templates/        # PRD template
├── AGENTS.md             # Agent instructions (for AI coding agents)
├── Cargo.toml            # Rust dependencies
├── Makefile.toml         # cargo-make task definitions
└── rust-toolchain.toml   # Rust version pinning
```

## Architecture

### Core Loop

```
mr run
  │
  ├─► Pick PRD (first incomplete by priority)
  │
  ├─► Pick Task (highest priority, status=todo)
  │
  ├─► Build Prompt (load .mr/prompts/run_task.md, expand placeholders)
  │
  ├─► Invoke Runner (copilot, mock, etc.)
  │     │
  │     └─► Runner executes: implement → UAT → update PRD → commit
  │
  └─► Exit (context freed for next run)
```

### Runner Abstraction

Runners implement the `Runner` trait:

```rust
pub trait Runner: Send + Sync {
    fn execute(&self, prompt: &str) -> Result<RunnerOutput>;
    fn execute_streaming(&self, prompt: &str, output: &mut dyn Write) -> Result<RunnerOutput>;
}
```

- **CopilotRunner**: Shells out to `copilot-cli` (GitHub Copilot CLI)
- **MockRunner**: Returns scripted responses for testing

### Prompt System

Prompts are static markdown files in `.mr/prompts/` with Handlebars-style placeholders:

- `{{variable}}` — String substitution
- `{{#if var}}...{{/if}}` — Conditional
- `{{#each list}}...{{/each}}` — List iteration

See the README's "Prompt Placeholders" section for all available variables.

### PRD Format

PRDs use YAML frontmatter + Markdown body:

```yaml
---
id: PRD-0001
title: My Feature
status: active  # draft | active | done | parked
tasks:
  - id: T-001
    title: "Do the thing"
    priority: 1
    status: todo  # todo | in-progress | done
---
```

The body contains Summary, sections, and History (appended by `mr run`).

## Testing Strategy

### Unit Tests

Most modules have inline `#[cfg(test)]` tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_parse_prd() {
        // ...
    }
}
```

### Integration Tests

Integration tests use `MockRunner` to simulate agent responses without hitting real APIs.

### UAT

`cargo make uat` is the acceptance gate. It runs:
- Format check
- Clippy lints
- All tests

PRs must pass UAT before merge.

## Code Style

- Use `anyhow::Result` for fallible functions
- Prefer `tracing` over `println!` for diagnostics
- Use vertical whitespace to separate logical sections
- Reduce nesting with guard clauses and early returns
- Prefer functional patterns where appropriate
- No `#[allow(...)]` without a reason comment

## Adding a New Command

1. Add variant to the CLI enum in `main.rs`
2. Create `src/my_command.rs` with implementation
3. Add `PromptKind` variant if the command uses prompts
4. Add default prompt to `PROMPT_*` constants in `init.rs`
5. Add tests
6. Update README command table

## Adding a New Runner

1. Create `src/runner/my_runner.rs`
2. Implement `Runner` trait
3. Add to runner factory in `runner/mod.rs`
4. Add to `allow_runners` in PRD frontmatter schema

## Debugging

### Verbose Output

```bash
mr run -v  # Shows debug-level logs
```

### Streaming Mode

```bash
mr run --stream  # Watch agent output in real-time
```

### Mock Runner

For development without invoking real agents:

```bash
mr run --runner mock
```

## Troubleshooting

### "cargo-make not found"

```bash
cargo install cargo-make
```

### "cargo-nextest not found"

```bash
cargo binstall cargo-nextest --no-confirm
# or
cargo install cargo-nextest
```

### "copilot-cli not found"

Install GitHub Copilot CLI:
```bash
gh extension install github/gh-copilot
```

### Tests Failing on CI

Check if tests require specific environment:
- Some tests use temp directories
- Mock runner tests are self-contained
- Integration tests may need `.mr/` structure

## Release Process

1. Update version in `Cargo.toml`
2. Run `cargo make uat`
3. Commit: `git commit -m "chore: bump version to X.Y.Z"`
4. Tag: `git tag vX.Y.Z`
5. Push: `git push origin main --tags`
6. CI builds and publishes to crates.io

## Upcoming PRD / Change Ideas

- Ability to select options in CLI, instead of typing all commands.  Does clap support this?

## License

MIT
