//! PRD creation via guided Q/A flow.
//!
//! This module implements `mr new` which mediates a Q/A session between
//! the runner (coding agent) and the user to create a new PRD.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::config::load_constitution;
use crate::prd::{Prd, PrdSummary, generate_index_from_root, parse_prd, scan_prd_summaries};
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::qa_workflow::{self, QaPair};
use crate::runner::Runner;
use crate::spinner::start_spinner;

/// Maximum number of Q/A rounds before forcing synthesis.
const MAX_QA_ROUNDS: usize = 5;

/// The ready signal from the runner.
const READY_SIGNAL: &str = "READY_TO_SYNTHESIZE";

/// Result of the PRD creation process.
#[derive(Debug)]
pub struct PrdNewResult {
    /// The created PRD.
    pub prd: Prd,

    /// The path where the PRD was written.
    pub path: std::path::PathBuf,

    /// Number of Q/A rounds.
    pub rounds: usize,

    /// The Q/A history.
    pub qa_history: Vec<QaPair>,
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
    /// Used by `build_round1_prompt` and subsequent rounds.
    pub context: Option<&'a str>,

    /// Stream runner output to stdout in real-time.
    /// When true, spinner is disabled.
    pub stream: bool,
}

/// Runs the PRD creation flow.
///
/// This function:
/// 1. Scans existing PRDs for context
/// 2. Invokes the runner with round 1 questions
/// 3. Collects user answers
/// 4. Loops with round N questions until ready
/// 5. Synthesizes the final PRD
/// 6. Writes the PRD to disk
/// 7. Updates the index
/// 8. Updates AGENTS.md auto-managed section
///
/// # Multi-Round Q/A State Machine
///
/// This function implements a multi-round Q/A state machine with the following states:
///
/// 1. **Initialize**: Gather upfront context (CLI-provided or interactive prompt)
/// 2. **Round 1 - Question Generation**: Runner generates initial questions based on context
/// 3. **Round 1 - Answer Collection**: User provides answers to initial questions
/// 4. **Loop State - Round N Start**: Check if we've hit MAX_QA_ROUNDS limit
/// 5. **Loop State - Question Generation**: Runner examines Q/A history and generates follow-up questions
/// 6. **Loop State - Ready Check**: Parse runner response for READY_TO_SYNTHESIZE signal
/// 7. **Loop State - Additional Questions**: If runner provided more questions → collect answers
/// 8. **Loop State - Auto-Advance**: If no questions and no ready signal → proceed to synthesis
/// 9. **Synthesis**: Runner generates final PRD content from complete Q/A history
/// 10. **Persist**: Write PRD to disk (runner may create file directly or return content)
/// 11. **Finalize**: Update index and return result
///
/// The loop has multiple exit conditions:
/// - Runner signals READY_TO_SYNTHESIZE
/// - Runner returns no additional questions
/// - MAX_QA_ROUNDS limit reached
///
/// This adaptive approach allows the runner to gather just enough information without
/// burdening the user with unnecessary questions.
pub fn create_prd<R, I, O>(
    config: &PrdNewConfig,
    runner: &R,
    input: &mut I,
    output: &mut O,
) -> Result<PrdNewResult>
where
    R: Runner + ?Sized,
    I: BufRead,
    O: Write,
{
    writeln!(output, "Creating new PRD: {}", config.slug)?;
    writeln!(output)?;

    // STATE: Initialize - Determine user context for the PRD
    // Context can be provided via CLI flag (non-interactive) or prompted interactively
    let user_context: Option<String> = if config.context.is_some() {
        config.context.map(|s| s.to_string())
    } else {
        prompt_for_context(input, output)?
    };

    // Scan existing PRDs for context.
    let existing_prds = scan_prd_summaries(config.root)?;

    // Generate next PRD ID.
    let next_id = generate_next_prd_id(&existing_prds);
    tracing::debug!(next_id = %next_id, runner = %runner.name(), "Starting PRD creation");

    // STATE: Round 1 - Question Generation
    // Runner analyzes context and existing PRDs to generate initial questions
    let round1_prompt = build_round1_prompt(config, &existing_prds, user_context.as_deref());

    // Print command info before spinner (only when not streaming).
    if !config.stream
        && let Some(cmd_display) = runner.format_command_display(&round1_prompt, config.root)
    {
        println!("\n🔧 Executing: {cmd_display}");
    }

    tracing::info!(
        runner = %runner.name(),
        slug = %config.slug,
        "Invoking runner for PRD creation round 1"
    );

    let spinner = start_spinner(!config.stream, "Generating questions...");

    let round1_output = runner
        .execute(&round1_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?;

    spinner.finish_and_clear();

    if !round1_output.success {
        bail!("Runner failed during round 1: {}", round1_output.text);
    }

    let questions = qa_workflow::parse_questions(&round1_output.text);

    if questions.is_empty() {
        bail!("Runner did not generate any questions");
    }

    writeln!(output)?;
    writeln!(
        output,
        "Please answer the following questions to help create your PRD:"
    )?;
    writeln!(
        output,
        "💡 Tip: Press Enter twice (blank line) to complete each answer"
    )?;
    writeln!(output)?;

    // STATE: Round 1 - Answer Collection
    // User provides answers to initial questions; these become the Q/A history
    let mut qa_history = qa_workflow::collect_multiline_answers(&questions, input, output)?;
    let mut rounds = 1;

    // STATE: Loop State - Multi-round Q/A until runner is ready to synthesize
    // Each iteration builds on previous Q/A history to gather more specific details
    loop {
        rounds += 1;

        // STATE: Loop State - Check MAX_QA_ROUNDS limit
        // Force synthesis if we've asked too many rounds (prevents infinite loops)
        if rounds > MAX_QA_ROUNDS {
            writeln!(output)?;
            writeln!(
                output,
                "Maximum Q/A rounds reached, proceeding to synthesis..."
            )?;
            break;
        }

        // STATE: Loop State - Question Generation (Round N)
        // Runner reviews Q/A history and decides whether to ask follow-ups or signal readiness
        let round_n_prompt = build_round_n_prompt(config, &qa_history, user_context.as_deref());

        // Print command info before spinner (only when not streaming).
        if !config.stream
            && let Some(cmd_display) = runner.format_command_display(&round_n_prompt, config.root)
        {
            println!("\n🔧 Executing: {cmd_display}");
        }

        tracing::info!(
            runner = %runner.name(),
            round = rounds,
            slug = %config.slug,
            "Invoking runner for PRD creation follow-up round"
        );

        let spinner = start_spinner(
            !config.stream,
            format!("Generating follow-up questions (round {rounds})..."),
        );

        let round_n_output = runner
            .execute(&round_n_prompt, config.root)
            .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?;

        spinner.finish_and_clear();

        if !round_n_output.success {
            bail!(
                "Runner failed during round {rounds}: {}",
                round_n_output.text
            );
        }

        // STATE: Loop State - Ready Check
        // Runner signals READY_TO_SYNTHESIZE when it has enough information
        if round_n_output.text.contains(READY_SIGNAL) {
            tracing::debug!("Runner signaled ready to synthesize");
            break;
        }

        // STATE: Loop State - Additional Questions parsing
        // Runner may provide follow-up questions to clarify details
        let additional_questions = qa_workflow::parse_questions(&round_n_output.text);

        // STATE: Loop State - Auto-Advance
        // If runner didn't provide questions or signal ready, assume it's ready
        if additional_questions.is_empty() {
            tracing::debug!("No additional questions, proceeding to synthesis");
            break;
        }

        writeln!(output)?;
        writeln!(output, "A few more questions:")?;
        writeln!(
            output,
            "💡 Tip: Press Enter twice (blank line) to complete each answer"
        )?;
        writeln!(output)?;

        // Collect additional answers and append to Q/A history
        let additional_qa =
            qa_workflow::collect_multiline_answers(&additional_questions, input, output)?;
        qa_history.extend(additional_qa);
        // Continue loop for next round
    }

    // STATE: Synthesis - Generate final PRD from complete Q/A history
    // Runner creates PRD frontmatter (id, title, tasks, etc.) and body content
    writeln!(output)?;

    let synthesize_prompt =
        build_synthesize_prompt(config, &qa_history, &existing_prds, user_context.as_deref());

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

    // STATE: Persist - Write PRD to disk
    // Two strategies: (1) runner creates file directly, or (2) we parse runner's response and write
    // Strategy (1) is used in production, strategy (2) in tests with MockRunner
    let prds_dir = config.root.join(".mr").join("prds");

    // Look for the newly created PRD file matching our slug.
    let (prd, prd_path, prd_content) = if let Some((path, content)) =
        find_created_prd_file(&prds_dir, config.slug, &next_id)?
    {
        // Strategy (1): Runner created the file directly
        tracing::debug!(path = %path.display(), "Found PRD file created by runner");

        let parsed = match parse_prd(&content) {
            Ok(p) => p,
            Err(e) => {
                // Runner created file but it's malformed - keep it as-is but warn
                tracing::warn!(
                    path = %path.display(),
                    error = %e,
                    "Failed to parse PRD file created by runner, using expected values for operations"
                );
                writeln!(
                    output,
                    "⚠️  Warning: Failed to parse runner-created PRD, leaving file as-is"
                )?;

                // Construct minimal Prd for return using expected values
                // The file already contains the runner's actual output
                let frontmatter = crate::prd::types::PrdFrontmatter {
                    id: next_id.clone(),
                    title: format!("PRD for {}", config.slug),
                    status: crate::prd::PrdStatus::Draft,
                    ..Default::default()
                };
                crate::prd::Prd::new(frontmatter, String::new())
            }
        };

        (parsed, path, content)
    } else {
        // Strategy (2): Parse runner's response and write ourselves (fallback for tests)
        tracing::debug!("No PRD file found, parsing response content");

        let prd_content = match qa_workflow::extract_prd_content(&synthesize_output.text) {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "Failed to extract PRD content from runner output, using raw output"
                );
                // Use the raw output if we can't extract PRD content
                synthesize_output.text.clone()
            }
        };

        let prd = match parse_prd(&prd_content) {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    content_preview = ?prd_content.chars().take(200).collect::<String>(),
                    "Failed to parse synthesized PRD content, using expected values for operations"
                );
                writeln!(
                    output,
                    "⚠️  Warning: Failed to parse synthesized PRD content, leaving file as-is"
                )?;

                // Construct minimal Prd for return using expected values
                // The file will contain the runner's actual output
                let frontmatter = crate::prd::types::PrdFrontmatter {
                    id: next_id.clone(),
                    title: format!("PRD for {}", config.slug),
                    status: crate::prd::PrdStatus::Draft,
                    ..Default::default()
                };
                crate::prd::Prd::new(frontmatter, String::new())
            }
        };

        // Write the PRD to disk.
        let filename = format!("{}-{}.md", prd.id(), config.slug);
        let prd_path = prds_dir.join(&filename);

        std::fs::write(&prd_path, &prd_content).context("Failed to write PRD file")?;

        (prd, prd_path, prd_content)
    };

    // Validate the PRD has required fields
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

    writeln!(output)?;
    writeln!(output, "Created PRD: {}", prd_path.display())?;

    // STATE: Finalize - Update index to reflect new PRD
    generate_index_from_root(config.root)?;
    writeln!(output, "Updated PRD index")?;

    Ok(PrdNewResult {
        prd,
        path: prd_path,
        rounds,
        qa_history,
    })
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

        let filename = match path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n,
            None => continue,
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

