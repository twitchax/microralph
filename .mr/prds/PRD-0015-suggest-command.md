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
  uat_status: verified
- id: uat-002
  name: User can select a suggestion by number
  command: cargo make uat
  uat_status: verified
- id: uat-003
  name: Selected suggestion flows into mr new with pre-filled context
  command: cargo make uat
  uat_status: verified
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
  status: done
  notes: Create .mr/prompts/suggest-generate.md with instructions for analyzing codebase, existing PRDs, and conducting research
- id: T-003
  title: Implement top-level suggest command in CLI
  priority: 3
  status: done
  notes: Add Suggest subcommand to main.rs at top level, similar to List/Edit/Finalize
- id: T-004
  title: Create suggest module with analysis logic
  priority: 4
  status: done
  notes: Create src/suggest.rs that reads PRDs, analyzes codebase patterns, and invokes runner
- id: T-005
  title: Implement interactive numbered picker
  priority: 5
  status: done
  notes: Parse runner output for 5 suggestions, display numbered list, read user input
- id: T-006
  title: Integrate suggestion selection with mr new flow
  priority: 6
  status: done
  notes: Pass selected suggestion context to prd_new module using existing context mechanism
- id: T-007
  title: Add comprehensive UAT tests
  priority: 7
  status: done
  notes: Create integration tests verifying suggestion generation, selection, and flow into new
- id: T-008
  title: Update AGENTS.md and README with suggest command
  priority: 8
  status: done
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

## 2026-01-25 — T-002 Completed
- **Task**: Create suggest generation prompt template
- **Status**: ✅ Done
- **Changes**:
  - Added `create_file_if_missing` call in `src/init.rs` to create `.mr/prompts/suggest_generate.md` during initialization
  - Inserted the file creation call after `pick_prd.md` and before the PRDS.md index creation
  - Created `.mr/prompts/suggest_generate.md` with comprehensive prompt template from `PROMPT_SUGGEST_GENERATE` constant
  - Updated test `test_init_creates_structure`: Added assertion for `suggest_generate.md` file existence, changed expected file count from 18 to 19 (14 prompts instead of 13)
  - Updated test `test_init_is_idempotent`: Changed expected file counts from 18 to 19 for both initialization runs
  - UAT passed: All 293 tests passing

---

## 2026-01-25 — T-003 Completed
- **Task**: Implement top-level suggest command in CLI
- **Status**: ✅ Done
- **Changes**:
  - Added `mod suggest;` declaration in `src/main.rs` to include new suggest module
  - Added `Suggest` subcommand to `Command` enum with `runner` and `model` parameters
  - Added match arm in main() to handle `Command::Suggest` and route to `cmd_suggest()`
  - Implemented `cmd_suggest()` function that initializes runner and calls `suggest::suggest()`
  - Created `src/suggest.rs` with placeholder `suggest()` function (implementation deferred to T-004)
  - Verified command is accessible via `mr suggest --help`
  - UAT passed: All 293 tests passing

---

## 2026-01-25 — T-004, T-005, T-006 Completed
- **Tasks**: Create suggest module with analysis logic, Implement interactive numbered picker, Integrate suggestion selection with mr new flow
- **Status**: ✅ Done
- **Changes**:
  - Implemented complete `suggest()` function in `src/suggest.rs` with full flow from analysis to PRD creation
  - Added `Suggestion` struct to represent parsed suggestions with number, title, description, category, effort, and rationale
  - Implemented `analyze_codebase()` to gather repository structure, tools/dependencies, recent git commits, and TODO comments
  - Created `build_suggestion_prompt()` to expand placeholders with existing PRDs and codebase snapshot
  - Implemented `parse_suggestions()` to extract exactly 5 suggestions from runner output with structured fields
  - Added interactive numbered picker that displays suggestions with formatting and prompts user for selection (1-5 or 'q')
  - Implemented `generate_slug()` utility to create URL-friendly slugs from suggestion titles
  - Integrated with `prd_new::create_prd()` to flow selected suggestion into PRD creation with pre-filled context
  - All three tasks (T-004, T-005, T-006) completed as single cohesive implementation
  - UAT passed: All 293 tests passing

