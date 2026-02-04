//! Color utilities for terminal output.
//!
//! This module provides utilities for colorizing terminal output using `owo-colors`.
//! Colors automatically degrade to plain text when output is piped or `NO_COLOR` is set.

use owo_colors::{OwoColorize, Stream};

/// Returns a string styled for success messages (green + ✅ emoji).
pub fn success(msg: &str) -> String {
    format!(
        "✅ {}",
        msg.if_supports_color(Stream::Stdout, |text| text.green())
    )
}

/// Returns a string styled for error messages (red + ❌ emoji).
pub fn error(msg: &str) -> String {
    format!(
        "❌ {}",
        msg.if_supports_color(Stream::Stdout, |text| text.red())
    )
}

/// Returns a string styled for warning messages (yellow + ⚠️ emoji).
pub fn warning(msg: &str) -> String {
    format!(
        "⚠️  {}",
        msg.if_supports_color(Stream::Stdout, |text| text.yellow())
    )
}

/// Returns a string styled for info messages (cyan).
pub fn info(msg: &str) -> String {
    msg.if_supports_color(Stream::Stdout, |text| text.cyan())
        .to_string()
}

/// Returns a string styled for question prompts (blue + bold + ❓ emoji).
pub fn question(msg: &str) -> String {
    format!(
        "❓ {}",
        format!("{}", msg.blue()).if_supports_color(Stream::Stdout, |text| text.bold())
    )
}

/// Returns a string styled for headers/sections (bold).
pub fn header(msg: &str) -> String {
    msg.if_supports_color(Stream::Stdout, |text| text.bold())
        .to_string()
}

/// Returns a string styled for dim/secondary text.
pub fn dim(msg: &str) -> String {
    msg.if_supports_color(Stream::Stdout, |text| text.dimmed())
        .to_string()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_question_has_emoji_and_styling() {
        let result = question("What is your name");

        // Should contain the question mark emoji
        assert!(result.contains("❓"), "Question should have ❓ emoji");

        // Should contain the message
        assert!(
            result.contains("What is your name"),
            "Question should contain the message"
        );

        // When TTY is not detected (testing environment), it should still have emoji
        // but may not have ANSI codes. Let's verify the emoji is at the start.
        assert!(
            result.starts_with("❓"),
            "Question should start with ❓ emoji"
        );
    }

    #[test]
    fn test_success_has_correct_emoji_prefix() {
        let result = success("Task completed");

        // Success should start with checkmark emoji.
        assert!(
            result.starts_with("✅"),
            "Success output should start with ✅ emoji"
        );

        // Message should follow the emoji.
        assert!(
            result.contains("Task completed"),
            "Success output should contain the message"
        );
    }

    #[test]
    fn test_error_has_correct_emoji_prefix() {
        let result = error("Something failed");

        // Error should start with X emoji.
        assert!(
            result.starts_with("❌"),
            "Error output should start with ❌ emoji"
        );

        // Message should follow the emoji.
        assert!(
            result.contains("Something failed"),
            "Error output should contain the message"
        );
    }

    #[test]
    fn test_warning_has_correct_emoji_prefix() {
        let result = warning("Heads up");

        // Warning should start with warning emoji.
        assert!(
            result.starts_with("⚠️"),
            "Warning output should start with ⚠️ emoji"
        );

        // Message should follow the emoji (accounting for spacing).
        assert!(
            result.contains("Heads up"),
            "Warning output should contain the message"
        );
    }

    #[test]
    fn test_styling_functions_handle_empty_string() {
        // All styling functions should not panic on empty input.
        // In non-TTY test environment, output should be predictable.
        assert!(success("").starts_with("✅"));
        assert!(error("").starts_with("❌"));
        assert!(warning("").starts_with("⚠️"));
        assert_eq!(info(""), "");
        assert_eq!(header(""), "");
        assert_eq!(dim(""), "");
    }

    #[test]
    fn test_info_header_dim_are_identity_in_non_tty() {
        // In test environment (non-TTY), info/header/dim should return input unchanged.
        // This tests the degradation behavior when colors are not supported.
        let msg = "Plain text message";

        assert_eq!(info(msg), msg, "info should return plain text in non-TTY");
        assert_eq!(
            header(msg),
            msg,
            "header should return plain text in non-TTY"
        );
        assert_eq!(dim(msg), msg, "dim should return plain text in non-TTY");
    }
}
