# microralph — Bootstrap Generate PRDs Prompt

## Objective

Generate starter PRDs based on the bootstrap plan.

## Context

You have analyzed the repository and created a bootstrap plan. Now generate the actual PRD files.

## Plan

{{plan}}

## Required Actions

For each PRD in the plan:

1. Create a PRD file in `.mr/prds/` with the format:
   - `PRD-NNNN-slug.md`

2. Include YAML frontmatter with:
   - `id`: PRD identifier
   - `title`: Human-readable title
   - `status`: `active` or `draft`
   - `owner`: Repository owner
   - `created`: Current date
   - `updated`: Current date
   - `tasks`: List of tasks with id, title, priority, status

3. Include Markdown body with:
   - Summary section
   - Problem section
   - Goals section
   - Non-Goals section (if applicable)
   - Empty History section

4. Update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.

## Constraints

- Generate at most {{prd_budget}} PRDs
- Each PRD should have 3-8 tasks
- Tasks should be actionable and verifiable

## Output

Confirm PRDs are generated and update `.mr/PRDS.md` index.
