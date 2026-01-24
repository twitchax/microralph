---
id: PRD-0005
title: Verify UATs at End of Run Loop
status: active
owner: Aaron Roney
created: 2026-01-24
updated: 2026-01-24

principles:
- UAT verification is a loop, not a single pass.
- The model can opt-out of verification with an explanation.
- Unverified UATs block PRD completion.
- Reuse existing max_iterations config for loop limits.

acceptance_tests:
- id: uat-001
  name: UAT verification loop triggers after all tasks done
  command: cargo make uat
  uat_status: verified
- id: uat-002
  name: Loop addresses unverified UATs (create tests, run tests, or document)
  command: cargo make uat
  uat_status: verified
- id: uat-003
  name: Model can opt-out with explanation appended to History
  command: cargo make uat
  uat_status: unverified
- id: uat-004
  name: Loop respects max_iterations config
  command: cargo make uat
  uat_status: unverified
- id: uat-005
  name: Unverified UATs block PRD finalization
  command: cargo make uat finalize_unverified_blocks
  uat_status: unverified
- id: uat-006
  name: Verified UATs updated in PRD frontmatter
  command: cargo make uat
  uat_status: unverified
- id: uat-007
  name: UAT verification results appended to History
  command: cargo make uat
  uat_status: unverified

tasks:
- id: T-001
  title: Add unverified UAT check to run loop (detect when tasks done but UATs unverified)
  priority: 1
  status: done
  notes: In src/run.rs, after all tasks are done, check if any UATs are unverified. If so, transition to UAT verification phase instead of exiting.
- id: T-002
  title: Create UAT verification prompt template
  priority: 1
  status: done
  notes: Create .mr/prompts/run_uat_verify.md with instructions for verifying a single UAT (create test, run test, or document why not feasible).
- id: T-003
  title: Implement UAT verification loop in run.rs
  priority: 1
  status: done
  notes: Add run_uat_verification_loop() function that iterates over unverified UATs, invokes runner with verification prompt, respects max_iterations limit.
- id: T-004
  title: Add model opt-out mechanism with History entry
  priority: 2
  status: done
  notes: Allow runner to respond with OPT-OUT or similar. Parse response, append explanation to History, and exit loop gracefully.
- id: T-005
  title: Update mr prd finalize to block on unverified UATs
  priority: 2
  status: done
  notes: In src/prd_finalize.rs, add validate_all_uats_verified() check alongside task validation. Return error if any UAT has uat_status unverified.
- id: T-006
  title: Update run_task.md prompt to reference UAT verification phase
  priority: 2
  status: done
  notes: Update the When All Tasks Are Done section to indicate UAT verification loop will handle unverified UATs, not the single-task runner.
- id: T-007
  title: Add UAT status update logic to write verified status back to PRD
  priority: 2
  status: done
  notes: After runner successfully verifies a UAT, update the PRD frontmatter to set uat_status verified for that UAT.
- id: T-008
  title: Add integration test for UAT verification loop
  priority: 3
  status: done
  notes: Create test in src/run.rs that simulates all tasks done with unverified UATs, verifies loop executes, and respects max_iterations.
---

# PRD-0005: Verify UATs at End of Run Loop

## Summary

Add a UAT verification loop that runs after all tasks are complete. This loop iterates over unverified acceptance tests, allowing the model to create missing tests, run existing tests, or document why verification isn't feasible. The model can opt-out of remaining verifications with an explanation. Unverified UATs block PRD finalization.

## Problem

Currently, the run task prompt instructs the runner to verify UATs when completing the final task, but this is a single-pass operation. If UATs remain unverified:
1. There's no structured loop to address them
2. The model might skip verification due to context limits
3. PRDs can be marked "done" with unverified UATs
4. No clear mechanism for the model to explain why verification isn't feasible

This leads to PRDs being finalized without proper acceptance test coverage.

## Goals

1. **UAT verification loop**: After all tasks are done, enter a dedicated loop to address unverified UATs.
2. **Flexible verification actions**: Model can create tests, run existing tests, or document why verification isn't feasible.
3. **Model opt-out with explanation**: Allow the model to end the loop early with an explanation appended to History.
4. **Respect iteration limits**: Use existing `loop.max_iterations` config to bound the verification loop.
5. **Block PRD finalization**: Unverified UATs prevent `mr prd finalize` from succeeding.
6. **Update UAT status**: Write `uat_status: verified` back to PRD frontmatter when verification succeeds.

## Non-Goals

