//! Shared Q/A workflow utilities for PRD operations.
//!
//! This module contains common types and functions used across `prd_new`, `prd_edit`,
//! and other commands that follow a question/answer workflow with AI runners.

use std::collections::HashMap;
use std::io::{BufRead, Write};

use anyhow::Result;

use crate::runner::CopilotRunner;

/// A question/answer pair from the multi-round workflow.
#[derive(Debug, Clone)]
pub struct QaPair {
    pub question: String,
    pub answer: String,
}

/// Converts a slice of Q/A pairs to a list format suitable for placeholder expansion.
///
/// This is the standard format used by prompt templates that reference `{{qa_history}}`.
pub fn to_placeholder_list(qa_history: &[QaPair]) -> Vec<HashMap<String, String>> {
    qa_history
        .iter()
        .map(|qa| {
            [
                ("question".to_string(), qa.question.clone()),
                ("answer".to_string(), qa.answer.clone()),
            ]
            .into_iter()
            .collect()
        })
        .collect()
}

/// Extracts PRD content from runner output.
///
/// Handles various output formats:
/// - Direct frontmatter (starts with `---`)
/// - Markdown/YAML code blocks
/// - Content after READY_TO_APPLY signal
///
/// This is the robust implementation from `prd_new.rs` that handles:
/// - ANSI escape sequences
/// - Usage statistics stripping
/// - Multiple code fence formats
/// - Nested code blocks
pub fn extract_prd_content(output: &str) -> Result<String> {
    // First, strip any ANSI escape sequences that might be in the output.
    let cleaned = strip_ansi_escapes(output);

    // Strip usage statistics that shouldn't be in PRD content
    let cleaned = CopilotRunner::strip_usage_stats(&cleaned);

    // Look for READY_TO_APPLY signal and start from there if present
    let content_start = if let Some(idx) = cleaned.find("READY_TO_APPLY") {
        &cleaned[idx + "READY_TO_APPLY".len()..]
    } else {
        cleaned.as_str()
    };

    let trimmed = content_start.trim();

    tracing::debug!(
        output_len = output.len(),
        cleaned_len = trimmed.len(),
        first_50_chars = ?trimmed.chars().take(50).collect::<String>(),
        "Extracting PRD content from runner output"
    );

    // If output starts directly with frontmatter, use it as-is.
    if trimmed.starts_with("---") {
        tracing::debug!("Output starts with frontmatter directly");
        return Ok(trimmed.to_string());
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
                    tracing::debug!(
                        fence_pattern,
                        "Extracted from code block with newline+fence"
                    );
                    return Ok(content_trimmed.to_string());
                }
            }

            // Fallback: try to find closing fence, even if not at line start.
            if let Some(end) = remaining.rfind("```") {
                let content = &remaining[..end];
                let content_trimmed = content.trim();

                if content_trimmed.starts_with("---") {
                    tracing::debug!(fence_pattern, "Extracted from code block with inline fence");
                    return Ok(content_trimmed.to_string());
                }
            }
        }
    }

    // Last resort: look for --- delimiters directly in the output.
    // Find the first --- that starts at the beginning of a line.
    for (i, line) in trimmed.lines().enumerate() {
        if line.trim() == "---" {
            // Found a frontmatter delimiter, extract from here.
            let lines: Vec<&str> = trimmed.lines().skip(i).collect();
            let result = lines.join("\n");
            tracing::debug!(line_number = i, "Found frontmatter delimiter on line");
            return Ok(result);
        }
    }

    // Really last resort: find --- anywhere.
    if let Some(fm_start) = trimmed.find("---") {
        tracing::debug!(position = fm_start, "Found --- at position (fallback)");
        return Ok(trimmed[fm_start..].trim().to_string());
    }

    // No recognizable PRD content found - this is an error for PRD extraction
    anyhow::bail!("Could not extract PRD content from runner output")
}

