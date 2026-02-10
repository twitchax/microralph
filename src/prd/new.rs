//! PRD creation via single-phase interactive flow.
//!
//! This module implements `mr new` which uses a single-phase architecture:
//!
//! 1. **Interactive Session**: The user is dropped into a direct interactive
//!    chat session with the underlying agent (Copilot/Claude). The agent asks
//!    questions until it has enough context, then **writes the PRD file directly
//!    to disk** and tells the user to exit.
//! 2. **Validation**: On clean exit, the Rust side picks up the file from
//!    `.mr/prds/`, validates it, and regenerates the index.
//!
//! On Ctrl+C or error during the interactive phase, the process aborts
//! entirely without creating a PRD.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::{Prd, PrdSummary, generate_index_from_root, parse_prd, scan_prd_summaries};
use crate::config::load_constitution;
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;

/// Result of the PRD creation process.
#[derive(Debug)]
pub struct PrdNewResult {
    /// The created PRD.
    pub prd: Prd,

    /// The path where the PRD was written.
    pub path: std::path::PathBuf,
}

/// Configuration for the PRD new command.
#[derive(Debug)]
pub struct PrdNewConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// The slug for the new PRD.
    pub slug: &'a str,

    /// Optional initial description from the user.
    pub description: Option<&'a str>,

    /// Optional upfront context from the user (via --context flag).
    pub context: Option<&'a str>,
}

/// Runs the PRD creation flow using a single-phase interactive architecture.
///
/// **Interactive Session**:
/// Drops the user into a direct interactive chat session with the underlying
/// agent. The prompt includes existing PRDs, constitution, the next PRD ID,
/// the target file path, and any user-provided context. The agent gathers
/// information, writes the PRD file directly to disk, and tells the user
/// to exit.
///
/// **Validation**:
/// On clean exit, the Rust side scans `.mr/prds/` for the newly created
/// PRD file, validates it, and regenerates the index.
///
/// **Abort on error**: If the interactive session is interrupted (Ctrl+C)
/// or fails, the process aborts entirely without creating a PRD.
pub fn create_prd<R, O>(config: &PrdNewConfig, runner: &R, output: &mut O) -> Result<PrdNewResult>
where
    R: Runner + ?Sized,
    O: Write,
{
    writeln!(output, "Creating new PRD: {}", config.slug)?;
    writeln!(output)?;

    // Scan existing PRDs for context.
    let existing_prds = scan_prd_summaries(config.root)?;

    // Generate next PRD ID and target path.
    let next_id = generate_next_prd_id(&existing_prds);
    let prd_filename = format!("{}-{}.md", next_id, config.slug);
    let prd_path = config.root.join(".mr").join("prds").join(&prd_filename);

    tracing::debug!(next_id = %next_id, runner = %runner.name(), "Starting PRD creation");

    // Build the interactive prompt with all context.
    let interactive_prompt = build_interactive_prompt(config, &existing_prds, &next_id, &prd_path);

    writeln!(
        output,
        "Launching interactive session with {}...",
        runner.name()
    )?;
    writeln!(
        output,
        "💡 Discuss your PRD with the agent. It will write the file when ready."
    )?;
    writeln!(output)?;

    tracing::info!(
        runner = %runner.name(),
        slug = %config.slug,
        next_id = %next_id,
        prd_path = %prd_path.display(),
        "Launching interactive PRD creation session"
    );

    // Launch interactive session — agent gathers info and writes the PRD file.
    match runner.execute_interactive(&interactive_prompt, config.root) {
        Ok(()) => {}
        Err(e) => {
            if e.is_interrupted() {
                writeln!(output)?;
                writeln!(
                    output,
                    "⚠️  Interactive session interrupted. PRD creation aborted — no PRD was created."
                )?;
                tracing::info!("Interactive session interrupted by signal, aborting PRD creation");
                bail!(
                    "Interactive session interrupted (Ctrl+C or signal): {e}\n  Suggestion: Re-run `mr new` to start a fresh PRD creation session."
                );
            }

            tracing::error!(error = %e, "Interactive session failed");
            bail!(
                "Interactive session failed: {e}\n  Suggestion: Re-run `mr new` to retry. If the problem persists, check that your runner (e.g., `copilot` or `claude` CLI) is installed and working."
            );
        }
    }

    writeln!(output)?;
    writeln!(output, "Interactive session complete.")?;

    // Validation: scan for the PRD file the agent should have written.
    let prds_dir = config.root.join(".mr").join("prds");

    let (prd, final_path) = if let Some((path, content)) =
        find_created_prd_file(&prds_dir, config.slug, &next_id)?
    {
        tracing::debug!(path = %path.display(), "Found PRD file created by agent");

        let prd = parse_prd_or_fallback(&content, &next_id, config.slug, Some(&path), output)?;

        // Validate required fields.
        if prd.id().is_empty() || prd.title().is_empty() {
            tracing::warn!(
                content_preview = ?content.chars().take(200).collect::<String>(),
                "PRD file is missing required fields (id or title)"
            );
            writeln!(
                output,
                "⚠️  Warning: PRD is missing required fields (id or title)"
            )?;
        }

        (prd, path)
    } else {
        // Agent didn't write the file — create a placeholder.
        tracing::warn!("No PRD file found after interactive session");
        writeln!(
            output,
            "⚠️  Warning: Agent did not create a PRD file. Creating a placeholder."
        )?;

        let frontmatter = crate::prd::types::PrdFrontmatter {
            id: next_id.clone(),
            title: format!("PRD for {}", config.slug),
            status: crate::prd::PrdStatus::Draft,
            ..Default::default()
        };

        let placeholder_content = format!(
            "---\nid: {}\ntitle: \"PRD for {}\"\nstatus: draft\ntasks: []\n---\n\n# Summary\n\nPlaceholder PRD — the interactive session did not produce a file.\nPlease edit this PRD manually or re-run `mr new`.\n",
            next_id, config.slug
        );

        std::fs::write(&prd_path, &placeholder_content)
            .context("Failed to write placeholder PRD file")?;

        (
            crate::prd::Prd::new(frontmatter, String::new()),
            prd_path.clone(),
        )
    };

    writeln!(output)?;
    writeln!(output, "Created PRD: {}", final_path.display())?;

    // Finalize: Update index to reflect new PRD.
    generate_index_from_root(config.root)?;
    writeln!(output, "Updated PRD index")?;

    Ok(PrdNewResult {
        prd,
        path: final_path,
    })
}