---

## 2026-01-25 — T-007 Completed
- **Task**: Add comprehensive UAT tests
- **Status**: ✅ Done
- **Changes**:
  - Added comprehensive test suite to `src/suggest.rs` covering all core functionality
  - Created `test_suggest_parses_five_suggestions()` to verify parsing exactly 5 suggestions with correct structure
  - Added `test_parse_suggestions_empty_output()` and `test_parse_suggestions_incomplete()` to test error handling
  - Implemented `test_generate_slug()` to verify URL-friendly slug generation from various title formats
  - Created `test_analyze_codebase()` to verify repository analysis includes structure and tools detection
  - Added `test_build_suggestion_prompt()` to verify placeholder expansion with PRDs and codebase snapshot
  - Implemented `test_suggest_integration_with_mock_runner()` as integration test covering full suggestion workflow
  - Fixed UTF-8 byte boundary issue in `parse_suggestions()` when splitting on em dash separator
  - All 300 tests passing (up from 293)
  - UAT passed: `cargo make uat` completed successfully

---

## 2026-01-25 — T-008 Completed
- **Task**: Update AGENTS.md and README with suggest command
- **Status**: ✅ Done
- **Changes**:
  - Updated README.md: Added `mr suggest` to usage examples section before `mr new`
  - Updated README.md: Added `mr suggest` entry to Commands table with description of AI-powered suggestions
  - Updated AGENTS.md: Added new "Suggest Command Workflow" section documenting the 4-phase workflow (analysis, generation, selection, integration)
  - Documented that suggestions balance strategic features with quick wins
  - Referenced PRD-0009 compliance for CLI patterns
  - UAT passed: `cargo make uat` completed successfully with exit code 0

---

## 2026-01-25 — uat-001 Verification
- **UAT**: Suggest command generates exactly 5 PRD suggestions
- **Status**: ✅ Verified
- **Method**: Existing test
- **Details**:
  - Test: `src/suggest.rs::test_suggest_parses_five_suggestions()`
  - Verifies `parse_suggestions()` extracts exactly 5 suggestions with correct structure
  - Asserts `suggestions.len() == 5` and validates first and last suggestion fields
  - Test passed in full UAT run: 300/300 tests passing

---

## 2026-01-25 — uat-002 Verification
- **UAT**: User can select a suggestion by number
- **Status**: ✅ Verified
- **Method**: New test
- **Details**:
  - Created `validate_selection()` helper function in `src/suggest.rs` to extract selection validation logic
  - Function validates user input (1-5 or 'q'), converts to 0-based index, and handles errors
  - Added comprehensive test `test_validate_selection()` covering:
    - Valid selections (1-5) with correct 0-based index conversion
    - Whitespace trimming
    - Quit with 'q' or 'Q' (case-insensitive)
    - Out of range errors (0, 6)
    - Invalid input (non-numeric, empty)
  - Refactored `suggest()` function to use `validate_selection()` helper
  - Test passed: 301/301 tests passing (up from 300)

---

## 2026-01-25 — uat-003 Verification
- **UAT**: Selected suggestion flows into mr new with pre-filled context
- **Status**: ✅ Verified
- **Method**: New test
- **Details**:
  - Created `test_suggestion_flows_to_prd_new_with_context()` in `src/suggest.rs`
  - Test verifies that when a suggestion is selected:
    - A slug is generated from the suggestion title (e.g., "implement-configuration-validation")
    - Context is built with description, category, effort, and rationale
    - All fields are properly extracted and formatted for PrdNewConfig
  - Test parses sample suggestion output, selects suggestion #2, and validates all context fields
  - Test passed: 302/302 tests passing (up from 301)