//! Runner type definitions.

use std::fmt;
use std::io::Write;

/// Error type for runner operations.
#[derive(Debug)]
pub enum RunnerError {
    /// The runner process failed to start.
    ProcessFailed(String),

    /// The runner process was interrupted by a signal (e.g., Ctrl+C / SIGINT).
    Interrupted(String),

    /// IO error.
    Io(std::io::Error),
}

impl std::error::Error for RunnerError {}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProcessFailed(msg) => write!(f, "Process failed to start: {msg}"),
            Self::Interrupted(msg) => write!(f, "Process interrupted: {msg}"),
            Self::Io(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl From<std::io::Error> for RunnerError {
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}

impl RunnerError {
    /// Returns true if this error represents a signal interruption (e.g., Ctrl+C).
    pub fn is_interrupted(&self) -> bool {
        matches!(self, Self::Interrupted(_))
    }
}

/// Result type for runner operations.
pub type RunnerResult<T> = Result<T, RunnerError>;

/// Token usage information from the underlying agent.
#[derive(Debug, Clone)]
pub struct TokenUsageInfo {
    /// Number of input tokens consumed.
    pub input: Option<u64>,

    /// Number of output tokens generated.
    pub output: Option<u64>,

    /// Total tokens (input + output), if available separately.
    pub total: Option<u64>,
}

impl TokenUsageInfo {
    /// Returns true if any usage information is present.
    pub fn has_data(&self) -> bool {
        self.input.is_some() || self.output.is_some() || self.total.is_some()
    }

    /// Adds two optional u64 values, returning None only if both are None.
    fn add_optional(a: Option<u64>, b: Option<u64>) -> Option<u64> {
        match (a, b) {
            (Some(x), Some(y)) => Some(x + y),
            (Some(x), None) | (None, Some(x)) => Some(x),
            (None, None) => None,
        }
    }

    /// Merges another [`TokenUsageInfo`] into this one, summing token counts.
    pub fn merge(&mut self, other: &TokenUsageInfo) {
        self.input = Self::add_optional(self.input, other.input);
        self.output = Self::add_optional(self.output, other.output);
        self.total = Self::add_optional(self.total, other.total);
    }

    /// Aggregates an optional [`TokenUsageInfo`] into an optional accumulator.
    ///
    /// This is a common pattern when accumulating usage across multiple operations.
    /// If `new` is `Some`, it's merged into `total`. If `total` is `None`, it becomes a clone of `new`.
    pub fn aggregate(total: &mut Option<TokenUsageInfo>, new: Option<&TokenUsageInfo>) {
        if let Some(new_usage) = new {
            if let Some(total_usage) = total {
                total_usage.merge(new_usage);
            } else {
                *total = Some(new_usage.clone());
            }
        }
    }
}

/// Result from an interactive runner session.
///
/// Contains context from the interactive session that can be used
/// for subsequent non-interactive calls (e.g., synthesis phase).
#[derive(Debug, Clone)]
pub struct InteractiveResult {
    /// Optional session or conversation ID for resume-based context handoff.
    pub session_id: Option<String>,

    /// Optional transcript of the interactive conversation.
    pub transcript: Option<String>,
}

/// Output from a runner invocation.
#[derive(Debug, Clone)]
pub struct RunnerOutput {
    /// The text output from the runner.
    pub text: String,

    /// Whether the runner completed successfully.
    pub success: bool,

    /// Optional usage information from the underlying agent.
    pub usage: Option<TokenUsageInfo>,
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

    /// Launches an interactive session with inherited stdio.
    ///
    /// The user interacts directly with the underlying agent. On clean exit,
    /// returns an [`InteractiveResult`] containing a session ID and/or transcript
    /// for context handoff to a subsequent non-interactive call.
    ///
    /// # Arguments
    ///
    /// * `prompt` - Initial prompt/context to seed the interactive session
    /// * `working_dir` - The working directory for the runner
    ///
    /// # Returns
    ///
    /// An [`InteractiveResult`] with session context, or an error if the
    /// session could not be started or was interrupted.
    fn execute_interactive(
        &self,
        _prompt: &str,
        _working_dir: &std::path::Path,
    ) -> RunnerResult<InteractiveResult> {
        Err(RunnerError::ProcessFailed(
            "interactive mode is not supported by this runner".to_string(),
        ))
    }

    /// Executes a prompt by continuing/resuming the most recent session.
    ///
    /// Some runners (e.g., Claude) support session resume, which provides
    /// full conversational context from a previous interactive session without
    /// needing to pass a transcript in the prompt.
    ///
    /// Returns `None` if the runner does not support session resume,
    /// in which case the caller should fall back to [`execute`] with
    /// transcript context injected into the prompt.
    fn execute_continue(
        &self,
        _prompt: &str,
        _working_dir: &std::path::Path,
    ) -> Option<RunnerResult<RunnerOutput>> {
        None
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_info_merge_both_some() {
        let mut total = TokenUsageInfo {
            input: Some(100),
            output: Some(50),
            total: Some(150),
        };

        let other = TokenUsageInfo {
            input: Some(200),
            output: Some(100),
            total: Some(300),
        };

        total.merge(&other);

        assert_eq!(total.input, Some(300));
        assert_eq!(total.output, Some(150));
        assert_eq!(total.total, Some(450));
    }

    #[test]
    fn test_usage_info_merge_partial() {
        let mut total = TokenUsageInfo {
            input: Some(100),
            output: None,
            total: None,
        };

        let other = TokenUsageInfo {
            input: None,
            output: Some(50),
            total: Some(50),
        };

        total.merge(&other);

        assert_eq!(total.input, Some(100));
        assert_eq!(total.output, Some(50));
        assert_eq!(total.total, Some(50));
    }

    #[test]
    fn test_usage_info_merge_both_none() {
        let mut total = TokenUsageInfo {
            input: None,
            output: None,
            total: None,
        };

        let other = TokenUsageInfo {
            input: None,
            output: None,
            total: None,
        };

        total.merge(&other);

        assert_eq!(total.input, None);
        assert_eq!(total.output, None);
        assert_eq!(total.total, None);
    }

    #[test]
    fn test_usage_info_has_data() {
        let empty = TokenUsageInfo {
            input: None,
            output: None,
            total: None,
        };
        assert!(!empty.has_data());

        let with_input = TokenUsageInfo {
            input: Some(100),
            output: None,
            total: None,
        };
        assert!(with_input.has_data());
    }

    #[test]
    fn test_usage_info_aggregate_into_none() {
        let mut total = None;
        let new = Some(TokenUsageInfo {
            input: Some(100),
            output: Some(50),
            total: Some(150),
        });

        TokenUsageInfo::aggregate(&mut total, new.as_ref());

        assert!(total.is_some());
        let total = total.unwrap();
        assert_eq!(total.input, Some(100));
        assert_eq!(total.output, Some(50));
        assert_eq!(total.total, Some(150));
    }

    #[test]
    fn test_usage_info_aggregate_into_some() {
        let mut total = Some(TokenUsageInfo {
            input: Some(100),
            output: Some(50),
            total: Some(150),
        });
        let new = Some(TokenUsageInfo {
            input: Some(200),
            output: Some(100),
            total: Some(300),
        });

        TokenUsageInfo::aggregate(&mut total, new.as_ref());

        assert!(total.is_some());
        let total = total.unwrap();
        assert_eq!(total.input, Some(300));
        assert_eq!(total.output, Some(150));
        assert_eq!(total.total, Some(450));
    }

    #[test]
    fn test_usage_info_aggregate_with_none_new() {
        let mut total = Some(TokenUsageInfo {
            input: Some(100),
            output: Some(50),
            total: Some(150),
        });
        let new = None;

        TokenUsageInfo::aggregate(&mut total, new.as_ref());

        // Total should be unchanged.
        assert!(total.is_some());
        let total = total.unwrap();
        assert_eq!(total.input, Some(100));
        assert_eq!(total.output, Some(50));
        assert_eq!(total.total, Some(150));
    }

    #[test]
    fn test_usage_info_aggregate_both_none() {
        let mut total: Option<TokenUsageInfo> = None;
        let new: Option<TokenUsageInfo> = None;

        TokenUsageInfo::aggregate(&mut total, new.as_ref());

        assert!(total.is_none());
    }

    #[test]
    fn test_interactive_result_default_fields() {
        let result = InteractiveResult {
            session_id: None,
            transcript: None,
        };

        assert!(result.session_id.is_none());
        assert!(result.transcript.is_none());
    }

    #[test]
    fn test_interactive_result_with_data() {
        let result = InteractiveResult {
            session_id: Some("session-123".to_string()),
            transcript: Some("User: Hello\nAgent: Hi!".to_string()),
        };

        assert_eq!(result.session_id.as_deref(), Some("session-123"));
        assert!(result.transcript.unwrap().contains("Hello"));
    }

    /// A minimal runner that only implements required methods, using defaults for the rest.
    struct MinimalRunner;

    impl Runner for MinimalRunner {
        fn name(&self) -> &'static str {
            "minimal"
        }

        fn execute(
            &self,
            _prompt: &str,
            _working_dir: &std::path::Path,
        ) -> RunnerResult<RunnerOutput> {
            Ok(RunnerOutput::success("ok"))
        }
    }

    #[test]
    fn test_runner_default_execute_interactive_returns_error() {
        let runner = MinimalRunner;
        let result = runner.execute_interactive("test", std::path::Path::new("."));

        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("not supported"),
            "Expected 'not supported' error, got: {err}"
        );
    }

    #[test]
    fn test_runner_default_execute_continue_returns_none() {
        let runner = MinimalRunner;
        let result = runner.execute_continue("test", std::path::Path::new("."));

        assert!(
            result.is_none(),
            "Default execute_continue should return None"
        );
    }

    #[test]
    fn test_runner_error_is_interrupted() {
        let interrupted = RunnerError::Interrupted("signal 2".to_string());
        assert!(interrupted.is_interrupted());

        let failed = RunnerError::ProcessFailed("exit code 1".to_string());
        assert!(!failed.is_interrupted());

        let io_err = RunnerError::Io(std::io::Error::other("test"));
        assert!(!io_err.is_interrupted());
    }

    #[test]
    fn test_runner_error_interrupted_display() {
        let err = RunnerError::Interrupted("signal 2 (SIGINT/Ctrl+C)".to_string());
        let display = err.to_string();
        assert!(
            display.contains("interrupted"),
            "Display should contain 'interrupted', got: {display}"
        );
        assert!(display.contains("signal 2"));
    }
}
