//! Runner abstraction for invoking coding agents.
//!
//! This module provides:
//! - A trait for runners that can execute prompts
//! - Mock runner for testing
//! - Copilot CLI runner (shell-based)

mod mock;
mod types;

pub use mock::MockRunner;
#[allow(unused_imports)]
pub use types::{Runner, RunnerOutput};
