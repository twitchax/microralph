# microralph — Adapt Language Prompt

## Objective

Rewrite the microralph prompts and templates for a different programming language.

## Context

The user has initialized microralph for a **{{language}}** project. The default prompts and templates are designed for Rust projects. You need to adapt them.

## Target Language

**{{language}}**

## Typical Build/Test Commands for {{language}}

{{#if build_commands}}
{{#each build_commands}}
- {{command}}
{{/each}}
{{/if}}

## Files to Update

The following files in `.mr/` need to be adapted for {{language}}:

### Templates (`.mr/templates/`)
- `prd.md` — Update example commands in the template

### Prompts (`.mr/prompts/`)
- `run_task.md` — Change `cargo make uat` to the appropriate test/build command for {{language}}
- `run_task_finalize.md` — Update verification commands
- `bootstrap_plan.md` — Update detection heuristics for {{language}} project structure
- `init.md` — Update Makefile.toml references if not applicable

### AGENTS.md
- Update the Quick Start section with {{language}}-appropriate commands
- Update build/test commands

## Required Actions

1. Read each file listed above from `.mr/` and the root `AGENTS.md`.
2. For each file, rewrite it to use {{language}}-appropriate:
   - Build commands (e.g., `npm test`, `pytest`, `go test`, `mvn test`)
   - Project structure references (e.g., `package.json`, `pyproject.toml`, `go.mod`)
   - Tool chains and conventions
3. Write the updated files back to disk.
4. Keep the overall structure and purpose of each file intact.
5. Preserve all `{{placeholder}}` template variables — only change the static content.

## Constraints

- Do not change the file structure or add new files.
- Do not remove placeholder variables like `{{prd_path}}`, `{{next_task_id}}`, etc.
- Keep the microralph-specific sections (e.g., History format, PRD frontmatter references).
- Preserve the auto-managed section markers in AGENTS.md.

## Output

Confirm which files were updated and summarize the key changes made for {{language}}.
