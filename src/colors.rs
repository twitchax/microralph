//! Color utilities for terminal output.
//!
//! This module provides utilities for colorizing terminal output using owo-colors.
//! Colors automatically degrade to plain text when output is piped or NO_COLOR is set.

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
    fn test_finalization_summary_box_styling() {
        // Test that the finalization summary box components render with appropriate styling
        let separator = info("═══════════════════════════════════════════════════════════════");
        let title = header("FINALIZATION SUMMARY");
        let detail = dim("PRD Path: /some/path");

        // Verify separator contains the expected characters
        assert!(
            separator.contains("═══"),
            "Separator should contain box drawing characters"
        );

        // Verify header contains the title text
        assert!(
            title.contains("FINALIZATION SUMMARY"),
            "Header should contain title text"
        );

        // Verify dim detail contains the path
        assert!(
            detail.contains("PRD Path:"),
            "Detail should contain the label"
        );
        assert!(
            detail.contains("/some/path"),
            "Detail should contain the path"
        );
    }
}