/// Builds the round 1 prompt with context.
fn build_round1_prompt(
    config: &PrdNewConfig,
    existing_prds: &[PrdSummary],
    user_context: Option<&str>,
) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdNewRound1Questions);

    let mut ctx = PlaceholderContext::new();
    ctx.insert("slug", config.slug);

    if let Some(desc) = config.description {
        ctx.insert("user_description", desc);
    }

    if let Some(context) = user_context {
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

/// Builds the round N prompt with Q/A history.
fn build_round_n_prompt(
    config: &PrdNewConfig,
    qa_history: &[QaPair],
    user_context: Option<&str>,
) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdNewRoundNQuestions);

    let mut ctx = PlaceholderContext::new();
    ctx.insert("slug", config.slug);

    if let Some(context) = user_context {
        ctx.insert("user_context", context);
    }

    // Load constitution if available.
    if let Ok(Some(constitution)) = load_constitution(config.root) {
        ctx.insert("constitution", constitution);
    }

    // Build Q/A history.
    ctx.insert(
        "qa_history",
        PlaceholderValue::List(qa_workflow::to_placeholder_list(qa_history)),
    );

    expand_placeholders(&template, &ctx)
}

/// Builds the synthesis prompt.
fn build_synthesize_prompt(
    config: &PrdNewConfig,
    qa_history: &[QaPair],
    existing_prds: &[PrdSummary],
    user_context: Option<&str>,
) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdNewSynthesizePrd);

    let mut ctx = PlaceholderContext::new();
    ctx.insert("slug", config.slug);

    if let Some(context) = user_context {
        ctx.insert("user_context", context);
    }

    // Load constitution if available.
    if let Ok(Some(constitution)) = load_constitution(config.root) {
        ctx.insert("constitution", constitution);
    }

    // Build Q/A history.
    ctx.insert(
        "qa_history",
        PlaceholderValue::List(qa_workflow::to_placeholder_list(qa_history)),
    );

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

