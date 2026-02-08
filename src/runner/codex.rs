//! Codex CLI runner.
//!
//! This runner shells out to the Codex CLI (`codex`) to execute prompts.
//! It uses `--full-auto` by default for yolo mode (no permission prompts).
//!
//! The `Runner` trait is automatically implemented via the blanket impl in `cli_runner`
//! for all types that implement `CliRunnerConfig`.

use std::path::Path;

use super::cli_runner::CliRunnerConfig;
use super::types::TokenUsageInfo;

/// Permission mode for the Codex runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CodexPermissionMode {
    /// Full auto mode (--full-auto).
    #[default]
    Yolo,

    /// No special permission flags (will prompt for permissions).
    #[cfg(test)]
    Manual,
}

/// Configuration for the Codex runner.
#[derive(Debug, Clone)]
pub struct CodexConfig {
    /// The path to the codex CLI binary.
    pub codex_path: String,

    /// Permission mode.
    pub permission_mode: CodexPermissionMode,

    /// The model to use (e.g., "o4-mini").
    pub model: Option<String>,
}

impl Default for CodexConfig {
    fn default() -> Self {
        Self {
            codex_path: "codex".to_string(),
            permission_mode: CodexPermissionMode::Yolo,
            model: None,
        }
    }
}

impl CodexConfig {
    /// Creates a new config with the default codex path.
    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path to the codex binary.
    #[cfg(test)]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.codex_path = path.into();
        self
    }

    /// Sets the permission mode.
    #[cfg(test)]
    pub fn with_permission_mode(mut self, mode: CodexPermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Sets the model to use.
    pub fn with_model(mut self, model: Option<String>) -> Self {
        self.model = model;
        self
    }
}

/// A runner that shells out to the Codex CLI.
///
/// This runner invokes `codex exec "<prompt>"` with appropriate flags.
/// By default, it uses `--full-auto` (yolo mode) to auto-approve actions.
#[derive(Debug)]
pub struct CodexRunner {
    config: CodexConfig,
}

impl CodexRunner {
    /// Creates a new Codex runner with default configuration.
    pub fn new() -> Self {
        Self {
            config: CodexConfig::default(),
        }
    }

    /// Creates a new Codex runner with the specified model.
    pub fn with_model(model: Option<String>) -> Self {
        Self {
            config: CodexConfig::default().with_model(model),
        }
    }

    /// Creates a new Codex runner with the given configuration.
    #[cfg(test)]
    pub fn with_config(config: CodexConfig) -> Self {
        Self { config }
    }

    /// Appends common configuration flags to the provided args vector.
    ///
    /// This is the single source of truth for flag generation, used by both
    /// `build_args` and `format_display_parts_impl` to avoid duplication.
    fn append_config_flags(&self, args: &mut Vec<String>) {
        // Permission flags.
        match self.config.permission_mode {
            CodexPermissionMode::Yolo => {
                args.push("--full-auto".to_string());
            }
            #[cfg(test)]
            CodexPermissionMode::Manual => {
                // No permission flags.
            }
        }

        // Model selection.
        if let Some(ref model) = self.config.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
    }

    /// Builds display parts for [`format_command_display`].
    fn format_display_parts_impl(&self, working_dir: &Path) -> Vec<String> {
        let mut parts = vec![self.config.codex_path.clone()];

        parts.push("exec".to_string());
        self.append_config_flags(&mut parts);

        parts.push("<prompt>".to_string());
        parts.push("--cd".to_string());
        parts.push(working_dir.display().to_string());

        parts
    }

