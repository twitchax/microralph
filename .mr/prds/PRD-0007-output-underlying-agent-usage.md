---
id: PRD-0007
title: Output Underlying Agent Usage
status: done
owner: twitchax
created: 2026-01-24
updated: 2026-01-24
depends_on: ["PRD-0001"]
principles:
- Runner-specific implementation - each runner handles its own usage metrics
- Graceful degradation - runners without usage info simply omit the output
- Ephemeral display only - no persistence of token usage data
- Minimal output footprint - display usage inline with existing truncated output
references:
- name: GitHub Copilot CLI Documentation
  url: https://docs.github.com/en/copilot/using-github-copilot/using-github-copilot-in-the-command-line
acceptance_tests:
- id: uat-001
  name: CopilotRunner displays token usage after truncated output when available
  command: cargo make uat
  uat_status: verified
- id: uat-002
  name: Token usage is omitted when runner does not provide metrics
  command: cargo make uat
  uat_status: verified
tasks:
- id: T-001
  title: Extend RunnerOutput to include optional usage metadata
  priority: 1
  status: done
  notes: Add a struct like `UsageInfo` with optional fields for input_tokens, output_tokens, total_tokens, etc.
- id: T-002
  title: Parse token usage from Copilot CLI output in CopilotRunner
  priority: 2
  status: done
  notes: Investigate what usage info Copilot CLI emits (likely in stderr or special format). Extract and populate UsageInfo.
- id: T-003
  title: Display usage info in stdout after truncated LLM output
  priority: 3
  status: done
  notes: Only display if usage info is present. Format should be concise and readable.
- id: T-004
  title: Ensure runners without usage info omit the usage display
  priority: 4
  status: done
  notes: Default behavior should be to omit usage output when UsageInfo is None.
---

# Summary

Add support for displaying token usage information (input tokens, output tokens, etc.) from the underlying agent during `mr run` iterations. This will be implemented for CopilotRunner initially, with the architecture allowing future runners to provide their own usage metrics.

---

# Problem

When running `mr run`, users see truncated LLM output but have no visibility into token consumption. This makes it difficult to understand resource usage, monitor costs, or debug issues related to context limits. The Copilot CLI may emit usage information that is currently being ignored.

---

# Goals

1. Capture available token usage metrics from CopilotRunner output
2. Display usage information in stdout immediately after the truncated LLM output
3. Design the system so future runners can easily provide their own usage metrics
4. Gracefully handle runners that don't provide usage information by omitting the display

---

# Non-Goals (MVP)

- Persisting token usage to PRD history or log files
- Cost estimation or budget tracking
- Support for runners other than CopilotRunner
- Aggregating usage across multiple iterations
- Rate limit information display

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-24 — T-001 Completed
- **Task**: Extend RunnerOutput to include optional usage metadata
- **Status**: ✅ Done
- **Changes**:
  - Added `UsageInfo` struct in `src/runner/types.rs` with optional fields for `input_tokens`, `output_tokens`, and `total_tokens`
  - Extended `RunnerOutput` to include optional `usage: Option<UsageInfo>` field
  - Added `with_usage()` builder method to `RunnerOutput`
  - Added `has_data()` method to `UsageInfo` to check if any usage information is present
  - Updated `CopilotRunner` to parse token usage from CLI output using regex patterns
  - Added `parse_usage()` method to `CopilotRunner` that supports multiple common token usage output formats
  - Extended `RunResult::TaskExecuted` to include `usage: Option<UsageInfo>` field
  - Updated display logic in `main.rs` to show token usage in dim color after runner output
  - Usage info is displayed inline (e.g., "Token usage: Input: 123, Output: 456, Total: 579") when available
  - UAT passes: All 256 tests pass, CI pipeline (fmt, clippy, test) passes

## 2026-01-24 — T-002 Completed
- **Task**: Parse token usage from Copilot CLI output in CopilotRunner
- **Status**: ✅ Done
- **Changes**:
  - Investigated Copilot CLI output format and discovered usage stats are emitted in non-silent mode
  - Updated `parse_usage()` method to correctly parse Copilot CLI's actual format: "18.3k in, 38 out"
  - Added support for k/M suffixes (e.g., "18.3k" = 18,300 tokens, "1.2M" = 1,200,000 tokens)
  - Changed default silent mode from `true` to `false` in `CopilotConfig` to enable usage tracking
  - Added `strip_stats()` method to remove the statistics section from output while preserving the actual response
  - Updated both `execute()` and `execute_streaming()` to parse from combined stdout+stderr and strip stats
  - Updated `parse_usage()` to compute `total_tokens` as input + output when both are available
  - Added comprehensive unit tests for `parse_usage()` and `strip_stats()` functions
  - Fixed all tests to expect silent mode disabled by default
  - UAT passes: All 261 tests pass, CI pipeline (fmt, clippy, test) passes

## 2026-01-24 — T-003 Completed
- **Task**: Display usage info in stdout after truncated LLM output
- **Status**: ✅ Done
- **Changes**:
  - Added display logic in `src/main.rs` (lines 1046-1076) to conditionally render token usage
  - Usage info is displayed only when `usage` is present and `has_data()` returns true
  - Format: "Token usage:" header followed by inline display (e.g., "Input: 18300, Output: 38, Total: 18338")
  - All output uses `colors::dim()` for consistent secondary text styling
  - Gracefully degrades when no usage info is available (nothing displayed)
  - UAT passes: All 261 tests pass, CI pipeline (fmt, clippy, test) passes
  - **UAT Verified**: uat-001 (CopilotRunner displays token usage) - verified via implementation review and test suite

## 2026-01-24 — T-004 Completed
- **Task**: Ensure runners without usage info omit the usage display
- **Status**: ✅ Done
- **Changes**:
  - Verified that `RunnerOutput::success()` already sets `usage: None` by default
  - Confirmed display logic in `src/main.rs` (lines 1047-1076) properly checks for `Some(usage_info) && usage_info.has_data()` before displaying
  - MockRunner already uses `RunnerOutput::success()` exclusively, ensuring no usage info is emitted
  - Added explicit test `test_mock_runner_omits_usage_info` in `src/runner/mock.rs` to document this behavior
  - UAT passes: All 262 tests pass (including new test), CI pipeline (fmt, clippy, test) passes
  - **UAT Verified**: uat-002 (Token usage omitted when runner lacks metrics) - verified via new unit test and implementation review

## 2026-01-24 — PRD Finalized
- **Status**: ✅ Finalized
- **Outcome**: All 4 tasks completed, 2 acceptance tests verified
- **Tests**: 262/262 tests passed (cargo make uat)
- **Changelog**: Entry added under [Unreleased] → Added
- **Cleanup**: No temporary files or debug statements found

---
## 2026-01-24 — Finalization Complete
- **PRD**: PRD-0007 — Output Underlying Agent Usage
- **Finalized**: 2026-01-24T22:05:56Z
- **Tasks Completed**: 4
- **Summary**:
  - T-001: Extend RunnerOutput to include optional usage metadata
  - T-002: Parse token usage from Copilot CLI output in CopilotRunner
  - T-003: Display usage info in stdout after truncated LLM output
  - T-004: Ensure runners without usage info omit the usage display
- **Status**: ✅ All acceptance tests passed