/// Prompts the user for optional upfront context.
///
/// Returns `Some(context)` if the user provides context, or `None` if they skip.
fn prompt_for_context<I, O>(input: &mut I, output: &mut O) -> Result<Option<String>>
where
    I: BufRead,
    O: Write,
{
    writeln!(
        output,
        "{}",
        crate::colors::question(
            "Would you like to provide additional context for the AI? (optional)"
        )
    )?;
    writeln!(
        output,
        "This helps generate more relevant questions. Press Enter to skip, or type your context:"
    )?;
    write!(output, "> ")?;
    output.flush()?;

    let mut context = String::new();
    input.read_line(&mut context)?;

    let trimmed = context.trim();

    if trimmed.is_empty() {
        Ok(None)
    } else {
        writeln!(output)?;
        Ok(Some(trimmed.to_string()))
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::qa_workflow;
    use crate::runner::MockRunner;
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
        let output = r#"Here are some questions:

1. What problem are you solving?
2. What does success look like?
3. Are there dependencies?
"#;

        let questions = qa_workflow::parse_questions(output);
        assert_eq!(questions.len(), 3);
        assert_eq!(questions[0], "What problem are you solving?");
        assert_eq!(questions[1], "What does success look like?");
        assert_eq!(questions[2], "Are there dependencies?");
    }

    #[test]
    fn test_parse_questions_numbered_paren() {
        let output = r#"1) First question?
2) Second question?
"#;

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
        let output = r#"Here are some questions:

1. What problem are you solving?
2. What features do you need?
   - Feature A
   - Feature B
   - Feature C
3. What is your timeline?

Some additional text here."#;

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
        let output = r#"Here's the PRD:

```markdown
---
id: PRD-0001
title: Test
---

# Summary

This is a test.
```

Done!
"#;

        let content = qa_workflow::extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
        assert!(content.contains("id: PRD-0001"));
    }

    #[test]
    fn test_extract_prd_content_md_fence() {
        // LLMs often use ```md instead of ```markdown
        let output = r#"Here's the PRD:

```md
---
id: PRD-0001
title: Test
---

# Summary
```
"#;

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
        let output = r#"---
id: PRD-0001
title: Test
---

# Summary
"#;

        let content = qa_workflow::extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
    }

    #[test]
    fn test_extract_prd_content_with_leading_text() {
        // Fallback: find --- in output even without proper fencing
        let output = r#"Sure, here's the PRD you asked for:

---
id: PRD-0001
title: Test
---

# Summary
"#;

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
    fn test_create_prd_basic_flow() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create minimal prompt files.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Generate questions for {{slug}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "Continue Q/A for {{slug}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}",
        )
        .unwrap();

        // Create mock runner with scripted responses.
        let prd_content = r#"---
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
"#;

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success(
                "1. What problem are you solving?\n2. What is the scope?",
            ),
            crate::runner::RunnerOutput::success("READY_TO_SYNTHESIZE"),
            crate::runner::RunnerOutput::success(prd_content),
        ]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "test-feature",
            description: None,
            context: None,
            stream: false,
        };

        let input = "Solving problem X\n\nMVP scope\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");
        assert_eq!(result.rounds, 2);
        assert_eq!(result.qa_history.len(), 2);
        assert!(result.path.exists());

        // Verify runner was called 3 times.
        assert_eq!(runner.recorded_prompts().len(), 3);
    }

    #[test]
    fn test_create_prd_max_rounds() {
        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create minimal prompt files.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Generate questions",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "Continue Q/A",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD",
        )
        .unwrap();

        let prd_content = r#"---
