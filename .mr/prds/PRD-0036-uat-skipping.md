---
id: PRD-0036
title: "UAT Skipping and Dynamic Task Addition During Run"
status: active
owner: twitchax
created: 2026-02-21
updated: 2026-02-21
principles:
  - "Skipped is a valid terminal UAT state; history provides the justification"
  - "Agents can autonomously add tasks during run for any reason"
  - "Prefer adding a task over skipping a UAT when the task would unblock verification"
  - "Opt-out flags give users control over autonomous behaviors"
  - "Minimal changes to existing run loop; extend, don't rewrite"
references:
  - name: "Run command implementation"
    url: "src/commands/run.rs"
  - name: "UAT status enum"
    url: "src/prd/types.rs"
  - name: "Finalize logic"
    url: "src/prd/finalize.rs"
  - name: "Prompt definitions"
    url: "src/commands/init.rs"
acceptance_tests:
  - id: uat-001
    name: "UatStatus::Skipped deserializes from YAML and serializes back correctly"
    command: cargo make test
    uat_status: unverified
  - id: uat-002
    name: "Finalize accepts PRDs where all UATs are verified or skipped"
    command: cargo make test
    uat_status: unverified
  - id: uat-003
    name: "Finalize rejects PRDs with unverified UATs (existing behavior preserved)"
    command: cargo make test
    uat_status: unverified
  - id: uat-004
    name: "Run command accepts --disallow-skip-uat and --disallow-add-task flags"
    command: cargo make test
    uat_status: unverified
  - id: uat-005
    name: "Agent can add a new task to a PRD during task execution"
    command: cargo make test
    uat_status: unverified
  - id: uat-006
    name: "Agent can mark a UAT as skipped during UAT verification"
    command: cargo make test
    uat_status: unverified
  - id: uat-007
    name: "--disallow-skip-uat prevents the agent from skipping UATs in prompt"
    command: cargo make test
    uat_status: unverified
  - id: uat-008
    name: "--disallow-add-task prevents the agent from adding tasks in prompt"
    command: cargo make test
    uat_status: unverified
tasks:
  - id: T-001
    title: "Add Skipped variant to UatStatus enum"
    priority: 1
    status: done
    notes: "Add UatStatus::Skipped to src/prd/types.rs with serde rename 'skipped' and Display impl. This is purely additive."
  - id: T-002
    title: "Update finalize to accept skipped UATs"
    priority: 2
    status: done
    notes: "Modify validate_all_uats_verified in src/prd/finalize.rs to treat Skipped as acceptable alongside Verified. Unverified remains blocking."
  - id: T-003
    title: "Add --disallow-skip-uat and --disallow-add-task CLI flags"
    priority: 3
    status: done
    notes: "Add flags to the Run variant in src/main.rs clap args. Thread them through RunConfig into prompt building."
  - id: T-004
    title: "Update run task prompt to allow dynamic task addition"
    priority: 4
    status: todo
    notes: "Update PROMPT_RUN_TASK in src/commands/init.rs to instruct the agent it may add new tasks to the PRD frontmatter. Include guidance to prefer adding a task over skipping a UAT when the task would unblock verification. Conditioned on --disallow-add-task flag via placeholder."
  - id: T-005
    title: "Update UAT verification prompt to allow skipping"
    priority: 5
    status: todo
    notes: "Update PROMPT_RUN_UAT_VERIFY in src/commands/init.rs to add a new option for marking a UAT as skipped (set uat_status to skipped and append history). Conditioned on --disallow-skip-uat flag via placeholder. Also allow adding a task as alternative to skipping."
  - id: T-006
    title: "Thread new flags through prompt building"
    priority: 6
    status: todo
    notes: "Add allow_skip_uat and allow_add_task placeholders to build_prompt and build_uat_verify_prompt in run.rs. Pass negated flag values so prompts can conditionally include/exclude instructions."
  - id: T-007
    title: "Restore .mr/prompts with updated prompt files"
    priority: 7
    status: todo
    notes: "Run mr restore or update .mr/prompts/ to reflect the new prompt templates from init.rs. Per constitution rule 7, the two sources must stay synchronized."
  - id: T-008
    title: "Write unit tests for new UatStatus variant and finalize logic"
    priority: 8
    status: todo
    notes: "Test UatStatus::Skipped serde round-trip, Display impl, and finalize acceptance of skipped UATs. Test that unverified UATs still block finalization."
