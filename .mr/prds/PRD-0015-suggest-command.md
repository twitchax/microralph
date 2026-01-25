---
id: PRD-0015
title: "Suggest Command for AI-Generated PRD Recommendations"
status: active
owner: "microralph"
created: 2026-01-24
updated: 2026-01-25

principles:
- Focus on strategic feature suggestions but include quick wins
- Leverage both internal codebase analysis and external research
- Maintain consistency with existing PRD workflow patterns
- Provide actionable, scoped suggestions that fit the PRD format

references:
- name: PRD-0003 PRD New Allows Upfront Context
  url: .mr/prds/PRD-0003-prd-new-allows-upfront-context.md
- name: PRD-0009 CLI Ergonomics Improvements
  url: .mr/prds/PRD-0009-cli-ergonomics-improvements.md

acceptance_tests:
- id: uat-001
  name: Suggest command generates exactly 5 PRD suggestions
  command: cargo make uat
  uat_status: unverified
- id: uat-002
  name: User can select a suggestion by number
  command: cargo make uat
  uat_status: unverified
- id: uat-003
  name: Selected suggestion flows into mr new with pre-filled context
  command: cargo make uat
  uat_status: unverified
- id: uat-004
  name: Suggestions include both strategic and quick-win categories
  command: cargo make uat
  uat_status: unverified
- id: uat-005
  name: Codebase analysis covers tech debt and dependency versions
  command: cargo make uat
  uat_status: unverified

tasks:
- id: T-001
  title: Add PromptKind variant for suggestion generation
  priority: 1
  status: done
  notes: Add SuggestGenerate variant to PromptKind enum in src/prompt/types.rs
- id: T-002
  title: Create suggest generation prompt template
  priority: 2
  status: todo
  notes: Create .mr/prompts/suggest-generate.md with instructions for analyzing codebase, existing PRDs, and conducting research
- id: T-003
  title: Implement top-level suggest command in CLI
  priority: 3
  status: todo
  notes: Add Suggest subcommand to main.rs at top level, similar to List/Edit/Finalize
- id: T-004
  title: Create suggest module with analysis logic
  priority: 4
  status: todo
  notes: Create src/suggest.rs that reads PRDs, analyzes codebase patterns, and invokes runner
- id: T-005
  title: Implement interactive numbered picker
  priority: 5
  status: todo
  notes: Parse runner output for 5 suggestions, display numbered list, read user input
- id: T-006
  title: Integrate suggestion selection with mr new flow
  priority: 6
  status: todo
  notes: Pass selected suggestion context to prd_new module using existing context mechanism
- id: T-007
  title: Add comprehensive UAT tests
  priority: 7
  status: todo
  notes: Create integration tests verifying suggestion generation, selection, and flow into new
- id: T-008
  title: Update AGENTS.md and README with suggest command
  priority: 8
  status: todo
  notes: Document new command usage and behavior in both files

---

# Summary

Add a `mr suggest` command that uses AI to analyze the codebase, existing PRDs (especially completed ones), and external research sources to generate five strategic PRD suggestions. Users select a suggestion via a numbered picker, which then flows directly into `mr new` with pre-filled context, streamlining the PRD creation workflow for both strategic features and quick improvements.

---

# Problem

Currently, users must manually identify opportunities for new PRDs by reviewing the codebase, researching best practices, and evaluating technical debt. This manual process is time-consuming and may miss opportunities that emerge from patterns across completed PRDs or external ecosystem developments. There's no systematic way to leverage AI's analytical capabilities to proactively suggest improvements based on holistic codebase analysis.

---

# Goals

1. Enable AI-driven analysis of codebase, completed PRDs, and external research to identify improvement opportunities
2. Present exactly 5 actionable PRD suggestions to users via numbered selection
3. Support seamless flow from suggestion selection into `mr new` with pre-filled context
4. Balance strategic feature suggestions with quick-win improvements
5. Analyze technical debt, dependency versions, TODO comments, test coverage, and ecosystem best practices
6. Integrate naturally with existing CLI command structure following PRD-0009 patterns

---

# Non-Goals (MVP)

- Automatic PRD creation without user review
- Configurable number of suggestions (fixed at 5)
- Filtering suggestions by category or priority
- Caching or persisting suggestions across sessions
- Integration with issue trackers or project management tools
- Multi-repo analysis or cross-project suggestions

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-25 — T-001 Completed
- **Task**: Add PromptKind variant for suggestion generation
- **Status**: ✅ Done
- **Changes**:
  - Added `SuggestGenerate` variant to `PromptKind` enum in `src/prompt/types.rs`
  - Implemented `filename()` method to return `"suggest_generate.md"`
  - Added variant to `all()` array in `types.rs`
  - Created `PROMPT_SUGGEST_GENERATE` constant in `src/init.rs` with comprehensive prompt template
  - Added match arm in `src/prompt/loader.rs` to return the new prompt
  - Updated test `test_prompt_kind_all` to expect 16 prompts (previously 15)
  - Updated test `test_prompt_loader_missing_prompts` to expect 16/15 counts
  - UAT passed: All 293 tests passing

---