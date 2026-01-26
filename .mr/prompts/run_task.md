# microralph — Run Task Prompt

## Objective

Execute the next incomplete task from a PRD, verify with UATs, update the PRD, and regenerate the index.

## Context

Look at the PRD file at `{{prd_path}}` to understand:
- The project goals and constraints
- The task list and their statuses
- The History section showing previous attempts and outcomes

The suggested next task is `{{next_task_id}}` based on priority, but verify against the PRD.

{{#if constitution}}
## Constitution

This project has a constitution that defines governance rules and constraints. Your implementation should respect these rules:

```
{{constitution}}
```

**Important**: If your implementation violates any constitutional rules, you MUST mention the violation in the History entry with reasoning about why it was necessary or unavoidable. Constitution violations are logged for transparency but do not block task execution.
{{/if}}

## Required Actions

1. **Study the README** at the repository root to understand the project's purpose, conventions, and development workflow.
2. **Study the PRD** at `{{prd_path}}` and understand it fully, including goals, constraints, and task history.
3. **Identify the task** `{{next_task_id}}` and its requirements.
4. **Implement the task** as described.
5. **Run `cargo make uat`** to verify all acceptance tests pass.
6. **Update AGENTS.md** if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.
7. **Update the PRD file** (see below for details).
8. **Regenerate the index** by running: `cargo run -- list` (or manually update `.mr/PRDS.md`).
{{#if commit}}
9. **Commit your work** with a descriptive commit message.
{{else}}
9. **Do NOT commit your work** — leave changes staged or unstaged for manual review.
{{/if}}

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

**YAML Quoting Reminder**: When editing frontmatter, ensure strings containing colons (`:`) or hashes (`#`) are quoted. Example: `title: "Feature: Add new command"`

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
{{#if constitution}}
- **Constitution Compliance**: If any constitutional rules were violated, note them here with reasoning. If fully compliant, you may omit this section or note "No violations."
{{/if}}
```

## Opportunistic UAT Verification (Per-Task)

After completing a task, check if any UATs (acceptance tests) can be verified **now** based on the work just completed:

1. **Review the PRD's `acceptance_tests`**: Identify UATs that are currently `unverified`.
2. **Determine feasibility**: A UAT can be verified NOW if:
   - The functionality it tests was implemented by this task or previous completed tasks
   - It does NOT depend on incomplete tasks (check the task list)
   - A test can be created or an existing test can be run
3. **If a UAT is feasible**: Create or run the test, and if it passes, update `uat_status: verified` in the frontmatter.
4. **If a UAT requires incomplete tasks**: Skip it — the full UAT verification loop will handle it later.
5. **Document in History**: Note which UATs (if any) were opportunistically verified.

**Example reasoning**:
- UAT "CLI accepts --verbose flag" → If T-003 (add verbose flag) is done, verify it now.
- UAT "Build pipeline passes" → Requires all tasks, skip until finalization.
- UAT "Color output works" → If T-001 (add color module) is done, verify it now.

This reduces work during the final UAT verification loop and catches issues earlier.

## Constraints

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
{{#if commit}}
4. Commit all changes with message: `prd({{prd_id}})feat({{next_task_id}}): [brief description]`
{{else}}
4. Do NOT commit — leave changes for manual review.
{{/if}}

## On Failure

If `cargo make uat` fails:
1. Leave task status as `todo` or `in-progress`.
2. Append a failure History entry describing what was attempted and what failed.
3. Do NOT regenerate the index (status unchanged).
{{#if commit}}
4. Do NOT commit (leave changes for next attempt or manual review).
{{else}}
4. Leave changes uncommitted for next attempt or manual review.
{{/if}}

## Output

Report what happened:
- Whether the task was completed successfully
- What changes were made
- UAT results (pass/fail with brief evidence)
- What was committed (if anything)
