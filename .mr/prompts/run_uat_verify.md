# microralph — UAT Verification Prompt

## Objective

Verify a single unverified acceptance test (UAT) from a PRD by creating a test, running an existing test, or documenting why verification isn't feasible.

## Context

You are verifying acceptance test `{{uat_id}}` from PRD `{{prd_id}}`.

**PRD Path**: `{{prd_path}}`

**Acceptance Test Details**:
- **ID**: `{{uat_id}}`
- **Name**: `{{uat_name}}`
- **Command**: `{{uat_command}}`
- **Current Status**: unverified

All tasks in this PRD are complete. You are now in the UAT verification phase to ensure acceptance criteria are covered by real tests.

## Required Actions

Choose ONE of the following verification approaches:

### Option A: Verify Existing Test

If a test already exists that covers this acceptance criterion:
1. Identify the test (file path and test name).
2. Run the test to confirm it passes: `{{uat_command}}`
3. If it passes, update the PRD to mark `uat_status: verified` for `{{uat_id}}`.
4. Append a History entry documenting the verification.

### Option B: Create New Test

If no test exists but one can feasibly be created:
1. Create a minimal test that covers the acceptance criterion.
2. Run `cargo make uat` to verify the test passes.
3. Update the PRD to mark `uat_status: verified` for `{{uat_id}}`.
4. Append a History entry documenting the new test.

### Option C: Opt-Out with Explanation

If verification is not feasible (e.g., requires manual testing, external dependencies, or is covered implicitly by other tests), you may opt out:
1. Do NOT update `uat_status` (leave as `unverified`).
2. Append a History entry explaining why verification isn't feasible.
3. Respond with `OPT-OUT:` followed by your explanation on a single line.
{{#if allow_skip_uat}}

### Option D: Mark as Skipped

If this UAT genuinely cannot be automated or verified (e.g., requires manual testing, depends on external services not available in CI, or is not feasible to test programmatically), you may permanently skip it:
1. Update `uat_status: skipped` for `{{uat_id}}` in the PRD frontmatter.
2. Append a History entry with a clear justification for why the UAT was skipped.
3. Respond with `SKIPPED:` followed by your justification on a single line.

**Important**: Skipping is a **terminal state** — the UAT will not be retried. Only skip when verification is truly infeasible. If a new task could unblock verification, prefer Option E (if available) over skipping.
{{/if}}
{{#if allow_add_task}}

### Option E: Add a Task to Unblock Verification

If this UAT cannot currently be verified but a new task could unblock it (e.g., missing test infrastructure, prerequisite not implemented, or additional setup needed):
1. Add a new task to the PRD's `tasks` array in the YAML frontmatter.
2. Assign the next available `T-XXX` ID, set `status: todo`, and include descriptive `title` and `notes`.
3. Leave `uat_status` as `unverified` — the UAT will be retried after the new task is completed.
4. Append a History entry explaining what task was added and why.
5. Respond with `OPT-OUT: Added task T-XXX to unblock this UAT` so the run loop knows to proceed.

**Prefer this over skipping**: When a UAT could be verified with additional work, adding a task is better than permanently skipping the UAT.

**Example:**
```yaml
tasks:
  # ... existing tasks ...
  - id: T-009
    title: "Add test helper for external service mock"
    priority: 9
    status: todo
    notes: "Added during uat-003 verification: needed to mock external API for UAT."
```
{{/if}}

## Updating the PRD

### Update UAT Status in Frontmatter

If verification succeeds (Option A or B), update the acceptance test entry:

```yaml
acceptance_tests:
  - id: {{uat_id}}
    name: "{{uat_name}}"
    command: {{uat_command}}
    uat_status: verified  # <-- Change from 'unverified' to 'verified'
```
{{#if allow_skip_uat}}

If skipping (Option D), update to:

```yaml
acceptance_tests:
  - id: {{uat_id}}
    name: "{{uat_name}}"
    command: {{uat_command}}
    uat_status: skipped  # <-- Change from 'unverified' to 'skipped'
```
{{/if}}

### Append to History Section

Add a History entry documenting your verification attempt:

```markdown
## YYYY-MM-DD — {{uat_id}} Verification
- **UAT**: {{uat_name}}
- **Status**: ✅ Verified (or ⏭️ Opted-out{{#if allow_skip_uat}} or ⏭️ Skipped{{/if}})
- **Method**: [Existing test / New test / Opt-out{{#if allow_skip_uat}} / Skipped{{/if}}]
- **Details**:
  - [Test file and name if applicable]
  - [Explanation if opted out or skipped]
```

## Constraints

- Focus on this single UAT (`{{uat_id}}`). Do not verify other UATs in this invocation.
- Keep test code minimal — just enough to cover the acceptance criterion.
- Always update the PRD even if opting out or skipping (document your reasoning).

## On Success

If verification succeeds:
1. Update `uat_status: verified` in the PRD frontmatter.
2. Append a verification History entry.
3. Regenerate `.mr/PRDS.md` by running: `cargo run -- list`
4. Commit with message: `prd({{prd_id}})uat({{uat_id}}): [brief description]`

## On Opt-Out

If opting out:
1. Leave `uat_status: unverified` unchanged.
2. Append an opt-out History entry with clear explanation.
3. Respond with `OPT-OUT: [your explanation]` so the run loop knows to proceed.
4. Do NOT commit (opt-outs don't change UAT status).
{{#if allow_skip_uat}}

## On Skip

If skipping:
1. Update `uat_status: skipped` in the PRD frontmatter.
2. Append a skip History entry with clear justification.
3. Respond with `SKIPPED: [your justification]` so the run loop knows to proceed.
4. Commit with message: `prd({{prd_id}})uat({{uat_id}}): skipped — [brief justification]`
{{/if}}

## Output

Report what happened:
- Whether verification succeeded, opted out, or was skipped
- What approach was used (existing test, new test, opt-out, skip, or task addition)
- Test details or opt-out/skip explanation
- What was committed (if anything)
