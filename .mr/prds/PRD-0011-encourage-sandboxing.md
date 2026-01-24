---
id: PRD-0011
title: "Dev Container Support and Generation"
status: draft
owner: "twitchax"
created: 2026-01-24
updated: 2026-01-24

principles:
- Use `.devcontainer/devcontainer.json` format for broad tooling compatibility
- Generation should be fully autonomous, analyzing repo state, git history, and PRDs
- Dev container warnings should be informative but non-blocking
- Generated config should enable "sandbox first" development workflow

references:
- name: Dev Container Specification
  url: https://containers.dev/implementors/json_reference/

acceptance_tests:
- id: uat-001
  name: Verify devcontainer generate creates valid config with fake context
  command: cargo make test
  uat_status: unverified

tasks:
- id: T-001
  title: Add README section documenting dev container setup and usage
  priority: 1
  status: todo
  notes: Include instructions for VSCode, Codespaces, and CLI-based workflows. Explain benefits of sandboxed development.

- id: T-002
  title: Implement dev container detection utility
  priority: 2
  status: todo
  notes: Check for `/workspaces` path, `REMOTE_CONTAINERS` env var, or other indicators. Make reusable across commands.

- id: T-003
  title: Add dev container warning to model-invoking commands
  priority: 3
  status: todo
  notes: Show warning if not in dev container. Keep message brief and non-blocking. Apply to `mr run`, `mr prd new`, and `mr devcontainer generate`.

- id: T-004
  title: Implement `mr devcontainer generate` command
  priority: 4
  status: todo
  notes: New top-level command. Analyze repo structure, git logs, installed tools, and PRD content. Generate `.devcontainer/devcontainer.json` with appropriate base image, extensions, and tool installations.

- id: T-005
  title: Add repo analysis module for dev container generation
  priority: 5
  status: todo
  notes: Detect languages/frameworks from cargo.toml, package files, etc. Parse git logs for recently added dependencies. Scan PRDs for tool references.

- id: T-006
  title: Create dev container prompt template
  priority: 6
  status: todo
  notes: Template should instruct model to generate devcontainer.json based on repo analysis. Include context about microralph requirements (Rust, cargo-make, cargo-nextest).

- id: T-007
  title: Write unit test for devcontainer generate with fake context
  priority: 7
  status: todo
  notes: Test that command creates valid JSON in temporary location. Mock repo analysis to provide deterministic input.

---

# Summary

Enable developers to easily use dev containers for sandboxed microralph development by adding comprehensive README documentation and a new `mr devcontainer generate` command that autonomously creates `.devcontainer/devcontainer.json` based on repository analysis.

---

# Problem

Currently, there is no guidance or tooling to help developers set up a consistent, sandboxed development environment for microralph. As the project evolves and dependencies/tools are added, manually maintaining a dev container configuration becomes tedious and error-prone.

---

# Goals

1. Document dev container setup and usage in README for VSCode, Codespaces, and CLI workflows
2. Implement `mr devcontainer generate` command that autonomously creates/updates `.devcontainer/devcontainer.json`
3. Analyze repo state (dependencies, tools, git history, PRDs) to generate appropriate container config
4. Show non-blocking warnings when running model-invoking commands outside dev containers
5. Support regeneration after significant project changes to keep sandbox environment useful

---

# Non-Goals (MVP)

- Supporting non-standard container formats beyond `.devcontainer/devcontainer.json`
- Automatic detection and triggering of regeneration (user must explicitly run command)
- IDE-specific customizations beyond standard dev container features
- Multi-container or docker-compose configurations
- Custom Dockerfile generation (use base images from registry)

---

# History

(Entries appended by `mr run` will go below this line.)

---