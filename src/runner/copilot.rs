//! Copilot CLI runner.
//!
//! This runner shells out to the GitHub Copilot CLI (`copilot`) to execute prompts.
//! It uses `--allow-all` by default for yolo mode (no permission prompts).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use super::types::{Runner, RunnerError, RunnerOutput, RunnerResult, UsageInfo};

/// Permission mode for the Copilot runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
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
    pub permission_mode: PermissionMode,

    /// Whether to use silent mode (-s) for clean output.
    /// When false, enables usage tracking via stats output.
    pub silent: bool,

    /// Whether to disable the ask_user tool.
    pub no_ask_user: bool,

    /// The model to use (e.g., "claude-sonnet-4-20250514").
    pub model: Option<String>,
}

impl Default for CopilotConfig {
    fn default() -> Self {
        Self {
            copilot_path: "copilot".to_string(),
            permission_mode: PermissionMode::Yolo,
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
    pub fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
        self
    }

    /// Sets whether to use silent mode.
    #[cfg(test)]
    pub fn with_silent(mut self, silent: bool) -> Self {
        self.silent = silent;
        self
    }

    /// Sets whether to disable the ask_user tool.
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

    /// Builds the command arguments based on configuration.
    fn build_args(&self, prompt: &str) -> Vec<String> {
        let mut args = Vec::new();

        // Prompt (non-interactive mode).
        args.push("-p".to_string());
        args.push(prompt.to_string());

        // Permission flags.
        match self.config.permission_mode {
            PermissionMode::Yolo => {
                args.push("--allow-all".to_string());
            }
            #[cfg(test)]
            PermissionMode::AllowAll => {
                args.push("--allow-all-tools".to_string());
                args.push("--allow-all-paths".to_string());
                args.push("--allow-all-urls".to_string());
            }
            #[cfg(test)]
            PermissionMode::Manual => {
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

        args
    }

    /// Checks if the copilot CLI is installed and accessible.
    fn check_copilot_available(&self) -> bool {
        Command::new("which")
            .arg(&self.config.copilot_path)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Attempts to parse token usage information from Copilot CLI output.
    ///
    /// Copilot CLI outputs usage information in non-silent mode like:
    /// ```
    /// Breakdown by AI model:
    ///  claude-opus-4.5         18.3k in, 38 out, 0 cached (Est. 3 Premium requests)
    /// ```
    ///
    /// This function parses the actual format emitted by the CLI.
    fn parse_usage(text: &str) -> Option<UsageInfo> {
        let mut input_tokens = None;
        let mut output_tokens = None;

        // Pattern for Copilot CLI format: "18.3k in, 38 out"
        // This matches lines like:
        // " claude-opus-4.5         18.3k in, 38 out, 0 cached (Est. 3 Premium requests)"
        // " gpt-5                   1.2M in, 456 out"
        if let Some(caps) =
            regex::Regex::new(r"(?m)^\s+[\w\-\.]+\s+([\d.]+)([kKmM]?)\s+in,\s+(\d+)\s+out")
                .ok()
                .and_then(|re| re.captures(text))
        {
            // Parse input tokens with possible k/M suffix
            if let (Some(num_match), Some(suffix_match)) = (caps.get(1), caps.get(2))
                && let Ok(num) = num_match.as_str().parse::<f64>()
            {
                let multiplier = match suffix_match.as_str().to_lowercase().as_str() {
                    "k" => 1000.0,
                    "m" => 1_000_000.0,
                    _ => 1.0,
                };
                input_tokens = Some((num * multiplier) as u64);
            }

            // Parse output tokens (no suffix in observed format)
            output_tokens = caps.get(3).and_then(|m| m.as_str().parse().ok());
        }

        // Fallback: Generic patterns for other potential formats
        // Pattern 1: "Token usage: input=123, output=456"
        if input_tokens.is_none()
            && let Some(caps) =
                regex::Regex::new(r"[Tt]oken usage:\s*input[=:\s]+(\d+)[,\s]*output[=:\s]+(\d+)")
                    .ok()
                    .and_then(|re| re.captures(text))
        {
            input_tokens = caps.get(1).and_then(|m| m.as_str().parse().ok());
            output_tokens = caps.get(2).and_then(|m| m.as_str().parse().ok());
        }

        // Pattern 2: "Input tokens: 123" and "Output tokens: 456" (separate lines)
        if input_tokens.is_none()
            && let Some(caps) = regex::Regex::new(r"[Ii]nput tokens[=:\s]+(\d+)")
                .ok()
                .and_then(|re| re.captures(text))
        {
            input_tokens = caps.get(1).and_then(|m| m.as_str().parse().ok());
        }

        if output_tokens.is_none()
            && let Some(caps) = regex::Regex::new(r"[Oo]utput tokens[=:\s]+(\d+)")
                .ok()
                .and_then(|re| re.captures(text))
        {
            output_tokens = caps.get(2).and_then(|m| m.as_str().parse().ok());
        }

        // Compute total if we have both input and output
        let total_tokens = match (input_tokens, output_tokens) {
            (Some(i), Some(o)) => Some(i + o),
            _ => None,
        };

        // Only return UsageInfo if we found at least one piece of information.
        if input_tokens.is_some() || output_tokens.is_some() || total_tokens.is_some() {
            Some(UsageInfo {
                input_tokens,
                output_tokens,
                total_tokens,
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
    fn strip_stats(text: &str) -> String {
        // Find the start of the stats section
        // It typically starts with "Total usage est:" or "API time spent:"
        if let Some(pos) = text.find("\n\nTotal usage est:") {
            text[..pos].to_string()
        } else if let Some(pos) = text.find("\n\nAPI time spent:") {
            text[..pos].to_string()
        } else if let Some(pos) = text.find("\n\nBreakdown by AI model:") {
            text[..pos].to_string()
        } else {
            // No stats section found, return as-is
            text.to_string()
        }
    }
}

impl Default for CopilotRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner for CopilotRunner {
    fn name(&self) -> &str {
        "copilot"
    }

    fn execute(&self, prompt: &str, working_dir: &Path) -> RunnerResult<RunnerOutput> {
        let args = self.build_args(prompt);

        tracing::debug!(
            copilot_path = %self.config.copilot_path,
            working_dir = %working_dir.display(),
            args = ?args,
            "Executing copilot CLI"
        );

        let mut command = Command::new(&self.config.copilot_path);

        command.args(&args).current_dir(working_dir);

        let output = command.output().map_err(|e| {
            RunnerError::ProcessFailed(format!(
                "Failed to start copilot CLI at '{}': {}",
                self.config.copilot_path, e
            ))
        })?;

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        let combined_output = if stderr.is_empty() {
            stdout.clone()
        } else if stdout.is_empty() {
            stderr.clone()
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        // Try to parse usage information from combined output (stats are in stdout when not silent).
        let usage = Self::parse_usage(&combined_output);

        // Strip stats section from output to keep it clean
        let cleaned_output = Self::strip_stats(&combined_output);

        let success = output.status.success();

        tracing::debug!(
            exit_code = ?output.status.code(),
            success = success,
            output_len = cleaned_output.len(),
            usage_present = usage.is_some(),
            "Copilot CLI completed"
        );

        let mut runner_output = RunnerOutput {
            text: cleaned_output,
            success,
            usage: None,
        };

        if let Some(usage_info) = usage {
            runner_output = runner_output.with_usage(usage_info);
        }

        Ok(runner_output)
    }

    fn execute_streaming(
        &self,
        prompt: &str,
        working_dir: &Path,
        output: &mut dyn Write,
    ) -> RunnerResult<RunnerOutput> {
        let args = self.build_args(prompt);

        tracing::debug!(
            copilot_path = %self.config.copilot_path,
            working_dir = %working_dir.display(),
            args = ?args,
            "Executing copilot CLI (streaming)"
        );

        let mut command = Command::new(&self.config.copilot_path);

        command
            .args(&args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|e| {
            RunnerError::ProcessFailed(format!(
                "Failed to start copilot CLI at '{}': {}",
                self.config.copilot_path, e
            ))
        })?;

        let mut captured_output = String::new();

        // Stream stdout in real-time.
        if let Some(stdout) = child.stdout.take() {
            let reader = BufReader::new(stdout);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        // Write to the output stream.
                        let _ = writeln!(output, "{}", line);
                        let _ = output.flush();

                        // Capture for return value.
                        if !captured_output.is_empty() {
                            captured_output.push('\n');
                        }

                        captured_output.push_str(&line);
                    }
                    Err(e) => {
                        tracing::warn!("Error reading copilot stdout: {}", e);
                        break;
                    }
                }
            }
        }

        // Capture any stderr after stdout is done.
        if let Some(stderr) = child.stderr.take() {
            let reader = BufReader::new(stderr);

            for line in reader.lines() {
                match line {
                    Ok(line) => {
                        // Write stderr to output stream as well.
                        let _ = writeln!(output, "{}", line);
                        let _ = output.flush();

                        if !captured_output.is_empty() {
                            captured_output.push('\n');
                        }

                        captured_output.push_str(&line);
                    }
                    Err(e) => {
                        tracing::warn!("Error reading copilot stderr: {}", e);
                        break;
                    }
                }
            }
        }

        // Wait for the process to complete.
        let status = child.wait().map_err(|e| {
            RunnerError::ProcessFailed(format!("Failed to wait for copilot CLI: {}", e))
        })?;

        let success = status.success();

        // Try to parse usage information from captured output.
        let usage = Self::parse_usage(&captured_output);

        // Strip stats section from output to keep it clean
        let cleaned_output = Self::strip_stats(&captured_output);

        tracing::debug!(
            exit_code = ?status.code(),
            success = success,
            output_len = cleaned_output.len(),
            usage_present = usage.is_some(),
            "Copilot CLI completed (streaming)"
        );

        let mut runner_output = RunnerOutput {
            text: cleaned_output,
            success,
            usage: None,
        };

        if let Some(usage_info) = usage {
            runner_output = runner_output.with_usage(usage_info);
        }

        Ok(runner_output)
    }

    fn is_available(&self) -> bool {
        self.check_copilot_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copilot_config_default() {
        let config = CopilotConfig::default();

        assert_eq!(config.copilot_path, "copilot");
        assert_eq!(config.permission_mode, PermissionMode::Yolo);
        assert!(!config.silent); // Silent mode disabled by default to enable usage tracking
        assert!(config.no_ask_user);
    }

    #[test]
    fn test_copilot_config_builder() {
        let config = CopilotConfig::new()
            .with_path("/custom/path/copilot")
            .with_permission_mode(PermissionMode::AllowAll)
            .with_silent(false)
            .with_no_ask_user(false);

        assert_eq!(config.copilot_path, "/custom/path/copilot");
        assert_eq!(config.permission_mode, PermissionMode::AllowAll);
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
        let config = CopilotConfig::new().with_permission_mode(PermissionMode::AllowAll);
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
            .with_permission_mode(PermissionMode::Manual)
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
        assert_eq!(runner.name(), "copilot");
    }

    #[test]
    fn test_runner_default() {
        let runner = CopilotRunner::default();
        assert_eq!(runner.name(), "copilot");
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

        let usage = CopilotRunner::parse_usage(output).expect("Should parse usage");

        assert_eq!(usage.input_tokens, Some(18300));
        assert_eq!(usage.output_tokens, Some(38));
        assert_eq!(usage.total_tokens, Some(18338));
    }

    #[test]
    fn test_parse_usage_copilot_format_megabytes() {
        let output = "Breakdown by AI model:\n gpt-5                   1.2M in, 456 out";

        let usage = CopilotRunner::parse_usage(output).expect("Should parse usage");

        assert_eq!(usage.input_tokens, Some(1_200_000));
        assert_eq!(usage.output_tokens, Some(456));
    }

    #[test]
    fn test_parse_usage_no_stats() {
        let output = "Hello world\nThis is just normal output.";

        let usage = CopilotRunner::parse_usage(output);

        assert!(usage.is_none());
    }

    #[test]
    fn test_strip_stats() {
        let output = "Hello world\n\nTotal usage est:        3 Premium requests\nAPI time spent:         2s\nTotal session time:     4s\nTotal code changes:     +0 -0\nBreakdown by AI model:\n claude-opus-4.5         18.3k in, 38 out, 11.8k cached (Est. 3 Premium requests)";

        let cleaned = CopilotRunner::strip_stats(output);

        assert_eq!(cleaned, "Hello world");
    }

    #[test]
    fn test_strip_stats_no_stats() {
        let output = "Hello world\nThis is just normal output.";

        let cleaned = CopilotRunner::strip_stats(output);

        assert_eq!(cleaned, output);
    }

    // Note: Integration tests that actually invoke copilot should be
    // separate and gated behind a feature flag or environment variable,
    // as they require copilot CLI to be installed and authenticated.
}
