//! Mock runner for testing.
//!
//! This module is test infrastructure, so unwrap usage is acceptable.
#![allow(clippy::unwrap_used)]

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;

use super::types::{InteractiveResult, Runner, RunnerOutput, RunnerResult};

/// A mock runner for deterministic testing.
///
/// The mock runner returns pre-configured responses in sequence.
/// This allows testing the two-phase interactive flow without invoking a real runner.
#[derive(Debug)]
pub struct MockRunner {
    /// The name of this runner.
    name: String,

    /// Queue of responses to return.
    responses: Mutex<VecDeque<RunnerOutput>>,

    /// Recorded prompts that were executed.
    recorded_prompts: Mutex<Vec<String>>,

    /// Pre-configured interactive result to return from `execute_interactive()`.
    interactive_result: Mutex<Option<InteractiveResult>>,

    /// Recorded interactive prompts.
    recorded_interactive_prompts: Mutex<Vec<String>>,
}

impl MockRunner {
    /// Creates a new mock runner with the given responses.
    ///
    /// Responses are returned in order as `execute()` is called.
    pub fn new(responses: Vec<RunnerOutput>) -> Self {
        Self {
            name: "mock".to_string(),
            responses: Mutex::new(responses.into()),
            recorded_prompts: Mutex::new(Vec::new()),
            interactive_result: Mutex::new(None),
            recorded_interactive_prompts: Mutex::new(Vec::new()),
        }
    }

    /// Creates an empty mock runner.
    pub fn empty() -> Self {
        Self::new(Vec::new())
    }

    /// Sets the interactive result to return from `execute_interactive()`.
    #[cfg(test)]
    pub fn set_interactive_result(&self, result: InteractiveResult) {
        *self.interactive_result.lock().unwrap() = Some(result);
    }

    /// Adds a response to the queue.
    #[cfg(test)]
    pub fn add_response(&self, response: RunnerOutput) {
        self.responses.lock().unwrap().push_back(response);
    }

    /// Adds a successful text response to the queue.
    #[cfg(test)]
    pub fn add_success(&self, text: impl Into<String>) {
        self.add_response(RunnerOutput::success(text));
    }

    /// Returns all recorded prompts.
    #[cfg(test)]
    pub fn recorded_prompts(&self) -> Vec<String> {
        self.recorded_prompts.lock().unwrap().clone()
    }

    /// Returns the number of remaining responses.
    #[cfg(test)]
    pub fn remaining_responses(&self) -> usize {
        self.responses.lock().unwrap().len()
    }

    /// Returns all recorded interactive prompts.
    #[cfg(test)]
    pub fn recorded_interactive_prompts(&self) -> Vec<String> {
        self.recorded_interactive_prompts.lock().unwrap().clone()
    }
}

impl Default for MockRunner {
    fn default() -> Self {
        Self::empty()
    }
}

impl Runner for MockRunner {
    fn name(&self) -> &str {
        &self.name
    }

    fn format_command_display(&self, _prompt: &str, _working_dir: &Path) -> Option<String> {
        Some("mock-runner (no actual command)".to_string())
    }

    fn execute(&self, prompt: &str, _working_dir: &Path) -> RunnerResult<RunnerOutput> {
        // Record the prompt.
        self.recorded_prompts
            .lock()
            .unwrap()
            .push(prompt.to_string());

        // Return the next response, or a default.
        let response = self
            .responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| RunnerOutput::success("Mock response (no more scripted responses)"));

