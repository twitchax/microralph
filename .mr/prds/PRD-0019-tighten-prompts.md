---
id: PRD-0019
title: Tighten Prompts and Encourage Constitution
status: draft
owner: twitchax
created: 2026-01-26
updated: 2026-01-26
principles:
  - Prompts should be workflow-focused (what to do) without philosophy
  - Constitution should contain all behavioral guidance (how to behave)
  - Users should edit only the constitution to change overall agent behavior
  - Reduce duplication of behavioral rules across multiple prompt files
references: []
acceptance_tests:
  - id: uat-001
    name: All scattered "don't modify unrelated code" instances removed from prompts
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: Constitution has comprehensive behavior rules (under 10 rules)
    command: cargo make uat
    uat_status: unverified
  - id: uat-003
    name: Prompts are workflow-focused with no philosophy or opinion
    command: cargo make uat
    uat_status: unverified
tasks:
  - id: T-001
    title: Audit all 16 prompt files for behavioral/philosophical guidance
    priority: 1
    status: todo
    notes: Identify all instances of "don't modify unrelated code", DRY, minimal changes, follow patterns, etc.
  - id: T-002
    title: Draft comprehensive constitutional rules covering identified themes
    priority: 2
    status: todo
    notes: Rules should include DRY, SOC, minimal changes, language best practices, public API stability, research expectations
  - id: T-003
    title: Update default constitution in src/init.rs with new rules
    priority: 3
    status: todo
    notes: Keep under 10 rules, make generic/language-agnostic, expect users to customize
  - id: T-004
    title: Remove behavioral guidance from all prompt templates in src/init.rs
    priority: 4
    status: todo
    notes: Keep prompts purely operational (workflow steps and constraints)
  - id: T-005
    title: Test updated prompts and constitution with mr restore
    priority: 5
    status: todo
    notes: Verify prompts are tighter and constitution is comprehensive
  - id: T-006
    title: Update AGENTS.md with new prompt/constitution philosophy if needed
    priority: 6
    status: todo
    notes: Document the "what to do vs how to behave" boundary
---

# Summary

Consolidate all behavioral guidance and philosophy from scattered prompt files into the constitution, reducing duplication and creating a single source of truth for agent behavior. Tighten all 16 prompt files to focus purely on workflow steps and constraints, leaving the constitution as the sole location for editing how agents behave.

# Problem

Currently, behavioral rules like "don't modify unrelated code", "follow DRY", "minimal changes", and "follow existing patterns" are repeated across multiple prompt files. This creates maintenance burden and forces users to edit many files to change agent behavior. Prompts mix operational instructions ("what to do") with philosophical guidance ("how to behave"), making them verbose and duplicative.

# Goals

1. Move all behavioral/philosophical guidance from prompts to constitution
2. Update default constitution (src/init.rs) with comprehensive rules covering DRY, SOC, minimal changes, language best practices, public API stability, and research expectations
3. Tighten all 16 prompt templates in src/init.rs to be workflow-focused
4. Establish clear boundary: prompts define "what to do" and constraints, constitution defines "how to behave"
5. Reduce duplication so users edit only one file (constitution) to change agent behavior
6. Keep constitution under 10 rules, generic/language-agnostic, and user-customizable

# Non-Goals (MVP)

- Modifying user-customized prompt files (this affects defaults only)
- Adding project-specific or language-specific rules to default constitution
- Changing the structure or format of prompts or constitution
- Implementing new prompt or constitution features beyond consolidation

# History