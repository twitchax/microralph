# microralph — Refactor Prompt

## Objective

Identify one impactful code improvement, apply it, verify UATs pass, and commit (if allowed).

## Context

You are performing iteration {{iteration}} of {{max_iterations}} in a refactor loop.

{{#if context}}
### Focus Hint

The user has requested you focus on: **{{context}}**

This takes priority over general constitution-based discovery.
{{/if}}

{{#if path}}
### Scope Constraint

Limit your changes to files within: `{{path}}`
{{/if}}

{{#if constitution}}
### Constitution

The project's constitution defines behavioral rules and constraints:

```markdown
{{constitution}}
```

Use these rules to guide your refactor selection when no specific focus hint is provided.
{{/if}}

## Task

1. **Analyze** the codebase for one impactful refactor opportunity
2. **Apply** the change with minimal modifications
3. **Verify** by running `cargo make uat`
4. **Commit** with message format: `refactor: [brief description]`

{{#if preview}}
### Preview Mode

This is a **preview**. Do NOT apply changes.

Instead, output your suggested refactor in this format:

```
REFACTOR SUGGESTION:
File: [path/to/file.rs]
Lines: [start-end]
Description: [What would be changed and why]
Impact: [Expected benefit]
```

After outputting the suggestion, respond with `PREVIEW-COMPLETE` on a new line.
{{/if}}

{{#if no_commit}}
### No-Commit Mode

Do NOT commit changes. Leave them staged or unstaged for manual review.
{{/if}}

## Early Termination

If you find no impactful refactors remaining (codebase already adheres well to principles), respond with exactly:

```
NO-MORE-REFACTORS
```

This signals early termination of the refactor loop.

## Constraints

- Make **one** focused change per iteration
- Keep changes minimal and surgical
- Do not fix unrelated issues
- Follow existing code style and conventions
- Run UATs to verify changes don't break anything

## Output

After completing (or in preview mode, after suggesting), summarize what you did.
