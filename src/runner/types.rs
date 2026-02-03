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
#[allow(clippy::struct_field_names)] // Fields are named for clarity in token usage context
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

    /// Adds two optional u64 values, returning None only if both are None.
    fn add_optional(a: Option<u64>, b: Option<u64>) -> Option<u64> {
        match (a, b) {
            (Some(x), Some(y)) => Some(x + y),
            (Some(x), None) | (None, Some(x)) => Some(x),
            (None, None) => None,
        }
    }

    /// Merges another [`UsageInfo`] into this one, summing token counts.
    pub fn merge(&mut self, other: &UsageInfo) {
        self.input_tokens = Self::add_optional(self.input_tokens, other.input_tokens);
        self.output_tokens = Self::add_optional(self.output_tokens, other.output_tokens);
        self.total_tokens = Self::add_optional(self.total_tokens, other.total_tokens);
    }

    /// Aggregates an optional [`UsageInfo`] into an optional accumulator.
    ///
    /// This is a common pattern when accumulating usage across multiple operations.
    /// If `new` is `Some`, it's merged into `total`. If `total` is `None`, it becomes a clone of `new`.
    pub fn aggregate(total: &mut Option<UsageInfo>, new: Option<&UsageInfo>) {
        if let Some(new_usage) = new {
            if let Some(total_usage) = total {
                total_usage.merge(new_usage);
            } else {
                *total = Some(new_usage.clone());
            }
        }
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_usage_info_merge_both_some() {
        let mut total = UsageInfo {
            input_tokens: Some(100),
            output_tokens: Some(50),
            total_tokens: Some(150),
        };

        let other = UsageInfo {
            input_tokens: Some(200),
            output_tokens: Some(100),
            total_tokens: Some(300),
        };

        total.merge(&other);

        assert_eq!(total.input_tokens, Some(300));
        assert_eq!(total.output_tokens, Some(150));
        assert_eq!(total.total_tokens, Some(450));
    }

    #[test]
    fn test_usage_info_merge_partial() {
        let mut total = UsageInfo {
            input_tokens: Some(100),
            output_tokens: None,
            total_tokens: None,
        };

        let other = UsageInfo {
            input_tokens: None,
            output_tokens: Some(50),
            total_tokens: Some(50),
        };

        total.merge(&other);

        assert_eq!(total.input_tokens, Some(100));
        assert_eq!(total.output_tokens, Some(50));
        assert_eq!(total.total_tokens, Some(50));
    }

    #[test]
    fn test_usage_info_merge_both_none() {
        let mut total = UsageInfo {
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
        };

        let other = UsageInfo {
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
        };

        total.merge(&other);

        assert_eq!(total.input_tokens, None);
        assert_eq!(total.output_tokens, None);
        assert_eq!(total.total_tokens, None);
    }

    #[test]
    fn test_usage_info_has_data() {
        let empty = UsageInfo {
            input_tokens: None,
            output_tokens: None,
            total_tokens: None,
        };
        assert!(!empty.has_data());

        let with_input = UsageInfo {
            input_tokens: Some(100),
            output_tokens: None,
            total_tokens: None,
        };
        assert!(with_input.has_data());
    }

    #[test]
    fn test_usage_info_aggregate_into_none() {
        let mut total = None;
        let new = Some(UsageInfo {
            input_tokens: Some(100),
            output_tokens: Some(50),
            total_tokens: Some(150),
        });

        UsageInfo::aggregate(&mut total, new.as_ref());

        assert!(total.is_some());
        let total = total.unwrap();
        assert_eq!(total.input_tokens, Some(100));
        assert_eq!(total.output_tokens, Some(50));
        assert_eq!(total.total_tokens, Some(150));
    }

    #[test]
    fn test_usage_info_aggregate_into_some() {
        let mut total = Some(UsageInfo {
            input_tokens: Some(100),
            output_tokens: Some(50),
            total_tokens: Some(150),
        });
        let new = Some(UsageInfo {
            input_tokens: Some(200),
            output_tokens: Some(100),
            total_tokens: Some(300),
        });

        UsageInfo::aggregate(&mut total, new.as_ref());

        assert!(total.is_some());
        let total = total.unwrap();
        assert_eq!(total.input_tokens, Some(300));
        assert_eq!(total.output_tokens, Some(150));
        assert_eq!(total.total_tokens, Some(450));
    }

    #[test]
    fn test_usage_info_aggregate_with_none_new() {
        let mut total = Some(UsageInfo {
            input_tokens: Some(100),
            output_tokens: Some(50),
            total_tokens: Some(150),
        });
        let new = None;

        UsageInfo::aggregate(&mut total, new.as_ref());

        // Total should be unchanged.
        assert!(total.is_some());
        let total = total.unwrap();
        assert_eq!(total.input_tokens, Some(100));
        assert_eq!(total.output_tokens, Some(50));
        assert_eq!(total.total_tokens, Some(150));
    }

    #[test]
    fn test_usage_info_aggregate_both_none() {
        let mut total: Option<UsageInfo> = None;
        let new: Option<UsageInfo> = None;

        UsageInfo::aggregate(&mut total, new.as_ref());

        assert!(total.is_none());
    }
}
