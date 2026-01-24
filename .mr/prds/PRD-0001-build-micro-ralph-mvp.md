---
id: PRD-0001
title: Build microralph MVP
status: active                 # draft | active | done | parked
owner: Aaron Roney
created: 2026-01-23
updated: 2026-01-23

product_name: microralph
binary_name: mr

state_dir: .mr
prds_dir: .mr/prds
index_file: .mr/PRDS.md
templates_dir: .mr/templates
prompts_dir: .mr/prompts
agents_file: AGENTS.md

principles:
  - "No direct API calls: microralph shells out to runner CLIs only."
  - "State lives in git + Markdown PRDs (YAML frontmatter + History section)."
  - "One-or-zero tasks per `mr run` invocation."
  - "Runner can fail; we still append History so the next run has context."
  - "Avoid XML/JSON state blobs."
  - "Almost all dev workflows run via `cargo make` (fmt/clippy/test/ci/uat wrappers)."

references:
  - name: "kord (style reference for CI + README + repo ergonomics)"
    url: https://github.com/twitchax/kord            # use as reference patterns :contentReference[oaicite:1]{index=1}

runner:
  default: copilot
  allow_runners: [copilot, mock]     # typed adapters; more later
  permissions_mode: yolo             # yolo | allow_all | manual
  fallback_flags:
    - "--allow-all-tools"
    - "--allow-all-paths"
    - "--allow-all-urls"

git:
  branch_mode: current               # current | feature
  feature_branch_prefix: mr/
  commit_policy: auto_clean          # never | auto_clean | always

loop:
  prd_pick: first_incomplete         # first_incomplete | by_priority | explicit
  task_pick: highest_priority        # highest_priority | oldest_incomplete
  max_iterations: 1                  # `mr run` attempts one task max
  max_task_attempts: 8               # ergonomic limit across history entries per task
  max_session_minutes: 30            # per runner invocation (timeout)
  max_transcript_kb: 128             # stored into PRD history (truncate beyond)

bootstrap:
  generate_index: true
  generate_prds: true
  prd_budget: 6
  heuristics:
    - "Detect cargo-make entrypoints and required tasks"
    - "Detect crates/modules and responsibilities"
    - "Detect CI workflows and required checks"
    - "Detect docs that imply features (README/DEVELOPMENT/etc.)"
    - "Detect TODO/FIXME hotspots"

prompts:
  # Static prompt files are committed and versioned. They are the “system prompt” layer for each stage.
  # microralph fills placeholders and passes to the runner CLI.
  init: ".mr/prompts/init.md"
  bootstrap_plan: ".mr/prompts/bootstrap_plan.md"
  bootstrap_generate_prds: ".mr/prompts/bootstrap_generate_prds.md"
  prd_new_round1_questions: ".mr/prompts/prd_new_round1_questions.md"
  prd_new_roundN_questions: ".mr/prompts/prd_new_roundN_questions.md"
  prd_new_synthesize_prd: ".mr/prompts/prd_new_synthesize_prd.md"
  run_task: ".mr/prompts/run_task.md"
  run_task_finalize: ".mr/prompts/run_task_finalize.md"
  update_agents: ".mr/prompts/update_agents.md"

dev:
  command_router: "cargo make"       # make is the default entrypoint for dev actions
  make_tasks_required:
    - "ci"
    - "fmt"
    - "clippy"
    - "test"
    - "uat"                          # alias for whatever this repo considers acceptance gate
    - "mr:test"                      # integration tests for MR itself
    - "mr:uat"                       # MR end-to-end (mock runner) acceptance tests

tags: [mvp, cli, prd, loop, copilot, cargo-make]