---

# Summary

Extend the `mr run` workflow with two new capabilities: (1) allow UATs to be marked as `skipped` — a valid terminal state where history justifies the decision — and (2) allow the agent to autonomously add new tasks to a PRD during execution. These features work together: when a UAT cannot be verified, the agent can either skip it with justification or add a task that would unblock it (leaving the UAT `unverified` for retry).

# Problem

Currently, `UatStatus` only has two states: `Unverified` and `Verified`. If a UAT genuinely cannot be automated or verified (e.g., requires manual testing, depends on external services, or is not feasible in CI), the only option is the `OPT-OUT` mechanism which leaves the UAT `unverified` — and finalization blocks on unverified UATs. There is no clean way to signal "this was intentionally skipped" as a valid end state.

Additionally, during `mr run`, the agent cannot add new tasks to the PRD. If the agent discovers during execution that additional work is needed (e.g., a prerequisite was missed, a UAT needs supporting infrastructure, or scope was underestimated), it has no way to capture that work item — it either silently ignores it or fails.

# Goals

1. Add `UatStatus::Skipped` as a valid terminal state that does not block finalization
2. Allow the agent to autonomously add tasks to a PRD during `mr run` task execution
3. When a UAT would need skipping but a new task could unblock it, prefer adding the task and leaving the UAT `unverified` for retry
4. Provide `--disallow-skip-uat` and `--disallow-add-task` flags for users who want stricter control
5. Maintain backward compatibility — existing PRDs and workflows are unaffected

# Technical Approach

## UatStatus::Skipped

Add a `Skipped` variant to the `UatStatus` enum in `src/prd/types.rs`. It serializes as `"skipped"` via serde. The `validate_all_uats_verified` function in `src/prd/finalize.rs` is updated to accept both `Verified` and `Skipped` as terminal states.

## Dynamic Task Addition

The run task prompt (`PROMPT_RUN_TASK` in `src/commands/init.rs`) is extended with instructions that the agent may add new tasks to the PRD's `tasks` array in the YAML frontmatter. The agent writes directly to the PRD file (same mechanism it already uses to update task status). New tasks get the next available `T-XXX` ID and `status: todo`.

## UAT Skipping

The UAT verification prompt (`PROMPT_RUN_UAT_VERIFY` in `src/commands/init.rs`) is extended with a new option: mark the UAT as `skipped` by setting `uat_status: skipped` in the frontmatter and appending a history entry explaining why. The existing `OPT-OUT` mechanism remains for cases where the agent wants to defer without committing to a final state.

## Flag Threading

```
mr run
  ├─ --disallow-skip-uat   → sets allow_skip_uat = false
  ├─ --disallow-add-task    → sets allow_add_task = false
  │
  ├─ build_prompt()
  │     └─ {{allow_add_task}} placeholder → conditional section in prompt
  │
  └─ build_uat_verify_prompt()
        ├─ {{allow_skip_uat}} placeholder → conditional section in prompt
        └─ {{allow_add_task}} placeholder → conditional section in prompt
```

The flags default to allowing both behaviors. When a `--disallow-*` flag is set, the corresponding prompt section is omitted, so the agent never receives instructions to perform that action.

# Assumptions

- The agent already modifies PRD frontmatter directly during `mr run` (task status updates), so adding tasks follows the same pattern
- The run loop already reloads the PRD on each iteration, so newly added tasks will be picked up automatically
- Prompt placeholder expansion supports conditional sections (or can be adapted with simple presence/absence of content)

# Constraints

