# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- PRD-0002: Complete release infrastructure with multi-platform builds (Linux, macOS, Windows, WASM), code coverage via Codecov, cargo-release for versioning, git-cliff for changelog generation, crates.io publishing, and GitHub Releases with binary artifacts
- PRD-0014: Cleanup Pass - Extracted duplicated Q/A workflow patterns, reduced unnecessary clones, added comprehensive comments to parsing logic and state machines, aligned code with Rust idioms, and fixed performance issues
- PRD-0017: Add `mr restore` command to overwrite `.mr/prompts/` and `.mr/templates/` with built-in defaults. Reuses `mr init` logic for DRY, leaves changes uncommitted for Git review, and supports idempotent restoration.

### 🚀 Features

- Implement static prompt library + placeholder system (T-005)
- *T-006, T-007*: Implement `mr prd new` guided Q/A + Runner abstraction
- *T-008*: Implement CopilotRunner adapter
- *T-009*: Implement `mr run` command
- *T-010*: Implement `mr status` command
- *T-011*: AGENTS.md updater with safe, bounded patching
- *T-012*: Implement mr bootstrap command
- *T-013*: Remove allow flags and clean clippy lints
- *T-014*: Implement `mr prd edit` for quick PRD modifications
- *T-016*: Add --language flag to init and bootstrap commands
- *T-017*: Document placeholder variables for each prompt in README
- *T-018*: Add .mr/config.toml for persistent settings
- *T-019*: Stream runner output during mr run
- *T-020*: Add reindex command for regenerating PRD index and fixing links
- *T-099*: Revamp docs, add DEVELOPMENT.md, comparison table
- *T-001*: Add mr prd finalize CLI command
- *T-002*: Implement task completion validation for PRD finalization
- *T-003*: Run acceptance test verification via finalization prompt

### 🐛 Bug Fixes

- *finalize*: Enable finalization summary report output

### 📋 PRD Tasks

- Prd(PRD-0004)feat(T-004): add changelog.rs module for Keep a Changelog support
- Prd(PRD-0004)feat(T-005): Add changelog entry generation to finalization prompt
- Prd(PRD-0004)feat(T-006): Generate summary report (append to PRD + stdout)

- Added generate_summary_report() function for formatted finalization reports
- Added append_to_prd() function to append history entries to PRD files
- Extended PrdFinalizeResult with summary_report field
- Enhanced cmd_prd_finalize output with visual formatting
- Added unit tests for summary report generation and file append
- UAT passes: 217/217 tests
- Prd(PRD-0004)feat(T-007): Update PRD status to done and refresh PRDS.md index

- Made serialize_prd() public in prd/parser.rs
- Added update_prd_status_to_done() function to set PRD status to done
- Integrated status update and index regeneration into finalize_prd()
- Added output messages for status update and index regeneration
- Added unit tests for status update functionality
- UAT passes: 219/219 tests
- Prd(PRD-0004)feat(T-010): Verify finalization history entry appending

T-010 functionality was already implemented by T-006:
- generate_summary_report() creates formatted history entry
- append_to_prd() appends entry to PRD file
- Entry includes date, timestamp, PRD info, task list, outcome

No code changes required; verified complete with 219/219 tests passing.
- Prd(PRD-0004)feat(T-011): Comprehensive finalization prompt instructions

Updated run_task_finalize.md and init.rs PROMPT_RUN_TASK_FINALIZE with:
- 6 numbered sections matching Design Notes requirements
- Detailed acceptance test verification instructions
- Keep a Changelog format and category guidelines
- Summary report format template
- Cleanup guidance for temp files and excessive comments
- Inter-PRD link update instructions
- Finalization history entry format
- Documentation check section (README, AGENTS, inline docs)
- Example output format for completion confirmation
- Prd(PRD-0004)feat(T-008): Add inter-PRD cross-references to index

