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

        // When enabled=false, the spinner should have no underlying progress bar.
        assert!(
            spinner.bar.is_none(),
            "Spinner bar should be None when enabled=false"
        );

        // Verify methods work without panic on disabled spinner.
        spinner.set_message("Updated message");
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_operations_do_not_panic() {
        // In test environment, stdout is typically not a TTY, so spinner will be disabled.
        let spinner = start_spinner(true, "Testing...");

        // Even with enabled=true, bar should be None in non-TTY test environment.
        assert!(
            spinner.bar.is_none(),
            "Spinner bar should be None in non-TTY environment"
        );

        // Verify methods work without panic on disabled spinner.
        spinner.set_message("Updated message");
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_with_static_message() {
        // Test that static &str messages (Cow::Borrowed) work correctly.
        let spinner = start_spinner(true, "Static message");

        // In test (non-TTY) environment, bar should be None.
        assert!(
            spinner.bar.is_none(),
            "Spinner bar should be None in non-TTY environment"
        );

        // Multiple static message updates should work without panic.
        spinner.set_message("Another static");
        spinner.set_message("Third static");
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_with_dynamic_message() {
        // Test that dynamic String messages (Cow::Owned) work correctly.
        let spinner = start_spinner(true, format!("Task {}/{}", 1, 5));

        // In test (non-TTY) environment, bar should be None.
        assert!(
            spinner.bar.is_none(),
            "Spinner bar should be None in non-TTY environment"
        );

        // Multiple dynamic message updates should work without panic.
        spinner.set_message(format!("Task {}/{}", 2, 5));
        spinner.set_message(format!("Task {}/{}", 3, 5));
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
        // This tests the idempotency contract - safe to call repeatedly.
        let spinner = start_spinner(true, "Testing...");

        // The real test: calling finish_and_clear multiple times must not panic.
        // This is important for error handling paths where cleanup may run twice.
        spinner.finish_and_clear();
        spinner.finish_and_clear();
        spinner.finish_and_clear();

        // Also verify disabled spinner (enabled=false) has same idempotency.
        let disabled_spinner = start_spinner(false, "Disabled");
        disabled_spinner.finish_and_clear();
        disabled_spinner.finish_and_clear();

        // If we reach here without panicking, the test passes.
        // The idempotency contract is verified.
    }

    #[test]
    fn test_spinner_message_updates_on_disabled_spinner() {
        // Updating message on disabled spinner should be a no-op and not panic.
        // This tests that the set_message API is safe to call regardless of enabled state.
        let spinner = start_spinner(false, "Initial");

        // The real test: calling set_message repeatedly must not panic on a disabled spinner.
        // This is important for code that doesn't check spinner state before updating.
        for i in 1..=10 {
            spinner.set_message(format!("Iteration {}", i));
        }

        // Also test with static strings (Cow::Borrowed path).
        spinner.set_message("Static message 1");
        spinner.set_message("Static message 2");

        // And empty strings (edge case).
        spinner.set_message("");

        spinner.finish_and_clear();

        // If we reach here without panicking, the no-op contract is verified.
    }

    #[test]
    fn test_spinner_run_task_simulation() {
        // Simulate the run_task workflow: create spinner, update, clear before output.
        // Verify that each spinner in the loop is correctly disabled in non-TTY environment.
        let task_count = 5;
        let mut spinners_disabled = 0;

        for task_idx in 1..=task_count {
            let spinner =
                start_spinner(true, format!("Running task {}/{}...", task_idx, task_count));

            // Verify spinner is disabled in non-TTY test environment.
            if spinner.bar.is_none() {
                spinners_disabled += 1;
            }

            // Simulate some work with message updates.
            spinner.set_message(format!("Processing task {}...", task_idx));

            // Clear before output.
            spinner.finish_and_clear();
        }

        // All spinners should be disabled in non-TTY test environment.
        assert_eq!(
            spinners_disabled, task_count,
            "All spinners should be disabled in non-TTY environment"
        );
    }

    #[test]
    fn test_spinner_refactor_iteration_simulation() {
        // Simulate the refactor workflow: one spinner per iteration.
        // Verify each spinner is disabled in non-TTY and remains so through lifecycle.
        let max_iterations = 3;

        for iteration in 1..=max_iterations {
            let spinner = start_spinner(
                true,
                format!("Refactor iteration {}/{}...", iteration, max_iterations),
            );

            // Verify spinner is disabled in non-TTY environment.
            assert!(
                spinner.bar.is_none(),
                "Spinner {} should be disabled in non-TTY",
                iteration
            );

            spinner.set_message(format!("Analyzing codebase (iteration {})...", iteration));

            // Verify spinner bar remains None after message update.
            assert!(
                spinner.bar.is_none(),
                "Spinner {} should remain disabled after message update",
                iteration
            );

            spinner.finish_and_clear();

            // Verify spinner bar remains None after finish_and_clear.
            assert!(
                spinner.bar.is_none(),
                "Spinner {} should remain disabled after finish_and_clear",
                iteration
            );
        }
    }

    #[test]
    fn test_spinner_suggest_workflow_simulation() {
        // Simulate the suggest workflow: spinner during AI generation.
        // Verify spinner is disabled in non-TTY and remains so through the workflow.
        let spinner = start_spinner(true, "Analyzing codebase...");

        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled initially in non-TTY"
        );

        // Simulate AI work with a phase transition.
        spinner.set_message("Generating suggestions...");

        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after phase transition"
        );

        // Clear before displaying suggestions.
        spinner.finish_and_clear();

        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after finish_and_clear"
        );
    }

    #[test]
    fn test_spinner_finalize_workflow_simulation() {
        // Simulate the finalize workflow: spinner during agent execution.
        // Verify spinner remains disabled through multiple phase transitions.
        let spinner = start_spinner(true, "Finalizing PRD...");

        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled initially"
        );

        // Simulate agent execution with multiple phase transitions.
        spinner.set_message("Verifying changes...");
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after 'Verifying changes'"
        );

        spinner.set_message("Committing...");
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after 'Committing'"
        );

        spinner.set_message("Done.");
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after 'Done'"
        );

        // Clear before processing output.
        spinner.finish_and_clear();

        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after finish_and_clear"
        );
    }

    #[test]
    fn test_spinner_reindex_workflow_simulation() {
        // Simulate the reindex workflow: spinner during link verification.
        // Verify spinner remains disabled through the two-phase workflow.
        let spinner = start_spinner(true, "Verifying links...");

        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled initially"
        );

        // Simulate link verification with phase transitions.
        spinner.set_message("Checking dependencies...");
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after 'Checking dependencies'"
        );

        spinner.set_message("Updating index...");
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after 'Updating index'"
        );

        // Clear before completion.
        spinner.finish_and_clear();

        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after finish_and_clear"
        );
    }

    #[test]
    fn test_disabled_spinner_set_message_empty_string() {
        // Test that empty string messages work correctly on an explicitly disabled spinner.
        // Verify spinner remains disabled with empty string edge cases.
        let spinner = start_spinner(false, "");

        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled initially"
        );

        // Empty string operations should not panic and bar should remain None.
        spinner.set_message("");
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after empty message"
        );

        spinner.set_message(""); // Multiple empty sets.
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after second empty message"
        );

        spinner.finish_and_clear();
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after finish_and_clear"
        );
    }

    #[test]
    fn test_spinner_cow_static_vs_owned() {
        // Test that both Cow::Borrowed (static &str) and Cow::Owned (String) paths
        // correctly keep the spinner in a disabled state in non-TTY environments.
        // This verifies no accidental state mutation occurs during message type transitions.
        let spinner = start_spinner(true, "Static &str");

        // Verify initial state is disabled (bar is None).
        assert!(
            spinner.bar.is_none(),
            "Spinner should be disabled after static message init"
        );

        // Set an owned String message.
        spinner.set_message(String::from("Owned String"));
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after Cow::Owned message"
        );

        // Set another static &str message.
        spinner.set_message("Another static");
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after Cow::Borrowed message"
        );

        // Set a formatted String (also Cow::Owned).
        spinner.set_message(format!("Formatted {}", "string"));
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after formatted message"
        );

        // Clear and verify final state.
        spinner.finish_and_clear();
        assert!(
            spinner.bar.is_none(),
            "Spinner should remain disabled after finish_and_clear"
        );
    }
}
