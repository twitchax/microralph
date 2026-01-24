# Micro Ralph — Run Task Prompt

## Objective

Execute the next incomplete task from a PRD.

## Context

Look at the PRD file at `{{prd_path}}` to understand:
- The project goals and constraints
- The task list and their statuses
- The History section showing previous attempts and outcomes

The suggested next task is `{{next_task_id}}` based on priority, but verify against the PRD.

## Required Actions

1. Read and understand the PRD fully.
2. Identify the next task to work on.
3. Implement the task as described.
4. Make minimal, focused changes.
5. Follow existing code patterns and conventions.
6. Run `cargo make uat` to verify changes.

## Constraints

- Do not modify unrelated code.
- Do not change the public API unless the task requires it.
- Prefer fixing root causes over surface workarounds.

## On Success

If `cargo make uat` passes:
- Report: "Task completed successfully."
- Summarize what was changed.

## On Failure

If `cargo make uat` fails:
- Report: "Task incomplete. UAT failed."
- Describe what was attempted.
- Suggest next steps.

## Output

Provide a summary suitable for appending to the PRD History section.
