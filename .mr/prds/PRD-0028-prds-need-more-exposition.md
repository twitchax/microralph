---
id: PRD-0028
title: "PRDs Need More Exposition"
status: active
owner: twitchax
created: 2026-01-27
updated: 2026-01-27

principles:
- "Prompts define what to do; constitution defines how to behave"
- "Single Source of Truth: PRD template in src/init.rs is the authoritative source"
- "Exposition should help AI agents during mr run understand implementation strategy"
- "Tasks reference main body details for implementation hints rather than duplicating information"

references:
- name: "PRD Template (src/init.rs)"
  url: "file://src/init.rs#L124-L191"
- name: "PRD New Round1 Prompt (src/init.rs)"
  url: "file://src/init.rs#L368-L425"
- name: "PRD New Synthesize Prompt (src/init.rs)"
  url: "file://src/init.rs#L495-L612"
- name: ".mr/templates/prd.md"
  url: "file://.mr/templates/prd.md"
- name: ".mr/prompts/prd_new_round1_questions.md"
  url: "file://.mr/prompts/prd_new_round1_questions.md"
- name: ".mr/prompts/prd_new_synthesize_prd.md"
  url: "file://.mr/prompts/prd_new_synthesize_prd.md"

acceptance_tests:
- id: uat-001
  name: "PRD template contains Technical Approach section"
  command: cargo make uat
  uat_status: unverified
- id: uat-002
  name: "PRD template contains Assumptions section"
  command: cargo make uat
  uat_status: unverified
- id: uat-003
  name: "PRD template contains Constraints section"
  command: cargo make uat
  uat_status: unverified
- id: uat-004
  name: "PRD template contains References to Code section"
  command: cargo make uat
  uat_status: unverified
- id: uat-005
  name: "Round1 questions prompt elicits technical approach details"
  command: cargo make uat
  uat_status: unverified
- id: uat-006
  name: "Synthesize prompt instructs agent to populate new sections"
  command: cargo make uat
  uat_status: unverified

tasks:
- id: T-001
  title: "Add Technical Approach section to PRD template"
  priority: 1
  status: done
  notes: "Update PRD_TEMPLATE in src/init.rs to include a new '# Technical Approach' section between Goals and Non-Goals. This section outlines the implementation strategy."
- id: T-002
  title: "Add Assumptions section to PRD template"
  priority: 2
  status: done
  notes: "Add '# Assumptions' section to PRD_TEMPLATE after Technical Approach. Lists preconditions and assumptions the implementation depends on."
- id: T-003
  title: "Add Constraints section to PRD template"
  priority: 3
  status: done
  notes: "Add '# Constraints' section to PRD_TEMPLATE. Documents technical or scope constraints that limit implementation options."
- id: T-004
  title: "Add References to Code section to PRD template"
  priority: 4
  status: todo
  notes: "Add '# References to Code' section to PRD_TEMPLATE. Lists specific files, modules, or patterns relevant to this PRD."
- id: T-005
  title: "Update PROMPT_PRD_NEW_ROUND1 to elicit new section content"
  priority: 5
  status: todo
  notes: "Modify the Round1 questions prompt in src/init.rs to ask about technical approach, assumptions, constraints, and code references. See T-001 through T-004 for section definitions."
- id: T-006
  title: "Update PROMPT_PRD_NEW_SYNTHESIZE to populate new sections"
  priority: 6
  status: todo
  notes: "Modify the synthesize prompt in src/init.rs to instruct the agent to fill in the new sections. Include guidance to encourage architecture diagrams when appropriate."
- id: T-007
  title: "Run mr restore to update .mr/prompts and .mr/templates"
  priority: 7
  status: todo
  notes: "After updating src/init.rs, run 'cargo run -- restore' to synchronize .mr/prompts/ and .mr/templates/ with the new embedded defaults."
- id: T-008
  title: "Verify all UATs pass"
  priority: 8
  status: todo
  notes: "Run 'cargo make uat' to verify all acceptance tests pass."

---

# Summary

Enhance PRD exposition by adding new body sections that provide richer context for AI agents during `mr run`. The current PRD structure has Summary, Problem, Goals, and Non-Goals sections, but lacks guidance on **how** to implement the feature. This PRD adds Technical Approach, Assumptions, Constraints, and References to Code sections, and updates the Q&A flow to elicit the information needed to populate them.

---

# Problem

When `mr run` executes tasks from a PRD, the agent sees "what" needs to be done (Summary, Goals) but not "how" to approach it. This leads to:

1. **Suboptimal implementations**: Agents may choose approaches that conflict with codebase conventions or overlook existing patterns.
2. **Redundant exploration**: Agents spend time discovering code references and constraints that could have been captured upfront.
3. **Inconsistent PRD quality**: Without structured prompts for technical details, PRD authors omit valuable context.

---

# Goals

