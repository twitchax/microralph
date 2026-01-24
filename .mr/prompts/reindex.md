# microralph — Reindex Prompt

## Objective

Regenerate the `.mr/PRDS.md` index file and verify/fix inter-PRD links and code links across all PRDs.

## Context

The user wants to:
1. Regenerate the `.mr/PRDS.md` index to reflect the current state of all PRDs.
2. Scan all PRDs for inter-PRD links (e.g., references to other PRD IDs) and code links (e.g., references to source files).
3. Verify that all links are valid and use proper Markdown link syntax.
4. Fix any broken or incorrectly formatted links.

## PRDs Directory

Path: `{{prds_dir}}`

## Current PRD Files

{{#each prd_files}}
- `{{filename}}` (ID: {{id}}, Title: {{title}})
{{/each}}

## Repository Root

Path: `{{repo_root}}`

## Required Actions

1. **Read all PRD files** in `{{prds_dir}}`.

2. **Regenerate the index** by running: `cargo run -- list`

3. **Scan for inter-PRD references** in each PRD:
   - Look for mentions of PRD IDs like `PRD-0001`, `PRD-0002`, etc.
   - Convert plain text references to proper Markdown links: `[PRD-0001](./PRD-0001-slug.md)`
   - Use relative paths from the PRD's location.

4. **Scan for code references** in each PRD:
   - Look for file paths like `src/module.rs`, `lib/file.js`, etc.
   - Verify the files exist in the repository.
   - Convert plain text references to proper Markdown links: `[src/module.rs](../../src/module.rs)`
   - Use relative paths from the PRD's location (`.mr/prds/`).
   - For line references like "line 42", consider using GitHub-style anchors: `[src/module.rs#L42](../../src/module.rs#L42)`

5. **Update PRD files** with fixed links:
   - Only modify files that have broken or incorrectly formatted links.
   - Preserve all other content exactly as-is.

## Link Format Guidelines

### Inter-PRD Links

- From: `see PRD-0002 for details`
- To: `see [PRD-0002](./PRD-0002-feature-name.md) for details`

### Code File Links

- From: `implementation in src/run.rs`
- To: `implementation in [src/run.rs](../../src/run.rs)`

### Code Line Links

- From: `defined at src/run.rs line 42`
- To: `defined at [src/run.rs#L42](../../src/run.rs#L42)`

## Constraints

- Do not modify PRD content other than fixing links.
- Do not change the structure of PRD files.
- Do not add links where none were intended (only convert existing plain-text references).
- Preserve the YAML frontmatter exactly.
- Keep History sections intact.

## Output

Report what was done:
- Confirmation that the index was regenerated
- Number of PRDs scanned
- Number of inter-PRD links verified/fixed
- Number of code links verified/fixed
- List of files modified (if any)
