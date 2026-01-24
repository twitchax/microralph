//! Initialization logic for `mr init`.
//!
//! Creates the `.mr/` directory structure, templates, prompts, and starter AGENTS.md.

use std::path::Path;

use anyhow::{Context, Result};

/// Default content for the PRD template.
pub const PRD_TEMPLATE: &str = r#"---
id: PRD-NNNN
title: "{{title}}"
status: draft                 # draft | active | done | parked
owner: "{{owner}}"
created: {{date}}
updated: {{date}}

tags: []

acceptance_tests: []

tasks: []

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

/// Default content for the init prompt.
pub const PROMPT_INIT: &str = r#"# Micro Ralph — Init Prompt

## Objective

Initialize a new repository with Micro Ralph structure.

## Context

You are initializing a new repository for use with Micro Ralph (`mr`).

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
pub const PROMPT_BOOTSTRAP_PLAN: &str = r#"# Micro Ralph — Bootstrap Plan Prompt

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
pub const PROMPT_BOOTSTRAP_GENERATE_PRDS: &str = r#"# Micro Ralph — Bootstrap Generate PRDs Prompt

## Objective

Generate PRDs based on the bootstrap plan.

## Context

You have analyzed the repository and created a plan. Now generate the PRD files.

## Plan

{{plan}}

## Required Actions

1. For each proposed PRD in the plan:
   - Generate a complete PRD file with YAML frontmatter
   - Include tasks with IDs, titles, priorities, and status
   - Add acceptance tests where applicable

2. Ensure PRD IDs are sequential (PRD-0001, PRD-0002, etc.)

3. Follow the PRD template format.

## Output

Generate the complete content for each PRD file.
"#;

/// Default content for the PRD new round 1 questions prompt.
pub const PROMPT_PRD_NEW_ROUND1: &str = r#"# Micro Ralph — PRD New Round 1 Questions Prompt

## Objective

Generate follow-up questions to clarify a new PRD request.

## Context

The user wants to create a new PRD with slug: `{{slug}}`