/// Parses PRD content or returns a fallback PRD if parsing fails.
fn parse_prd_or_fallback<O: Write>(
    content: &str,
    next_id: &str,
    slug: &str,
    path: Option<&std::path::Path>,
    output: &mut O,
) -> Result<Prd> {
    match parse_prd(content) {
        Ok(p) => Ok(p),
        Err(e) => {
            if let Some(p) = path {
                tracing::warn!(
                    path = %p.display(),
                    error = %e,
                    "Failed to parse PRD file, using expected values"
                );
            } else {
                tracing::warn!(
                    error = %e,
                    content_preview = ?content.chars().take(200).collect::<String>(),
                    "Failed to parse PRD content, using expected values"
                );
            }
            writeln!(
                output,
                "⚠️  Warning: Failed to parse PRD, leaving file as-is"
            )?;

            let frontmatter = crate::prd::types::PrdFrontmatter {
                id: next_id.to_string(),
                title: format!("PRD for {slug}"),
                status: crate::prd::PrdStatus::Draft,
                ..Default::default()
            };
            Ok(crate::prd::Prd::new(frontmatter, String::new()))
        }
    }
}

/// Searches for a PRD file that was created by the runner.
///
/// The runner may create the file directly, so we need to check
/// for files matching our expected patterns.
fn find_created_prd_file(
    prds_dir: &Path,
    slug: &str,
    expected_id: &str,
) -> Result<Option<(std::path::PathBuf, String)>> {
    if !prds_dir.exists() {
        return Ok(None);
    }

    // Look for files matching the pattern: PRD-XXXX-<slug>.md
    // or just containing the slug.
    let entries = std::fs::read_dir(prds_dir).context("Failed to read prds directory")?;

    for entry in entries.flatten() {
        let path = entry.path();

        if path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let Some(filename) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };

        // Check if this file matches our slug.
        // Could be PRD-XXXX-slug.md or variations.
        let slug_normalized = slug.to_lowercase().replace('_', "-");

        if filename.to_lowercase().contains(&slug_normalized) {
            // Found a potential match - read and validate it.
            let content = std::fs::read_to_string(&path).with_context(|| {
                format!("Failed to read potential PRD file: {}", path.display())
            })?;

            // Quick validation: must have frontmatter.
            if content.trim().starts_with("---") {
                tracing::debug!(
                    path = %path.display(),
                    expected_id = %expected_id,
                    "Found matching PRD file"
                );

                return Ok(Some((path, content)));
            }
        }
    }

    Ok(None)
}