    /// Attempts to parse token usage information from Codex CLI JSON output.
    ///
    /// Codex CLI with `--json` flag outputs JSON that includes a `usage` object:
    /// ```json
    /// {
    ///   "usage": {
    ///     "input_tokens": 26549,
    ///     "output_tokens": 1590
    ///   }
    /// }
    /// ```
    fn parse_usage(text: &str) -> Option<TokenUsageInfo> {
        let json: serde_json::Value = serde_json::from_str(text).ok()?;

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

    /// Extracts the actual response text from Codex CLI JSON output.
    ///
    /// When using `--json`, Codex CLI returns structured JSON. This function
    /// extracts the `result` field and returns it as plain text.
    fn extract_result_from_json(text: &str) -> String {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(text)
            && let Some(result) = json.get("result").and_then(|v| v.as_str())
        {
            return result.to_string();
        }

        text.to_string()
    }

    /// Strips the statistics metadata from Codex CLI JSON output.
    ///
    /// Returns only the actual response text (the `result` field).
    #[allow(dead_code)] // Public API for consistency with CopilotRunner
    pub fn strip_usage_stats(text: &str) -> String {
        Self::extract_result_from_json(text)
    }
}

impl Default for CodexRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CliRunnerConfig for CodexRunner {
    fn name(&self) -> &'static str {
        "codex"
    }

    fn binary_path(&self) -> &str {
        &self.config.codex_path
    }

    fn build_args(&self, prompt: &str) -> Vec<String> {
        let mut args = Vec::new();

        // Use exec subcommand for non-interactive mode.
        args.push("exec".to_string());

        self.append_config_flags(&mut args);

        // Prompt text.
        args.push(prompt.to_string());

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

        self.append_config_flags(&mut args);

        // Prompt text (no exec subcommand for interactive mode).
        args.push(prompt.to_string());

        Some(args)
    }
}

