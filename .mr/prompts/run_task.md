# Micro Ralph — Run Task Prompt

## Objective

Execute a single task from a PRD.

## Context

You are executing task `{{task_id}}` from `{{prd_id}}`.

## PRD Summary

{{prd_summary}}

## Task Details

- **ID**: {{task_id}}
- **Title**: {{task_title}}
- **Priority**: {{task_priority}}
- **Notes**: {{task_notes}}

## Previous History

{{#each recent_history}}
### {{timestamp}}
{{summary}}
{{/each}}

## Required Actions

1. Implement the task as described.
2. Make minimal, focused changes.
3. Follow existing code patterns and conventions.
4. Run `cargo make uat` to verify changes.

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
