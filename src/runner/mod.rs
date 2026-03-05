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

/// Create a runner by name with an optional model override.
///
/// Supported runners: `copilot`, `claude`, `codex`, `mock`.
pub fn create_runner(runner_name: &str, model: Option<String>) -> anyhow::Result<Box<dyn Runner>> {
    match runner_name {
        "mock" => Ok(Box::new(MockRunner::empty())),
        "copilot" => {
            let copilot = CopilotRunner::with_model(model);

            if !copilot.is_available() {
                anyhow::bail!(
                    "Copilot CLI is not available. Install it or use `--runner copilot` for testing."
                );
            }

            Ok(Box::new(copilot))
        }
        "claude" => {
            let claude = ClaudeRunner::with_model(model);

            if !claude.is_available() {
                anyhow::bail!(
                    "Claude CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Ok(Box::new(claude))
        }
        "codex" => {
            let codex = CodexRunner::with_model(model);

            if !codex.is_available() {
                anyhow::bail!(
                    "Codex CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Ok(Box::new(codex))
        }
        other => anyhow::bail!("Unknown runner: {other}. Supported: copilot, claude, codex, mock"),
    }
}
