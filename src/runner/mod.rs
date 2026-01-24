//! Runner abstraction for invoking coding agents.
//!
//! This module provides:
//! - A trait for runners that can execute prompts
//! - Mock runner for testing
//! - Copilot CLI runner (shell-based)

mod copilot;
mod mock;
mod types;

pub use copilot::CopilotRunner;
pub use mock::MockRunner;
pub use types::Runner;

// Re-export for internal crate use (tests, other modules).
// TODO(T-013): Remove allow when RunnerOutput is used in non-test code.
#[allow(unused_imports)]
pub(crate) use types::RunnerOutput;
