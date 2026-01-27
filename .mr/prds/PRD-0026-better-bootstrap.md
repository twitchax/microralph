---
id: PRD-0026
title: "Better Bootstrap: Reconstruct, depends_on, and Graph Command"
status: active
owner: Aaron Roney
created: 2026-01-27
updated: 2026-01-27

principles:
- "depends_on represents directed edges: 'this PRD should be done after the dependencies'"
- "Graph output formats are separate subcommands for SOC (ascii, mermaid, dot)"
- "Reconstruct infers major milestones from git history, not incremental bug fixes"
- "LLM-driven dependency inference and fixing during reindex"
- "Minimal schema changes: add depends_on and reconstructed fields to frontmatter"

references:
- name: Mermaid Flowchart Syntax
  url: https://mermaid.js.org/syntax/flowchart.html
- name: Graphviz DOT Language
  url: https://graphviz.org/doc/info/lang.html

acceptance_tests:
- id: uat-001
  name: "bootstrap --reconstruct creates PRDs from git history with depends_on"
  command: cargo make uat
  uat_status: unverified
- id: uat-002
  name: "graph ascii renders dependency graph in terminal"
  command: cargo make uat
  uat_status: unverified
- id: uat-003
  name: "graph mermaid outputs valid Mermaid flowchart"
  command: cargo make uat
  uat_status: unverified
- id: uat-004
  name: "graph dot outputs valid Graphviz DOT format"
  command: cargo make uat
  uat_status: unverified
- id: uat-005
  name: "reindex auto-fixes depends_on using LLM"
  command: cargo make uat
  uat_status: unverified
- id: uat-006
  name: "graph warns and renders dashed nodes for missing depends_on references"
  command: cargo make uat
  uat_status: unverified

tasks:
- id: T-001
  title: "Add depends_on field to PrdFrontmatter struct"
  priority: 1
  status: done
  notes: "Add `depends_on: Option<Vec<String>>` to src/prd/types.rs. List of PRD IDs (e.g., ['PRD-0001', 'PRD-0003'])."
- id: T-002
  title: "Add reconstructed field to PrdFrontmatter struct"
  priority: 2
  status: done
  notes: "Add `reconstructed: Option<bool>` to mark PRDs created via --reconstruct."
- id: T-003
  title: "Update PRD template to include depends_on field"
  priority: 3
  status: done
  notes: "Add optional depends_on field to .mr/templates/prd.md and init.rs embedded template."
- id: T-004
  title: "Implement --reconstruct flag for bootstrap command"
  priority: 4
  status: done
  notes: "Add flag to main.rs CLI, update BootstrapConfig. When set, runs reconstruct workflow instead of normal bootstrap."
- id: T-005
  title: "Create bootstrap_reconstruct.md prompt"
  priority: 5
  status: done
  notes: "Agent-driven prompt that analyzes commits, tags, major changes to infer historical PRDs. Add to init.rs and PromptKind."
- id: T-006
  title: "Implement reconstruct logic in bootstrap.rs"
  priority: 6
  status: done
  notes: "New function that invokes runner with reconstruct prompt. Infers depends_on relationships from temporal order. Sets status: done and reconstructed: true."
- id: T-007
  title: "Handle idempotency for --reconstruct with existing PRDs"
  priority: 7
  status: done
  notes: "Skip/merge with existing PRDs. Only create new PRDs for work not covered by existing ones."
- id: T-008
  title: "Create graph module with shared graph-building logic"
  priority: 8
  status: done
  notes: "New src/graph.rs with GraphNode, GraphEdge structs and build_graph(root) function. Handle missing depends_on refs as warnings."
- id: T-009
  title: "Implement graph ascii subcommand"
  priority: 9
  status: done
  notes: "Render ASCII art dependency graph to terminal. Show missing refs as dashed/special nodes with warning."
- id: T-010
  title: "Implement graph mermaid subcommand"
  priority: 10
  status: done
  notes: "Output Mermaid flowchart syntax for GitHub rendering. Use dashed lines for missing refs."
- id: T-011
  title: "Implement graph dot subcommand"
  priority: 11
  status: todo
  notes: "Output Graphviz DOT format. Use dashed style for missing refs."
