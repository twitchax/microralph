//! Initialization logic for `mr init`.
//!
//! Creates the `.mr/` directory structure, templates, prompts, and starter AGENTS.md.

use std::fmt;
use std::path::Path;
use std::str::FromStr;

use anyhow::{Context, Result};

use crate::config;
use crate::prompt::PromptKind;

/// Supported programming languages for project initialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    /// Rust projects (default).
    #[default]
    Rust,
    /// Python projects.
    Python,
    /// Node.js/JavaScript/TypeScript projects.
    Node,
    /// Go projects.
    Go,
    /// Java projects.
    Java,
}

impl Language {
    /// Returns the typical build/test commands for this language.
    pub fn build_commands(&self) -> &'static [&'static str] {
        match self {
            Self::Rust => &[
                "cargo build",
                "cargo test",
                "cargo make ci",
                "cargo make uat",
            ],
            Self::Python => &[
                "pip install -e .",
                "pytest",
                "python -m pytest",
                "make test",
            ],
            Self::Node => &["npm install", "npm test", "npm run build", "npm run lint"],
            Self::Go => &[
                "go build ./...",
                "go test ./...",
                "go vet ./...",
                "make test",
            ],
            Self::Java => &["mvn compile", "mvn test", "gradle build", "gradle test"],
        }
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Rust => write!(f, "rust"),
            Self::Python => write!(f, "python"),
            Self::Node => write!(f, "node"),
            Self::Go => write!(f, "go"),
            Self::Java => write!(f, "java"),
        }
    }
}

impl FromStr for Language {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "rust" | "rs" => Ok(Self::Rust),
            "python" | "py" => Ok(Self::Python),
            "node" | "nodejs" | "js" | "javascript" | "ts" | "typescript" => Ok(Self::Node),
            "go" | "golang" => Ok(Self::Go),
            "java" | "jvm" | "kotlin" | "kt" => Ok(Self::Java),
            _ => Err(format!(
                "Unknown language: '{s}'. Supported: rust, python, node, go, java"
            )),
        }
    }
}

/// Detects the primary language of a repository based on project files.
///
/// Returns `None` if no recognizable project files are found.
pub fn detect_language(root: impl AsRef<Path>) -> Option<Language> {
    let root = root.as_ref();

    // Order matters: check most specific first.
    if root.join("Cargo.toml").exists() {
        return Some(Language::Rust);
    }

    if root.join("go.mod").exists() {
        return Some(Language::Go);
    }

    if root.join("package.json").exists() {
        return Some(Language::Node);
    }

    if root.join("pyproject.toml").exists()
        || root.join("setup.py").exists()
        || root.join("requirements.txt").exists()
    {
        return Some(Language::Python);
    }

    if root.join("pom.xml").exists()
        || root.join("build.gradle").exists()
        || root.join("build.gradle.kts").exists()
    {
        return Some(Language::Java);
    }

    None
}

/// Default content for the PRD template.
pub const PRD_TEMPLATE: &str = r#"---
id: PRD-NNNN
title: "{{title}}"
status: draft                 # draft | active | done | parked
owner: "{{owner}}"
created: {{date}}
updated: {{date}}

principles:
- Principle 1 (guiding constraint or design decision)
- Principle 2

references:
- name: Reference Name
  url: https://example.com/reference

acceptance_tests:
- id: uat-001
  name: Description of what the test verifies
  command: cargo make uat  # or specific test command
  uat_status: unverified  # unverified | verified (verified = a real UAT test exists)

tasks:
- id: T-001
  title: First task title
  priority: 1
  status: todo
  notes: Additional context, dependencies, or implementation hints.

---

# Summary

{{summary}}

---

# Problem

{{problem}}

---

# Goals

1. Goal 1
2. Goal 2
3. Goal 3

---

# Non-Goals (MVP)

- Non-goal 1
- Non-goal 2

---

# History

(Entries appended by `mr run` will go below this line.)

---
"#;

/// Default content for the constitution template.
pub const CONSTITUTION_TEMPLATE: &str = r#"# Constitution

This file defines project-specific governance rules and constraints that guide PRD creation and execution.

## Purpose

The constitution:
- Encodes project-specific best practices and constraints
- Influences PRD creation and finalization via LLM prompts
- Is version-controlled and user-editable
- Violations are logged in PRD history but do not block execution

## Rules

<!-- Example rules (uncomment and customize as needed):

1. **Acceptance tests must be codified**: One-off acceptance tests are unacceptable. Every UAT must be implemented as a repeatable test in the codebase.

2. **Follow existing architecture patterns**: New features must align with established patterns in the codebase. Introduce new patterns only when necessary and document why.

3. **No breaking changes without migration path**: Breaking API changes require a clear migration guide and deprecation warnings.

4. **Security-first design**: All user inputs must be validated and sanitized. Authentication and authorization must be explicit.

5. **Performance baselines**: New features must not degrade performance by more than 10% without explicit justification in the PRD.

6. **Documentation requirements**: All public APIs must have docstrings. Complex logic must have inline comments explaining the "why".

7. **Test coverage standards**: New code must maintain or improve test coverage. Aim for at least 80% line coverage.

8. **Code review requirements**: All PRs require at least one approval. High-risk changes require two approvals.

-->

<!-- Add your project-specific rules below: -->
"#;

/// Default content for the init prompt.
pub const PROMPT_INIT: &str = r#"# microralph — Init Prompt

## Objective

Initialize a new repository with microralph structure.

## Context

You are initializing a new repository for use with microralph (`mr`).

## Required Actions

1. Create the `.mr/` directory structure:
   - `.mr/prds/` — PRD files
   - `.mr/templates/` — PRD templates
   - `.mr/prompts/` — Static prompt files
   - `.mr/PRDS.md` — PRD index

2. Create a starter `AGENTS.md` file at the repo root.

3. Ensure `Makefile.toml` exists with required tasks:
   - `ci`
   - `fmt`
   - `clippy`
   - `test`
   - `uat`

## Output

Confirm initialization is complete and list the files created.
"#;

/// Default content for the bootstrap plan prompt.
pub const PROMPT_BOOTSTRAP_PLAN: &str = r#"# microralph — Bootstrap Plan Prompt

## Objective

Analyze an existing repository and plan PRD generation.

## Context

You are analyzing an existing repository to understand its structure and plan the generation of PRDs.

## Required Analysis

1. **Detect cargo-make entrypoints and required tasks**
   - Look for `Makefile.toml`, `Makefile`, `package.json` scripts
   - Identify build, test, lint, and deployment commands

2. **Detect crates/modules and responsibilities**
   - Identify the main modules and their purposes
   - Understand the architectural layers

3. **Detect CI workflows and required checks**
   - Look for `.github/workflows/`, `.gitlab-ci.yml`, etc.
   - Understand the existing CI/CD pipeline

4. **Detect docs that imply features**
   - Read README, DEVELOPMENT, CONTRIBUTING, etc.
   - Identify planned features, TODOs, or roadmap items

5. **Detect TODO/FIXME hotspots**
   - Search for TODO, FIXME, HACK comments
   - Prioritize areas needing attention

