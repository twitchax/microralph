---
id: PRD-0005
title: Verify UATs at End of Run Loop
status: draft
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
  uat_status: unverified
- id: uat-002
  name: Loop addresses unverified UATs (create tests, run tests, or document)
  command: cargo make uat
  uat_status: unverified
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
  status: todo
  notes: In src/run.rs, after all tasks are done, check if any UATs are unverified. If so, transition to UAT verification phase instead of exiting.
- id: T-002
  title: Create UAT verification prompt template
  priority: 1
  status: todo
  notes: Create .mr/prompts/run_uat_verify.md with instructions for verifying a single UAT (create test, run test, or document why not feasible).
- id: T-003
  title: Implement UAT verification loop in run.rs
  priority: 1
  status: todo
  notes: Add run_uat_verification_loop() function that iterates over unverified UATs, invokes runner with verification prompt, respects max_iterations limit.
- id: T-004
  title: Add model opt-out mechanism with History entry
  priority: 2
  status: todo
  notes: Allow runner to respond with OPT-OUT or similar. Parse response, append explanation to History, and exit loop gracefully.
- id: T-005
  title: Update mr prd finalize to block on unverified UATs
  priority: 2
  status: todo
  notes: In src/prd_finalize.rs, add validate_all_uats_verified() check alongside task validation. Return error if any UAT has uat_status unverified.
- id: T-006
  title: Update run_task.md prompt to reference UAT verification phase
  priority: 2
  status: todo
  notes: Update the When All Tasks Are Done section to indicate UAT verification loop will handle unverified UATs, not the single-task runner.
- id: T-007
  title: Add UAT status update logic to write verified status back to PRD
  priority: 2
  status: todo
  notes: After runner successfully verifies a UAT, update the PRD frontmatter to set uat_status verified for that UAT.
- id: T-008
  title: Add integration test for UAT verification loop
  priority: 3
  status: todo
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