        Ok(response)
    }

    fn is_available(&self) -> bool {
        true
    }

    fn execute_interactive(
        &self,
        prompt: &str,
        _working_dir: &Path,
    ) -> RunnerResult<InteractiveResult> {
        // Record the interactive prompt.
        self.recorded_interactive_prompts
            .lock()
            .unwrap()
            .push(prompt.to_string());

        // Return the pre-configured interactive result, or a default.
        let result = self
            .interactive_result
            .lock()
            .unwrap()
            .clone()
            .unwrap_or(InteractiveResult {
                session_id: None,
                transcript: Some("Mock interactive session transcript".to_string()),
            });

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_runner_returns_responses_in_order() {
        let runner = MockRunner::new(vec![
            RunnerOutput::success("First"),
            RunnerOutput::success("Second"),
            RunnerOutput::success("Third"),
        ]);

        let path = Path::new(".");

        let r1 = runner.execute("prompt1", path).unwrap();
        assert_eq!(r1.text, "First");

        let r2 = runner.execute("prompt2", path).unwrap();
        assert_eq!(r2.text, "Second");

        let r3 = runner.execute("prompt3", path).unwrap();
        assert_eq!(r3.text, "Third");
    }

    #[test]
    fn test_mock_runner_records_prompts() {
        let runner = MockRunner::empty();
        let path = Path::new(".");

        runner.execute("first prompt", path).unwrap();
        runner.execute("second prompt", path).unwrap();

        let prompts = runner.recorded_prompts();
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "first prompt");
        assert_eq!(prompts[1], "second prompt");
    }

    #[test]
    fn test_mock_runner_add_response() {
        let runner = MockRunner::empty();
        let path = Path::new(".");

        runner.add_success("Added response");

        let result = runner.execute("test", path).unwrap();
        assert_eq!(result.text, "Added response");
    }

    #[test]
    fn test_mock_runner_remaining_responses() {
        let runner = MockRunner::new(vec![
            RunnerOutput::success("One"),
            RunnerOutput::success("Two"),
        ]);

        assert_eq!(runner.remaining_responses(), 2);

        let path = Path::new(".");
        runner.execute("test", path).unwrap();

        assert_eq!(runner.remaining_responses(), 1);
    }

    #[test]
    fn test_mock_runner_default_response_when_empty() {
        let runner = MockRunner::empty();
        let path = Path::new(".");

        let result = runner.execute("test", path).unwrap();
        assert!(result.success);
        assert!(result.text.contains("no more scripted responses"));
    }

    #[test]
    fn test_mock_runner_name() {
        let runner = MockRunner::empty();
        assert_eq!(runner.name(), "mock");
    }

    #[test]
    fn test_mock_runner_is_available() {
        let runner = MockRunner::empty();
        assert!(runner.is_available());
    }

    #[test]
    fn test_mock_runner_omits_usage_info() {
        let runner = MockRunner::empty();
        let path = Path::new(".");

        let result = runner.execute("test", path).unwrap();
        assert!(
            result.usage.is_none(),
            "MockRunner should not provide usage info"
        );
    }

    #[test]
    fn test_mock_runner_execute_interactive_returns_default() {
        let runner = MockRunner::empty();
        let path = Path::new(".");

        let result = runner.execute_interactive("test", path);
        assert!(result.is_ok(), "MockRunner should support interactive mode");

        let interactive = result.unwrap();
        assert!(interactive.transcript.is_some());
    }

    #[test]
    fn test_mock_runner_execute_interactive_with_custom_result() {
        let runner = MockRunner::empty();
        let path = Path::new(".");

        runner.set_interactive_result(InteractiveResult {
            session_id: Some("session-42".to_string()),
            transcript: Some("User: Tell me about X\nAgent: X is...".to_string()),
        });

        let result = runner.execute_interactive("test", path).unwrap();
        assert_eq!(result.session_id.as_deref(), Some("session-42"));
        assert!(result.transcript.unwrap().contains("Tell me about X"));
    }

    #[test]
    fn test_mock_runner_records_interactive_prompts() {
        let runner = MockRunner::empty();
        let path = Path::new(".");

        runner
            .execute_interactive("interactive prompt 1", path)
            .unwrap();
        runner
            .execute_interactive("interactive prompt 2", path)
            .unwrap();

        let prompts = runner.recorded_interactive_prompts();
        assert_eq!(prompts.len(), 2);
        assert_eq!(prompts[0], "interactive prompt 1");
        assert_eq!(prompts[1], "interactive prompt 2");
    }

    #[test]
    fn test_mock_runner_execute_continue_returns_none() {
        let runner = MockRunner::empty();
        let path = Path::new(".");

        let result = runner.execute_continue("test", path);
        assert!(
            result.is_none(),
            "MockRunner should return None for execute_continue (no session resume)"
        );
    }
}
