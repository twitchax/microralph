---
id: PRD-0014
title: Cleanup Pass - Refactoring and Documentation
status: done
owner: microralph
created: 2026-01-24
updated: 2026-01-25
principles:
- Extract clearly duplicated logic (e.g., handlebars helpers) to shared modules
- Reduce clones where practical, especially in high-density areas like run.rs
- Add comments to complex parsing, orchestration, and state machine logic
- Prefer `?` operator over explicit error handling
- Avoid string concatenation in loops
- Exit criteria is subjective improvement validated by UAT only
references: []
acceptance_tests:
- id: uat-001
  name: All existing tests pass after refactoring
  command: cargo make uat
  uat_status: verified
tasks:
- id: T-001
  title: Extract duplicated Q/A workflow patterns to shared modules
  priority: 1
  status: done
  notes: Focus on parse_questions, extract_prd_content, and prompt building logic across prd_new.rs, prd_edit.rs, and run.rs. Only extract if truly duplicated, not semi-duplicated with context-specific variations.
- id: T-002
  title: Reduce unnecessary clones in run.rs
  priority: 2
  status: done
  notes: Target the ~154 clone calls. Focus on cases where owned data isn't needed. Balance risk vs. benefit.
- id: T-003
  title: Add comments to complex parsing logic
  priority: 3
  status: done
  notes: Target prd/parser.rs and prompt/expand.rs. Explain non-obvious logic.
- id: T-004
  title: Add comments to orchestration state machines
  priority: 3
  status: done
  notes: Document run.rs UAT loop and prd_new.rs multi-round Q/A flows.
- id: T-005
  title: Check and fix Rust idiom violations
  priority: 4
  status: done
  notes: Prefer `?` operator, avoid string concatenation in loops. Don't force changes if too risky.
- id: T-006
  title: Identify and fix obvious performance issues
  priority: 4
  status: done
  notes: Spot checks during refactoring. Don't do deep profiling.
---

# Summary

Perform a comprehensive cleanup pass over the codebase to reduce duplication, improve code documentation, align with Rust idioms, and reduce unnecessary allocations. This is a quality-of-life improvement with subjective success criteria validated solely by UAT.

# Problem

The codebase has grown organically and now contains:
- Duplicated Q/A workflow patterns across `prd_new.rs`, `prd_edit.rs`, and `run.rs`
- High clone density (~154 calls in `run.rs` alone)
- Under-commented complex logic in parsing and orchestration modules
- Inconsistent use of Rust idioms (explicit error handling vs. `?`, string concatenation)
- No systematic review for obvious performance inefficiencies

# Goals

1. Extract clearly duplicated logic into shared modules where it makes sense
2. Reduce unnecessary clones, especially in high-density files like `run.rs`
3. Add comments to complex parsing logic, orchestration state machines, and other non-obvious areas
4. Align code with Rust idioms (prefer `?` operator, avoid string concatenation in loops)
5. Fix obvious performance issues identified during cleanup

# Non-Goals (MVP)

- Deep profiling or micro-optimization
- Aggressive refactoring that introduces significant risk
- Forcing Rust pattern changes where existing code is reasonable
- Achieving specific quantitative targets (e.g., "zero clones", "100% doc coverage")
- Changing behavior or fixing unrelated bugs

# History

## 2026-01-25 — T-001 Completed
- **Task**: Extract duplicated Q/A workflow patterns to shared modules
- **Status**: ✅ Done
- **Changes**:
  - Created new `src/qa_workflow.rs` module with shared Q/A workflow utilities
  - Extracted `QaPair` struct from `prd_new.rs` to shared module
  - Extracted and unified `parse_questions()` function (supports multi-line questions)
  - Extracted and unified `extract_prd_content()` function (robust version with ANSI stripping, code fence handling)
  - Extracted `strip_ansi_escapes()` helper function
  - Created two variants of answer collection: `collect_multiline_answers()` and `collect_singleline_answers()`
  - Updated `prd_new.rs` to use shared module (uses multiline variant)
  - Updated `prd_edit.rs` to use shared module (uses singleline variant)
  - Updated `constitution_edit.rs` to import QaPair from shared module
  - Added comprehensive tests in `qa_workflow.rs` covering all extracted functions
  - UAT passed: All 312 tests pass after refactoring

## 2026-01-25 — T-002 Completed
- **Task**: Reduce unnecessary clones in run.rs
- **Status**: ✅ Done
- **Changes**:
  - Reduced clone calls in `src/run.rs` from 16 to 10 (37.5% reduction)
  - Replaced `task.title.clone()` with `task.title.as_str()` in context insertion (line 294)
  - Replaced `notes.clone()` with `notes.as_str()` in context insertion (line 298)
  - Replaced `output.text.clone()` with direct move of `output.text` (line 425)
  - Replaced `uat.id.clone()` with `uat.id.as_str()` in context insertion (line 455)
  - Replaced `uat.name.clone()` with `uat.name.as_str()` in context insertion (line 456)
  - Replaced `uat.command.clone()` with `uat.command.as_str()` in context insertion (line 457)
  - UAT passed: All 312 tests pass after refactoring
  - Opportunistically verified uat-001: All existing tests continue to pass

