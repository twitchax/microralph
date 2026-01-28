//! `mr refactor` command implementation.
//!
//! Runs an iterative loop where the AI agent identifies and applies
//! one impactful refactor per iteration, verifies UATs, and commits.

use std::path::Path;

use anyhow::Result;

use crate::config::load_constitution;
use crate::prompt::{
    PlaceholderContext, PromptKind, expand_placeholders, load_prompt_with_fallback,
};
use crate::runner::{Runner, RunnerOutput, UsageInfo};
use crate::spinner::start_spinner;

/// Configuration for `mr refactor`.
#[derive(Debug)]
pub struct RefactorConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// Maximum number of refactor iterations.
    pub max_iterations: u32,

    /// Optional focus hint from the user (e.g., "improve error handling").
    pub context: Option<&'a str>,

    /// Optional path constraint (e.g., "src/").
    pub path: Option<&'a str>,

    /// If true, suggest refactors without applying them.
    pub dry_run: bool,

    /// If true, do not instruct agent to commit.
    pub no_commit: bool,

    /// Whether to stream runner output in real-time.
    pub stream: bool,
}

/// Result from a single refactor iteration.
#[derive(Debug)]
pub enum RefactorIterationResult {
    /// A refactor was applied successfully.
    Applied {
        /// Summary of what was changed.
        summary: String,

        /// Optional usage information from the underlying agent.
        usage: Option<UsageInfo>,
    },

    /// Dry-run mode: a refactor was suggested but not applied.
    Suggested {
        /// The suggested refactor.
        suggestion: String,

        /// Optional usage information.
        usage: Option<UsageInfo>,
    },

    /// Agent signaled no more impactful refactors remain.
    NoMoreRefactors {
        /// Optional usage information.
        usage: Option<UsageInfo>,
    },

    /// The iteration failed (UAT failure or other error).
    Failed {
        /// Error message.
        error: String,

        /// Optional usage information.
        usage: Option<UsageInfo>,
    },
}

/// Result from the entire refactor loop.
#[derive(Debug)]
pub struct RefactorLoopResult {
    /// Number of iterations performed.
    pub iterations: usize,

    /// Number of refactors successfully applied.
    pub applied_count: usize,

    /// Number of dry-run suggestions (in dry-run mode).
    pub suggested_count: usize,

    /// Whether the loop terminated early (agent signaled no more refactors).
    pub early_termination: bool,

    /// Total token usage across all iterations.
    pub total_usage: Option<UsageInfo>,
}

/// Builds the prompt for a refactor iteration.
fn build_refactor_prompt(root: &Path, config: &RefactorConfig, iteration: u32) -> String {
    let prompt_template = load_prompt_with_fallback(root, PromptKind::Refactor);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("iteration", iteration.to_string());
    ctx.insert("max_iterations", config.max_iterations.to_string());

    if let Some(focus) = config.context {
        ctx.insert("context", focus);
    }

    if let Some(path) = config.path {
        ctx.insert("path", path);
    }

    ctx.insert("preview", config.dry_run);
    ctx.insert("commit", !config.no_commit);
    ctx.insert("no_commit", config.no_commit);

    // Load constitution if available.
    if let Ok(Some(constitution)) = load_constitution(root) {
        ctx.insert("constitution", constitution);
    }

    expand_placeholders(&prompt_template, &ctx)
}

/// Checks if the runner response signals no more refactors.
fn is_no_more_refactors(text: &str) -> bool {
    text.lines().any(|line| line.trim() == "NO-MORE-REFACTORS")
}

/// Checks if the runner response signals preview completion.
fn is_preview_complete(text: &str) -> bool {
    text.lines().any(|line| line.trim() == "PREVIEW-COMPLETE")
}

