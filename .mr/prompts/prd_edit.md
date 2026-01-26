# microralph — PRD Edit Prompt

## Objective

Make targeted edits to an existing PRD based on user request.

## Context

The user wants to modify the PRD at `{{prd_path}}`.

## User Request

{{user_request}}

## Current PRD Content

```markdown
{{prd_content}}
```

## Q/A History (if any)

{{#each qa_history}}
**Q**: {{question}}
**A**: {{answer}}

{{/each}}

## Required Actions

1. **Understand the request**: Read the user's request carefully.
2. **Analyze the PRD**: Review the current PRD content.
3. **Apply changes**: Make the requested modifications.
4. **Preserve structure**: Keep the YAML frontmatter valid and the Markdown body properly formatted.

## Constraints

- Do not change the PRD ID.
- Do not remove existing History entries.
- Keep the overall structure intact (frontmatter, Summary, Problem, Goals, Non-Goals, History sections).
- If adding tasks, assign appropriate IDs (T-NNN) and priorities.
- If adding acceptance tests, assign appropriate IDs (uat-NNN).
- **YAML Quoting**: Strings containing colons (`:`) or hashes (`#`) MUST be quoted to avoid parse errors. Example: `title: "Fix: Bug in parser"`

## Output Format

If you need more information, respond with a numbered list of questions (1-3 max):
```
1. Question one?
2. Question two?
```

If you have enough information, respond with exactly `READY_TO_APPLY` on its own line, followed by the complete updated PRD content in a markdown code block:
```
READY_TO_APPLY

```markdown
---
id: PRD-XXXX
...
---
# Summary
...
```
```

Ensure the output is the complete PRD file, not just the changed sections.
