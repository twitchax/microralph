# Dev Container Configuration Generation

## Objective

Generate a `.devcontainer/devcontainer.json` file based on repository analysis that enables developers to run this project in a consistent, sandboxed dev container environment.

## Context

You are analyzing a repository to create a dev container configuration. Below is the analysis of the repository:

```
{{analysis}}
```

## Requirements

Generate a complete, valid `devcontainer.json` file that:

1. **Uses an appropriate base image** for `{{language}}` development
   - For Rust: `mcr.microsoft.com/devcontainers/rust:latest`
   - For Python: `mcr.microsoft.com/devcontainers/python:3.11`
   - For Node.js: `mcr.microsoft.com/devcontainers/javascript-node:18`
   - For Go: `mcr.microsoft.com/devcontainers/go:latest`
   - For Java: `mcr.microsoft.com/devcontainers/java:17`

2. **Includes necessary VS Code extensions** for the detected language and frameworks

3. **Installs development tools** referenced in the analysis (e.g., cargo-make, cargo-nextest for Rust)

4. **Sets up the environment** with appropriate post-create commands to install dependencies

5. **Forwards relevant ports** if the project includes web servers or APIs

6. **Configures container settings** like mounts, environment variables as needed

## Analysis Guidelines

Based on the repository analysis:
- Identify tools from git commit messages (e.g., "add cargo-make", "use nextest")
- Include extensions for detected frameworks (e.g., rust-analyzer for Rust, pylint for Python)
- Set up post-create commands to run common initialization (e.g., `cargo build`, `npm install`)
- If microralph is detected, ensure `gh` CLI is available for GitHub Copilot integration

## Constraints

- Use only official Microsoft dev container base images
- Include only extensions that are actively maintained and widely used
- Keep post-create commands minimal and fast (prefer lazy installation)
- Do not include secrets or credentials in the configuration
- Ensure the JSON is valid and properly formatted

---

## Task

**Create the file `.devcontainer/devcontainer.json` directly** with the generated configuration.

Use your file creation tools to write the file. Do not just output the JSON content - actually create the file in the repository.
