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

    let interactive_result = runner
        .execute_interactive(&discovery_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Interactive session failed (aborted or error): {e}"))?;

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

    let synthesize_output = runner
        .execute(&synthesize_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?;

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
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdNewRound1Questions);

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
    fn test_parse_questions_numbered_dot() {
        let output = r"Here are some questions:

1. What problem are you solving?
2. What does success look like?
3. Are there dependencies?
";

        let questions = qa_workflow::parse_questions(output);
        assert_eq!(questions.len(), 3);
        assert_eq!(questions[0], "What problem are you solving?");
        assert_eq!(questions[1], "What does success look like?");
        assert_eq!(questions[2], "Are there dependencies?");
    }

    #[test]
    fn test_parse_questions_numbered_paren() {
        let output = r"1) First question?
2) Second question?
";

        let questions = qa_workflow::parse_questions(output);
        assert_eq!(questions.len(), 2);
    }

    #[test]
    fn test_parse_questions_empty() {
        let output = "No questions here, just text.";
        let questions = qa_workflow::parse_questions(output);
        assert!(questions.is_empty());
    }

    #[test]
    fn test_parse_questions_multiline_with_bullets() {
        let output = r"Here are some questions:

1. What problem are you solving?
2. What features do you need?
   - Feature A
   - Feature B
   - Feature C
3. What is your timeline?

Some additional text here.";

        let questions = qa_workflow::parse_questions(output);
        assert_eq!(questions.len(), 3);
        assert_eq!(questions[0], "What problem are you solving?");
        assert_eq!(
            questions[1],
            "What features do you need?\n- Feature A\n- Feature B\n- Feature C"
        );
        assert_eq!(questions[2], "What is your timeline?");
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
    fn test_collect_answers() {
        let questions = vec!["Question 1?".to_string(), "Question 2?".to_string()];

        let input = "Answer 1\n\nAnswer 2\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let pairs =
            qa_workflow::collect_multiline_answers(&questions, &mut input, &mut output).unwrap();

        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].question, "Question 1?");
        assert_eq!(pairs[0].answer, "Answer 1");
        assert_eq!(pairs[1].question, "Question 2?");
        assert_eq!(pairs[1].answer, "Answer 2");
    }

    #[test]
    fn test_collect_answers_multiline() {
        let questions = vec!["Describe your feature?".to_string()];

        let input = "Line 1\nLine 2\nLine 3\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let pairs =
            qa_workflow::collect_multiline_answers(&questions, &mut input, &mut output).unwrap();

        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].question, "Describe your feature?");
        assert_eq!(pairs[0].answer, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_create_prd_two_phase_flow() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create minimal prompt files.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
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
            prompts_dir.join("prd_new_round1_questions.md"),
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
            prompts_dir.join("prd_new_round1_questions.md"),
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
            prompts_dir.join("prd_new_round1_questions.md"),
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

        std::fs::write(prompts_dir.join("prd_new_round1_questions.md"), "Discovery").unwrap();
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
            prompts_dir.join("prd_new_round1_questions.md"),
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
}