1. Add a **Technical Approach** section to the PRD template that outlines the implementation strategy before execution begins.
2. Add an **Assumptions** section to capture preconditions the implementation depends on.
3. Add a **Constraints** section to document technical or scope limitations.
4. Add a **References to Code** section to list relevant files, modules, and patterns.
5. Update the `mr new` Q&A prompts (Round1 and Synthesize) to elicit and populate these new sections.
6. Encourage architecture diagrams in the Technical Approach section when they would aid understanding.
7. Ensure task descriptions in frontmatter can reference these body sections for implementation hints.

---

# Technical Approach

## Template Changes (src/init.rs)

Modify `PRD_TEMPLATE` to insert four new sections between Goals and Non-Goals:

```markdown
# Technical Approach

{{technical_approach}}

---

# Assumptions

{{assumptions}}

---

# Constraints

{{constraints}}

---

# References to Code

{{references_to_code}}
```

## Prompt Changes (src/init.rs)

### PROMPT_PRD_NEW_ROUND1

Add questions to the "Required Actions" list:
- What is the high-level technical approach or implementation strategy?
- What assumptions does this feature rely on?
- What constraints limit implementation options (performance, compatibility, dependencies)?
- What existing code files, modules, or patterns are relevant?
- Would an architecture diagram help clarify the approach?

### PROMPT_PRD_NEW_SYNTHESIZE

Update the "Markdown Body" documentation to list the new sections and instruct the agent to populate them from Q&A responses. Add guidance to include ASCII or Mermaid diagrams when the technical approach involves complex component interactions.

## Synchronization

After modifying `src/init.rs`, run `mr restore` to overwrite `.mr/prompts/` and `.mr/templates/` with the updated embedded defaults. This follows the Prompt Management constitutional rule.

---

# Assumptions

1. The existing PRD parsing logic in microralph handles arbitrary Markdown sections in the body (no schema validation beyond frontmatter).
2. Task notes can reference body section names (e.g., "See Technical Approach for details") without requiring structural links.
3. AI agents (Copilot, Claude) can generate useful technical approach content when prompted with appropriate questions.

---

# Constraints

1. **Constitution Compliance**: All changes must update `src/init.rs` first, then synchronize via `mr restore` (per Prompt Management rule).
2. **Backward Compatibility**: Existing PRDs without the new sections should remain valid. The template provides placeholder text, but the sections are optional in practice.
3. **Minimal Frontmatter Changes**: This PRD does not add new frontmatter fields; exposition remains in the Markdown body.

---

# References to Code

- `src/init.rs`: Contains `PRD_TEMPLATE`, `PROMPT_PRD_NEW_ROUND1`, `PROMPT_PRD_NEW_ROUNDN`, and `PROMPT_PRD_NEW_SYNTHESIZE` constants (lines 124-612).
- `.mr/templates/prd.md`: Materialized template file (synchronized via `mr restore`).
- `.mr/prompts/prd_new_round1_questions.md`: Materialized Round1 prompt (synchronized via `mr restore`).
- `.mr/prompts/prd_new_synthesize_prd.md`: Materialized synthesize prompt (synchronized via `mr restore`).
- `src/commands/restore.rs`: Implements the `mr restore` command that synchronizes prompts and templates.

---

# Non-Goals (MVP)

- Adding new frontmatter fields for structured technical metadata (keep exposition in Markdown body)
- Automatic validation of PRD body section content
- Migrating existing PRDs to the new format (they remain valid as-is)
- Custom section ordering or user-defined body sections

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-01-28 — T-001 Completed
- **Task**: Add Technical Approach section to PRD template
- **Status**: ✅ Done
- **Changes**:
  - Modified `src/init.rs` to add `# Technical Approach` section with `{{technical_approach}}` placeholder to `PRD_TEMPLATE`
  - Section is placed between Goals and Non-Goals as specified in the PRD
  - UAT passed: 451 tests run, all passed

- **Constitution Compliance**: No violations. Change is minimal and follows the Prompt Management rule (updating src/init.rs first; synchronization via mr restore will happen in T-007).

---

## 2026-01-28 — T-002 Completed
- **Task**: Add Assumptions section to PRD template
- **Status**: ✅ Done
- **Changes**:
  - Modified `src/init.rs` to add `# Assumptions` section with `{{assumptions}}` placeholder to `PRD_TEMPLATE`
  - Section is placed between Technical Approach and Non-Goals as specified in the PRD
  - UAT passed: 451 tests run, all passed

- **Constitution Compliance**: No violations. Change is minimal and follows the Prompt Management rule (updating src/init.rs first; synchronization via mr restore will happen in T-007).

---

## 2026-01-28 — T-003 Completed
- **Task**: Add Constraints section to PRD template
- **Status**: ✅ Done
- **Changes**:
  - Modified `src/init.rs` to add `# Constraints` section with `{{constraints}}` placeholder to `PRD_TEMPLATE`
  - Section is placed between Assumptions and Non-Goals as specified in the PRD
  - UAT passed: 451 tests run, all passed

- **Constitution Compliance**: No violations. Change is minimal and follows the Prompt Management rule (updating src/init.rs first; synchronization via mr restore will happen in T-007).

---