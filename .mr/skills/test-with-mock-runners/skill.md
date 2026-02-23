# test-with-mock-runners

**Summary**: All runner-dependent tests use `MockRunner` — never require actual CLI tools (copilot, claude, codex) to be installed.

## When to Use

- Writing tests for commands that invoke runners (run, refactor, suggest, bootstrap)
- Testing error paths in interactive mode
- Verifying prompt construction or output parsing

## MockRunner Setup

```rust
use crate::runner::mock::MockRunner;

let mock = MockRunner::new();
mock.set_response("expected output text");

// For error testing:
mock.set_interactive_error(RunnerError::Interrupted("SIGINT".to_string()));
```

## Key Patterns

### 1. Test Directory Structure

Create temp directories that mirror `.mr/` structure:
```rust
let tmp = tempfile::tempdir()?;
let root = tmp.path();
// Create .mr/, .mr/prds/, .mr/prompts/, etc.
init::init(root, Language::Rust)?;
```

### 2. Runner Tests Are Unit Tests

Runner `build_args()` methods are tested directly without spawning processes:
```rust
#[test]
fn test_build_args_includes_model() {
    let config = ClaudeConfig { model: Some("sonnet".into()), ..Default::default() };
    let args = config.build_args("prompt");
    assert!(args.contains(&"--model".to_string()));
}
```

### 3. No External Dependencies

Tests must pass in CI without copilot-cli, claude, or codex installed. `MockRunner` simulates all runner behaviors including streaming, interactive mode, and error conditions.

### 4. Test Naming Convention

Tests are descriptive: `test_build_prompt_skills_manifest_injected`, `test_restore_preserves_existing_skills`, etc.