id: PRD-0001
title: Test
status: draft
tasks: []
---
# Summary
"#;

        // Create runner that never says ready.
        // Round 1 generates 1 question, then rounds 2-5 each generate 1 question.
        // At round 6, we break (rounds > MAX_QA_ROUNDS), so we need:
        // - 1 response for round1
        // - 4 responses for rounds 2-5 (loop runs while rounds <= MAX_QA_ROUNDS)
        // - 1 response for synthesis
        let mut responses = vec![crate::runner::RunnerOutput::success("1. Question?")];

        // We need MAX_QA_ROUNDS - 1 round N responses (rounds 2 through MAX_QA_ROUNDS)
        for _ in 0..(MAX_QA_ROUNDS - 1) {
            responses.push(crate::runner::RunnerOutput::success("1. Another question?"));
        }

        responses.push(crate::runner::RunnerOutput::success(prd_content));

        let runner = MockRunner::new(responses);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "test",
            description: None,
            context: None,
            stream: false,
        };

        // Provide enough answers: 1 for round1 + (MAX_QA_ROUNDS - 1) for subsequent rounds
        let input = "Answer\n\n".repeat(MAX_QA_ROUNDS);
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        // Should have hit max rounds.
        assert!(result.rounds > 1);
    }

    #[test]
    fn test_prd_new_context_interactive() {
        // UAT: uat-001 — Interactive flow prompts for context
        // This test verifies that when no --context flag is provided,
        // the create_prd flow prompts the user interactively for optional context.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create minimal prompt files.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Generate questions for {{slug}}{{#if user_context}} with context: {{user_context}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "Continue Q/A",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD",
        )
        .unwrap();

        let prd_content = r#"---