acceptance_tests:
  # UATs should be callable via cargo-make; most repos can map uat -> ci.
  - id: uat-001
    name: "Repo acceptance gate passes via cargo-make"
    command: "cargo make uat"
  - id: uat-002
    name: "`mr init` is idempotent (via cargo-make test target)"
    command: "cargo make mr:test init"
  - id: uat-003
    name: "`mr prd new` produces valid PRD (frontmatter + tasks + history scaffold)"
    command: "cargo make mr:test prd_new"
  - id: uat-004
    name: "`mr run --runner mock` attempts <=1 task and appends History (success + failure)"
    command: "cargo make mr:test run_mock"
  - id: uat-005
    name: "`mr status` output is stable"
    command: "cargo make mr:test status"
  - id: uat-006
    name: "`mr bootstrap --runner mock` generates PRD index + starter PRDs deterministically"
    command: "cargo make mr:test bootstrap_mock"

tasks:
  - id: T-001
    title: "Scaffold repo + CLI skeleton (`mr`), cargo-make, CI (kord-style)"
    priority: 1
    status: done
    notes: "Use kord as the reference: CI shape, README tone, cargo-make workflow. :contentReference[oaicite:2]{index=2}"
  - id: T-002
    title: "Define PRD file format + parser (YAML frontmatter + Markdown body)"
    priority: 2
    status: done
    notes: "Must round-trip without trashing human Markdown."
  - id: T-003
    title: "Implement PRD index generator (`.mr/PRDS.md`)"
    priority: 3
    status: done
    notes: "Index is derived from scanning `.mr/prds/*.md`."
  - id: T-004
    title: "Implement `mr init` (new repo setup)"
    priority: 4
    status: done
    notes: "Creates `.mr/` dirs, templates, prompts, starter AGENTS.md, PRD index."
  - id: T-005
    title: "Implement static prompt library + placeholder system"
    priority: 5
    status: done
    notes: "All stages use committed prompt files; placeholders expanded by MR."
  - id: T-006
    title: "Implement `mr prd new` as a guided Q/A (MR mediates runner+user in one session)"
    priority: 6
    status: done
    notes: "Round1: runner generates follow-ups; user answers; MR loops runner until 'enough'; final synthesis writes PRD."
  - id: T-007
    title: "Runner abstraction + MockRunner (deterministic tests)"
    priority: 7
    status: done
    notes: "Typed adapters; mock runner supports scripted Q/A and task runs."
  - id: T-008
    title: "CopilotRunner adapter (programmatic prompts + allow-all perms by default)"
    priority: 8
    status: done
    notes: "No API calls. Always prefer allow-all/yolo; fallback to allow-all-* flags."
  - id: T-009
    title: "Implement `mr run` (one-or-zero tasks per invocation)"
    priority: 9
    status: done
    notes: "Supports `--prd <id>` to target a specific PRD. Pick task; invoke runner with prompt instructing it to: implement task, run UAT, update PRD status/history, regenerate index, and commit. Runner handles the full loop."
  - id: T-010
    title: "Implement `mr status`"
    priority: 10
    status: done
    notes: "Summarize PRDs + tasks; show next task and last History summary."
  - id: T-011
    title: "AGENTS.md updater (safe, bounded patching driven by a prompt stage)"
    priority: 11
    status: done
    notes: "Auto-managed section; updated during prd_new and run."
  - id: T-012
    title: "Implement `mr bootstrap` (ingest an existing repo into PRDs)"
    priority: 12
    status: done
    notes: "Scan repo; generate PRD index; generate starter PRDs reflecting current repo reality."
  - id: T-013
    title: "Remove `allow` flags and make sure clippy lints are clean"
    priority: 13
    status: done
    notes: "No `allow(dead_code)` or `allow(unused_imports)` in committed code."
  - id: T-014
    title: "Implement `mr prd edit` for quick PRD modifications via runner"
    priority: 14
    status: done
    notes: "Invoke runner with PRD context + user request; runner suggests edits; MR applies changes. Lighter than `prd new`.  Should allow for a follow up question loop if needed."
  - id: T-016
    title: "Add `--language` flag to `init` (explicit) and `bootstrap` (auto-detected)"
    priority: 15
    status: done
    notes: "Allow user to specify language (rust, python, node, etc.). If rust or unspecified, use current defaults. Otherwise, invoke runner to 'rewrite the default prompts/templates for the target language' after scaffolding. Bootstrap should auto-detect language from repo (Cargo.toml → rust, package.json → node, pyproject.toml → python, etc.)."
  - id: T-017
    title: "Document placeholder variables for each prompt in README"
    priority: 16
    status: done
    notes: "Add tables in README showing available `{{placeholder}}` variables for each prompt type (run_task, prd_new_*, bootstrap_*, etc.). Helps users who want to customize prompts by hand. Include variable name, type (string/list), and description."
  - id: T-018
    title: "Add `.mr/config.toml` for persistent settings (model, runner, permissions, etc.)"
    priority: 17
    status: done
    notes: "Support a config file for common settings: default runner, default model (e.g., `model = \"claude-sonnet-4-20250514\"`), permission_mode, timeout, etc. CLI flags should override config. Also add `--model` flag to `run`, `prd new`, and `bootstrap` commands that passes through to the runner."
  - id: T-019
    title: "Stream/display runner output during `mr run`"
    priority: 18
    status: todo
    notes: "Currently runner output is captured silently and only a truncated summary is shown at the end. Add real-time streaming of copilot CLI output to stdout so users can watch progress. Consider a `--verbose` or `--stream` flag, or make streaming the default with `--quiet` to suppress. May require switching from `Command::output()` to `Command::spawn()` with piped stdout."
  - id: T-020
    title: "Add a `reindex` command to regenerate `.mr/PRDS.md` and edit PRD interlinks / code links."
    priority: 19
    status: todo
    notes: "This will allow users to force a new set of indexing to make sure everything is up to date. Also, during reindexing, MR can scan PRDs for inter-PRD links (e.g., 'see PRD-0002 for...') and code links (e.g., 'in src/module.rs line 42...') and verify/fix them.  These should all use _real_ markdown links.  This will likely require a new default prompt in init, and please make sure there is one in this repo, so we can dogfood it."
  - id: T-099
    title: "Wrap-up: docs + example PRDs + end-to-end smoke"
    priority: 99
    status: todo
    notes: "README + DEVELOPMENT + example PRDs; `cargo make uat` is the one true gate.  README should describe normal flow.  README should be a little bit funny...something 'small ralph to help you ralph your ralphs' style.  Also, make sure to reference that this whole thing was 'ralph'ed into existence by `microralph` itself.  It should be stylized as `microralph` everywhere."

