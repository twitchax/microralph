//! Runner type definitions.

#![allow(dead_code)]

use std::fmt;

/// Error type for runner operations.
#[derive(Debug)]
pub enum RunnerError {
    /// The runner process failed to start.
    ProcessFailed(String),

    /// The runner timed out.
    Timeout,

    /// The runner returned an error.
    ExecutionFailed(String),

    /// IO error.
    Io(std::io::Error),

    /// Other error.
    Other(String),
}

impl std::error::Error for RunnerError {}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProcessFailed(msg) => write!(f, "Process failed to start: {msg}"),
            Self::Timeout => write!(f, "Runner timed out"),
            Self::ExecutionFailed(msg) => write!(f, "Execution failed: {msg}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
            Self::Other(msg) => write!(f, "{msg}"),
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

    /// Exit code, if available.
    pub exit_code: Option<i32>,
}

impl RunnerOutput {
    /// Creates a new successful runner output.
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            success: true,
            exit_code: Some(0),
        }
    }

    /// Creates a new failed runner output.
    pub fn failure(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            success: false,
            exit_code: Some(1),
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