## Output

Produce a structured plan for PRD generation, including:
- List of proposed PRDs with titles
- Priority ordering
- Key tasks for each PRD
"#;

/// Default content for the bootstrap generate PRDs prompt.
pub const PROMPT_BOOTSTRAP_GENERATE_PRDS: &str = r#"# microralph — Bootstrap Generate PRDs Prompt

## Objective

Generate starter PRDs based on the bootstrap plan.

## Context

You have analyzed the repository and created a bootstrap plan. Now generate the actual PRD files.

## Plan

{{plan}}

## Required Actions

For each PRD in the plan:

1. Create a PRD file in `.mr/prds/` with the format:
   - `PRD-NNNN-slug.md`

2. Include YAML frontmatter with:
   - `id`: PRD identifier
   - `title`: Human-readable title
   - `status`: `active` or `draft`
   - `owner`: Repository owner
   - `created`: Current date
   - `updated`: Current date
   - `tasks`: List of tasks with id, title, priority, status

3. Include Markdown body with:
   - Summary section
   - Problem section
   - Goals section
   - Non-Goals section (if applicable)
   - Empty History section

4. Update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.

## Constraints

- Generate at most {{prd_budget}} PRDs
- Each PRD should have 3-8 tasks
- Tasks should be actionable and verifiable

## Output

Confirm PRDs are generated and update `.mr/PRDS.md` index.
"#;

/// Default content for the PRD new round 1 questions prompt.
pub const PROMPT_PRD_NEW_ROUND1: &str = r#"# microralph — PRD New Round 1 Questions Prompt

## Objective

Generate follow-up questions to clarify a new PRD request.

## Context

The user wants to create a new PRD with slug: `{{slug}}`