- id: T-012
  title: "Add graph command to CLI with subcommands"
  priority: 12
  status: todo
  notes: "Add to main.rs: `mr graph ascii`, `mr graph mermaid`, `mr graph dot`."
- id: T-013
  title: "Enhance reindex to auto-fix depends_on using LLM"
  priority: 13
  status: todo
  notes: "Update reindex.rs to invoke runner with prompt that analyzes PRDs and infers/fixes depends_on relationships. Write changes in-place."
- id: T-014
  title: "Create reindex_depends_on.md prompt"
  priority: 14
  status: todo
  notes: "Prompt for LLM to analyze PRD summaries/dates and infer dependencies. Add to init.rs and PromptKind."
- id: T-015
  title: "Update PRDS.md index generation to include depends_on info"
  priority: 15
  status: todo
  notes: "Show dependency relationships in the index if present."
- id: T-016
  title: "Add unit tests for graph building and rendering"
  priority: 16
  status: todo
  notes: "Test graph construction, missing ref handling, and each output format."
- id: T-017
  title: "Add integration tests for reconstruct and graph commands"
  priority: 17
  status: todo
  notes: "Test full workflow with mock runner."
- id: T-018
  title: "Update AGENTS.md with graph and reconstruct documentation"
  priority: 18
  status: todo
  notes: "Document new commands, flags, and workflows."

---

# Summary

This PRD adds three related capabilities to microralph: (1) a `--reconstruct` flag for `bootstrap` that analyzes git history to create PRDs representing major development milestones, (2) a `depends_on` field in PRD frontmatter to express dependency relationships between PRDs, and (3) a new `graph` command with subcommands for rendering the dependency graph in ASCII, Mermaid, and DOT formats. Additionally, `reindex` is enhanced to auto-fix `depends_on` relationships using LLM analysis.

---

# Problem

Currently, microralph has no way to:
1. **Reconstruct history**: For existing repos without PRDs, there's no way to retroactively create PRDs that represent the major work that led to the current state.
2. **Express dependencies**: PRDs exist in isolation with no formal way to express that one PRD should be completed before another or was built upon another's work.
3. **Visualize relationships**: There's no way to see the dependency graph between PRDs, making it hard to understand the evolution of the project or identify what needs to be done first.

This makes it harder to onboard to existing projects and understand the logical structure of completed and pending work.

---

# Goals

1. Add `--reconstruct` flag to `bootstrap` that uses LLM to analyze git history (commits, tags, major changes) and create PRDs representing major milestones, each with `status: done`, `reconstructed: true`, and inferred `depends_on` relationships.
2. Add `depends_on` field to PRD YAML frontmatter as a list of PRD IDs representing logical/temporal dependencies ("this PRD should be done after these").
3. Implement `mr graph` command with three subcommands (`ascii`, `mermaid`, `dot`) that render the dependency graph in different formats, with each format handler in its own function for SOC.
4. Enhance `reindex` to auto-fix `depends_on` relationships using LLM analysis of PRD content and dates.
5. Handle missing `depends_on` references gracefully in graph rendering (warn and show as dashed/special nodes).
6. Ensure `--reconstruct` is idempotent with existing PRDs (skip/merge, only add new ones for uncovered work).
7. All "decisions" should be made by LLM agent.  No Rust code should be editing these files.  We just want the agent to make the updates in place.

---

# Non-Goals (MVP)

- Interactive graph navigation or web-based visualization
- Filtering options for graph (e.g., `--status done`, `--root PRD-XXXX`)
- Cycle detection or enforcement (depends_on is advisory, not blocking)
- Migration of existing 25 PRDs (user will run `reindex` manually after this PRD is complete)
- Bi-directional cross-references beyond depends_on (existing Cross-References section in PRDS.md remains separate)

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-27 — T-001 Completed
- **Task**: Add depends_on field to PrdFrontmatter struct
- **Status**: ✅ Done
- **Changes**:
  - Added `depends_on: Option<Vec<String>>` field to `PrdFrontmatter` struct in `src/prd/types.rs`
  - Field includes doc comment explaining it represents PRD IDs that this PRD depends on
  - Uses `#[serde(skip_serializing_if = "Option::is_none")]` to follow existing conventions
  - UAT passed: 360 tests run, 360 passed

