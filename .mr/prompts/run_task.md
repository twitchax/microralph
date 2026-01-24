# microralph — Run Task Prompt

## Objective

Execute the next incomplete task from a PRD, verify with UATs, update the PRD, and regenerate the index.

## Context

Look at the PRD file at `{{prd_path}}` to understand:
- The project goals and constraints
- The task list and their statuses
- The History section showing previous attempts and outcomes

The suggested next task is `{{next_task_id}}` based on priority, but verify against the PRD.

## Required Actions

1. **Study the README** at the repository root to understand the project's purpose, conventions, and development workflow.
2. **Study the PRD** at `{{prd_path}}` and understand it fully, including goals, constraints, and task history.
3. **Identify the task** `{{next_task_id}}` and its requirements.
4. **Implement the task** as described, making minimal and focused changes.
5. **Follow existing patterns** and conventions in the codebase.
6. **Run `cargo make uat`** to verify all acceptance tests pass.
7. **Update the PRD file** (see below for details).
8. **Regenerate the index** by running: `cargo run -- prd list` (or manually update `.mr/PRDS.md`).
9. **Commit your work** with a descriptive commit message.

## Updating the PRD

You MUST update the PRD file at `{{prd_path}}` as you work:

### Update PRD Status to Active (if currently draft)

When starting work on a PRD, change its status from `draft` to `active`:

```yaml
status: active  # <-- Change from 'draft' to 'active' when starting work
```

### Update Task Status in Frontmatter

Change the task's status from `todo` or `in-progress` to `done` if UAT passes:

```yaml
tasks:
  - id: {{next_task_id}}
    title: "..."
    priority: N
    status: done  # <-- Change from 'todo' to 'done'
```

### Append to History Section

Add a new History entry at the bottom of the PRD file with this format:

```markdown
## YYYY-MM-DD — {{next_task_id}} Completed
- **Task**: [Task title]
- **Status**: ✅ Done (or ❌ Failed if UAT failed)
- **Changes**:
  - Bullet points describing what was changed
  - Include file names and key details
  - Note UAT pass/fail with brief evidence
```

## Constraints

- Do not modify unrelated code.
- Do not change the public API unless the task requires it.
- Prefer fixing root causes over surface workarounds.
- Always update the PRD even if the task fails (document what was attempted).

## When All Tasks Are Done

If completing this task means all tasks in the PRD are now `done`:
1. **Complete and commit this task** as normal (update status, append History, commit).
2. **UAT verification happens automatically**: microralph will detect unverified acceptance tests and enter a dedicated UAT verification loop in subsequent `mr run` invocations.
3. **Do NOT attempt to verify UATs yourself** in this task — the verification loop handles each UAT individually with focused prompts.

Note: Unverified UATs will block PRD finalization. The UAT verification loop allows you to verify tests, create new tests, or opt-out with an explanation for each UAT.

## On Success

If `cargo make uat` passes:
1. Update task status to `done` in the PRD frontmatter.
2. Append a success History entry.
3. Regenerate `.mr/PRDS.md` to reflect new progress.
4. Commit all changes with message: `prd({{prd_id}})feat({{next_task_id}}): [brief description]`

## On Failure

If `cargo make uat` fails:
1. Leave task status as `todo` or `in-progress`.
2. Append a failure History entry describing what was attempted and what failed.
3. Do NOT regenerate the index (status unchanged).
4. Do NOT commit (leave changes for next attempt or manual review).

## Output

Report what happened:
- Whether the task was completed successfully
- What changes were made
- UAT results (pass/fail with brief evidence)
- What was committed (if anything)