---

# Summary

microralph (`mr`) is a tiny CLI that helps you **create PRDs** and **execute PRDs** by repeatedly invoking an underlying coding-agent CLI (starting with GitHub Copilot CLI) and updating PRD state (tasks + History) after every run.

**MVP promise (GSD-style):** minimal ceremony. You can:
- bootstrap or init a repo,
- write PRDs via a guided Q/A,
- run an iterative “try → verify → log” loop,
- and watch tasks flip to done when `cargo make uat` passes.

---

# Problem

Agent loops are useful, but many systems make the *workflow engine* the project. I want the smallest possible system where:

- PRDs are easy to write, and progress lives inside them
- Each run attempts one task and logs what happened
- The PRD itself becomes the memory
- The runner can fail without losing context
- Almost everything routes through `cargo make`
- No direct API calls: only shelling out to runner CLIs

---

# Goals

1. **PRD authoring is a guided Q/A**
   - `mr prd new` drives a back-and-forth with the runner and the user in one command session.
2. **Execution loop is one-step**
   - `mr run` attempts <=1 task, runs `cargo make uat`, updates task status, appends History.
3. **Static prompts for every stage**
   - `.mr/prompts/*.md` are committed, versioned, and used as the stable instruction layer.
4. **Kord as a reference**
   - microralph should copy patterns from `kord` for CI, README, and cargo-make ergonomics. :contentReference[oaicite:3]{index=3}
5. **No direct API calls**
   - Only runner CLIs. Typed adapters.

---

# Non-Goals (MVP)

- No TUI.
- No daemon/service.
- No direct API usage.
- No requirement of one commit per task.
- No forced branch workflow.

---

# Commands (MVP)

## `mr init`
New repo setup:
- `.mr/` structure: `prds/`, `templates/`, `prompts/`
- `.mr/PRDS.md` index
- starter `AGENTS.md`
- `Makefile.toml` tasks (kord-style) so most actions are `cargo make ...` :contentReference[oaicite:4]{index=4}