{{#if user_description}}
User's initial description:
> {{user_description}}
{{/if}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}} ({{status}})
{{/each}}

## Required Actions

1. Review the existing PRDs to understand context.
2. Generate 3-5 clarifying questions to understand:
   - What problem does this PRD solve?
   - What are the success criteria?
   - What are the acceptance tests?
   - What are the dependencies or blockers?
   - What is the scope (MVP vs full feature)?

## Output Format

Return a numbered list of questions. Keep questions concise and actionable.

Example:
1. What specific problem are you trying to solve?
2. What does "done" look like for this feature?
3. Are there any existing patterns in the codebase to follow?
"#;

/// Default content for the PRD new round N questions prompt.
pub const PROMPT_PRD_NEW_ROUNDN: &str = r#"# Micro Ralph — PRD New Round N Questions Prompt

## Objective

Continue the Q/A session for PRD creation, or signal readiness.

## Context

The user is creating a new PRD with slug: `{{slug}}`

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
pub const PROMPT_PRD_NEW_SYNTHESIZE: &str = r#"# Micro Ralph — PRD New Synthesize Prompt

## Objective

Synthesize a complete PRD from the Q/A session.

## Context

The user is creating a new PRD with slug: `{{slug}}`

## Q/A Session

{{#each qa_history}}
**Q**: {{question}}
**A**: {{answer}}

{{/each}}

## Existing PRDs

{{#each existing_prds}}
- {{id}}: {{title}}
{{/each}}

## Required Actions

1. Generate the next PRD ID (e.g., PRD-0002 if PRD-0001 exists).
2. Create a complete PRD file with:
   - YAML frontmatter (id, title, status, owner, created, updated, tasks)
   - Summary section
   - Problem section
   - Goals section
   - Non-Goals section (if applicable)
   - Acceptance Tests section
   - Empty History section

3. Tasks should:
   - Have unique IDs (T-001, T-002, etc.)
   - Have clear, actionable titles
   - Be prioritized (1 = highest)
   - Start with status: todo

## Output

The complete PRD file content in Markdown format.
"#;

/// Default content for the run task prompt.
pub const PROMPT_RUN_TASK: &str = r#"# Micro Ralph — Run Task Prompt

## Objective

Execute the next incomplete task from a PRD.

## Context

Look at the PRD file at `{{prd_path}}` to understand:
- The project goals and constraints
- The task list and their statuses
- The History section showing previous attempts and outcomes

The suggested next task is `{{next_task_id}}` based on priority, but verify against the PRD.

## Required Actions

1. Read and understand the PRD fully.
2. Identify the next task to work on.
3. Implement the task as described.
4. Make minimal, focused changes.
5. Follow existing code patterns and conventions.
6. Run `cargo make uat` to verify changes.

## Constraints

- Do not modify unrelated code.
- Do not change the public API unless the task requires it.
- Prefer fixing root causes over surface workarounds.

## On Success

If `cargo make uat` passes:
- Report: "Task completed successfully."
- Summarize what was changed.

## On Failure

If `cargo make uat` fails:
- Report: "Task incomplete. UAT failed."
- Describe what was attempted.
- Suggest next steps.

## Output

Provide a summary suitable for appending to the PRD History section.
"#;

/// Default content for the run task finalize prompt.
pub const PROMPT_RUN_TASK_FINALIZE: &str = r#"# Micro Ralph — Run Task Finalize Prompt

## Objective

Complete the final wrap-up task for a PRD.

## Context

You are executing the wrap-up task for `{{prd_id}}`.

All other tasks in this PRD have been completed.

## PRD Summary

{{prd_summary}}

## Required Actions

1. Review all changes made during this PRD.
2. Ensure documentation is up-to-date:
   - README.md
   - AGENTS.md
   - Inline comments
3. Verify all acceptance tests pass: `cargo make uat`
4. Clean up any temporary or debug code.
5. Ensure the codebase is in a releasable state.

## Constraints

- Do not introduce new features.
- Focus on polish and documentation.
- Ensure consistency across the codebase.

## Output

Provide a final summary suitable for closing out the PRD History section.
"#;

/// Default content for the update agents prompt.
pub const PROMPT_UPDATE_AGENTS: &str = r#"# Micro Ralph — Update Agents Prompt

## Objective

Update the AGENTS.md file with relevant conventions and patterns.

## Context

A PRD has been created or a task has been completed.

## Current AGENTS.md Content

{{agents_content}}

## Recent Changes

{{#each recent_changes}}
- {{file}}: {{description}}
{{/each}}

## Required Actions

1. Review the recent changes.
2. Identify any new conventions, patterns, or important notes.
3. Update the auto-managed section of AGENTS.md if needed.

## Auto-Managed Section

Only modify content between these markers:
```
<!-- BEGIN MICRO RALPH AUTO-MANAGED SECTION -->
...
<!-- END MICRO RALPH AUTO-MANAGED SECTION -->
```

## Constraints

- Do not modify content outside the auto-managed section.
- Keep additions concise and actionable.
- Only add information that helps future coding agents.

## Output

The updated content for the auto-managed section, or "NO_CHANGES" if no updates are needed.
"#;

/// Default content for the empty PRDS.md index.
pub const EMPTY_INDEX: &str = r#"# Micro Ralph — PRD Index

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
- `.mr/`: Micro Ralph state directory
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

<!-- BEGIN MICRO RALPH AUTO-MANAGED SECTION -->
<!-- This section is auto-updated by `mr prd new` and `mr run`. -->
<!-- END MICRO RALPH AUTO-MANAGED SECTION -->
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

/// Initializes the Micro Ralph directory structure in the given root.
///
/// Creates:
/// - `.mr/prds/` directory
/// - `.mr/templates/` directory with `prd.md`
/// - `.mr/prompts/` directory with all prompt files
/// - `.mr/PRDS.md` empty index
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
        &prompts_dir.join("update_agents.md"),
        PROMPT_UPDATE_AGENTS,
        &mut result,
    )?;

    // Create empty PRDS.md index.
    create_file_if_missing(&mr_dir.join("PRDS.md"), EMPTY_INDEX, &mut result)?;

    // Create AGENTS.md at repo root (if not exists).
    create_file_if_missing(&root.join("AGENTS.md"), STARTER_AGENTS, &mut result)?;

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

/// Checks if a directory has been initialized with Micro Ralph.
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
        assert!(root.join(".mr/prompts/update_agents.md").exists());

        // Check index exists.
        assert!(root.join(".mr/PRDS.md").exists());

        // Check AGENTS.md exists.
        assert!(root.join("AGENTS.md").exists());

        // Check result counts.
        assert_eq!(result.dirs_created, 3);
        assert_eq!(result.files_created, 12); // 1 template + 9 prompts + 1 index + 1 AGENTS.md
        assert_eq!(result.files_skipped, 0);
    }

    #[test]
    fn test_init_is_idempotent() {
        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // First init.
        let result1 = init(root).unwrap();
        assert_eq!(result1.files_created, 12);
        assert_eq!(result1.files_skipped, 0);

        // Second init should skip all files.
        let result2 = init(root).unwrap();
        assert_eq!(result2.files_created, 0);
        assert_eq!(result2.files_skipped, 12);
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
    fn test_prompts_contain_placeholders() {
        // Round 1 questions should have slug placeholder.
        assert!(PROMPT_PRD_NEW_ROUND1.contains("{{slug}}"));

        // Run task should have prd_path placeholder.
        assert!(PROMPT_RUN_TASK.contains("{{prd_path}}"));

        // Update agents should have content placeholder.
        assert!(PROMPT_UPDATE_AGENTS.contains("{{agents_content}}"));
    }

    #[test]
    fn test_starter_agents_has_auto_managed_section() {
        assert!(STARTER_AGENTS.contains("<!-- BEGIN MICRO RALPH AUTO-MANAGED SECTION -->"));
        assert!(STARTER_AGENTS.contains("<!-- END MICRO RALPH AUTO-MANAGED SECTION -->"));
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
}
