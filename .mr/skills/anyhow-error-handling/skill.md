# anyhow-error-handling

**Summary**: Use `anyhow::Result` for all fallible functions, `.context()` for error enrichment, and never `.unwrap()` in production code.

## When to Use

- Writing any function that can fail
- Adding error context to operations
- Converting between error types

## Pattern

### Function Signatures

```rust
use anyhow::{Context, Result};

pub fn do_something(path: &Path) -> Result<String> {
    let content = std::fs::read_to_string(path)
        .context("Failed to read config file")?;
    Ok(content)
}
```

### Key Rules

1. **Return `anyhow::Result<T>`** — not `std::result::Result` with custom error types (except for `RunnerError` which is its own enum)
2. **Use `.context()`** to add human-readable messages to errors
3. **Never `.unwrap()`** — the project denies `clippy::unwrap_used` in production code
4. **Use `?` propagation** — chain with `.context()` when the default error message is unclear
5. **In tests**: `#[allow(clippy::unwrap_used)]` is acceptable on test modules

### Alternatives to `.unwrap()`

| Instead of | Use |
|-----------|-----|
| `.unwrap()` | `?` with `.context()` |
| `.unwrap_or()` | Still fine — not banned |
| `.expect("msg")` | Acceptable if panic is intentional and documented |
| `.unwrap_or_default()` | Fine for optional values |
