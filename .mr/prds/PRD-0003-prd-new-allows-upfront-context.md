---
id: PRD-0003
title: PRD New Allows Upfront Context
status: draft
owner: twitchax
created: 2026-01-24
updated: 2026-01-24
tasks:
  - id: T-001
    title: Add --context CLI flag to prd new command
    priority: 1
    status: todo
  - id: T-002
    title: Add upfront context prompt before question generation
    priority: 1
    status: todo
  - id: T-003
    title: Pass context to initial question generation prompt
    priority: 1
    status: todo
  - id: T-004
    title: Persist context through all Q/A rounds
    priority: 2
    status: todo
  - id: T-005
    title: Include context in final PRD synthesis prompt
    priority: 2
    status: todo
  - id: T-006
    title: Update help text and documentation
    priority: 3
    status: todo
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

## Acceptance Tests

1. **Interactive flow**: Running `mr prd new foo` prompts "Do you want to add more context?" before generating questions. User can type context or skip.
2. **Flag flow**: Running `mr prd new --context "This is a CLI tool for X" foo` skips the interactive prompt and uses provided context immediately.
3. **Context in questions**: When context mentions specific technologies or constraints, the generated questions reference them.
4. **Context persists**: The context is visible/used in subsequent Q/A rounds (not just the first).
5. **Synthesis includes context**: The final PRD synthesis prompt includes the upfront context alongside Q/A answers.

## History

