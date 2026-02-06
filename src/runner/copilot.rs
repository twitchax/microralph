//! Copilot CLI runner.
//!
//! This runner shells out to the GitHub Copilot CLI (`copilot`) to execute prompts.
//! It uses `--allow-all` by default for yolo mode (no permission prompts).
//!
//! The `Runner` trait is automatically implemented via the blanket impl in `cli_runner`
//! for all types that implement `CliRunnerConfig`.

use std::path::Path;

use super::cli_runner::CliRunnerConfig;
use super::types::TokenUsageInfo;

/// Copilot-specific permission mode with additional granular options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CopilotPermissionMode {
    /// Allow all permissions (--allow-all).
    #[default]
    Yolo,

    /// Use individual allow flags.
    #[cfg(test)]
    AllowAll,

    /// No special permission flags (will prompt for permissions).
    #[cfg(test)]
    Manual,
}

/// Configuration for the Copilot runner.
#[derive(Debug, Clone)]
pub struct CopilotConfig {
    /// The path to the copilot CLI binary.
    pub copilot_path: String,

    /// Permission mode.
    pub permission_mode: CopilotPermissionMode,

    /// Whether to use silent mode (-s) for clean output.
    /// When false, enables usage tracking via stats output.
    pub silent: bool,

    /// Whether to disable the `ask_user` tool.
    pub no_ask_user: bool,

    /// The model to use (e.g., "claude-sonnet-4.5").
    pub model: Option<String>,
}

impl Default for CopilotConfig {
    fn default() -> Self {
        Self {
            copilot_path: "copilot".to_string(),
            permission_mode: CopilotPermissionMode::Yolo,
            silent: false, // Disable silent mode to get usage stats
            no_ask_user: true,
            model: None,
        }
    }
}

