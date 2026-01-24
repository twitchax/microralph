---
id: PRD-0003
title: PRD New Allows Upfront Context
status: draft
owner: twitchax
created: 2026-01-24
updated: 2026-01-24

principles:
- Context should be opt-in (interactive prompt or CLI flag) to avoid mandatory extra steps.
- Context is ephemeral; used only during PRD creation, not stored in the final PRD.
- Better context leads to better initial questions from the AI.

references:
- name: PRD-0001 (prd new implementation)
  url: ./PRD-0001-build-micro-ralph-mvp.md

acceptance_tests:
- id: uat-001
  name: Interactive flow prompts for context
  command: cargo make mr:test prd_new_context_interactive
- id: uat-002
  name: Flag flow uses provided context
  command: cargo make mr:test prd_new_context_flag
- id: uat-003
  name: Context influences question generation
  command: cargo make mr:test prd_new_context_in_questions
- id: uat-004
  name: Context persists through Q/A rounds
  command: cargo make mr:test prd_new_context_persistence
- id: uat-005
  name: Context included in final synthesis
  command: cargo make mr:test prd_new_context_synthesis

tasks:
- id: T-001
  title: Add --context CLI flag to prd new command
  priority: 1
  status: done
  notes: Add optional --context argument to the prd new subcommand in main.rs.
- id: T-002
  title: Add upfront context prompt before question generation
  priority: 1
  status: done
  notes: Before invoking round 1 questions, ask the user if they want to add more context.
- id: T-003
  title: Pass context to initial question generation prompt
  priority: 1
  status: todo
  notes: Include user_context in the PlaceholderContext for prd_new_round1_questions.md.
- id: T-004
  title: Persist context through all Q/A rounds
  priority: 2
  status: todo
  notes: Store context in PrdNewConfig and include it in all roundN prompts.
- id: T-005
  title: Include context in final PRD synthesis prompt
  priority: 2
  status: todo
  notes: Add user_context to prd_new_synthesize_prd.md placeholder expansion.
- id: T-006
  title: Update help text and documentation
  priority: 3
  status: todo
  notes: Update README placeholder docs and CLI help for the new --context flag.
---

## Summary

Enhance `mr prd new` to allow users to provide upfront context before the first round of AI-generated questions. Context can be provided interactively (via a prompt asking "Do you want to add more context?") or directly via a `--context` CLI flag. This context is carried through all Q/A rounds and into final PRD synthesis, enabling more relevant and targeted questions from the start.

## Problem

Currently, `mr prd new <slug>` generates initial questions based only on the PRD slug name. This provides minimal context for the AI, often resulting in generic or less targeted questions. Users may have important context (project constraints, related systems, user stories, file contents) that would help the AI ask better, more relevant questions from the first round.

## Goals

1. **Interactive context prompt**: By default, before generating questions, ask the user "Do you want to add more context?" allowing them to provide free-form text.
2. **CLI flag for context**: Support `--context "text"` flag to pass context directly, skipping the interactive prompt.
3. **Context persistence**: The provided context is included in all subsequent Q/A rounds and in the final PRD synthesis prompt.
4. **Better initial questions**: The AI uses the upfront context to generate more relevant, targeted questions about the feature.

## Non-Goals

- Automatic file reading (users can paste file contents into context text if desired).
- Structured context format (context is free-form text).
- Context stored in PRD frontmatter (context is ephemeral, used only during creation).

## History

## 2026-01-24 — T-001 Completed
- **Task**: Add --context CLI flag to prd new command
- **Status**: ✅ Done
- **Changes**:
  - Added `--context <CONTEXT>` optional argument to `PrdCommand::New` in `src/main.rs`
  - Updated `cmd_prd_new` function signature to accept `context: Option<&str>`
  - Added `context` field to `PrdNewConfig` struct in `src/prd_new.rs`
  - Updated all tests to include the new `context` field
  - UAT: All 227 tests passed
  - CLI help now shows: `--context <CONTEXT>  Upfront context to provide before question generation`

## 2026-01-24 — T-002 Completed
- **Task**: Add upfront context prompt before question generation
- **Status**: ✅ Done
- **Changes**:
  - Added `prompt_for_context` function in `src/prd_new.rs` to interactively prompt users for optional context
  - Modified `create_prd` function to call `prompt_for_context` when no `--context` flag is provided
  - Updated `build_round1_prompt` to accept and include `user_context` in placeholder expansion
  - Removed `#[allow(dead_code)]` attribute from `context` field in `PrdNewConfig`
  - UAT: All 227 tests passed