## `mr bootstrap`
Existing repo ingest:
- Generate `.mr/PRDS.md`
- Generate starter PRDs (bounded)
- Patch `AGENTS.md` auto-managed section with inferred norms (via prompt stage)

## `mr prd new <slug>`
Guided Q/A flow (one user session):
1. MR invokes runner using `prompts/prd_new_round1_questions.md`:
   - "Look at PRD list; here is what the user wants; ask follow-up questions."
2. MR prints questions to user and collects answers.
3. MR invokes runner using `prompts/prd_new_roundN_questions.md`:
   - include prior Q/A + repo context; ask any remaining questions or say “ready”.
4. When ready, MR invokes runner using `prompts/prd_new_synthesize_prd.md`:
   - create PRD draft with tasks + UATs + links + history scaffold.
5. MR may invoke `prompts/update_agents.md` to patch `AGENTS.md`.

> Alternative (future): let Copilot CLI run an interactive “interview” inside the agent itself.
> MVP prefers MR-mediated Q/A because it’s deterministic and testable.

## `mr run [--prd ...] [--runner ...]`
- Pick PRD + task
- Invoke runner with `prompts/run_task.md` (or `run_task_finalize.md` for wrap-up)
- Run `cargo make uat`
- Update task status if UAT passes
- Append History always (success/failure/partial)

## `mr status`
- Summarize PRDs + tasks + last History entry

---

# Static Prompts (Contract)

Prompts are files (not hard-coded strings), so:
- users can tune the “house style”
- behavior is stable across runs
- tests can validate exact prompt payloads

Each prompt must define:
- objective
- allowed actions (shell-only)
- required outputs (structured but *not* JSON state blobs)
- what to write into PRD History for success/failure

---

# Task Completion Semantics

- UATs are commands, usually `cargo make uat`.
- A task is “done” when all PRD UATs pass.
- Runner failure is acceptable; History captures what happened and what to try next.
- Each PRD includes a wrap-up task using `run_task_finalize.md`.

---

# History

## 2026-01-23
- Updated PRD: route most dev commands through `cargo make`.
- Updated PRD: added static prompt library for every stage.
- Updated PRD: `mr prd new` becomes MR-mediated runner↔user Q/A in one session.
- Updated PRD: use `kord` repo as reference for CI/README/workflow norms. :contentReference[oaicite:5]{index=5}

## 2026-01-23 — T-001 Completed
- **Task**: Scaffold repo + CLI skeleton (`mr`), cargo-make, CI (kord-style)
- **Status**: ✅ Done
- **Changes**:
  - Created CLI skeleton with clap (init, bootstrap, prd new/list, run, status commands)
  - Added anyhow for error handling, tracing for diagnostics
  - Created Makefile.toml with cargo-make tasks (fmt, clippy, test, ci, uat, build-linux/windows/macos)
  - Created GitHub Actions CI workflow (.github/workflows/build.yml) following kord patterns
  - Created README.md with badges, usage, and development instructions
  - Created AGENTS.md with workspace overview and conventions
  - Created .mr/ directory structure with prompts, templates, and PRDS.md index
  - Created rust-toolchain.toml pinning nightly-2025-12-22
  - Created LICENSE (MIT)
  - All tests pass (8/8), clippy clean, builds successfully

(Entries appended by `mr run` will go below this line.)

## 2026-01-24 — T-002 Completed
- **Task**: Define PRD file format + parser (YAML frontmatter + Markdown body)
- **Status**: ✅ Done
- **Changes**:
  - Added `serde` and `serde_yaml` dependencies for YAML parsing
  - Created `src/prd/` module with `mod.rs`, `types.rs`, and `parser.rs`
  - Defined comprehensive PRD data types: `Prd`, `PrdFrontmatter`, `Task`, `PrdStatus`, `TaskStatus`
  - Implemented `parse_prd()` and `parse_prd_file()` for parsing PRDs from strings/files
  - Implemented `serialize_prd()` for round-trip serialization preserving body content
  - Wrote 16 tests covering parsing, serialization, round-trips, and edge cases
  - Parser successfully handles the actual PRD-0001 file (complex real-world PRD)
  - All 24 tests pass (including 16 new prd module tests), clippy clean, CI green

