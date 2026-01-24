//! Runner type definitions.

use std::fmt;

/// Error type for runner operations.
#[derive(Debug)]
pub enum RunnerError {
    /// The runner process failed to start.
    ProcessFailed(String),

    /// IO error.
    Io(std::io::Error),
}

impl std::error::Error for RunnerError {}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProcessFailed(msg) => write!(f, "Process failed to start: {msg}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl From<std::io::Error> for RunnerError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

/// Result type for runner operations.
pub type RunnerResult<T> = Result<T, RunnerError>;

/// Output from a runner invocation.
#[derive(Debug, Clone)]
pub struct RunnerOutput {
    /// The text output from the runner.
    pub text: String,

    /// Whether the runner completed successfully.
    pub success: bool,
}

impl RunnerOutput {
    /// Creates a new successful runner output.
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            success: true,
        }
    }

    /// Creates a new failed runner output.
    #[cfg(test)]
    pub fn failure(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            success: false,
        }
    }
}

/// Trait for runners that can execute prompts.
///
/// Runners are responsible for taking a prompt and returning the agent's response.
/// Different implementations can use different backends (CLI, API, mock, etc.).
pub trait Runner: Send + Sync {
    /// Returns the name of the runner.
    fn name(&self) -> &str;

    /// Executes a prompt and returns the response.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt text to send to the agent
    /// * `working_dir` - The working directory for the runner
    ///
    /// # Returns
    ///
    /// The runner's response.
    fn execute(&self, prompt: &str, working_dir: &std::path::Path) -> RunnerResult<RunnerOutput>;

    /// Checks if the runner is available/configured.
    fn is_available(&self) -> bool {
        true
    }
}
