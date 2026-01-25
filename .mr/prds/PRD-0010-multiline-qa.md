---
id: PRD-0010
title: Support Multi-line Q/A During PRD Creation
status: active
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
    uat_status: verified
  - id: uat-002
    name: Multi-line answer is captured correctly
    command: cargo make uat
    uat_status: verified
tasks:
  - id: T-001
    title: Detect and display newlines in model-generated questions
    priority: 1
    status: done
    notes: Model output sometimes includes newlines for lists/formatting; ensure full question is displayed with proper line breaks
  - id: T-002
    title: Support multi-line answer input with double-enter termination
    priority: 2
    status: done
    notes: User presses Enter twice to finish multi-line answer; simple approach preferred
  - id: T-003
    title: Verify UAT scenarios pass
    priority: 3
    status: done
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

## 2026-01-24 — T-001 Completed
- **Task**: Detect and display newlines in model-generated questions
- **Status**: ✅ Done
- **Changes**:
  - Modified `parse_questions()` function in `src/prd_new.rs` to capture multi-line questions instead of truncating after the first line
  - Enhanced parsing logic to accumulate question content across multiple lines until an empty line or next question number is encountered
  - Updated `collect_answers()` function to display multi-line questions with proper formatting (first line with number, subsequent lines indented)
  - Added `test_parse_questions_multiline_with_bullets` test to verify multi-line question parsing works correctly
  - All 272 UAT tests pass
- **UATs Verified**:
  - uat-001: Question with bullet list displays completely ✅ (verified via `test_parse_questions_multiline_with_bullets`)

## 2026-01-25 — T-002 Completed
- **Task**: Support multi-line answer input with double-enter termination
- **Status**: ✅ Done
- **Changes**:
  - Implementation already exists in `collect_answers()` function (lines 659-683 of `src/prd_new.rs`)
  - Multi-line answer input works by:
    - User types first line and presses Enter
    - System prompts with `> ` for additional lines
    - User presses Enter on blank line (double-enter) to finish
    - All lines are joined with newlines and stored
  - Existing test `test_collect_answers_multiline` verifies functionality
  - All 273 UAT tests pass
- **UATs Verified**:
  - uat-002: Multi-line answer is captured correctly ✅ (verified via existing implementation and `test_collect_answers_multiline` test)

## 2026-01-25 — T-003 Completed
- **Task**: Verify UAT scenarios pass
- **Status**: ✅ Done
- **Changes**:
  - Ran `cargo make uat` to verify all acceptance tests pass
  - All 273 tests passed successfully
  - Both acceptance tests (uat-001 and uat-002) were already verified by previous tasks:
    - uat-001: Question with bullet list displays completely (verified via `test_parse_questions_multiline_with_bullets`)
    - uat-002: Multi-line answer is captured correctly (verified via `test_collect_answers_multiline`)
  - No code changes required; all functionality already implemented and tested