# microralph — PRD Edit Interactive Prompt

## Objective

Have an interactive conversation with the user to understand what changes they want to make to an existing PRD, then write the updated PRD directly to disk.

## Context

The user wants to edit the PRD at `{{prd_path}}`.

{{#if context}}
User's upfront context:
> {{context}}
{{/if}}

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**CRITICAL**: The updated PRD MUST respect these constitutional rules. If any aspect of the edit would violate the constitution, adjust the approach or note the constraint explicitly.
{{/if}}

## Current PRD Content

```markdown
{{prd_content}}
```

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}} ({{status}})
{{/each}}

## Phase 1: Interactive Discovery

1. Review the current PRD content carefully.
2. If the user provided upfront context, use it to understand what they want to change.
3. Engage the user in a natural conversation to understand:
   - What specific changes do they want to make?
   - Should tasks be added, removed, or modified?
   - Should acceptance tests be updated?
   - Are there scope or priority changes?
4. Ask follow-up questions based on the user's responses.
5. Reference existing PRDs and the current PRD content when relevant.

## Phase 2: Write the Updated PRD

When you have enough information, tell the user you're ready to apply the changes. Then:

1. **Write the updated PRD file** directly to `{{prd_path}}` using your file editing tools.
2. The PRD MUST preserve the existing template structure.
3. After writing the file, tell the user the PRD has been updated and they can exit the chat.

## Constraints

- Do NOT change the PRD ID.
- Do NOT remove existing History entries.
- Keep the overall structure intact (frontmatter, Summary, Problem, Goals, Non-Goals, History sections).
- If adding tasks, assign appropriate IDs (T-NNN) and priorities.
- If adding acceptance tests, assign appropriate IDs (uat-NNN).
- **YAML Quoting**: Strings containing colons (`:`) or hashes (`#`) MUST be quoted to avoid parse errors. Example: `title: "Fix: Bug in parser"`

## Important

- The PRD file MUST be written to `{{prd_path}}`.
- Do NOT just output the PRD content to the chat — you MUST write it to disk using your file tools.
- After writing the file, tell the user the PRD is updated and they can exit.
