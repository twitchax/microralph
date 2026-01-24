---
id: PRD-0014
title: Cleanup Pass - Refactoring and Documentation
status: draft
owner: microralph
created: 2026-01-24
updated: 2026-01-24
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
    uat_status: unverified
tasks:
  - id: T-001
    title: Extract duplicated Q/A workflow patterns to shared modules
    priority: 1
    status: todo
    notes: Focus on parse_questions, extract_prd_content, and prompt building logic across prd_new.rs, prd_edit.rs, and run.rs. Only extract if truly duplicated, not semi-duplicated with context-specific variations.
  - id: T-002
    title: Reduce unnecessary clones in run.rs
    priority: 2
    status: todo
    notes: Target the ~154 clone calls. Focus on cases where owned data isn't needed. Balance risk vs. benefit.
  - id: T-003
    title: Add comments to complex parsing logic
    priority: 3
    status: todo
    notes: Target prd/parser.rs and prompt/expand.rs. Explain non-obvious logic.
  - id: T-004
    title: Add comments to orchestration state machines
    priority: 3
    status: todo
    notes: Document run.rs UAT loop and prd_new.rs multi-round Q/A flows.
  - id: T-005
    title: Check and fix Rust idiom violations
    priority: 4
    status: todo
    notes: Prefer `?` operator, avoid string concatenation in loops. Don't force changes if too risky.
  - id: T-006
    title: Identify and fix obvious performance issues
    priority: 4
    status: todo
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