---

## 2026-01-27 — T-003 Completed
- **Task**: Update PRD template to include depends_on field
- **Status**: ✅ Done
- **Changes**:
  - Added commented `depends_on` field to PRD template in `src/init.rs` (`PRD_TEMPLATE` constant)
  - Updated `.mr/templates/prd.md` with the same commented `depends_on` field
  - Field is commented out by default with example PRD IDs and usage instructions
  - Placed after `updated:` date field and before `principles:` section for logical grouping
  - Both sources are now synchronized per the constitution's Prompt Management rule
  - UAT passed: 360 tests run, 360 passed

---

## 2026-01-27 — T-004 Completed
- **Task**: Implement --reconstruct flag for bootstrap command
- **Status**: ✅ Done
- **Changes**:
  - Added `--reconstruct` flag to Bootstrap command in `src/main.rs`
  - Added `reconstruct: bool` field to `BootstrapConfig` struct in `src/bootstrap.rs`
  - Updated `cmd_bootstrap` function to accept and pass the reconstruct flag
  - Added conditional output message when reconstruct mode is enabled
  - Added new test `test_args_parse_bootstrap_with_reconstruct` to verify flag parsing
  - Updated existing bootstrap tests to include the new field in pattern matches
  - UAT passed: 361 tests run, 361 passed
- **Constitution Compliance**: No violations. Changes were minimal and surgical.

---

## 2026-01-27 — T-005 Completed
- **Task**: Create bootstrap_reconstruct.md prompt
- **Status**: ✅ Done
- **Changes**:
  - Added `BootstrapReconstruct` variant to `PromptKind` enum in `src/prompt/types.rs`
  - Added `PROMPT_BOOTSTRAP_RECONSTRUCT` constant in `src/init.rs` with comprehensive prompt for analyzing git history
  - Added mapping in `get_default_prompt()` in `src/prompt/loader.rs`
  - Added prompt file creation in both `init()` and `init_prompts_and_templates()` functions in `src/init.rs`
  - Created physical `.mr/prompts/bootstrap_reconstruct.md` file synchronized with the embedded constant
  - Updated test counts: 17 → 18 prompts across `PromptKind::all()`, `test_init_creates_structure`, `test_init_is_idempotent`, and `test_prompt_loader_missing_prompts`
  - UAT passed: 361 tests run, 361 passed
- **Constitution Compliance**: No violations. Prompt Management rule followed — prompt defined in `init.rs` and materialized to `.mr/prompts/`.

---

## 2026-01-27 — T-006 Completed
- **Task**: Implement reconstruct logic in bootstrap.rs
- **Status**: ✅ Done
- **Changes**:
  - Updated module doc comment in `src/bootstrap.rs` to describe reconstruct mode
  - Modified `bootstrap()` function to branch on `config.reconstruct` flag and call `bootstrap_reconstruct()`
  - Added `bootstrap_reconstruct()` function that invokes runner with `PromptKind::BootstrapReconstruct` prompt
  - Added `build_reconstruct_prompt()` helper function to load and expand the reconstruct prompt template
  - Added 4 new unit tests: `test_bootstrap_reconstruct_workflow`, `test_bootstrap_reconstruct_skips_init_if_exists`, `test_bootstrap_reconstruct_runner_failure`, `test_build_reconstruct_prompt`
  - UAT passed: 365 tests run, 365 passed
- **Constitution Compliance**: No violations. Minimal changes to existing code, reused existing patterns.

---

## 2026-01-27 — T-007 Completed
- **Task**: Handle idempotency for --reconstruct with existing PRDs
- **Status**: ✅ Done
- **Changes**:
  - Updated `build_reconstruct_prompt()` in `src/bootstrap.rs` to scan for existing PRDs using `scan_prd_summaries()` and include them in the prompt context
  - Added `HashMap` import and `PlaceholderValue::List` for existing PRD data
  - Updated `PROMPT_BOOTSTRAP_RECONSTRUCT` constant in `src/init.rs` to include "Existing PRDs (Do Not Duplicate)" section with `{{#if existing_prds}}` and `{{#each existing_prds}}` placeholders
  - Synchronized `.mr/prompts/bootstrap_reconstruct.md` with the updated constant
  - Added 2 new unit tests: `test_build_reconstruct_prompt_includes_existing_prds`, `test_build_reconstruct_prompt_no_existing_prds`
  - UAT passed: 367 tests run, 367 passed