## 2026-01-24 — T-003 Completed
- **Task**: Implement PRD index generator (`.mr/PRDS.md`)
- **Status**: ✅ Done
- **Changes**:
  - Added `chrono` dependency for date formatting
  - Created `src/prd/index.rs` module with PRD scanning and index generation
  - Implemented `scan_prds()` to scan `.mr/prds/` directory for PRD files
  - Implemented `generate_index()` to create PRDS.md content with tables grouped by status
  - Implemented `generate_index_file()` to write the index to disk
  - Added `PrdSummary` struct with progress tracking (completed/total tasks)
  - Added `Hash` derive to `PrdStatus` for HashMap usage
  - Wrote 7 tests covering index generation, PRD scanning, and edge cases
  - All 31 tests pass, clippy clean, CI green

## 2026-01-24 — T-004 Completed
- **Task**: Implement `mr init` (new repo setup)
- **Status**: ✅ Done
- **Changes**:
  - Created `src/init.rs` module with all initialization logic
  - Defined default content constants for all templates and prompts (PRD template, 9 prompt files)
  - Implemented `init()` function that creates `.mr/` directory structure:
    - `.mr/prds/` for PRD files
    - `.mr/templates/` with `prd.md` template
    - `.mr/prompts/` with all 9 prompt files
    - `.mr/PRDS.md` empty index
    - `AGENTS.md` starter file at repo root
  - Added `is_initialized()` helper to check if a repo has been initialized
  - Added `InitResult` struct with counts and paths of created/skipped items
  - Init is idempotent: re-running skips existing files without overwriting
  - Integrated with CLI: `mr init` command now fully functional
  - Wrote 8 tests covering structure creation, idempotency, and content validation
  - All 39 tests pass, clippy clean, CI green

## 2026-01-24 — T-005 Completed
- **Task**: Implement static prompt library + placeholder system
- **Status**: ✅ Done
- **Changes**:
  - Created `src/prompt/` module with `mod.rs`, `types.rs`, `loader.rs`, and `expand.rs`
  - Implemented `PromptKind` enum with all 9 prompt types and filename mapping
  - Implemented `PromptLoader` for loading prompts from `.mr/prompts/` with fallback to embedded defaults
  - Implemented `PlaceholderContext` and `PlaceholderValue` types for template expansion
  - Implemented `expand_placeholders()` supporting:
    - Simple `{{variable}}` substitution
    - Conditional `{{#if variable}}...{{/if}}` blocks
    - List iteration `{{#each variable}}...{{/each}}` with `{{@index}}` support
  - Added convenience functions: `load_prompt()`, `load_prompt_with_fallback()`
  - Wrote 36 tests covering prompt loading, placeholder expansion, conditionals, and loops
  - All 75 tests pass, clippy clean, CI green

## 2026-01-24 — T-006 & T-007 Completed
- **Task**: Implement `mr prd new` as a guided Q/A + Runner abstraction
- **Status**: ✅ Done
- **Changes**:
  - Created `src/runner/` module with `mod.rs`, `types.rs`, and `mock.rs`
  - Implemented `Runner` trait with `name()`, `execute()`, and `is_available()` methods
  - Implemented `RunnerOutput` and `RunnerError` types for runner responses
  - Implemented `MockRunner` with scripted responses, prompt recording, and deterministic behavior
  - Created `src/prd_new.rs` module for `mr prd new` guided Q/A flow:
    - `PrdNewConfig` for configurable max_rounds, max_questions_per_round, root_dir
    - `QaPair` struct for storing question/answer pairs
    - `PrdNewResult` with generated PRD path and Q/A history
    - Multi-round Q/A loop: runner asks questions → user answers → loop until "READY_TO_SYNTHESIZE"
    - Final synthesis: runner generates PRD from collected context
    - Automatic PRD ID generation (scans existing PRDs)
  - Integrated with CLI: `mr prd new <slug>` and `mr prd list` commands functional
  - Updated `scan_prds()` to return `Vec<(String, Prd, PathBuf)>` including file paths
  - Added `scan_prd_summaries()` and `generate_index_from_root()` helper functions
  - Wrote 17 new tests covering Q/A flow, question parsing, PRD generation, and edge cases
  - All 92 tests pass, clippy clean, CI green

