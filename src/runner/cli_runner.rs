//! Shared CLI runner infrastructure.
//!
//! This module provides common functionality for CLI-based runners (Copilot, Claude, etc.)
//! to reduce code duplication.
//!
//! Types implementing `CliRunnerConfig` automatically get a `Runner` implementation
//! via the blanket impl, eliminating boilerplate in individual runner modules.

use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

use super::types::{
    InteractiveResult, Runner, RunnerError, RunnerOutput, RunnerResult, TokenUsageInfo,
};

/// Reads lines from a stream, writes them to output, and captures them.
///
/// This helper function handles the common pattern of reading from a process
/// stream (stdout or stderr), writing each line to an output writer for
/// real-time display, and accumulating the lines into a captured string.
fn read_stream_lines<R: Read>(reader: R, output: &mut dyn Write, captured: &mut String) {
    let buf_reader = BufReader::new(reader);

    for line in buf_reader.lines() {
        match line {
            Ok(line) => {
                let _ = writeln!(output, "{line}");
                let _ = output.flush();

                if !captured.is_empty() {
                    captured.push('\n');
                }
                captured.push_str(&line);
            }
            Err(e) => {
                tracing::warn!("Error reading stream: {}", e);
                break;
            }
        }
    }
}

/// Trait for CLI-based runners that share common execution logic.
///
/// Implementors only need to provide CLI-specific details (binary path, args, parsing),
/// while the shared infrastructure handles command execution and output capture.
pub trait CliRunnerConfig {
    /// Returns the name of this runner (e.g., "copilot", "claude").
    fn name(&self) -> &'static str;

    /// Returns the path to the CLI binary.
    fn binary_path(&self) -> &str;

    /// Builds the command arguments for a given prompt.
    fn build_args(&self, prompt: &str) -> Vec<String>;

    /// Parses token usage information from CLI output.
    /// Returns None if usage info cannot be parsed.
    fn parse_usage(&self, text: &str) -> Option<TokenUsageInfo>;

    /// Post-processes the output text (e.g., extracting result from JSON).
    /// Default implementation returns text as-is.
    fn post_process_output(&self, text: &str) -> String {
        text.to_string()
    }

    /// Returns additional display information for [`format_command_display`].
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

    /// Builds the command arguments for an interactive session.
    ///
    /// Returns `None` if the runner does not support interactive mode.
    /// Implementors should return arguments that launch the CLI in interactive
    /// chat mode with the given prompt as initial context.
    #[allow(dead_code)] // Interactive args for runners that support it
    fn build_interactive_args(&self, _prompt: &str) -> Option<Vec<String>> {
        None
    }
}

/// Checks if a CLI binary is available on the system.
pub fn check_cli_available(binary_path: &str) -> bool {
    which::which(binary_path).is_ok()
}

/// Resolves a binary name to its full path on the system.
///
/// On Windows, this is critical because `Command::new("copilot")` uses
/// `CreateProcessW` which only auto-appends `.exe`—it does NOT find `.cmd`
/// or `.bat` wrappers (common for npm-installed CLIs). By resolving the
/// full path via `which::which()`, we get the actual path including the
/// correct extension (e.g., `C:\...\copilot.cmd`), which `Command::new()`
/// can then execute directly.
///
/// On Unix systems, this is a no-op in practice (returns the same or
/// absolute path), but it keeps the logic uniform.
fn resolve_binary(binary_path: &str) -> String {
    which::which(binary_path).map_or_else(|_| binary_path.to_string(), |p| p.display().to_string())
}

/// Quotes an argument for safe passage through cmd.exe when invoking batch files.
///
/// Inside double-quoted strings, cmd.exe treats most characters literally
/// except `%` (variable expansion) and `"` (end of quote). This function:
/// - Wraps the argument in double quotes
/// - Doubles internal `"` characters (`""` is cmd.exe's escape sequence)
/// - Doubles `%` characters to prevent variable expansion
#[cfg(windows)]
fn quote_for_cmd(arg: &str) -> String {
    let mut result = String::with_capacity(arg.len() + 4);
    result.push('"');

    for c in arg.chars() {
        match c {
            '"' => result.push_str("\"\""),
            '%' => result.push_str("%%"),
            _ => result.push(c),
        }
    }

    result.push('"');
    result
}

