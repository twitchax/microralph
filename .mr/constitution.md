# Constitution of Microralph

Microralph is an AI-powered software development assistant designed to help developers create, maintain, and improve software projects through AI-generated code, documentation, and tests.

## Purpose

This constitution defines the core principles and rules that govern the development and maintenance of the Microralph project. All contributors, including AI agents, must adhere to these rules when making changes to the codebase.

## Rules

1. **Prompt Management**: All prompt templates and default content must be defined in `src/init.rs` (as embedded constants) and materialized to `.mr/prompts/` during initialization. These two sources must remain consistent and synchronized.

2. **Single Source of Truth**: Follow the DRY (Don't Repeat Yourself) principle. Avoid duplicating logic, data, or configuration across multiple files. When the same information must exist in multiple places, derive it from a single authoritative source.

3. **Separation of Concerns**: Follow SOC (Separation of Concerns) principles. Each module, function, and file should have a single, well-defined responsibility. Avoid mixing unrelated concerns in the same code unit.

4. **Minimal Changes**: When making changes, modify only what is necessary to achieve the objective. Avoid unrelated refactoring, style changes, or "improvements" that are not directly related to the task at hand.

5. **Consistency**: Follow the existing code style, conventions, and patterns established in the codebase. Do not introduce new patterns without justification.

6. **Public API Stability**: Do not change public API signatures unless the task explicitly requires it. Breaking changes must be documented and justified in the PRD history.

7. **Root Cause Resolution**: Prefer fixing root causes over applying surface-level workarounds. When a workaround is necessary, document the underlying issue and rationale.