## 2026-01-25 — T-003 Completed
- **Task**: Add comments to complex parsing logic
- **Status**: ✅ Done
- **Changes**:
  - Enhanced `src/prd/parser.rs::split_frontmatter()` with inline comments explaining:
    - Byte offset calculation for moving past delimiters
    - Newline-prefixed delimiter search strategy
    - Frontmatter extraction and body normalization logic
  - Enhanced `src/prompt/expand.rs::expand_simple_placeholders()` with inline comments explaining:
    - Character-by-character parsing with peekable iterator
    - Block tag detection and skip logic ({{#if}}, {{/each}})
    - Variable name collection and closing brace detection
    - Block-scoped reference handling ({{@index}})
    - Value substitution and fallback for unknown placeholders
  - Enhanced `src/prompt/expand.rs::expand_if_blocks()` with inline comments explaining:
    - Iterative block processing and offset recalculation
    - Tag extraction and matching logic
    - Truthiness evaluation for different value types
    - Block replacement strategy
  - Enhanced `src/prompt/expand.rs::expand_each_blocks()` with inline comments explaining:
    - Template iteration and offset management
    - Item template extraction
    - List value retrieval and validation
    - Index substitution and field replacement
    - Expanded content generation
  - UAT passed: All 312 tests pass after adding comments

## 2026-01-25 — T-004 Completed
- **Task**: Add comments to orchestration state machines
- **Status**: ✅ Done
- **Changes**:
  - Enhanced `src/run.rs::run_uat_verification_loop()` with comprehensive state machine comments
  - Documented 9 distinct states in the UAT verification loop: Load, Loop Start, Check Completion, Check Iteration Limit, Pick UAT, Execute Runner, Parse Response, Update State (OPT-OUT/Success paths), and Loop
  - Explained critical design decisions: PRD reloading between iterations, max_iterations enforcement, OPT-OUT handling, and UAT status update strategy
  - Enhanced `src/prd_new.rs::create_prd()` with comprehensive state machine comments
  - Documented 11 distinct states in the multi-round Q/A flow: Initialize, Round 1 Question Generation, Round 1 Answer Collection, Loop State (Round N Start, Question Generation, Ready Check, Additional Questions, Auto-Advance), Synthesis, Persist, and Finalize
  - Explained Q/A loop exit conditions: READY_TO_SYNTHESIZE signal, no additional questions, MAX_QA_ROUNDS limit
  - Clarified two-strategy persist approach: runner-created file vs. response parsing
  - UAT passed: All 312 tests pass after adding comments

## 2026-01-25 — T-005 Completed
- **Task**: Check and fix Rust idiom violations
- **Status**: ✅ Done
- **Changes**:
  - Replaced string concatenation in loops with iterator-based collection and join:
    - `src/main.rs::analyze_project_structure()`: Refactored tool discovery loop to use `filter().map().collect()` instead of iterative `push_str()` (lines 1402-1407)
    - `src/prd_finalize.rs::generate_finalization_report()`: Refactored task summary loop to use `map().collect()` instead of iterative `push_str()` (lines 205-210)
  - Changed `vec![]` to array `[]` in `main.rs` line 1392 per clippy suggestion (useless_vec lint)
  - Verified no risky explicit error handling patterns that should use `?` operator - all existing `match` statements have necessary fallback logic or context-specific error handling
  - UAT passed: All 312 tests pass after refactoring

## 2026-01-25 — T-006 Completed
- **Task**: Identify and fix obvious performance issues
- **Status**: ✅ Done
- **Changes**:
  - Optimized runner output handling in `src/runner/claude.rs` and `src/runner/copilot.rs`:
    - Removed unnecessary `.clone()` calls when combining stdout/stderr (lines 320, 322, 384, 386)
    - Avoided redundant `.to_string()` after `String::from_utf8_lossy()` by keeping `Cow<str>` until final allocation needed
    - Reduces allocations in runner execution path
  - Optimized string building in `src/status.rs`:
    - Replaced `format!() + push_str()` pattern with direct `push_str()` calls in loop over PRD summaries (lines 197-324)
    - Eliminates intermediate format allocations in status report generation
    - Affects 4 loops (active, draft, done, parked PRDs) and statistics section
  - Fixed double string conversion in `src/prd/index.rs` line 154:
    - Changed `.to_string_lossy().to_string()` to `.to_string_lossy().into_owned()`
    - Avoids unnecessary intermediate allocation during PRD filename processing
  - UAT passed: All 312 tests pass after performance optimizations

## 2026-01-25 — PRD Finalized
- **Status**: ✅ Finalized
- **Tasks Completed**: 6 tasks (T-001 through T-006)
- **Outcome**: All tasks completed, acceptance tests passed (312/312 tests)
- **Changelog**: Entry added under [Unreleased] → Added
- **Cleanup**: No temporary files or excessive comments found
- **Summary**:
  - Created shared `qa_workflow.rs` module extracting duplicated Q/A patterns
  - Reduced clone calls in `run.rs` by 37.5% (16→10)
  - Added comprehensive inline comments to complex parsing and state machine logic
  - Replaced string concatenation in loops with iterator-based patterns
  - Optimized runner output handling and string building to reduce allocations
