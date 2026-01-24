# microralph — Pick PRD Prompt

## Objective

Analyze the available PRDs and determine which one should be worked on next.

## Context

The user has invoked `mr run` without specifying a PRD ID. Your job is to study the available PRDs and recommend the best one to work on next.

## Available PRDs

{{#each prds}}
### {{id}}: {{title}}

- **Status**: {{status}}
- **Progress**: {{completed}}/{{total}} tasks complete
- **Incomplete Tasks**:
{{#each incomplete_tasks}}
  - {{id}}: {{title}} (priority: {{priority}})
{{/each}}

{{/each}}

## Required Analysis

Consider the following when choosing:

1. **PRD Status**: Active PRDs should generally be prioritized over Draft PRDs.
2. **Progress**: PRDs that are closer to completion may be worth finishing first.
3. **Task Priority**: Look at the priorities of remaining tasks.
4. **Dependencies**: Check if any PRD references or depends on another.
5. **Momentum**: Consider which PRD would provide the most value if completed next.

## Output Format

Respond with ONLY the PRD ID that should be worked on next, on a single line. No explanation, no markdown, just the ID.

Example:
```
PRD-0002
```

If there are no valid PRDs to work on (no active/draft PRDs with incomplete tasks), respond with:
```
NONE
```