// NOTE: `Runner` trait is automatically implemented via blanket impl in `cli_runner.rs`
// for all types that implement `CliRunnerConfig`.

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use std::path::Path;

    use super::*;
    use crate::runner::Runner;

    // ── Config tests ──────────────────────────────────────────────────

    #[test]
    fn test_codex_config_default() {
        let config = CodexConfig::default();

        assert_eq!(config.codex_path, "codex");
        assert_eq!(config.permission_mode, CodexPermissionMode::Yolo);
        assert!(config.model.is_none());
    }

    #[test]
    fn test_codex_config_builder() {
        let config = CodexConfig::new()
            .with_path("/custom/path/codex")
            .with_permission_mode(CodexPermissionMode::Manual);

        assert_eq!(config.codex_path, "/custom/path/codex");
        assert_eq!(config.permission_mode, CodexPermissionMode::Manual);
    }

    #[test]
    fn test_codex_runner_with_config() {
        let config = CodexConfig::new()
            .with_path("/custom/codex")
            .with_permission_mode(CodexPermissionMode::Manual);
        let runner = CodexRunner::with_config(config);

        assert_eq!(runner.config.codex_path, "/custom/codex");
        assert_eq!(runner.config.permission_mode, CodexPermissionMode::Manual);
    }

    // ── build_args tests (non-interactive) ────────────────────────────

    #[test]
    fn test_build_args_yolo_mode() {
        let runner = CodexRunner::new();
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"exec".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(args.contains(&"--full-auto".to_string()));
    }

    #[test]
    fn test_build_args_manual_mode() {
        let config = CodexConfig::new().with_permission_mode(CodexPermissionMode::Manual);
        let runner = CodexRunner::with_config(config);
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"exec".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(!args.contains(&"--full-auto".to_string()));
    }

    #[test]
    fn test_build_args_with_model() {
        let runner = CodexRunner::with_model(Some("o4-mini".to_string()));
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"o4-mini".to_string()));
    }

    #[test]
    fn test_build_args_without_model() {
        let runner = CodexRunner::new();
        let args = runner.build_args("test prompt");

        assert!(!args.contains(&"--model".to_string()));
    }

    #[test]
    fn test_with_model_constructor() {
        let runner = CodexRunner::with_model(Some("o3".to_string()));
        let args = runner.build_args("prompt");

        // Should have model flag.
        let model_idx = args.iter().position(|a| a == "--model").unwrap();
        assert_eq!(args[model_idx + 1], "o3");

        // Should still have other default flags.
        assert!(args.contains(&"--full-auto".to_string()));
    }

    // ── parse_usage tests ─────────────────────────────────────────────

    #[test]
    fn test_parse_usage_returns_none() {
        let output = "Hello world\nThis is just normal output.";

        let usage = CodexRunner::parse_usage(output);

        assert!(usage.is_none());
    }

    #[test]
    fn test_parse_usage_from_json() {
        let json_output = r#"{
            "result": "The answer is 42",
            "usage": {
                "input_tokens": 26549,
                "output_tokens": 1590
            }
        }"#;

        let usage = CodexRunner::parse_usage(json_output).unwrap();

        assert_eq!(usage.input, Some(26549));
        assert_eq!(usage.output, Some(1590));
        assert_eq!(usage.total, Some(28139));
    }

    #[test]
    fn test_parse_usage_missing_fields() {
        let json_output = r#"{
            "result": "partial",
            "usage": {
                "input_tokens": 100
            }
        }"#;

        let usage = CodexRunner::parse_usage(json_output).unwrap();

        assert_eq!(usage.input, Some(100));
        assert_eq!(usage.output, None);
        assert_eq!(usage.total, None);
    }

    #[test]
    fn test_parse_usage_no_usage_object() {
        let json_output = r#"{
            "result": "no usage here"
        }"#;

        let usage = CodexRunner::parse_usage(json_output);

        assert!(usage.is_none());
    }

    // ── post_process_output / extract_result tests ────────────────────

    #[test]
    fn test_extract_result_from_json() {
        let json_output = r#"{
            "result": "Hello, world!",
            "usage": {
                "input_tokens": 10,
                "output_tokens": 5
            }
        }"#;

        let result = CodexRunner::extract_result_from_json(json_output);

        assert_eq!(result, "Hello, world!");
    }

    #[test]
    fn test_extract_result_from_invalid_json() {
        let invalid_json = "This is not JSON";

        let result = CodexRunner::extract_result_from_json(invalid_json);

        assert_eq!(result, "This is not JSON");
    }

    #[test]
    fn test_extract_result_missing_result_field() {
        let json_output = r#"{
            "usage": {
                "input_tokens": 10
            }
        }"#;

        let result = CodexRunner::extract_result_from_json(json_output);

        assert_eq!(result, json_output);
    }

    // ── post_process_output via trait ─────────────────────────────────

    #[test]
    fn test_post_process_output_extracts_result() {
        let runner = CodexRunner::new();
        let json_output = r#"{
            "result": "extracted text",
            "usage": { "input_tokens": 5, "output_tokens": 3 }
        }"#;

        let result = runner.post_process_output(json_output);

        assert_eq!(result, "extracted text");
    }

    #[test]
    fn test_post_process_output_plain_text() {
        let runner = CodexRunner::new();
        let plain = "plain text output";

        let result = runner.post_process_output(plain);

        assert_eq!(result, "plain text output");
    }

    // ── format_display_parts tests ────────────────────────────────────

    #[test]
    fn test_format_command_display() {
        let runner = CodexRunner::with_model(Some("o4-mini".to_string()));
        let prompt = "test prompt";
        let working_dir = Path::new("/home/user/project");

        let cmd_display = runner.format_command_display(prompt, working_dir).unwrap();

        // Should include codex path.
        assert!(cmd_display.contains("codex"));
        // Should include exec subcommand.
        assert!(cmd_display.contains("exec"));
        // Should include permission flag.
        assert!(cmd_display.contains("--full-auto"));
        // Should include model.
        assert!(cmd_display.contains("--model"));
        assert!(cmd_display.contains("o4-mini"));
        // Should include prompt placeholder.
        assert!(cmd_display.contains("<prompt>"));
        // Should include --cd and working directory.
        assert!(cmd_display.contains("--cd"));
        assert!(cmd_display.contains("/home/user/project"));
        // Should NOT include the actual prompt content.
        assert!(!cmd_display.contains("test prompt"));
    }

    #[test]
    fn test_format_command_display_no_model() {
        let runner = CodexRunner::new();
        let prompt = "test";
        let working_dir = Path::new("/tmp");

        let cmd_display = runner.format_command_display(prompt, working_dir).unwrap();

        // Should NOT include model flags.
        assert!(!cmd_display.contains("--model"));
    }

    // ── strip_usage_stats tests ───────────────────────────────────────

    #[test]
    fn test_strip_usage_stats_with_full_json() {
        let json_output = r#"{
            "result": "Hello, world!",
            "usage": {
                "input_tokens": 1234,
                "output_tokens": 56
            }
        }"#;

        let stripped = CodexRunner::strip_usage_stats(json_output);

        assert_eq!(stripped, "Hello, world!");
        assert!(!stripped.contains("usage"));
        assert!(!stripped.contains("input_tokens"));
    }

    #[test]
    fn test_strip_usage_stats_with_plain_text() {
        let plain_text = "This is just plain text output";

        let stripped = CodexRunner::strip_usage_stats(plain_text);

        assert_eq!(stripped, plain_text);
    }

    #[test]
    fn test_strip_usage_stats_preserves_multiline_result() {
        let json_output = r#"{
            "result": "Line 1\nLine 2\nLine 3",
            "usage": {
                "input_tokens": 100,
                "output_tokens": 50
            }
        }"#;

        let stripped = CodexRunner::strip_usage_stats(json_output);

        assert_eq!(stripped, "Line 1\nLine 2\nLine 3");
    }

    #[test]
    fn test_strip_usage_stats_with_empty_result() {
        let json_output = r#"{
            "result": "",
            "usage": {
                "input_tokens": 10
            }
        }"#;

        let stripped = CodexRunner::strip_usage_stats(json_output);

        assert_eq!(stripped, "");
    }

    #[test]
    fn test_strip_usage_stats_missing_result_field() {
        let json_output = r#"{
            "error": "Something went wrong",
            "usage": {
                "input_tokens": 10
            }
        }"#;

        let stripped = CodexRunner::strip_usage_stats(json_output);

        assert_eq!(stripped, json_output);
    }

    // ── Runner trait tests ────────────────────────────────────────────

    #[test]
    fn test_runner_name() {
        let runner = CodexRunner::new();
        assert_eq!(Runner::name(&runner), "codex");
    }

    #[test]
    fn test_runner_default() {
        let runner = CodexRunner::default();
        assert_eq!(Runner::name(&runner), "codex");
    }

    // ── Interactive args tests ────────────────────────────────────────

    #[test]
    fn test_build_interactive_args_yolo_mode() {
        let runner = CodexRunner::new();
        let args = runner.build_interactive_args("discovery prompt").unwrap();

        // Should include prompt text.
        assert!(args.contains(&"discovery prompt".to_string()));

        // Should NOT use exec subcommand.
        assert!(!args.contains(&"exec".to_string()));

        // Should include permission flags.
        assert!(args.contains(&"--full-auto".to_string()));
    }

    #[test]
    fn test_build_interactive_args_with_model() {
        let runner = CodexRunner::with_model(Some("o4-mini".to_string()));
        let args = runner.build_interactive_args("prompt").unwrap();

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"o4-mini".to_string()));
        assert!(args.contains(&"prompt".to_string()));
    }

    #[test]
    fn test_build_interactive_args_manual_mode() {
        let config = CodexConfig::new().with_permission_mode(CodexPermissionMode::Manual);
        let runner = CodexRunner::with_config(config);
        let args = runner.build_interactive_args("prompt").unwrap();

        assert!(args.contains(&"prompt".to_string()));
        assert!(!args.contains(&"--full-auto".to_string()));
    }
}
