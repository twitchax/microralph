//! Copilot CLI runner.
//!
//! This runner shells out to the GitHub Copilot CLI (`copilot`) to execute prompts.
//! It uses `--allow-all` by default for yolo mode (no permission prompts).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use super::types::{Runner, RunnerError, RunnerOutput, RunnerResult};

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
            silent: true,
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
            stdout
        } else if stdout.is_empty() {
            stderr
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        let success = output.status.success();

        tracing::debug!(
            exit_code = ?output.status.code(),
            success = success,
            output_len = combined_output.len(),
            "Copilot CLI completed"
        );

        Ok(RunnerOutput {
            text: combined_output,
            success,
        })
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

        tracing::debug!(
            exit_code = ?status.code(),
            success = success,
            output_len = captured_output.len(),
            "Copilot CLI completed (streaming)"
        );

        Ok(RunnerOutput {
            text: captured_output,
            success,
        })
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
        assert!(config.silent);
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
        assert!(args.contains(&"-s".to_string()));
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
        assert!(args.contains(&"-s".to_string()));
    }

    // Note: Integration tests that actually invoke copilot should be
    // separate and gated behind a feature flag or environment variable,
    // as they require copilot CLI to be installed and authenticated.
}
