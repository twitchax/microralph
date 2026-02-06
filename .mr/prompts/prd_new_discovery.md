# microralph — PRD New Interactive Discovery Prompt

## Objective

Have an interactive conversation with the user to gather enough information to create a well-defined PRD.

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

**Note**: The resulting PRD should respect these constitutional rules.
{{/if}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}} ({{status}})
{{/each}}

## Required Actions

1. Review the existing PRDs to understand project context.
2. Scan the codebase for relevant files, patterns, or entry points.
3. Engage the user in a natural conversation to understand:
   - What problem does this PRD solve?
   - What are the success criteria and acceptance tests?
   - What are the dependencies or blockers?
   - What is the scope (MVP vs full feature)?
   - What is the high-level technical approach?
   - What assumptions and constraints apply?
4. Ask follow-up questions based on the user's responses.
5. When you have enough information to write a complete PRD, let the user know and end the conversation.

## Conversation Guidelines

- Ask questions naturally, one or a few at a time.
- Follow up on interesting threads or ambiguous answers.
- Reference existing PRDs and code when relevant.
- Do NOT generate a PRD during this conversation — that happens in a separate synthesis step.
- When you have enough information, tell the user and suggest they exit the chat.