id: PRD-0001
title: Test Feature
status: draft
tasks: []
---
# Summary
"#;

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("1. What is the goal?"),
            crate::runner::RunnerOutput::success("READY_TO_SYNTHESIZE"),
            crate::runner::RunnerOutput::success(prd_content),
        ]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "test-feature",
            description: None,
            context: None, // No context flag provided
            stream: false,
        };

        // Simulate user input: provide context, then answer the question
        let input = "This is a test context for the feature\nThe goal is to test\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");

        // Verify output contains the context prompt
        let output_str = String::from_utf8(output).unwrap();
        assert!(output_str.contains("Would you like to provide additional context"));

        // Verify the first prompt includes the user context
        let recorded = runner.recorded_prompts();
        assert!(recorded[0].contains("This is a test context for the feature"));
    }

    #[test]
    fn test_prd_new_context_flag() {
        // UAT: uat-002 — Flag flow uses provided context
        // This test verifies that when --context flag is provided,
        // the context is used directly without an interactive prompt.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create minimal prompt files.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Generate questions for {{slug}}{{#if user_context}} with context: {{user_context}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "Continue Q/A",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD",
        )
        .unwrap();

        let prd_content = r#"---
id: PRD-0002
title: Flag Feature
status: draft
tasks: []
---
# Summary
"#;

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("1. What is the goal?"),
            crate::runner::RunnerOutput::success("READY_TO_SYNTHESIZE"),
            crate::runner::RunnerOutput::success(prd_content),
        ]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "flag-feature",
            description: None,
            context: Some("This context came from the --context flag"),
            stream: false,
        };

        // Simulate user input: only the answer to the question (no context prompt expected)
        let input = "The goal is to test the flag\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0002");

        // Verify output does NOT contain the context prompt (flag was provided)
        let output_str = String::from_utf8(output).unwrap();
        assert!(!output_str.contains("Would you like to provide additional context"));

        // Verify the first prompt includes the flag-provided context
        let recorded = runner.recorded_prompts();
        assert!(recorded[0].contains("This context came from the --context flag"));
    }

    #[test]
    fn test_prd_new_context_in_questions() {
        // UAT: uat-003 — Context influences question generation
        // This test verifies that user-provided context actually influences
        // the questions generated by the AI in the round1 prompt.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create minimal prompt files that demonstrate context inclusion.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Generate questions for {{slug}}{{#if user_context}} considering context: {{user_context}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "Continue Q/A",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD",
        )
        .unwrap();

        let prd_content = r#"---
