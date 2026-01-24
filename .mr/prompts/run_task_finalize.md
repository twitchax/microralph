# microralph — Run Task Finalize Prompt

## Objective

Complete the final wrap-up task for a PRD.

## Context

You are executing the wrap-up task for `{{prd_id}}`.

All other tasks in this PRD have been completed.

## PRD Summary

{{prd_summary}}

## Required Actions

1. Review all changes made during this PRD.
2. Ensure documentation is up-to-date:
   - README.md
   - AGENTS.md
   - Inline comments
3. Verify all acceptance tests pass: `cargo make uat`
4. Clean up any temporary or debug code.
5. Ensure the codebase is in a releasable state.

## Constraints

- Do not introduce new features.
- Focus on polish and documentation.
- Ensure consistency across the codebase.

## Output

Provide a final summary suitable for closing out the PRD History section.
