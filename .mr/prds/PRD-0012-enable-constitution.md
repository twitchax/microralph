---
id: PRD-0012
title: Enable Constitution
status: draft
owner: twitchax
created: 2026-01-24
updated: 2026-01-24

principles:
- Constitution is version-controlled and part of project governance
- Constitution enforcement is informational, not blocking
- Constitution can be updated intelligently via LLM

references:
- name: PRD-0001 (bootstrap implementation)
  url: ./PRD-0001-build-micro-ralph-mvp.md
- name: PRD-0003 (upfront context pattern)
  url: ./PRD-0003-prd-new-allows-upfront-context.md
- name: PRD-0004 (finalization workflow)
  url: ./PRD-0004-prd-finalization-steps.md

acceptance_tests:
- id: uat-001
  name: Bootstrap creates initial constitution file
  command: cargo make uat constitution_bootstrap
  uat_status: unverified
- id: uat-002
  name: Constitution contains numbered example rules
  command: cargo make uat constitution_template
  uat_status: unverified
- id: uat-003
  name: prd new reads and respects constitution
  command: cargo make uat constitution_prd_new
  uat_status: unverified
- id: uat-004
  name: prd finalize reads and respects constitution
  command: cargo make uat constitution_prd_finalize
  uat_status: unverified
- id: uat-005
  name: constitution edit command updates via LLM
  command: cargo make uat constitution_edit
  uat_status: unverified
- id: uat-006
  name: Runner logs violations in PRD history
  command: cargo make uat constitution_violation_logging
  uat_status: unverified

tasks:
- id: T-001
  title: Create constitution template with numbered examples
  priority: 1
  status: todo
  notes: Create a markdown template with commented-out numbered example rules (e.g., "1. One-off acceptance tests are unacceptable..."). Should be mostly empty but provide clear guidance.
- id: T-002
  title: Emit constitution during bootstrap
  priority: 1
  status: todo
  notes: Update bootstrap.rs to create .mr/constitution.md from template. Constitution should be placed alongside configs in .mr/ directory.
- id: T-003
  title: Add constitution edit subcommand
  priority: 2
  status: todo
  notes: Add `mr constitution edit <request>` command that invokes the runner/LLM to intelligently update the constitution based on natural language request.
- id: T-004
  title: Load constitution in runner context
  priority: 2
  status: todo
  notes: Add constitution reading function in runner.rs or config.rs. Constitution should be loaded and available to runner prompts.
- id: T-005
  title: Include constitution in prd new prompts
  priority: 2
  status: todo
  notes: Update prd_new_round1_questions.md and prd_new_roundN_questions.md to include constitution content in prompt context.
- id: T-006
  title: Include constitution in prd finalize prompts
  priority: 2
  status: todo
  notes: Update prd_finalize prompts to include constitution content, enabling finalization to respect project governance rules.
- id: T-007
  title: Update runner prompts to log constitution violations
  priority: 3
  status: todo
  notes: Modify runner prompts to instruct LLM to mention and reason about constitution violations in PRD history entries. No programmatic enforcement needed.
- id: T-008
  title: Document constitution feature in README
  priority: 3
  status: todo
  notes: Add section explaining constitution purpose, location, editing workflow, and how it influences PRD creation and execution.

---

# Summary

Add a `.mr/constitution.md` file that defines project-specific governance rules and constraints. The constitution is emitted during `mr bootstrap`, version-controlled, and user-editable. Commands like `mr prd new` and `mr prd finalize` read and respect the constitution, and a new `mr constitution edit <request>` command allows intelligent updates via LLM. Constitution violations are logged in PRD history with reasoning but do not block execution.

---

# Problem

microralph currently has no mechanism to encode project-specific constraints, best practices, or governance rules that should influence PRD creation and execution. For example, a project might require that all acceptance tests be codified (not one-offs), or that certain architectural patterns be followed. Without a constitution, these rules must be manually enforced or repeatedly mentioned, and there's no single source of truth for project governance.

---

# Goals

1. **Bootstrap emits constitution**: `mr bootstrap` creates `.mr/constitution.md` with numbered, commented-out example rules.
2. **Version-controlled governance**: Constitution lives in `.mr/` alongside configs and is committed to version control.
3. **Intelligent editing**: `mr constitution edit <request>` invokes LLM to update constitution based on natural language request.
4. **PRD workflow integration**: `mr prd new` and `mr prd finalize` read constitution and incorporate it into LLM prompts.
5. **Violation logging**: Runner logs constitution violations with reasoning in PRD history; no blocking enforcement.

---

# Non-Goals (MVP)

- Programmatic enforcement (e.g., failing builds on violation)
- Constitution validation or schema beyond markdown format
- Multi-file constitution or imports
- Constitution versioning or rollback beyond git
- Constitution diff or approval workflow
- Support for other constitution formats (TOML, YAML)

---

# History

(Entries appended by `mr run` will go below this line.)

---