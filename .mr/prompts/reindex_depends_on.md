# microralph — Reindex Depends On Prompt

## Objective

Analyze all PRDs and infer/fix `depends_on` relationships based on content, dates, and logical dependencies.

## Context

The user wants to auto-populate or fix the `depends_on` field in PRD frontmatter. This field represents directed edges: "this PRD should be done after the dependencies".

## PRDs Directory

Path: `{{prds_dir}}`

## Current PRD Files

{{#each prd_files}}
### {{id}}: {{title}}
- **File**: `{{filename}}`
- **Status**: {{status}}
- **Created**: {{created}}
- **Depends On**: {{#if depends_on}}{{depends_on}}{{else}}(none){{/if}}
- **Summary**: {{summary}}
{{/each}}

## Repository Root

Path: `{{repo_root}}`

## Required Actions

1. **Read all PRD files** in `{{prds_dir}}` to understand their content, goals, and relationships.

2. **Analyze dependencies** by considering:
   - **Temporal order**: Earlier PRDs (by creation date or ID) often are dependencies
   - **Content references**: If PRD-B mentions concepts or files introduced by PRD-A, PRD-B likely depends on PRD-A
   - **Logical progression**: Foundation/infrastructure PRDs are dependencies for feature PRDs
   - **Explicit mentions**: References to other PRD IDs in body text suggest dependencies

3. **Infer missing depends_on** relationships:
   - For PRDs with empty `depends_on`, analyze their content to determine likely dependencies
   - Be conservative: only add dependencies that are clearly implied
   - Don't create circular dependencies

4. **Fix existing depends_on** relationships:
   - Verify that referenced PRD IDs actually exist
   - Remove references to non-existent PRDs
   - Add missing dependencies that are clearly implied by content

5. **Update PRD files** with corrected `depends_on` fields:
   - Modify only the frontmatter YAML, preserving all other content
   - Use the format: `depends_on: ["PRD-0001", "PRD-0003"]`
   - Keep the list sorted by PRD ID

## Dependency Inference Guidelines

### Strong indicators of dependency:
- PRD-B explicitly mentions implementing something "from PRD-A"
- PRD-B modifies files first created in PRD-A
- PRD-B's tasks require PRD-A's completed work
- PRD-B's acceptance tests depend on PRD-A's features

### Weak indicators (use cautiously):
- PRD-B was created after PRD-A (not sufficient alone)
- PRD-B works on the same module as PRD-A (may be parallel, not dependent)
- Similar topic areas (may be unrelated)

### What NOT to infer:
- Don't add dependencies just because PRDs touch the same file
- Don't create long dependency chains unnecessarily
- Don't assume all earlier PRDs are dependencies
- Avoid circular dependencies (PRD-A → PRD-B → PRD-A)

## Constraints

- Only modify the `depends_on` field in frontmatter
- Preserve all other frontmatter fields exactly
- Preserve the body content exactly
- Keep History sections intact
- Don't add dependencies that create cycles

## Output

Report what was done:
- Number of PRDs analyzed
- Number of depends_on relationships added
- Number of depends_on relationships fixed (invalid refs removed)
- List of changes made (e.g., "PRD-0005: added depends_on PRD-0003")
