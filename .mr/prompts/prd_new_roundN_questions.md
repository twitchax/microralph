# microralph — PRD New Round N Questions Prompt

## Objective

Continue the Q/A session for PRD creation, or signal readiness.

## Context

The user is creating a new PRD with slug: `{{slug}}`

{{#if user_context}}
### User-Provided Context

{{user_context}}

{{/if}}
{{#if constitution}}
### Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**Note**: Your questions and the resulting PRD should respect these constitutional rules.

{{/if}}
## Previous Q/A

{{#each qa_history}}
**Q{{@index}}**: {{question}}
**A{{@index}}**: {{answer}}

{{/each}}

## Required Actions

1. Review the Q/A history.
2. Determine if you have enough information to synthesize a PRD.
3. If more clarification is needed, ask 1-5 additional questions.  IMPORTANT: lean toward asking questions if you think it will help the PRD quality.
4. If ready, respond with exactly: `READY_TO_SYNTHESIZE`

## Output Format

Either:
- A numbered list of follow-up questions (1-5 max)
- Or the exact text: `READY_TO_SYNTHESIZE`
