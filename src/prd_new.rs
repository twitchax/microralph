//! PRD creation via guided Q/A flow.
//!
//! This module implements `mr prd new` which mediates a Q/A session between
//! the runner (coding agent) and the user to create a new PRD.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::agents::{RecentChange, update_agents_md};
use crate::prd::{Prd, PrdSummary, generate_index_from_root, parse_prd, scan_prd_summaries};
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;

/// Maximum number of Q/A rounds before forcing synthesis.
const MAX_QA_ROUNDS: usize = 5;

/// The ready signal from the runner.
const READY_SIGNAL: &str = "READY_TO_SYNTHESIZE";

/// A question-answer pair from the Q/A session.
#[derive(Debug, Clone)]
pub struct QaPair {
    pub question: String,
    pub answer: String,
}

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

    // Determine user context: use CLI-provided context, or prompt interactively.
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

    // Round 1: Get initial questions.
    writeln!(output, "Generating questions...")?;

    let round1_prompt = build_round1_prompt(config, &existing_prds, user_context.as_deref());
    let round1_output = runner
        .execute(&round1_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?;

    if !round1_output.success {
        bail!("Runner failed during round 1: {}", round1_output.text);
    }

    let questions = parse_questions(&round1_output.text);

    if questions.is_empty() {
        bail!("Runner did not generate any questions");
    }

    writeln!(output)?;
    writeln!(
        output,
        "Please answer the following questions to help create your PRD:"
    )?;
    writeln!(output)?;

    // Collect answers.
    let mut qa_history = collect_answers(&questions, input, output)?;
    let mut rounds = 1;

    // Loop with round N until ready.
    loop {
        rounds += 1;

        if rounds > MAX_QA_ROUNDS {
            writeln!(output)?;
            writeln!(
                output,
                "Maximum Q/A rounds reached, proceeding to synthesis..."
            )?;
            break;
        }

        let round_n_prompt = build_round_n_prompt(config, &qa_history, user_context.as_deref());
        let round_n_output = runner
            .execute(&round_n_prompt, config.root)
            .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?;

        if !round_n_output.success {
            bail!(
                "Runner failed during round {rounds}: {}",
                round_n_output.text
            );
        }

        // Check if ready to synthesize.
        if round_n_output.text.contains(READY_SIGNAL) {
            tracing::debug!("Runner signaled ready to synthesize");
            break;
        }

        // Parse additional questions.
        let additional_questions = parse_questions(&round_n_output.text);

        if additional_questions.is_empty() {
            tracing::debug!("No additional questions, proceeding to synthesis");
            break;
        }

        writeln!(output)?;
        writeln!(output, "A few more questions:")?;
        writeln!(output)?;

        // Collect additional answers.
        let additional_qa = collect_answers(&additional_questions, input, output)?;
        qa_history.extend(additional_qa);
    }

    // Synthesize the PRD.
    writeln!(output)?;
    writeln!(output, "Synthesizing PRD...")?;

    let synthesize_prompt = build_synthesize_prompt(config, &qa_history, &existing_prds);
    let synthesize_output = runner
        .execute(&synthesize_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?;

    if !synthesize_output.success {
        bail!("Runner failed during synthesis: {}", synthesize_output.text);
    }

    // Parse the PRD content.
    let prd_content = extract_prd_content(&synthesize_output.text);
    let prd = parse_prd(&prd_content).context("Failed to parse synthesized PRD")?;

    // Write the PRD to disk.
    let filename = format!("{}-{}.md", prd.id(), config.slug);
    let prd_path = config.root.join(".mr").join("prds").join(&filename);

    std::fs::write(&prd_path, &prd_content).context("Failed to write PRD file")?;

    writeln!(output)?;
    writeln!(output, "Created PRD: {}", prd_path.display())?;

    // Update the index.
    generate_index_from_root(config.root)?;
    writeln!(output, "Updated PRD index")?;

    // Update AGENTS.md with recent changes.
    let changes = vec![RecentChange {
        file: prd_path.display().to_string(),
        description: format!("Created PRD: {} ({})", prd.id(), prd.title()),
    }];

    let agents_result = update_agents_md(config.root, runner, &changes);
    match agents_result {
        Ok(result) if result.modified => {
            writeln!(output, "Updated AGENTS.md auto-managed section")?;
            if let Some(content) = &result.new_content {
                tracing::debug!(content_len = content.len(), "AGENTS.md new section content");
            }
        }
        Ok(_) => {
            tracing::debug!("No changes needed for AGENTS.md");
        }
        Err(e) => {
            tracing::warn!("Failed to update AGENTS.md: {e}");
        }
    }

    Ok(PrdNewResult {
        prd,
        path: prd_path,
        rounds,
        qa_history,
    })
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

    // Build Q/A history.
    let qa_list: Vec<HashMap<String, String>> = qa_history
        .iter()
        .map(|qa| {
            [
                ("question".to_string(), qa.question.clone()),
                ("answer".to_string(), qa.answer.clone()),
            ]
            .into_iter()
            .collect()
        })
        .collect();

    ctx.insert("qa_history", PlaceholderValue::List(qa_list));

    expand_placeholders(&template, &ctx)
}

/// Builds the synthesis prompt.
fn build_synthesize_prompt(
    config: &PrdNewConfig,
    qa_history: &[QaPair],
    existing_prds: &[PrdSummary],
) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdNewSynthesizePrd);

    let mut ctx = PlaceholderContext::new();
    ctx.insert("slug", config.slug);

    // Build Q/A history.
    let qa_list: Vec<HashMap<String, String>> = qa_history
        .iter()
        .map(|qa| {
            [
                ("question".to_string(), qa.question.clone()),
                ("answer".to_string(), qa.answer.clone()),
            ]
            .into_iter()
            .collect()
        })
        .collect();

    ctx.insert("qa_history", PlaceholderValue::List(qa_list));

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

