# microralph — Bootstrap Plan Prompt

## Objective

Analyze an existing repository and plan PRD generation.

## Context

You are analyzing an existing repository to understand its structure and plan the generation of PRDs.

## Required Analysis

1. **Detect cargo-make entrypoints and required tasks**
   - Look for `Makefile.toml`, `Makefile`, `package.json` scripts
   - Identify build, test, lint, and deployment commands

2. **Detect crates/modules and responsibilities**
   - Identify the main modules and their purposes
   - Understand the architectural layers

3. **Detect CI workflows and required checks**
   - Look for `.github/workflows/`, `.gitlab-ci.yml`, etc.
   - Understand the existing CI/CD pipeline

4. **Detect docs that imply features**
   - Read README, DEVELOPMENT, CONTRIBUTING, etc.
   - Identify planned features, TODOs, or roadmap items

5. **Detect TODO/FIXME hotspots**
   - Search for TODO, FIXME, HACK comments
   - Prioritize areas needing attention

## Output

Produce a structured plan for PRD generation, including:
- List of proposed PRDs with titles
- Priority ordering
- Key tasks for each PRD
