//! Spinner utilities for long-running operations.
//!
//! This module provides utilities for displaying progress spinners during agent execution.
//! Spinners automatically disable when stdout is not a TTY (CI, redirected output).

use std::io::{IsTerminal, stdout};
use std::sync::LazyLock;

use indicatif::{ProgressBar, ProgressStyle};

/// Pre-compiled spinner style for reuse across spinner instances.
static SPINNER_STYLE: LazyLock<ProgressStyle> = LazyLock::new(|| {
    ProgressStyle::default_spinner()
        .template("{spinner:.cyan} {msg}")
        .expect("spinner template is valid")
});

/// A wrapper around indicatif's ProgressBar that handles TTY detection.
///
/// When stdout is not a TTY, all operations become no-ops.
#[allow(dead_code)] // Will be used by T-003+
pub struct Spinner {
    bar: Option<ProgressBar>,
}

#[allow(dead_code)] // Will be used by T-003+
impl Spinner {
    /// Creates a new spinner if stdout is a TTY, otherwise returns a no-op spinner.
    fn new_internal(enabled: bool) -> Self {
        if enabled && stdout().is_terminal() {
            let bar = ProgressBar::new_spinner();
            bar.set_style(SPINNER_STYLE.clone());
            bar.enable_steady_tick(std::time::Duration::from_millis(80));
            Self { bar: Some(bar) }
        } else {
            Self { bar: None }
        }
    }

    /// Updates the spinner message.
    pub fn set_message(&self, msg: impl Into<std::borrow::Cow<'static, str>>) {
        if let Some(bar) = &self.bar {
            bar.set_message(msg);
        }
    }

    /// Clears the spinner from the terminal without leaving any residue.
    pub fn finish_and_clear(&self) {
        if let Some(bar) = &self.bar {
            bar.finish_and_clear();
        }
    }
}