/// Creates a [`Command`] with proper argument handling for all platforms.
///
/// On Windows, `.cmd`/`.bat` files (common for npm-installed CLIs like `copilot`)
/// are internally executed through cmd.exe. Rust 1.77.2+ validates arguments
/// passed to batch files and rejects characters special to cmd.exe (CVE-2024-24576).
/// Since runner prompts routinely contain these characters (`<`, `>`, `&`, `|`, `"`,
/// etc.), this function uses `raw_arg` with cmd.exe-safe quoting to bypass the
/// validation when the target is a batch file.
fn build_command(resolved_binary: &str, args: &[String], working_dir: &Path) -> Command {
    let mut command = Command::new(resolved_binary);
    command.current_dir(working_dir);

    #[cfg(windows)]
    {
        let lower = resolved_binary.to_lowercase();
        if lower.ends_with(".cmd") || lower.ends_with(".bat") {
            use std::os::windows::process::CommandExt;

            for arg in args {
                command.raw_arg(quote_for_cmd(arg));
            }

            return command;
        }
    }

    command.args(args);
    command
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
    let resolved_binary = resolve_binary(config.binary_path());

    tracing::debug!(
        binary = %resolved_binary,
        working_dir = %working_dir.display(),
        args = ?args,
        "Executing CLI"
    );

    let mut command = build_command(&resolved_binary, &args, working_dir);

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
        format!("{stdout}\n{stderr}")
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

    Ok(RunnerOutput {
        text: processed_output,
        success,
        usage,
    })
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
    println!("\n🔧 Executing: {cmd_display}");

    let args = config.build_args(prompt);
    let resolved_binary = resolve_binary(config.binary_path());

    tracing::debug!(
        binary = %resolved_binary,
        working_dir = %working_dir.display(),
        args = ?args,
        "Executing CLI (streaming)"
    );

    let mut command = build_command(&resolved_binary, &args, working_dir);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());

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
        read_stream_lines(stdout, output, &mut captured_output);
    }

    // Capture any stderr after stdout is done
    if let Some(stderr) = child.stderr.take() {
        read_stream_lines(stderr, output, &mut captured_output);
    }

    // Wait for the process to complete
    let status = child
        .wait()
        .map_err(|e| RunnerError::ProcessFailed(format!("Failed to wait for CLI: {e}")))?;

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

    Ok(RunnerOutput {
        text: processed_output,
        success,
        usage,
    })
}

/// Executes a CLI command in interactive mode with inherited stdio.
///
/// The process inherits stdin, stdout, and stderr so the user interacts
/// directly with the underlying agent. On exit, returns an [`InteractiveResult`]
/// with any available session context.
pub fn execute_interactive_cli<C: CliRunnerConfig + ?Sized>(
    config: &C,
    prompt: &str,
    working_dir: &Path,
) -> RunnerResult<InteractiveResult> {
    let Some(args) = config.build_interactive_args(prompt) else {
        return Err(RunnerError::ProcessFailed(
            "interactive mode is not supported by this runner".to_string(),
        ));
    };

    let resolved_binary = resolve_binary(config.binary_path());

    tracing::debug!(
        binary = %resolved_binary,
        working_dir = %working_dir.display(),
        args = ?args,
        "Executing CLI (interactive)"
    );

    let mut command = build_command(&resolved_binary, &args, working_dir);
    command
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    let status = command.status().map_err(|e| {
        RunnerError::ProcessFailed(format!(
            "Failed to start CLI at '{}': {}",
            config.binary_path(),
            e
        ))
    })?;

    if !status.success() {
        return Err(RunnerError::ProcessFailed(format!(
            "Interactive session exited with status: {status}"
        )));
    }

    tracing::debug!(
        exit_code = ?status.code(),
        "CLI completed (interactive)"
    );

    Ok(InteractiveResult {
        session_id: None,
        transcript: None,
    })
}

