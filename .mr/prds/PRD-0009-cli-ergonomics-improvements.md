---
id: PRD-0009
title: "CLI Ergonomics Improvements"
status: active
owner: "microralph"
created: 2026-01-24
updated: 2026-01-24

principles:
- Make common workflows more ergonomic by reducing keystrokes
- Flatten command hierarchy by moving PRD subcommands to top level
- Show tail of LLM output instead of beginning for better debugging
- Maintain consistency between task loop and UAT verification loop

references:
- name: clap command structure
  url: https://docs.rs/clap/latest/clap/

acceptance_tests:
- id: uat-001
  name: Run command accepts optional positional PRD argument
  command: cargo run -- run PRD-0001
  uat_status: unverified
- id: uat-002
  name: Run command works without arguments (interactive mode)
  command: cargo run -- run
  uat_status: unverified
- id: uat-003
  name: Top-level list command works
  command: cargo run -- list
  uat_status: unverified
- id: uat-004
  name: Top-level new command works
  command: cargo run -- new test-slug
  uat_status: unverified
- id: uat-005
  name: Top-level edit command works
  command: cargo run -- edit PRD-0001 "test edit"
  uat_status: unverified
- id: uat-006
  name: Top-level finalize command works
  command: cargo run -- finalize PRD-0001
  uat_status: unverified
- id: uat-007
  name: LLM output shows tail instead of beginning
  command: cargo make uat
  uat_status: unverified
- id: uat-008
  name: UAT verification loop shows tail of output
  command: cargo make uat
  uat_status: unverified

tasks:
- id: T-001
  title: Remove --prd flag and add optional positional PRD argument to run command
  priority: 1
  status: done
  notes: Change Run command to accept optional positional prd argument. Remove --prd long flag.
- id: T-002
  title: Remove Prd subcommand enum and flatten subcommands to top level
  priority: 2
  status: done
  notes: Move List, New, Edit, Finalize from PrdCommand to top-level Command enum. Remove Prd variant.
- id: T-003
  title: Update all code references from prd subcommands to top-level commands
  priority: 3
  status: todo
  notes: Update main.rs match statements and any helper code that references PrdCommand variants.
- id: T-004
  title: Add tracing info for commands executed by runner
  priority: 4
  status: todo
  notes: Add tracing::info! calls when invoking model to log command parameters for debugging.
- id: T-005
  title: Change LLM output display to show tail instead of beginning
  priority: 5
  status: todo
  notes: Update all places that truncate/display model output to show last N chars/lines instead of first N.
- id: T-006
  title: Apply tail output behavior to UAT verification loop
  priority: 6
  status: todo
  notes: Ensure UAT verification loop uses same tail output truncation as task loop.
- id: T-007
  title: Update documentation for new CLI structure
  priority: 7
  status: todo
  notes: Update README.md, AGENTS.md, and any other docs that reference old command structure.
- id: T-008
  title: Run full test suite to verify changes
  priority: 8
  status: todo
  notes: cargo make ci and cargo make uat to ensure no regressions.

---

# Summary

Improve CLI ergonomics by simplifying command structure and improving output readability. The main changes are: (1) making the PRD argument optional and positional on `run`, (2) flattening the `prd` subcommand hierarchy by moving `list`, `new`, `edit`, and `finalize` to the top level, and (3) showing the tail of LLM output instead of the beginning for better debugging and context.

---

# Problem

The current CLI requires excessive typing for common operations (`mr prd list`, `mr run --prd PRD-0001`) and shows truncated LLM output from the beginning, which often lacks the most relevant information (errors, completion status). The `prd` subcommand creates an unnecessary layer of hierarchy for commands that are frequently used.

---

# Goals

1. Reduce keystrokes for common workflows by accepting optional positional PRD argument on `run`
2. Flatten command hierarchy by moving `list`, `new`, `edit`, `finalize` to top level
3. Improve debugging by showing tail of LLM output instead of beginning
4. Maintain consistency between task loop and UAT verification output behavior
5. Add command tracing for model invocations to aid debugging

---

# Non-Goals (MVP)

- Changing the behavior of `init`, `bootstrap`, `status`, or `reindex` commands
- Adding new functionality beyond ergonomic improvements
- Changing the PRD file format or storage structure
- Modifying the streaming behavior (--stream flag)

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-24 — T-001 Completed
- **Task**: Remove --prd flag and add optional positional PRD argument to run command
- **Status**: ✅ Done
- **Changes**:
  - Modified `src/main.rs`: Changed the `Run` command's `prd` field from `#[arg(long)]` to a positional optional argument
  - Removed the `--prd` long flag requirement
  - Updated documentation comment to clarify the positional argument usage
  - UAT passed: All 262 tests passed successfully
  - The CLI now accepts `mr run PRD-0001` instead of `mr run --prd PRD-0001`
  - `mr run` without arguments still works (interactive mode)

## 2026-01-24 — T-002 Completed
- **Task**: Remove Prd subcommand enum and flatten subcommands to top level
- **Status**: ✅ Done
- **Changes**:
  - Modified `src/main.rs`: Removed `PrdCommand` enum entirely
  - Moved `New`, `Edit`, `List`, and `Finalize` from `PrdCommand` to top-level `Command` enum
  - Removed the `Prd` variant from `Command` enum that wrapped `PrdCommand`
  - Updated main() match statements to handle flattened commands directly
  - Updated test functions to use new command structure:
    - Changed `mr prd new` to `mr new` in tests
    - Changed `mr prd finalize` to `mr finalize` in tests
  - UAT passed: All 262 tests passed successfully
  - The CLI now uses `mr new <slug>`, `mr edit <id>`, `mr list`, `mr finalize <id>` instead of `mr prd new`, etc.

---