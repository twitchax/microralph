# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **PRD-0004: PRD Finalization Steps** — Explicit `mr prd finalize <id>` command
  - Task completion validation (blocks finalization if tasks are incomplete)
  - Acceptance test verification via finalization prompt
  - Automatic CHANGELOG.md creation with Keep a Changelog format
  - Summary report generation (stdout + PRD history)
  - PRD status update to `done` and PRDS.md index refresh
  - Inter-PRD cross-references in the index
  - Cleanup guidance in finalization prompt (temp files, excessive comments)

- **PRD-0001: Build microralph MVP** — Complete CLI tool for PRD-driven development
  - `mr init` — Initialize a new repo with `.mr/` structure, templates, and prompts
  - `mr bootstrap` — Ingest existing repos and auto-generate PRDs with language detection
  - `mr prd new` — Guided Q/A flow to create PRDs interactively
  - `mr prd edit` — Quick PRD modifications via runner
  - `mr prd list` — List all PRDs with status and progress
  - `mr run` — Execute tasks one at a time with optional streaming output
  - `mr status` — Display PRD status summary and next task
  - `mr reindex` — Regenerate PRD index and verify inter-PRD links
  - Static prompt library with placeholder expansion (`{{variable}}`, `{{#if}}`, `{{#each}}`)
  - Runner abstraction with CopilotRunner (GitHub Copilot CLI) and MockRunner for testing
  - YAML frontmatter PRD format with automatic status tracking and history
  - Auto-managed AGENTS.md section for agent conventions
  - Configuration via `.mr/config.toml` (model, runner, permissions)
  - 227 tests, comprehensive documentation (README.md, DEVELOPMENT.md, AGENTS.md)
