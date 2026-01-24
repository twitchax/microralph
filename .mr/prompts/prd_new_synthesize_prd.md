# microralph — PRD New Synthesize Prompt

## Objective

Synthesize a complete PRD from the Q/A session.

## Context

The user is creating a new PRD with slug: `{{slug}}`

{{#if user_context}}
User's upfront context:
> {{user_context}}
{{/if}}

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
2. Scan the codebase for relevant files, patterns, or entry points that could bootstrap specific tasks.
3. Review existing PRDs for relevant sections (e.g., patterns, lessons learned, related features).
4. Create a complete PRD file with:
   - YAML frontmatter (id, title, status, owner, created, updated, tasks)
   - Summary section
   - Problem section
   - Goals section
   - Non-Goals section (if applicable)
   - Relevant References section (links to specific sections in other PRDs and code files)
   - Acceptance Tests section
   - Empty History section

5. Tasks should:
   - Have unique IDs (T-001, T-002, etc.)
   - Have clear, actionable titles
   - Be prioritized (1 = highest)
   - Start with status: todo
   - Include code file links where relevant (e.g., `See src/module.rs` for entry point)

6. Acceptance tests should:
   - Have unique IDs (uat-001, uat-002, etc.)
   - Include a `uat_status` field: `unverified` (default, no real test exists yet) or `verified` (a real test exists)
   - New acceptance tests should start as `unverified` unless you create the actual test

7. Relevant References section should:
   - Link to specific sections in other PRDs that inform this work (e.g., `See PRD-0001 ## Lessons Learned`)
   - Link to relevant code files that bootstrap tasks (e.g., `src/main.rs` for CLI entry points)
   - Only include genuinely useful references, not exhaustive lists

## Output

The complete PRD file content in Markdown format.
