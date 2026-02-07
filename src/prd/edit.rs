//! PRD editing via interactive runner session.
//!
//! This module implements `mr edit` which drops the user into an interactive
//! chat session with the underlying agent. The agent reads the existing PRD,
//! discusses changes with the user, and writes the updated PRD directly to disk.
//!
//! On Ctrl+C or error during the interactive phase, the process aborts
//! without corrupting the PRD file.

use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use super::{Prd, PrdSummary, generate_index_from_root, parse_prd, scan_prd_summaries, scan_prds};
use crate::config::load_constitution;
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;

/// Result of the PRD edit process.
#[derive(Debug)]
pub struct PrdEditResult {
    /// The updated PRD.
    pub prd: Prd,

    /// The path where the PRD was written.
    pub path: PathBuf,
}

/// Configuration for the PRD edit command.
#[derive(Debug)]
pub struct PrdEditConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// The PRD ID to edit (e.g., "PRD-0001").
    pub prd_id: &'a str,

    /// Optional upfront context from the user (via --context flag).
    pub context: Option<&'a str>,
}

/// Runs the PRD edit flow using an interactive session.
///
/// This function:
/// 1. Loads the existing PRD
/// 2. Launches an interactive session with the runner
/// 3. The agent chats with the user and writes the updated PRD to disk
/// 4. Validates the modified PRD and regenerates the index
pub fn edit_prd<R, O>(config: &PrdEditConfig, runner: &R, output: &mut O) -> Result<PrdEditResult>
where
    R: Runner + ?Sized,
    O: Write,
{
    writeln!(output, "Editing PRD: {}", config.prd_id)?;
    writeln!(output)?;

    // Find and load the PRD.
    let (prd_path, prd_content) = find_prd(config.root, config.prd_id)?;

    tracing::debug!(prd_id = %config.prd_id, prd_path = %prd_path.display(), runner = %runner.name(), "Starting PRD edit");

    // Scan existing PRDs for context.
    let existing_prds = scan_prd_summaries(config.root)?;

    // Build the interactive prompt with all context.
    let interactive_prompt = build_edit_prompt(config, &prd_content, &prd_path, &existing_prds);

    writeln!(
        output,
        "Launching interactive session with {}...",
        runner.name()
    )?;
    writeln!(
        output,
        "💡 Discuss your PRD changes with the agent. It will write the file when ready."
    )?;
    writeln!(output)?;

    tracing::info!(
        runner = %runner.name(),
        prd_id = %config.prd_id,
        prd_path = %prd_path.display(),
        "Launching interactive PRD edit session"
    );

    // Launch interactive session — agent discusses changes and writes the PRD file.
    match runner.execute_interactive(&interactive_prompt, config.root) {
        Ok(()) => {}
        Err(e) => {
            if e.is_interrupted() {
                writeln!(output)?;
                writeln!(
                    output,
                    "⚠️  Interactive session interrupted. PRD edit aborted — no changes were made."
                )?;
                tracing::info!("Interactive session interrupted by signal, aborting PRD edit");
                bail!(
                    "Interactive session interrupted (Ctrl+C or signal): {e}\n  Suggestion: Re-run `mr edit` to start a fresh PRD edit session."
                );
            }

            tracing::error!(error = %e, "Interactive session failed");
            bail!(
                "Interactive session failed: {e}\n  Suggestion: Re-run `mr edit` to retry. If the problem persists, check that your runner (e.g., `copilot` or `claude` CLI) is installed and working."
            );
        }
    }

    writeln!(output)?;
    writeln!(output, "Interactive session complete.")?;

    // Validation: re-read the PRD file to validate the agent's changes.
    let updated_content = std::fs::read_to_string(&prd_path)
        .with_context(|| format!("Failed to read PRD file after edit: {}", prd_path.display()))?;

    let prd = parse_prd(&updated_content).context("Failed to parse updated PRD")?;

    tracing::debug!(prd_path = %prd_path.display(), "Validating PRD frontmatter after agent edit");
    crate::commands::validate::validate_prd_frontmatter(&prd_path);

    writeln!(output)?;
    writeln!(output, "Updated PRD: {}", prd_path.display())?;

    // Regenerate the index.
    generate_index_from_root(config.root)?;
    writeln!(output, "Updated PRD index")?;

    Ok(PrdEditResult {
        prd,
        path: prd_path,
    })
}

