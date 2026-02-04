---
id: PRD-0030
title: "Source Module Organization Refactor"
status: active
owner: "twitchax"
created: 2026-02-04
updated: 2026-02-04

principles:
- Feature/domain-based grouping over layer-based organization
- Maximum nesting depth of 3-4 levels
- Consolidate related functionality into cohesive modules
- Move utility functions into dedicated utility modules
- CI must pass after reorganization

references:
- name: Rust Module System
  url: https://doc.rust-lang.org/book/ch07-02-defining-modules-to-control-scope-and-privacy.html

acceptance_tests:
- id: uat-001
  name: Full CI pipeline passes after reorganization
  command: cargo make ci
  uat_status: unverified
- id: uat-002
  name: All existing tests pass
  command: cargo make test
  uat_status: unverified

tasks:
- id: T-001
  title: Create commands/ module for CLI command implementations
  priority: 1
  status: done
  notes: "Move bootstrap.rs, devcontainer.rs, graph.rs, init.rs, refactor.rs, reindex.rs, restore, run.rs, status.rs, suggest.rs, validate.rs into commands/"
- id: T-002
  title: Consolidate prd_*.rs files into prd/ module
  priority: 1
  status: done
  notes: "Move prd_edit.rs, prd_new.rs, prd_finalize.rs into prd/ module as edit.rs, new.rs, finalize.rs"
- id: T-003
  title: Create util/ module for utility functions
  priority: 2
  status: done
  notes: "Move colors.rs, spinner.rs, qa_workflow.rs into util/ module"
- id: T-004
  title: Move config.rs and constitution_edit.rs into config/ module
  priority: 2
  status: done
  notes: "Create config/ module with mod.rs (config loading), constitution.rs (constitution editing)"
- id: T-005
  title: Update main.rs module declarations and imports
  priority: 3
  status: todo
  notes: "Update mod declarations and use statements to reflect new structure"
- id: T-006
  title: Update internal cross-module imports throughout codebase
  priority: 3
  status: todo
  notes: "Fix all crate:: imports to use new module paths"
- id: T-007
  title: Verify CI passes with new structure
  priority: 4
  status: todo
  notes: "Run cargo make ci to ensure fmt, clippy, and tests pass"

---

# Summary

Reorganize the `src/` directory from a flat structure with 20+ files at the root level into a well-organized hierarchy of feature/domain-based submodules. This improves code discoverability, logical grouping, and long-term maintainability.

---

# Problem

The `src/` directory currently contains 20+ files at the root level, making it difficult to understand the codebase structure at a glance. Files that logically belong together (e.g., `prd_edit.rs`, `prd_new.rs`, `prd_finalize.rs`) are scattered across the root instead of being grouped with the existing `prd/` module. Utility functions like colors and spinners sit alongside core command implementations.

Current root-level file count: 19 `.rs` files plus 3 subdirectories (`prd/`, `runner/`, `prompt/`).

---

# Goals

1. Group related command implementations into a `commands/` module
2. Consolidate all PRD-related files into the existing `prd/` module
3. Create a `util/` module for shared utilities (colors, spinner, qa_workflow)
4. Create a `config/` module for configuration and constitution handling
5. Reduce root-level file count to ~5-7 files (main.rs + core module declarations)
6. Maintain all existing functionality—CI must pass

---

# Technical Approach

## Target Directory Structure

```
src/
├── main.rs                    # CLI entry point, argument parsing
├── commands/                  # All CLI command implementations
│   ├── mod.rs
│   ├── bootstrap.rs
│   ├── devcontainer.rs
│   ├── graph.rs
│   ├── init.rs
│   ├── refactor.rs
│   ├── reindex.rs
│   ├── restore.rs             # (extract from init.rs if needed)
│   ├── run.rs
│   ├── status.rs
│   ├── suggest.rs
│   └── validate.rs
├── prd/                       # PRD types, parsing, and operations
│   ├── mod.rs
│   ├── types.rs
│   ├── parser.rs
│   ├── index.rs
│   ├── edit.rs                # (from prd_edit.rs)
│   ├── new.rs                 # (from prd_new.rs)
│   └── finalize.rs            # (from prd_finalize.rs)
├── config/                    # Configuration management
│   ├── mod.rs                 # (from config.rs)
│   └── constitution.rs        # (from constitution_edit.rs)
├── prompt/                    # Prompt loading and expansion (unchanged)
│   ├── mod.rs
│   ├── expand.rs
│   ├── loader.rs
│   └── types.rs
├── runner/                    # Runner implementations (unchanged)
│   ├── mod.rs
│   ├── types.rs
│   ├── cli_runner.rs
│   ├── copilot.rs
│   ├── claude.rs
│   └── mock.rs
├── util/                      # Shared utilities
│   ├── mod.rs
│   ├── colors.rs
│   ├── spinner.rs
│   └── qa_workflow.rs
└── changelog.rs               # Stays at root (small, self-contained)
```

## Migration Strategy

1. **Create new directories** with `mod.rs` files
2. **Move files** one module at a time, updating imports
3. **Update re-exports** in each module's `mod.rs` to maintain API surface
4. **Fix imports** in `main.rs` and across modules
5. **Run CI** after each major move to catch issues early

## Import Pattern

After reorganization, imports in `main.rs` will look like:

```rust
mod changelog;
mod commands;
mod config;
mod prd;
mod prompt;
mod runner;
mod util;

use commands::{bootstrap, devcontainer, graph, init, ...};
use prd::{Prd, PrdStatus, ...};
use runner::Runner;
use util::{colors, spinner};
```

---

# Assumptions

