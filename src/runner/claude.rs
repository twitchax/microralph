//! Claude CLI runner.
//!
//! This runner shells out to the Claude CLI (`claude`) to execute prompts.
//! It uses `--dangerously-skip-permissions` by default for yolo mode (no permission prompts).
//!
//! The `Runner` trait is automatically implemented via the blanket impl in `cli_runner`
//! for all types that implement `CliRunnerConfig`.

use std::path::Path;

use super::cli_runner::CliRunnerConfig;
use super::types::TokenUsageInfo;

/// Permission mode for the Claude runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClaudePermissionMode {
    /// Skip all permissions (--dangerously-skip-permissions).
    #[default]
    Yolo,

    /// No special permission flags (will prompt for permissions).
    #[cfg(test)]
    Manual,
}

/// Configuration for the Claude runner.
#[derive(Debug, Clone)]
pub struct ClaudeConfig {
    /// The path to the claude CLI binary.
    pub claude_path: String,

    /// Permission mode.
    pub permission_mode: ClaudePermissionMode,

    /// Whether to disable the `ask_user` tool.
    pub no_ask_user: bool,

    /// The model to use (e.g., "claude-sonnet-4.5").
    pub model: Option<String>,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            claude_path: "claude".to_string(),
            permission_mode: ClaudePermissionMode::Yolo,
            no_ask_user: true,
            model: None,
        }
    }
}

impl ClaudeConfig {
    /// Creates a new config with the default claude path.
    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path to the claude binary.
    #[cfg(test)]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.claude_path = path.into();
        self
    }

    /// Sets the permission mode.
    #[cfg(test)]
    pub fn with_permission_mode(mut self, mode: ClaudePermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Sets whether to disable the `ask_user` tool.
    #[cfg(test)]
    pub fn with_no_ask_user(mut self, no_ask_user: bool) -> Self {
        self.no_ask_user = no_ask_user;
        self
    }

    /// Sets the model to use.
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }
}

/// A runner that shells out to the Claude CLI.
///
/// This runner invokes `claude -p "<prompt>"` with appropriate permission flags.
/// By default, it uses `--dangerously-skip-permissions` (yolo mode) to avoid permission prompts.
#[derive(Debug)]
pub struct ClaudeRunner {
    config: ClaudeConfig,
}

impl ClaudeRunner {
    /// Creates a new Claude runner with default configuration.
    pub fn new() -> Self {
        Self {
            config: ClaudeConfig::default(),
        }
    }

    /// Creates a new Claude runner with the specified model.
    pub fn with_model(model: Option<String>) -> Self {
        Self {
            config: ClaudeConfig::default().with_model(model),
        }
    }

    /// Creates a new Claude runner with the given configuration.
    #[cfg(test)]
    pub fn with_config(config: ClaudeConfig) -> Self {
        Self { config }
    }

    /// Appends common configuration flags to the provided args vector.
    ///
    /// This is the single source of truth for flag generation, used by both
    /// `build_args` and `format_display_parts_impl` to avoid duplication.
    fn append_config_flags(&self, args: &mut Vec<String>) {
        // Permission flags.
        match self.config.permission_mode {
            ClaudePermissionMode::Yolo => {
                args.push("--dangerously-skip-permissions".to_string());
            }
            #[cfg(test)]
            ClaudePermissionMode::Manual => {
                // No permission flags.
            }
        }

        // Disable ask_user tool for autonomous operation via permission mode.
        if self.config.no_ask_user {
            args.push("--permission-mode".to_string());
            args.push("dontAsk".to_string());
        }

        // Model selection.
        if let Some(ref model) = self.config.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
    }

    /// Builds display parts for [`format_command_display`].
    fn format_display_parts_impl(&self, working_dir: &Path) -> Vec<String> {
        let mut parts = vec![self.config.claude_path.clone()];

        self.append_config_flags(&mut parts);

        parts.push("-p".to_string());
        parts.push("<prompt>".to_string());
        parts.push("--working-dir".to_string());
        parts.push(working_dir.display().to_string());

        parts
    }

