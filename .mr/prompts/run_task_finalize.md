# microralph — Run Task Finalize Prompt

## Objective

Complete the final wrap-up task for a PRD, including changelog entry generation.

## Context

You are executing the wrap-up task for `{{prd_id}}`: **{{prd_title}}**.

All other tasks in this PRD have been completed.

## PRD Summary

{{prd_summary}}

## Completed Tasks

{{completed_tasks}}

## Current Changelog

The current `CHANGELOG.md` content is:

```markdown
{{changelog_content}}
```

## Required Actions

1. Review all changes made during this PRD.
2. Ensure documentation is up-to-date:
   - README.md
   - AGENTS.md
   - Inline comments
3. Verify all acceptance tests pass: `cargo make uat`
4. Clean up any temporary or debug code.
5. Ensure the codebase is in a releasable state.
6. **Generate a changelog entry** under `## [Unreleased]` in `CHANGELOG.md`:
   - Use Keep a Changelog format with appropriate category (`Added`, `Changed`, `Fixed`, `Deprecated`, `Removed`, `Security`)
   - Include the PRD ID and title
   - Summarize key changes based on completed tasks
   - Example format: `- {{prd_id}}: {{prd_title}} — Brief description of key changes`

## Changelog Entry Guidelines

- **Added**: New features or functionality
- **Changed**: Changes to existing functionality
- **Fixed**: Bug fixes
- **Deprecated**: Features marked for removal
- **Removed**: Removed features
- **Security**: Security-related changes

Choose the most appropriate category based on the PRD's completed tasks.

## Constraints

- Do not introduce new features.
- Focus on polish and documentation.
- Ensure consistency across the codebase.
- The changelog entry should be concise but informative.

## Output

Provide a final summary suitable for closing out the PRD History section, including confirmation that the changelog entry was added.