/// Runs a single refactor iteration.
fn run_iteration(
    config: &RefactorConfig,
    runner: &dyn Runner,
    iteration: u32,
) -> Result<RefactorIterationResult> {
    let prompt = build_refactor_prompt(config.root, config, iteration);

    tracing::info!(
        iteration = iteration,
        max = config.max_iterations,
        dry_run = config.dry_run,
        "Running refactor iteration"
    );

    // Start spinner when not streaming (streaming already provides visual feedback).
    // Print command info before spinner (only when not streaming).
    if !config.stream
        && let Some(cmd_display) = runner.format_command_display(&prompt, config.root)
    {
        println!("\n🔧 Executing: {}", cmd_display);
    }

    let spinner = start_spinner(
        !config.stream,
        format!(
            "Refactor iteration {}/{}...",
            iteration, config.max_iterations
        ),
    );

    let output: RunnerOutput = if config.stream {
        let mut stdout = std::io::stdout();
        runner.execute_streaming(&prompt, config.root, &mut stdout)?
    } else {
        runner.execute(&prompt, config.root)?
    };

    // Clear spinner before processing output.
    spinner.finish_and_clear();

    let text = output.text.trim();

    // Check for early termination signal.
    if is_no_more_refactors(text) {
        tracing::info!("Agent signaled no more impactful refactors");
        return Ok(RefactorIterationResult::NoMoreRefactors {
            usage: output.usage,
        });
    }

    // Check for preview completion.
    if config.dry_run && is_preview_complete(text) {
        return Ok(RefactorIterationResult::Suggested {
            suggestion: text.to_string(),
            usage: output.usage,
        });
    }

    // Check if the runner reported success.
    if output.success {
        Ok(RefactorIterationResult::Applied {
            summary: if text.len() > 500 {
                format!("{}...", &text[..500])
            } else {
                text.to_string()
            },
            usage: output.usage,
        })
    } else {
        Ok(RefactorIterationResult::Failed {
            error: text.to_string(),
            usage: output.usage,
        })
    }
}

/// Adds two optional u64 values, returning None only if both are None.
fn add_optional(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (Some(x), Some(y)) => Some(x + y),
        (Some(x), None) | (None, Some(x)) => Some(x),
        (None, None) => None,
    }
}

/// Aggregates usage info from multiple iterations.
fn aggregate_usage(total: &mut Option<UsageInfo>, new: &Option<UsageInfo>) {
    if let Some(new_usage) = new {
        if let Some(total_usage) = total {
            total_usage.input_tokens =
                add_optional(total_usage.input_tokens, new_usage.input_tokens);
            total_usage.output_tokens =
                add_optional(total_usage.output_tokens, new_usage.output_tokens);
            total_usage.total_tokens =
                add_optional(total_usage.total_tokens, new_usage.total_tokens);
        } else {
            *total = Some(new_usage.clone());
        }
    }
}

