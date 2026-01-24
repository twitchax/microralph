# microralph — PRD New Synthesize Prompt

## Objective

Synthesize a complete PRD from the Q/A session.

## Context

The user is creating a new PRD with slug: `{{slug}}`

## Q/A Session

{{#each qa_history}}
**Q**: {{question}}
**A**: {{answer}}

{{/each}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}}
{{/each}}

## Required Actions

1. Generate the next PRD ID (e.g., PRD-0002 if PRD-0001 exists).
2. Create a complete PRD file with:
   - YAML frontmatter (id, title, status, owner, created, updated, tasks)
   - Summary section
   - Problem section
   - Goals section
   - Non-Goals section (if applicable)
   - Acceptance Tests section
   - Empty History section

3. Tasks should:
   - Have unique IDs (T-001, T-002, etc.)
   - Have clear, actionable titles
   - Be prioritized (1 = highest)
   - Start with status: todo

## Output

The complete PRD file content in Markdown format.
