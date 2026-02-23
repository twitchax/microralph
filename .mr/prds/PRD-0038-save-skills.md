---
id: PRD-0038
title: "Save Skills: Persistent Agent Learning Across Runs"
status: active
owner: twitchax
created: 2026-02-23
updated: 2026-02-23
principles:
  - "Manifest pattern: inject a lightweight index into prompts, not full skill content"
  - "Agent-managed lifecycle: no CLI commands for skill CRUD — agents create/update skills organically"
  - "Selective persistence: bias toward saving only genuinely reusable techniques"
  - "Survive restore: skills are learned artifacts, not defaults — mr restore must not delete them"
  - "Prompt Management (Constitution Rule 7): new prompt content defined in init.rs and materialized to .mr/prompts/"
references:
  - name: "RAG / Manifest pattern for agent memory"
    url: "https://en.wikipedia.org/wiki/Retrieval-augmented_generation"
  - name: "Run loop implementation"
    url: "src/commands/run.rs"
  - name: "Prompt constants and init"
    url: "src/commands/init.rs"
  - name: "Prompt expansion engine"
    url: "src/prompt/expand.rs"
acceptance_tests:
  - id: uat-001
    name: "mr init creates .mr/skills/ directory and empty SKILLS.md"
    command: cargo make test
    uat_status: verified
  - id: uat-002
    name: "mr restore creates .mr/skills/ if missing but does not overwrite existing skills"
    command: cargo make test
    uat_status: verified
  - id: uat-003
    name: "build_prompt injects skills_manifest placeholder into run_task prompt"
    command: cargo make test
    uat_status: verified
  - id: uat-004
    name: "run_task.md prompt includes skills manifest section and skill-saving instructions"
    command: cargo make test
    uat_status: verified
  - id: uat-005
    name: "Full CI passes (fmt, clippy, test)"
    command: cargo make ci
    uat_status: verified
tasks:
  - id: T-001
    title: "Create .mr/skills/ directory and SKILLS.md during mr init"
    priority: 1
    status: done
    notes: "Add skills_dir creation to init_prompts_and_templates() or a new init_skills() helper in src/commands/init.rs. Create empty SKILLS.md with a header comment explaining the manifest format. Define SKILLS_TEMPLATE constant."
  - id: T-002
    title: "Handle skills directory in mr restore"
    priority: 2
    status: done
    notes: "In the restore flow, create .mr/skills/ and SKILLS.md only if they do not already exist. Use create_dir_if_missing and create_file_if_missing (not create_file_always) to preserve learned skills."
  - id: T-003
    title: "Load skills manifest and inject into build_prompt()"
    priority: 3
    status: done
    notes: "In src/commands/run.rs build_prompt(), read .mr/skills/SKILLS.md content. Insert as skills_manifest placeholder into PlaceholderContext. If the file is empty or missing, the placeholder expands to empty string (guarded by {{#if skills_manifest}} in the prompt)."
  - id: T-004
    title: "Update run_task.md prompt with skills manifest and saving instructions"
    priority: 4
    status: done
    notes: "Add two sections to run_task.md: (1) A context section showing {{#if skills_manifest}}...{{/if}} with the manifest and a note to read .mr/skills/<name>/skill.md for details. (2) An end-of-task action instructing the agent to save reusable skills to .mr/skills/<slug>/skill.md and update SKILLS.md, with bias toward only saving genuinely useful skills."
  - id: T-005
    title: "Update PROMPT_RUN_TASK constant in init.rs to match run_task.md"
    priority: 5
    status: done
    notes: "Per Constitution Rule 7, the embedded constant in init.rs must stay synchronized with the .mr/prompts/run_task.md file. Update PROMPT_RUN_TASK to include the new skills sections."
  - id: T-006
    title: "Add PromptKind support and tests"
    priority: 6
    status: done
    notes: "No new PromptKind variant needed (skills manifest is data, not a prompt). Add unit tests for: init creating skills dir, restore preserving existing skills, build_prompt expanding skills_manifest placeholder. Ensure cargo make ci passes."
---

# Summary

Add a persistent skills system to microralph where agents learn reusable techniques during `mr run` task execution and save them to `.mr/skills/` for future runs. Skills are surfaced via a lightweight manifest (`SKILLS.md`) injected into the run_task prompt, following the RAG/manifest pattern — agents see skill summaries and can read full details on demand, but full content is not injected into every prompt.

