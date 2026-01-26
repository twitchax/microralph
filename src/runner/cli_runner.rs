//! Shared CLI runner infrastructure.
//!
//! This module provides common functionality for CLI-based runners (Copilot, Claude, etc.)
//! to reduce code duplication.

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use super::types::{RunnerError, RunnerOutput, RunnerResult, UsageInfo};

/// Trait for CLI-based runners that share common execution logic.
///
/// Implementors only need to provide CLI-specific details (binary path, args, parsing),
/// while the shared infrastructure handles command execution and output capture.
pub trait CliRunnerConfig {
    /// Returns the name of this runner (e.g., "copilot", "claude").
    fn name(&self) -> &str;

    /// Returns the path to the CLI binary.
    fn binary_path(&self) -> &str;

    /// Builds the command arguments for a given prompt.
    fn build_args(&self, prompt: &str) -> Vec<String>;

    /// Parses token usage information from CLI output.
    /// Returns None if usage info cannot be parsed.
    fn parse_usage(&self, text: &str) -> Option<UsageInfo>;

    /// Post-processes the output text (e.g., extracting result from JSON).
    /// Default implementation returns text as-is.
    fn post_process_output(&self, text: &str) -> String {
        text.to_string()
    }

    /// Returns additional display information for format_command_display.
    /// Default implementation returns base format.
    fn format_display_parts(&self, working_dir: &Path) -> Vec<String> {
        vec![
            self.binary_path().to_string(),
            "-p".to_string(),
            "<prompt>".to_string(),
            "--working-dir".to_string(),
            working_dir.display().to_string(),
        ]
    }
}

