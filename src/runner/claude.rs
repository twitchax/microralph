//! Claude CLI runner.
//!
//! This runner shells out to the Claude CLI (`claude`) to execute prompts.
//! It uses `--dangerously-skip-permissions` by default for yolo mode (no permission prompts).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use super::types::{Runner, RunnerError, RunnerOutput, RunnerResult, UsageInfo};

/// Permission mode for the Claude runner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PermissionMode {
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
    pub permission_mode: PermissionMode,

    /// Whether to disable the ask_user tool.
    pub no_ask_user: bool,

    /// The model to use (e.g., "claude-sonnet-4-20250514").
    pub model: Option<String>,
}

impl Default for ClaudeConfig {
    fn default() -> Self {
        Self {
            claude_path: "claude".to_string(),
            permission_mode: PermissionMode::Yolo,
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
    pub fn with_permission_mode(mut self, mode: PermissionMode) -> Self {
        self.permission_mode = mode;
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

    /// Builds the command arguments based on configuration.
    fn build_args(&self, prompt: &str) -> Vec<String> {
        let mut args = Vec::new();

        // Prompt (non-interactive mode).
        args.push("-p".to_string());
        args.push(prompt.to_string());

        // Permission flags.
        match self.config.permission_mode {
            PermissionMode::Yolo => {
                args.push("--dangerously-skip-permissions".to_string());
            }
            #[cfg(test)]
            PermissionMode::Manual => {
                // No permission flags.
            }
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

    /// Checks if the claude CLI is installed and accessible.
    fn check_claude_available(&self) -> bool {
        Command::new("which")
            .arg(&self.config.claude_path)
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
    }

    /// Attempts to parse token usage information from Claude CLI output.
    ///
    /// Note: As of early 2025, Claude CLI does not provide built-in token usage
    /// statistics in its stdout output like Copilot CLI does. This function is
    /// provided for future compatibility if the CLI adds this feature.
    ///
    /// For now, it returns None. External tools like `ccusage` can be used for
    /// token tracking.
    fn parse_usage(_text: &str) -> Option<UsageInfo> {
        // Claude CLI does not currently output token usage statistics directly.
        // If this changes in the future, we can add parsing logic here.
        None
    }
}

impl Default for ClaudeRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl Runner for ClaudeRunner {
    fn name(&self) -> &str {
        "claude"
    }

    fn format_command_display(&self, _prompt: &str, working_dir: &Path) -> Option<String> {
        let mut parts = vec![self.config.claude_path.clone()];

        // Add permission flags
        match self.config.permission_mode {
            PermissionMode::Yolo => {
                parts.push("--dangerously-skip-permissions".to_string());
            }
            #[cfg(test)]
            PermissionMode::Manual => {}
        }

        // Add no-ask-user flag
        if self.config.no_ask_user {
            parts.push("--no-ask-user".to_string());
        }

        // Add model flag
        if let Some(ref model) = self.config.model {
            parts.push("--model".to_string());
            parts.push(model.clone());
        }

        // Add working directory info
        parts.push("-p".to_string());
        parts.push("<prompt>".to_string());
        parts.push("--working-dir".to_string());
        parts.push(working_dir.display().to_string());

        Some(parts.join(" "))
    }

    fn execute(&self, prompt: &str, working_dir: &Path) -> RunnerResult<RunnerOutput> {
        let args = self.build_args(prompt);

        tracing::debug!(
            claude_path = %self.config.claude_path,
            working_dir = %working_dir.display(),
            args = ?args,
            "Executing claude CLI"
        );

        let mut command = Command::new(&self.config.claude_path);

        command.args(&args).current_dir(working_dir);

        let output = command.output().map_err(|e| {
            RunnerError::ProcessFailed(format!(
                "Failed to start claude CLI at '{}': {}",
                self.config.claude_path, e
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

        // Try to parse usage information (currently returns None for Claude CLI).
        let usage = Self::parse_usage(&combined_output);

        let success = output.status.success();

        tracing::debug!(
            exit_code = ?output.status.code(),
            success = success,
            output_len = combined_output.len(),
            usage_present = usage.is_some(),
            "Claude CLI completed"
        );

        let mut runner_output = RunnerOutput {
            text: combined_output,
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
            claude_path = %self.config.claude_path,
            working_dir = %working_dir.display(),
            args = ?args,
            "Executing claude CLI (streaming)"
        );

        let mut command = Command::new(&self.config.claude_path);

        command
            .args(&args)
            .current_dir(working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = command.spawn().map_err(|e| {
            RunnerError::ProcessFailed(format!(
                "Failed to start claude CLI at '{}': {}",
                self.config.claude_path, e
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
                        tracing::warn!("Error reading claude stdout: {}", e);
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
                        tracing::warn!("Error reading claude stderr: {}", e);
                        break;
                    }
                }
            }
        }

        // Wait for the process to complete.
        let status = child.wait().map_err(|e| {
            RunnerError::ProcessFailed(format!("Failed to wait for claude CLI: {}", e))
        })?;

        let success = status.success();

        // Try to parse usage information (currently returns None for Claude CLI).
        let usage = Self::parse_usage(&captured_output);

        tracing::debug!(
            exit_code = ?status.code(),
            success = success,
            output_len = captured_output.len(),
            usage_present = usage.is_some(),
            "Claude CLI completed (streaming)"
        );

        let mut runner_output = RunnerOutput {
            text: captured_output,
            success,
            usage: None,
        };

        if let Some(usage_info) = usage {
            runner_output = runner_output.with_usage(usage_info);
        }

        Ok(runner_output)
    }

    fn is_available(&self) -> bool {
        self.check_claude_available()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_claude_config_default() {
        let config = ClaudeConfig::default();

        assert_eq!(config.claude_path, "claude");
        assert_eq!(config.permission_mode, PermissionMode::Yolo);
        assert!(config.no_ask_user);
    }

    #[test]
    fn test_claude_config_builder() {
        let config = ClaudeConfig::new()
            .with_path("/custom/path/claude")
            .with_permission_mode(PermissionMode::Manual)
            .with_no_ask_user(false);

        assert_eq!(config.claude_path, "/custom/path/claude");
        assert_eq!(config.permission_mode, PermissionMode::Manual);
        assert!(!config.no_ask_user);
    }

    #[test]
    fn test_build_args_yolo_mode() {
        let runner = ClaudeRunner::new();
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(args.contains(&"--no-ask-user".to_string()));
    }

    #[test]
    fn test_build_args_manual_mode() {
        let config = ClaudeConfig::new()
            .with_permission_mode(PermissionMode::Manual)
            .with_no_ask_user(false);
        let runner = ClaudeRunner::with_config(config);
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"-p".to_string()));
        assert!(args.contains(&"test prompt".to_string()));
        assert!(!args.contains(&"--dangerously-skip-permissions".to_string()));
        assert!(!args.contains(&"--no-ask-user".to_string()));
    }

    #[test]
    fn test_runner_name() {
        let runner = ClaudeRunner::new();
        assert_eq!(runner.name(), "claude");
    }

    #[test]
    fn test_runner_default() {
        let runner = ClaudeRunner::default();
        assert_eq!(runner.name(), "claude");
    }

    #[test]
    fn test_build_args_with_model() {
        let runner = ClaudeRunner::with_model(Some("claude-sonnet-4-20250514".to_string()));
        let args = runner.build_args("test prompt");

        assert!(args.contains(&"--model".to_string()));
        assert!(args.contains(&"claude-sonnet-4-20250514".to_string()));
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
    fn test_format_command_display() {
        let runner = ClaudeRunner::with_model(Some("claude-sonnet-4".to_string()));
        let prompt = "test prompt";
        let working_dir = Path::new("/home/user/project");

        let cmd_display = runner.format_command_display(prompt, working_dir).unwrap();

        // Should include claude path
        assert!(cmd_display.contains("claude"));
        // Should include permission flags
        assert!(cmd_display.contains("--dangerously-skip-permissions"));
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
        let runner = ClaudeRunner::new();
        let prompt = "test";
        let working_dir = Path::new("/tmp");

        let cmd_display = runner.format_command_display(prompt, working_dir).unwrap();

        // Should NOT include model flags
        assert!(!cmd_display.contains("--model"));
    }
}