/// Strips ANSI escape sequences from a string.
///
/// Removes control sequences like color codes that might appear in runner output.
pub fn strip_ansi_escapes(s: &str) -> String {
    // Simple regex-like removal of ANSI escape sequences.
    // Matches: ESC [ ... (letter) sequences.
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // ESC character - start of escape sequence.
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['

                // Skip until we hit a letter (the terminator).
                while let Some(&next) = chars.peek() {
                    chars.next();

                    if next.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                // Some other escape, skip next char.
                chars.next();
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Parses numbered questions from runner output.
///
/// Supports multi-line questions separated by blank lines.
/// Questions are identified by:
/// - Starting with a digit followed by `.` or `)`
/// - Ending with a `?` (for single-line) or blank line (for multi-line)
///
/// Returns empty vector if READY_TO_APPLY signal is present.
pub fn parse_questions(output: &str) -> Vec<String> {
    let mut questions = Vec::new();
    let mut current_question = String::new();
    let mut in_question = false;

    // Don't parse questions if ready signal is present
    if output.contains("READY_TO_APPLY") {
        return questions;
    }

    for line in output.lines() {
        let trimmed = line.trim();

        // Check if this line starts a new numbered question (1., 2., etc.).
        let is_question_start = trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_digit())
            .unwrap_or(false)
            && (trimmed.contains(". ") || trimmed.contains(") "));

        if is_question_start {
            // Save previous question if any.
            if in_question && !current_question.trim().is_empty() {
                questions.push(current_question.trim().to_string());
            }

            // Start new question.
            if let Some(rest) = trimmed
                .strip_prefix(|c: char| c.is_ascii_digit())
                .and_then(|s| s.strip_prefix('.'))
                .or_else(|| {
                    trimmed
                        .strip_prefix(|c: char| c.is_ascii_digit())
                        .and_then(|s| s.strip_prefix(')'))
                })
            {
                current_question = rest.trim().to_string();
                in_question = true;
            }
        } else if in_question {
            // Continue previous question if line is non-empty.
            if !trimmed.is_empty() {
                current_question.push('\n');
                current_question.push_str(trimmed);
            } else if !current_question.trim().is_empty() {
                // Empty line after content signals end of question.
                questions.push(current_question.trim().to_string());
                current_question.clear();
                in_question = false;
            }
        }
    }

    // Save last question if any.
    if in_question && !current_question.trim().is_empty() {
        questions.push(current_question.trim().to_string());
    }

    questions
}

/// Collects multi-line answers from the user for each question.
///
/// Displays each question with proper formatting and prompts the user to provide
/// an answer. Users can enter multiple lines by pressing Enter to continue,
/// and press Enter twice (blank line) to finish the answer.
pub fn collect_multiline_answers<I, O>(
    questions: &[String],
    input: &mut I,
    output: &mut O,
) -> Result<Vec<QaPair>>
where
    I: BufRead,
    O: Write,
{
    let mut pairs = Vec::new();

    for (i, question) in questions.iter().enumerate() {
        // Display question with proper multi-line formatting.
        let question_lines: Vec<&str> = question.lines().collect();
        if question_lines.len() == 1 {
            writeln!(output, "{}. {}", i + 1, crate::colors::question(question))?;
        } else {
            // First line with number.
            writeln!(
                output,
                "{}. {}",
                i + 1,
                crate::colors::question(question_lines[0])
            )?;
            // Subsequent lines indented.
            for line in &question_lines[1..] {
                writeln!(output, "   {}", crate::colors::question(line))?;
            }
        }
        write!(output, "   > ")?;
        output.flush()?;

        // Read multi-line answer; user presses Enter twice to finish.
        let mut answer_lines = Vec::new();
        loop {
            let mut line = String::new();
            let bytes_read = input.read_line(&mut line)?;

            // Check for EOF (no more input).
            if bytes_read == 0 {
                break;
            }

            // Check if the line is empty (just newline = double-enter).
            if line.trim().is_empty() {
                // If we already have content, this is the terminating blank line.
                if !answer_lines.is_empty() {
                    break;
                }
                // Otherwise, it's a leading blank line; skip it.
            } else {
                answer_lines.push(line.trim_end().to_string());
                // Prompt for next line if user continues.
                write!(output, "   > ")?;
                output.flush()?;
            }
        }

        let answer = answer_lines.join("\n");

        pairs.push(QaPair {
            question: question.clone(),
            answer: answer.trim().to_string(),
        });
    }

    Ok(pairs)
}

/// Collects single-line answers from the user for each question.
///
/// Simpler version for commands that don't need multi-line input.
pub fn collect_singleline_answers<I, O>(
    questions: &[String],
    input: &mut I,
    output: &mut O,
) -> Result<Vec<QaPair>>
where
    I: BufRead,
    O: Write,
{
    let mut pairs = Vec::new();

    for (i, question) in questions.iter().enumerate() {
        writeln!(output, "{}. {}", i + 1, crate::colors::question(question))?;
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_escapes() {
        let input = "\x1b[31mRed text\x1b[0m Normal text";
        let output = strip_ansi_escapes(input);
        assert_eq!(output, "Red text Normal text");
    }

    #[test]
    fn test_parse_questions_single_line() {
        let output = "Here are some questions:\n\n1. What is your name?\n2. What is your quest?";
        let questions = parse_questions(output);
        assert_eq!(questions.len(), 2);
        assert_eq!(questions[0], "What is your name?");
        assert_eq!(questions[1], "What is your quest?");
    }

    #[test]
    fn test_parse_questions_multi_line() {
        let output = r#"I need clarification:

1. What is the task priority?
This will help determine scheduling.

2. Should this task block others?
We need to know dependencies.
"#;
        let questions = parse_questions(output);
        assert_eq!(questions.len(), 2);
        assert!(questions[0].contains("priority"));
        assert!(questions[0].contains("scheduling"));
        assert!(questions[1].contains("block"));
        assert!(questions[1].contains("dependencies"));
    }

    #[test]
    fn test_parse_questions_with_ready_signal() {
        let output = "READY_TO_APPLY\n\n1. This is not a question?";
        let questions = parse_questions(output);
        assert!(questions.is_empty());
    }

    #[test]
    fn test_extract_prd_content_direct() {
        let output = "---\nid: PRD-0001\ntitle: Test\n---\n\n# Summary";
        let content = extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
        assert!(content.contains("PRD-0001"));
    }

    #[test]
    fn test_extract_prd_content_markdown_block() {
        let output = "Here's the PRD:\n\n```markdown\n---\nid: PRD-0001\n---\n# Summary\n```";
        let content = extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
        assert!(content.contains("PRD-0001"));
    }

    #[test]
    fn test_extract_prd_content_with_ready_signal() {
        let output = "READY_TO_APPLY\n\n```markdown\n---\nid: PRD-0001\n---\n```";
        let content = extract_prd_content(output).unwrap();
        assert!(content.starts_with("---"));
    }

    #[test]
    fn test_collect_singleline_answers() {
        let questions = vec!["Question 1?".to_string(), "Question 2?".to_string()];
        let input = "Answer 1\nAnswer 2\n";
        let mut input = input.as_bytes();
        let mut output = Vec::new();

        let pairs = collect_singleline_answers(&questions, &mut input, &mut output).unwrap();

        assert_eq!(pairs.len(), 2);
        assert_eq!(pairs[0].question, "Question 1?");
        assert_eq!(pairs[0].answer, "Answer 1");
        assert_eq!(pairs[1].question, "Question 2?");
        assert_eq!(pairs[1].answer, "Answer 2");
    }
}
