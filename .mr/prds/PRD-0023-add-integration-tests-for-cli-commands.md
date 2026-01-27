---
id: PRD-0023
title: "Add Integration Tests for CLI Commands"
status: draft
owner: ""
created: 2026-01-26
updated: 2026-01-26
depends_on: ["PRD-0001"]
principles:
  - Each test runs in its own temp directory with complete isolation
  - Use dependency injection to provide MockRunner to commands
  - Verify underlying state changes, not CLI output formatting
  - Follow Rust integration test conventions (tests/ directory)
  - MockRunner test methods must be visible to integration tests

references:
  - name: Rust Integration Tests
    url: https://doc.rust-lang.org/book/ch11-03-test-organization.html
  - name: tempfile crate
    url: https://docs.rs/tempfile/latest/tempfile/

acceptance_tests:
  - id: uat-001
    name: Integration tests pass for all CLI commands
    command: cargo make test
    uat_status: unverified
  - id: uat-002
    name: MockRunner test methods accessible from integration tests
    command: cargo build --tests
    uat_status: unverified

tasks:
  - id: T-001
    title: "Remove #[cfg(test)] from MockRunner helper methods"
    priority: 1
    status: todo
    notes: "Change add_response(), add_success(), recorded_prompts(), remaining_responses() to be unconditionally public so integration tests can use them."
  - id: T-002
    title: Refactor command functions to accept Box<dyn Runner>
    priority: 2
    status: todo
    notes: "Modify cmd_init, cmd_bootstrap, cmd_new, cmd_edit, cmd_run, cmd_finalize, cmd_suggest, cmd_refactor, cmd_reindex, cmd_devcontainer_generate, cmd_constitution_edit to accept runner as a trait object parameter instead of creating runners internally."
  - id: T-003
    title: Create test helper module for temp directory setup
    priority: 3
    status: todo
    notes: "Create tests/common/mod.rs with helpers: setup_temp_mr_dir() that creates temp dir with initialized .mr/ structure, and teardown utilities. Use tempfile crate."
  - id: T-004
    title: Add integration tests for init command
    priority: 4
    status: todo
    notes: "Test basic init, init with language, verify .mr/ structure created."
  - id: T-005
    title: Add integration tests for bootstrap command
    priority: 4
    status: todo
    notes: "Test bootstrap with mock runner responses, verify PRDS.md and PRD files created."
  - id: T-006
    title: Add integration tests for restore command
    priority: 4
    status: todo
    notes: "Test restore overwrites prompts/templates, verify files match built-in defaults."
  - id: T-007
    title: Add integration tests for new command
    priority: 4
    status: todo
    notes: "Test PRD creation with Q/A workflow, verify PRD file created with expected content."
  - id: T-008
    title: Add integration tests for edit command
    priority: 4
    status: todo
    notes: "Test PRD editing with mock runner, verify PRD content updated."
  - id: T-009
    title: Add integration tests for run command
    priority: 4
    status: todo
    notes: "Test task execution with mock runner, verify task status updated and history appended."
  - id: T-010
    title: Add integration tests for finalize command
    priority: 4
    status: todo
    notes: "Test PRD finalization, verify status changed to done."
  - id: T-011
    title: Add integration tests for list command
    priority: 4
    status: todo
    notes: "Test listing PRDs, verify correct PRDs returned with and without --done flag."
  - id: T-012
    title: Add integration tests for status command
    priority: 4
    status: todo
    notes: "Test status output reflects correct PRD and task states."
  - id: T-013
    title: Add integration tests for suggest command
    priority: 4
    status: todo
    notes: "Test suggestion generation with mock runner, verify suggestions returned."
  - id: T-014
    title: Add integration tests for refactor command
    priority: 4
    status: todo
    notes: "Test refactor iterations with mock runner, verify iteration count and termination signals."
  - id: T-015
    title: Add integration tests for reindex command
    priority: 4
    status: todo
    notes: "Test PRDS.md regeneration, verify index updated correctly."
  - id: T-016
    title: Add integration tests for devcontainer generate command
    priority: 4
    status: todo
    notes: "Test dev container config generation with mock runner, verify devcontainer.json created."
  - id: T-017
    title: Add integration tests for constitution edit command
    priority: 4
    status: todo
    notes: "Test constitution editing with mock runner, verify constitution.md updated."
  - id: T-018
    title: Add argument parsing edge case tests if straightforward
    priority: 5
    status: todo
    notes: "Optional: test invalid arguments, missing required fields where easy to identify."

---

# Summary

Add end-to-end integration tests for all CLI commands using dependency injection to provide MockRunner. Tests run in isolated temp directories using the tempfile crate and verify underlying state changes rather than CLI output formatting.

---

# Problem

While 28 unit test modules exist for individual components, there are no integration tests that exercise CLI subcommands end-to-end. Unit tests verify module logic in isolation but cannot catch regressions in:
- Command wiring and argument parsing
- Cross-module interactions during command execution
- File system side effects (PRD creation, status updates, history appending)
- Runner integration and prompt assembly

The MockRunner exists but its helper methods are gated behind `#[cfg(test)]`, making them inaccessible to integration tests in the `tests/` directory.

---

# Goals

1. Create integration tests for all 16+ CLI commands following Rust conventions
2. Refactor command functions to accept `Box<dyn Runner>` for testability
3. Use temp directories (via tempfile) for complete test isolation
4. Expose MockRunner helper methods for integration test access
5. Verify underlying state changes (files, PRD content) rather than CLI output

---

# Non-Goals (MVP)

- Testing actual CopilotRunner or ClaudeRunner (requires real CLI tools)
- Verifying stdout/stderr formatting or colors
- Comprehensive fuzzing of argument parsing edge cases
- Performance or stress testing
- Testing interactive prompts or user input flows

---

# History

(Entries appended by `mr run` will go below this line.)

---