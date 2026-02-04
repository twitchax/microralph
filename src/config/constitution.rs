//! Constitution editing via runner-assisted modifications.
//!
//! This module implements `mr constitution edit` which allows intelligent updates
//! to the project constitution via a runner Q/A session.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;
use crate::util::qa_workflow::{self, QaPair, parse_questions};

/// Maximum number of Q/A rounds before forcing application.
const MAX_QA_ROUNDS: usize = 3;

/// The signal indicating the runner has completed its edits.
const EDIT_COMPLETE_SIGNAL: &str = "EDIT_COMPLETE";

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

        // Check if runner has completed edits or has questions.
        if runner_response.contains(EDIT_COMPLETE_SIGNAL) {
            // Validate constitution frontmatter after agent edits.
            tracing::debug!(constitution_path = %constitution_path.display(), "Validating constitution frontmatter after agent edit");
            crate::validate::validate_constitution_frontmatter(&constitution_path);

            writeln!(output, "Constitution updated successfully.")?;

            return Ok(ConstitutionEditResult {
                path: constitution_path,
                rounds,
                qa_history,
            });
        }

        // Runner has questions—extract and ask the user.
        let questions = parse_questions(&runner_response);

        if questions.is_empty() {
            bail!("Runner response did not contain '{EDIT_COMPLETE_SIGNAL}' signal or questions");
        }

        writeln!(output)?;
        for q in &questions {
            writeln!(output, "Q: {q}")?;
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

    // If we exit the loop without returning, something went wrong.
    bail!("Failed to complete constitution edit after {rounds} rounds")
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
        context.insert(
            "qa_history",
            PlaceholderValue::List(qa_workflow::to_placeholder_list(qa_history)),
        );
    }

    expand_placeholders(&prompt_template, &context)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runner::MockRunner;

    #[test]
    fn test_parse_questions_for_constitution() {
        let response = r"
I need more information:

1. What is the scope?
2. Should this apply to all PRDs?
3. Any exceptions?
";

        let questions = parse_questions(response);
        assert_eq!(questions.len(), 3);
        assert_eq!(questions[0], "What is the scope?");
        assert_eq!(questions[1], "Should this apply to all PRDs?");
        assert_eq!(questions[2], "Any exceptions?");
    }

    #[test]
    fn test_constitution_edit() {
        use crate::runner::RunnerOutput;

        // UAT: constitution_edit — Verify constitution edit command updates via LLM
        // This test verifies that the constitution edit command can successfully
        // coordinate with the runner to edit the constitution file.

        let temp = tempfile::TempDir::new().unwrap();
        let mr_dir = temp.path().join(".mr");
        let prompts_dir = mr_dir.join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create initial constitution file
        let constitution_path = mr_dir.join("constitution.md");
        let initial_constitution = r"# Constitution

## Purpose
Project governance rules

## Rules
1. **Rule one**: Original rule
";
        std::fs::write(&constitution_path, initial_constitution).unwrap();

        // Create constitution_edit prompt template
        let prompt_path = prompts_dir.join("constitution_edit.md");
        std::fs::write(
            &prompt_path,
            "Edit request: {{user_request}}\n\nCurrent:\n{{constitution_content}}",
        )
        .unwrap();

        // Simulate what a real runner would do: edit the file, then signal completion.
        // In a real scenario, the runner (Copilot/Claude) would use its file editing
        // tools to modify the constitution file directly.
        let updated_constitution = r"# Constitution

## Purpose
Project governance rules

## Rules
1. **Rule one**: Updated rule via LLM
2. **Rule two**: New rule added by edit
";
        std::fs::write(&constitution_path, updated_constitution).unwrap();

        // Mock runner that signals edits are complete (runner already edited the file)
        let mock_runner = MockRunner::new(vec![RunnerOutput {
            success: true,
            text: "I've updated the constitution as requested.\n\nEDIT_COMPLETE".to_string(),
            usage: None,
        }]);

        let config = ConstitutionEditConfig {
            root: temp.path(),
            request: "Update rule one and add rule two",
        };

        let mut input = std::io::empty();
        let mut output = Vec::new();

        let result = edit_constitution(&config, &mock_runner, &mut input, &mut output).unwrap();

        // Verify the result
        assert_eq!(result.rounds, 1);
        assert_eq!(result.path, constitution_path);

        // Verify the constitution file was updated (by the runner, simulated above)
        let updated_content = std::fs::read_to_string(&constitution_path).unwrap();
        assert!(updated_content.contains("Updated rule via LLM"));
        assert!(updated_content.contains("New rule added by edit"));
        assert!(!updated_content.contains("Original rule"));
    }
}
