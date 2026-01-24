//! Color utilities for terminal output.
//!
//! This module provides utilities for colorizing terminal output using owo-colors.
//! Colors automatically degrade to plain text when output is piped or NO_COLOR is set.

#![allow(dead_code)]

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