- Must not change existing `OPT-OUT` behavior — it remains a separate mechanism
- Must not change the meaning of `Verified` or `Unverified` states
- Skipped UATs must have a history entry explaining the reason (enforced by prompt, not code)
- Per constitution rule 7, prompt constants in `init.rs` and files in `.mr/prompts/` must stay synchronized

# References to Code

- `src/prd/types.rs` — `UatStatus` enum (lines ~88-107)
- `src/prd/finalize.rs` — `validate_all_uats_verified` function
- `src/commands/run.rs` — `build_prompt`, `build_uat_verify_prompt`, `RunConfig`, CLI args
- `src/commands/init.rs` — `PROMPT_RUN_TASK`, `PROMPT_RUN_UAT_VERIFY` constants
- `src/main.rs` — `Run` clap variant (lines ~141-165)

# Non-Goals (MVP)

- UI for interactively skipping UATs (agent decides autonomously)
- Retroactively marking existing OPT-OUT UATs as skipped
- Task removal or reordering during run
- Validation that added tasks have unique IDs (trusted to the agent via prompt)

# History

## 2026-02-21 — T-001 Completed
- **Task**: Add Skipped variant to UatStatus enum
- **Status**: ✅ Done
- **Changes**:
  - Added `UatStatus::Skipped` variant to `src/prd/types.rs` with serde rename `"skipped"`
  - Added `Display` impl for `Skipped` → outputs `"skipped"`
  - Doc comment: "UAT was intentionally skipped with justification recorded in history"
  - UAT passed: 552 tests run, 552 passed, 0 skipped
- **Constitution Compliance**: No violations. Change is purely additive and minimal.

## 2026-02-21 — T-002 Completed
- **Task**: Update finalize to accept skipped UATs
- **Status**: ✅ Done
- **Changes**:
  - Updated doc comment on `validate_all_uats_verified` in `src/prd/finalize.rs` to explicitly state that both `Verified` and `Skipped` are acceptable terminal states
  - Updated call-site comment to reflect terminal status semantics
  - Added 3 new tests: `test_validate_all_uats_verified_with_all_skipped`, `test_validate_uats_mixed_verified_and_skipped`, `test_validate_uats_skipped_with_unverified_fails`
  - UAT passed: 555 tests run, 555 passed, 0 skipped (3 new tests added)
- **Opportunistic UAT verification**: uat-002 ("Finalize accepts PRDs where all UATs are verified or skipped") and uat-003 ("Finalize rejects PRDs with unverified UATs") are covered by the new tests, but left as `unverified` per instructions to defer to the UAT verification loop.
- **Constitution Compliance**: No violations. Minimal changes, consistent with existing patterns.

## 2026-02-21 — T-003 Completed
- **Task**: Add --disallow-skip-uat and --disallow-add-task CLI flags
- **Status**: ✅ Done
- **Changes**:
  - Added `disallow_skip_uat` and `disallow_add_task` fields to the `Run` CLI variant in `src/main.rs`
  - Created `CmdRunOpts` struct to group `cmd_run` parameters (avoids clippy `fn_params_excessive_bools`)
  - Added `allow_add_task` field to `RunConfig` in `src/commands/run.rs`
  - Added `allow_skip_uat` and `allow_add_task` fields to `UatVerificationConfig` in `src/commands/run.rs`
  - Threaded `allow_add_task` into `build_prompt()` and `allow_skip_uat`/`allow_add_task` into `build_uat_verify_prompt()` as placeholder context values
  - Added 3 new CLI arg parsing tests: `test_args_parse_run_with_disallow_skip_uat`, `test_args_parse_run_with_disallow_add_task`, `test_args_parse_run_default_disallow_flags_off`
  - Updated all test `RunConfig` and `UatVerificationConfig` instantiations with new fields
  - UAT passed: 558 tests run, 558 passed, 0 skipped
- **Constitution Compliance**: Used `#[allow(clippy::struct_excessive_bools)]` on `CmdRunOpts` and `#[allow(clippy::too_many_lines)]` on `cmd_run` — necessary because the function inherently manages many independent boolean CLI flags and orchestrates the full run loop. No other violations.
