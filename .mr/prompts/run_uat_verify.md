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

### Append to History Section

Add a History entry documenting your verification attempt:

```markdown
## YYYY-MM-DD — {{uat_id}} Verification
- **UAT**: {{uat_name}}
- **Status**: ✅ Verified (or ⏭️ Opted-out)
- **Method**: [Existing test / New test / Opt-out]
- **Details**:
  - [Test file and name if applicable]
  - [Explanation if opted out]
```

## Constraints

- Focus on this single UAT (`{{uat_id}}`). Do not verify other UATs in this invocation.
- Keep test code minimal — just enough to cover the acceptance criterion.
- Always update the PRD even if opting out (document your reasoning).

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

## Output

Report what happened:
- Whether verification succeeded or opted out
- What approach was used (existing test, new test, or opt-out)
- Test details or opt-out explanation
- What was committed (if anything)