- Auto-generating test code without model involvement.
- Parallel UAT verification (loop is sequential).
- Separate max_iterations config for UAT verification (reuse existing config).

## Relevant References

- See `src/run.rs` for current run loop implementation and task execution flow.
- See `src/prd/types.rs:88-107` for `UatStatus` enum and `AcceptanceTest` struct.
- See `.mr/prompts/run_task.md:65-74` for current UAT verification instructions.
- See `src/prd_finalize.rs:102-116` for task validation pattern to follow for UAT validation.
- See PRD-0004 ## Design Notes for finalization workflow patterns.

## History

## 2026-01-24 — T-001 Completed
- **Task**: Add unverified UAT check to run loop (detect when tasks done but UATs unverified)
- **Status**: ✅ Done
- **Changes**:
  - Added `acceptance_tests()`, `all_tasks_done()`, `has_unverified_uats()`, and `unverified_uats()` methods to `Prd` struct in `src/prd/types.rs`
  - Changed `RunResult` from a struct to an enum with three variants: `TaskExecuted`, `NeedsUatVerification`, and `PrdComplete`
  - Updated `run_task()` in `src/run.rs` to detect when all tasks are done and check for unverified UATs, returning appropriate result variant
  - Updated `main.rs` run loop to handle new `RunResult` variants with appropriate user messages
  - Added 3 new unit tests: `test_run_task_all_done_with_unverified_uats`, `test_run_task_all_done_and_verified`, `test_run_task_all_done_no_uats`
  - UAT passed: 230 tests, all passed

## 2026-01-24 — T-002 Completed
- **Task**: Create UAT verification prompt template
- **Status**: ✅ Done
- **Changes**:
  - Created `.mr/prompts/run_uat_verify.md` with comprehensive instructions for verifying a single UAT
  - Added `PromptKind::RunUatVerify` variant to `src/prompt/types.rs`
  - Added `PROMPT_RUN_UAT_VERIFY` constant to `src/init.rs` with embedded prompt content
  - Wired up the new prompt in `init()` function and `get_default_prompt()` in `src/prompt/loader.rs`
  - Updated test counts in `test_prompt_kind_all()`, `test_init_creates_structure()`, `test_init_is_idempotent()`, and `test_prompt_loader_missing_prompts()`
  - Prompt includes placeholders: `{{uat_id}}`, `{{uat_name}}`, `{{uat_command}}`, `{{prd_id}}`, `{{prd_path}}`
  - Supports three verification approaches: Option A (verify existing test), Option B (create new test), Option C (opt-out with explanation)
  - UAT passed: 230 tests, all passed

## 2026-01-24 — T-003 Completed
- **Task**: Implement UAT verification loop in run.rs
- **Status**: ✅ Done
- **Changes**:
  - Added `run_uat_verification_loop()` function in `src/run.rs` that iterates over unverified UATs
  - Added `UatVerificationConfig` struct to configure the verification loop (root, prd_id, stream, max_iterations)
  - Added `UatVerificationLoopResult` struct to report results (verified_count, opted_out_count, iterations, hit_max_iterations, remaining_unverified)
  - Added `build_uat_verify_prompt()` helper to construct verification prompts using the RunUatVerify template
  - Added `parse_opt_out()` function to detect "OPT-OUT:" responses from the runner
  - Loop respects `max_iterations` from PRD's `loop_config` or defaults to 10
  - Wired up verification loop in `main.rs` when `RunResult::NeedsUatVerification` is returned
  - Added 4 new unit tests: `test_parse_opt_out`, `test_build_uat_verify_prompt`, `test_uat_verification_loop_all_verified_by_runner`, `test_uat_verification_loop_opt_out`, `test_uat_verification_loop_max_iterations`
  - UAT passed: 238 tests, all passed

## 2026-01-24 — T-004 Completed
- **Task**: Add model opt-out mechanism with History entry
- **Status**: ✅ Done
- **Changes**:
  - Added `append_opt_out_history()` function in `src/run.rs` to append opt-out History entries to the PRD
  - Modified the opt-out detection in `run_uat_verification_loop()` to call `append_opt_out_history()` when an OPT-OUT response is detected
  - Added imports for `std::fs` and `std::io::Write` to support file appending
  - Added unit test `test_append_opt_out_history` to verify History entry format and content
  - Updated `test_uat_verification_loop_opt_out` to verify History entry is appended during loop opt-out
  - History entry format: `## YYYY-MM-DD — {uat_id} Opt-Out` with UAT name, status, and reason
  - UAT passed: 239 tests, all passed