- Add extract_prd_references() to scan PRD body and task notes for PRD-XXXX patterns
- Add references field to PrdSummary struct
- Add Cross-References section to generated PRDS.md index
- Add 7 new unit tests for reference extraction
- Update existing tests for new references field
- Prd(PRD-0004)feat(T-009): Verify cleanup tasks in finalization prompt
- Prd(PRD-0004)finalize: PRD Finalization Steps complete
- Prd(PRD-0003)feat(T-001): add --context CLI flag to prd new command
- Prd(PRD-0003)feat(T-002): add interactive context prompt before question generation
- Prd(PRD-0003)feat(T-003): add user_context placeholder to round1 prompt
- Prd(PRD-0003)feat(T-004): persist context through all Q/A rounds

- Modified build_round_n_prompt to accept user_context parameter
- Updated create_prd to pass context to round N prompts
- Added {{#if user_context}} block to prd_new_roundN_questions.md
- Context now flows through all Q/A rounds, not just round 1
- All 227 tests pass
- Prd(PRD-0003)feat(T-005): Include context in final PRD synthesis prompt
- Prd(PRD-0003)feat(T-006): update help text and documentation for --context flag
- Prd(PRD-0004)finalize: PRD Finalization Steps complete
- Prd(PRD-0005)feat(T-001): Add unverified UAT check to run loop

- Add acceptance_tests(), all_tasks_done(), has_unverified_uats(), and
  unverified_uats() methods to Prd struct
- Change RunResult from struct to enum with TaskExecuted,
  NeedsUatVerification, and PrdComplete variants
- Update run_task() to detect when all tasks done but UATs unverified
- Update main.rs run loop to handle new RunResult variants
- Add 3 new unit tests for UAT verification detection
- All 230 tests passing
- Prd(PRD-0005)feat(T-002): Create UAT verification prompt template
- Prd(PRD-0005)feat(T-003): Implement UAT verification loop in run.rs

- Add run_uat_verification_loop() function that iterates over unverified UATs
- Add UatVerificationConfig and UatVerificationLoopResult structs
- Add build_uat_verify_prompt() helper for verification prompts
- Add parse_opt_out() to detect OPT-OUT responses from runner
- Wire up verification loop in main.rs when NeedsUatVerification returned
- Loop respects max_iterations from PRD config (default: 10)
- Add 4 new unit tests for verification loop functionality

UAT passed: 238 tests, all passed
- Prd(PRD-0005)feat(T-004): Add model opt-out mechanism with History entry
- Prd(PRD-0005)feat(T-005): Block finalization on unverified UATs

- Added UnverifiedUats variant to FinalizeError enum
- Added validate_all_uats_verified() check in finalize_prd()
- Added 5 unit tests for UAT validation
- UAT passed: 244 tests
- Prd(PRD-0005)feat(T-006): Update run_task.md prompt to reference UAT verification phase
- Prd(PRD-0005)feat(T-007): Add UAT status update logic

- Add update_uat_status() function to programmatically set UAT status to verified
- Modify run_uat_verification_loop() to auto-update UAT status when runner succeeds
- Update test assertions for new behavior (runner success → verified count)
- Add unit tests for update_uat_status() function

246 tests pass.
- Prd(PRD-0005)feat(T-008): Add integration test for UAT verification loop
- Prd(PRD-0005)uat(uat-001): verify UAT verification loop triggers after tasks done
- Prd(PRD-0005)uat(uat-002): Verified loop addresses unverified UATs
- Prd(PRD-0005)uat(uat-003): Verify opt-out mechanism with existing tests
- Prd(PRD-0005)uat(uat-004): verify max_iterations respected
- Prd(PRD-0005)uat(uat-005): Verify unverified UATs block finalization
- Prd(PRD-0005)uat(uat-006): Verify update_uat_status updates PRD frontmatter
- Prd(PRD-0005)uat(uat-007): Verify UAT verification results appended to History

Added test_uat_verification_history_appending to verify that:
- Opt-out History entries are automatically appended via append_opt_out_history()
- Successful verification History entries are manually appended by the runner
- This behavior is by design: runners have full context to write meaningful entries

Test: cargo make uat test_uat_verification_history_appending
Result: All 249 tests passed
- Prd(PRD-0005)finalize: Verify UATs at End of Run Loop
- Prd(PRD-0003)uat(uat-001): verify interactive context prompt flow
- Prd(PRD-0003)uat(uat-002): Verify context flag flow with new test
- Prd(PRD-0003)uat(uat-003): verify context influences question generation
- Prd(PRD-0003)uat(uat-004): verify context persists through Q/A rounds
- Prd(PRD-0003)uat(uat-005): verify context included in final synthesis
- Prd(PRD-0003)finalize: PRD New Allows Upfront Context
- Prd(PRD-0006)feat(T-001): Add owo-colors dependency and create color utilities module
- Prd(PRD-0006)feat(T-002): Colorize success messages with green and emoji prefixes
- Prd(PRD-0006)feat(T-003): Colorize error and warning messages with red/yellow and emoji prefixes
- Prd(PRD-0006)feat(T-004): Style question prompts with blue bold text and emoji prefix
- Prd(PRD-0006)feat(T-005): Add color to informational and status messages
- Prd(PRD-0006)feat(T-006): Colorize finalization summary box and separators
- Prd(PRD-0006)uat(uat-001): verify success messages display green with emoji
- Prd(PRD-0006)uat(uat-003): verify question prompts display in blue with bold text
- Prd(PRD-0006)uat(uat-004): verify colors disabled when piped
- Prd(PRD-0006)uat(uat-005): verify emoji preservation in prd list output
- Prd(PRD-0006)uat(uat-006): verify finalization summary box styling with unit test
- Prd(PRD-0006)uat(uat-007): verify NO_COLOR disables colors
- Prd(PRD-0006)finalize: Add Stdout Colors with terminal colorization
- Prd(PRD-0007)feat(T-001): Extend RunnerOutput with optional usage metadata

- Added UsageInfo struct with optional input_tokens, output_tokens, and total_tokens fields
- Extended RunnerOutput to include optional usage field
- Implemented token usage parsing in CopilotRunner using regex patterns
- Added display logic in main.rs to show usage info after runner output
- Usage information displayed in dim color when available
- All tests pass (256/256), CI passes
- Prd(PRD-0007)feat(T-002): Parse token usage from Copilot CLI output

- Investigated Copilot CLI output format and discovered stats are emitted in non-silent mode
- Updated parse_usage() to correctly parse Copilot format: '18.3k in, 38 out'
- Added support for k/M suffixes (18.3k = 18,300, 1.2M = 1,200,000)
- Changed default silent mode from true to false to enable usage tracking
- Added strip_stats() to remove statistics section while preserving response
- Updated both execute() and execute_streaming() to parse and strip stats
- Added comprehensive unit tests for parsing and stripping functions
- All 261 tests pass, CI pipeline passes
- Prd(PRD-0007)feat(T-003): Display usage info after LLM output
- Prd(PRD-0007)feat(T-004): ensure runners without usage info omit display

- Added test to verify MockRunner omits usage info
- Confirmed existing implementation already handles this correctly
- Updated PRD status and verified UAT-002
- Prd(PRD-0007)finalize: Output Underlying Agent Usage
- Prd(PRD-0009)feat(T-001): Remove --prd flag, add positional PRD arg to run command
- Prd(PRD-0009)feat(T-002): Flatten CLI command structure by removing Prd subcommand
- Prd(PRD-0009)feat(T-003): Update code references to use flattened CLI commands
- Prd(PRD-0009)feat(T-004): Add tracing info for runner invocations

Added tracing::info! calls at all runner invocation points across the
codebase to improve debugging visibility. Each invocation now logs:
- Runner name
- Relevant IDs (PRD, task, slug, etc.)
- Stream mode status
- Prompt length

Changed existing debug-level logging to info-level where appropriate.
All 262 tests passed.
- Prd(PRD-0009)feat(T-005): Show tail of LLM output instead of beginning for better debugging
- Prd(PRD-0009)feat(T-006): Apply tail output to UAT verification loop
- Prd(PRD-0009)feat(T-007): Update documentation for new CLI structure

- Updated README.md with flattened CLI commands (mr new/list/edit vs mr prd new/list/edit)
- Changed mr run --prd <id> to mr run <id> (positional argument)
- Updated AGENTS.md auto-managed section with comprehensive CLI reference
- All 263 tests passing
- Prd(PRD-0009)feat(T-008): Verify all tests pass after CLI ergonomics improvements

- Executed cargo make ci: All 263 tests passed (fmt, clippy, unit tests)
- Executed cargo make uat: All 263 tests passed (acceptance tests)
- No regressions detected from CLI structure changes (T-001 through T-007)
- All CLI ergonomics improvements verified and working correctly
- Prd(PRD-0009)uat(uat-001): verify run command accepts positional PRD argument
- Prd(PRD-0009)uat(uat-002): verify interactive mode test
- Prd(PRD-0009)uat(uat-003): verify list command with new test
- Prd(PRD-0009)uat(uat-004): verify top-level new command works
- Prd(PRD-0009)uat(uat-005): verify top-level edit command
- Prd(PRD-0001)finalize: Complete microralph MVP build with full feature set
- Prd(PRD-0009)uat(uat-006): verify finalize command with test
- Prd(PRD-0009)finalize: CLI Ergonomics Improvements - flattened command structure and improved output readability
- Prd(PRD-0009)finalize: Update prompts to reflect new CLI structure
- Prd(PRD-0009)finalize: Update hardcoded prompt constants in init.rs
- Prd(PRD-0009)finalize: Sync remaining prompts with new CLI structure
- Prd(PRD-0009)finalize: Update DEVELOPMENT.md with new CLI syntax
- Prd(PRD-0010)feat(T-001): detect and display multi-line questions
- Prd(PRD-0010)test(T-001): add multi-line question test and verify uat-001
- Prd(PRD-0010)feat(T-002): Mark multi-line answer input as done

Multi-line answer input was already implemented in collect_answers() function.
Users can now:
- Type multi-line answers by pressing Enter after each line
- Finish by pressing Enter on a blank line (double-enter pattern)
- All lines are joined with newlines and stored

Updated PRD status:
- T-002: marked as done
- UAT-002: marked as verified
- Added History entry documenting completion

All 273 UAT tests pass.
- Prd(PRD-0010)feat(T-003): Verify UAT scenarios pass - all 273 tests passing
- Prd(PRD-0010)finalize: Support multi-line Q/A during PRD creation
- Prd(PRD-0012)feat(T-001): Add constitution template with numbered examples
- Prd(PRD-0012)feat(T-002): emit constitution during bootstrap

- Constitution already emitted via existing init() call in bootstrap()
- Added test test_bootstrap_creates_constitution to verify constitution creation
- Test confirms constitution.md is created when bootstrap initializes .mr/ structure
- Updated PRD-0012 task T-002 status to done
- Regenerated PRD index
- All 275 tests pass
- *PRD-0012*: Document UAT verification status for T-002
- Prd(PRD-0012)feat(T-003): Add constitution edit subcommand

- Created src/constitution_edit.rs module with edit_constitution() function
- Added ConstitutionEdit prompt kind and template
- Implemented 'mr constitution edit <request>' CLI command
- Supports Q/A flow with max 3 rounds before forcing application
- Runner uses READY_TO_APPLY signal to indicate completion
- All 277 tests passed
- Prd(PRD-0010)finalize: Support Multi-line Q/A During PRD Creation
- Prd(PRD-0012)feat(T-004): Load constitution in runner context
- Prd(PRD-0012)feat(T-005): Include constitution in prd new prompts
- Prd(PRD-0012)feat(T-006): Include constitution in prd finalize prompts
- Prd(PRD-0012)feat(T-007): Add constitution violation logging to runner prompts
- Prd(PRD-0012)feat(T-008): Document constitution feature in README
- Prd(PRD-0012)uat(uat-001): Verify bootstrap creates constitution
- Prd(PRD-0012)uat(uat-002): Verified constitution template contains numbered example rules
- Prd(PRD-0012)uat(uat-003): verify prd new reads constitution
- Prd(PRD-0012)uat(uat-004): verify prd finalize reads constitution
- Prd(PRD-0012)uat(uat-005): verify constitution edit command with integration test
- Prd(PRD-0012)uat(uat-006): verify runner logs constitution violations in prompts
- Prd(PRD-0012)finalize: Enable Constitution feature with LLM-assisted editing
- Prd(PRD-0016)feat(T-001): Remove update_agents_md() call from run.rs
- Prd(PRD-0016)feat(T-002): Remove update_agents_md() call from prd_new.rs
- Prd(PRD-0016)feat(T-003): Remove update_agents_md() call from bootstrap.rs
- Prd(PRD-0016)feat(T-007): Add AGENTS.md update reminder to run_task.md prompt
- Prd(PRD-0016)feat(T-008): Add AGENTS.md update reminder to PRD synthesis prompt
- Prd(PRD-0016)feat(T-009): Add AGENTS.md update reminder to bootstrap prompt
- Prd(PRD-0016)feat(T-004): Delete unused agents.rs module

- Removed mod agents declaration from src/main.rs
- Deleted src/agents.rs file (531 lines)
- Module was completely unused after T-001, T-002, T-003 removed all update_agents_md() calls
- Updated PRD-0016 task T-004 status to done and added History entry
- All UATs pass
- Prd(PRD-0016)feat(T-005): Remove update_agents.md prompt creation

- Removed PROMPT_UPDATE_AGENTS constant and file creation logic from init.rs
- Removed PromptKind::UpdateAgents enum variant and all references
- Replaced auto-managed section in STARTER_AGENTS with manual update guidance
- Updated all related tests to reflect new prompt count (14 instead of 15)
- All 267 tests pass
- Prd(PRD-0016)feat(T-006): Delete update_agents.md prompt file
- Prd(PRD-0016)feat(T-010): Verify prompt consistency between init.rs and .md files

- Updated PROMPT_RUN_TASK constant in src/init.rs to add AGENTS.md update reminder (step 7)
- Verified all 14 prompts match between init.rs constants and .mr/prompts/*.md files
- Confirmed AGENTS.md update reminders are present in run_task, prd_new_synthesize_prd, and bootstrap_generate_prds prompts
- All 267 tests pass
- Prd(PRD-0016)uat(uat-001): Verified all UATs pass after AGENTS.md automation removal
- Prd(PRD-0016)uat(uat-002): verify task execution without update_agents_md call
- Prd(PRD-0016)finalize: Remove automatic AGENTS.md update step
- Prd(PRD-0011)feat(T-001): Add dev container documentation to README
- Prd(PRD-0011)feat(T-002): Implement dev container detection utility
- Prd(PRD-0011)feat(T-003): Add dev container warning to model-invoking commands
- Prd(PRD-0011)feat(T-004): Implement mr devcontainer generate command

- Added DevcontainerGenerate prompt kind and default template
- Created new Devcontainer CLI subcommand with Generate command
- Implemented cmd_devcontainer_generate() with repo analysis
- Analyzes repo files, git history, and tools for context
- Generates .devcontainer/devcontainer.json via runner
- Extracts JSON from markdown-wrapped responses
- Also completes T-005 (repo analysis), T-006 (prompt template)
- All 270 tests pass with cargo make uat
- Prd(PRD-0011)feat(T-005): Update PRD status for repo analysis module

T-005 was already implemented in T-004 via analyze_repo_for_devcontainer().
This commit updates the PRD to reflect the actual completion status.
- Prd(PRD-0011)feat(T-006): Verify and document dev container prompt template completion
- Prd(PRD-0011)feat(T-007): Add unit test for devcontainer generate with mock runner

- Added serde_json dependency for JSON validation
- Refactored cmd_devcontainer_generate to extract testable generate_devcontainer_config function
- Created comprehensive unit test with MockRunner and temporary directory
- Test verifies valid JSON generation and file creation
- Opportunistically verified UAT-001 (devcontainer generate test)
- All 271 tests pass
- Prd(PRD-0011)finalize: Dev Container Support and Generation
- Prd(PRD-0008)feat(T-001): Disable rust-cache bin caching to fix cargo-make availability
- Prd(PRD-0008)feat(T-002): Verify CI cargo-make fix complete
- Prd(PRD-0008)uat(uat-001): verified CI passes with all tests
- Prd(PRD-0008)finalize: Fix CI cargo-make availability with rust-cache
- Prd(PRD-0013)feat(T-001): Add ClaudeRunner with full CLI support

- Created src/runner/claude.rs implementing ClaudeRunner
- Mirrors CopilotRunner surface area with Claude-specific flags
- Supports -p for non-interactive mode
- Supports --dangerously-skip-permissions for yolo mode
- Supports --model for model selection
- Supports --no-ask-user for autonomous operation
- Token usage not available in Claude CLI stdout (noted in code)
- Added create_runner() helper in main.rs for centralized runner creation
- Updated all runner instantiation sites to support claude runner
- Updated runner module exports
- All 283 UAT tests pass
- Prd(PRD-0013)feat(T-002): ClaudeConfig struct verified complete
- Prd(PRD-0013)feat(T-003): Complete ClaudeRunner Runner trait implementation

- Verified ClaudeRunner struct fully implements Runner trait with all required methods
- name() returns 'claude'
- execute() runs Claude CLI non-streaming
- execute_streaming() provides real-time output
- is_available() checks Claude CLI installation
- format_command_display() formats command for user display
- Implementation mirrors CopilotRunner pattern exactly
- All 283 tests pass (cargo make uat)
- Prd(PRD-0013)feat(T-004): Implement build_args method for Claude CLI flags
- Prd(PRD-0013)feat(T-005): Implement token usage parsing for Claude CLI

- Implemented JSON output format support via --output-format json
- Fixed invalid --no-ask-user flag to use --permission-mode dontAsk
- Added parse_usage() to extract input/output tokens from JSON response
- Added extract_result_from_json() to extract response text from JSON
- Updated execute() and execute_streaming() to parse JSON and extract usage
- Added comprehensive tests for token usage parsing and JSON extraction
- All 288 tests pass, UAT successful
- *PRD-0013*: Verify all UATs opportunistically after T-005
- Prd(PRD-0013)feat(T-006): Implement output stripping for Claude CLI

- Added public strip_usage_stats() method to ClaudeRunner
- Mirrors CopilotRunner::strip_usage_stats() API for consistency
- Leverages Claude CLI's --output-format json for clean metadata separation
- Extracts only the 'result' field, automatically stripping usage/type/session metadata
- Added comprehensive unit tests for all edge cases
- All 293 tests pass, cargo make uat successful
- Prd(PRD-0013)feat(T-007): verify comprehensive unit test coverage for ClaudeRunner
- Prd(PRD-0013)feat(T-008): Export ClaudeRunner from runner module
- Prd(PRD-0013)feat(T-009): Document ClaudeRunner implementation patterns in AGENTS.md
- Prd(PRD-0013)finalize: Complete Claude CLI runner implementation
- Prd(PRD-0015)feat(T-001): Add SuggestGenerate PromptKind variant

- Added SuggestGenerate variant to PromptKind enum
- Created PROMPT_SUGGEST_GENERATE constant with comprehensive template
- Updated prompt loader to handle new variant
- Updated tests to reflect new prompt count (16 instead of 15)
- All 293 tests passing
- Prd(PRD-0015)feat(T-002): Create suggest_generate.md prompt template

- Added create_file_if_missing call in src/init.rs for suggest_generate.md
- Created .mr/prompts/suggest_generate.md with comprehensive prompt template
- Updated init tests to expect 19 files (14 prompts) instead of 18
- Added assertion for suggest_generate.md in test_init_creates_structure
- All 293 tests passing
- Prd(PRD-0015)feat(T-003): Add top-level suggest command to CLI
- Prd(PRD-0015)feat(T-004,T-005,T-006): Implement suggest module with codebase analysis, numbered picker, and PRD integration
- Prd(PRD-0015)feat(T-007): Add comprehensive UAT tests for suggest command
- Prd(PRD-0015)feat(T-008): Document mr suggest command in AGENTS.md and README.md
- Prd(PRD-0015)uat(uat-001): verify suggest parses exactly 5 suggestions
- Prd(PRD-0015)uat(uat-002): Add validate_selection test for numbered picker
- Prd(PRD-0015)uat(uat-003): verify suggestion flows to mr new with pre-filled context
- Prd(PRD-0015)uat(uat-004): Verify suggestions include strategic and quick-win categories
- Prd(PRD-0015)uat(uat-005): Verify codebase analysis covers tech debt and dependencies
- Prd(PRD-0015)finalize: suggest command for AI-generated PRD recommendations
- Prd(PRD-0002)feat(T-001): Verify code coverage infrastructure complete
- Prd(PRD-0002)feat(T-002): Add WASM build support to complete cross-platform CI coverage

- Added build-wasm task to Makefile.toml for wasm32-wasip2 target
- Added build_wasm CI job to build.yml with artifact upload
- All four target platforms now supported: Linux x86_64, macOS ARM, Windows x86_64, WASM32-WASIP2
- All build jobs conditional on main branch
- Verified UAT-002 (Linux builds) and UAT-005 (WASM builds) pass
- Updated PRD status: T-002 marked as done, UAT-005 marked as verified
- Prd(PRD-0002)feat(T-003): Verify and document cargo-make build tasks for all target platforms
- Prd(PRD-0002)feat(T-004): Set up cargo-release for version management

- Added install-cargo-release task following kord pattern
- Updated release task to depend on install-cargo-release
- Changed from script-based to command-based implementation
- Verified UAT-007: cargo release dry-run works successfully
- All 304 tests pass
- Prd(PRD-0002)feat(T-005): add changelog generation with git-cliff

- Created cliff.toml with conventional commit support and Keep a Changelog format
- Added install-git-cliff task to Makefile.toml
- Added changelog task to Makefile.toml for generating CHANGELOG.md
- Verified UAT-006: cargo make changelog generates proper changelog output
- All 304 tests pass
- Prd(PRD-0002)feat(T-006): add publish-crates task for crates.io publishing
- Prd(PRD-0002)feat(T-007): Add GitHub Release creation task

- Added github-release cargo-make task that uses gh CLI
- Task accepts version tag and optional --draft flag
- Uses CHANGELOG.md for release notes
- Supports attaching binaries from release-artifacts/ directory
- Includes helpful instructions for downloading CI artifacts
- All 304 tests pass

### 📚 Documentation

- Add release workflow section to AGENTS.md

### ⚙️ Miscellaneous Tasks

- *prd*: Add T-018 for config.toml and --model flag support
- *prd*: Add T-019 for streaming runner output during mr run
- Stylize as lowercase microralph everywhere

<!-- generated by git-cliff -->
