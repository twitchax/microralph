//! Runner abstraction for invoking coding agents.
//!
//! This module provides:
//! - A trait for runners that can execute prompts
//! - Mock runner for testing
//! - Copilot CLI runner (shell-based)
//! - Claude CLI runner (shell-based)
//! - Shared CLI runner infrastructure

mod claude;
mod cli_runner;
mod codex;
mod copilot;
mod mock;
mod types;

pub use claude::ClaudeRunner;
pub use codex::CodexRunner;
pub use copilot::CopilotRunner;
pub use mock::MockRunner;
pub use types::Runner;

// Re-export for internal crate use (tests, other modules).
pub(crate) use types::{RunnerOutput, TokenUsageInfo};

// Re-export RunnerError for test code in other modules.
#[cfg(test)]
pub(crate) use types::RunnerError;
