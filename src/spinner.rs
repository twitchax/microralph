//! Spinner utilities for long-running operations.
//!
//! This module provides utilities for displaying progress spinners during agent execution.
//! Spinners automatically disable when stdout is not a TTY (CI, redirected output).

use std::io::{IsTerminal, stdout};

use indicatif::{ProgressBar, ProgressStyle};

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
            bar.set_style(
                ProgressStyle::default_spinner()
                    .template("{spinner:.cyan} {msg}")
                    .expect("valid template"),
            );
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
        spinner.set_message("Another static");
        spinner.finish_and_clear();
    }

    #[test]
    fn test_spinner_with_dynamic_message() {
        let spinner = start_spinner(true, format!("Task {}/{}", 1, 5));
        spinner.set_message(format!("Task {}/{}", 2, 5));
        spinner.finish_and_clear();
    }
}
