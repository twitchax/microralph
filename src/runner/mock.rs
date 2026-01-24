//! Mock runner for testing.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::Mutex;

use super::types::{Runner, RunnerOutput, RunnerResult};

/// A mock runner for deterministic testing.
///
/// The mock runner returns pre-configured responses in sequence.
/// This allows testing the Q/A flow without invoking a real runner.
#[derive(Debug)]
pub struct MockRunner {
    /// The name of this runner.
    name: String,

    /// Queue of responses to return.
    responses: Mutex<VecDeque<RunnerOutput>>,

    /// Recorded prompts that were executed.
    recorded_prompts: Mutex<Vec<String>>,
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
        }
    }

    /// Creates an empty mock runner.
    pub fn empty() -> Self {
        Self::new(Vec::new())
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
}