/// Finds a PRD by ID and returns its path and content.
fn find_prd(root: &Path, prd_id: &str) -> Result<(PathBuf, String)> {
    let prds_dir = root.join(".mr").join("prds");
    let prds = scan_prds(&prds_dir)?;

    for (_filename, prd, path) in prds {
        if prd.id().eq_ignore_ascii_case(prd_id) {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read PRD file: {}", path.display()))?;
            return Ok((path, content));
        }
    }

    bail!("PRD not found: {prd_id}.\n  Suggestion: Run `mr status` to list available PRDs.")
}

/// Builds the interactive edit prompt with context.
fn build_edit_prompt(
    config: &PrdEditConfig,
    prd_content: &str,
    prd_path: &Path,
    existing_prds: &[PrdSummary],
) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdEditInteractive);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("prd_path", prd_path.display().to_string());
    ctx.insert("prd_content", prd_content);
    ctx.insert("context", config.context.unwrap_or(""));

    // Load constitution if available.
    if let Ok(Some(constitution)) = load_constitution(config.root) {
        ctx.insert("constitution", constitution);
    }

    // Build existing PRDs list.
    let prd_list: Vec<HashMap<String, String>> = existing_prds
        .iter()
        .map(|p| {
            [
                ("id".to_string(), p.id.clone()),
                ("title".to_string(), p.title.clone()),
                ("status".to_string(), p.status.to_string()),
            ]
            .into_iter()
            .collect()
        })
        .collect();

    ctx.insert("existing_prds", PlaceholderValue::List(prd_list));

    expand_placeholders(&template, &ctx)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runner::MockRunner;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let temp = TempDir::new().unwrap();
        let prds_dir = temp.path().join(".mr").join("prds");
        std::fs::create_dir_all(&prds_dir).unwrap();
        temp
    }

    fn create_test_prd(temp: &TempDir, id: &str, title: &str) -> PathBuf {
        let prds_dir = temp.path().join(".mr").join("prds");
        let content = format!(
            r#"---
id: {id}
title: "{title}"
status: active

tasks:
  - id: T-001
    title: Initial task
    priority: 1
    status: todo

---

# Summary

A test PRD.

# History

(Entries appended by `mr run` will go below this line.)
"#
        );
        let path = prds_dir.join(format!("{}-test.md", id.to_lowercase()));
        std::fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn test_find_prd() {
        let temp = setup_test_repo();
        create_test_prd(&temp, "PRD-0001", "Test PRD");

        let (path, content) = find_prd(temp.path(), "PRD-0001").unwrap();
        assert!(path.exists());
        assert!(content.contains("PRD-0001"));
    }

    #[test]
    fn test_find_prd_not_found() {
        let temp = setup_test_repo();
        let result = find_prd(temp.path(), "PRD-9999");
        assert!(result.is_err());
    }

    #[test]
    fn test_find_prd_not_found_includes_suggestion() {
        let temp = setup_test_repo();
        let err = find_prd(temp.path(), "PRD-9999").unwrap_err();
        let msg = err.to_string();

        assert!(
            msg.contains("Suggestion:"),
            "Error should contain 'Suggestion:', got: {msg}"
        );
        assert!(
            msg.contains("mr status"),
            "Error should suggest `mr status`, got: {msg}"
        );
    }

    #[test]
    fn test_edit_prd_interactive_flow() {
        let temp = setup_test_repo();
        let prd_path = create_test_prd(&temp, "PRD-0001", "Original Title");

        // Simulate the agent writing the updated PRD during the interactive session.
        let updated_content = r#"---
id: PRD-0001
title: "Updated Title"
status: active

tasks:
  - id: T-001
    title: Initial task
    priority: 1
    status: todo
  - id: T-002
    title: New task from edit
    priority: 2
    status: todo

---

# Summary

An updated test PRD.

# History

(Entries appended by `mr run` will go below this line.)
"#;

        // The mock runner's interactive handler writes the updated PRD to disk,
        // simulating what the real agent does during the interactive session.
        std::fs::write(&prd_path, updated_content).unwrap();

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: Some("Add a new task T-002 for testing"),
        };

        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");
        assert_eq!(result.prd.title(), "Updated Title");

        // Verify interactive session was called.
        assert_eq!(runner.recorded_interactive_prompts().len(), 1);

        // Verify no non-interactive execute was called.
        assert_eq!(runner.recorded_prompts().len(), 0);
    }

    #[test]
    fn test_edit_prd_aborts_on_interrupted_signal() {
        let temp = setup_test_repo();
        let prd_path = create_test_prd(&temp, "PRD-0001", "Original Title");

        // Save original content to verify it's unchanged after abort.
        let original_content = std::fs::read_to_string(&prd_path).unwrap();

        let runner = MockRunner::empty();
        runner.set_interactive_error(crate::runner::RunnerError::Interrupted(
            "Interactive session terminated by signal 2 (SIGINT/Ctrl+C)".to_string(),
        ));

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: Some("Add a new task"),
        };

        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut output);

        assert!(result.is_err(), "PRD edit should fail on interrupt");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("interrupted"),
            "Error should mention interruption, got: {err_msg}"
        );
        assert!(
            err_msg.contains("Suggestion:"),
            "Error should contain 'Suggestion:', got: {err_msg}"
        );
        assert!(
            err_msg.contains("mr edit"),
            "Error should suggest `mr edit`, got: {err_msg}"
        );

        // Verify user-facing output mentions abort.
        let output_str = String::from_utf8(output).unwrap();
        assert!(
            output_str.contains("aborted"),
            "Output should mention abort, got: {output_str}"
        );

        // Verify PRD file is unchanged.
        let content_after = std::fs::read_to_string(&prd_path).unwrap();
        assert_eq!(
            original_content, content_after,
            "PRD should not be modified on interrupt"
        );
    }

    #[test]
    fn test_edit_prd_aborts_on_process_failure() {
        let temp = setup_test_repo();
        create_test_prd(&temp, "PRD-0001", "Original Title");

        let runner = MockRunner::empty();
        runner.set_interactive_error(crate::runner::RunnerError::ProcessFailed(
            "Interactive session exited with status: exit status: 1".to_string(),
        ));

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: Some("Add a new task"),
        };

        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut output);

        assert!(result.is_err(), "PRD edit should fail on process error");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("failed"),
            "Error should mention failure, got: {err_msg}"
        );
        assert!(
            err_msg.contains("Suggestion:"),
            "Error should contain 'Suggestion:', got: {err_msg}"
        );
    }

    /// Verify old Q/A loop code is fully removed from `prd::edit`.
    #[test]
    fn test_old_qa_loop_code_removed() {
        let source = include_str!("edit.rs");

        // Split source at `#[cfg(test)]` to only inspect the non-test code.
        let production_code = source
            .split("#[cfg(test)]")
            .next()
            .expect("source should contain #[cfg(test)]");

        for pattern in [
            "parse_questions",
            "collect_singleline_answers",
            "QaPair",
            "MAX_QA_ROUNDS",
            "qa_history",
            "READY_TO_APPLY",
            "READY_SIGNAL",
            "extract_prd_content",
        ] {
            assert!(
                !production_code.contains(pattern),
                "Old Q/A pattern `{pattern}` should not appear in production code of prd::edit"
            );
        }
    }

    #[test]
    fn test_edit_prd_context_in_interactive_prompt() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_edit_interactive.md"),
            "Edit PRD at {{prd_path}}{{#if context}} with context: {{context}}{{/if}}",
        )
        .unwrap();

        let prd_path = create_test_prd(&temp, "PRD-0001", "Test PRD");

        // Simulate agent updating the PRD during interactive session.
        let updated = "---\nid: PRD-0001\ntitle: \"Test PRD\"\nstatus: active\ntasks:\n  - id: T-001\n    title: Initial task\n    priority: 1\n    status: todo\n---\n# Summary\nUpdated.\n";
        std::fs::write(&prd_path, updated).unwrap();

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: Some("add a logging task"),
        };

        let mut output = Vec::new();

        edit_prd(&config, &runner, &mut output).unwrap();

        // Verify the interactive prompt includes user request as context.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("with context:"),
            "Interactive prompt should contain context marker"
        );
        assert!(
            interactive_prompts[0].contains("add a logging task"),
            "Interactive prompt should contain actual user request"
        );
    }

    #[test]
    fn test_edit_prd_no_context() {
        let temp = setup_test_repo();
        let prd_path = create_test_prd(&temp, "PRD-0001", "Original Title");

        // PRD remains unchanged (agent writes same content back).
        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: None,
        };

        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");
        assert_eq!(result.path, prd_path);

        // Verify interactive session was still called.
        assert_eq!(runner.recorded_interactive_prompts().len(), 1);
    }

    #[test]
    fn test_edit_prd_constitution_in_prompt() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create constitution file.
        let constitution_content = "# Constitution\n\n## Rules\n1. **Use semantic versioning** — All releases follow semver.\n";
        std::fs::write(
            temp.path().join(".mr").join("constitution.md"),
            constitution_content,
        )
        .unwrap();

        // Create prompt that includes constitution placeholder.
        std::fs::write(
            prompts_dir.join("prd_edit_interactive.md"),
            "Edit PRD{{#if constitution}}\n\nConstitution:\n{{constitution}}{{/if}}",
        )
        .unwrap();

        let prd_path = create_test_prd(&temp, "PRD-0001", "Test PRD");

        // Simulate agent writing unchanged PRD back.
        let content = std::fs::read_to_string(&prd_path).unwrap();
        std::fs::write(&prd_path, &content).unwrap();

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: None,
        };

        let mut output = Vec::new();

        edit_prd(&config, &runner, &mut output).unwrap();

        // Verify constitution appears in the interactive prompt.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("Use semantic versioning"),
            "Interactive prompt should contain constitution content"
        );
    }

    #[test]
    fn test_edit_prd_existing_prds_in_prompt() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Use template pattern for existing_prds.
        std::fs::write(
            prompts_dir.join("prd_edit_interactive.md"),
            "Edit PRD\n\n{{#each existing_prds}}\n- {{id}}: {{title}} ({{status}})\n{{/each}}",
        )
        .unwrap();

        // Create multiple PRDs in the prds directory.
        let prds_dir = temp.path().join(".mr").join("prds");

        std::fs::write(
            prds_dir.join("PRD-0001-auth.md"),
            "---\nid: PRD-0001\ntitle: Authentication System\nstatus: done\ntasks: []\n---\n# Summary\n",
        )
        .unwrap();

        let prd_path = prds_dir.join("PRD-0002-api.md");
        std::fs::write(
            &prd_path,
            "---\nid: PRD-0002\ntitle: REST API Layer\nstatus: active\ntasks:\n  - id: T-001\n    title: task\n    priority: 1\n    status: todo\n---\n# Summary\n",
        )
        .unwrap();

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0002",
            context: None,
        };

        let mut output = Vec::new();

        edit_prd(&config, &runner, &mut output).unwrap();

        // Verify existing PRDs appear in the interactive prompt.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("PRD-0001"),
            "Interactive prompt should contain existing PRD-0001"
        );
        assert!(
            interactive_prompts[0].contains("Authentication System"),
            "Interactive prompt should contain PRD-0001 title"
        );
    }

    #[test]
    fn test_edit_prd_fails_on_missing_file_after_session() {
        let temp = setup_test_repo();
        let prd_path = create_test_prd(&temp, "PRD-0001", "Original Title");

        // Simulate the agent deleting the PRD file during the interactive session.
        std::fs::remove_file(&prd_path).unwrap();

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: Some("delete everything"),
        };

        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut output);

        // The function reads the PRD before launching interactive, then re-reads after.
        // Since the file was deleted, the post-session read should fail.
        assert!(
            result.is_err(),
            "Edit should fail when PRD file is missing after session"
        );
    }

    #[test]
    fn test_edit_prd_fails_on_corrupted_file_after_session() {
        let temp = setup_test_repo();
        let prd_path = create_test_prd(&temp, "PRD-0001", "Original Title");

        // Simulate the agent writing corrupted content to the PRD file.
        std::fs::write(&prd_path, "This is not valid YAML frontmatter at all").unwrap();

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: Some("corrupt the file"),
        };

        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut output);

        // The parse_prd call should fail on corrupted content.
        assert!(
            result.is_err(),
            "Edit should fail when PRD file is corrupted after session"
        );
    }

    #[test]
    fn test_edit_prd_prd_path_in_prompt() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_edit_interactive.md"),
            "Edit PRD at {{prd_path}}",
        )
        .unwrap();

        let prd_path = create_test_prd(&temp, "PRD-0001", "Test PRD");

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: None,
        };

        let mut output = Vec::new();

        edit_prd(&config, &runner, &mut output).unwrap();

        // Verify the prd_path placeholder is expanded in the prompt.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains(&prd_path.display().to_string()),
            "Interactive prompt should contain the PRD file path"
        );
    }

    #[test]
    fn test_edit_prd_prd_content_in_prompt() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_edit_interactive.md"),
            "Existing content:\n{{prd_content}}",
        )
        .unwrap();

        create_test_prd(&temp, "PRD-0001", "My Unique Title");

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: None,
        };

        let mut output = Vec::new();

        edit_prd(&config, &runner, &mut output).unwrap();

        // Verify the existing PRD content is injected into the prompt.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("My Unique Title"),
            "Interactive prompt should contain the existing PRD content"
        );
        assert!(
            interactive_prompts[0].contains("A test PRD"),
            "Interactive prompt should contain the PRD body"
        );
    }

    #[test]
    fn test_edit_prd_case_insensitive_id_lookup() {
        let temp = setup_test_repo();
        create_test_prd(&temp, "PRD-0001", "Test PRD");

        let runner = MockRunner::empty();

        // Use lowercase prd id.
        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "prd-0001",
            context: None,
        };

        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");
    }

    #[test]
    fn test_edit_prd_validates_and_regenerates_index() {
        let temp = setup_test_repo();
        let prd_path = create_test_prd(&temp, "PRD-0001", "Original Title");

        // Simulate the agent writing the updated PRD during the interactive session.
        let updated_content = r#"---
id: PRD-0001
title: "Validated Updated Title"
status: active

tasks:
  - id: T-001
    title: Initial task
    priority: 1
    status: todo

---

# Summary

An updated test PRD after edit.

# History

(Entries appended by `mr run` will go below this line.)
"#;

        std::fs::write(&prd_path, updated_content).unwrap();

        let runner = MockRunner::empty();

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            context: Some("update the title"),
        };

        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut output).unwrap();

        // Verify validation passed: returned PRD has correct data.
        assert_eq!(result.prd.id(), "PRD-0001");
        assert_eq!(result.prd.title(), "Validated Updated Title");

        // Verify index regeneration: PRDS.md exists and contains the updated title.
        let index_path = temp.path().join(".mr").join("PRDS.md");
        assert!(
            index_path.exists(),
            "PRDS.md should be regenerated after edit"
        );

        let index_content = std::fs::read_to_string(&index_path).unwrap();
        assert!(
            index_content.contains("Validated Updated Title"),
            "PRDS.md should contain the updated PRD title, got: {index_content}"
        );

        // Verify output mentions index update.
        let output_str = String::from_utf8(output).unwrap();
        assert!(
            output_str.contains("Updated PRD index"),
            "Output should confirm index regeneration, got: {output_str}"
        );
    }
}
