//! PRD creation via two-phase interactive flow.
//!
//! This module implements `mr new` which uses a two-phase architecture:
//!
//! 1. **Interactive Discovery**: The user is dropped into a direct interactive
//!    chat session with the underlying agent (Copilot/Claude). The agent asks
//!    questions until it has enough context, then the user exits.
//! 2. **Synthesis**: A non-interactive call synthesizes the PRD from the
//!    conversation transcript/session context.
//!
//! On Ctrl+C or error during the interactive phase, the process aborts
//! entirely without creating a PRD.

use std::collections::HashMap;
use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};

use super::{Prd, PrdSummary, generate_index_from_root, parse_prd, scan_prd_summaries};
use crate::config::load_constitution;
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::{InteractiveResult, Runner};
use crate::util::qa_workflow;
use crate::util::spinner::start_spinner;

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

    /// Stream runner output to stdout in real-time.
    /// When true, spinner is disabled during synthesis phase.
    pub stream: bool,
}

/// Runs the PRD creation flow using a two-phase interactive architecture.
///
/// **Phase 1 — Interactive Discovery**:
/// Drops the user into a direct interactive chat session with the underlying
/// agent. The discovery prompt includes existing PRDs, constitution, and any
/// user-provided context. The agent asks questions until it has enough info,
/// then the user exits.
///
/// **Phase 2 — Synthesis**:
/// On clean exit from the interactive session, a non-interactive call
/// synthesizes the PRD from the conversation transcript/session context.
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

    // Generate next PRD ID.
    let next_id = generate_next_prd_id(&existing_prds);
    tracing::debug!(next_id = %next_id, runner = %runner.name(), "Starting PRD creation");

    // PHASE 1: Interactive Discovery
    // Build discovery prompt with all available context, then launch interactive session.
    let discovery_prompt = build_discovery_prompt(config, &existing_prds);

    writeln!(
        output,
        "Launching interactive session with {}...",
        runner.name()
    )?;
    writeln!(
        output,
        "💡 Discuss your PRD with the agent. When finished, exit the chat session."
    )?;
    writeln!(output)?;

    tracing::info!(
        runner = %runner.name(),
        slug = %config.slug,
        "Launching interactive discovery session"
    );

    let interactive_result = match runner.execute_interactive(&discovery_prompt, config.root) {
        Ok(result) => result,
        Err(e) => {
            if e.is_interrupted() {
                writeln!(output)?;
                writeln!(
                    output,
                    "⚠️  Interactive session interrupted. PRD creation aborted — no PRD was created."
                )?;
                tracing::info!("Interactive session interrupted by signal, aborting PRD creation");
                bail!("Interactive session interrupted (Ctrl+C or signal): {e}");
            }

            tracing::error!(error = %e, "Interactive session failed");
            bail!("Interactive session failed: {e}");
        }
    };

    writeln!(output)?;
    writeln!(output, "Interactive session complete.")?;

    // PHASE 2: Synthesis
    // Use conversation transcript/session context from phase 1 to synthesize the PRD.
    let (prd, prd_path) = synthesize_and_persist_prd(
        config,
        runner,
        &existing_prds,
        &interactive_result,
        &next_id,
        output,
    )?;

    writeln!(output)?;
    writeln!(output, "Created PRD: {}", prd_path.display())?;

    // Finalize: Update index to reflect new PRD.
    generate_index_from_root(config.root)?;
    writeln!(output, "Updated PRD index")?;

    Ok(PrdNewResult {
        prd,
        path: prd_path,
    })
}

