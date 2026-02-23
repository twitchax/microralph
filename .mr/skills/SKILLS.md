# Skills

<!-- This file is auto-managed by the run agent. Each entry is a one-line summary. -->
<!-- Read .mr/skills/<name>/skill.md for full details on any skill. -->

- **cargo-make-workflow**: Use `cargo make` tasks for all dev operations — never raw `cargo` commands for CI, testing, or releases.
- **clippy-pedantic-compliance**: All production code enforces `clippy::pedantic` — know common lints and how to satisfy them without suppression.
- **prompt-sync-constitution-rule7**: Prompt templates must be defined as constants in `src/commands/init.rs` AND materialized to `.mr/prompts/` — both must stay in sync.
- **runner-implementation-pattern**: Follow the established pattern when implementing or modifying runners — config struct, build_args, token parsing, and mock testing.
- **prd-frontmatter-editing**: Correctly edit PRD YAML frontmatter — quote strings with colons/hashes, preserve structure, and follow status conventions.
- **anyhow-error-handling**: Use `anyhow::Result` for all fallible functions, `.context()` for error enrichment, and never `.unwrap()` in production code.
- **test-with-mock-runners**: All runner-dependent tests use `MockRunner` — never require actual CLI tools to be installed.