## 2026-01-24 — T-008 Completed
- **Task**: CopilotRunner adapter (programmatic prompts + allow-all perms by default)
- **Status**: ✅ Done
- **Changes**:
  - Created `src/runner/copilot.rs` module with CopilotRunner implementation
  - Implemented `CopilotRunner` that shells out to the `copilot` CLI with `--allow-all` (yolo mode)
  - Added `CopilotConfig` struct with configurable options:
    - `copilot_path`: path to the copilot CLI binary
    - `permission_mode`: Yolo (--allow-all), AllowAll (individual flags), or Manual
    - `silent`: use `-s` for clean output
    - `no_ask_user`: disable ask_user tool for autonomous operation
  - Implemented `PermissionMode` enum for permission strategies
  - CopilotRunner executes `copilot -p "<prompt>" --allow-all -s --no-ask-user`
  - Added `is_available()` check using `which copilot`
  - Integrated CopilotRunner into main.rs `cmd_prd_new()` function
  - Added 7 tests covering config, arg building, and runner behavior
  - All 99 tests pass, clippy clean, CI green, UAT passes

## 2026-01-24 — T-009 Completed
- **Task**: Implement `mr run` (one-or-zero tasks per invocation)
- **Status**: ✅ Done
- **Changes**:
  - Created `src/run.rs` module with `mr run` implementation
  - Implemented `RunConfig` and `RunResult` types
  - Implemented `pick_prd()` function: selects first active PRD with incomplete tasks, or explicit `--prd <id>`
  - Implemented `build_prompt()` function: expands `run_task.md` prompt with PRD/task context
  - Implemented `run_task()` function: picks task, invokes runner, returns result
  - Updated `run_task.md` prompt to instruct the runner to:
    - Implement the task
    - Run `cargo make uat` to verify
    - Update task status in PRD frontmatter (todo → done)
    - Append history entry to PRD
    - Regenerate `.mr/PRDS.md` index
    - Commit changes with descriptive message
  - Updated `init.rs` with new embedded prompt content
  - Integrated with CLI: `mr run` and `mr run --prd PRD-0001` commands functional
  - Added `#[cfg(test)]` exports for test-only types (PrdFrontmatter, Task, serialize_prd)
  - Wrote 9 new tests covering PRD picking, task selection, and runner invocation
  - All 108 tests pass, clippy clean, CI green, UAT passes

## 2026-01-24 — T-010 Completed
- **Task**: Implement `mr status`
- **Status**: ✅ Done
- **Changes**:
  - Created `src/status.rs` module with status report logic
  - Implemented `StatusReport`, `NextTaskInfo`, and `StatusStats` types
  - Implemented `get_status()` to scan PRDs and compute status
  - Implemented `format_status()` for pretty-printing to terminal
  - Implemented `extract_last_history()` to parse the most recent History entry from PRD body
  - Implemented `find_next_task()` to identify the next incomplete task from active PRDs
  - Integrated with CLI: `mr status` command now fully functional
  - Shows next task with PRD context, last history entry, PRD list grouped by status, and statistics
  - Wrote 9 new tests covering status generation, history extraction, and formatting
  - All 117 tests pass, clippy clean, CI green, UAT passes