# Problem

Currently, microralph's run loop has no cross-iteration memory beyond PRD History entries and AGENTS.md. When an agent discovers a useful technique (e.g., fixing a recurring lint pattern, a build workaround, a testing strategy), that knowledge is either lost after the session or buried in verbose History entries that aren't designed for reuse.

This means:
1. Agents repeatedly rediscover the same solutions across different PRDs and tasks
2. Useful scripts written during one task are not available for future tasks
3. There's no structured way to accumulate project-specific agent expertise over time

# Goals

1. Create a `.mr/skills/` directory structure for persistent, agent-managed skill storage
2. Surface available skills to the run agent via a lightweight manifest in the prompt
3. Instruct the agent to save genuinely useful, reusable skills at the end of each task iteration
4. Ensure skills survive `mr restore` (they are learned artifacts, not defaults)
5. Keep prompt bloat minimal by injecting only the manifest (titles + one-line summaries), not full skill content

# Technical Approach

## Directory Structure

```
.mr/skills/
├── SKILLS.md                        ← manifest/index (injected into prompts)
├── fix-clippy-pedantic/
│   ├── skill.md                     ← full description, when to use, examples
│   └── suppress_in_tests.sh         ← optional helper script
├── cargo-nextest-parallel/
│   └── skill.md
└── ...
```

## SKILLS.md Manifest Format

```markdown
# Skills

<!-- This file is auto-managed by the run agent. Each entry is a one-line summary. -->
<!-- Read .mr/skills/<name>/skill.md for full details on any skill. -->

- **fix-clippy-pedantic**: Techniques for resolving common clippy::pedantic lints without suppressing them.
- **cargo-nextest-parallel**: Configure nextest for optimal parallel test execution.
```

## Prompt Integration (Manifest Pattern)

```
┌─────────────────────────────────────┐
│         run_task.md prompt          │
│                                     │
│  {{#if skills_manifest}}            │
│  ## Available Skills                │
│  {{skills_manifest}}                │
│  (Read .mr/skills/<name>/skill.md   │
│   for full details when relevant)   │
│  {{/if}}                            │
│                                     │
│  ...existing task instructions...   │
│                                     │
│  ## After Task Completion           │
│  - Save reusable skills to          │
│    .mr/skills/<slug>/skill.md       │
│  - Update .mr/skills/SKILLS.md      │
│  - Only save genuinely useful       │
│    techniques                       │
└─────────────────────────────────────┘
```

## Data Flow

```
build_prompt()
  ├─ Load .mr/skills/SKILLS.md content
  ├─ Insert as {{skills_manifest}} placeholder
  └─ Expand into run_task.md template

Agent executes task
  ├─ Reads manifest, optionally reads full skill.md files
  ├─ Completes task work
  └─ Evaluates if a reusable skill was learned
       ├─ If yes: creates .mr/skills/<slug>/skill.md + updates SKILLS.md
       └─ If no: skips skill saving (bias toward selectivity)
```

## Integration Points

1. **`src/commands/init.rs`**: Add `SKILLS_TEMPLATE` constant, create `.mr/skills/` dir and `SKILLS.md` in `init_prompts_and_templates()`. Update restore to use `create_dir_if_missing` / `create_file_if_missing` for skills.
2. **`src/commands/run.rs`**: In `build_prompt()`, load `.mr/skills/SKILLS.md` and insert as `skills_manifest` placeholder.
3. **`.mr/prompts/run_task.md`** and **`PROMPT_RUN_TASK`** constant: Add skills context section and end-of-task saving instructions.

# Assumptions

- Agents (Copilot, Claude, Codex) can create directories and files via their standard file editing tools during `mr run`
- The SKILLS.md manifest will remain small enough to inject into prompts without significant context bloat (dozens of skills, not thousands)
- Agents can be effectively instructed to exercise judgment about which skills are worth saving

# Constraints

- **Constitution Rule 7**: Prompt changes must be made in both `init.rs` (constant) and `.mr/prompts/` (file), kept in sync
- **Constitution Rule 3**: Minimal changes — only modify `run_task.md` prompt, `build_prompt()`, and init; do not touch other prompts
- **Context window**: Only the manifest is injected, not full skill content, to avoid prompt bloat
- No new CLI subcommands — skills are purely agent-managed

