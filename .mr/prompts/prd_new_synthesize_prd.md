# microralph — PRD New Synthesize Prompt

## Objective

Synthesize a complete PRD from the interactive discovery conversation, following the template structure exactly.

## Context

The user is creating a new PRD with slug: `{{slug}}`

{{#if user_context}}
User's upfront context:
> {{user_context}}
{{/if}}

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**CRITICAL**: The PRD you synthesize MUST respect these constitutional rules. If any aspect of the PRD would violate the constitution, adjust the approach or note the constraint explicitly.
{{/if}}

{{#if conversation_transcript}}
## Conversation Transcript

The following is a transcript of the interactive discovery conversation between the user and the agent. Extract all requirements, constraints, decisions, and context from this conversation to inform the PRD:

{{conversation_transcript}}

**Synthesis guidance**: Identify the user's goals, technical preferences, scope boundaries, and any decisions made during the conversation. Transform discussion points into structured PRD sections — goals become Goals, discussed limitations become Constraints, and agreed-upon implementation details become the Technical Approach.
{{/if}}

{{#if session_id}}
## Session Context

This prompt is a continuation of a previous interactive session (session: {{session_id}}).
Use the full conversational context from that session to inform PRD synthesis.
{{/if}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}}
{{/each}}

## PRD Template Structure

The PRD has two parts that you MUST follow exactly:

### 1. YAML Frontmatter (between `---` delimiters)

The frontmatter contains ALL structured data:
- `id`: PRD-NNNN (generate next ID based on existing PRDs)
- `title`: Human-readable title
- `status`: draft (for new PRDs)
- `owner`: Owner name
- `created` / `updated`: Date in YYYY-MM-DD format
- `principles`: List of guiding constraints or design decisions
- `references`: List of objects with `name` and `url` fields
- `acceptance_tests`: List of UATs with `id`, `name`, `command`, `uat_status`
- `tasks`: List of tasks with `id`, `title`, `priority`, `status`, `notes`

### 2. Markdown Body (after closing `---`)

The body contains ONLY narrative/exposition sections:
- `# Summary` — Brief overview
- `# Problem` — Problem statement
- `# Goals` — Numbered list of goals
- `# Technical Approach` — Implementation strategy, architecture decisions, and high-level design. Include ASCII or Mermaid diagrams when the approach involves complex component interactions or data flows.
- `# Assumptions` — Preconditions the implementation depends on
- `# Constraints` — Technical or scope limitations that affect implementation options
- `# References to Code` — Relevant files, modules, patterns, or entry points in the codebase
- `# Non-Goals (MVP)` — What's explicitly out of scope
- `# History` — Empty section for `mr run` to append entries

**Technical Approach Guidance**: When the feature involves multiple components, services, or complex data flows, include an architecture diagram. Use ASCII art for simple diagrams or Mermaid syntax for more complex ones. Diagrams help AI agents during `mr run` understand the implementation strategy at a glance.

## Required Actions

1. Generate the next PRD ID (e.g., PRD-0006 if PRD-0005 exists).
2. Scan the codebase for relevant files, patterns, or entry points.
3. Review existing PRDs for related work or patterns.
4. Create the PRD following the template structure EXACTLY.
5. Update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.

## Acceptance Tests Format

Each acceptance test in the frontmatter MUST have these fields:
```yaml
- id: uat-001
  name: Short description of what the test verifies
  command: cargo make uat  # or specific test command
  uat_status: unverified   # always start as unverified
```

## Tasks Format

Each task in the frontmatter MUST have these fields:
```yaml
- id: T-001
  title: Clear, actionable task title
  priority: 1              # lower = higher priority
  status: todo             # always start as todo
  notes: Optional implementation hints or dependencies
```

## YAML Frontmatter Quoting Rules

**CRITICAL**: YAML strings containing special characters MUST be quoted to avoid parse errors:
- **Colons (`:`)**: Any string with a colon must be quoted: `title: "Fix: Bug in parser"`
- **Hashes (`#`)**: Strings with `#` must be quoted to avoid comment interpretation
- **Leading/trailing spaces**: Use quotes to preserve whitespace
- **Empty strings**: Use `""` for empty values

When in doubt, wrap the value in double quotes. This is especially important for:
- `title` fields that often contain colons (e.g., "Feature: Add X")
- `notes` fields with complex descriptions
- `name` fields in references and acceptance tests

## Output

CRITICAL: Output ONLY the raw PRD file content. Start your response IMMEDIATELY with the `---` frontmatter delimiter. Do NOT wrap the output in code blocks. Do NOT include any preamble, explanation, or commentary.

The first three characters of your response MUST be exactly: `---`
