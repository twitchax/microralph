//! PRD editing via runner-assisted modifications.
//!
//! This module implements `mr prd edit` which allows quick modifications to
//! existing PRDs via a runner Q/A session.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::prd::{Prd, generate_index_from_root, parse_prd, scan_prds};
use crate::prd_new::QaPair;
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;

/// Maximum number of Q/A rounds before forcing application.
const MAX_QA_ROUNDS: usize = 3;

/// The ready signal from the runner.
const READY_SIGNAL: &str = "READY_TO_APPLY";

/// Result of the PRD edit process.
#[derive(Debug)]
pub struct PrdEditResult {
    /// The updated PRD.
    pub prd: Prd,

    /// The path where the PRD was written.
    pub path: PathBuf,

    /// Number of Q/A rounds.
    pub rounds: usize,

    /// The Q/A history (if any follow-up questions were asked).
    pub qa_history: Vec<QaPair>,
}

/// Configuration for the PRD edit command.
#[derive(Debug)]
pub struct PrdEditConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// The PRD ID to edit (e.g., "PRD-0001").
    pub prd_id: &'a str,

    /// The user's edit request.
    pub request: &'a str,
}

/// Runs the PRD edit flow.
///
/// This function:
/// 1. Loads the existing PRD
/// 2. Invokes the runner with the edit prompt
/// 3. Optionally collects user answers for follow-up questions
/// 4. Applies changes when runner signals ready
/// 5. Writes the updated PRD to disk
/// 6. Updates the index
pub fn edit_prd<R, I, O>(
    config: &PrdEditConfig,
    runner: &R,
    input: &mut I,
    output: &mut O,
) -> Result<PrdEditResult>
where
    R: Runner + ?Sized,
    I: BufRead,
    O: Write,
{
    writeln!(output, "Editing PRD: {}", config.prd_id)?;
    writeln!(output)?;

    // Find and load the PRD.
    let (prd_path, prd_content) = find_prd(config.root, config.prd_id)?;

    tracing::debug!(prd_id = %config.prd_id, prd_path = %prd_path.display(), runner = %runner.name(), "Starting PRD edit");

    // Initial edit request.
    writeln!(output, "Analyzing edit request...")?;

    let mut qa_history: Vec<QaPair> = Vec::new();
    let mut rounds = 0;

    loop {
        rounds += 1;

        if rounds > MAX_QA_ROUNDS {
            writeln!(output)?;
            writeln!(
                output,
                "Maximum Q/A rounds reached, proceeding with current context..."
            )?;
            break;
        }

        let prompt = build_edit_prompt(config, &prd_content, &qa_history);
        let runner_output = runner
            .execute(&prompt, config.root)
            .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?;

        if !runner_output.success {
            bail!("Runner failed during edit: {}", runner_output.text);
        }

        // Check if ready to apply.
        if runner_output.text.contains(READY_SIGNAL) {
            tracing::debug!("Runner signaled ready to apply");

            // Extract and apply the new PRD content.
            let new_content = extract_prd_content(&runner_output.text)?;
            let new_prd = parse_prd(&new_content).context("Failed to parse updated PRD")?;

            // Write the updated PRD.
            std::fs::write(&prd_path, &new_content).context("Failed to write updated PRD")?;

            writeln!(output)?;
            writeln!(output, "Updated PRD: {}", prd_path.display())?;

            // Update the index.
            generate_index_from_root(config.root)?;
            writeln!(output, "Updated PRD index")?;

            return Ok(PrdEditResult {
                prd: new_prd,
                path: prd_path,
                rounds,
                qa_history,
            });
        }

        // Parse follow-up questions.
        let questions = parse_questions(&runner_output.text);

        if questions.is_empty() {
            // No questions and no ready signal - try to extract content anyway
            if let Ok(new_content) = extract_prd_content(&runner_output.text) {
                let new_prd = parse_prd(&new_content).context("Failed to parse updated PRD")?;

                std::fs::write(&prd_path, &new_content).context("Failed to write updated PRD")?;

                writeln!(output)?;
                writeln!(output, "Updated PRD: {}", prd_path.display())?;

                generate_index_from_root(config.root)?;
                writeln!(output, "Updated PRD index")?;

                return Ok(PrdEditResult {
                    prd: new_prd,
                    path: prd_path,
                    rounds,
                    qa_history,
                });
            }

            bail!("Runner did not provide updated PRD content or follow-up questions");
        }

        writeln!(output)?;
        writeln!(output, "The runner needs some clarification:")?;
        writeln!(output)?;

        // Collect answers.
        let additional_qa = collect_answers(&questions, input, output)?;
        qa_history.extend(additional_qa);
    }

    // If we hit max rounds without ready signal, try to get final result
    let final_prompt = build_edit_prompt(config, &prd_content, &qa_history);
    let final_output = runner
        .execute(&final_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Runner failed: {e}"))?;

    if !final_output.success {
        bail!(
            "Runner failed during final edit attempt: {}",
            final_output.text
        );
    }

    let new_content = extract_prd_content(&final_output.text)?;
    let new_prd = parse_prd(&new_content).context("Failed to parse updated PRD")?;

    std::fs::write(&prd_path, &new_content).context("Failed to write updated PRD")?;

    writeln!(output)?;
    writeln!(output, "Updated PRD: {}", prd_path.display())?;

    generate_index_from_root(config.root)?;
    writeln!(output, "Updated PRD index")?;

    Ok(PrdEditResult {
        prd: new_prd,
        path: prd_path,
        rounds,
        qa_history,
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

    bail!("PRD not found: {prd_id}")
}

/// Builds the edit prompt with context.
fn build_edit_prompt(config: &PrdEditConfig, prd_content: &str, qa_history: &[QaPair]) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::PrdEdit);

    let mut ctx = PlaceholderContext::new();

    let prds_dir = config.root.join(".mr").join("prds");
    let prd_path = prds_dir.join(format!("{}.md", config.prd_id));
    ctx.insert("prd_path", prd_path.display().to_string());
    ctx.insert("user_request", config.request);
    ctx.insert("prd_content", prd_content);

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

/// Parses questions from runner output.
fn parse_questions(output: &str) -> Vec<String> {
    let mut questions = Vec::new();

    // Check if this looks like a READY_TO_APPLY response
    if output.contains(READY_SIGNAL) {
        return questions;
    }

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

            if !question.is_empty() && question.ends_with('?') {
                questions.push(question);
            }
        }
    }

    questions
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
fn extract_prd_content(output: &str) -> Result<String> {
    // Look for content after READY_TO_APPLY signal
    let content_start = if let Some(idx) = output.find(READY_SIGNAL) {
        &output[idx + READY_SIGNAL.len()..]
    } else {
        output
    };

    // Check for markdown code block.
    if let Some(start) = content_start.find("```markdown")
        && let Some(end) = content_start[start + 11..].find("```")
    {
        return Ok(content_start[start + 11..start + 11 + end]
            .trim()
            .to_string());
    }

    if let Some(start) = content_start.find("```yaml")
        && let Some(end) = content_start[start + 7..].find("```")
    {
        return Ok(content_start[start + 7..start + 7 + end].trim().to_string());
    }

    if let Some(start) = content_start.find("```") {
        let after_first = start + 3;

        // Skip the language identifier if present.
        let inner_start = content_start[after_first..]
            .find('\n')
            .map(|i| after_first + i + 1)
            .unwrap_or(after_first);

        if let Some(end) = content_start[inner_start..].find("```") {
            return Ok(content_start[inner_start..inner_start + end]
                .trim()
                .to_string());
        }
    }

    // Check if it looks like a PRD (starts with ---)
    let trimmed = content_start.trim();
    if trimmed.starts_with("---") {
        return Ok(trimmed.to_string());
    }

    bail!("Could not extract PRD content from runner output")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{MockRunner, RunnerOutput};
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
    fn test_parse_questions() {
        let output = r#"I need some clarification:

1. What should the new task priority be?
2. Should this replace the existing task?
"#;

        let questions = parse_questions(output);
        assert_eq!(questions.len(), 2);
        assert!(questions[0].contains("priority"));
        assert!(questions[1].contains("replace"));
    }

    #[test]
    fn test_parse_questions_with_ready_signal() {
        let output = r#"READY_TO_APPLY

```markdown
---
id: PRD-0001
...
```
"#;

        let questions = parse_questions(output);
        assert!(questions.is_empty());
    }

    #[test]
    fn test_extract_prd_content_markdown_block() {
        let output = r#"READY_TO_APPLY

```markdown
---
id: PRD-0001
title: Updated PRD
status: active
tasks: []
---

# Summary

Updated content.
```
"#;

        let content = extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
        assert!(content.contains("PRD-0001"));
    }

    #[test]
    fn test_extract_prd_content_plain() {
        let output = r#"READY_TO_APPLY

---
id: PRD-0001
title: Test
status: active
tasks: []
---

# Summary
"#;

        let content = extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
    }

    #[test]
    fn test_edit_prd_basic_flow() {
        let temp = setup_test_repo();
        create_test_prd(&temp, "PRD-0001", "Original Title");

        let updated_prd = r#"---
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

        let runner = MockRunner::new(vec![RunnerOutput::success(format!(
            "READY_TO_APPLY\n\n```markdown\n{updated_prd}\n```"
        ))]);

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            request: "Add a new task T-002 for testing",
        };

        let input = "";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.prd.id(), "PRD-0001");
        assert_eq!(result.prd.title(), "Updated Title");
        assert_eq!(result.rounds, 1);
        assert!(result.qa_history.is_empty());
    }

    #[test]
    fn test_edit_prd_with_followup() {
        let temp = setup_test_repo();
        create_test_prd(&temp, "PRD-0001", "Original Title");

        let updated_prd = r#"---
id: PRD-0001
title: "Original Title"
status: active

tasks:
  - id: T-001
    title: Initial task
    priority: 1
    status: todo
  - id: T-002
    title: High priority task
    priority: 1
    status: todo

---

# Summary

A test PRD with a new high priority task.

# History

(Entries appended by `mr run` will go below this line.)
"#;

        let runner = MockRunner::new(vec![
            RunnerOutput::success("1. What priority should the new task have?"),
            RunnerOutput::success(format!("READY_TO_APPLY\n\n```markdown\n{updated_prd}\n```")),
        ]);

        let config = PrdEditConfig {
            root: temp.path(),
            prd_id: "PRD-0001",
            request: "Add a new task T-002",
        };

        let input = "High priority\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let result = edit_prd(&config, &runner, &mut input, &mut output).unwrap();

        assert_eq!(result.rounds, 2);
        assert_eq!(result.qa_history.len(), 1);
        assert!(result.qa_history[0].answer.contains("High priority"));
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
    }
}
