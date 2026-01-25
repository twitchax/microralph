//! Runner type definitions.

use std::fmt;
use std::io::Write;

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

/// Token usage information from the underlying agent.
#[derive(Debug, Clone)]
pub struct UsageInfo {
    /// Number of input tokens consumed.
    pub input_tokens: Option<u64>,

    /// Number of output tokens generated.
    pub output_tokens: Option<u64>,

    /// Total tokens (input + output), if available separately.
    pub total_tokens: Option<u64>,
}

impl UsageInfo {
    /// Returns true if any usage information is present.
    pub fn has_data(&self) -> bool {
        self.input_tokens.is_some() || self.output_tokens.is_some() || self.total_tokens.is_some()
    }
}

/// Output from a runner invocation.
#[derive(Debug, Clone)]
pub struct RunnerOutput {
    /// The text output from the runner.
    pub text: String,

    /// Whether the runner completed successfully.
    pub success: bool,

    /// Optional usage information from the underlying agent.
    pub usage: Option<UsageInfo>,
}

impl RunnerOutput {
    /// Creates a new successful runner output.
    pub fn success(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            success: true,
            usage: None,
        }
    }

    /// Creates a new failed runner output.
    #[cfg(test)]
    pub fn failure(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            success: false,
            usage: None,
        }
    }

    /// Adds usage information to this output.
    pub fn with_usage(mut self, usage: UsageInfo) -> Self {
        self.usage = Some(usage);
        self
    }
}

/// Trait for runners that can execute prompts.
///
/// Runners are responsible for taking a prompt and returning the agent's response.
/// Different implementations can use different backends (CLI, API, mock, etc.).
pub trait Runner: Send + Sync {
    /// Returns the name of the runner.
    fn name(&self) -> &str;

    /// Formats the command that will be executed for display to the user.
    /// Should include all relevant parameters but exclude the prompt content.
    /// Returns None if command display is not applicable for this runner.
    #[allow(dead_code)]
    fn format_command_display(
        &self,
        _prompt: &str,
        _working_dir: &std::path::Path,
    ) -> Option<String> {
        None
    }

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

    /// Executes a prompt with real-time output streaming.
    ///
    /// Output is written to the provided writer as it becomes available,
    /// allowing users to watch progress in real-time.
    ///
    /// Default implementation falls back to `execute()` and writes output at the end.
    ///
    /// # Arguments
    ///
    /// * `prompt` - The prompt text to send to the agent
    /// * `working_dir` - The working directory for the runner
    /// * `output` - Writer to stream output to (typically stdout)
    ///
    /// # Returns
    ///
    /// The runner's response (also contains the full captured output).
    fn execute_streaming(
        &self,
        prompt: &str,
        working_dir: &std::path::Path,
        output: &mut dyn Write,
    ) -> RunnerResult<RunnerOutput> {
        let result = self.execute(prompt, working_dir)?;

        // Default: write all output at the end.
        let _ = writeln!(output, "{}", result.text);

        Ok(result)
    }

    /// Checks if the runner is available/configured.
    fn is_available(&self) -> bool {
        true
    }
}