    /// Attempts to parse token usage information from Claude CLI output.
    ///
    /// Claude CLI supports `--output-format json` which includes a `usage` object with:
    /// - `input_tokens`: Number of input tokens
    /// - `output_tokens`: Number of output tokens
    /// - `cache_creation_input_tokens`: Tokens used for cache creation
    /// - `cache_read_input_tokens`: Tokens read from cache
    ///
    /// This function parses the JSON output and extracts token usage.
    fn parse_usage(text: &str) -> Option<TokenUsageInfo> {
        // Try to parse as JSON.
        let json: serde_json::Value = serde_json::from_str(text).ok()?;

        // Extract usage object.
        let usage = json.get("usage")?;

        let input = usage
            .get("input_tokens")
            .and_then(serde_json::Value::as_u64);

        let output = usage
            .get("output_tokens")
            .and_then(serde_json::Value::as_u64);

        let total = match (input, output) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };

        // Return TokenUsageInfo if we found at least one piece of information.
        if input.is_some() || output.is_some() {
            Some(TokenUsageInfo {
                input,
                output,
                total,
            })
        } else {
            None
        }
    }

    /// Extracts the actual response text from Claude CLI JSON output.
    ///
    /// When using `--output-format json`, Claude CLI returns:
    /// ```json
    /// {
    ///   "type": "result",
    ///   "result": "The actual response text...",
    ///   "usage": {...},
    ///   ...
    /// }
    /// ```
    ///
    /// This function extracts the `result` field and returns it as plain text.
    fn extract_result_from_json(text: &str) -> String {
        // Try to parse as JSON.
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text)
            && let Some(result) = json.get("result").and_then(|v| v.as_str())
        {
            return result.to_string();
        }

        // If parsing fails or result is missing, return original text.
        text.to_string()
    }

    /// Strips the statistics metadata from Claude CLI JSON output.
    ///
    /// When using `--output-format json`, Claude CLI returns JSON with usage stats,
    /// type information, and other metadata. This function removes all metadata and
    /// returns only the actual response text (the `result` field).
    ///
    /// This is analogous to `CopilotRunner::strip_usage_stats` but leverages Claude's
    /// structured JSON output for cleaner parsing.
    ///
    /// # Example
    /// ```ignore
    /// let json_output = r#"{"type": "result", "result": "Hello!", "usage": {...}}"#;
    /// let clean = ClaudeRunner::strip_usage_stats(json_output);
    /// assert_eq!(clean, "Hello!");
    /// ```
    #[allow(dead_code)] // Public API for consistency with CopilotRunner
    pub fn strip_usage_stats(text: &str) -> String {
        Self::extract_result_from_json(text)
    }
}

impl Default for ClaudeRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CliRunnerConfig for ClaudeRunner {
    fn name(&self) -> &'static str {
        "claude"
    }

    fn binary_path(&self) -> &str {
        &self.config.claude_path
    }

    fn build_args(&self, prompt: &str) -> Vec<String> {
        let mut args = Vec::new();

        // Prompt (non-interactive mode).
        args.push("-p".to_string());
        args.push(prompt.to_string());

        self.append_config_flags(&mut args);

        // Request JSON output format for token usage parsing.
        args.push("--output-format".to_string());
        args.push("json".to_string());

        args
    }

    fn parse_usage(&self, text: &str) -> Option<TokenUsageInfo> {
        Self::parse_usage(text)
    }

    fn post_process_output(&self, text: &str) -> String {
        Self::extract_result_from_json(text)
    }

    fn format_display_parts(&self, working_dir: &Path) -> Vec<String> {
        self.format_display_parts_impl(working_dir)
    }

    fn build_interactive_args(&self, prompt: &str) -> Option<Vec<String>> {
        let mut args = Vec::new();

        // Initial prompt for the interactive session (no -p flag).
        args.push("--initial-prompt".to_string());
        args.push(prompt.to_string());

        self.append_config_flags(&mut args);

        Some(args)
    }
}