## 2026-01-24 — T-011 Completed
- **Task**: AGENTS.md updater (safe, bounded patching driven by a prompt stage)
- **Status**: ✅ Done
- **Changes**:
  - Created `src/agents.rs` module with 15 tests for AGENTS.md management
  - Implemented `RecentChange` struct to describe file changes for prompt context
  - Implemented `AgentsUpdateResult` struct with `modified` and `new_content` fields
  - Implemented `read_agents_file()` to read current AGENTS.md content
  - Implemented `extract_auto_managed_section()` to find content between markers
  - Implemented `patch_auto_managed_section()` for safe, bounded replacement
  - Implemented `build_update_agents_prompt()` to construct prompt with changes context
  - Implemented `parse_update_response()` to handle `NO_CHANGES`, code blocks, and plain text
  - Implemented `update_agents_md()` as the main entry point for updates
  - Integrated into `prd_new.rs`: updates AGENTS.md after creating a new PRD
  - Integrated into `run.rs`: updates AGENTS.md after successful task completion
  - Uses existing `update_agents.md` prompt template with placeholders
  - All 132 tests pass, clippy clean, CI green, UAT passes

## 2026-01-24 — T-012 Completed
- **Task**: Implement `mr bootstrap` (ingest an existing repo into PRDs)
- **Status**: ✅ Done
- **Changes**:
  - Created `src/bootstrap.rs` module with 13 tests for bootstrap functionality
  - Implemented `BootstrapConfig` struct with `root` and `prd_budget` fields
  - Implemented `BootstrapResult` struct tracking initialization, plan generation, and PRD creation
  - Implemented `bootstrap()` function as the main entry point:
    - Step 1: Ensures `.mr/` structure exists (runs `init` if needed)
    - Step 2: Invokes runner with `bootstrap_plan.md` to analyze the repo
    - Step 3: Invokes runner with `bootstrap_generate_prds.md` to generate PRDs
    - Step 4: Regenerates `.mr/PRDS.md` index
    - Step 5: Updates AGENTS.md auto-managed section
  - Implemented `build_plan_prompt()` and `build_generate_prompt()` for prompt expansion
  - Implemented `summarize_plan()` and `count_prds_in_output()` helper functions
  - Added `regex` dependency for PRD pattern matching
  - Integrated into `main.rs`: `mr bootstrap --runner <runner>` command fully functional
  - All 145 tests pass, clippy clean, UAT passes

## 2026-01-24 — T-013 Completed
- **Task**: Remove `allow` flags and make sure clippy lints are clean
- **Status**: ✅ Done
- **Changes**:
  - Removed all 24 `#[allow(dead_code)]` and `#[allow(unused_imports)]` attributes from codebase
  - Used `#[cfg(test)]` for test-only code instead of `#[allow(dead_code)]`:
    - `serialize_prd()`, `incomplete_tasks()`, `PrdSummary::path` field removal
    - `PlaceholderContext::len/is_empty/from_iter()` methods
    - `PromptLoader::prompts_dir/exists/all_exist/missing_prompts()` methods
    - `load_prompt()` function, `RunnerOutput::failure()` method
    - `MockRunner` test methods: `add_response/add_success/recorded_prompts/remaining_responses`
    - `CopilotConfig` builder methods and `with_config` constructor
    - `PermissionMode::AllowAll/Manual` variants
  - Removed unused code entirely:
    - `RunnerError::Timeout/ExecutionFailed/Other` variants (never constructed)
    - `RunnerOutput::exit_code` field (set but never read)
    - `CopilotConfig::timeout_secs` field (never used)
    - `PrdSummary::path` field (never read)
  - Used `Runner::name()` method in logging (`run.rs`, `prd_new.rs`) to make it non-dead
  - Used `PrdNewResult::qa_history` in output to show questions answered
  - All 145 tests pass, clippy clean, UAT passes

## 2026-01-24 — T-014 Completed
- **Task**: Implement `mr prd edit` for quick PRD modifications via runner
- **Status**: ✅ Done
- **Changes**:
  - Added `PrdEdit` variant to `PromptKind` enum in `src/prompt/types.rs`
  - Added `PROMPT_PRD_EDIT` constant to `src/init.rs` with the edit prompt template
  - Updated `init()` function to create `prd_edit.md` prompt file on initialization
  - Updated `get_default_prompt()` in `src/prompt/loader.rs` to handle new prompt kind
  - Created `src/prd_edit.rs` module with full edit functionality:
    - `PrdEditConfig` and `PrdEditResult` types
    - `edit_prd()` function with Q/A loop support (up to 3 rounds)
    - `find_prd()` to locate PRD by ID
    - `build_edit_prompt()` for prompt expansion with context
    - `parse_questions()` and `collect_answers()` for follow-up Q/A
    - `extract_prd_content()` to parse runner output
  - Added `Edit` subcommand to `PrdCommand` in `main.rs`
  - Implemented `cmd_prd_edit()` function for CLI integration
  - Created `.mr/prompts/prd_edit.md` prompt file
  - Updated test counts for new prompt file (12 → 13 files, 9 → 10 prompt kinds)
  - Wrote 9 tests covering edit flow, Q/A, and content extraction
  - All 154 tests pass, clippy clean, UAT passes