## 2026-01-24 — T-005 Completed
- **Task**: Update mr prd finalize to block on unverified UATs
- **Status**: ✅ Done
- **Changes**:
  - Added `UnverifiedUats` variant to `FinalizeError` enum in `src/prd_finalize.rs` with `unverified_count` and `uat_details` fields
  - Added `get_unverified_uats()` helper function to retrieve unverified UATs from a PRD
  - Added `validate_all_uats_verified()` function to validate all UATs are verified before finalization
  - Updated `finalize_prd()` to call `validate_all_uats_verified()` after task validation, blocking finalization if any UATs are unverified
  - Updated doc comment to document `FinalizeError::UnverifiedUats` error case
  - Added 5 unit tests: `test_validate_all_uats_verified_with_all_verified`, `test_validate_all_uats_verified_with_unverified`, `test_validate_all_uats_verified_with_no_uats`, `test_validate_multiple_unverified_uats`, `test_validate_all_unverified_uats`
  - Added `make_test_prd_with_uats()` and `make_uat()` helper functions for testing
  - UAT passed: 244 tests, all passed

## 2026-01-24 — T-006 Completed
- **Task**: Update run_task.md prompt to reference UAT verification phase
- **Status**: ✅ Done
- **Changes**:
  - Updated "When All Tasks Are Done" section in `.mr/prompts/run_task.md` to indicate UAT verification happens automatically via the dedicated loop
  - Removed instructions for single-task runner to verify UATs manually
  - Updated "On Success" section to remove reference to verifying acceptance tests
  - Updated embedded `PROMPT_RUN_TASK` constant in `src/init.rs` to match the prompt file changes
  - Added clarifying note that unverified UATs will block PRD finalization
  - UAT passed: 244 tests, all passed

## 2026-01-24 — T-007 Completed
- **Task**: Add UAT status update logic to write verified status back to PRD
- **Status**: ✅ Done
- **Changes**:
  - Added `update_uat_status()` function in `src/run.rs` that reads PRD, updates a specific UAT's status to `verified`, and writes back
  - Modified `run_uat_verification_loop()` to call `update_uat_status()` when runner succeeds but didn't update the PRD itself
  - Updated existing test `test_uat_verification_loop_max_iterations` to reflect new behavior (runner success → UATs get verified)
  - Added 2 new unit tests: `test_update_uat_status` and `test_update_uat_status_not_found`
  - UAT passed: 246 tests, all passed

## 2026-01-24 — T-008 Completed
- **Task**: Add integration test for UAT verification loop
- **Status**: ✅ Done
- **Changes**:
  - Added `test_uat_verification_integration_flow` integration test in `src/run.rs`
  - Test covers full flow: `run_task()` → `NeedsUatVerification` → `run_uat_verification_loop()`
  - Simulates PRD with all tasks done and 3 unverified UATs
  - Verifies loop respects max_iterations limit (set to 2)
  - Verifies first UAT gets verified, second UAT opts out with History entry
  - Asserts PRD frontmatter is correctly updated
  - UAT passed: 247 tests, all passed
## 2026-01-24 — uat-001 Verification
- **UAT**: UAT verification loop triggers after all tasks done
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Test file: `src/run.rs`
  - Tests covering this UAT:
    - `test_run_task_all_done_with_unverified_uats` (line 1026): Verifies that when all tasks are done but UATs are unverified, `run_task()` returns `RunResult::NeedsUatVerification`
    - `test_uat_verification_integration_flow` (line 1592): Integration test covering the full flow from task completion through the UAT verification loop
  - Test command: `cargo make uat`
  - Result: All 247 tests passed
## 2026-01-24 — uat-002 Verification
- **UAT**: Loop addresses unverified UATs (create tests, run tests, or document)
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Test file: `src/run.rs`
  - Test name: `test_uat_verification_integration_flow` (line 1592)
  - This integration test comprehensively covers the loop's ability to:
    1. Verify a UAT when runner succeeds (uat-001 verified)
    2. Handle opt-out with explanation appended to History (uat-002 opts out)
    3. Respect max_iterations config (loop stops after 2 iterations)
  - Additional supporting tests: `test_uat_verification_loop_all_verified_by_runner`, `test_uat_verification_loop_opt_out`, `test_uat_verification_loop_max_iterations`
  - Test command: `cargo make uat`
  - Result: All 247 tests passed