/// Checks if a CLI binary is available on the system.
pub fn check_cli_available(binary_path: &str) -> bool {
    Command::new("which")
        .arg(binary_path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Formats the command display for logging/user feedback.
pub fn format_command_display<C: CliRunnerConfig + ?Sized>(
    config: &C,
    working_dir: &Path,
) -> String {
    config.format_display_parts(working_dir).join(" ")
}

/// Executes a CLI command and captures the output.
///
/// This is the non-streaming version that waits for the command to complete.
/// Note: Does NOT print command info to stdout (caller shows spinner instead).
pub fn execute_cli<C: CliRunnerConfig + ?Sized>(
    config: &C,
    prompt: &str,
    working_dir: &Path,
) -> RunnerResult<RunnerOutput> {
    let args = config.build_args(prompt);

    tracing::debug!(
        binary = %config.binary_path(),
        working_dir = %working_dir.display(),
        args = ?args,
        "Executing CLI"
    );

    let mut command = Command::new(config.binary_path());
    command.args(&args).current_dir(working_dir);

    let output = command.output().map_err(|e| {
        RunnerError::ProcessFailed(format!(
            "Failed to start CLI at '{}': {}",
            config.binary_path(),
            e
        ))
    })?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    let combined_output = if stderr.is_empty() {
        stdout.to_string()
    } else if stdout.is_empty() {
        stderr.to_string()
    } else {
        format!("{}\n{}", stdout, stderr)
    };

    // Parse usage information and post-process output
    let usage = config.parse_usage(&combined_output);
    let processed_output = config.post_process_output(&combined_output);

    let success = output.status.success();

    tracing::debug!(
        exit_code = ?output.status.code(),
        success = success,
        raw_len = combined_output.len(),
        processed_len = processed_output.len(),
        usage_present = usage.is_some(),
        "CLI completed"
    );

    let mut runner_output = RunnerOutput {
        text: processed_output,
        success,
        usage: None,
    };

    if let Some(usage_info) = usage {
        runner_output = runner_output.with_usage(usage_info);
    }

    Ok(runner_output)
}

/// Executes a CLI command with real-time output streaming.
///
/// Output is written to the provided writer as it becomes available.
pub fn execute_cli_streaming<C: CliRunnerConfig + ?Sized>(
    config: &C,
    prompt: &str,
    working_dir: &Path,
    output: &mut dyn Write,
) -> RunnerResult<RunnerOutput> {
    // Display the command being invoked
    let cmd_display = format_command_display(config, working_dir);
    println!("\n🔧 Executing: {}", cmd_display);

    let args = config.build_args(prompt);

    tracing::debug!(
        binary = %config.binary_path(),
        working_dir = %working_dir.display(),
        args = ?args,
        "Executing CLI (streaming)"
    );

    let mut command = Command::new(config.binary_path());
    command
        .args(&args)
        .current_dir(working_dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let mut child = command.spawn().map_err(|e| {
        RunnerError::ProcessFailed(format!(
            "Failed to start CLI at '{}': {}",
            config.binary_path(),
            e
        ))
    })?;

    let mut captured_output = String::new();

    // Stream stdout in real-time
    if let Some(stdout) = child.stdout.take() {
        let reader = BufReader::new(stdout);

        for line in reader.lines() {
            match line {
                Ok(line) => {
                    // Write to the output stream
                    let _ = writeln!(output, "{}", line);
                    let _ = output.flush();

                    // Capture for return value
                    if !captured_output.is_empty() {
                        captured_output.push('\n');
                    }
                    captured_output.push_str(&line);
                }
                Err(e) => {
                    tracing::warn!("Error reading stdout: {}", e);
                    break;
                }
            }
        }
    }

    // Capture any stderr after stdout is done
    if let Some(stderr) = child.stderr.take() {
        let reader = BufReader::new(stderr);

        for line in reader.lines() {
            match line {
                Ok(line) => {
                    // Write stderr to output stream as well
                    let _ = writeln!(output, "{}", line);
                    let _ = output.flush();

                    if !captured_output.is_empty() {
                        captured_output.push('\n');
                    }
                    captured_output.push_str(&line);
                }
                Err(e) => {
                    tracing::warn!("Error reading stderr: {}", e);
                    break;
                }
            }
        }
    }

    // Wait for the process to complete
    let status = child
        .wait()
        .map_err(|e| RunnerError::ProcessFailed(format!("Failed to wait for CLI: {}", e)))?;

    let success = status.success();

    // Parse usage information and post-process output
    let usage = config.parse_usage(&captured_output);
    let processed_output = config.post_process_output(&captured_output);

    tracing::debug!(
        exit_code = ?status.code(),
        success = success,
        raw_len = captured_output.len(),
        processed_len = processed_output.len(),
        usage_present = usage.is_some(),
        "CLI completed (streaming)"
    );

    let mut runner_output = RunnerOutput {
        text: processed_output,
        success,
        usage: None,
    };

    if let Some(usage_info) = usage {
        runner_output = runner_output.with_usage(usage_info);
    }

    Ok(runner_output)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestConfig {
        binary: String,
        args: Vec<String>,
    }

    impl CliRunnerConfig for TestConfig {
        fn name(&self) -> &str {
            "test"
        }

        fn binary_path(&self) -> &str {
            &self.binary
        }

        fn build_args(&self, _prompt: &str) -> Vec<String> {
            self.args.clone()
        }

        fn parse_usage(&self, _text: &str) -> Option<UsageInfo> {
            None
        }
    }

    #[test]
    fn test_check_cli_available_echo() {
        // 'echo' should be available on all Unix systems
        assert!(check_cli_available("echo"));
    }

    #[test]
    fn test_check_cli_available_nonexistent() {
        assert!(!check_cli_available("nonexistent-binary-xyz123"));
    }

    #[test]
    fn test_format_command_display() {
        let config = TestConfig {
            binary: "/path/to/cli".to_string(),
            args: vec![],
        };
        let working_dir = Path::new("/test/dir");

        let display = format_command_display(&config, working_dir);

        assert!(display.contains("/path/to/cli"));
        assert!(display.contains("/test/dir"));
    }

    #[test]
    fn test_execute_cli_echo() {
        let config = TestConfig {
            binary: "echo".to_string(),
            args: vec!["hello world".to_string()],
        };
        let working_dir = Path::new(".");

        let result = execute_cli(&config, "ignored", working_dir).unwrap();

        assert!(result.success);
        assert!(result.text.contains("hello"));
    }
}
