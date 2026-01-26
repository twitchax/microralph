---
id: PRD-0020
title: "Add No-Commit Flag"
status: draft
owner: "twitchax"
created: 2026-01-26
updated: 2026-01-26

principles:
- CLI flag supersedes config.toml option
- Default behavior unchanged (commit by default)
- Prompt instructions are inverted, not removed, when flag is active
- No behavioral enforcement; only affects prompt text

references:
- name: Handlebars Templating
  url: https://handlebarsjs.com/guide/

acceptance_tests:
- id: uat-001
  name: CLI flag --no-commit is accepted by mr run
  command: cargo run -- run --help | grep -q "no-commit"
  uat_status: unverified
- id: uat-002
  name: Config option no_commit is parsed from config.toml
  command: cargo make test
  uat_status: unverified
- id: uat-003
  name: Prompts include "Do NOT commit" when flag is active
  command: cargo make uat
  uat_status: unverified
- id: uat-004
  name: Default behavior still instructs to commit
  command: cargo make uat
  uat_status: unverified

tasks:
- id: T-001
  title: Add no_commit option to config.rs and CLI args
  priority: 1
  status: todo
  notes: "Add `no_commit: Option<bool>` to Config struct with parsing and an `effective_no_commit()` method. Add `--no-commit` flag to Run command in main.rs."
- id: T-002
  title: Add commit conditional to prompt templates
  priority: 2
  status: todo
  notes: Update run_task.md and run_task_finalize.md with `{{#if commit}}` blocks. When commit=true, show existing commit instructions. When commit=false, show "Do NOT commit" instructions.
- id: T-003
  title: Thread no_commit flag through run module
  priority: 3
  status: todo
  notes: Add no_commit field to RunConfig, pass through to prompt expansion as `commit` variable (inverted logic).
- id: T-004
  title: Update init.rs embedded prompts
  priority: 4
  status: todo
  notes: Per constitution rule 7, update the embedded prompt constants in init.rs to match the new conditional template syntax.
- id: T-005
  title: Add tests for no_commit functionality
  priority: 5
  status: todo
  notes: Unit tests for config parsing, effective_no_commit precedence, and prompt expansion with commit variable.

---

# Summary

Add a `--no-commit` CLI flag and corresponding `no_commit` config option that instructs agents to NOT commit changes, allowing users to review edits before manual commit. Default behavior remains unchanged (commit by default).

---

# Problem

Currently, `mr run` and `mr finalize` prompts instruct the agent to commit changes automatically. Users who want to review changes before committing have no way to prevent this instruction. This makes it difficult to audit agent work before it becomes part of git history.

---

# Goals

1. Add `--no-commit` flag to `mr run` command that prevents commit instructions in prompts.
2. Add `no_commit` option to `.mr/config.toml` for persistent configuration.
3. CLI flag supersedes config option (explicit flag wins).
4. When active, prompts say "Do NOT commit" instead of commit instructions.
5. Default behavior unchanged: commit instructions present when flag is not set.

---

# Non-Goals (MVP)

- No rollback or undo functionality
- No automatic staging summary (per user Q/A)
- No enforcement mechanism—flag only affects prompt text, not agent behavior
- Does not affect `mr finalize` initially (though same pattern applies)

---

# History

(Entries appended by `mr run` will go below this line.)

---