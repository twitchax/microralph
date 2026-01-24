# microralph — Update Agents Prompt

## Objective

Update the AGENTS.md file with relevant conventions and patterns.

## Context

A PRD has been created or a task has been completed.

## Current AGENTS.md Content

{{agents_content}}

## Recent Changes

{{#each recent_changes}}
- {{file}}: {{description}}
{{/each}}

## Required Actions

1. Review the recent changes.
2. Identify any new conventions, patterns, or important notes.
3. Update the auto-managed section of AGENTS.md if needed.

## Auto-Managed Section

Only modify content between these markers:
```
<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
...
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
```

## Constraints

- Do not modify content outside the auto-managed section.
- Keep additions concise and actionable.
- Only add information that helps future coding agents.

## Output

The updated content for the auto-managed section, or "NO_CHANGES" if no updates are needed.
