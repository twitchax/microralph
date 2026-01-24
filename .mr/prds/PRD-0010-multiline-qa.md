---
id: PRD-0010
title: Support Multi-line Q/A During PRD Creation
status: draft
owner: ""
created: 2026-01-24
updated: 2026-01-24
principles:
  - Keep implementation simple; avoid complex formatting if it adds significant complexity
  - Detect newline characters in model output for questions
  - Use double-enter pattern for multi-line answer input
  - Support unbounded multi-line content
references: []
acceptance_tests:
  - id: uat-001
    name: Question with bullet list displays completely
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: Multi-line answer is captured correctly
    command: cargo make uat
    uat_status: unverified
tasks:
  - id: T-001
    title: Detect and display newlines in model-generated questions
    priority: 1
    status: todo
    notes: Model output sometimes includes newlines for lists/formatting; ensure full question is displayed with proper line breaks
  - id: T-002
    title: Support multi-line answer input with double-enter termination
    priority: 2
    status: todo
    notes: User presses Enter twice to finish multi-line answer; simple approach preferred
  - id: T-003
    title: Verify UAT scenarios pass
    priority: 3
    status: todo
    notes: Test with questions containing bullet lists and multi-line answers
---

# Summary

Enable proper multi-line support for both questions and answers during the PRD creation Q/A session. Currently, questions from the model that contain newlines (e.g., questions with bullet lists) are truncated, and answers cannot span multiple lines.

# Problem

During `mr new`, the Q/A session does not correctly handle multi-line content:
- **Questions**: Model-generated questions that include newlines or lists are truncated or not fully displayed
- **Answers**: User cannot provide multi-line answers, limiting expressiveness for complex responses

This limitation makes the interactive PRD creation process less effective when dealing with structured or detailed content.

# Goals

1. Detect and properly display newline characters in model-generated questions
2. Allow users to input multi-line answers using a double-enter pattern (press Enter twice to finish)
3. Support unbounded multi-line content for both questions and answers

# Non-Goals (MVP)

- Advanced formatting (indentation, markdown rendering) for questions or answers
- Shift+Enter behavior for multi-line input (deferred to keep implementation simple)
- Maximum length or line limits for multi-line content
- Editing or reformatting of multi-line content after input

# History