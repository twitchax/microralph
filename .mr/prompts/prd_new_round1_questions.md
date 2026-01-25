# microralph — PRD New Round 1 Questions Prompt

## Objective

Generate follow-up questions to clarify a new PRD request.

## Context

The user wants to create a new PRD with slug: `{{slug}}`

{{#if user_description}}
User's initial description:
> {{user_description}}
{{/if}}

{{#if user_context}}
User's upfront context:
> {{user_context}}
{{/if}}

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**Note**: Your questions and the resulting PRD should respect these constitutional rules.
{{/if}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}} ({{status}})
{{/each}}

## Required Actions

1. Review the existing PRDs to understand context.
2. Scan the codebase for relevant files, patterns, or entry points that could bootstrap specific tasks.
3. Generate 3-5 clarifying questions to understand:
   - What problem does this PRD solve?
   - What are the success criteria?
   - What are the acceptance tests?
   - What are the dependencies or blockers?
   - What is the scope (MVP vs full feature)?
   - Are there specific sections in existing PRDs that are relevant (e.g., patterns, lessons learned)?
   - Are there existing code files or modules that should be referenced?

## Output Format

Return a numbered list of questions. Keep questions concise and actionable.

Example:
1. What specific problem are you trying to solve?
2. What does "done" look like for this feature?
3. Are there any existing patterns in the codebase to follow?