id: PRD-0003
title: Context Test
status: draft
tasks: []
---
# Summary
"#;

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("1. What is the goal?"),
            crate::runner::RunnerOutput::success("READY_TO_SYNTHESIZE"),
            crate::runner::RunnerOutput::success(prd_content),
        ]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "context-test",
            description: None,
            context: Some("This is a payment processing feature for e-commerce"),
            stream: false,
        };

        let input = "Process payments securely\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0003");

        // Verify the round1 prompt includes context influence marker
        let recorded = runner.recorded_prompts();
        assert!(
            recorded[0].contains("considering context:"),
            "Round1 prompt should contain context influence marker"
        );
        assert!(
            recorded[0].contains("This is a payment processing feature for e-commerce"),
            "Round1 prompt should contain actual user context"
        );
    }

    #[test]
    fn test_prd_new_context_persistence() {
        // UAT: uat-004 — Context persists through Q/A rounds
        // This test verifies that user-provided context is carried through
        // all Q/A rounds, not just round 1.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create prompt files that show context in both round1 and roundN.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Round1 for {{slug}}{{#if user_context}} with context: {{user_context}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "RoundN for {{slug}}{{#if user_context}} with persisted context: {{user_context}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD",
        )
        .unwrap();

        let prd_content = r#"---
id: PRD-0004
title: Persistence Test
status: draft
tasks: []
---
# Summary
"#;

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("1. First question?"),
            crate::runner::RunnerOutput::success("2. Follow-up question?"), // RoundN
            crate::runner::RunnerOutput::success("READY_TO_SYNTHESIZE"),    // RoundN ready signal
            crate::runner::RunnerOutput::success(prd_content),
        ]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "persistence-test",
            description: None,
            context: Some("Multi-tenant auth system with role-based access"),
            stream: false,
        };

        let input = "Answer 1\n\nAnswer 2\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0004");
        assert_eq!(result.rounds, 3, "Should have 3 rounds (round1 + 2 roundN)");

        // Verify context in round1 prompt (index 0)
        let recorded = runner.recorded_prompts();
        assert!(
            recorded[0].contains("with context:"),
            "Round1 prompt should contain context marker"
        );
        assert!(
            recorded[0].contains("Multi-tenant auth system with role-based access"),
            "Round1 prompt should contain user context"
        );

        // Verify context persists in roundN prompt (index 1)
        assert!(
            recorded[1].contains("with persisted context:"),
            "RoundN prompt should contain persisted context marker"
        );
        assert!(
            recorded[1].contains("Multi-tenant auth system with role-based access"),
            "RoundN prompt should contain the same user context"
        );
    }

    #[test]
    fn test_prd_new_context_synthesis() {
        // UAT: uat-005 — Context included in final synthesis
        // This test verifies that user-provided context is included in the final
        // PRD synthesis prompt so the AI can use it during PRD generation.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create prompt files, with synthesis prompt showing context inclusion.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Round1 for {{slug}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "RoundN for {{slug}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}{{#if user_context}} with synthesis context: {{user_context}}{{/if}}",
        )
        .unwrap();

        let prd_content = r#"---
