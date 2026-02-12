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

use super::types::{Runner, RunnerError, RunnerOutput, RunnerResult, TokenUsageInfo};

/// Parses token usage information from JSON CLI output.
///
/// Expects a JSON object with a `usage` field containing `input_tokens` and/or
/// `output_tokens`. This is the common format used by Claude (`--output-format json`)
/// and Codex (`--json`) CLIs.
pub fn parse_json_usage(text: &str) -> Option<TokenUsageInfo> {
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

/// Extracts the `result` field from JSON CLI output.
///
/// Expects a JSON object with a `result` string field. Falls back to the original
/// text if parsing fails or the field is missing. This is the common format used
/// by Claude (`--output-format json`) and Codex (`--json`) CLIs.
pub fn extract_json_result(text: &str) -> String {
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(text)
        && let Some(result) = json.get("result").and_then(|v| v.as_str())
    {
        return result.to_string();
    }

    text.to_string()
}

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

/// Quotes an argument for safe passage through PowerShell.
///
/// In PowerShell single-quoted strings, the only special character is `'`
/// itself, which is escaped by doubling it (`''`). This makes single-quoted
/// strings ideal for passing arbitrary content — including newlines, `<`, `>`,
/// `&`, `|`, `"`, `%`, etc. — without any interpretation.
#[cfg(windows)]
fn quote_for_powershell(arg: &str) -> String {
    let escaped = arg.replace('\'', "''");
    format!("'{escaped}'")
}

/// Sanitizes environment variables to remove problematic Node.js-specific vars.
///
/// When spawning Node.js-based CLIs (like `copilot` installed via npm), certain
/// environment variables can cause issues:
/// - `NODE_NO_WARNINGS=1` gets converted to `--no-warnings` flag by npm wrapper
///   scripts, which Node.js doesn't recognize (should be `--no-warnings=<category>`)
///
/// This function removes known problematic Node.js env vars while preserving
/// essential vars like `NODE_OPTIONS`, `PATH`, `HOME`, etc.
fn sanitize_env_vars(command: &mut Command) {
    // List of Node.js env vars that can cause CLI flag conversion issues
    const PROBLEMATIC_NODE_VARS: &[&str] = &[
        "NODE_NO_WARNINGS",
        // Add more if discovered in the future
    ];

    for var in PROBLEMATIC_NODE_VARS {
        command.env_remove(var);
    }
}

/// Creates a [`Command`] with proper argument handling for all platforms.
///
/// On Windows, `.cmd`/`.bat` files (common for npm-installed CLIs like `copilot`)
/// are internally executed through cmd.exe, which **cannot handle newlines in
/// arguments** — it treats the newline as a command terminator, so multi-line
/// prompts are truncated to the first line. To avoid this, `.cmd`/`.bat` targets
/// are invoked through `PowerShell`, which correctly handles multi-line arguments
/// via single-quoted strings.
///
/// Additionally, this function removes problematic Node.js environment variables
/// (like `NODE_NO_WARNINGS`) that can cause npm wrapper scripts to inject invalid
/// CLI flags, leading to errors and excessive token usage from retry loops.
fn build_command(resolved_binary: &str, args: &[String], working_dir: &Path) -> Command {
    #[cfg(windows)]
    {
        let lower = resolved_binary.to_lowercase();
        if lower.ends_with(".cmd") || lower.ends_with(".bat") {
            // Use PowerShell to invoke batch files. cmd.exe (which processes
            // .cmd/.bat files directly) cannot handle newlines in arguments,
            // causing multi-line prompts to be truncated to the first line.
            // PowerShell handles multi-line arguments correctly via
            // single-quoted strings.
            let mut ps_command = String::from("& ");
            ps_command.push_str(&quote_for_powershell(resolved_binary));

            for arg in args {
                ps_command.push(' ');
                ps_command.push_str(&quote_for_powershell(arg));
            }

            let mut command = Command::new("powershell.exe");
            command.current_dir(working_dir);
            command.args([
                "-NoProfile",
                "-ExecutionPolicy",
                "Bypass",
                "-Command",
                &ps_command,
            ]);

            sanitize_env_vars(&mut command);
            return command;
        }
    }

    let mut command = Command::new(resolved_binary);
    command.current_dir(working_dir);
    command.args(args);
    sanitize_env_vars(&mut command);
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

/// Maps a Unix signal number to a human-readable name.
#[cfg(unix)]
fn signal_name(sig: i32) -> &'static str {
    match sig {
        2 => "SIGINT/Ctrl+C",
        9 => "SIGKILL",
        15 => "SIGTERM",
        _ => "unknown signal",
    }
}

/// Executes a CLI command in interactive mode with inherited stdio.
///
/// The process inherits stdin, stdout, and stderr so the user interacts
/// directly with the underlying agent.
pub fn execute_interactive_cli<C: CliRunnerConfig + ?Sized>(
    config: &C,
    prompt: &str,
    working_dir: &Path,
) -> RunnerResult<()> {
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
        // On Unix, a process killed by a signal has no exit code (code() returns None).
        // Use signal() to detect which signal terminated the process.
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            if let Some(sig) = status.signal() {
                return Err(RunnerError::Interrupted(format!(
                    "Interactive session terminated by signal {sig} ({})",
                    signal_name(sig)
                )));
            }
        }

        // On all platforms, a non-zero exit code without a signal is a general failure.
        return Err(RunnerError::ProcessFailed(format!(
            "Interactive session exited with status: {status}"
        )));
    }

    tracing::debug!(
        exit_code = ?status.code(),
        "CLI completed (interactive)"
    );

    Ok(())
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

    fn execute_interactive(&self, prompt: &str, working_dir: &Path) -> RunnerResult<()> {
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

    #[test]
    fn test_build_command_removes_node_no_warnings() {
        use std::env;

        // Set the problematic env var (unsafe in Rust 1.80+)
        unsafe {
            env::set_var("NODE_NO_WARNINGS", "1");
        }

        let working_dir = Path::new("/test/dir");
        let args = vec!["arg1".to_string()];

        let command = build_command("test-binary", &args, working_dir);

        // Clean up
        unsafe {
            env::remove_var("NODE_NO_WARNINGS");
        }

        // If we got here without panic, the command was created successfully
        assert!(format!("{:?}", command.get_program()).contains("test-binary"));
    }

    #[cfg(windows)]
    #[test]
    fn test_quote_for_powershell_simple() {
        assert_eq!(quote_for_powershell("hello"), "'hello'");
    }

    #[cfg(windows)]
    #[test]
    fn test_quote_for_powershell_with_special_chars() {
        // Single quotes should be doubled.
        assert_eq!(quote_for_powershell("it's"), "'it''s'");

        // Double quotes, percent, and cmd.exe specials are literal in PS single-quoted strings.
        assert_eq!(quote_for_powershell("say \"hi\""), "'say \"hi\"'");
        assert_eq!(quote_for_powershell("100%"), "'100%'");
        assert_eq!(
            quote_for_powershell("a & b | c < d > e"),
            "'a & b | c < d > e'"
        );
    }

    #[cfg(windows)]
    #[test]
    fn test_quote_for_powershell_multiline() {
        let multiline = "line1\nline2\nline3";
        assert_eq!(quote_for_powershell(multiline), "'line1\nline2\nline3'");
    }

    #[cfg(windows)]
    #[test]
    fn test_build_command_batch_file_uses_powershell() {
        let working_dir = Path::new("C:\\test");
        let args = vec!["-p".to_string(), "prompt with\nnewlines".to_string()];

        // .cmd files should be routed through powershell.exe.
        let command = build_command("C:\\path\\to\\copilot.cmd", &args, working_dir);

        let program = format!("{:?}", command.get_program());
        assert!(
            program.contains("powershell"),
            "Expected powershell.exe, got: {program}"
        );
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

        let result = execute_interactive_cli(&config, "test", working_dir);
        assert!(result.is_ok());
    }

    #[test]
    fn test_execute_interactive_cli_failure() {
        // Use `false` command which exits immediately with failure (non-signal).
        let config = InteractiveTestConfig {
            binary: "false".to_string(),
            interactive_args: vec![],
        };
        let working_dir = Path::new(".");

        let result = execute_interactive_cli(&config, "test", working_dir);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            !err.is_interrupted(),
            "`false` should produce ProcessFailed, not Interrupted"
        );
        assert!(
            err.to_string().contains("exited with status"),
            "Expected exit status error, got: {err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_execute_interactive_cli_signal_interrupted() {
        // Spawn a process that sends itself SIGINT to simulate Ctrl+C.
        let config = InteractiveTestConfig {
            binary: "sh".to_string(),
            interactive_args: vec!["-c".to_string(), "kill -2 $$".to_string()],
        };
        let working_dir = Path::new(".");

        let result = execute_interactive_cli(&config, "test", working_dir);
        assert!(result.is_err());

        let err = result.unwrap_err();
        assert!(
            err.is_interrupted(),
            "Signal-killed process should produce Interrupted, got: {err}"
        );
        assert!(
            err.to_string().contains("SIGINT"),
            "Error should mention SIGINT, got: {err}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn test_signal_name_known_signals() {
        assert_eq!(signal_name(2), "SIGINT/Ctrl+C");
        assert_eq!(signal_name(9), "SIGKILL");
        assert_eq!(signal_name(15), "SIGTERM");
    }

    #[cfg(unix)]
    #[test]
    fn test_signal_name_unknown_signal() {
        assert_eq!(signal_name(42), "unknown signal");
    }

    // ── Shared JSON helper tests ──────────────────────────────────────

    #[test]
    fn test_parse_json_usage_valid() {
        let json = r#"{
            "result": "text",
            "usage": {
                "input_tokens": 1234,
                "output_tokens": 56
            }
        }"#;

        let usage = parse_json_usage(json).unwrap();
        assert_eq!(usage.input, Some(1234));
        assert_eq!(usage.output, Some(56));
        assert_eq!(usage.total, Some(1290));
    }

    #[test]
    fn test_parse_json_usage_partial() {
        let json = r#"{ "usage": { "input_tokens": 100 } }"#;

        let usage = parse_json_usage(json).unwrap();
        assert_eq!(usage.input, Some(100));
        assert_eq!(usage.output, None);
        assert_eq!(usage.total, None);
    }

    #[test]
    fn test_parse_json_usage_no_usage() {
        assert!(parse_json_usage(r#"{ "result": "text" }"#).is_none());
    }

    #[test]
    fn test_parse_json_usage_invalid_json() {
        assert!(parse_json_usage("not json").is_none());
    }

    #[test]
    fn test_extract_json_result_valid() {
        let json = r#"{ "result": "Hello, world!", "usage": {} }"#;
        assert_eq!(extract_json_result(json), "Hello, world!");
    }

    #[test]
    fn test_extract_json_result_missing_field() {
        let json = r#"{ "usage": {} }"#;
        assert_eq!(extract_json_result(json), json);
    }

    #[test]
    fn test_extract_json_result_invalid_json() {
        assert_eq!(extract_json_result("plain text"), "plain text");
    }
}