{{#if user_description}}
User's initial description:
> {{user_description}}
{{/if}}

{{#if user_context}}
User's upfront context:
> {{user_context}}
{{/if}}

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**Note**: Your questions and the resulting PRD should respect these constitutional rules.
{{/if}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}} ({{status}})
{{/each}}

## Required Actions

1. Review the existing PRDs to understand context.
2. Scan the codebase for relevant files, patterns, or entry points that could bootstrap specific tasks.
3. Generate 3-5 clarifying questions to understand:
   - What problem does this PRD solve?
   - What are the success criteria?
   - What are the acceptance tests?
   - What are the dependencies or blockers?
   - What is the scope (MVP vs full feature)?
   - Are there specific sections in existing PRDs that are relevant (e.g., patterns, lessons learned)?
   - Are there existing code files or modules that should be referenced?

## Output Format

Return a numbered list of questions. Keep questions concise and actionable.

Example:
1. What specific problem are you trying to solve?
2. What does "done" look like for this feature?
3. Are there any existing patterns in the codebase to follow?
"#;

/// Default content for the PRD new round N questions prompt.
pub const PROMPT_PRD_NEW_ROUNDN: &str = r#"# microralph — PRD New Round N Questions Prompt

## Objective

Continue the Q/A session for PRD creation, or signal readiness.

## Context

The user is creating a new PRD with slug: `{{slug}}`

{{#if user_context}}
### User-Provided Context

{{user_context}}

{{/if}}
{{#if constitution}}
### Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**Note**: Your questions and the resulting PRD should respect these constitutional rules.

{{/if}}
## Previous Q/A

{{#each qa_history}}
**Q{{@index}}**: {{question}}
**A{{@index}}**: {{answer}}

{{/each}}

## Required Actions

1. Review the Q/A history.
2. Determine if you have enough information to synthesize a PRD.
3. If more clarification is needed, ask 1-3 additional questions.
4. If ready, respond with exactly: `READY_TO_SYNTHESIZE`

## Output Format

Either:
- A numbered list of follow-up questions (1-3 max)
- Or the exact text: `READY_TO_SYNTHESIZE`
"#;

/// Default content for the PRD new synthesize prompt.
pub const PROMPT_PRD_NEW_SYNTHESIZE: &str = r#"# microralph — PRD New Synthesize Prompt

## Objective

Synthesize a complete PRD from the Q/A session, following the template structure exactly.

## Context

The user is creating a new PRD with slug: `{{slug}}`

{{#if user_context}}
User's upfront context:
> {{user_context}}
{{/if}}

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**CRITICAL**: The PRD you synthesize MUST respect these constitutional rules. If any aspect of the PRD would violate the constitution, adjust the approach or note the constraint explicitly.
{{/if}}

## Q/A Session

{{#each qa_history}}
**Q**: {{question}}
**A**: {{answer}}

{{/each}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}}
{{/each}}

## PRD Template Structure

**CRITICAL**: Read `.mr/templates/prd.md` for the exact structure. The PRD has two parts:

### 1. YAML Frontmatter (between `---` delimiters)

The frontmatter contains ALL structured data:
- `id`: PRD-NNNN (generate next ID based on existing PRDs)
- `title`: Human-readable title
- `status`: draft (for new PRDs)
- `owner`: Owner name
- `created` / `updated`: Date in YYYY-MM-DD format
- `principles`: List of guiding constraints or design decisions
- `references`: List of objects with `name` and `url` fields
- `acceptance_tests`: List of UATs with `id`, `name`, `command`, `uat_status`
- `tasks`: List of tasks with `id`, `title`, `priority`, `status`, `notes`

### 2. Markdown Body (after closing `---`)

The body contains ONLY narrative/exposition sections:
- `# Summary` — Brief overview
- `# Problem` — Problem statement
- `# Goals` — Numbered list of goals
- `# Non-Goals (MVP)` — What's explicitly out of scope
- `# History` — Empty section for `mr run` to append entries

## Required Actions

1. Generate the next PRD ID (e.g., PRD-0006 if PRD-0005 exists).
2. Scan the codebase for relevant files, patterns, or entry points.
3. Review existing PRDs for related work or patterns.
4. Create the PRD following the template structure EXACTLY.
5. Update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.

## Acceptance Tests Format

Each acceptance test in the frontmatter MUST have these fields:
```yaml
- id: uat-001
  name: Short description of what the test verifies
  command: cargo make uat  # or specific test command
  uat_status: unverified   # always start as unverified
```

## Tasks Format

Each task in the frontmatter MUST have these fields:
```yaml
- id: T-001
  title: Clear, actionable task title
  priority: 1              # lower = higher priority
  status: todo             # always start as todo
  notes: Optional implementation hints or dependencies
```

## Output

CRITICAL: Output ONLY the raw PRD file content. Start your response IMMEDIATELY with the `---` frontmatter delimiter. Do NOT wrap the output in code blocks. Do NOT include any preamble, explanation, or commentary.

The first three characters of your response MUST be exactly: `---`
"#;

/// Default content for the run task prompt.
pub const PROMPT_RUN_TASK: &str = r#"# microralph — Run Task Prompt

## Objective

Execute the next incomplete task from a PRD, verify with UATs, update the PRD, and regenerate the index.

## Context

Look at the PRD file at `{{prd_path}}` to understand:
- The project goals and constraints
- The task list and their statuses
- The History section showing previous attempts and outcomes

The suggested next task is `{{next_task_id}}` based on priority, but verify against the PRD.

{{#if constitution}}
## Constitution

This project has a constitution that defines governance rules and constraints. Your implementation should respect these rules:

```
{{constitution}}
```

**Important**: If your implementation violates any constitutional rules, you MUST mention the violation in the History entry with reasoning about why it was necessary or unavoidable. Constitution violations are logged for transparency but do not block task execution.
{{/if}}

## Required Actions

1. **Study the README** at the repository root to understand the project's purpose, conventions, and development workflow.
2. **Study the PRD** at `{{prd_path}}` and understand it fully, including goals, constraints, and task history.
3. **Identify the task** `{{next_task_id}}` and its requirements.
4. **Implement the task** as described, making minimal and focused changes.
5. **Follow existing patterns** and conventions in the codebase.
6. **Run `cargo make uat`** to verify all acceptance tests pass.
7. **Update AGENTS.md** if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.
8. **Update the PRD file** (see below for details).
9. **Regenerate the index** by running: `cargo run -- list` (or manually update `.mr/PRDS.md`).
10. **Commit your work** with a descriptive commit message.

## Updating the PRD

You MUST update the PRD file at `{{prd_path}}` as you work:

### Update PRD Status to Active (if currently draft)

When starting work on a PRD, change its status from `draft` to `active`:

```yaml
status: active  # <-- Change from 'draft' to 'active' when starting work
```

### Update Task Status in Frontmatter

Change the task's status from `todo` or `in-progress` to `done` if UAT passes:

```yaml
tasks:
  - id: {{next_task_id}}
    title: "..."
    priority: N
    status: done  # <-- Change from 'todo' to 'done'
```

### Append to History Section

Add a new History entry at the bottom of the PRD file with this format:

```markdown
## YYYY-MM-DD — {{next_task_id}} Completed
- **Task**: [Task title]
- **Status**: ✅ Done (or ❌ Failed if UAT failed)
- **Changes**:
  - Bullet points describing what was changed
  - Include file names and key details
  - Note UAT pass/fail with brief evidence
{{#if constitution}}
- **Constitution Compliance**: If any constitutional rules were violated, note them here with reasoning. If fully compliant, you may omit this section or note "No violations."
{{/if}}
```

## Opportunistic UAT Verification (Per-Task)

After completing a task, check if any UATs (acceptance tests) can be verified **now** based on the work just completed:

1. **Review the PRD's `acceptance_tests`**: Identify UATs that are currently `unverified`.
2. **Determine feasibility**: A UAT can be verified NOW if:
   - The functionality it tests was implemented by this task or previous completed tasks
   - It does NOT depend on incomplete tasks (check the task list)
   - A test can be created or an existing test can be run
3. **If a UAT is feasible**: Create or run the test, and if it passes, update `uat_status: verified` in the frontmatter.
4. **If a UAT requires incomplete tasks**: Skip it — the full UAT verification loop will handle it later.
5. **Document in History**: Note which UATs (if any) were opportunistically verified.

**Example reasoning**:
- UAT "CLI accepts --verbose flag" → If T-003 (add verbose flag) is done, verify it now.
- UAT "Build pipeline passes" → Requires all tasks, skip until finalization.
- UAT "Color output works" → If T-001 (add color module) is done, verify it now.

This reduces work during the final UAT verification loop and catches issues earlier.

## Constraints

- Do not modify unrelated code.
- Do not change the public API unless the task requires it.
- Prefer fixing root causes over surface workarounds.
- Always update the PRD even if the task fails (document what was attempted).

## When All Tasks Are Done

If completing this task means all tasks in the PRD are now `done`:
1. **Complete and commit this task** as normal (update status, append History, commit).
2. **UAT verification happens automatically**: microralph will detect unverified acceptance tests and enter a dedicated UAT verification loop in subsequent `mr run` invocations.
3. **Do NOT attempt to verify UATs yourself** in this task — the verification loop handles each UAT individually with focused prompts.

Note: Unverified UATs will block PRD finalization. The UAT verification loop allows you to verify tests, create new tests, or opt-out with an explanation for each UAT.

## On Success

If `cargo make uat` passes:
1. Update task status to `done` in the PRD frontmatter.
2. Append a success History entry.
3. Regenerate `.mr/PRDS.md` to reflect new progress.
4. Commit all changes with message: `prd({{prd_id}})feat({{next_task_id}}): [brief description]`

## On Failure

If `cargo make uat` fails:
1. Leave task status as `todo` or `in-progress`.
2. Append a failure History entry describing what was attempted and what failed.
3. Do NOT regenerate the index (status unchanged).
4. Do NOT commit (leave changes for next attempt or manual review).

## Output

Report what happened:
- Whether the task was completed successfully
- What changes were made
- UAT results (pass/fail with brief evidence)
- What was committed (if anything)
"#;

/// Default content for the run task finalize prompt.
pub const PROMPT_RUN_TASK_FINALIZE: &str = r#"# microralph — PRD Finalization Prompt

## Objective

Execute the complete finalization workflow for a PRD: verify acceptance tests, generate changelog entry, create summary report, clean up temporary artifacts, update inter-PRD links, and append finalization history entry.

## Context

You are finalizing `{{prd_id}}`: **{{prd_title}}**.

All tasks in this PRD have been completed. This is the final wrap-up step before marking the PRD as done.

## PRD Summary

{{prd_summary}}

## Completed Tasks

{{completed_tasks}}

## Current Changelog

The current `CHANGELOG.md` content is:

```markdown
{{changelog_content}}
```

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**Note**: Your finalization work (changelog entries, documentation updates, cleanup decisions) should respect these constitutional rules.
{{/if}}

---

## Required Actions

Execute the following steps in order:

### 1. Verify All Acceptance Tests Pass

Run the full test suite to ensure nothing is broken:

```bash
cargo make uat
```

**Criteria**:
- All tests must pass
- No warnings that indicate broken functionality
- If tests fail, stop and report the failure — do not proceed with finalization

### 2. Generate Changelog Entry

Add an entry under `## [Unreleased]` in `CHANGELOG.md`:

**Format** (Keep a Changelog):
```markdown
### Added
- {{prd_id}}: {{prd_title}} — Brief description of key changes
```

**Guidelines**:
- Choose the appropriate category based on the PRD's work:
  - **Added**: New features or functionality
  - **Changed**: Changes to existing functionality
  - **Fixed**: Bug fixes
  - **Deprecated**: Features marked for removal
  - **Removed**: Removed features
  - **Security**: Security-related changes
- Include the PRD ID and title
- Summarize key changes (1-3 bullet points if multiple significant changes)
- Keep entries concise but informative

### 3. Create Summary Report

Generate a summary report that will be:
- Printed to stdout (for the user)
- Appended to the PRD as a finalization history entry

**Report Format**:
```markdown
## Finalization Summary — {{prd_id}}

**Date**: YYYY-MM-DD
**PRD**: {{prd_id}} — {{prd_title}}
**Tasks Completed**: N tasks
**Status**: ✅ Finalized

### Completed Tasks
- [List of completed tasks]

### Changes Made
- [Brief summary of main changes]

### Changelog Entry Added
- [Confirm category and brief description]
```

### 4. Clean Up Temporary Files and Excessive Comments

Search for and remove:

**Temporary files**:
- Debug scripts or test files created during development
- Temporary data files (`.tmp`, `.bak`, scratch files)
- Generated files that shouldn't be committed

**Excessive comments**:
- TODO comments that are now resolved
- Debug logging statements (e.g., `println!`, `console.log`, `dbg!`)
- Commented-out code that is no longer needed
- Development notes that don't belong in final code

**Do NOT remove**:
- Legitimate TODOs for future work
- Documentation comments
- Necessary inline explanations

### 5. Update Inter-PRD Links in Index

Check `PRDS.md` for any references to this PRD that need updating:

- If this PRD was blocked by another PRD, verify the blocker is resolved
- If other PRDs reference this PRD, ensure links/references are accurate
- Update any "See Also" or cross-reference sections

Run to regenerate the index:
```bash
cargo run -- list
```

### 6. Append Finalization History Entry

Add a final history entry to the PRD file documenting the finalization:

**Format**:
```markdown
## YYYY-MM-DD — PRD Finalized
- **Status**: ✅ Finalized
- **Outcome**: All tasks completed, acceptance tests passed
- **Changelog**: Entry added under [Unreleased] → [Category]
- **Cleanup**: [Brief note on any cleanup performed]
```

### 7. Commit All Changes

After all finalization steps are complete, commit the changes:

```bash
git add -A
git commit -m "prd({{prd_id}})finalize: [brief description]"
```

**Commit message format**: `prd({{prd_id}})finalize: [brief description]`

Example: `prd(PRD-0001)finalize: Complete MVP build with finalization workflow`

---

## Final Documentation Check

Ensure these documents are up-to-date:

- [ ] **README.md** — Reflects any new features or usage changes
- [ ] **AGENTS.md** — Updated with new conventions or patterns discovered
- [ ] **Inline documentation** — Code comments and docstrings are accurate

---

## Constraints

- **No new features**: This is finalization only — polish and documentation
- **No breaking changes**: The codebase should be in a releasable state
- **Minimal changes**: Only make changes required for finalization
- **Concise entries**: Changelog and history entries should be brief but complete

---

## Output

After completing all steps, provide:

1. **UAT Result**: Pass/fail with test count
2. **Changelog Entry**: The exact entry added
3. **Cleanup Summary**: List of any files/comments cleaned up
4. **Commit**: The commit hash and message
5. **Final Summary**: Brief confirmation that finalization is complete

Example output format:
```
✅ UAT: 219/219 tests passed
✅ Changelog: Added entry under "Added" — {{prd_id}}: {{prd_title}}
✅ Cleanup: Removed 2 debug println! statements, 1 TODO comment
✅ Commit: abc1234 — prd({{prd_id}})finalize: {{prd_title}}
✅ Finalization complete for {{prd_id}}
```
"#;

/// Default content for the run UAT verification prompt.
pub const PROMPT_RUN_UAT_VERIFY: &str = r#"# microralph — UAT Verification Prompt

## Objective

Verify a single unverified acceptance test (UAT) from a PRD by creating a test, running an existing test, or documenting why verification isn't feasible.

## Context

You are verifying acceptance test `{{uat_id}}` from PRD `{{prd_id}}`.

**PRD Path**: `{{prd_path}}`

**Acceptance Test Details**:
- **ID**: `{{uat_id}}`
- **Name**: `{{uat_name}}`
- **Command**: `{{uat_command}}`
- **Current Status**: unverified

All tasks in this PRD are complete. You are now in the UAT verification phase to ensure acceptance criteria are covered by real tests.

## Required Actions

Choose ONE of the following verification approaches:

### Option A: Verify Existing Test

If a test already exists that covers this acceptance criterion:
1. Identify the test (file path and test name).
2. Run the test to confirm it passes: `{{uat_command}}`
3. If it passes, update the PRD to mark `uat_status: verified` for `{{uat_id}}`.
4. Append a History entry documenting the verification.

### Option B: Create New Test

If no test exists but one can feasibly be created:
1. Create a minimal test that covers the acceptance criterion.
2. Run `cargo make uat` to verify the test passes.
3. Update the PRD to mark `uat_status: verified` for `{{uat_id}}`.
4. Append a History entry documenting the new test.

### Option C: Opt-Out with Explanation

If verification is not feasible (e.g., requires manual testing, external dependencies, or is covered implicitly by other tests), you may opt out:
1. Do NOT update `uat_status` (leave as `unverified`).
2. Append a History entry explaining why verification isn't feasible.
3. Respond with `OPT-OUT:` followed by your explanation on a single line.

## Updating the PRD

### Update UAT Status in Frontmatter

If verification succeeds (Option A or B), update the acceptance test entry:

```yaml
acceptance_tests:
  - id: {{uat_id}}
    name: "{{uat_name}}"
    command: {{uat_command}}
    uat_status: verified  # <-- Change from 'unverified' to 'verified'
```

### Append to History Section

Add a History entry documenting your verification attempt:

```markdown
## YYYY-MM-DD — {{uat_id}} Verification
- **UAT**: {{uat_name}}
- **Status**: ✅ Verified (or ⏭️ Opted-out)
- **Method**: [Existing test / New test / Opt-out]
- **Details**:
  - [Test file and name if applicable]
  - [Explanation if opted out]
```

## Constraints

- Focus on this single UAT (`{{uat_id}}`). Do not verify other UATs in this invocation.
- Keep test code minimal — just enough to cover the acceptance criterion.
- Do not modify unrelated code.
- Always update the PRD even if opting out (document your reasoning).

## On Success

If verification succeeds:
1. Update `uat_status: verified` in the PRD frontmatter.
2. Append a verification History entry.
3. Regenerate `.mr/PRDS.md` by running: `cargo run -- list`
4. Commit with message: `prd({{prd_id}})uat({{uat_id}}): [brief description]`

## On Opt-Out

If opting out:
1. Leave `uat_status: unverified` unchanged.
2. Append an opt-out History entry with clear explanation.
3. Respond with `OPT-OUT: [your explanation]` so the run loop knows to proceed.
4. Do NOT commit (opt-outs don't change UAT status).

## Output

Report what happened:
- Whether verification succeeded or opted out
- What approach was used (existing test, new test, or opt-out)
- Test details or opt-out explanation
- What was committed (if anything)
"#;

/// Default content for the PRD edit prompt.
pub const PROMPT_PRD_EDIT: &str = r#"# microralph — PRD Edit Prompt

## Objective

Make targeted edits to an existing PRD based on user request.

## Context

The user wants to modify the PRD at `{{prd_path}}`.

## User Request

{{user_request}}

## Current PRD Content

```markdown
{{prd_content}}
```

## Q/A History (if any)

{{#each qa_history}}
**Q**: {{question}}
**A**: {{answer}}

{{/each}}

## Required Actions

1. **Understand the request**: Read the user's request carefully.
2. **Analyze the PRD**: Review the current PRD content.
3. **Apply changes**: Make the requested modifications.
4. **Preserve structure**: Keep the YAML frontmatter valid and the Markdown body properly formatted.
5. **Minimize changes**: Only modify what's necessary to fulfill the request.

## Constraints

- Do not change the PRD ID.
- Do not remove existing History entries.
- Keep the overall structure intact (frontmatter, Summary, Problem, Goals, Non-Goals, History sections).
- If adding tasks, assign appropriate IDs (T-NNN) and priorities.
- If adding acceptance tests, assign appropriate IDs (uat-NNN).

## Output Format

If you need more information, respond with a numbered list of questions (1-3 max):
```
1. Question one?
2. Question two?
```

If you have enough information, respond with exactly `READY_TO_APPLY` on its own line, followed by the complete updated PRD content in a markdown code block:
```
READY_TO_APPLY

```markdown
---
id: PRD-XXXX
...
---
# Summary
...
```
```

Ensure the output is the complete PRD file, not just the changed sections.
"#;

/// Default content for the constitution edit prompt.
pub const PROMPT_CONSTITUTION_EDIT: &str = r#"# microralph — Constitution Edit Prompt

## Objective

Intelligently update the project constitution based on a natural language request.

## Context

The constitution is a governance file at `.mr/constitution.md` that defines project-specific rules and constraints. The user wants to modify it based on the following request.

## User Request

{{user_request}}

## Current Constitution Content

```markdown
{{constitution_content}}
```

## Q/A History (if any)

{{#each qa_history}}
**Q**: {{question}}
**A**: {{answer}}

{{/each}}

## Required Actions

1. **Understand the request**: Parse the user's natural language request to identify what rule(s) should be added, modified, or removed.
2. **Analyze the constitution**: Review the current constitution structure and content.
3. **Apply changes intelligently**: 
   - Add new rules with appropriate numbering
   - Modify existing rules while preserving intent
   - Remove rules if requested
   - Maintain clear, actionable language
4. **Preserve structure**: Keep the constitution format consistent (Purpose section, Rules section, numbered list).
5. **Be precise**: Rules should be unambiguous and enforceable by an LLM.

## Constraints

- Maintain the basic structure: `# Constitution`, `## Purpose`, `## Rules`
- Rules must be numbered (e.g., `1. **Rule title**: Description`)
- Keep rules concise but complete
- Ensure rules are actionable and verifiable
- Do not add vague or unenforceable rules
- If removing a rule, renumber remaining rules appropriately

## Output Format

If you need more information, respond with a numbered list of questions (1-3 max):
```
1. Question one?
2. Question two?
```

If you have enough information, respond with exactly `READY_TO_APPLY` on its own line, followed by the complete updated constitution content in a markdown code block:
```
READY_TO_APPLY

```markdown
# Constitution
...
```
```

Ensure the output is the complete constitution file, not just the changed sections.
"#;

/// Default content for the language adaptation prompt.
pub const PROMPT_ADAPT_LANGUAGE: &str = r#"# microralph — Adapt Language Prompt

## Objective

Rewrite the microralph prompts and templates for a different programming language.

## Context

The user has initialized microralph for a **{{language}}** project. The default prompts and templates are designed for Rust projects. You need to adapt them.

## Target Language

**{{language}}**

## Typical Build/Test Commands for {{language}}

{{#if build_commands}}
{{#each build_commands}}
- {{command}}
{{/each}}
{{/if}}

## Files to Update

The following files in `.mr/` need to be adapted for {{language}}:

### Templates (`.mr/templates/`)
- `prd.md` — Update example commands in the template

### Prompts (`.mr/prompts/`)
- `run_task.md` — Change `cargo make uat` to the appropriate test/build command for {{language}}
- `run_task_finalize.md` — Update verification commands
- `bootstrap_plan.md` — Update detection heuristics for {{language}} project structure
- `init.md` — Update Makefile.toml references if not applicable

### AGENTS.md
- Update the Quick Start section with {{language}}-appropriate commands
- Update build/test commands

## Required Actions

1. Read each file listed above from `.mr/` and the root `AGENTS.md`.
2. For each file, rewrite it to use {{language}}-appropriate:
   - Build commands (e.g., `npm test`, `pytest`, `go test`, `mvn test`)
   - Project structure references (e.g., `package.json`, `pyproject.toml`, `go.mod`)
   - Tool chains and conventions
3. Write the updated files back to disk.
4. Keep the overall structure and purpose of each file intact.
5. Preserve all `{{placeholder}}` template variables — only change the static content.

## Constraints

- Do not change the file structure or add new files.
- Do not remove placeholder variables like `{{prd_path}}`, `{{next_task_id}}`, etc.
- Keep the microralph-specific sections (e.g., History format, PRD frontmatter references).
- Preserve the auto-managed section markers in AGENTS.md.

## Output

Confirm which files were updated and summarize the key changes made for {{language}}.
"#;

/// Default content for the reindex prompt.
pub const PROMPT_REINDEX: &str = r#"# microralph — Reindex Prompt

## Objective

Regenerate the `.mr/PRDS.md` index file and verify/fix inter-PRD links and code links across all PRDs.

## Context

The user wants to:
1. Regenerate the `.mr/PRDS.md` index to reflect the current state of all PRDs.
2. Scan all PRDs for inter-PRD links (e.g., references to other PRD IDs) and code links (e.g., references to source files).
3. Verify that all links are valid and use proper Markdown link syntax.
4. Fix any broken or incorrectly formatted links.

## PRDs Directory

Path: `{{prds_dir}}`

## Current PRD Files

{{#each prd_files}}
- `{{filename}}` (ID: {{id}}, Title: {{title}})
{{/each}}

## Repository Root

Path: `{{repo_root}}`

## Required Actions

1. **Read all PRD files** in `{{prds_dir}}`.

2. **Regenerate the index** by running: `cargo run -- list`

3. **Scan for inter-PRD references** in each PRD:
   - Look for mentions of PRD IDs like `PRD-0001`, `PRD-0002`, etc.
   - Convert plain text references to proper Markdown links: `[PRD-0001](./PRD-0001-slug.md)`
   - Use relative paths from the PRD's location.

4. **Scan for code references** in each PRD:
   - Look for file paths like `src/module.rs`, `lib/file.js`, etc.
   - Verify the files exist in the repository.
   - Convert plain text references to proper Markdown links: `[src/module.rs](../../src/module.rs)`
   - Use relative paths from the PRD's location (`.mr/prds/`).
   - For line references like "line 42", consider using GitHub-style anchors: `[src/module.rs#L42](../../src/module.rs#L42)`

5. **Update PRD files** with fixed links:
   - Only modify files that have broken or incorrectly formatted links.
   - Preserve all other content exactly as-is.

## Link Format Guidelines

### Inter-PRD Links

- From: `see PRD-0002 for details`
- To: `see [PRD-0002](./PRD-0002-feature-name.md) for details`

### Code File Links

- From: `implementation in src/run.rs`
- To: `implementation in [src/run.rs](../../src/run.rs)`

### Code Line Links

- From: `defined at src/run.rs line 42`
- To: `defined at [src/run.rs#L42](../../src/run.rs#L42)`

## Constraints

- Do not modify PRD content other than fixing links.
- Do not change the structure of PRD files.
- Do not add links where none were intended (only convert existing plain-text references).
- Preserve the YAML frontmatter exactly.
- Keep History sections intact.

## Output

Report what was done:
- Confirmation that the index was regenerated
- Number of PRDs scanned
- Number of inter-PRD links verified/fixed
- Number of code links verified/fixed
- List of files modified (if any)
"#;

/// Default content for the pick PRD prompt.
pub const PROMPT_PICK_PRD: &str = r#"# microralph — Pick PRD Prompt

## Objective

Analyze the available PRDs and determine which one should be worked on next.

## Context

The user has invoked `mr run` without specifying a PRD ID. Your job is to study the available PRDs and recommend the best one to work on next.

## Available PRDs

{{#each prds}}
### {{id}}: {{title}}

- **Status**: {{status}}
- **Progress**: {{completed}}/{{total}} tasks complete
- **Incomplete Tasks**:
{{#each incomplete_tasks}}
  - {{id}}: {{title}} (priority: {{priority}})
{{/each}}

{{/each}}

## Required Analysis

Consider the following when choosing:

1. **PRD Status**: Active PRDs should generally be prioritized over Draft PRDs.
2. **Progress**: PRDs that are closer to completion may be worth finishing first.
3. **Task Priority**: Look at the priorities of remaining tasks.
4. **Dependencies**: Check if any PRD references or depends on another.
5. **Momentum**: Consider which PRD would provide the most value if completed next.

## Output Format

Respond with ONLY the PRD ID that should be worked on next, on a single line. No explanation, no markdown, just the ID.

Example:
```
PRD-0002
```

If there are no valid PRDs to work on (no active/draft PRDs with incomplete tasks), respond with:
```
NONE
```
"#;

pub const PROMPT_DEVCONTAINER_GENERATE: &str = r#"# Dev Container Configuration Generation

## Objective

Generate a `.devcontainer/devcontainer.json` file based on repository analysis that enables developers to run this project in a consistent, sandboxed dev container environment.

## Context

You are analyzing a repository to create a dev container configuration. Below is the analysis of the repository:

```
{{analysis}}
```

## Requirements

Generate a complete, valid `devcontainer.json` file that:

1. **Uses an appropriate base image** for `{{language}}` development
   - For Rust: `mcr.microsoft.com/devcontainers/rust:latest`
   - For Python: `mcr.microsoft.com/devcontainers/python:3.11`
   - For Node.js: `mcr.microsoft.com/devcontainers/javascript-node:18`
   - For Go: `mcr.microsoft.com/devcontainers/go:latest`
   - For Java: `mcr.microsoft.com/devcontainers/java:17`

2. **Includes necessary VS Code extensions** for the detected language and frameworks

3. **Installs development tools** referenced in the analysis (e.g., cargo-make, cargo-nextest for Rust)

4. **Sets up the environment** with appropriate post-create commands to install dependencies

5. **Forwards relevant ports** if the project includes web servers or APIs

6. **Configures container settings** like mounts, environment variables as needed

## Analysis Guidelines

Based on the repository analysis:
- Identify tools from git commit messages (e.g., "add cargo-make", "use nextest")
- Include extensions for detected frameworks (e.g., rust-analyzer for Rust, pylint for Python)
- Set up post-create commands to run common initialization (e.g., `cargo build`, `npm install`)
- If microralph is detected, ensure `gh` CLI is available for GitHub Copilot integration

## Output Format

Return ONLY the JSON content for `devcontainer.json`. Do NOT include any explanatory text before or after the JSON.

You may wrap the JSON in a markdown code block like this:

```json
{
  "name": "Project Name",
  "image": "mcr.microsoft.com/devcontainers/...",
  ...
}
```

## Example Structure

```json
{
  "name": "My Project",
  "image": "mcr.microsoft.com/devcontainers/rust:latest",
  "features": {
    "ghcr.io/devcontainers/features/github-cli:1": {}
  },
  "customizations": {
    "vscode": {
      "extensions": [
        "rust-lang.rust-analyzer",
        "tamasfe.even-better-toml"
      ]
    }
  },
  "postCreateCommand": "cargo build",
  "forwardPorts": [8080],
  "remoteUser": "vscode"
}
```

## Constraints

- Use only official Microsoft dev container base images
- Include only extensions that are actively maintained and widely used
- Keep post-create commands minimal and fast (prefer lazy installation)
- Do not include secrets or credentials in the configuration
- Ensure the JSON is valid and properly formatted

---

Generate the `devcontainer.json` content now.
"#;

/// Default content for the empty PRDS.md index.
pub const EMPTY_INDEX: &str = r#"# microralph — PRD Index

This file is auto-generated by `mr`. Do not edit manually.

## Active PRDs

*No active PRDs.*

## Draft PRDs

*No draft PRDs.*

## Done PRDs

*No completed PRDs.*

## Parked PRDs

*No parked PRDs.*

## Statistics

- **Total PRDs**: 0
- **Active**: 0
- **Draft**: 0
- **Done**: 0
- **Parked**: 0

---

*Last updated: N/A*
"#;

/// Default content for the starter AGENTS.md file.
pub const STARTER_AGENTS: &str = r#"# Agents Guide

This document provides guidance for AI coding agents working in this repository.

## Workspace Overview

- `src/`: Main source code
- `.mr/`: microralph state directory
  - `prds/`: PRD files
  - `templates/`: PRD templates
  - `prompts/`: Static prompt files for each stage
  - `PRDS.md`: Auto-generated PRD index

## Quick Start

```bash
# Build
cargo build

# Test
cargo make test

# Full CI (fmt, clippy, test)
cargo make ci

# UAT (the one true gate)
cargo make uat
```

## Conventions for Agents

- Keep changes minimal and focused; avoid unrelated refactors.
- Follow existing style; don't add license headers.
- Use `anyhow::Result` for fallible functions.
- Prefer `tracing` over `println!` for diagnostics.
- All dev commands route through `cargo make`.

### Code Style

- Use vertical whitespace generously to separate logical sections.
- Prefer explicitness over implicitness.
- Reduce nesting by using guard clauses and early returns.
- Prefer functional programming techniques where appropriate.

## PRD Format

PRDs are Markdown files with YAML frontmatter containing:

- `id`: Unique identifier (e.g., PRD-0001)
- `title`: Human-readable title
- `status`: draft | active | done | parked
- `tasks`: List of tasks with id, title, priority, status

History entries are appended by `mr run` at the bottom of the PRD.

---

## Manual Updates by Agents

Automatic AGENTS.md updates have been removed to give agents more flexibility. Agents should update AGENTS.md manually when:

- Discovering new build/test commands or troubleshooting steps
- Identifying code patterns or conventions not already documented
- Adding new tools or dependencies that affect the workflow
- Finding solutions to common issues during implementation

Update any relevant section, not just this one. Keep additions concise and actionable.
"#;

/// Result of initialization, containing counts and paths of created items.
#[derive(Debug, Default)]
pub struct InitResult {
    /// Number of directories created.
    pub dirs_created: usize,

    /// Number of files created.
    pub files_created: usize,

    /// Number of files skipped (already existed).
    pub files_skipped: usize,

    /// List of created file paths (relative to root).
    pub created_paths: Vec<String>,

    /// List of skipped file paths (relative to root).
    pub skipped_paths: Vec<String>,
}

/// Initializes the microralph directory structure in the given root.
///
/// Creates:
/// - `.mr/prds/` directory
/// - `.mr/templates/` directory with `prd.md`
/// - `.mr/prompts/` directory with all prompt files
/// - `.mr/PRDS.md` empty index
/// - `.mr/constitution.md` governance rules
/// - `AGENTS.md` starter file (if not exists)
///
/// # Arguments
///
/// * `root` - The root directory of the repository
///
/// # Returns
///
/// An `InitResult` with counts and paths of created/skipped items.
pub fn init(root: impl AsRef<Path>) -> Result<InitResult> {
    let root = root.as_ref();
    let mut result = InitResult::default();

    // Create .mr directory structure.
    let mr_dir = root.join(".mr");
    let prds_dir = mr_dir.join("prds");
    let templates_dir = mr_dir.join("templates");
    let prompts_dir = mr_dir.join("prompts");

    create_dir_if_missing(&prds_dir, &mut result)?;
    create_dir_if_missing(&templates_dir, &mut result)?;
    create_dir_if_missing(&prompts_dir, &mut result)?;

    // Create template file.
    create_file_if_missing(&templates_dir.join("prd.md"), PRD_TEMPLATE, &mut result)?;

    // Create prompt files.
    create_file_if_missing(&prompts_dir.join("init.md"), PROMPT_INIT, &mut result)?;
    create_file_if_missing(
        &prompts_dir.join("bootstrap_plan.md"),
        PROMPT_BOOTSTRAP_PLAN,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("bootstrap_generate_prds.md"),
        PROMPT_BOOTSTRAP_GENERATE_PRDS,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("prd_new_round1_questions.md"),
        PROMPT_PRD_NEW_ROUND1,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("prd_new_roundN_questions.md"),
        PROMPT_PRD_NEW_ROUNDN,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("prd_new_synthesize_prd.md"),
        PROMPT_PRD_NEW_SYNTHESIZE,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("run_task.md"),
        PROMPT_RUN_TASK,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("run_task_finalize.md"),
        PROMPT_RUN_TASK_FINALIZE,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("run_uat_verify.md"),
        PROMPT_RUN_UAT_VERIFY,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("prd_edit.md"),
        PROMPT_PRD_EDIT,
        &mut result,
    )?;
    create_file_if_missing(
        &prompts_dir.join("adapt_language.md"),
        PROMPT_ADAPT_LANGUAGE,
        &mut result,
    )?;
    create_file_if_missing(&prompts_dir.join("reindex.md"), PROMPT_REINDEX, &mut result)?;
    create_file_if_missing(
        &prompts_dir.join("pick_prd.md"),
        PROMPT_PICK_PRD,
        &mut result,
    )?;

    // Create empty PRDS.md index.
    create_file_if_missing(&mr_dir.join("PRDS.md"), EMPTY_INDEX, &mut result)?;

    // Create default config.toml.
    create_file_if_missing(
        &mr_dir.join("config.toml"),
        config::DEFAULT_CONFIG,
        &mut result,
    )?;

    // Create constitution.md.
    create_file_if_missing(
        &mr_dir.join("constitution.md"),
        CONSTITUTION_TEMPLATE,
        &mut result,
    )?;

    // Create AGENTS.md at repo root (if not exists).
    create_file_if_missing(&root.join("AGENTS.md"), STARTER_AGENTS, &mut result)?;

    // Log the number of available prompt kinds (uses PromptKind::all()).
    tracing::debug!(
        prompt_kinds = PromptKind::all().len(),
        "Initialization complete with all prompt kinds available"
    );

    Ok(result)
}

/// Creates a directory if it doesn't exist.
fn create_dir_if_missing(path: &Path, result: &mut InitResult) -> Result<()> {
    if !path.exists() {
        std::fs::create_dir_all(path)
            .with_context(|| format!("Failed to create directory: {}", path.display()))?;
        result.dirs_created += 1;
        tracing::debug!(path = %path.display(), "Created directory");
    }

    Ok(())
}

/// Creates a file if it doesn't exist.
fn create_file_if_missing(path: &Path, content: &str, result: &mut InitResult) -> Result<()> {
    // Get relative path for logging.
    let relative = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.display().to_string());

    if path.exists() {
        result.files_skipped += 1;
        result.skipped_paths.push(relative);
        tracing::debug!(path = %path.display(), "File already exists, skipping");
    } else {
        std::fs::write(path, content)
            .with_context(|| format!("Failed to create file: {}", path.display()))?;
        result.files_created += 1;
        result.created_paths.push(relative);
        tracing::debug!(path = %path.display(), "Created file");
    }

    Ok(())
}

/// Checks if a directory has been initialized with microralph.
pub fn is_initialized(root: impl AsRef<Path>) -> bool {
    let root = root.as_ref();
    let mr_dir = root.join(".mr");

    mr_dir.exists()
        && mr_dir.join("prds").exists()
        && mr_dir.join("templates").exists()
        && mr_dir.join("prompts").exists()
        && mr_dir.join("PRDS.md").exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_init_creates_structure() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let result = init(root).unwrap();

        // Check directories exist.
        assert!(root.join(".mr/prds").exists());
        assert!(root.join(".mr/templates").exists());
        assert!(root.join(".mr/prompts").exists());

        // Check template exists.
        assert!(root.join(".mr/templates/prd.md").exists());

        // Check prompts exist.
        assert!(root.join(".mr/prompts/init.md").exists());
        assert!(root.join(".mr/prompts/bootstrap_plan.md").exists());
        assert!(root.join(".mr/prompts/bootstrap_generate_prds.md").exists());
        assert!(
            root.join(".mr/prompts/prd_new_round1_questions.md")
                .exists()
        );
        assert!(
            root.join(".mr/prompts/prd_new_roundN_questions.md")
                .exists()
        );
        assert!(root.join(".mr/prompts/prd_new_synthesize_prd.md").exists());
        assert!(root.join(".mr/prompts/run_task.md").exists());
        assert!(root.join(".mr/prompts/run_task_finalize.md").exists());
        assert!(root.join(".mr/prompts/run_uat_verify.md").exists());
        assert!(root.join(".mr/prompts/adapt_language.md").exists());
        assert!(root.join(".mr/prompts/reindex.md").exists());
        assert!(root.join(".mr/prompts/pick_prd.md").exists());

        // Check index exists.
        assert!(root.join(".mr/PRDS.md").exists());

        // Check config.toml exists.
        assert!(root.join(".mr/config.toml").exists());

        // Check constitution.md exists.
        assert!(root.join(".mr/constitution.md").exists());

        // Check AGENTS.md exists.
        assert!(root.join("AGENTS.md").exists());

        // Check result counts.
        assert_eq!(result.dirs_created, 3);
        assert_eq!(result.files_created, 18); // 1 template + 13 prompts + 1 index + 1 config + 1 constitution + 1 AGENTS.md
        assert_eq!(result.files_skipped, 0);
    }

    #[test]
    fn test_init_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // First init.
        let result1 = init(root).unwrap();
        assert_eq!(result1.files_created, 18);
        assert_eq!(result1.files_skipped, 0);

        // Second init should skip all files.
        let result2 = init(root).unwrap();
        assert_eq!(result2.files_created, 0);
        assert_eq!(result2.files_skipped, 18);
        assert_eq!(result2.dirs_created, 0);
    }

    #[test]
    fn test_init_preserves_existing_files() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create an existing AGENTS.md with custom content.
        let agents_path = root.join("AGENTS.md");
        let custom_content = "# My Custom AGENTS.md\n\nThis should be preserved.";
        std::fs::write(&agents_path, custom_content).unwrap();

        // Run init.
        let result = init(root).unwrap();

        // AGENTS.md should be skipped.
        assert!(result.skipped_paths.contains(&"AGENTS.md".to_string()));

        // Content should be preserved.
        let content = std::fs::read_to_string(&agents_path).unwrap();
        assert_eq!(content, custom_content);
    }

    #[test]
    fn test_is_initialized() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Not initialized yet.
        assert!(!is_initialized(root));

        // Initialize.
        init(root).unwrap();

        // Now initialized.
        assert!(is_initialized(root));
    }

    #[test]
    fn test_prd_template_content() {
        assert!(PRD_TEMPLATE.contains("id: PRD-NNNN"));
        assert!(PRD_TEMPLATE.contains("{{title}}"));
        assert!(PRD_TEMPLATE.contains("# Summary"));
        assert!(PRD_TEMPLATE.contains("# History"));
    }

    #[test]
    fn test_constitution_template_content() {
        assert!(CONSTITUTION_TEMPLATE.contains("# Constitution"));
        assert!(CONSTITUTION_TEMPLATE.contains("## Purpose"));
        assert!(CONSTITUTION_TEMPLATE.contains("## Rules"));
        assert!(CONSTITUTION_TEMPLATE.contains("<!-- Example rules"));
        // Check for numbered example rules
        assert!(CONSTITUTION_TEMPLATE.contains("1. **Acceptance tests must be codified**"));
    }

    #[test]
    fn test_prompts_contain_placeholders() {
        // Round 1 questions should have slug placeholder.
        assert!(PROMPT_PRD_NEW_ROUND1.contains("{{slug}}"));

        // Run task should have prd_path placeholder.
        assert!(PROMPT_RUN_TASK.contains("{{prd_path}}"));

        // Run UAT verify should have uat_id and prd_id placeholders.
        assert!(PROMPT_RUN_UAT_VERIFY.contains("{{uat_id}}"));
        assert!(PROMPT_RUN_UAT_VERIFY.contains("{{prd_id}}"));
        assert!(PROMPT_RUN_UAT_VERIFY.contains("{{prd_path}}"));
    }

    #[test]
    fn test_init_result_created_paths() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let result = init(root).unwrap();

        // Check that created paths are reasonable.
        assert!(result.created_paths.contains(&"prd.md".to_string()));
        assert!(result.created_paths.contains(&"init.md".to_string()));
        assert!(result.created_paths.contains(&"PRDS.md".to_string()));
        assert!(result.created_paths.contains(&"AGENTS.md".to_string()));
    }

    #[test]
    fn test_language_display() {
        assert_eq!(format!("{}", Language::Rust), "rust");
        assert_eq!(format!("{}", Language::Python), "python");
        assert_eq!(format!("{}", Language::Node), "node");
        assert_eq!(format!("{}", Language::Go), "go");
        assert_eq!(format!("{}", Language::Java), "java");
    }

    #[test]
    fn test_language_from_str() {
        assert_eq!("rust".parse::<Language>().unwrap(), Language::Rust);
        assert_eq!("rs".parse::<Language>().unwrap(), Language::Rust);
        assert_eq!("python".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("py".parse::<Language>().unwrap(), Language::Python);
        assert_eq!("node".parse::<Language>().unwrap(), Language::Node);
        assert_eq!("nodejs".parse::<Language>().unwrap(), Language::Node);
        assert_eq!("js".parse::<Language>().unwrap(), Language::Node);
        assert_eq!("typescript".parse::<Language>().unwrap(), Language::Node);
        assert_eq!("go".parse::<Language>().unwrap(), Language::Go);
        assert_eq!("golang".parse::<Language>().unwrap(), Language::Go);
        assert_eq!("java".parse::<Language>().unwrap(), Language::Java);
        assert_eq!("jvm".parse::<Language>().unwrap(), Language::Java);

        assert!("unknown".parse::<Language>().is_err());
    }

    #[test]
    fn test_language_build_commands() {
        assert!(Language::Rust.build_commands().contains(&"cargo test"));
        assert!(Language::Python.build_commands().contains(&"pytest"));
        assert!(Language::Node.build_commands().contains(&"npm test"));
        assert!(Language::Go.build_commands().contains(&"go test ./..."));
        assert!(Language::Java.build_commands().contains(&"mvn test"));
    }

    #[test]
    fn test_detect_language_rust() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("Cargo.toml"), "").unwrap();

        assert_eq!(detect_language(temp.path()), Some(Language::Rust));
    }

    #[test]
    fn test_detect_language_python() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("pyproject.toml"), "").unwrap();

        assert_eq!(detect_language(temp.path()), Some(Language::Python));

        let temp2 = TempDir::new().unwrap();
        std::fs::write(temp2.path().join("setup.py"), "").unwrap();

        assert_eq!(detect_language(temp2.path()), Some(Language::Python));
    }

    #[test]
    fn test_detect_language_node() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("package.json"), "").unwrap();

        assert_eq!(detect_language(temp.path()), Some(Language::Node));
    }

    #[test]
    fn test_detect_language_go() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("go.mod"), "").unwrap();

        assert_eq!(detect_language(temp.path()), Some(Language::Go));
    }

    #[test]
    fn test_detect_language_java() {
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("pom.xml"), "").unwrap();

        assert_eq!(detect_language(temp.path()), Some(Language::Java));
    }

    #[test]
    fn test_detect_language_none() {
        let temp = TempDir::new().unwrap();

        assert_eq!(detect_language(temp.path()), None);
    }

    #[test]
    fn test_detect_language_priority() {
        // Rust takes priority over others.
        let temp = TempDir::new().unwrap();
        std::fs::write(temp.path().join("Cargo.toml"), "").unwrap();
        std::fs::write(temp.path().join("package.json"), "").unwrap();

        assert_eq!(detect_language(temp.path()), Some(Language::Rust));
    }

    #[test]
    fn test_adapt_language_prompt_placeholders() {
        assert!(PROMPT_ADAPT_LANGUAGE.contains("{{language}}"));
        assert!(PROMPT_ADAPT_LANGUAGE.contains("{{#each build_commands}}"));
    }
}