/// Blanket implementation of `Runner` for all types implementing `CliRunnerConfig`.
///
/// This eliminates boilerplate in individual runner modules (Copilot, Claude, etc.)
/// by providing a single, shared implementation that delegates to the CLI runner
/// infrastructure functions.
impl<T: CliRunnerConfig + Send + Sync> Runner for T {
    fn name(&self) -> &str {
        CliRunnerConfig::name(self)
    }

    fn format_command_display(&self, _prompt: &str, working_dir: &Path) -> Option<String> {
        Some(format_command_display(self, working_dir))
    }

    fn execute(&self, prompt: &str, working_dir: &Path) -> RunnerResult<RunnerOutput> {
        execute_cli(self, prompt, working_dir)
    }

    fn execute_streaming(
        &self,
        prompt: &str,
        working_dir: &Path,
        output: &mut dyn Write,
    ) -> RunnerResult<RunnerOutput> {
        execute_cli_streaming(self, prompt, working_dir, output)
    }

    fn is_available(&self) -> bool {
        check_cli_available(self.binary_path())
    }

    fn execute_interactive(
        &self,
        prompt: &str,
        working_dir: &Path,
    ) -> RunnerResult<InteractiveResult> {
        execute_interactive_cli(self, prompt, working_dir)
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    struct TestConfig {
        binary: String,
        args: Vec<String>,
    }

    impl CliRunnerConfig for TestConfig {
        fn name(&self) -> &'static str {
            "test"
        }

        fn binary_path(&self) -> &str {
            &self.binary
        }

        fn build_args(&self, _prompt: &str) -> Vec<String> {
            self.args.clone()
        }

        fn parse_usage(&self, _text: &str) -> Option<TokenUsageInfo> {
            None
        }
    }

    #[test]
    fn test_check_cli_available_cargo() {
        // 'cargo' is guaranteed to be on PATH since we're running inside cargo test
        assert!(check_cli_available("cargo"));
    }

    #[test]
    fn test_check_cli_available_nonexistent() {
        assert!(!check_cli_available("nonexistent-binary-xyz123"));
    }

    #[test]
    fn test_resolve_binary_found() {
        // 'cargo' is guaranteed to be on PATH; resolve should return an absolute path
        let resolved = resolve_binary("cargo");
        assert!(resolved.contains("cargo"));
        // Should resolve to an absolute path (not just the bare name)
        assert!(resolved.len() > "cargo".len());
    }

    #[test]
    fn test_resolve_binary_not_found_falls_back() {
        // Nonexistent binary should fall back to the original name
        let resolved = resolve_binary("nonexistent-binary-xyz123");
        assert_eq!(resolved, "nonexistent-binary-xyz123");
    }