# References to Code

- `src/commands/run.rs` — `build_prompt()` function (lines ~314-357): where `skills_manifest` placeholder will be added
- `src/commands/init.rs` — `init_prompts_and_templates()` (lines ~2103-2124): where `.mr/skills/` creation will be added
- `src/commands/init.rs` — `PROMPT_FILES` array (lines ~1985-2003): prompt filename-to-constant mappings
- `src/commands/init.rs` — `PROMPT_RUN_TASK` constant: embedded run_task prompt to update
- `src/prompt/expand.rs` — placeholder expansion engine (supports `{{#if}}` conditionals)
- `.mr/prompts/run_task.md` — the prompt file that will get skills sections

# Non-Goals (MVP)

- **CLI skill management**: No `mr skills list`, `mr skills prune`, or `mr skills search` commands
- **Other prompts**: Skills are only surfaced in `run_task.md` — refactor, suggest, and other prompts are out of scope
- **Semantic search / RAG retrieval**: No embedding-based skill matching; agents use the flat manifest
- **Skill versioning or conflict resolution**: Agents overwrite skill files freely
- **Skill sharing across projects**: Skills are project-local in `.mr/skills/`

# History

## 2026-02-23 — T-001 Completed
- **Task**: Create .mr/skills/ directory and SKILLS.md during mr init
- **Status**: ✅ Done
- **Changes**:
  - Added `SKILLS_TEMPLATE` constant in `src/commands/init.rs` with header comment explaining the manifest format
  - Added `create_dir_if_missing` for `.mr/skills/` and `create_file_if_missing` for `SKILLS.md` in `init()` function
  - Updated test assertions: `dirs_created` 3→4, `files_created` 22→23, `files_skipped` 22→23
  - Added test assertion for `.mr/skills/` and `.mr/skills/SKILLS.md` existence
  - UAT: `cargo make uat` passes — 575 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-23 — T-002 Completed
- **Task**: Handle skills directory in mr restore
- **Status**: ✅ Done
- **Changes**:
  - Extracted skills creation from `init()` into new `pub fn init_skills()` helper in `src/commands/init.rs` (DRY: reused by both `init()` and `restore_impl()`)
  - Updated `init()` to delegate to `init_skills()` instead of inline code
  - Added `init::init_skills(root)` call in `restore_impl()` in `src/main.rs` with user-facing messages for created/preserved skills
  - Added `test_restore_preserves_existing_skills` test: verifies custom SKILLS.md and skill files survive restore
  - Added `test_restore_creates_skills_if_missing` test: verifies skills dir/manifest are recreated if deleted
  - Added skills assertions to `test_restore_fresh` test
  - UAT: `cargo make uat` passes — 577 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-23 — T-003 Completed
- **Task**: Load skills manifest and inject into build_prompt()
- **Status**: ✅ Done
- **Changes**:
  - Added skills manifest loading to `build_prompt()` in `src/commands/run.rs` (lines 356-364)
  - Reads `.mr/skills/SKILLS.md`, compares against default `SKILLS_TEMPLATE`; only injects `skills_manifest` placeholder when file contains actual skill entries beyond the default boilerplate
  - If file is missing, empty, or matches the default template, the placeholder is not inserted — `{{#if skills_manifest}}` in the prompt evaluates to false
  - UAT: `cargo make uat` passes — 577 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-23 — T-004 Completed
- **Task**: Update run_task.md prompt with skills manifest and saving instructions
- **Status**: ✅ Done
- **Changes**:
  - Added `{{#if skills_manifest}}` context section to `.mr/prompts/run_task.md` after the Constitution section, showing the skills manifest with a note to read full skill files on demand
  - Added "Saving Skills (End-of-Task)" section before "When All Tasks Are Done" with instructions for creating skill directories, writing skill.md files, updating SKILLS.md manifest, and bias toward selectivity
  - UAT: `cargo make uat` passes — 577 tests, 0 failures
- **Constitution Compliance**: Temporary violation of Rule 7 (Prompt Management) — `.mr/prompts/run_task.md` is now out of sync with `PROMPT_RUN_TASK` constant in `init.rs`. This is expected: T-005 is the dedicated task to synchronize the constant. The violation is transient and will be resolved in the next task.

