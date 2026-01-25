//! Runner abstraction for invoking coding agents.
//!
//! This module provides:
//! - A trait for runners that can execute prompts
//! - Mock runner for testing
//! - Copilot CLI runner (shell-based)
//! - Claude CLI runner (shell-based)

mod claude;
mod copilot;
mod mock;
mod types;

pub use claude::ClaudeRunner;
pub use copilot::CopilotRunner;
pub use mock::MockRunner;
pub use types::Runner;

// Re-export for internal crate use (tests, other modules).
pub(crate) use types::{RunnerOutput, UsageInfo};
