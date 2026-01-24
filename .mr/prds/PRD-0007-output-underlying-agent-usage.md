---
id: PRD-0007
title: "Output Underlying Agent Usage"
status: draft
owner: "twitchax"
created: 2026-01-24
updated: 2026-01-24

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
  uat_status: unverified
- id: uat-002
  name: Token usage is omitted when runner does not provide metrics
  command: cargo make uat
  uat_status: unverified

tasks:
- id: T-001
  title: Extend RunnerOutput to include optional usage metadata
  priority: 1
  status: todo
  notes: Add a struct like `UsageInfo` with optional fields for input_tokens, output_tokens, total_tokens, etc.
- id: T-002
  title: Parse token usage from Copilot CLI output in CopilotRunner
  priority: 2
  status: todo
  notes: Investigate what usage info Copilot CLI emits (likely in stderr or special format). Extract and populate UsageInfo.
- id: T-003
  title: Display usage info in stdout after truncated LLM output
  priority: 3
  status: todo
  notes: Only display if usage info is present. Format should be concise and readable.
- id: T-004
  title: Ensure runners without usage info omit the usage display
  priority: 4
  status: todo
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

---