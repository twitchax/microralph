---
id: PRD-0019
title: Tighten Prompts and Encourage Constitution
status: done
owner: twitchax
created: 2026-01-26
updated: 2026-01-26
depends_on: ["PRD-0012"]
principles:
- Prompts should be workflow-focused (what to do) without philosophy
- Constitution should contain all behavioral guidance (how to behave)
- Users should edit only the constitution to change overall agent behavior
- Reduce duplication of behavioral rules across multiple prompt files
references: []
acceptance_tests:
- id: uat-001
  name: All scattered "don't modify unrelated code" instances removed from prompts
  command: cargo make uat
  uat_status: verified
- id: uat-002
  name: Constitution has comprehensive behavior rules (under 10 rules)
  command: cargo make uat
  uat_status: verified
- id: uat-003
  name: Prompts are workflow-focused with no philosophy or opinion
  command: cargo make uat
  uat_status: verified
tasks:
- id: T-001
  title: Audit all 16 prompt files for behavioral/philosophical guidance
  priority: 1
  status: done
  notes: Identify all instances of "don't modify unrelated code", DRY, minimal changes, follow patterns, etc.
- id: T-002
  title: Draft comprehensive constitutional rules covering identified themes
  priority: 2
  status: done
  notes: Rules should include DRY, SOC, minimal changes, language best practices, public API stability, research expectations
- id: T-003
  title: Update default constitution in src/init.rs with new rules
  priority: 3
  status: done
  notes: Keep under 10 rules, make generic/language-agnostic, expect users to customize
- id: T-004
  title: Remove behavioral guidance from all prompt templates in src/init.rs
  priority: 4
  status: done
  notes: Keep prompts purely operational (workflow steps and constraints)
- id: T-005
  title: Test updated prompts and constitution with mr restore
  priority: 5
  status: done
  notes: Verify prompts are tighter and constitution is comprehensive
- id: T-006
  title: Update AGENTS.md with new prompt/constitution philosophy if needed
  priority: 6
  status: done
  notes: Document the "what to do vs how to behave" boundary
---

# Summary

Consolidate all behavioral guidance and philosophy from scattered prompt files into the constitution, reducing duplication and creating a single source of truth for agent behavior. Tighten all 16 prompt files to focus purely on workflow steps and constraints, leaving the constitution as the sole location for editing how agents behave.

# Problem

Currently, behavioral rules like "don't modify unrelated code", "follow DRY", "minimal changes", and "follow existing patterns" are repeated across multiple prompt files. This creates maintenance burden and forces users to edit many files to change agent behavior. Prompts mix operational instructions ("what to do") with philosophical guidance ("how to behave"), making them verbose and duplicative.

# Goals

1. Move all behavioral/philosophical guidance from prompts to constitution
2. Update default constitution (src/init.rs) with comprehensive rules covering DRY, SOC, minimal changes, language best practices, public API stability, and research expectations
3. Tighten all 16 prompt templates in src/init.rs to be workflow-focused
4. Establish clear boundary: prompts define "what to do" and constraints, constitution defines "how to behave"
5. Reduce duplication so users edit only one file (constitution) to change agent behavior
6. Keep constitution under 10 rules, generic/language-agnostic, and user-customizable

# Non-Goals (MVP)

- Modifying user-customized prompt files (this affects defaults only)
- Adding project-specific or language-specific rules to default constitution
- Changing the structure or format of prompts or constitution
- Implementing new prompt or constitution features beyond consolidation

# History

## 2026-01-26 — T-001 Completed
- **Task**: Audit all 16 prompt files for behavioral/philosophical guidance
- **Status**: ✅ Done
- **Changes**:
  - Audited all 18 templates in `src/init.rs` (16 prompts + constitution + AGENTS)
  - Identified 6 recurring behavioral themes requiring consolidation
  - Documented findings with specific line numbers for each occurrence