/// Generates the next PRD ID based on existing PRDs.
fn generate_next_prd_id(existing: &[PrdSummary]) -> String {
    let max_num = existing
        .iter()
        .filter_map(|p| {
            p.id.strip_prefix("PRD-")
                .and_then(|s| s.parse::<u32>().ok())
        })
        .max()
        .unwrap_or(0);

    format!("PRD-{:04}", max_num + 1)
}

/// Builds the interactive prompt for the single-phase PRD creation session.
///
/// Includes existing PRDs, constitution, user-provided context, the next PRD ID,
/// and the target file path so the agent can write the PRD directly to disk.
fn build_interactive_prompt(
    config: &PrdNewConfig,
    existing_prds: &[PrdSummary],
    next_id: &str,
    prd_path: &Path,
) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdNewInteractive);

    let mut ctx = PlaceholderContext::new();
    ctx.insert("slug", config.slug);
    ctx.insert("next_id", next_id);

    let prd_path_str = prd_path.display().to_string();
    ctx.insert("prd_path", prd_path_str.as_str());

    if let Some(desc) = config.description {
        ctx.insert("user_description", desc);
    }

    if let Some(context) = config.context {
        ctx.insert("user_context", context);
    }

    // Load constitution if available.
    if let Ok(Some(constitution)) = load_constitution(config.root) {
        ctx.insert("constitution", constitution);
    }

    // Build existing PRDs list.
    ctx.insert(
        "existing_prds",
        PlaceholderValue::List(PrdSummary::to_placeholder_list(existing_prds)),
    );

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

    fn setup_test_repo_with_prompts() -> TempDir {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create the interactive prompt file.
        std::fs::write(
            prompts_dir.join("prd_new_interactive.md"),
            "Interactive PRD creation for {{slug}} with next ID {{next_id}} at {{prd_path}}",
        )
        .unwrap();

        temp
    }

    #[test]
    fn test_generate_next_prd_id_empty() {
        let existing: Vec<PrdSummary> = vec![];
        assert_eq!(generate_next_prd_id(&existing), "PRD-0001");
    }

    #[test]
    fn test_generate_next_prd_id_with_existing() {
        let existing = vec![
            PrdSummary {
                id: "PRD-0001".to_string(),
                title: "First".to_string(),
                status: crate::prd::PrdStatus::Active,
                relative_path: "prds/test.md".to_string(),
                completed_tasks: 0,
                total_tasks: 0,
                verified_uats: 0,
                total_uats: 0,
                references: vec![],
                depends_on: vec![],
            },
            PrdSummary {
                id: "PRD-0003".to_string(),
                title: "Third".to_string(),
                status: crate::prd::PrdStatus::Done,
                relative_path: "prds/test2.md".to_string(),
                completed_tasks: 0,
                total_tasks: 0,
                verified_uats: 0,
                total_uats: 0,
                references: vec![],
                depends_on: vec![],
            },
        ];

        assert_eq!(generate_next_prd_id(&existing), "PRD-0004");
    }

    #[test]
    fn test_create_prd_single_phase_agent_writes_file() {
        // Simulate the agent writing the PRD file during the interactive session.
        let temp = setup_test_repo_with_prompts();
        let prds_dir = temp.path().join(".mr").join("prds");

        let prd_content = r"---
id: PRD-0001
title: Test Feature
status: draft

tasks:
  - id: T-001
    title: Implement feature
    priority: 1
    status: todo

---

# Summary

A test feature.
";

        // The mock runner's interactive handler writes the PRD file to disk,
        // simulating what the real agent does during the interactive session.
        let prd_path = prds_dir.join("PRD-0001-test-feature.md");
        std::fs::write(&prd_path, prd_content).unwrap();

        let runner = MockRunner::empty();

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "test-feature",
            description: None,
            context: None,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");
        assert!(result.path.exists());

        // Verify interactive session was called once.
        assert_eq!(runner.recorded_interactive_prompts().len(), 1);

        // Verify no synthesis (execute) was called — single-phase.
        assert_eq!(runner.recorded_prompts().len(), 0);
    }

    #[test]
    fn test_create_prd_placeholder_when_agent_doesnt_write() {
        // If the agent doesn't write the PRD file, we create a placeholder.
        let temp = setup_test_repo_with_prompts();

        let runner = MockRunner::empty();

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "no-file-test",
            description: None,
            context: None,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");
        assert!(result.path.exists());

        // Verify placeholder content.
        let content = std::fs::read_to_string(&result.path).unwrap();
        assert!(content.contains("Placeholder PRD"));

        // Verify warning was emitted.
        let output_str = String::from_utf8(output).unwrap();
        assert!(
            output_str.contains("Warning"),
            "Output should contain warning about missing file"
        );
    }

    #[test]
    fn test_prd_new_context_in_interactive_prompt() {
        // Verifies that user-provided context is included in the interactive prompt.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_new_interactive.md"),
            "Interactive for {{slug}} ID={{next_id}}{{#if user_context}} considering context: {{user_context}}{{/if}}",
        )
        .unwrap();

        // Pre-create the PRD file as if the agent wrote it.
        let prds_dir = temp.path().join(".mr").join("prds");
        let prd_content =
            "---\nid: PRD-0001\ntitle: Context Test\nstatus: draft\ntasks: []\n---\n# Summary\n";
        std::fs::write(prds_dir.join("PRD-0001-context-test.md"), prd_content).unwrap();

        let runner = MockRunner::empty();

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "context-test",
            description: None,
            context: Some("This is a payment processing feature for e-commerce"),
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");

        // Verify the interactive prompt includes user context.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("considering context:"),
            "Interactive prompt should contain context marker"
        );
        assert!(
            interactive_prompts[0].contains("This is a payment processing feature for e-commerce"),
            "Interactive prompt should contain actual user context"
        );
    }

    #[test]
    fn test_prd_new_next_id_and_path_in_prompt() {
        // Verifies that next_id and prd_path are included in the interactive prompt.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_new_interactive.md"),
            "Create PRD {{next_id}} at {{prd_path}} for {{slug}}",
        )
        .unwrap();

        // No pre-existing PRDs → next ID should be PRD-0001.
        let runner = MockRunner::empty();

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "id-test",
            description: None,
            context: None,
        };

        let mut output = Vec::new();

        // Agent doesn't write a file, so we get a placeholder.
        create_prd(&config, &runner, &mut output).unwrap();

        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("PRD-0001"),
            "Interactive prompt should contain next PRD ID, got: {}",
            &interactive_prompts[0]
        );
        assert!(
            interactive_prompts[0].contains("PRD-0001-id-test.md"),
            "Interactive prompt should contain target file path, got: {}",
            &interactive_prompts[0]
        );
    }

    #[test]
    fn test_constitution_in_interactive_prompt() {
        // Verifies that when a constitution file exists, its content
        // is loaded and included in the interactive prompt.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create constitution file.
        let constitution_content = r"# Constitution

## Purpose
Project governance rules.

## Rules
1. **Acceptance tests must be codified** — No one-off manual tests.
2. **Use semantic versioning** — All releases follow semver.
";
        std::fs::write(
            temp.path().join(".mr").join("constitution.md"),
            constitution_content,
        )
        .unwrap();

        // Create prompt file that includes constitution placeholder.
        std::fs::write(
            prompts_dir.join("prd_new_interactive.md"),
            "Interactive for {{slug}}{{#if constitution}}\n\nConstitution:\n{{constitution}}{{/if}}",
        )
        .unwrap();

        // Pre-create the PRD file.
        let prds_dir = temp.path().join(".mr").join("prds");
        let prd_content = "---\nid: PRD-0001\ntitle: Constitution Test\nstatus: draft\ntasks: []\n---\n# Summary\n";
        std::fs::write(prds_dir.join("PRD-0001-constitution-test.md"), prd_content).unwrap();

        let runner = MockRunner::empty();

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "constitution-test",
            description: None,
            context: None,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");

        // Verify constitution in interactive prompt.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("Acceptance tests must be codified"),
            "Interactive prompt should contain constitution content"
        );
    }

    #[test]
    fn test_existing_prds_injected_into_interactive_prompt() {
        // Verifies that existing PRD summaries are injected into the
        // interactive prompt so the agent has project context.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Use the real template pattern for existing_prds.
        std::fs::write(
            prompts_dir.join("prd_new_interactive.md"),
            "Interactive for {{slug}} ID={{next_id}}\n\n{{#each existing_prds}}\n- {{id}}: {{title}} ({{status}})\n{{/each}}",
        )
        .unwrap();

        // Create existing PRD files in the prds directory.
        let prds_dir = temp.path().join(".mr").join("prds");

        std::fs::write(
            prds_dir.join("PRD-0001-auth.md"),
            "---\nid: PRD-0001\ntitle: Authentication System\nstatus: done\ntasks: []\n---\n# Summary\n",
        )
        .unwrap();

        std::fs::write(
            prds_dir.join("PRD-0002-api.md"),
            "---\nid: PRD-0002\ntitle: REST API Layer\nstatus: active\ntasks: []\n---\n# Summary\n",
        )
        .unwrap();

        // Pre-create the new PRD file (agent writes it during interactive session).
        let prd_content =
            "---\nid: PRD-0003\ntitle: New Feature\nstatus: draft\ntasks: []\n---\n# Summary\n";
        std::fs::write(prds_dir.join("PRD-0003-new-feature.md"), prd_content).unwrap();

        let runner = MockRunner::empty();

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "new-feature",
            description: None,
            context: None,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0003");

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
        assert!(
            interactive_prompts[0].contains("PRD-0002"),
            "Interactive prompt should contain existing PRD-0002"
        );
        assert!(
            interactive_prompts[0].contains("REST API Layer"),
            "Interactive prompt should contain PRD-0002 title"
        );
    }

    #[test]
    fn test_create_prd_aborts_on_interrupted_signal() {
        // Verifies that Ctrl+C (signal interruption) during the interactive
        // session aborts PRD creation entirely without creating a file.

        let temp = setup_test_repo_with_prompts();

        let runner = MockRunner::empty();
        runner.set_interactive_error(crate::runner::RunnerError::Interrupted(
            "Interactive session terminated by signal 2 (SIGINT/Ctrl+C)".to_string(),
        ));

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "interrupted-test",
            description: None,
            context: None,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output);

        assert!(result.is_err(), "PRD creation should fail on interrupt");

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
            err_msg.contains("mr new"),
            "Error should suggest `mr new`, got: {err_msg}"
        );

        // Verify user-facing output mentions abort.
        let output_str = String::from_utf8(output).unwrap();
        assert!(
            output_str.contains("aborted"),
            "Output should mention abort, got: {output_str}"
        );

        // Verify no PRD file was created.
        let prds_dir = temp.path().join(".mr").join("prds");
        let prd_files: Vec<_> = std::fs::read_dir(&prds_dir)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .collect();
        assert!(
            prd_files.is_empty(),
            "No PRD files should be created on interrupt"
        );
    }

    #[test]
    fn test_create_prd_aborts_on_process_failure() {
        // Verifies that a non-zero exit code (non-signal failure) also aborts.

        let temp = setup_test_repo_with_prompts();

        let runner = MockRunner::empty();
        runner.set_interactive_error(crate::runner::RunnerError::ProcessFailed(
            "Interactive session exited with status: exit status: 1".to_string(),
        ));

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "failed-test",
            description: None,
            context: None,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output);

        assert!(result.is_err(), "PRD creation should fail on process error");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("failed"),
            "Error should mention failure, got: {err_msg}"
        );
        assert!(
            err_msg.contains("Suggestion:"),
            "Error should contain 'Suggestion:', got: {err_msg}"
        );
        assert!(
            err_msg.contains("mr new"),
            "Error should suggest `mr new`, got: {err_msg}"
        );

        // Verify no PRD file was created.
        let prds_dir = temp.path().join(".mr").join("prds");
        let prd_files: Vec<_> = std::fs::read_dir(&prds_dir)
            .unwrap()
            .filter_map(std::result::Result::ok)
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "md"))
            .collect();
        assert!(
            prd_files.is_empty(),
            "No PRD files should be created on failure"
        );
    }

    /// UAT-007: Verify old multi-round Q/A code is fully removed from `prd::new`.
    ///
    /// The old workflow used iterative Q/A loop functions. These must not appear
    /// in the non-test portion of this module.
    #[test]
    fn test_old_qa_loop_code_removed() {
        let source = include_str!("new.rs");

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
        ] {
            assert!(
                !production_code.contains(pattern),
                "Old Q/A pattern `{pattern}` should not appear in production code of prd::new"
            );
        }
    }

    /// Verify the two-phase synthesis code is fully removed from `prd::new`.
    ///
    /// The old workflow had a separate synthesis phase with `execute_continue`
    /// fallback. These must not appear in the non-test portion of this module.
    #[test]
    fn test_synthesis_phase_code_removed() {
        let source = include_str!("new.rs");

        // Split source at `#[cfg(test)]` to only inspect the non-test code.
        let production_code = source
            .split("#[cfg(test)]")
            .next()
            .expect("source should contain #[cfg(test)]");

        for pattern in [
            "synthesize_and_persist_prd",
            "build_synthesize_prompt",
            "build_discovery_prompt",
            "execute_continue",
            "InteractiveResult",
            "start_spinner",
            "PrdNewSynthesizePrd",
            "PrdNewDiscovery",
        ] {
            assert!(
                !production_code.contains(pattern),
                "Old synthesis pattern `{pattern}` should not appear in production code of prd::new"
            );
        }
    }
}
