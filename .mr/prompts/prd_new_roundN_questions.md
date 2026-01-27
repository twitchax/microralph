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
**Q{{@index}}**:

{{question}}

**A{{@index}}**:

{{answer}}

{{/each}}

## Required Actions

1. Review the Q/A history.
2. Determine if you have enough information to synthesize a PRD.
3. If more clarification is needed, ask 1-5 additional **genuine questions** that require new information from the user.
4. If ready, respond with exactly: `READY_TO_SYNTHESIZE`

## Output Format

Either:
- A numbered list of follow-up questions (1-5 max)
- Or the exact text: `READY_TO_SYNTHESIZE`

## CRITICAL: What Counts as a Question

**DO NOT** output confirmations, summaries, or restatements of previous answers. These are NOT questions:
- "**`--scaffold` flag**: Uses hybrid detection..." ❌ (This is a summary)
- "**Feature X**: Confirmed as discussed" ❌ (This is a confirmation)

**DO** ask questions that require the user to provide NEW information:
- "What error behavior should occur if X fails?" ✓
- "Should this feature support Y use case?" ✓
- "How should the system handle edge case Z?" ✓

If you have no genuine questions requiring new information, output `READY_TO_SYNTHESIZE`.