## 2026-01-24 — T-016 Completed
- **Task**: Add `--language` flag to `init` (explicit) and `bootstrap` (auto-detected)
- **Status**: ✅ Done
- **Changes**:
  - Added `Language` enum to `src/init.rs` with variants: Rust, Python, Node, Go, Java
  - Implemented `FromStr` for Language with aliases (e.g., "py" → Python, "ts" → Node)
  - Added `Language::build_commands()` returning typical commands per language
  - Implemented `detect_language()` function for auto-detection from project files:
    - Cargo.toml → Rust, package.json → Node, pyproject.toml/setup.py → Python
    - go.mod → Go, pom.xml/build.gradle → Java
  - Added `AdaptLanguage` variant to `PromptKind` enum in `src/prompt/types.rs`
  - Added `PROMPT_ADAPT_LANGUAGE` constant with language adaptation prompt
  - Updated `init` command with `--language` and `--runner` flags
  - Updated `bootstrap` command with `--language` flag (auto-detects if not specified)
  - Implemented `adapt_language()` function in `main.rs` to invoke runner for non-Rust languages
  - Created `.mr/prompts/adapt_language.md` prompt file
  - Added 14 new tests for Language enum and detection logic
  - Updated test counts (14 files, 11 prompt kinds, 167 total tests)
  - All 167 tests pass, clippy clean, UAT passes

## 2026-01-24 — T-017 Completed
- **Task**: Document placeholder variables for each prompt in README
- **Status**: ✅ Done
- **Changes**:
  - Added comprehensive "Prompt Placeholders" section to README.md
  - Documented placeholder syntax: `{{variable}}`, `{{#if}}`, `{{#each}}`
  - Created tables for all 11 prompt types showing:
    - Variable name
    - Type (string/list)
    - Description
  - Documented prompts: run_task, run_task_finalize, prd_new_round1_questions, prd_new_roundN_questions, prd_new_synthesize_prd, prd_edit, bootstrap_plan, bootstrap_generate_prds, update_agents, adapt_language, init
  - Included list iteration field notation (↳) for nested fields in `{{#each}}` blocks
  - All 167 tests pass, clippy clean, UAT passes

## 2026-01-24 — T-018 Completed
- **Task**: Add `.mr/config.toml` for persistent settings (model, runner, permissions, etc.)
- **Status**: ✅ Done
- **Changes**:
  - Added `toml` dependency to Cargo.toml for config file parsing
  - Created `src/config.rs` module with `Config` struct and loading logic
    - Supports `runner`, `model`, `permission_mode`, and `timeout_minutes` settings
    - `load()` and `load_or_default()` functions for loading config
    - `effective_model()` for CLI flag override logic
  - Updated `CopilotConfig` with `model` field and `with_model()` method
  - Updated `CopilotRunner::build_args()` to pass `--model <model>` to copilot CLI
  - Added `CopilotRunner::with_model()` constructor for easy model configuration
  - Added `--model` flag to all runner-using commands:
    - `mr init --model <model>` (for language adaptation)
    - `mr bootstrap --model <model>`
    - `mr prd new <slug> --model <model>`
    - `mr prd edit <id> <request> --model <model>`
    - `mr run --model <model>`
  - Updated `init()` to create default `config.toml` with commented-out options
  - Created `.mr/config.toml` in this repo
  - Added 18 new tests for config loading, CLI parsing, and model handling
  - All 186 tests pass, clippy clean, UAT passes

---