/// Parses questions from runner output.
///
/// Expects a numbered list like:
/// 1. Question one?
/// 2. Question two?
fn parse_questions(output: &str) -> Vec<String> {
    let mut questions = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();

        // Match numbered questions (1., 2., etc.).
        if let Some(rest) = trimmed
            .strip_prefix(|c: char| c.is_ascii_digit())
            .and_then(|s| s.strip_prefix('.'))
            .or_else(|| {
                trimmed
                    .strip_prefix(|c: char| c.is_ascii_digit())
                    .and_then(|s| s.strip_prefix(')'))
            })
        {
            let question = rest.trim().to_string();

            if !question.is_empty() {
                questions.push(question);
            }
        }
    }

    questions
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
        "Would you like to provide additional context for the AI? (optional)"
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

/// Collects answers from the user for each question.
fn collect_answers<I, O>(questions: &[String], input: &mut I, output: &mut O) -> Result<Vec<QaPair>>
where
    I: BufRead,
    O: Write,
{
    let mut pairs = Vec::new();

    for (i, question) in questions.iter().enumerate() {
        writeln!(output, "{}. {}", i + 1, question)?;
        write!(output, "   > ")?;
        output.flush()?;

        let mut answer = String::new();
        input.read_line(&mut answer)?;

        pairs.push(QaPair {
            question: question.clone(),
            answer: answer.trim().to_string(),
        });
    }

    Ok(pairs)
}

/// Extracts PRD content from runner output.
///
/// Handles markdown code blocks if present, with robust fence detection.
/// PRDs must start with `---` frontmatter, so we look for that as the
/// primary content indicator.
fn extract_prd_content(output: &str) -> String {
    let trimmed = output.trim();

    // If output starts directly with frontmatter, use it as-is.
    if trimmed.starts_with("---") {
        return trimmed.to_string();
    }

    // Try to find a code fence containing frontmatter.
    // Handle various fence patterns: ```markdown, ```md, ```yaml, ``` (generic).
    for fence_pattern in ["```markdown", "```md", "```yaml", "```"] {
        if let Some(fence_start) = trimmed.find(fence_pattern) {
            let after_fence = fence_start + fence_pattern.len();

            // Skip to the next newline (past any remaining language identifier).
            let content_start = trimmed[after_fence..]
                .find('\n')
                .map(|i| after_fence + i + 1)
                .unwrap_or(after_fence);

            // Look for the closing fence, but find the LAST one to handle
            // nested code blocks inside the PRD content.
            let remaining = &trimmed[content_start..];

            if let Some(end) = remaining.rfind("\n```") {
                let content = &remaining[..end];

                // Verify this looks like a PRD (starts with ---).
                let content_trimmed = content.trim();

                if content_trimmed.starts_with("---") {
                    return content_trimmed.to_string();
                }
            }

            // Fallback: try to find closing fence, even if not at line start.
            if let Some(end) = remaining.rfind("```") {
                let content = &remaining[..end];
                let content_trimmed = content.trim();

                if content_trimmed.starts_with("---") {
                    return content_trimmed.to_string();
                }
            }
        }
    }

    // Last resort: look for --- delimiters directly in the output.
    // Find the first --- and extract from there.
    if let Some(fm_start) = trimmed.find("---") {
        return trimmed[fm_start..].trim().to_string();
    }

    // No recognizable PRD content found, return as-is.
    trimmed.to_string()
}

#[cfg(test)]
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
                references: vec![],
            },
            PrdSummary {
                id: "PRD-0003".to_string(),
                title: "Third".to_string(),
                status: crate::prd::PrdStatus::Done,
                relative_path: "prds/test2.md".to_string(),
                completed_tasks: 0,
                total_tasks: 0,
                references: vec![],
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

        let questions = parse_questions(output);
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

        let questions = parse_questions(output);
        assert_eq!(questions.len(), 2);
    }

    #[test]
    fn test_parse_questions_empty() {
        let output = "No questions here, just text.";
        let questions = parse_questions(output);
        assert!(questions.is_empty());
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

        let content = extract_prd_content(output);
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

        let content = extract_prd_content(output);
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

        let content = extract_prd_content(output);
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

        let content = extract_prd_content(output);
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

        let content = extract_prd_content(output);
        assert!(content.starts_with("---"), "Content was: {content}");
        assert!(content.contains("id: PRD-0001"));
    }

    #[test]
    fn test_collect_answers() {
        let questions = vec!["Question 1?".to_string(), "Question 2?".to_string()];

        let input = "Answer 1\nAnswer 2\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let pairs = collect_answers(&questions, &mut input, &mut output).unwrap();

        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].question, "Question 1?");
        assert_eq!(pairs[0].answer, "Answer 1");
        assert_eq!(pairs[1].question, "Question 2?");
        assert_eq!(pairs[1].answer, "Answer 2");
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
        };

        let input = "Solving problem X\nMVP scope\n";
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
        };

        // Provide enough answers: 1 for round1 + (MAX_QA_ROUNDS - 1) for subsequent rounds
        let input = "Answer\n".repeat(MAX_QA_ROUNDS);
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = create_prd(&config, &runner, &mut input, &mut output).unwrap();

        // Should have hit max rounds.
        assert!(result.rounds > 1);
    }
}