- **Audit Findings**:
  
  **Prompts with behavioral guidance to remove:**
  1. `PROMPT_PRD_NEW_SYNTHESIZE` (line 570): DRY rule
  2. `PROMPT_RUN_TASK` (lines 612, 683-687): Minimal changes, don't modify unrelated code, public API stability, root cause preference, DRY rule
  3. `PROMPT_RUN_TASK_FINALIZE` (lines 902-906): No new features, no breaking changes, minimal changes, DRY rule
  4. `PROMPT_RUN_UAT_VERIFY` (lines 1009-1012): Minimal test code, don't modify unrelated code, DRY rule
  5. `PROMPT_PRD_EDIT` (lines 1074, 1083): Minimize changes, DRY rule
  6. `STARTER_AGENTS` (lines 1580-1581): Minimal changes, follow existing style

  **Themes identified for constitution rules:**
  1. **DRY (Don't Repeat Yourself)** — 5 occurrences
  2. **Minimal Changes** — 4 occurrences
  3. **Don't modify unrelated code** — 3 occurrences
  4. **Follow existing patterns/style** — 2 occurrences
  5. **Public API stability** — 1 occurrence
  6. **Root cause over workarounds** — 1 occurrence

  **Prompts that are already clean (workflow-focused):**
  - PROMPT_INIT, PROMPT_BOOTSTRAP_PLAN, PROMPT_BOOTSTRAP_GENERATE_PRDS
  - PROMPT_PRD_NEW_ROUND1, PROMPT_PRD_NEW_ROUNDN
  - PROMPT_CONSTITUTION_EDIT, PROMPT_ADAPT_LANGUAGE
  - PROMPT_PICK_PRD, PROMPT_DEVCONTAINER_GENERATE, PROMPT_SUGGEST_GENERATE
  - PROMPT_REINDEX (has task-specific constraints, which are appropriate)

- **UAT**: ✅ All 324 tests passed
- **Constitution Compliance**: No violations

## 2026-01-26 — T-002 Completed
- **Task**: Draft comprehensive constitutional rules covering identified themes
- **Status**: ✅ Done
- **Changes**:
  - Added 2 new rules to `.mr/constitution.md` covering previously missing themes:
    - Rule 6: **Public API Stability** — addresses theme 5 (public API stability)
    - Rule 7: **Root Cause Resolution** — addresses theme 6 (root cause over workarounds)
  - Constitution now has 7 rules total (well under the 10-rule limit)
  - All 6 themes identified in T-001 are now covered by constitution rules

- **Theme Coverage Summary**:
  | Theme | Constitution Rule |
  |-------|------------------|
  | DRY (Don't Repeat Yourself) | Rule 2: Single Source of Truth |
  | Minimal Changes | Rule 4: Minimal Changes |
  | Don't modify unrelated code | Rule 4: Minimal Changes |
  | Follow existing patterns/style | Rule 5: Consistency |
  | Public API stability | Rule 6: Public API Stability (NEW) |
  | Root cause over workarounds | Rule 7: Root Cause Resolution (NEW) |

- **UAT**: ✅ All 324 tests passed
- **Constitution Compliance**: No violations

## 2026-01-26 — T-003 Completed
- **Task**: Update default constitution in src/init.rs with new rules
- **Status**: ✅ Done
- **Changes**:
  - Updated `CONSTITUTION_TEMPLATE` in `src/init.rs` to include 6 comprehensive default rules
  - Replaced commented-out example rules with actual actionable rules
  - Rules included: Single Source of Truth (DRY), Separation of Concerns, Minimal Changes, Consistency, Public API Stability, Root Cause Resolution
  - Updated `test_constitution_template_content()` to verify the 6 new rules
  - Kept format generic/language-agnostic with customization comment at bottom

- **Rule Count**: 6 rules (well under 10-rule limit)
  - Note: Microralph's own constitution has 7 rules (includes "Prompt Management" which is project-specific)
  - The default template has 6 generic rules suitable for any project

- **UAT**: ✅ All 324 tests passed
- **Constitution Compliance**: No violations

## 2026-01-26 — T-004 Completed
- **Task**: Remove behavioral guidance from all prompt templates in src/init.rs
- **Status**: ✅ Done
- **Changes**:
  - Removed DRY rule from `PROMPT_PRD_NEW_SYNTHESIZE` Constraints section
  - Removed 4 behavioral constraints from `PROMPT_RUN_TASK`: "don't modify unrelated code", "don't change public API", "prefer root causes", and DRY
  - Simplified `PROMPT_RUN_TASK` Required Actions from 10 steps to 9 by removing "Follow existing patterns" (now in constitution)
  - Removed "Minimal changes" and DRY from `PROMPT_RUN_TASK_FINALIZE` Constraints
  - Removed 2 constraints from `PROMPT_RUN_UAT_VERIFY`: "don't modify unrelated code" and DRY
  - Removed "Minimize changes" step and DRY from `PROMPT_PRD_EDIT`
  - Kept `STARTER_AGENTS` unchanged (agent guidance is appropriate in AGENTS.md file)

- **UAT**: ✅ All 324 tests passed
- **Constitution Compliance**: No violations

## 2026-01-26 — T-005 Completed
- **Task**: Test updated prompts and constitution with mr restore
- **Status**: ✅ Done
- **Changes**:
  - Ran `mr restore` command successfully (17 prompt and template files restored)
  - Verified all prompts are now workflow-focused without behavioral guidance
  - Confirmed no occurrences of "DRY", "minimal changes", or "unrelated code" in prompts
  - Verified constitution template has 6 comprehensive default rules:
    1. Single Source of Truth (DRY)
    2. Separation of Concerns
    3. Minimal Changes
    4. Consistency
    5. Public API Stability
    6. Root Cause Resolution
  - `run_task.md` Constraints reduced from 4-5 items to 1 item
  - `run_task_finalize.md` now has only workflow-specific constraints
  - `run_uat_verify.md` now has only workflow-specific constraints
  - `prd_edit.md` now has only structural constraints

- **UAT**: ✅ All 324 tests passed
- **Constitution Compliance**: No violations

## 2026-01-26 — T-006 Completed
- **Task**: Update AGENTS.md with new prompt/constitution philosophy if needed
- **Status**: ✅ Done
- **Changes**:
  - Added new "Prompt vs Constitution Philosophy" section to AGENTS.md (before "Conventions for Agents")
  - Documented the separation: prompts define "what to do" (workflow), constitution defines "how to behave" (behavioral rules)
  - Added "Customization Guide" subsection explaining how to modify behavior globally vs per-workflow
  - Explained the `mr restore` command for resetting to defaults

- **UAT**: ✅ All 324 tests passed
- **Constitution Compliance**: No violations
## 2026-01-26 — uat-001 Verification
- **UAT**: All scattered "don't modify unrelated code" instances removed from prompts
- **Status**: ✅ Verified
- **Method**: Existing implementation verified via grep search
- **Details**:
  - Searched .mr/prompts/ and src/init.rs for "don't modify unrelated" and "unrelated code" patterns
  - No matches found in any prompt files
  - All 324 tests passed (cargo make uat)

## 2026-01-26 — uat-002 Verification
- **UAT**: Constitution has comprehensive behavior rules (under 10 rules)
- **Status**: ✅ Verified
- **Method**: New test created
- **Details**:
  - Added `test_constitution_template_has_under_10_rules()` to `src/init.rs`
  - Test counts numbered rules (pattern: "N. **Rule**") and verifies count > 0 and < 10
  - Default template has 6 rules; project constitution has 7 rules
  - All 325 tests passed (cargo make uat)

## 2026-01-26 — uat-003 Verification
- **UAT**: Prompts are workflow-focused with no philosophy or opinion
- **Status**: ✅ Verified
- **Method**: New test created
- **Details**:
  - Added `test_prompts_are_workflow_focused_no_philosophy()` to `src/init.rs`
  - Test checks all 16 prompts for forbidden philosophical/behavioral terms
  - Forbidden patterns: "DRY", "Don't Repeat Yourself", "minimal change", "unrelated code", "how to behave"
  - All 326 tests passed (cargo make uat)

## 2026-01-26 — PRD Finalized
- **Status**: ✅ Finalized
- **Tasks Completed**: 6 tasks (T-001 through T-006)
- **Outcome**: All tasks completed, acceptance tests passed (326/326 tests)
- **Changelog**: Entry added under [Changed] — Consolidated behavioral guidance into constitution
- **Cleanup**: None required (no temp files or debug statements found)
- **Summary**:
  - Consolidated all behavioral rules (DRY, minimal changes, consistency, etc.) into constitution
  - Tightened 16 prompt templates to be purely workflow-focused
  - Added "Prompt vs Constitution Philosophy" documentation to AGENTS.md
  - Default constitution now has 6 comprehensive rules; project constitution has 7 rules
