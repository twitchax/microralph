# Constitution

This file defines project-specific governance rules and constraints that guide PRD creation and execution.

## Purpose

The constitution:
- Encodes project-specific best practices and constraints
- Influences PRD creation and finalization via LLM prompts
- Is version-controlled and user-editable
- Violations are logged in PRD history but do not block execution

## Rules

1. **Single Source of Truth**: Follow the DRY (Don't Repeat Yourself) principle. Avoid duplicating logic, data, or configuration across multiple files. When the same information must exist in multiple places, derive it from a single authoritative source.

2. **Separation of Concerns**: Follow SOC (Separation of Concerns) principles. Each module, function, and file should have a single, well-defined responsibility. Avoid mixing unrelated concerns in the same code unit.

3. **Minimal Changes**: When making changes, modify only what is necessary to achieve the objective. Avoid unrelated refactoring, style changes, or "improvements" that are not directly related to the task at hand.

4. **Consistency**: Follow the existing code style, conventions, and patterns established in the codebase. Do not introduce new patterns without justification.

5. **Public API Stability**: Do not change public API signatures unless the task explicitly requires it. Breaking changes must be documented and justified in the PRD history.

6. **Root Cause Resolution**: Prefer fixing root causes over applying surface-level workarounds. When a workaround is necessary, document the underlying issue and rationale.

<!-- Add your project-specific rules below: -->