- The existing `prd/`, `runner/`, and `prompt/` module structures are sound and should be preserved
- Internal APIs between modules are flexible (no external consumers)
- Test files remain co-located with their modules using `#[cfg(test)]` patterns

---

# Constraints

- **Minimal Changes**: Each file move should require only import path updates, not logic changes
- **CI Gating**: Every intermediate state must pass `cargo make ci`
- **No Public API Concerns**: The crate is binary-only, so module reorganization has no external impact
- **Clippy Pedantic**: All moved files must continue to pass pedantic clippy lints

---

# References to Code

- `src/main.rs` — CLI entry point, contains all `mod` declarations (lines 1-50)
- `src/prd/mod.rs` — Existing PRD module with re-exports pattern
- `src/runner/mod.rs` — Existing runner module with trait and implementations
- `src/prompt/mod.rs` — Existing prompt module structure
- Files to consolidate into `prd/`: `prd_edit.rs` (504 lines), `prd_new.rs` (1584 lines), `prd_finalize.rs` (1059 lines)
- Files to move to `util/`: `colors.rs` (161 lines), `spinner.rs` (464 lines), `qa_workflow.rs` (433 lines)
- Files to move to `config/`: `config.rs` (352 lines), `constitution_edit.rs` (307 lines)

---

# Non-Goals (MVP)

- Splitting large files into smaller ones (beyond moving existing files)
- Changing any public function signatures or behavior
- Refactoring internal logic within files
- Adding new abstraction layers
- Changing test structure beyond import path updates

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-02-04 — T-001 Completed
- **Task**: Create commands/ module for CLI command implementations
- **Status**: ✅ Done
- **Changes**:
  - Created `src/commands/` directory with `mod.rs`
  - Moved 10 command implementation files to `src/commands/`: bootstrap.rs, devcontainer.rs, graph.rs, init.rs, refactor.rs, reindex.rs, run.rs, status.rs, suggest.rs, validate.rs
  - Updated `src/main.rs` to declare `commands` module and import submodules via `use commands::{...}`
  - `restore` functionality remains in main.rs (it was never a separate file, contrary to PRD notes mentioning "restore.rs")
  - UAT: `cargo make uat` passed with 484 tests

---

## 2026-02-04 — T-002 Completed
- **Task**: Consolidate prd_*.rs files into prd/ module
- **Status**: ✅ Done
- **Changes**:
  - Moved `src/prd_edit.rs` to `src/prd/edit.rs`
  - Moved `src/prd_new.rs` to `src/prd/new.rs`
  - Moved `src/prd_finalize.rs` to `src/prd/finalize.rs`
  - Updated `src/prd/mod.rs` to declare new submodules (`edit`, `new`, `finalize`) as public modules
  - Updated imports in moved files to use `super::` instead of `crate::prd::` for intra-module references
  - Updated `src/main.rs` to remove old `mod prd_edit`, `mod prd_new`, `mod prd_finalize` declarations
  - Updated `src/main.rs` usages to reference `prd::edit::`, `prd::new::`, `prd::finalize::`
  - Fixed `src/commands/suggest.rs` import to use `crate::prd::new::` path
  - UAT: `cargo make uat` passed with 484 tests

---

## 2026-02-04 — T-003 Completed
- **Task**: Create util/ module for utility functions
- **Status**: ✅ Done
- **Changes**:
  - Created `src/util/` directory with `mod.rs` declaring public submodules: `colors`, `spinner`, `qa_workflow`
  - Moved `src/colors.rs` to `src/util/colors.rs`
  - Moved `src/spinner.rs` to `src/util/spinner.rs`
  - Moved `src/qa_workflow.rs` to `src/util/qa_workflow.rs`
  - Updated `src/main.rs` to replace individual module declarations with `mod util;` and added `use util::colors;`
  - Updated `src/util/qa_workflow.rs` to use `super::colors::` for intra-module references
  - Updated imports across the codebase to use `crate::util::` paths:
    - `src/constitution_edit.rs`: `crate::util::qa_workflow`
    - `src/prd/edit.rs`: `crate::util::qa_workflow`
    - `src/prd/new.rs`: `crate::util::qa_workflow`, `crate::util::spinner`
    - `src/prd/finalize.rs`: `crate::util::spinner`
    - `src/commands/run.rs`: `crate::util::spinner`
    - `src/commands/refactor.rs`: `crate::util::spinner`
    - `src/commands/reindex.rs`: `crate::util::spinner`
    - `src/commands/suggest.rs`: `crate::util::colors`, `crate::util::spinner`
    - `src/commands/bootstrap.rs`: `crate::util::spinner`
  - UAT: `cargo make uat` passed with 484 tests

---

## 2026-02-04 — T-004 Completed
- **Task**: Move config.rs and constitution_edit.rs into config/ module
- **Status**: ✅ Done
- **Changes**:
  - Created `src/config/` directory
  - Moved `src/config.rs` to `src/config/mod.rs` (contains `Config` struct, `load_constitution`, constants)
  - Moved `src/constitution_edit.rs` to `src/config/constitution.rs` (contains `ConstitutionEditConfig`, `edit_constitution`)
  - Updated `src/config/mod.rs` to declare `pub mod constitution;` submodule
  - Updated `src/main.rs` to remove `mod constitution_edit;` declaration
  - Updated `src/main.rs` usages to use `config::constitution::ConstitutionEditConfig` and `config::constitution::edit_constitution`
  - All existing imports of `crate::config::` (in prd/new.rs, commands/run.rs, commands/init.rs, commands/refactor.rs, prd/finalize.rs) continue to work as `Config` and `load_constitution` remain exported from config/mod.rs
  - UAT: `cargo make uat` passed with 484 tests

---