    #[test]
    fn test_is_available_delegates_to_check_cli_available() {
        let config = TestConfig {
            binary: "cargo".to_string(),
            args: vec![],
        };
        assert!(config.is_available());

        let missing = TestConfig {
            binary: "nonexistent-binary-xyz123".to_string(),
            args: vec![],
        };
        assert!(!missing.is_available());
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

    #[test]
    fn test_read_stream_lines_single_line() {
        let input = b"hello world";
        let mut output = Vec::new();
        let mut captured = String::new();

        read_stream_lines(&input[..], &mut output, &mut captured);

        assert_eq!(captured, "hello world");
        assert!(String::from_utf8_lossy(&output).contains("hello world"));
    }

    #[test]
    fn test_read_stream_lines_multiple_lines() {
        let input = b"line1\nline2\nline3";
        let mut output = Vec::new();
        let mut captured = String::new();

        read_stream_lines(&input[..], &mut output, &mut captured);

        assert_eq!(captured, "line1\nline2\nline3");

        let output_str = String::from_utf8_lossy(&output);
        assert!(output_str.contains("line1"));
        assert!(output_str.contains("line2"));
        assert!(output_str.contains("line3"));
    }

    #[test]
    fn test_read_stream_lines_empty() {
        let input = b"";
        let mut output = Vec::new();
        let mut captured = String::new();

        read_stream_lines(&input[..], &mut output, &mut captured);

        assert!(captured.is_empty());
    }

    #[test]
    fn test_read_stream_lines_appends_to_existing() {
        let input = b"new line";
        let mut output = Vec::new();
        let mut captured = String::from("existing");

        read_stream_lines(&input[..], &mut output, &mut captured);

        assert_eq!(captured, "existing\nnew line");
    }

    #[test]
    fn test_build_command_sets_working_dir() {
        let working_dir = Path::new("/test/dir");
        let args = vec!["arg1".to_string(), "arg2".to_string()];

        let command = build_command("some-binary", &args, working_dir);

        // Verify the command was created (we can't easily inspect args,
        // but we can verify it doesn't panic and produces a valid Command).
        let program = format!("{:?}", command.get_program());
        assert!(program.contains("some-binary"));
    }

    #[cfg(windows)]
    #[test]
    fn test_quote_for_cmd_simple() {
        assert_eq!(quote_for_cmd("hello"), "\"hello\"");
    }

    #[cfg(windows)]
    #[test]
    fn test_quote_for_cmd_with_special_chars() {
        // Double quotes should be doubled.
        assert_eq!(quote_for_cmd("say \"hi\""), "\"say \"\"hi\"\"\"");

        // Percent signs should be doubled.
        assert_eq!(quote_for_cmd("100%"), "\"100%%\"");

        // cmd.exe special chars inside quotes are literal (no escaping needed).
        assert_eq!(quote_for_cmd("a & b | c < d > e"), "\"a & b | c < d > e\"");
    }

    #[cfg(windows)]
    #[test]
    fn test_build_command_batch_file_uses_raw_arg() {
        let working_dir = Path::new("C:\\test");
        let args = vec!["-p".to_string(), "prompt with <special> chars".to_string()];

        // Should not panic even with special characters when target is .cmd.
        let command = build_command("C:\\path\\to\\copilot.cmd", &args, working_dir);

        let program = format!("{:?}", command.get_program());
        assert!(program.contains("copilot.cmd"));
    }

    #[test]
    fn test_execute_interactive_unsupported_by_default() {
        // TestConfig does not override build_interactive_args, so it returns None.
        let config = TestConfig {
            binary: "echo".to_string(),
            args: vec![],
        };
        let working_dir = Path::new(".");

        let result = execute_interactive_cli(&config, "test prompt", working_dir);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("not supported"),
            "Expected 'not supported' error, got: {err}"
        );
    }

    struct InteractiveTestConfig {
        binary: String,
        interactive_args: Vec<String>,
    }

    impl CliRunnerConfig for InteractiveTestConfig {
        fn name(&self) -> &'static str {
            "interactive-test"
        }

        fn binary_path(&self) -> &str {
            &self.binary
        }

        fn build_args(&self, _prompt: &str) -> Vec<String> {
            vec![]
        }

        fn parse_usage(&self, _text: &str) -> Option<TokenUsageInfo> {
            None
        }

        fn build_interactive_args(&self, _prompt: &str) -> Option<Vec<String>> {
            Some(self.interactive_args.clone())
        }
    }

    #[test]
    fn test_execute_interactive_cli_success() {
        // Use `true` command which exits immediately with success.
        let config = InteractiveTestConfig {
            binary: "true".to_string(),
            interactive_args: vec![],
        };
        let working_dir = Path::new(".");

        let result = execute_interactive_cli(&config, "test", working_dir).unwrap();

        // `true` doesn't produce a session ID or transcript.
        assert!(result.session_id.is_none());
        assert!(result.transcript.is_none());
    }

    #[test]
    fn test_execute_interactive_cli_failure() {
        // Use `false` command which exits immediately with failure.
        let config = InteractiveTestConfig {
            binary: "false".to_string(),
            interactive_args: vec![],
        };
        let working_dir = Path::new(".");

        let result = execute_interactive_cli(&config, "test", working_dir);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("exited with status"),
            "Expected exit status error, got: {err}"
        );
    }
}
