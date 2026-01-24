# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **PRD-0007: Output Underlying Agent Usage** — Token usage tracking and display for CopilotRunner
  - Added `UsageInfo` struct to capture input/output/total token metrics from runners
  - CopilotRunner parses token usage from Copilot CLI output (format: "18.3k in, 38 out")
  - Token usage displayed after runner output during `mr run` iterations
  - Graceful degradation when runners don't provide usage metrics
  - Support for k/M suffixes in token counts

- **PRD-0006: Add Stdout Colors** — Terminal colorization and emoji enhancements for CLI output
  - Added `owo-colors` dependency with automatic TTY detection
  - Color utilities module with success (green), error (red), warning (yellow), info (cyan), question (blue bold), header (bold), and dim styling
  - Colorized success, error, warning, informational, and question messages throughout CLI
  - Automatic color degradation for piped output and `NO_COLOR` environment variable support
  - Enhanced finalization summary box with styled separators and headers

- **PRD-0003: PRD New Allows Upfront Context** — Enhanced `mr prd new` with optional upfront context
  - `--context` CLI flag to provide context directly, skipping interactive prompt
  - Interactive context prompt before question generation ("Do you want to add more context?")
  - Context persistence through all Q/A rounds and final PRD synthesis
  - AI uses upfront context for more relevant, targeted questions from the first round

- **PRD-0005: Verify UATs at End of Run Loop** — Dedicated UAT verification phase after task completion
  - UAT verification loop that triggers automatically when all tasks are done
  - Three verification approaches: run existing tests, create new tests, or opt-out with explanation
  - Model opt-out mechanism with automatic History entry appending
  - UAT status updates written back to PRD frontmatter (`uat_status: verified`)
  - Respects `loop.max_iterations` config to bound verification iterations
  - Unverified UATs block `mr prd finalize` from succeeding
  - New `run_uat_verify.md` prompt template for verification instructions
  - `RunResult` enum with variants: `TaskExecuted`, `NeedsUatVerification`, `PrdComplete`

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