- **Constitution Compliance**: No violations. Prompt Management rule followed — prompt defined in `init.rs` and materialized to `.mr/prompts/`.

---

## 2026-01-27 — T-008 Completed
- **Task**: Create graph module with shared graph-building logic
- **Status**: ✅ Done
- **Changes**:
  - Created new `src/graph.rs` module with PRD dependency graph data structures and functions
  - Added `GraphNode` struct with fields: `id`, `title`, `status`, `is_missing`
  - Added `GraphEdge` struct with fields: `from`, `to`, `is_missing` for directed dependency edges
  - Added `PrdGraph` struct aggregating nodes, edges, missing_refs, and warnings
  - Implemented `build_graph(root)` function that scans PRDs and builds dependency graph from `depends_on` fields
  - Implemented `build_graph_from_prds()` function for direct PRD slice input (useful for testing)
  - Missing `depends_on` references are handled as warnings (logged via `tracing::warn!`) and represented as placeholder nodes with `is_missing: true`
  - Added `from_summary()` and `missing()` constructors for `GraphNode`
  - Added 10 unit tests covering: no dependencies, valid dependencies, missing dependencies, multiple/chain dependencies, deduplication, and actual repo integration test
  - Module uses `#[allow(dead_code)]` as public APIs will be consumed by T-009, T-010, T-011, T-012
  - UAT passed: 377 tests run, 377 passed
- **Constitution Compliance**: No violations. Minimal changes, follows existing module patterns.

---

## 2026-01-27 — T-009 Completed
- **Task**: Implement graph ascii subcommand
- **Status**: ✅ Done
- **Changes**:
  - Added `AsciiConfig` struct to configure ASCII rendering (show_titles, max_title_len)
  - Implemented custom `Default` trait for `AsciiConfig` with sensible defaults (show_titles: true, max 40 chars)
  - Added `render_ascii()` function that renders a `PrdGraph` as ASCII art
  - Output includes header, node listings with `[ID] Title (status)` format
  - Dependencies shown with tree connectors (`├──` and `└──`)
  - Missing references rendered in separate section with dashed format (`- PRD-XXXX -`) and warning about what references them
  - Summary stats at bottom showing PRD count, edge count, and missing count
  - Added 7 unit tests: empty graph, single node, with dependencies, missing refs, multiple deps, config no titles, title truncation
  - UAT passed: 384 tests run, 384 passed
- **Constitution Compliance**: No violations. Minimal changes, follows existing module patterns and conventions.

---

## 2026-01-27 — T-010 Completed
- **Task**: Implement graph mermaid subcommand
- **Status**: ✅ Done
- **Changes**:
  - Added `MermaidConfig` struct with `show_titles`, `max_title_len`, and `direction` fields
  - Added `MermaidDirection` enum with `TopDown` and `LeftRight` variants
  - Implemented `render_mermaid()` function that renders a `PrdGraph` as Mermaid flowchart syntax
  - Output starts with `flowchart TD` (or `LR` based on config)
  - Node definitions use `ID["label"]` format with ID: Title (status) labels
  - Missing nodes use `{{double braces}}` shape with `:::missing` class styling
  - Valid dependencies rendered with solid arrows (`-->`)
  - Missing dependencies rendered with dashed arrows (`-.->`)
  - Added `classDef missing` for visual differentiation (red dashed outline)
  - Added helper functions: `mermaid_node_id()` (removes hyphens), `mermaid_node_label()` (formats label with title/status)
  - Added 9 unit tests: empty graph, single node, with dependencies, missing refs, multiple deps, config no titles, left-right direction, title truncation, node ID conversion
  - UAT passed: 393 tests run, 393 passed
- **Constitution Compliance**: No violations. Minimal changes, follows existing module patterns (mirrors ASCII rendering structure).

---