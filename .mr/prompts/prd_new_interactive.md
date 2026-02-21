# microralph — PRD New Interactive Prompt

## Objective

Have an interactive conversation with the user to gather enough information to create a well-defined PRD, then write it directly to disk.

## Context

The user wants to create a new PRD with slug: `{{slug}}`
The next available PRD ID is: `{{next_id}}`
The PRD file should be written to: `{{prd_path}}`

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

**CRITICAL**: The PRD you create MUST respect these constitutional rules. If any aspect of the PRD would violate the constitution, adjust the approach or note the constraint explicitly.
{{/if}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}} ({{status}})
{{/each}}

## Phase 1: Interactive Discovery

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
5. Reference existing PRDs and code when relevant.

## Phase 2: Write the PRD

When you have enough information, tell the user you're ready to write the PRD. Then:

1. **Write the PRD file** directly to `{{prd_path}}` using your file editing tools.
2. The PRD MUST follow the template structure below EXACTLY.
3. After writing the file, tell the user the PRD has been created and they can exit the chat.

## PRD Template Structure

The PRD has two parts that you MUST follow exactly:

### 1. YAML Frontmatter (between `---` delimiters)

The frontmatter contains ALL structured data:
- `id`: `{{next_id}}`
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

**Technical Approach Guidance**: When the feature involves multiple components, services, or complex data flows, include an architecture diagram. Use ASCII art for simple diagrams or Mermaid syntax for more complex ones.

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

## AGENTS.md

After writing the PRD, update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.

## Important

- The PRD ID MUST be `{{next_id}}`.
- The PRD file MUST be written to `{{prd_path}}`.
- Do NOT just output the PRD content to the chat — you MUST write it to disk using your file tools.
- After writing the file, tell the user the PRD is ready and they can exit.
