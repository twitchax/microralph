# microralph — Init Prompt

## Objective

Initialize a new repository with microralph structure.

## Context

You are initializing a new repository for use with microralph (`mr`).

## Required Actions

1. Create the `.mr/` directory structure:
   - `.mr/prds/` — PRD files
   - `.mr/templates/` — PRD templates
   - `.mr/prompts/` — Static prompt files
   - `.mr/PRDS.md` — PRD index

2. Create a starter `AGENTS.md` file at the repo root.

3. Ensure `Makefile.toml` exists with required tasks:
   - `ci`
   - `fmt`
   - `clippy`
   - `test`
   - `uat`

## Output

Confirm initialization is complete and list the files created.