/// Synthesizes and persists the PRD to disk using conversation context from phase 1.
///
/// Returns the parsed PRD and its file path.
fn synthesize_and_persist_prd<R, O>(
    config: &PrdNewConfig,
    runner: &R,
    existing_prds: &[PrdSummary],
    interactive_result: &InteractiveResult,
    next_id: &str,
    output: &mut O,
) -> Result<(Prd, std::path::PathBuf)>
where
    R: Runner + ?Sized,
    O: Write,
{
    writeln!(output)?;

    let synthesize_prompt = build_synthesize_prompt(config, existing_prds, interactive_result);

    // Print command info before spinner (only when not streaming).
    if !config.stream
        && let Some(cmd_display) = runner.format_command_display(&synthesize_prompt, config.root)
    {
        println!("\n🔧 Executing: {cmd_display}");
    }

    tracing::info!(
        runner = %runner.name(),
        slug = %config.slug,
        "Invoking runner to synthesize PRD"
    );

    let spinner = start_spinner(!config.stream, "Synthesizing PRD...");

    // Prefer session resume (e.g., Claude's --continue) for full conversational context.
    // Fall back to regular execute with transcript injected into the prompt.
    let synthesize_output =
        if let Some(result) = runner.execute_continue(&synthesize_prompt, config.root) {
            tracing::info!("Using session resume for synthesis context handoff");
            result.map_err(|e| anyhow::anyhow!("Runner failed (session resume): {e}"))?
        } else {
            tracing::info!("Using transcript-based synthesis context handoff");
            runner
                .execute(&synthesize_prompt, config.root)
                .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?
        };

    spinner.finish_and_clear();

    if !synthesize_output.success {
        bail!("Runner failed during synthesis: {}", synthesize_output.text);
    }

    // Persist to disk using one of two strategies.
    let prds_dir = config.root.join(".mr").join("prds");

    let (prd, prd_path, prd_content) = if let Some((path, content)) =
        find_created_prd_file(&prds_dir, config.slug, next_id)?
    {
        // Strategy (1): Runner created the file directly.
        tracing::debug!(path = %path.display(), "Found PRD file created by runner");

        let parsed = parse_prd_or_fallback(&content, next_id, config.slug, Some(&path), output)?;
        (parsed, path, content)
    } else {
        // Strategy (2): Parse runner's response and write ourselves.
        tracing::debug!("No PRD file found, parsing response content");

        let prd_content = match qa_workflow::extract_prd_content(&synthesize_output.text) {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!(error = %e, "Failed to extract PRD content, using raw output");
                synthesize_output.text.clone()
            }
        };

        let prd = parse_prd_or_fallback(&prd_content, next_id, config.slug, None, output)?;

        let filename = format!("{}-{}.md", prd.id(), config.slug);
        let prd_path = prds_dir.join(&filename);
        std::fs::write(&prd_path, &prd_content).context("Failed to write PRD file")?;

        (prd, prd_path, prd_content)
    };

    // Validate required fields.
    if prd.id().is_empty() || prd.title().is_empty() {
        tracing::warn!(
            content_preview = ?prd_content.chars().take(200).collect::<String>(),
            "PRD file is missing required fields (id or title)"
        );
        writeln!(
            output,
            "⚠️  Warning: PRD is missing required fields (id or title)"
        )?;
    }

    Ok((prd, prd_path))
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

/// Builds the discovery prompt for the interactive phase.
///
/// Includes existing PRDs, constitution, and user-provided context so the
/// agent has full project awareness during the interactive conversation.
fn build_discovery_prompt(config: &PrdNewConfig, existing_prds: &[PrdSummary]) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdNewDiscovery);

    let mut ctx = PlaceholderContext::new();
    ctx.insert("slug", config.slug);

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