## 2026-02-23 — T-005 Completed
- **Task**: Update PROMPT_RUN_TASK constant in init.rs to match run_task.md
- **Status**: ✅ Done
- **Changes**:
  - Added `{{#if skills_manifest}}` Available Skills section to `PROMPT_RUN_TASK` constant in `src/commands/init.rs`, after the Constitution `{{/if}}` block and before `## Required Actions`
  - Added "Saving Skills (End-of-Task)" section to `PROMPT_RUN_TASK` constant, after `## Constraints` and before `## When All Tasks Are Done`
  - Both additions match the corresponding sections in `.mr/prompts/run_task.md` exactly, resolving the Rule 7 sync issue from T-004
  - UAT: `cargo make uat` passes — 577 tests, 0 failures
- **Constitution Compliance**: No violations. This task resolves the transient Rule 7 violation introduced in T-004.

## 2026-02-23 — T-006 Completed
- **Task**: Add PromptKind support and tests
- **Status**: ✅ Done
- **Changes**:
  - Added 3 unit tests to `src/commands/init.rs`: `test_init_skills_creates_dir_and_manifest`, `test_init_skills_preserves_existing`, `test_init_skills_idempotent` — directly testing the `init_skills()` function
  - Added 3 unit tests to `src/commands/run.rs`: `test_build_prompt_skills_manifest_injected`, `test_build_prompt_skills_manifest_omitted_when_default`, `test_build_prompt_skills_manifest_omitted_when_missing` — testing all three `build_prompt` code paths for skills_manifest expansion
  - No new `PromptKind` variant needed (confirmed: skills manifest is data, not a prompt)
  - UAT: `cargo make uat` passes — 583 tests, 0 failures
- **Constitution Compliance**: No violations.

## 2026-02-23 — uat-001 Verification
- **UAT**: mr init creates .mr/skills/ directory and empty SKILLS.md
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - `test_init_creates_structure` (src/commands/init.rs:2313) — verifies `init()` creates `.mr/skills/` and `.mr/skills/SKILLS.md`
  - `test_init_skills_creates_dir_and_manifest` (src/commands/init.rs:2703) — verifies `init_skills()` creates dir, manifest file, and correct content matching `SKILLS_TEMPLATE`
  - All 583 tests pass

## 2026-02-23 — uat-002 Verification
- **UAT**: mr restore creates .mr/skills/ if missing but does not overwrite existing skills
- **Status**: ✅ Verified
- **Method**: Existing tests
- **Details**:
  - `test_restore_preserves_existing_skills` (src/main.rs:2761) — verifies custom SKILLS.md content and skill subdirectory files survive `restore_impl()`
  - `test_restore_creates_skills_if_missing` (src/main.rs:2798) — verifies skills dir and SKILLS.md are recreated by `restore_impl()` when missing
  - Both tests pass

## 2026-02-23 — uat-003 Verification
- **UAT**: build_prompt injects skills_manifest placeholder into run_task prompt
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - `test_build_prompt_skills_manifest_injected` (src/commands/run.rs:2758) — creates a non-default SKILLS.md with skill entries, calls `build_prompt()`, and asserts the expanded prompt contains "Available Skills" and the skill entry content
  - Test passes

## 2026-02-23 — uat-004 Verification
- **UAT**: run_task.md prompt includes skills manifest section and skill-saving instructions
- **Status**: ✅ Verified
- **Method**: New test
- **Details**:
  - Created `test_run_task_prompt_includes_skills_sections` (src/commands/init.rs) — verifies `PROMPT_RUN_TASK` contains `{{#if skills_manifest}}`, `## Available Skills`, `{{skills_manifest}}`, `## Saving Skills (End-of-Task)`, `.mr/skills/`, and `SKILLS.md`
  - Both `PROMPT_RUN_TASK` constant and `.mr/prompts/run_task.md` contain the required sections
  - All 584 tests pass

## 2026-02-23 — uat-005 Verification
- **UAT**: Full CI passes (fmt, clippy, test)
- **Status**: ✅ Verified
- **Method**: Direct execution
- **Details**:
  - Ran `cargo make ci` which executes fmt, clippy, and test stages
  - All 584 tests passed, 0 failures
  - No fmt or clippy issues detected