/// Runs the refactor loop.
///
/// Iterates up to `max_iterations` times, invoking the runner each time.
/// Stops early if the agent signals no more refactors.
///
/// # Arguments
///
/// * `config` - Configuration for the refactor loop
/// * `runner` - The runner to use for refactoring
///
/// # Returns
///
/// A `RefactorLoopResult` summarizing what happened.
pub fn refactor(config: &RefactorConfig, runner: &dyn Runner) -> Result<RefactorLoopResult> {
    let mut result = RefactorLoopResult {
        iterations: 0,
        applied_count: 0,
        suggested_count: 0,
        early_termination: false,
        total_usage: None,
    };

    for iteration in 1..=config.max_iterations {
        result.iterations += 1;

        let iter_result = run_iteration(config, runner, iteration)?;

        match iter_result {
            RefactorIterationResult::Applied { summary, usage } => {
                tracing::info!(iteration, summary = %summary, "Refactor applied");
                result.applied_count += 1;
                aggregate_usage(&mut result.total_usage, &usage);
            }

            RefactorIterationResult::Suggested { suggestion, usage } => {
                tracing::info!(iteration, "Dry-run suggestion generated");
                tracing::debug!(suggestion = %suggestion, "Suggestion details");
                result.suggested_count += 1;
                aggregate_usage(&mut result.total_usage, &usage);
            }

            RefactorIterationResult::NoMoreRefactors { usage } => {
                tracing::info!(iteration, "Early termination: no more refactors");
                result.early_termination = true;
                aggregate_usage(&mut result.total_usage, &usage);
                break;
            }

            RefactorIterationResult::Failed { error, usage } => {
                tracing::warn!(iteration, error = %error, "Refactor iteration failed");
                aggregate_usage(&mut result.total_usage, &usage);
                // Continue to next iteration per PRD: "leave UAT failure handling to agent's discretion"
            }
        }
    }

    Ok(result)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_is_no_more_refactors() {
        assert!(is_no_more_refactors("NO-MORE-REFACTORS"));
        assert!(is_no_more_refactors("Some output\nNO-MORE-REFACTORS\nMore"));
        assert!(is_no_more_refactors("  NO-MORE-REFACTORS  "));
        assert!(!is_no_more_refactors("NO-MORE-REFACTORS-NOT"));
        assert!(!is_no_more_refactors("Some random text"));
    }

    #[test]
    fn test_is_preview_complete() {
        assert!(is_preview_complete("PREVIEW-COMPLETE"));
        assert!(is_preview_complete("Suggestion...\nPREVIEW-COMPLETE"));
        assert!(is_preview_complete("  PREVIEW-COMPLETE  "));
        assert!(!is_preview_complete("PREVIEW-COMPLETED"));
        assert!(!is_preview_complete("Some random text"));
    }

    #[test]
    fn test_build_refactor_prompt_basic() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();

        // Create minimal .mr structure.
        std::fs::create_dir_all(root.join(".mr/prompts")).unwrap();

        let config = RefactorConfig {
            root,
            max_iterations: 3,
            context: None,
            path: None,
            dry_run: false,
            no_commit: false,
            stream: false,
        };

        let prompt = build_refactor_prompt(root, &config, 1);

        // Should contain iteration info.
        assert!(prompt.contains("iteration"));
    }

    #[test]
    fn test_build_refactor_prompt_with_context() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();

        std::fs::create_dir_all(root.join(".mr/prompts")).unwrap();

        let config = RefactorConfig {
            root,
            max_iterations: 5,
            context: Some("improve error handling"),
            path: Some("src/"),
            dry_run: true,
            no_commit: true,
            stream: false,
        };

        let prompt = build_refactor_prompt(root, &config, 2);

        // Prompt should be generated (uses fallback template).
        assert!(!prompt.is_empty());
    }

    #[test]
    fn test_build_refactor_prompt_no_commit_true() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();

        std::fs::create_dir_all(root.join(".mr/prompts")).unwrap();

        // Test with no_commit=true (should include "No-Commit Mode" instructions).
        let config = RefactorConfig {
            root,
            max_iterations: 3,
            context: None,
            path: None,
            dry_run: false,
            no_commit: true,
            stream: false,
        };

        let prompt = build_refactor_prompt(root, &config, 1);

        // When no_commit=true, prompt should include "No-Commit Mode" section.
        assert!(
            prompt.contains("No-Commit Mode"),
            "Prompt should contain 'No-Commit Mode' when no_commit=true. Got:\n{}",
            prompt
        );
        assert!(
            prompt.contains("Do NOT commit"),
            "Prompt should contain 'Do NOT commit' when no_commit=true"
        );
    }

    #[test]
    fn test_build_refactor_prompt_no_commit_false() {
        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();

        std::fs::create_dir_all(root.join(".mr/prompts")).unwrap();

        // Test with no_commit=false (should include commit instructions).
        let config = RefactorConfig {
            root,
            max_iterations: 3,
            context: None,
            path: None,
            dry_run: false,
            no_commit: false,
            stream: false,
        };

        let prompt = build_refactor_prompt(root, &config, 1);

        // When no_commit=false, prompt should NOT include "No-Commit Mode".
        assert!(
            !prompt.contains("No-Commit Mode"),
            "Prompt should NOT contain 'No-Commit Mode' when no_commit=false. Got:\n{}",
            prompt
        );
        assert!(
            !prompt.contains("Do NOT commit changes"),
            "Prompt should NOT contain 'Do NOT commit changes' when no_commit=false"
        );
    }

    #[test]
    fn test_refactor_loop_early_termination() {
        use crate::runner::MockRunner;

        let temp = tempfile::TempDir::new().unwrap();
        let root = temp.path();

        // Create minimal .mr structure.
        std::fs::create_dir_all(root.join(".mr/prompts")).unwrap();

        let config = RefactorConfig {
            root,
            max_iterations: 5, // Set high so we can verify early termination
            context: None,
            path: None,
            dry_run: false,
            no_commit: false,
            stream: false,
        };

        // Mock runner: first iteration applies a refactor, second signals no more.
        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Applied fix for DRY violation"),
            crate::runner::RunnerOutput::success("NO-MORE-REFACTORS"),
        ]);

        let result = refactor(&config, &runner).unwrap();

        // Should stop after 2 iterations, not 5.
        assert_eq!(result.iterations, 2);
        assert_eq!(result.applied_count, 1);
        assert!(result.early_termination);

        // Verify runner was only called twice.
        assert_eq!(runner.recorded_prompts().len(), 2);
    }
}