/// Builds the synthesis prompt with conversation context from the interactive session.
fn build_synthesize_prompt(
    config: &PrdNewConfig,
    existing_prds: &[PrdSummary],
    interactive_result: &InteractiveResult,
) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdNewSynthesizePrd);

    let mut ctx = PlaceholderContext::new();
    ctx.insert("slug", config.slug);

    if let Some(context) = config.context {
        ctx.insert("user_context", context);
    }

    // Load constitution if available.
    if let Ok(Some(constitution)) = load_constitution(config.root) {
        ctx.insert("constitution", constitution);
    }

    // Include conversation transcript from interactive session if available.
    if let Some(transcript) = &interactive_result.transcript {
        ctx.insert("conversation_transcript", transcript.as_str());
    }

    // Include session ID if available for resume-based context handoff.
    if let Some(session_id) = &interactive_result.session_id {
        ctx.insert("session_id", session_id.as_str());
    }

    // Build existing PRDs list.
    let prd_list: Vec<HashMap<String, String>> = existing_prds
        .iter()
        .map(|p| {
            [
                ("id".to_string(), p.id.clone()),
                ("title".to_string(), p.title.clone()),
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
    use crate::util::qa_workflow;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let temp = TempDir::new().unwrap();
        let prds_dir = temp.path().join(".mr").join("prds");
        std::fs::create_dir_all(&prds_dir).unwrap();
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
    fn test_extract_prd_content_code_block() {
        let output = r"Here's the PRD:

```markdown
---
id: PRD-0001
title: Test
---

# Summary

This is a test.
```

Done!
";

        let content = qa_workflow::extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
        assert!(content.contains("id: PRD-0001"));
    }

    #[test]
    fn test_extract_prd_content_md_fence() {
        // LLMs often use ```md instead of ```markdown
        let output = r"Here's the PRD:

```md
---
id: PRD-0001
title: Test
---

# Summary
```
";

        let content = qa_workflow::extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"), "Content was: {content}");
        assert!(content.contains("id: PRD-0001"));
    }

    #[test]
    fn test_extract_prd_content_nested_code_blocks() {
        // PRD content itself may contain code blocks
        let output = r#"```markdown
---
id: PRD-0001
title: Test
---

# Summary

Example code:

```bash
echo "hello"
```

More text.
```
"#;

        let content = qa_workflow::extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"), "Content was: {content}");
        assert!(content.contains("id: PRD-0001"));
        assert!(content.contains("echo \"hello\""));
        assert!(content.contains("More text."));
    }

    #[test]
    fn test_extract_prd_content_plain() {
        let output = r"---
id: PRD-0001
title: Test
---

# Summary
";

        let content = qa_workflow::extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
    }

    #[test]
    fn test_extract_prd_content_with_leading_text() {
        // Fallback: find --- in output even without proper fencing
        let output = r"Sure, here's the PRD you asked for:

---
id: PRD-0001
title: Test
---

# Summary
";

        let content = qa_workflow::extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"), "Content was: {content}");
        assert!(content.contains("id: PRD-0001"));
    }

    #[test]
    fn test_create_prd_two_phase_flow() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create minimal prompt files.
        std::fs::write(
            prompts_dir.join("prd_new_discovery.md"),
            "Discovery for {{slug}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}",
        )
        .unwrap();

        // Create mock runner with scripted synthesis response.
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

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(prd_content)]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "test-feature",
            description: None,
            context: None,
            stream: false,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");
        assert!(result.path.exists());

        // Verify interactive session was called once (discovery phase).
        assert_eq!(runner.recorded_interactive_prompts().len(), 1);

        // Verify synthesis (execute) was called once.
        assert_eq!(runner.recorded_prompts().len(), 1);
    }

    #[test]
    fn test_prd_new_context_in_discovery_prompt() {
        // Verifies that user-provided context is included in the discovery prompt
        // passed to execute_interactive().

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_new_discovery.md"),
            "Discovery for {{slug}}{{#if user_context}} considering context: {{user_context}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}",
        )
        .unwrap();

        let prd_content = r"---
id: PRD-0001
title: Context Test
status: draft
tasks: []
---
# Summary
";

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(prd_content)]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "context-test",
            description: None,
            context: Some("This is a payment processing feature for e-commerce"),
            stream: false,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");

        // Verify the discovery prompt includes user context.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("considering context:"),
            "Discovery prompt should contain context marker"
        );
        assert!(
            interactive_prompts[0].contains("This is a payment processing feature for e-commerce"),
            "Discovery prompt should contain actual user context"
        );
    }

    #[test]
    fn test_prd_new_context_in_synthesis_prompt() {
        // Verifies that user-provided context is included in the synthesis prompt.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_new_discovery.md"),
            "Discovery for {{slug}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}{{#if user_context}} with synthesis context: {{user_context}}{{/if}}",
        )
        .unwrap();

        let prd_content = r"---
id: PRD-0001
title: Synthesis Context Test
status: draft
tasks: []
---
# Summary
";

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(prd_content)]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "synthesis-test",
            description: None,
            context: Some("API Gateway with rate limiting and JWT auth"),
            stream: false,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");

        // Verify context in synthesis prompt.
        let recorded = runner.recorded_prompts();
        let synthesis_prompt = &recorded[0];

        assert!(
            synthesis_prompt.contains("with synthesis context:"),
            "Synthesis prompt should contain context marker"
        );
        assert!(
            synthesis_prompt.contains("API Gateway with rate limiting and JWT auth"),
            "Synthesis prompt should contain user context"
        );
    }

    #[test]
    fn test_prd_new_transcript_in_synthesis_prompt() {
        // Verifies that the conversation transcript from the interactive session
        // is passed through to the synthesis prompt.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_new_discovery.md"),
            "Discovery for {{slug}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}{{#if conversation_transcript}}\n\nTranscript:\n{{conversation_transcript}}{{/if}}",
        )
        .unwrap();

        let prd_content = r"---
id: PRD-0001
title: Transcript Test
status: draft
tasks: []
---
# Summary
";

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(prd_content)]);

        // Set a custom interactive result with a known transcript.
        runner.set_interactive_result(InteractiveResult {
            session_id: None,
            transcript: Some("User: I want feature X\nAgent: Tell me more about X".to_string()),
        });

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "transcript-test",
            description: None,
            context: None,
            stream: false,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");

        // Verify the synthesis prompt includes the transcript.
        let recorded = runner.recorded_prompts();
        assert!(
            recorded[0].contains("I want feature X"),
            "Synthesis prompt should contain transcript from interactive session"
        );
    }

    #[test]
    fn test_prd_new_parse_failure_warning() {
        // Verifies that when the runner returns unparseable content,
        // we emit a warning and create a fallback PRD rather than failing.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(prompts_dir.join("prd_new_discovery.md"), "Discovery").unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD",
        )
        .unwrap();

        // Return invalid PRD content that cannot be parsed.
        let invalid_content = r"This is not valid PRD content.
It has no frontmatter and will fail to parse.
Just some random text.";

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(invalid_content)]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "parse-fail-test",
            description: None,
            context: None,
            stream: false,
        };

        let mut output = Vec::new();

        // This should NOT fail, even though the content is unparseable.
        let result = create_prd(&config, &runner, &mut output);

        assert!(
            result.is_ok(),
            "PRD creation should succeed despite parse failure"
        );

        let result = result.unwrap();

        // Verify we got a fallback PRD.
        assert_eq!(result.prd.id(), "PRD-0001");
        assert!(result.prd.title().contains("parse-fail-test"));
        assert_eq!(result.prd.status(), crate::prd::PrdStatus::Draft);

        // Verify warning was emitted to output.
        let output_str = String::from_utf8(output).unwrap();
        assert!(
            output_str.contains("Warning"),
            "Output should contain warning message"
        );
    }

    #[test]
    fn test_constitution_in_discovery_and_synthesis() {
        // Verifies that when a constitution file exists, its content
        // is loaded and included in both discovery and synthesis prompts.

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

        // Create prompt files that include constitution placeholder.
        std::fs::write(
            prompts_dir.join("prd_new_discovery.md"),
            "Discovery for {{slug}}{{#if constitution}}\n\nConstitution:\n{{constitution}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}{{#if constitution}}\n\nConstitution:\n{{constitution}}{{/if}}",
        )
        .unwrap();

        let prd_content = r"---
id: PRD-0001
title: Constitution Test
status: draft
tasks: []
---
# Summary
";

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(prd_content)]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "constitution-test",
            description: None,
            context: None,
            stream: false,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");

        // Verify constitution in discovery prompt.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("Acceptance tests must be codified"),
            "Discovery prompt should contain constitution content"
        );

        // Verify constitution in synthesis prompt.
        let recorded = runner.recorded_prompts();
        assert!(
            recorded[0].contains("Acceptance tests must be codified"),
            "Synthesis prompt should contain constitution content"
        );
    }

    #[test]
    fn test_existing_prds_injected_into_discovery_prompt() {
        // Verifies that existing PRD summaries are injected into the
        // interactive discovery prompt so the agent has project context.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Use the real template pattern for existing_prds.
        std::fs::write(
            prompts_dir.join("prd_new_discovery.md"),
            "Discovery for {{slug}}\n\n{{#each existing_prds}}\n- {{id}}: {{title}} ({{status}})\n{{/each}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}",
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

        let prd_content = r"---
id: PRD-0003
title: New Feature
status: draft
tasks: []
---
# Summary
";

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(prd_content)]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "new-feature",
            description: None,
            context: None,
            stream: false,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0003");

        // Verify existing PRDs appear in the discovery prompt.
        let interactive_prompts = runner.recorded_interactive_prompts();
        assert!(
            interactive_prompts[0].contains("PRD-0001"),
            "Discovery prompt should contain existing PRD-0001"
        );
        assert!(
            interactive_prompts[0].contains("Authentication System"),
            "Discovery prompt should contain PRD-0001 title"
        );
        assert!(
            interactive_prompts[0].contains("PRD-0002"),
            "Discovery prompt should contain existing PRD-0002"
        );
        assert!(
            interactive_prompts[0].contains("REST API Layer"),
            "Discovery prompt should contain PRD-0002 title"
        );
    }

    #[test]
    fn test_create_prd_aborts_on_interrupted_signal() {
        // Verifies that Ctrl+C (signal interruption) during the interactive
        // session aborts PRD creation entirely without creating a file.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_new_discovery.md"),
            "Discovery for {{slug}}",
        )
        .unwrap();

        let runner = MockRunner::empty();
        runner.set_interactive_error(crate::runner::RunnerError::Interrupted(
            "Interactive session terminated by signal 2 (SIGINT/Ctrl+C)".to_string(),
        ));

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "interrupted-test",
            description: None,
            context: None,
            stream: false,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output);

        assert!(result.is_err(), "PRD creation should fail on interrupt");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("interrupted"),
            "Error should mention interruption, got: {err_msg}"
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

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("prd_new_discovery.md"),
            "Discovery for {{slug}}",
        )
        .unwrap();

        let runner = MockRunner::empty();
        runner.set_interactive_error(crate::runner::RunnerError::ProcessFailed(
            "Interactive session exited with status: exit status: 1".to_string(),
        ));

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "failed-test",
            description: None,
            context: None,
            stream: false,
        };

        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut output);

        assert!(result.is_err(), "PRD creation should fail on process error");

        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("failed"),
            "Error should mention failure, got: {err_msg}"
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
}