id: PRD-0005
title: Synthesis Context Test
status: draft
tasks: []
---
# Summary
"#;

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("1. First question?"),
            crate::runner::RunnerOutput::success("READY_TO_SYNTHESIZE"),
            crate::runner::RunnerOutput::success(prd_content),
        ]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "synthesis-test",
            description: None,
            context: Some("API Gateway with rate limiting and JWT auth"),
            stream: false,
        };

        let input = "Answer 1\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0005");
        assert_eq!(
            result.rounds, 2,
            "Should have 2 rounds (round1 + ready signal)"
        );

        // Verify context in final synthesis prompt (index 2, after round1 and roundN ready signal)
        let recorded = runner.recorded_prompts();
        let synthesis_prompt = &recorded[2];

        assert!(
            synthesis_prompt.contains("with synthesis context:"),
            "Synthesis prompt should contain context marker"
        );
        assert!(
            synthesis_prompt.contains("API Gateway with rate limiting and JWT auth"),
            "Synthesis prompt should contain user context so AI can use it during PRD generation"
        );
    }

    #[test]
    fn test_prd_new_parse_failure_warning() {
        // This test verifies that when the runner returns unparseable content,
        // we emit a warning and create a fallback PRD rather than failing.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create minimal prompt files.
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Generate questions",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "Continue Q/A",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD",
        )
        .unwrap();

        // Return invalid PRD content that cannot be parsed
        let invalid_content = r#"This is not valid PRD content.
It has no frontmatter and will fail to parse.
Just some random text."#;

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("1. What is the goal?"),
            crate::runner::RunnerOutput::success("READY_TO_SYNTHESIZE"),
            crate::runner::RunnerOutput::success(invalid_content),
        ]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "parse-fail-test",
            description: None,
            context: None,
            stream: false,
        };

        let input = "Test goal\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        // This should NOT fail, even though the content is unparseable
        let result = create_prd(&config, &runner, &mut input, &mut output);

        assert!(
            result.is_ok(),
            "PRD creation should succeed despite parse failure"
        );

        let result = result.unwrap();

        // Verify we got a fallback PRD
        assert_eq!(result.prd.id(), "PRD-0001");
        assert!(result.prd.title().contains("parse-fail-test"));
        assert_eq!(result.prd.status(), crate::prd::PrdStatus::Draft);

        // Verify warning was emitted to output
        let output_str = String::from_utf8(output).unwrap();
        assert!(
            output_str.contains("Warning"),
            "Output should contain warning message"
        );
    }

    #[test]
    fn test_constitution_prd_new() {
        // UAT: constitution_prd_new — Verify prd new reads and respects constitution
        // This test verifies that when a constitution file exists, its content
        // is loaded and included in all prompts during PRD creation.

        let temp = setup_test_repo();
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create constitution file
        let constitution_content = r#"# Constitution

## Purpose
Project governance rules.

## Rules
1. **Acceptance tests must be codified** — No one-off manual tests.
2. **Use semantic versioning** — All releases follow semver.
"#;
        std::fs::write(
            temp.path().join(".mr").join("constitution.md"),
            constitution_content,
        )
        .unwrap();

        // Create prompt files that include constitution placeholder
        std::fs::write(
            prompts_dir.join("prd_new_round1_questions.md"),
            "Round1 for {{slug}}{{#if constitution}}\n\nConstitution:\n{{constitution}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_roundN_questions.md"),
            "RoundN for {{slug}}{{#if constitution}}\n\nConstitution:\n{{constitution}}{{/if}}",
        )
        .unwrap();
        std::fs::write(
            prompts_dir.join("prd_new_synthesize_prd.md"),
            "Synthesize PRD for {{slug}}{{#if constitution}}\n\nConstitution:\n{{constitution}}{{/if}}",
        )
        .unwrap();

        let prd_content = r#"---
id: PRD-0001
title: Constitution Test
status: draft
tasks: []
---
# Summary
"#;

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("1. First question?"),
            crate::runner::RunnerOutput::success("READY_TO_SYNTHESIZE"),
            crate::runner::RunnerOutput::success(prd_content),
        ]);

        let config = PrdNewConfig {
            root: temp.path(),
            slug: "constitution-test",
            description: None,
            context: None,
            stream: false,
        };

        let input = "Answer 1\n\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");

        // Verify constitution was loaded and included in all prompts
        let recorded = runner.recorded_prompts();

        // Round 1 should include constitution
        assert!(
            recorded[0].contains("Acceptance tests must be codified"),
            "Round1 prompt should contain constitution content"
        );

        // Round N (ready signal response) should include constitution
        assert!(
            recorded[1].contains("Acceptance tests must be codified"),
            "RoundN prompt should contain constitution content"
        );

        // Synthesis should include constitution
        assert!(
            recorded[2].contains("Acceptance tests must be codified"),
            "Synthesis prompt should contain constitution content"
        );
    }
}
