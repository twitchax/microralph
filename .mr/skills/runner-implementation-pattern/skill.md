# runner-implementation-pattern

**Summary**: Follow the established pattern when implementing or modifying runners (Copilot, Claude, Codex) — config struct, build_args, token parsing, and mock testing.

## When to Use

- Adding a new runner (e.g., for a new AI CLI tool)
- Modifying runner behavior or adding flags
- Adding new Runner trait methods

## Pattern

### 1. Config Struct

Each runner has a `CliRunnerConfig`-based config with:
- Binary path
- Permission mode (Yolo/Manual)
- `no_ask_user` flag
- Optional model override

### 2. Build Args

Implement a private `build_args()` method that constructs CLI flags. Keep testable independently:
```rust
fn build_args(&self, prompt: &str) -> Vec<String> { ... }
```

### 3. Token Usage Parsing

Parse token usage from CLI output. Return `Option<UsageInfo>` with `input_tokens`, `output_tokens`, `total_tokens`.

### 4. Output Stripping

Implement `strip_usage_stats()` to remove metadata from output:
- JSON CLIs (Claude, Codex): extract `result` field
- Text CLIs (Copilot): regex-strip stats sections

### 5. Runner Trait

The `Runner` trait requires:
- `execute(prompt, working_dir)` — non-streaming
- `execute_streaming(prompt, working_dir, writer)` — real-time output
- `execute_interactive(prompt, working_dir)` — stdio inherit for user interaction
- `execute_continue(session_id, prompt, working_dir)` — session resume (optional)

### 6. Mock Testing

All tests use `MockRunner` — never require actual CLI tools. Use test-only constructors and `set_interactive_error()` for error path testing.

### 7. Default to Yolo Mode

Runners default to auto-granting permissions for autonomous operation.
