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
    pub fn build_commands(self) -> &'static [&'static str] {
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

# depends_on:                 # Optional: List of PRD IDs this PRD depends on
# - PRD-0001                  # (uncomment and add dependencies as needed)
# - PRD-0003

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

1. **Single Source of Truth**: Follow the DRY (Don't Repeat Yourself) principle. Avoid duplicating logic, data, or configuration across multiple files. When the same information must exist in multiple places, derive it from a single authoritative source.

2. **Separation of Concerns**: Follow SOC (Separation of Concerns) principles. Each module, function, and file should have a single, well-defined responsibility. Avoid mixing unrelated concerns in the same code unit.

3. **Minimal Changes**: When making changes, modify only what is necessary to achieve the objective. Avoid unrelated refactoring, style changes, or "improvements" that are not directly related to the task at hand.

4. **Consistency**: Follow the existing code style, conventions, and patterns established in the codebase. Do not introduce new patterns without justification.

5. **Public API Stability**: Do not change public API signatures unless the task explicitly requires it. Breaking changes must be documented and justified in the PRD history.

6. **Root Cause Resolution**: Prefer fixing root causes over applying surface-level workarounds. When a workaround is necessary, document the underlying issue and rationale.

<!-- Add your project-specific rules below: -->
"#;

/// Default content for the init prompt.
pub const PROMPT_INIT: &str = r"# microralph — Init Prompt

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
";

/// Default content for the bootstrap plan prompt.
pub const PROMPT_BOOTSTRAP_PLAN: &str = r"# microralph — Bootstrap Plan Prompt

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
";

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

## Tasks Format

Each task in the frontmatter MUST have these fields:
```yaml
- id: T-001
  title: Clear, actionable task title
  priority: 1              # MUST be a number (lower = higher priority)
  status: todo             # always start as todo
  notes: Optional implementation hints or dependencies
```

## YAML Frontmatter Quoting Rules

**CRITICAL**: YAML strings containing special characters MUST be quoted to avoid parse errors:
- **Colons (`:`)**: Any string with a colon must be quoted: `title: "Fix: Bug in parser"`
- **Hashes (`#`)**: Strings with `#` must be quoted to avoid comment interpretation
- When in doubt, wrap string values in double quotes.

## Constraints

- Generate at most {{prd_budget}} PRDs
- Each PRD should have 3-8 tasks
- Tasks should be actionable and verifiable
- **Priority MUST be a numeric value** (1, 2, 3, etc.) where 1 is highest priority

## Output

Confirm PRDs are generated and update `.mr/PRDS.md` index.
"#;

/// Default content for the PRD new interactive prompt.
///
/// This is a single-phase prompt: the agent gathers info from the user interactively,
/// then writes the PRD file directly to `.mr/prds/` before telling the user to exit.
/// The Rust side then picks up the file, validates, and indexes it.
pub const PROMPT_PRD_NEW_INTERACTIVE: &str = r#"# microralph — PRD New Interactive Prompt

## Objective

Have an interactive conversation with the user to gather enough information to create a well-defined PRD, then write it directly to disk.

## Context

The user wants to create a new PRD with slug: `{{slug}}`
The next available PRD ID is: `{{next_id}}`
The PRD file should be written to: `{{prd_path}}`

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

**CRITICAL**: The PRD you create MUST respect these constitutional rules. If any aspect of the PRD would violate the constitution, adjust the approach or note the constraint explicitly.
{{/if}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}} ({{status}})
{{/each}}

## Phase 1: Interactive Discovery

1. Review the existing PRDs to understand project context.
2. Scan the codebase for relevant files, patterns, or entry points.
3. Engage the user in a natural conversation to understand:
   - What problem does this PRD solve?
   - What are the success criteria and acceptance tests?
   - What are the dependencies or blockers?
   - What is the scope (MVP vs full feature)?
   - What is the high-level technical approach?
   - What assumptions and constraints apply?
4. Ask follow-up questions based on the user's responses.
5. Reference existing PRDs and code when relevant.

## Phase 2: Write the PRD

When you have enough information, tell the user you're ready to write the PRD. Then:

1. **Write the PRD file** directly to `{{prd_path}}` using your file editing tools.
2. The PRD MUST follow the template structure below EXACTLY.
3. After writing the file, tell the user the PRD has been created and they can exit the chat.

## PRD Template Structure

The PRD has two parts that you MUST follow exactly:

### 1. YAML Frontmatter (between `---` delimiters)

The frontmatter contains ALL structured data:
- `id`: `{{next_id}}`
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
- `# Technical Approach` — Implementation strategy, architecture decisions, and high-level design. Include ASCII or Mermaid diagrams when the approach involves complex component interactions or data flows.
- `# Assumptions` — Preconditions the implementation depends on
- `# Constraints` — Technical or scope limitations that affect implementation options
- `# References to Code` — Relevant files, modules, patterns, or entry points in the codebase
- `# Non-Goals (MVP)` — What's explicitly out of scope
- `# History` — Empty section for `mr run` to append entries

**Technical Approach Guidance**: When the feature involves multiple components, services, or complex data flows, include an architecture diagram. Use ASCII art for simple diagrams or Mermaid syntax for more complex ones.

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

## YAML Frontmatter Quoting Rules

**CRITICAL**: YAML strings containing special characters MUST be quoted to avoid parse errors:
- **Colons (`:`)**: Any string with a colon must be quoted: `title: "Fix: Bug in parser"`
- **Hashes (`#`)**: Strings with `#` must be quoted to avoid comment interpretation
- **Leading/trailing spaces**: Use quotes to preserve whitespace
- **Empty strings**: Use `""` for empty values

When in doubt, wrap the value in double quotes. This is especially important for:
- `title` fields that often contain colons (e.g., "Feature: Add X")
- `notes` fields with complex descriptions
- `name` fields in references and acceptance tests

## AGENTS.md

After writing the PRD, update AGENTS.md if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.

## Important

- The PRD ID MUST be `{{next_id}}`.
- The PRD file MUST be written to `{{prd_path}}`.
- Do NOT just output the PRD content to the chat — you MUST write it to disk using your file tools.
- After writing the file, tell the user the PRD is ready and they can exit.
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

{{#if skills_manifest}}
## Available Skills

The following skills have been learned from previous task executions and may be relevant:

{{skills_manifest}}

> Read `.mr/skills/<name>/skill.md` for full details on any skill when relevant to your current task.
{{/if}}

## Required Actions

1. **Study the README** at the repository root to understand the project's purpose, conventions, and development workflow.
2. **Study the PRD** at `{{prd_path}}` and understand it fully, including goals, constraints, and task history.
3. **Identify the task** `{{next_task_id}}` and its requirements.
4. **Implement the task** as described.
5. **Run `cargo make uat`** to verify all acceptance tests pass.
6. **Update AGENTS.md** if your changes introduce new patterns, workflows, or troubleshooting steps that future agents should know about.
7. **Update the PRD file** (see below for details).
8. **Regenerate the index** by running: `cargo run -- list` (or manually update `.mr/PRDS.md`).
{{#if commit}}
9. **Commit your work** with a descriptive commit message.
{{else}}
9. **Do NOT commit your work** — leave changes staged or unstaged for manual review.
{{/if}}

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

**YAML Quoting Reminder**: When editing frontmatter, ensure strings containing colons (`:`) or hashes (`#`) are quoted. Example: `title: "Feature: Add new command"`

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
{{#if allow_add_task}}

### Adding New Tasks (Dynamic Task Addition)

During task execution, if you discover that additional work is needed — such as a missed prerequisite, supporting infrastructure for a UAT, or underestimated scope — you MAY add new tasks to the PRD's `tasks` array in the YAML frontmatter.

**Guidelines for adding tasks:**
- Assign the next available `T-XXX` ID (e.g., if the last task is `T-008`, use `T-009`).
- Set the new task's `status: todo` and assign an appropriate `priority`.
- Include a descriptive `title` and `notes` explaining why the task was added.
- Document any added tasks in the History entry for the current task.

**Prefer adding a task over skipping a UAT**: If a UAT cannot currently be verified but a new task could unblock it (e.g., by implementing missing infrastructure or test fixtures), add the task and leave the UAT as `unverified` for retry — rather than skipping the UAT entirely.

**Example:**
```yaml
tasks:
  # ... existing tasks ...
  - id: T-009
    title: "Add integration test helper for auth module"
    priority: 9
    status: todo
    notes: "Added during T-004 execution: needed to unblock uat-003 verification."
```
{{/if}}

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

- Always update the PRD even if the task fails (document what was attempted).

## Saving Skills (End-of-Task)

After completing the task, evaluate whether you learned a genuinely reusable technique during this execution. If so, save it as a skill:

1. **Create a skill directory**: `.mr/skills/<slug>/` where `<slug>` is a short, descriptive kebab-case name (e.g., `fix-clippy-pedantic`, `cargo-nextest-parallel`).
2. **Write the skill file**: `.mr/skills/<slug>/skill.md` with:
   - A clear title and one-line summary
   - When to use this skill
   - Step-by-step instructions or examples
   - Any helper scripts can go alongside as separate files in the same directory
3. **Update the manifest**: Add a one-line entry to `.mr/skills/SKILLS.md`:
   ```
   - **<slug>**: One-line summary of what this skill does.
   ```

**Bias toward selectivity**: Only save skills that are genuinely reusable across multiple tasks or PRDs. Do NOT save:
- One-off fixes specific to a single task
- Obvious or well-documented techniques
- Trivial implementation details

If no reusable skill was learned, skip this step entirely.

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
{{#if commit}}
4. Commit all changes with message: `prd({{prd_id}})feat({{next_task_id}}): [brief description]`
{{else}}
4. Do NOT commit — leave changes for manual review.
{{/if}}

## On Failure

If `cargo make uat` fails:
1. Leave task status as `todo` or `in-progress`.
2. Append a failure History entry describing what was attempted and what failed.
3. Do NOT regenerate the index (status unchanged).
{{#if commit}}
4. Do NOT commit (leave changes for next attempt or manual review).
{{else}}
4. Leave changes uncommitted for next attempt or manual review.
{{/if}}

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

Execute the complete finalization workflow for a PRD: verify acceptance tests, clean up temporary artifacts, update documentation, and append finalization history entry.

## Context

You are finalizing `{{prd_id}}`: **{{prd_title}}**.

All tasks in this PRD have been completed. This is the final wrap-up step before marking the PRD as done.

## PRD Summary

{{prd_summary}}

## Completed Tasks

{{completed_tasks}}

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**Note**: Your finalization work (documentation updates, cleanup decisions) should respect these constitutional rules.
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

### 2. Clean Up Temporary Files and Excessive Comments

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

### 3. Append Finalization History Entry

Add a final history entry to the PRD file documenting the finalization:

**Format**:
```markdown
## YYYY-MM-DD — PRD Finalized
- **Status**: ✅ Finalized
- **Tasks Completed**: N tasks (T-001 through T-NNN)
- **Outcome**: All tasks completed, acceptance tests passed (XXX/XXX tests)
- **Cleanup**: [Brief note on any cleanup performed]
- **Summary**:
  - [Key accomplishment 1]
  - [Key accomplishment 2]
  - [Key accomplishment 3]
```

### 4. Print Summary to Console

After appending the history entry, print a summary to stdout for the user.

**This is important** - the user should see a clear finalization summary in their terminal.

**Format**:
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎉 Finalization Complete: {{prd_id}} — {{prd_title}}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ UATs: XXX/XXX tests passed
✅ Tasks: N tasks completed (T-001 through T-NNN)
✅ Cleanup: [Summary of cleanup or "None required"]
{{#if commit}}
✅ Commit: [commit_hash] — prd({{prd_id}})finalize: [description]
{{else}}
⏸️ Commit: Skipped (--no-commit flag active) — changes left for manual review
{{/if}}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

{{#if commit}}
### 5. Commit All Changes

After all finalization steps are complete, commit the changes.

**CRITICAL**: You MUST commit these files (they've all been updated):
1. `.mr/prds/{{prd_id}}-*.md` — The PRD file with your appended history entry
2. `.mr/PRDS.md` — Auto-regenerated with updated PRD status (already done, just commit it)
3. Any other files you modified during cleanup

**Git commands**:
```bash
git add .mr/prds/{{prd_id}}*.md .mr/PRDS.md
git commit -m "prd({{prd_id}})finalize: [brief description]"
```

If you modified other files during cleanup, add them too:
```bash
git add -A
git commit -m "prd({{prd_id}})finalize: [brief description]"
```

**Commit message format**: `prd({{prd_id}})finalize: [brief description]`

Example: `prd(PRD-0001)finalize: Complete MVP build with finalization workflow`
{{else}}
### 5. Do NOT Commit Changes

**CRITICAL**: Do NOT commit any changes. Leave all modifications staged or unstaged for manual review.

The following files have been updated and should be reviewed before committing:
1. `.mr/prds/{{prd_id}}-*.md` — The PRD file with your appended history entry
2. `.mr/PRDS.md` — Auto-regenerated with updated PRD status
3. Any other files you modified during cleanup

The user will review and commit these changes manually.
{{/if}}

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
- **Concise entries**: History entries should be brief but complete

---

## Output

After completing all steps, print a structured summary to stdout:

**Format**:
```
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
🎉 Finalization Complete: {{prd_id}} — {{prd_title}}
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✅ UATs: XXX/XXX tests passed
✅ Tasks: N tasks completed (T-001 through T-NNN)
✅ Cleanup: [Summary of cleanup or "None required"]
{{#if commit}}
✅ Commit: [commit_hash] — prd({{prd_id}})finalize: [description]
{{else}}
⏸️ Commit: Skipped (--no-commit flag active) — changes left for manual review
{{/if}}

━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
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
{{#if allow_skip_uat}}

### Option D: Mark as Skipped

If this UAT genuinely cannot be automated or verified (e.g., requires manual testing, depends on external services not available in CI, or is not feasible to test programmatically), you may permanently skip it:
1. Update `uat_status: skipped` for `{{uat_id}}` in the PRD frontmatter.
2. Append a History entry with a clear justification for why the UAT was skipped.
3. Respond with `SKIPPED:` followed by your justification on a single line.

**Important**: Skipping is a **terminal state** — the UAT will not be retried. Only skip when verification is truly infeasible. If a new task could unblock verification, prefer Option E (if available) over skipping.
{{/if}}
{{#if allow_add_task}}

### Option E: Add a Task to Unblock Verification

If this UAT cannot currently be verified but a new task could unblock it (e.g., missing test infrastructure, prerequisite not implemented, or additional setup needed):
1. Add a new task to the PRD's `tasks` array in the YAML frontmatter.
2. Assign the next available `T-XXX` ID, set `status: todo`, and include descriptive `title` and `notes`.
3. Leave `uat_status` as `unverified` — the UAT will be retried after the new task is completed.
4. Append a History entry explaining what task was added and why.
5. Respond with `OPT-OUT: Added task T-XXX to unblock this UAT` so the run loop knows to proceed.

**Prefer this over skipping**: When a UAT could be verified with additional work, adding a task is better than permanently skipping the UAT.

**Example:**
```yaml
tasks:
  # ... existing tasks ...
  - id: T-009
    title: "Add test helper for external service mock"
    priority: 9
    status: todo
    notes: "Added during uat-003 verification: needed to mock external API for UAT."
```
{{/if}}

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
{{#if allow_skip_uat}}

If skipping (Option D), update to:

```yaml
acceptance_tests:
  - id: {{uat_id}}
    name: "{{uat_name}}"
    command: {{uat_command}}
    uat_status: skipped  # <-- Change from 'unverified' to 'skipped'
```
{{/if}}

### Append to History Section

Add a History entry documenting your verification attempt:

```markdown
## YYYY-MM-DD — {{uat_id}} Verification
- **UAT**: {{uat_name}}
- **Status**: ✅ Verified (or ⏭️ Opted-out{{#if allow_skip_uat}} or ⏭️ Skipped{{/if}})
- **Method**: [Existing test / New test / Opt-out{{#if allow_skip_uat}} / Skipped{{/if}}]
- **Details**:
  - [Test file and name if applicable]
  - [Explanation if opted out or skipped]
```

## Constraints

- Focus on this single UAT (`{{uat_id}}`). Do not verify other UATs in this invocation.
- Keep test code minimal — just enough to cover the acceptance criterion.
- Always update the PRD even if opting out or skipping (document your reasoning).

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
{{#if allow_skip_uat}}

## On Skip

If skipping:
1. Update `uat_status: skipped` in the PRD frontmatter.
2. Append a skip History entry with clear justification.
3. Respond with `SKIPPED: [your justification]` so the run loop knows to proceed.
4. Commit with message: `prd({{prd_id}})uat({{uat_id}}): skipped — [brief justification]`
{{/if}}

## Output

Report what happened:
- Whether verification succeeded, opted out, or was skipped
- What approach was used (existing test, new test, opt-out, skip, or task addition)
- Test details or opt-out/skip explanation
- What was committed (if anything)
"#;

/// Default content for the PRD edit interactive prompt.
///
/// This is a single-phase prompt: the agent reads the existing PRD, chats with
/// the user about desired changes, then writes the updated PRD directly to disk.
/// The Rust side then validates the file and regenerates the index.
pub const PROMPT_PRD_EDIT_INTERACTIVE: &str = r#"# microralph — PRD Edit Interactive Prompt

## Objective

Have an interactive conversation with the user to understand what changes they want to make to an existing PRD, then write the updated PRD directly to disk.

## Context

The user wants to edit the PRD at `{{prd_path}}`.

{{#if context}}
User's upfront context:
> {{context}}
{{/if}}

{{#if constitution}}
## Project Constitution

The following governance rules and constraints apply to this project:

{{constitution}}

**CRITICAL**: The updated PRD MUST respect these constitutional rules. If any aspect of the edit would violate the constitution, adjust the approach or note the constraint explicitly.
{{/if}}

## Current PRD Content

```markdown
{{prd_content}}
```

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}} ({{status}})
{{/each}}

## Phase 1: Interactive Discovery

1. Review the current PRD content carefully.
2. If the user provided upfront context, use it to understand what they want to change.
3. Engage the user in a natural conversation to understand:
   - What specific changes do they want to make?
   - Should tasks be added, removed, or modified?
   - Should acceptance tests be updated?
   - Are there scope or priority changes?
4. Ask follow-up questions based on the user's responses.
5. Reference existing PRDs and the current PRD content when relevant.

## Phase 2: Write the Updated PRD

When you have enough information, tell the user you're ready to apply the changes. Then:

1. **Write the updated PRD file** directly to `{{prd_path}}` using your file editing tools.
2. The PRD MUST preserve the existing template structure.
3. After writing the file, tell the user the PRD has been updated and they can exit the chat.

## Constraints

- Do NOT change the PRD ID.
- Do NOT remove existing History entries.
- Keep the overall structure intact (frontmatter, Summary, Problem, Goals, Non-Goals, History sections).
- If adding tasks, assign appropriate IDs (T-NNN) and priorities.
- If adding acceptance tests, assign appropriate IDs (uat-NNN).
- **YAML Quoting**: Strings containing colons (`:`) or hashes (`#`) MUST be quoted to avoid parse errors. Example: `title: "Fix: Bug in parser"`

## Important

- The PRD file MUST be written to `{{prd_path}}`.
- Do NOT just output the PRD content to the chat — you MUST write it to disk using your file tools.
- After writing the file, tell the user the PRD is updated and they can exit.
"#;

/// Default content for the constitution edit prompt.
pub const PROMPT_CONSTITUTION_EDIT: &str = r"# microralph — Constitution Edit Prompt

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

If you have enough information, **directly edit the `.mr/constitution.md` file** using your file editing tools to apply the changes. After making the edits, respond with exactly `EDIT_COMPLETE` on its own line to signal that the changes have been applied.
";

/// Default content for the language adaptation prompt.
pub const PROMPT_ADAPT_LANGUAGE: &str = r"# microralph — Adapt Language Prompt

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
";

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

/// Default content for the reindex `depends_on` prompt.
pub const PROMPT_REINDEX_DEPENDS_ON: &str = r#"# microralph — Reindex Depends On Prompt

## Objective

Analyze all PRDs and infer/fix `depends_on` relationships based on content, dates, and logical dependencies.

## Context

The user wants to auto-populate or fix the `depends_on` field in PRD frontmatter. This field represents directed edges: "this PRD should be done after the dependencies".

## PRDs Directory

Path: `{{prds_dir}}`

## Current PRD Files

{{#each prd_files}}
### {{id}}: {{title}}
- **File**: `{{filename}}`
- **Status**: {{status}}
- **Created**: {{created}}
- **Depends On**: {{#if depends_on}}{{depends_on}}{{else}}(none){{/if}}
- **Summary**: {{summary}}
{{/each}}

## Repository Root

Path: `{{repo_root}}`

## Required Actions

1. **Read all PRD files** in `{{prds_dir}}` to understand their content, goals, and relationships.

2. **Analyze dependencies** by considering:
   - **Temporal order**: Earlier PRDs (by creation date or ID) often are dependencies
   - **Content references**: If PRD-B mentions concepts or files introduced by PRD-A, PRD-B likely depends on PRD-A
   - **Logical progression**: Foundation/infrastructure PRDs are dependencies for feature PRDs
   - **Explicit mentions**: References to other PRD IDs in body text suggest dependencies

3. **Infer missing depends_on** relationships:
   - For PRDs with empty `depends_on`, analyze their content to determine likely dependencies
   - Be conservative: only add dependencies that are clearly implied
   - Don't create circular dependencies

4. **Fix existing depends_on** relationships:
   - Verify that referenced PRD IDs actually exist
   - Remove references to non-existent PRDs
   - Add missing dependencies that are clearly implied by content

5. **Update PRD files** with corrected `depends_on` fields:
   - Modify only the frontmatter YAML, preserving all other content
   - Use the format: `depends_on: ["PRD-0001", "PRD-0003"]`
   - Keep the list sorted by PRD ID

## Dependency Inference Guidelines

### Strong indicators of dependency:
- PRD-B explicitly mentions implementing something "from PRD-A"
- PRD-B modifies files first created in PRD-A
- PRD-B's tasks require PRD-A's completed work
- PRD-B's acceptance tests depend on PRD-A's features

### Weak indicators (use cautiously):
- PRD-B was created after PRD-A (not sufficient alone)
- PRD-B works on the same module as PRD-A (may be parallel, not dependent)
- Similar topic areas (may be unrelated)

### What NOT to infer:
- Don't add dependencies just because PRDs touch the same file
- Don't create long dependency chains unnecessarily
- Don't assume all earlier PRDs are dependencies
- Avoid circular dependencies (PRD-A → PRD-B → PRD-A)

## Constraints

- Only modify the `depends_on` field in frontmatter
- Preserve all other frontmatter fields exactly
- Preserve the body content exactly
- Keep History sections intact
- Don't add dependencies that create cycles

## Output

Report what was done:
- Number of PRDs analyzed
- Number of depends_on relationships added
- Number of depends_on relationships fixed (invalid refs removed)
- List of changes made (e.g., "PRD-0005: added depends_on PRD-0003")
"#;

/// Default content for the pick PRD prompt.
pub const PROMPT_PICK_PRD: &str = r"# microralph — Pick PRD Prompt

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
";

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

## Constraints

- Use only official Microsoft dev container base images
- Include only extensions that are actively maintained and widely used
- Keep post-create commands minimal and fast (prefer lazy installation)
- Do not include secrets or credentials in the configuration
- Ensure the JSON is valid and properly formatted

---

## Task

**Create the file `.devcontainer/devcontainer.json` directly** with the generated configuration.

Use your file creation tools to write the file. Do not just output the JSON content - actually create the file in the repository.
"#;

/// Default suggest generation prompt.
pub const PROMPT_SUGGEST_GENERATE: &str = r"# microralph — Suggest Generation Prompt

## Objective

Analyze the codebase, existing PRDs (especially completed ones), and external research to generate exactly 5 strategic PRD suggestions that balance quick wins with longer-term improvements.

## Context

You are analyzing a project to suggest new PRDs that would improve the codebase, fix technical debt, enhance features, or align with ecosystem best practices.

### Existing PRDs

Below are the existing PRDs in the project:

```
{{existing_prds}}
```

### Recent Completions

Pay special attention to recently completed PRDs—they may reveal patterns, gaps, or follow-up opportunities.

### Codebase Snapshot

Here is a snapshot of the repository structure and key files:

```
{{codebase_snapshot}}
```

## Analysis Strategy

Use a multi-faceted approach to identify opportunities:

1. **Pattern Analysis**: Look across completed PRDs for themes or missing pieces
2. **Technical Debt**: Scan for TODO comments, deprecated patterns, or outdated dependencies
3. **Quick Wins**: Identify low-effort, high-value improvements (e.g., missing flags, better error messages)
4. **Strategic Features**: Consider ecosystem trends and best practices (e.g., telemetry, observability)
5. **Test Coverage**: Identify gaps in testing or CI/CD workflows
6. **Documentation**: Find areas where docs are missing or outdated
7. **External Research**: Consider recent developments in relevant ecosystems (Rust, CLI tools, AI agents)

## Requirements

Generate **exactly 5 suggestions** in the following format:

```
1. [Title] — [One-line description]
   Category: [Quick Win | Strategic | Debt | Testing | Docs]
   Effort: [Low | Medium | High]
   Rationale: [2-3 sentences explaining the value and approach]

2. [Title] — [One-line description]
   ...
```

Each suggestion must:
- Be **actionable and scoped** (suitable for a PRD)
- Include a clear **category** (Quick Win, Strategic, Debt, Testing, or Docs)
- Estimate **effort** realistically
- Provide **rationale** that references specific gaps or opportunities

Balance the suggestions:
- Include at least 1-2 **Quick Wins** (low-hanging fruit)
- Include at least 1-2 **Strategic** features (longer-term value)
- Consider **Debt**, **Testing**, or **Docs** for remaining slots

## Output Format

Return the 5 suggestions in plain text using the format above. Do NOT use markdown headings or extra formatting. Just numbered entries.

## Constraints

- Suggest features that fit the project's scope and principles
- Do not suggest features that duplicate existing PRDs
- Prioritize improvements that align with completed work or stated goals
- Keep suggestions realistic and implementable

---

Generate exactly 5 PRD suggestions now.
";

/// Default content for the refactor prompt.
pub const PROMPT_REFACTOR: &str = r"# microralph — Refactor Prompt

## Objective

Identify one impactful code improvement, apply it, verify UATs pass, and commit (if allowed).

## Context

You are performing iteration {{iteration}} of {{max_iterations}} in a refactor loop.

{{#if context}}
### Focus Hint

The user has requested you focus on: **{{context}}**

This takes priority over general constitution-based discovery.
{{/if}}

{{#if path}}
### Scope Constraint

Limit your changes to files within: `{{path}}`
{{/if}}

{{#if constitution}}
### Constitution

The project's constitution defines behavioral rules and constraints:

```markdown
{{constitution}}
```

Use these rules to guide your refactor selection when no specific focus hint is provided.
{{/if}}

## Task

1. **Analyze** the codebase for one impactful refactor opportunity
2. **Apply** the change with minimal modifications
3. **Verify** by running `cargo make uat`
4. **Commit** with message format: `refactor: [brief description]`

{{#if preview}}
### Preview Mode

This is a **preview**. Do NOT apply changes.

Instead, output your suggested refactor in this format:

```
REFACTOR SUGGESTION:
File: [path/to/file.rs]
Lines: [start-end]
Description: [What would be changed and why]
Impact: [Expected benefit]
```

After outputting the suggestion, respond with `PREVIEW-COMPLETE` on a new line.
{{/if}}

{{#if no_commit}}
### No-Commit Mode

Do NOT commit changes. Leave them staged or unstaged for manual review.
{{/if}}

## Early Termination

If you find no impactful refactors remaining (codebase already adheres well to principles), respond with exactly:

```
NO-MORE-REFACTORS
```

This signals early termination of the refactor loop.

## Constraints

- Make **one** focused change per iteration
- Keep changes minimal and surgical
- Do not fix unrelated issues
- Follow existing code style and conventions
- Run UATs to verify changes don't break anything

## Output

After completing (or in preview mode, after suggesting), summarize what you did.
";

/// Default content for the bootstrap reconstruct prompt.
pub const PROMPT_BOOTSTRAP_RECONSTRUCT: &str = r#"# microralph — Bootstrap Reconstruct Prompt

## Objective

Analyze the repository's git history (commits, tags, and major changes) to infer historical PRDs representing major development milestones.

## Context

You are running `mr bootstrap` on an existing repository that has development history but no PRDs. Reconstruct mode is the default behavior. Your goal is to create PRDs that represent the major milestones and features that have already been completed.

## Required Analysis

1. **Analyze Git History**
   - Review commit messages for patterns indicating features, fixes, and milestones
   - Identify version tags and releases
   - Look for merge commits indicating major feature branches
   - Detect clusters of related commits that represent cohesive work

2. **Identify Major Milestones**
   - Focus on significant features, not incremental bug fixes
   - Look for architectural changes, new modules, or major refactors
   - Prioritize work that would naturally be described as a "project" or "epic"
   - Aim for 3-10 PRDs representing the most significant completed work

3. **Infer Temporal Dependencies**
   - Earlier milestones should be dependencies of later ones where logical
   - Use commit timestamps and tag ordering to establish sequence
   - Foundation work (setup, core modules) should be dependencies of features built on them

4. **Consider Existing Structure**
   - Check for existing PRDs in `.mr/prds/` and avoid duplicating their scope
   - Review README, CHANGELOG, and documentation for feature descriptions
   - Look at module structure to understand architectural evolution

## Existing PRDs (Do Not Duplicate)

{{#if existing_prds}}
The following PRDs already exist in this repository. **Do NOT create new PRDs that duplicate their scope.** Only create new PRDs for completed work not already covered by these:

{{#each existing_prds}}
- **{{id}}**: {{title}} ({{status}})
{{/each}}

When inferring dependencies for new PRDs, you may reference these existing PRD IDs in the `depends_on` field.
{{/if}}
{{#unless existing_prds}}
No existing PRDs found. You may create PRDs for all significant completed work.
{{/unless}}

## Output Format

For each inferred PRD, create a file in `.mr/prds/` with:

### Frontmatter

```yaml
---
id: PRD-NNNN
title: "Descriptive title of the milestone"
status: done
owner: "{{owner}}"
created: YYYY-MM-DD   # First commit date for this work
updated: YYYY-MM-DD   # Last commit date for this work
reconstructed: true   # Mark as reconstructed from history

depends_on:           # PRD IDs that this work built upon
- PRD-XXXX
- PRD-YYYY

principles:
- Key principle or constraint that guided this work

acceptance_tests:
- id: uat-001
  name: Primary acceptance criteria (inferred from the completed work)
  command: cargo make uat
  uat_status: verified   # Assume verified since work is complete

tasks:
- id: T-001
  title: "Main task that was completed"
  priority: 1
  status: done
  notes: "Inferred from commits/tags"
---
```

### Markdown Body

Include:
- **Summary**: Brief description of what was accomplished
- **Problem**: What problem this milestone solved (inferred)
- **Goals**: What goals were achieved
- **History**: Single entry noting reconstruction

```markdown
# Summary

Brief description of the completed milestone.

# Problem

The problem or need this work addressed.

# Goals

1. Goal that was achieved
2. Another achieved goal

# History

## YYYY-MM-DD — Reconstructed from Git History
- **Source**: Inferred from commits [hash1, hash2, ...] and/or tag [vX.Y.Z]
- **Status**: ✅ Reconstructed
- **Notes**: This PRD was automatically generated by `mr bootstrap`
```

## Constraints

- Only create PRDs for **completed work** with `status: done`
- Always set `reconstructed: true` in the frontmatter
- Infer `depends_on` based on temporal and logical dependencies
- Use realistic dates from git history for `created` and `updated`
- Keep task lists minimal (1-3 tasks per PRD representing main accomplishments)
- Do not recreate work already covered by existing PRDs

## YAML Frontmatter Quoting Rules

**CRITICAL**: YAML strings containing special characters MUST be quoted:
- **Colons (`:`)**: Any string with a colon must be quoted: `title: "Fix: Bug in parser"`
- **Hashes (`#`)**: Strings with `#` must be quoted to avoid comment interpretation
- When in doubt, wrap string values in double quotes.

## After Generation

1. Run `mr list` to regenerate the PRD index
2. The new PRDs will appear as "done" in the index
3. Future `mr new` PRDs can reference these as dependencies

## Output

After creating the reconstructed PRDs, report:
- How many PRDs were created
- Brief summary of each (title and date range)
- The dependency graph that was inferred
"#;

/// Default content for the empty PRDS.md index.
pub const EMPTY_INDEX: &str = r"# microralph — PRD Index

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
";

/// Default content for the empty SKILLS.md manifest.
pub const SKILLS_TEMPLATE: &str = r"# Skills

<!-- This file is auto-managed by the run agent. Each entry is a one-line summary. -->
<!-- Read .mr/skills/<name>/skill.md for full details on any skill. -->
";

/// Default content for the starter AGENTS.md file.
pub const STARTER_AGENTS: &str = r"# Agents Guide

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
";

/// Mapping of prompt filenames to their content.
const PROMPT_FILES: &[(&str, &str)] = &[
    ("init.md", PROMPT_INIT),
    ("bootstrap_plan.md", PROMPT_BOOTSTRAP_PLAN),
    ("bootstrap_generate_prds.md", PROMPT_BOOTSTRAP_GENERATE_PRDS),
    ("prd_new_interactive.md", PROMPT_PRD_NEW_INTERACTIVE),
    ("run_task.md", PROMPT_RUN_TASK),
    ("run_task_finalize.md", PROMPT_RUN_TASK_FINALIZE),
    ("run_uat_verify.md", PROMPT_RUN_UAT_VERIFY),
    ("prd_edit_interactive.md", PROMPT_PRD_EDIT_INTERACTIVE),
    ("constitution_edit.md", PROMPT_CONSTITUTION_EDIT),
    ("devcontainer_generate.md", PROMPT_DEVCONTAINER_GENERATE),
    ("adapt_language.md", PROMPT_ADAPT_LANGUAGE),
    ("reindex.md", PROMPT_REINDEX),
    ("pick_prd.md", PROMPT_PICK_PRD),
    ("suggest_generate.md", PROMPT_SUGGEST_GENERATE),
    ("refactor.md", PROMPT_REFACTOR),
    ("bootstrap_reconstruct.md", PROMPT_BOOTSTRAP_RECONSTRUCT),
    ("reindex_depends_on.md", PROMPT_REINDEX_DEPENDS_ON),
];

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

    create_dir_if_missing(&prds_dir, &mut result)?;

    // Create .mr/skills directory for persistent agent skills.
    let skills_result = init_skills(root)?;
    result.dirs_created += skills_result.dirs_created;
    result.files_created += skills_result.files_created;
    result.files_skipped += skills_result.files_skipped;
    result.created_paths.extend(skills_result.created_paths);
    result.skipped_paths.extend(skills_result.skipped_paths);

    // Initialize prompts and templates (skips existing files).
    let prompts_templates_dir = mr_dir.join("templates");
    create_dir_if_missing(&prompts_templates_dir, &mut result)?;
    create_file_if_missing(
        &prompts_templates_dir.join("prd.md"),
        PRD_TEMPLATE,
        &mut result,
    )?;

    let prompts_dir = mr_dir.join("prompts");
    create_dir_if_missing(&prompts_dir, &mut result)?;

    for (filename, content) in PROMPT_FILES {
        create_file_if_missing(&prompts_dir.join(filename), content, &mut result)?;
    }

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

/// Recreates prompts and templates directories with built-in defaults.
///
/// This function:
/// 1. Creates the prompts and templates directories (if they don't exist)
/// 2. Writes all built-in prompt and template files, overwriting existing ones
///
/// Used by both `init()` (initial setup) and `cmd_restore()` (restoration).
pub fn init_prompts_and_templates(root: impl AsRef<Path>) -> Result<InitResult> {
    let root = root.as_ref();
    let mut result = InitResult::default();

    let mr_dir = root.join(".mr");
    let templates_dir = mr_dir.join("templates");
    let prompts_dir = mr_dir.join("prompts");

    // Ensure directories exist.
    create_dir_if_missing(&templates_dir, &mut result)?;
    create_dir_if_missing(&prompts_dir, &mut result)?;

    // Write template file (always overwrite).
    create_file_always(&templates_dir.join("prd.md"), PRD_TEMPLATE, &mut result)?;

    // Write all prompt files (always overwrite).
    for (filename, content) in PROMPT_FILES {
        create_file_always(&prompts_dir.join(filename), content, &mut result)?;
    }

    Ok(result)
}

/// Recreates constitution.md and config.toml with built-in defaults.
///
/// This function overwrites existing files (no skip behavior).
///
/// Used by `cmd_restore()` (restoration).
pub fn init_constitution_and_config(root: impl AsRef<Path>) -> Result<InitResult> {
    let root = root.as_ref();
    let mut result = InitResult::default();

    let mr_dir = root.join(".mr");

    // Write constitution.md (always overwrite).
    create_file_always(
        &mr_dir.join("constitution.md"),
        CONSTITUTION_TEMPLATE,
        &mut result,
    )?;

    // Write config.toml (always overwrite).
    create_file_always(
        &mr_dir.join("config.toml"),
        config::DEFAULT_CONFIG,
        &mut result,
    )?;

    Ok(result)
}

/// Creates `.mr/skills/` directory and `SKILLS.md` if they do not already exist.
///
/// Uses `create_dir_if_missing` and `create_file_if_missing` to preserve
/// existing learned skills. Used by both `init()` and `restore_impl()`.
pub fn init_skills(root: impl AsRef<Path>) -> Result<InitResult> {
    let root = root.as_ref();
    let mut result = InitResult::default();

    let skills_dir = root.join(".mr").join("skills");
    create_dir_if_missing(&skills_dir, &mut result)?;
    create_file_if_missing(&skills_dir.join("SKILLS.md"), SKILLS_TEMPLATE, &mut result)?;

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
    let relative = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().to_string(),
    );

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

/// Creates a file, always overwriting if it exists.
fn create_file_always(path: &Path, content: &str, result: &mut InitResult) -> Result<()> {
    let relative = path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().to_string(),
    );

    std::fs::write(path, content)
        .with_context(|| format!("Failed to create file: {}", path.display()))?;
    result.files_created += 1;
    result.created_paths.push(relative);
    tracing::debug!(path = %path.display(), "Created file");

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

/// Ensures the project is initialized, returning an error if not.
pub fn ensure_initialized(root: impl AsRef<Path>) -> Result<()> {
    let root = root.as_ref();

    if is_initialized(root) {
        return Ok(());
    }

    let mr_dir = root.join(".mr");

    if mr_dir.exists() {
        // Partial init: .mr/ exists but some subdirectories/files are missing.
        let missing: Vec<&str> = ["prds", "templates", "prompts", "PRDS.md"]
            .iter()
            .filter(|name| !mr_dir.join(name).exists())
            .copied()
            .collect();

        anyhow::bail!(
            "microralph is partially initialized — missing: {}.\n  Suggestion: Run `mr init` to complete initialization, or `mr restore` to reset to defaults.",
            missing.join(", ")
        );
    }

    anyhow::bail!("microralph is not initialized. Run `mr init` first.");
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
        assert!(root.join(".mr/skills").exists());
        assert!(root.join(".mr/templates").exists());
        assert!(root.join(".mr/prompts").exists());

        // Check skills manifest exists.
        assert!(root.join(".mr/skills/SKILLS.md").exists());

        // Check template exists.
        assert!(root.join(".mr/templates/prd.md").exists());

        // Check prompts exist.
        assert!(root.join(".mr/prompts/init.md").exists());
        assert!(root.join(".mr/prompts/bootstrap_plan.md").exists());
        assert!(root.join(".mr/prompts/bootstrap_generate_prds.md").exists());
        assert!(root.join(".mr/prompts/prd_new_interactive.md").exists());
        assert!(root.join(".mr/prompts/run_task.md").exists());
        assert!(root.join(".mr/prompts/run_task_finalize.md").exists());
        assert!(root.join(".mr/prompts/run_uat_verify.md").exists());
        assert!(root.join(".mr/prompts/adapt_language.md").exists());
        assert!(root.join(".mr/prompts/reindex.md").exists());
        assert!(root.join(".mr/prompts/pick_prd.md").exists());
        assert!(root.join(".mr/prompts/suggest_generate.md").exists());
        assert!(root.join(".mr/prompts/refactor.md").exists());
        assert!(root.join(".mr/prompts/reindex_depends_on.md").exists());

        // Check index exists.
        assert!(root.join(".mr/PRDS.md").exists());

        // Check config.toml exists.
        assert!(root.join(".mr/config.toml").exists());

        // Check constitution.md exists.
        assert!(root.join(".mr/constitution.md").exists());

        // Check AGENTS.md exists.
        assert!(root.join("AGENTS.md").exists());

        // Check result counts.
        assert_eq!(result.dirs_created, 4);
        assert_eq!(result.files_created, 23); // 1 template + 17 prompts + 1 index + 1 config + 1 constitution + 1 AGENTS.md + 1 SKILLS.md
        assert_eq!(result.files_skipped, 0);
    }

    #[test]
    fn test_init_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // First init.
        let result1 = init(root).unwrap();
        assert_eq!(result1.files_created, 23);
        assert_eq!(result1.files_skipped, 0);

        // Second init should skip all files.
        let result2 = init(root).unwrap();
        assert_eq!(result2.files_created, 0);
        assert_eq!(result2.files_skipped, 23);
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
    fn test_ensure_initialized_partial_init_includes_suggestion() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Create only .mr/ and prds/ — missing templates, prompts, PRDS.md.
        std::fs::create_dir_all(root.join(".mr/prds")).unwrap();

        let err = ensure_initialized(root).unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("partially initialized"),
            "Expected 'partially initialized' in: {msg}"
        );
        assert!(
            msg.contains("templates"),
            "Expected missing 'templates' in: {msg}"
        );
        assert!(
            msg.contains("prompts"),
            "Expected missing 'prompts' in: {msg}"
        );
        assert!(
            msg.contains("PRDS.md"),
            "Expected missing 'PRDS.md' in: {msg}"
        );
        assert!(
            msg.contains("mr init"),
            "Expected 'mr init' suggestion in: {msg}"
        );
        assert!(
            msg.contains("mr restore"),
            "Expected 'mr restore' suggestion in: {msg}"
        );
    }

    #[test]
    fn test_ensure_initialized_no_mr_dir_generic_message() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        let err = ensure_initialized(root).unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("not initialized"),
            "Expected 'not initialized' in: {msg}"
        );
        assert!(
            !msg.contains("partially"),
            "Should not say 'partially' when .mr/ doesn't exist: {msg}"
        );
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
        // Check for the comprehensive default rules
        assert!(CONSTITUTION_TEMPLATE.contains("1. **Single Source of Truth**"));
        assert!(CONSTITUTION_TEMPLATE.contains("2. **Separation of Concerns**"));
        assert!(CONSTITUTION_TEMPLATE.contains("3. **Minimal Changes**"));
        assert!(CONSTITUTION_TEMPLATE.contains("4. **Consistency**"));
        assert!(CONSTITUTION_TEMPLATE.contains("5. **Public API Stability**"));
        assert!(CONSTITUTION_TEMPLATE.contains("6. **Root Cause Resolution**"));
    }

    #[test]
    fn test_constitution_template_has_under_10_rules() {
        // UAT: Constitution has comprehensive behavior rules (under 10 rules)
        // Count numbered rules in the template (pattern: "N. **Rule Name**")
        let rule_count = CONSTITUTION_TEMPLATE
            .lines()
            .filter(|line| {
                let trimmed = line.trim();
                trimmed.starts_with(|c: char| c.is_ascii_digit()) && trimmed.contains(". **")
            })
            .count();

        assert!(rule_count > 0, "Constitution should have at least one rule");
        assert!(
            rule_count < 10,
            "Constitution should have fewer than 10 rules, found {rule_count}"
        );
    }

    #[test]
    fn test_prompts_are_workflow_focused_no_philosophy() {
        // UAT: Prompts are workflow-focused with no philosophy or opinion
        // Behavioral/philosophical terms should be in constitution, not prompts

        let prompts: &[(&str, &str)] = &[
            ("PROMPT_INIT", PROMPT_INIT),
            ("PROMPT_BOOTSTRAP_PLAN", PROMPT_BOOTSTRAP_PLAN),
            (
                "PROMPT_BOOTSTRAP_GENERATE_PRDS",
                PROMPT_BOOTSTRAP_GENERATE_PRDS,
            ),
            ("PROMPT_PRD_NEW_INTERACTIVE", PROMPT_PRD_NEW_INTERACTIVE),
            ("PROMPT_RUN_TASK", PROMPT_RUN_TASK),
            ("PROMPT_RUN_TASK_FINALIZE", PROMPT_RUN_TASK_FINALIZE),
            ("PROMPT_RUN_UAT_VERIFY", PROMPT_RUN_UAT_VERIFY),
            ("PROMPT_PRD_EDIT_INTERACTIVE", PROMPT_PRD_EDIT_INTERACTIVE),
            ("PROMPT_CONSTITUTION_EDIT", PROMPT_CONSTITUTION_EDIT),
            ("PROMPT_ADAPT_LANGUAGE", PROMPT_ADAPT_LANGUAGE),
            ("PROMPT_REINDEX", PROMPT_REINDEX),
            ("PROMPT_PICK_PRD", PROMPT_PICK_PRD),
            ("PROMPT_DEVCONTAINER_GENERATE", PROMPT_DEVCONTAINER_GENERATE),
            ("PROMPT_SUGGEST_GENERATE", PROMPT_SUGGEST_GENERATE),
            ("PROMPT_REFACTOR", PROMPT_REFACTOR),
            ("PROMPT_BOOTSTRAP_RECONSTRUCT", PROMPT_BOOTSTRAP_RECONSTRUCT),
            ("PROMPT_REINDEX_DEPENDS_ON", PROMPT_REINDEX_DEPENDS_ON),
        ];

        // Philosophical/behavioral terms that should only appear in constitution
        // These are the patterns identified in PRD-0019 T-001 audit
        let forbidden_patterns = [
            "DRY",                   // Should be in constitution Rule 1
            "Don't Repeat Yourself", // Expanded form of DRY
            "minimal change",        // Should be in constitution Rule 3
            "unrelated code",        // Part of minimal changes rule
            "how to behave",         // Philosophical guidance
        ];

        for (name, prompt) in prompts {
            let lowercase = prompt.to_lowercase();
            for pattern in &forbidden_patterns {
                let pattern_lower = pattern.to_lowercase();
                assert!(
                    !lowercase.contains(&pattern_lower),
                    "Prompt {name} contains philosophical term '{pattern}'. \
                     Behavioral guidance should be in constitution, not prompts."
                );
            }
        }
    }

    #[test]
    fn test_prompts_contain_placeholders() {
        // Interactive prompt should have slug and next_id placeholders.
        assert!(PROMPT_PRD_NEW_INTERACTIVE.contains("{{slug}}"));
        assert!(PROMPT_PRD_NEW_INTERACTIVE.contains("{{next_id}}"));
        assert!(PROMPT_PRD_NEW_INTERACTIVE.contains("{{prd_path}}"));

        // Run task should have prd_path placeholder.
        assert!(PROMPT_RUN_TASK.contains("{{prd_path}}"));

        // Run UAT verify should have uat_id and prd_id placeholders.
        assert!(PROMPT_RUN_UAT_VERIFY.contains("{{uat_id}}"));
        assert!(PROMPT_RUN_UAT_VERIFY.contains("{{prd_id}}"));
        assert!(PROMPT_RUN_UAT_VERIFY.contains("{{prd_path}}"));

        // Edit interactive prompt should have prd_path, prd_content.
        assert!(PROMPT_PRD_EDIT_INTERACTIVE.contains("{{prd_path}}"));
        assert!(PROMPT_PRD_EDIT_INTERACTIVE.contains("{{prd_content}}"));
    }

    #[test]
    fn test_run_task_prompt_includes_skills_sections() {
        // Skills manifest conditional section.
        assert!(PROMPT_RUN_TASK.contains("{{#if skills_manifest}}"));
        assert!(PROMPT_RUN_TASK.contains("## Available Skills"));
        assert!(PROMPT_RUN_TASK.contains("{{skills_manifest}}"));

        // Skill-saving instructions section.
        assert!(PROMPT_RUN_TASK.contains("## Saving Skills (End-of-Task)"));
        assert!(PROMPT_RUN_TASK.contains(".mr/skills/"));
        assert!(PROMPT_RUN_TASK.contains("SKILLS.md"));
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

    #[test]
    fn test_init_skills_creates_dir_and_manifest() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Pre-create .mr/ so init_skills can create skills/ inside it.
        std::fs::create_dir_all(root.join(".mr")).unwrap();

        let result = init_skills(root).unwrap();

        assert!(root.join(".mr/skills").exists());
        assert!(root.join(".mr/skills/SKILLS.md").exists());
        assert_eq!(result.dirs_created, 1);
        assert_eq!(result.files_created, 1);

        let content = std::fs::read_to_string(root.join(".mr/skills/SKILLS.md")).unwrap();
        assert_eq!(content, SKILLS_TEMPLATE);
    }

    #[test]
    fn test_init_skills_preserves_existing() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Pre-create skills dir with custom content.
        let skills_dir = root.join(".mr/skills");
        std::fs::create_dir_all(&skills_dir).unwrap();
        let custom = "# Skills\n\n- **my-skill**: A custom skill.\n";
        std::fs::write(skills_dir.join("SKILLS.md"), custom).unwrap();

        let result = init_skills(root).unwrap();

        // Should not overwrite.
        assert_eq!(result.dirs_created, 0);
        assert_eq!(result.files_created, 0);
        assert_eq!(result.files_skipped, 1);

        let content = std::fs::read_to_string(skills_dir.join("SKILLS.md")).unwrap();
        assert_eq!(content, custom);
    }

    #[test]
    fn test_init_skills_idempotent() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join(".mr")).unwrap();

        let r1 = init_skills(root).unwrap();
        assert_eq!(r1.dirs_created, 1);
        assert_eq!(r1.files_created, 1);

        let r2 = init_skills(root).unwrap();
        assert_eq!(r2.dirs_created, 0);
        assert_eq!(r2.files_created, 0);
        assert_eq!(r2.files_skipped, 1);
    }
}