/// Starts a new spinner with the given message.
///
/// The spinner will only be displayed if:
/// - `enabled` is true (typically when `--stream` is false)
/// - stdout is a TTY
///
/// # Arguments
///
/// * `enabled` - Whether spinners are enabled (typically `!stream`)
/// * `message` - The initial message to display
///
/// # Returns
///
/// A `Spinner` handle that can be used to update or clear the spinner.
#[allow(dead_code)] // Will be used by T-003+
pub fn start_spinner(enabled: bool, message: impl Into<std::borrow::Cow<'static, str>>) -> Spinner {
    let spinner = Spinner::new_internal(enabled);
    spinner.set_message(message);
    spinner
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_disabled_when_not_enabled() {
        let spinner = start_spinner(false, "Testing...");
        // Should not panic when calling methods on disabled spinner
        spinner.set_message("Updated message");
        spinner.finish_and_clear();
        // No assertions needed - just verifying no panics
    }

    #[test]
    fn test_spinner_operations_do_not_panic() {
        // In test environment, stdout is typically not a TTY, so spinner will be disabled
        let spinner = start_spinner(true, "Testing...");
        spinner.set_message("Updated message");
        spinner.finish_and_clear();
        // No assertions needed - just verifying no panics
    }

    #[test]
    fn test_spinner_with_static_message() {
        let spinner = start_spinner(true, "Static message");

        // In non-TTY (test) environment, bar should be None.
        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled in non-TTY environment"
        );

        spinner.set_message("Another static");
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_with_dynamic_message() {
        let spinner = start_spinner(true, format!("Task {}/{}", 1, 5));

        // Verify spinner is correctly disabled in test environment.
        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled in non-TTY environment"
        );

        spinner.set_message(format!("Task {}/{}", 2, 5));
        spinner.finish_and_clear();
    }

    // ========================================
    // Integration tests for spinner behavior
    // ========================================

    #[test]
    fn test_spinner_disabled_in_non_tty_environment() {
        // In test/CI environments, stdout is not a TTY.
        // Verify spinner becomes a no-op and bar is None.
        let spinner = Spinner::new_internal(true);

        // In non-TTY (test) environment, bar should be None.
        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled in non-TTY environment"
        );
    }

    #[test]
    fn test_spinner_explicitly_disabled_has_no_bar() {
        // When enabled=false, bar should always be None regardless of TTY.
        let spinner = Spinner::new_internal(false);

        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled when enabled=false"
        );
    }

    #[test]
    fn test_start_spinner_returns_disabled_in_non_tty() {
        // Verify start_spinner returns a spinner with no bar in non-TTY.
        let spinner = start_spinner(true, "Test message");

        assert!(
            spinner.bar.is_none(),
            "start_spinner should return disabled spinner in non-TTY"
        );
    }

    #[test]
    fn test_spinner_clear_is_idempotent() {
        // Calling finish_and_clear multiple times should not panic.
        let spinner = start_spinner(true, "Testing...");
        spinner.finish_and_clear();
        spinner.finish_and_clear();
        spinner.finish_and_clear();
        // No panic = success
    }

    #[test]
    fn test_spinner_message_updates_on_disabled_spinner() {
        // Updating message on disabled spinner should be a no-op without panic.
        let spinner = start_spinner(false, "Initial");
        for i in 1..=10 {
            spinner.set_message(format!("Iteration {}", i));
        }
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_run_task_simulation() {
        // Simulate the run_task workflow: create spinner, update, clear before output.
        let task_count = 5;
        for task_idx in 1..=task_count {
            let spinner =
                start_spinner(true, format!("Running task {}/{}...", task_idx, task_count));

            // Verify each spinner is correctly disabled in test/CI environment.
            assert!(
                spinner.bar.is_none(),
                "Spinner {} should be disabled in non-TTY environment",
                task_idx
            );

            // Simulate some work...
            spinner.set_message(format!("Processing task {}...", task_idx));

            // Clear before output.
            spinner.finish_and_clear();
        }
    }

    #[test]
    fn test_spinner_refactor_iteration_simulation() {
        // Simulate the refactor workflow: one spinner per iteration.
        let max_iterations = 3;
        for iteration in 1..=max_iterations {
            let spinner = start_spinner(
                true,
                format!("Refactor iteration {}/{}...", iteration, max_iterations),
            );

            // Verify each iteration's spinner is disabled in test environment.
            assert!(
                spinner.bar.is_none(),
                "Iteration {} spinner should be disabled in non-TTY",
                iteration
            );

            spinner.set_message(format!("Analyzing codebase (iteration {})...", iteration));

            // Clear before showing output.
            spinner.finish_and_clear();
        }
    }

    #[test]
    fn test_spinner_suggest_workflow_simulation() {
        // Simulate the suggest workflow: spinner during AI generation.
        let spinner = start_spinner(true, "Analyzing codebase...");

        // Verify spinner is disabled in test environment.
        assert!(
            spinner.bar.is_none(),
            "Suggest workflow spinner should be disabled in non-TTY"
        );

        // Simulate AI work...
        spinner.set_message("Generating suggestions...");

        // Clear before displaying suggestions.
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_finalize_workflow_simulation() {
        // Simulate the finalize workflow: spinner during agent execution.
        let spinner = start_spinner(true, "Finalizing PRD...");

        // Verify spinner is disabled in test environment.
        assert!(
            spinner.bar.is_none(),
            "Finalize workflow spinner should be disabled in non-TTY"
        );

        // Simulate agent execution...
        spinner.set_message("Verifying changes...");

        // Clear before processing output.
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_reindex_workflow_simulation() {
        // Simulate the reindex workflow: spinner during link verification.
        let spinner = start_spinner(true, "Verifying links...");

        // Verify spinner is disabled in test environment.
        assert!(
            spinner.bar.is_none(),
            "Reindex workflow spinner should be disabled in non-TTY"
        );

        // Simulate link verification...
        spinner.set_message("Updating index...");

        // Clear before completion.
        spinner.finish_and_clear();
    }

    #[test]
    fn test_disabled_spinner_set_message_empty_string() {
        let spinner = start_spinner(false, "");

        // Explicitly disabled spinner should have no bar.
        assert!(
            spinner.bar.is_none(),
            "Explicitly disabled spinner should have no bar"
        );

        spinner.set_message("");
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_cow_static_vs_owned() {
        // Test both Cow::Borrowed (static) and Cow::Owned (String) paths.
        let spinner = start_spinner(true, "Static &str");
        spinner.set_message(String::from("Owned String"));
        spinner.set_message("Another static");
        spinner.set_message(format!("Formatted {}", "string"));
        spinner.finish_and_clear();
    }
}
