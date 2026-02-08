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
    use super::*;

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
}
