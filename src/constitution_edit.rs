//! Constitution editing via runner-assisted modifications.
//!
//! This module implements `mr constitution edit` which allows intelligent updates
//! to the project constitution via a runner Q/A session.

use std::collections::HashMap;
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

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

/// Result of the constitution edit process.
#[derive(Debug)]
pub struct ConstitutionEditResult {
    /// The path where the constitution was written.
    pub path: PathBuf,

    /// Number of Q/A rounds.
    pub rounds: usize,

    /// The Q/A history (if any follow-up questions were asked).
    pub qa_history: Vec<QaPair>,
}

/// Configuration for the constitution edit command.
#[derive(Debug)]
pub struct ConstitutionEditConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// The user's edit request.
    pub request: &'a str,
}

/// Runs the constitution edit flow.
///
/// This function:
/// 1. Loads the existing constitution
/// 2. Invokes the runner with the edit prompt
/// 3. Optionally collects user answers for follow-up questions
/// 4. Applies changes when runner signals ready
/// 5. Writes the updated constitution to disk
pub fn edit_constitution<R, I, O>(
    config: &ConstitutionEditConfig,
    runner: &R,
    input: &mut I,
    output: &mut O,
) -> Result<ConstitutionEditResult>
where
    R: Runner + ?Sized,
    I: BufRead,
    O: Write,
{
    writeln!(output, "Editing constitution...")?;
    writeln!(output)?;

    // Find and load the constitution.
    let constitution_path = config.root.join(".mr").join("constitution.md");
    if !constitution_path.exists() {
        bail!(
            "Constitution file not found at {}",
            constitution_path.display()
        );
    }

    let constitution_content = std::fs::read_to_string(&constitution_path).with_context(|| {
        format!(
            "Failed to read constitution at {}",
            constitution_path.display()
        )
    })?;

    tracing::debug!(constitution_path = %constitution_path.display(), runner = %runner.name(), "Starting constitution edit");

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

        let runner_response = invoke_runner(
            runner,
            config.root,
            config.request,
            &constitution_content,
            &qa_history,
        )?;

        tracing::debug!(round = %rounds, response_len = %runner_response.len(), "Received runner response");

        // Check if runner is ready to apply or has questions.
        if runner_response.contains(READY_SIGNAL) {
            writeln!(output, "Constitution changes ready to apply.")?;
            writeln!(output)?;

            // Extract the updated constitution content.
            let updated_content = extract_constitution_content(&runner_response)?;

            // Write the updated constitution.
            std::fs::write(&constitution_path, updated_content).with_context(|| {
                format!(
                    "Failed to write constitution to {}",
                    constitution_path.display()
                )
            })?;

            writeln!(output, "Constitution updated successfully.")?;

            return Ok(ConstitutionEditResult {
                path: constitution_path,
                rounds,
                qa_history,
            });
        } else {
            // Runner has questions—extract and ask the user.
            let questions = extract_questions(&runner_response)?;

            if questions.is_empty() {
                bail!(
                    "Runner response did not contain '{}' signal or questions",
                    READY_SIGNAL
                );
            }

            writeln!(output)?;
            for q in &questions {
                writeln!(output, "Q: {}", q)?;
            }
            writeln!(output)?;

            // Collect answers.
            for question in questions {
                write!(output, "> ")?;
                output.flush()?;

                let mut answer = String::new();
                input.read_line(&mut answer)?;
                let answer = answer.trim().to_string();

                qa_history.push(QaPair {
                    question: question.clone(),
                    answer: answer.clone(),
                });
            }
        }
    }

    // If we exit the loop without returning, something went wrong.
    bail!(
        "Failed to complete constitution edit after {} rounds",
        rounds
    )
}

/// Invokes the runner with the constitution edit prompt.
fn invoke_runner<R>(
    runner: &R,
    root: &Path,
    request: &str,
    constitution_content: &str,
    qa_history: &[QaPair],
) -> Result<String>
where
    R: Runner + ?Sized,
{
    let prompt = build_constitution_edit_prompt(root, request, constitution_content, qa_history);

    tracing::debug!(prompt_len = %prompt.len(), "Invoking runner with constitution edit prompt");

    let output = runner.execute(&prompt, root)?;

    if !output.success {
        bail!("Runner failed: {}", output.text);
    }

    Ok(output.text)
}

/// Builds the constitution edit prompt with context.
fn build_constitution_edit_prompt(
    root: &Path,
    request: &str,
    constitution_content: &str,
    qa_history: &[QaPair],
) -> String {
    let prompt_template = load_prompt_with_fallback(root, PromptKind::ConstitutionEdit);

    let mut context = PlaceholderContext::new();
    context.insert("user_request", request);
    context.insert("constitution_content", constitution_content);

    if !qa_history.is_empty() {
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
        context.insert("qa_history", PlaceholderValue::List(qa_list));
    }

    expand_placeholders(&prompt_template, &context)
}

/// Extracts the updated constitution content from the runner response.
fn extract_constitution_content(response: &str) -> Result<String> {
    // Look for markdown code block after READY_TO_APPLY.
    let after_signal = response
        .split(READY_SIGNAL)
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("No content found after {} signal", READY_SIGNAL))?;

    let start = after_signal
        .find("```markdown")
        .or_else(|| after_signal.find("```"))
        .ok_or_else(|| anyhow::anyhow!("No markdown code block found in runner response"))?;

    let content_start = after_signal[start..]
        .find('\n')
        .map(|i| start + i + 1)
        .ok_or_else(|| anyhow::anyhow!("Malformed code block"))?;

    let content_end = after_signal[content_start..]
        .find("```")
        .ok_or_else(|| anyhow::anyhow!("Unclosed code block"))?;

    let constitution_content = &after_signal[content_start..content_start + content_end];
    Ok(constitution_content.trim().to_string())
}

/// Extracts questions from the runner response.
fn extract_questions(response: &str) -> Result<Vec<String>> {
    let mut questions = Vec::new();

    for line in response.lines() {
        let trimmed = line.trim();
        // Look for numbered questions like "1. Question?"
        if let Some(rest) = trimmed.strip_prefix(char::is_numeric)
            && let Some(question) = rest.strip_prefix('.').or_else(|| rest.strip_prefix(')'))
        {
            questions.push(question.trim().to_string());
        }
    }

    Ok(questions)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_constitution_content() {
        let response = r#"
READY_TO_APPLY

```markdown
# Constitution

## Purpose
Test constitution

## Rules
1. **Rule one**: Description
```
"#;

        let result = extract_constitution_content(response).unwrap();
        assert!(result.contains("# Constitution"));
        assert!(result.contains("## Rules"));
        assert!(result.contains("1. **Rule one**"));
    }

    #[test]
    fn test_extract_questions() {
        let response = r#"
I need more information:

1. What is the scope?
2. Should this apply to all PRDs?
3. Any exceptions?
"#;

        let questions = extract_questions(response).unwrap();
        assert_eq!(questions.len(), 3);
        assert_eq!(questions[0], "What is the scope?");
        assert_eq!(questions[1], "Should this apply to all PRDs?");
        assert_eq!(questions[2], "Any exceptions?");
    }
}