impl CopilotConfig {
    /// Creates a new config with the default copilot path.
    #[cfg(test)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the path to the copilot binary.
    #[cfg(test)]
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.copilot_path = path.into();
        self
    }

    /// Sets the permission mode.
    #[cfg(test)]
    pub fn with_permission_mode(mut self, mode: CopilotPermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Sets whether to use silent mode.
    #[cfg(test)]
    pub fn with_silent(mut self, silent: bool) -> Self {
        self.silent = silent;
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

/// A runner that shells out to the GitHub Copilot CLI.
///
/// This runner invokes `copilot -p "<prompt>"` with appropriate permission flags.
/// By default, it uses `--allow-all` (yolo mode) to avoid permission prompts.
#[derive(Debug)]
pub struct CopilotRunner {
    config: CopilotConfig,
}

impl CopilotRunner {
    /// Creates a new Copilot runner with default configuration.
    pub fn new() -> Self {
        Self {
            config: CopilotConfig::default(),
        }
    }

    /// Creates a new Copilot runner with the specified model.
    pub fn with_model(model: Option<String>) -> Self {
        Self {
            config: CopilotConfig::default().with_model(model),
        }
    }

    /// Creates a new Copilot runner with the given configuration.
    #[cfg(test)]
    pub fn with_config(config: CopilotConfig) -> Self {
        Self { config }
    }

    /// Appends common configuration flags to the provided args vector.
    ///
    /// This is the single source of truth for flag generation, used by both
    /// `build_args_impl` and `format_display_parts_impl` to avoid duplication.
    fn append_config_flags(&self, args: &mut Vec<String>) {
        // Permission flags.
        match self.config.permission_mode {
            CopilotPermissionMode::Yolo => {
                args.push("--allow-all".to_string());
            }
            #[cfg(test)]
            CopilotPermissionMode::AllowAll => {
                args.push("--allow-all-tools".to_string());
                args.push("--allow-all-paths".to_string());
                args.push("--allow-all-urls".to_string());
            }
            #[cfg(test)]
            CopilotPermissionMode::Manual => {
                // No permission flags.
            }
        }

        // Silent mode for clean output.
        if self.config.silent {
            args.push("-s".to_string());
        }

        // Disable ask_user tool for autonomous operation.
        if self.config.no_ask_user {
            args.push("--no-ask-user".to_string());
        }

        // Model selection.
        if let Some(ref model) = self.config.model {
            args.push("--model".to_string());
            args.push(model.clone());
        }
    }

    /// Builds the command arguments based on configuration.
    fn build_args_impl(&self, prompt: &str) -> Vec<String> {
        let mut args = Vec::new();

        // Prompt (non-interactive mode).
        args.push("-p".to_string());
        args.push(prompt.to_string());

        self.append_config_flags(&mut args);

        args
    }

    /// Builds display parts for [`format_command_display`].
    fn format_display_parts_impl(&self, working_dir: &Path) -> Vec<String> {
        let mut parts = vec![self.config.copilot_path.clone()];

        self.append_config_flags(&mut parts);

        parts.push("-p".to_string());
        parts.push("<prompt>".to_string());
        parts.push("--working-dir".to_string());
        parts.push(working_dir.display().to_string());

        parts
    }

    /// Attempts to parse token usage information from Copilot CLI output.
    ///
    /// Copilot CLI outputs usage information in non-silent mode like:
    /// ```
    /// Breakdown by AI model:
    ///  claude-opus-4.5         18.3k in, 38 out, 0 cached (Est. 3 Premium requests)
    /// ```
    fn parse_usage_impl(text: &str) -> Option<TokenUsageInfo> {
        let mut input = None;
        let mut output = None;

        // Pattern for Copilot CLI format: "18.3k in, 38 out"
        if let Some(caps) =
            regex::Regex::new(r"(?m)^\s+[\w\-\.]+\s+([\d.]+)([kKmM]?)\s+in,\s+(\d+)\s+out")
                .ok()
                .and_then(|re| re.captures(text))
        {
            if let (Some(num_match), Some(suffix_match)) = (caps.get(1), caps.get(2))
                && let Ok(num) = num_match.as_str().parse::<f64>()
            {
                let multiplier = match suffix_match.as_str().to_lowercase().as_str() {
                    "k" => 1000.0,
                    "m" => 1_000_000.0,
                    _ => 1.0,
                };

                // Token counts are always non-negative and within u64 range.
                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                let tokens = (num * multiplier) as u64;
                input = Some(tokens);
            }

            output = caps.get(3).and_then(|m| m.as_str().parse().ok());
        }

        // Fallback patterns
        if input.is_none()
            && let Some(caps) =
                regex::Regex::new(r"[Tt]oken usage:\s*input[=:\s]+(\d+)[,\s]*output[=:\s]+(\d+)")
                    .ok()
                    .and_then(|re| re.captures(text))
        {
            input = caps.get(1).and_then(|m| m.as_str().parse().ok());
            output = caps.get(2).and_then(|m| m.as_str().parse().ok());
        }

        if input.is_none()
            && let Some(caps) = regex::Regex::new(r"[Ii]nput tokens[=:\s]+(\d+)")
                .ok()
                .and_then(|re| re.captures(text))
        {
            input = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }

        if output.is_none()
            && let Some(caps) = regex::Regex::new(r"[Oo]utput tokens[=:\s]+(\d+)")
                .ok()
                .and_then(|re| re.captures(text))
        {
            output = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }

        let total = match (input, output) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };

        if input.is_some() || output.is_some() || total.is_some() {
            Some(TokenUsageInfo {
                input,
                output,
                total,
            })
        } else {
            None
        }
    }

    /// Strips the statistics section from Copilot CLI output.
    ///
    /// When not in silent mode, Copilot CLI appends statistics like:
    /// ```
    /// Total usage est:        3 Premium requests
    /// API time spent:         2s
    /// Total session time:     4s
    /// Total code changes:     +0 -0
    /// Breakdown by AI model:
    ///  claude-opus-4.5         18.3k in, 38 out, 0 cached (Est. 3 Premium requests)
    /// ```
    ///
    /// This function removes that section while preserving the actual response.
    /// It's useful when runner output will be written to files where stats shouldn't appear.
    pub fn strip_usage_stats(text: &str) -> String {
        // Find the start of the stats section
        // It typically starts with "Total usage est:" or "API time spent:"
        // Try to find stats sections in order of preference
        text.find("\n\nTotal usage est:")
            .or_else(|| text.find("\n\nAPI time spent:"))
            .or_else(|| text.find("\n\nBreakdown by AI model:"))
            .map_or_else(|| text.to_string(), |pos| text[..pos].to_string())
    }
}

impl Default for CopilotRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CliRunnerConfig for CopilotRunner {
    fn name(&self) -> &'static str {
        "copilot"
    }

    fn binary_path(&self) -> &str {
        &self.config.copilot_path
    }

    fn build_args(&self, prompt: &str) -> Vec<String> {
        self.build_args_impl(prompt)
    }

    fn parse_usage(&self, text: &str) -> Option<TokenUsageInfo> {
        Self::parse_usage_impl(text)
    }

    fn format_display_parts(&self, working_dir: &Path) -> Vec<String> {
        self.format_display_parts_impl(working_dir)
    }

    fn build_interactive_args(&self, prompt: &str) -> Option<Vec<String>> {
        let mut args = Vec::new();

        // Interactive mode with initial prompt.
        args.push("-i".to_string());
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
    fn test_copilot_config_default() {
        let config = CopilotConfig::default();

        assert_eq!(config.copilot_path, "copilot");
        assert_eq!(config.permission_mode, CopilotPermissionMode::Yolo);
        assert!(!config.silent); // Silent mode disabled by default to enable usage tracking
        assert!(config.no_ask_user);
    }

    #[test]
    fn test_copilot_config_builder() {
        let config = CopilotConfig::new()
            .with_path("/custom/path/copilot")
            .with_permission_mode(CopilotPermissionMode::AllowAll)
            .with_silent(false)
            .with_no_ask_user(false);

        assert_eq!(config.copilot_path, "/custom/path/copilot");
        assert_eq!(config.permission_mode, CopilotPermissionMode::AllowAll);
        assert!(!config.silent);
        assert!(!config.no_ask_user);
    }

    #[test]
    fn test_build_args_yolo_mode() {
        let runner = CopilotRunner::new();
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(args.contains(&"--allow-all".to_string()));
        assert!(!args.contains(&"-s".to_string())); // Silent mode disabled for usage tracking
        assert!(args.contains(&"--no-ask-user".to_string()));
    }

    #[test]
    fn test_build_args_allow_all_mode() {
        let config = CopilotConfig::new().with_permission_mode(CopilotPermissionMode::AllowAll);
        let runner = CopilotRunner::with_config(config);
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"--allow-all-tools".to_string()));
        assert!(args.contains(&"--allow-all-paths".to_string()));
        assert!(args.contains(&"--allow-all-urls".to_string()));
        assert!(!args.contains(&"--allow-all".to_string()));
    }

    #[test]
    fn test_build_args_manual_mode() {
        let config = CopilotConfig::new()
            .with_permission_mode(CopilotPermissionMode::Manual)
            .with_silent(false)
            .with_no_ask_user(false);
        let runner = CopilotRunner::with_config(config);
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(!args.contains(&"--allow-all".to_string()));
        assert!(!args.contains(&"--allow-all-tools".to_string()));
        assert!(!args.contains(&"-s".to_string()));
        assert!(!args.contains(&"--no-ask-user".to_string()));
    }

    #[test]
    fn test_runner_name() {
        let runner = CopilotRunner::new();
        assert_eq!(Runner::name(&runner), "copilot");
    }

    #[test]
    fn test_runner_default() {
        let runner = CopilotRunner::default();
        assert_eq!(Runner::name(&runner), "copilot");
    }

    #[test]
    fn test_build_args_with_model() {
        let runner = CopilotRunner::with_model(Some("claude-sonnet-4".to_string()));
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-sonnet-4".to_string()));
    }

    #[test]
    fn test_build_args_without_model() {
        let runner = CopilotRunner::new();
        let args = runner.build_args("test prompt");

        assert!(!args.contains(&"--model".to_string()));
    }

    #[test]
    fn test_with_model_constructor() {
        let runner = CopilotRunner::with_model(Some("gpt-4o".to_string()));
        let args = runner.build_args("prompt");

        // Should have model flag.
        let model_idx = args.iter().position(|a| a == "--model").unwrap();
        assert_eq!(args[model_idx + 1], "gpt-4o");

        // Should still have other default flags.
        assert!(args.contains(&"--allow-all".to_string()));
        assert!(!args.contains(&"-s".to_string())); // Silent mode disabled for usage tracking
    }

    #[test]
    fn test_parse_usage_copilot_format() {
        let output = "Hello world\n\nTotal usage est:        3 Premium requests\nAPI time spent:         2s\nTotal session time:     4s\nTotal code changes:     +0 -0\nBreakdown by AI model:\n claude-opus-4.5         18.3k in, 38 out, 11.8k cached (Est. 3 Premium requests)";

        let usage = CopilotRunner::parse_usage_impl(output).expect("Should parse usage");

        assert_eq!(usage.input, Some(18300));
        assert_eq!(usage.output, Some(38));
        assert_eq!(usage.total, Some(18338));
    }

    #[test]
    fn test_parse_usage_copilot_format_megabytes() {
        let output = "Breakdown by AI model:\n gpt-5                   1.2M in, 456 out";

        let usage = CopilotRunner::parse_usage_impl(output).expect("Should parse usage");

        assert_eq!(usage.input, Some(1_200_000));
        assert_eq!(usage.output, Some(456));
    }

    #[test]
    fn test_parse_usage_no_stats() {
        let output = "Hello world\nThis is just normal output.";

        let usage = CopilotRunner::parse_usage_impl(output);

        assert!(usage.is_none());
    }

    #[test]
    fn test_format_command_display() {
        let runner = CopilotRunner::with_model(Some("claude-sonnet-4".to_string()));
        let prompt = "test prompt";
        let working_dir = Path::new("/home/user/project");

        let cmd_display = runner.format_command_display(prompt, working_dir).unwrap();

        // Should include copilot path
        assert!(cmd_display.contains("copilot"));
        // Should include permission flags
        assert!(cmd_display.contains("--allow-all"));
        // Should include no-ask-user flag
        assert!(cmd_display.contains("--no-ask-user"));
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
        let runner = CopilotRunner::new();
        let prompt = "test";
        let working_dir = Path::new("/tmp");

        let cmd_display = runner.format_command_display(prompt, working_dir).unwrap();

        // Should NOT include model flags
        assert!(!cmd_display.contains("--model"));
    }

    #[test]
    fn test_strip_usage_stats() {
        let output = "Hello world\n\nTotal usage est:        3 Premium requests\nAPI time spent:         2s\nTotal session time:     4s\nTotal code changes:     +0 -0\nBreakdown by AI model:\n claude-opus-4.5         18.3k in, 38 out, 11.8k cached (Est. 3 Premium requests)";

        let cleaned = CopilotRunner::strip_usage_stats(output);

        assert_eq!(cleaned, "Hello world");
    }

    #[test]
    fn test_strip_usage_stats_no_stats() {
        let output = "Hello world\nThis is just normal output.";

        let cleaned = CopilotRunner::strip_usage_stats(output);

        assert_eq!(cleaned, output);
    }

    #[test]
    fn test_build_interactive_args_yolo_mode() {
        let runner = CopilotRunner::new();
        let args = runner.build_interactive_args("discovery prompt").unwrap();

        // Should use -i for interactive mode.
        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"discovery prompt".to_string()));

        // Should NOT use -p (non-interactive).
        assert!(!args.contains(&"-p".to_string()));

        // Should include config flags.
        assert!(args.contains(&"--allow-all".to_string()));
        assert!(args.contains(&"--no-ask-user".to_string()));
    }

    #[test]
    fn test_build_interactive_args_with_model() {
        let runner = CopilotRunner::with_model(Some("claude-sonnet-4".to_string()));
        let args = runner.build_interactive_args("prompt").unwrap();

        assert!(args.contains(&"-i".to_string()));
        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-sonnet-4".to_string()));
    }

    #[test]
    fn test_build_interactive_args_manual_mode() {
        let config = CopilotConfig::new()
            .with_permission_mode(CopilotPermissionMode::Manual)
            .with_no_ask_user(false);
        let runner = CopilotRunner::with_config(config);
        let args = runner.build_interactive_args("prompt").unwrap();

        assert!(args.contains(&"-i".to_string()));
        assert!(!args.contains(&"--allow-all".to_string()));
        assert!(!args.contains(&"--no-ask-user".to_string()));
    }

    #[test]
    fn test_build_continue_args_returns_none() {
        // Copilot does not support session resume.
        let runner = CopilotRunner::new();
        assert!(runner.build_continue_args("prompt").is_none());
    }

    // Note: Integration tests that actually invoke copilot should be
    // separate and gated behind a feature flag or environment variable,
    // as they require copilot CLI to be installed and authenticated.
}