// NOTE: `Runner` trait is automatically implemented via blanket impl in `cli_runner.rs`
// for all types that implement `CliRunnerConfig`.

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runner::Runner;

    #[test]
    fn test_claude_config_default() {
        let config = ClaudeConfig::default();

        assert_eq!(config.claude_path, "claude");
        assert_eq!(config.permission_mode, ClaudePermissionMode::Yolo);
        assert!(config.no_ask_user);
    }

    #[test]
    fn test_claude_config_builder() {
        let config = ClaudeConfig::new()
            .with_path("/custom/path/claude")
            .with_permission_mode(ClaudePermissionMode::Manual)
            .with_no_ask_user(false);

        assert_eq!(config.claude_path, "/custom/path/claude");
        assert_eq!(config.permission_mode, ClaudePermissionMode::Manual);
        assert!(!config.no_ask_user);
    }

    #[test]
    fn test_build_args_yolo_mode() {
        let runner = ClaudeRunner::new();
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(args.contains(&"--permission-mode".to_string()));
        assert!(args.contains(&"dontAsk".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"json".to_string()));
    }

    #[test]
    fn test_build_args_manual_mode() {
        let config = ClaudeConfig::new()
            .with_permission_mode(ClaudePermissionMode::Manual)
            .with_no_ask_user(false);
        let runner = ClaudeRunner::with_config(config);
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(!args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(!args.contains(&"--permission-mode".to_string()));
        assert!(!args.contains(&"dontAsk".to_string()));
        assert!(args.contains(&"--output-format".to_string()));
        assert!(args.contains(&"json".to_string()));
    }

    #[test]
    fn test_runner_name() {
        let runner = ClaudeRunner::new();
        assert_eq!(Runner::name(&runner), "claude");
    }

    #[test]
    fn test_runner_default() {
        let runner = ClaudeRunner::default();
        assert_eq!(Runner::name(&runner), "claude");
    }

    #[test]
    fn test_build_args_with_model() {
        let runner = ClaudeRunner::with_model(Some("claude-sonnet-4.5".to_string()));
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-sonnet-4.5".to_string()));
    }

    #[test]
    fn test_build_args_without_model() {
        let runner = ClaudeRunner::new();
        let args = runner.build_args("test prompt");

        assert!(!args.contains(&"--model".to_string()));
    }

    #[test]
    fn test_with_model_constructor() {
        let runner = ClaudeRunner::with_model(Some("claude-opus-4".to_string()));
        let args = runner.build_args("prompt");

        // Should have model flag.
        let model_idx = args.iter().position(|a| a == "--model").unwrap();
        assert_eq!(args[model_idx + 1], "claude-opus-4");

        // Should still have other default flags.
        assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
    }

    #[test]
    fn test_parse_usage_returns_none() {
        let output = "Hello world\nThis is just normal output.";

        let usage = ClaudeRunner::parse_usage(output);

        assert!(usage.is_none());
    }

    #[test]
    fn test_parse_usage_from_json() {
        let json_output = r#"{
            "type": "result",
            "subtype": "success",
            "result": "The answer is 42",
            "usage": {
                "input_tokens": 1234,
                "output_tokens": 56,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            }
        }"#;

        let usage = ClaudeRunner::parse_usage(json_output).unwrap();

        assert_eq!(usage.input, Some(1234));
        assert_eq!(usage.output, Some(56));
        assert_eq!(usage.total, Some(1290));
    }

    #[test]
    fn test_parse_usage_missing_fields() {
        let json_output = r#"{
            "type": "result",
            "usage": {
                "input_tokens": 100
            }
        }"#;

        let usage = ClaudeRunner::parse_usage(json_output).unwrap();

        assert_eq!(usage.input, Some(100));
        assert_eq!(usage.output, None);
        assert_eq!(usage.total, None);
    }

    #[test]
    fn test_extract_result_from_json() {
        let json_output = r#"{
            "type": "result",
            "result": "Hello, world!",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        }"#;

        let result = ClaudeRunner::extract_result_from_json(json_output);

        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_extract_result_from_invalid_json() {
        let invalid_json = "This is not JSON";

        let result = ClaudeRunner::extract_result_from_json(invalid_json);

        assert_eq!(result, "This is not JSON");
    }

    #[test]
    fn test_extract_result_missing_result_field() {
        let json_output = r#"{
            "type": "result",
            "usage": {
                "input_tokens": 10
            }
        }"#;

        let result = ClaudeRunner::extract_result_from_json(json_output);

        // Should return original text if result field is missing
        assert_eq!(result, json_output);
    }

    #[test]
    fn test_format_command_display() {
        let runner = ClaudeRunner::with_model(Some("claude-sonnet-4".to_string()));
        let prompt = "test prompt";
        let working_dir = Path::new("/home/user/project");

        let cmd_display = runner.format_command_display(prompt, working_dir).unwrap();

        // Should include claude path
        assert!(cmd_display.contains("claude"));
        // Should include permission flags
        assert!(cmd_display.contains("--dangerously-skip-permissions"));
        // Should include permission mode
        assert!(cmd_display.contains("--permission-mode"));
        assert!(cmd_display.contains("dontAsk"));
        // Should include model
        assert!(cmd_display.contains("--model"));
        assert!(cmd_display.contains("claude-sonnet-4"));
        // Should include prompt placeholder
        assert!(cmd_display.contains("-p"));
        assert!(cmd_display.contains("<prompt>"));
        // Should include working directory
        assert!(cmd_display.contains("--working-dir"));
        assert!(cmd_display.contains("/home/user/project"));
        // Should NOT include the actual prompt content
        assert!(!cmd_display.contains("test prompt"));
    }

    #[test]
    fn test_format_command_display_no_model() {
        let runner = ClaudeRunner::new();
        let prompt = "test";
        let working_dir = Path::new("/tmp");

        let cmd_display = runner.format_command_display(prompt, working_dir).unwrap();

        // Should NOT include model flags
        assert!(!cmd_display.contains("--model"));
    }

    #[test]
    fn test_strip_usage_stats_with_full_json() {
        let json_output = r#"{
            "type": "result",
            "subtype": "success",
            "result": "Hello, world!",
            "usage": {
                "input_tokens": 1234,
                "output_tokens": 56,
                "cache_creation_input_tokens": 0,
                "cache_read_input_tokens": 0
            },
            "metadata": {
                "session_id": "abc123"
            }
        }"#;

        let stripped = ClaudeRunner::strip_usage_stats(json_output);

        // Should return only the result text, stripping all metadata
        assert_eq!(stripped, "Hello, world!");
        assert!(!stripped.contains("usage"));
        assert!(!stripped.contains("input_tokens"));
        assert!(!stripped.contains("metadata"));
    }

    #[test]
    fn test_strip_usage_stats_with_plain_text() {
        let plain_text = "This is just plain text output";

        let stripped = ClaudeRunner::strip_usage_stats(plain_text);

        // Should return text as-is when not JSON
        assert_eq!(stripped, plain_text);
    }

    #[test]
    fn test_strip_usage_stats_preserves_multiline_result() {
        let json_output = r#"{
            "type": "result",
            "result": "Line 1\nLine 2\nLine 3",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50
            }
        }"#;

        let stripped = ClaudeRunner::strip_usage_stats(json_output);

        // Should preserve newlines in the result
        assert_eq!(stripped, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_strip_usage_stats_with_empty_result() {
        let json_output = r#"{
            "type": "result",
            "result": "",
            "usage": {
                "input_tokens": 10
            }
        }"#;

        let stripped = ClaudeRunner::strip_usage_stats(json_output);

        // Should return empty string for empty result
        assert_eq!(stripped, "");
    }

    #[test]
    fn test_build_interactive_args_yolo_mode() {
        let runner = ClaudeRunner::new();
        let args = runner.build_interactive_args("discovery prompt").unwrap();

        // Should use --initial-prompt for interactive mode.
        assert!(args.contains(&"--initial-prompt".to_string()));
        assert!(args.contains(&"discovery prompt".to_string()));

        // Should NOT use -p (non-interactive).
        assert!(!args.contains(&"-p".to_string()));

        // Should NOT use --output-format json (would break interactive display).
        assert!(!args.contains(&"--output-format".to_string()));

        // Should include config flags.
        assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(args.contains(&"--permission-mode".to_string()));
        assert!(args.contains(&"dontAsk".to_string()));
    }

    #[test]
    fn test_build_interactive_args_with_model() {
        let runner = ClaudeRunner::with_model(Some("claude-sonnet-4".to_string()));
        let args = runner.build_interactive_args("prompt").unwrap();

        assert!(args.contains(&"--initial-prompt".to_string()));
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-sonnet-4".to_string()));
    }

    #[test]
    fn test_build_interactive_args_manual_mode() {
        let config = ClaudeConfig::new()
            .with_permission_mode(ClaudePermissionMode::Manual)
            .with_no_ask_user(false);
        let runner = ClaudeRunner::with_config(config);
        let args = runner.build_interactive_args("prompt").unwrap();

        assert!(args.contains(&"--initial-prompt".to_string()));
        assert!(!args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(!args.contains(&"--permission-mode".to_string()));
        assert!(!args.contains(&"dontAsk".to_string()));
    }

    #[test]
    fn test_strip_usage_stats_missing_result_field() {
        let json_output = r#"{
            "type": "error",
            "error": "Something went wrong",
            "usage": {
                "input_tokens": 10
            }
        }"#;

        let stripped = ClaudeRunner::strip_usage_stats(json_output);

        // Should return original JSON when result field is missing
        assert_eq!(stripped, json_output);
    }
}
