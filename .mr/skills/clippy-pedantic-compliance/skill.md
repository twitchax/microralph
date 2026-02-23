# clippy-pedantic-compliance

**Summary**: All production code enforces `clippy::pedantic` — know common lints and how to satisfy them without suppression.

## When to Use

- Writing any new production code in `src/`
- Encountering clippy warnings after code changes
- Reviewing code that may trigger pedantic lints

## Details

The project sets `#![deny(clippy::pedantic)]` in `src/main.rs`, making all pedantic lints hard errors. Additionally, `#![deny(clippy::unwrap_used)]` bans `.unwrap()` in production code.

### Common Pedantic Lints and Fixes

| Lint | Fix |
|------|-----|
| `clippy::module_name_repetitions` | Rename types to avoid stuttering (e.g., `runner::RunnerConfig` → `runner::Config`) |
| `clippy::must_use_candidate` | Add `#[must_use]` to pure functions returning values |
| `clippy::missing_errors_doc` | Add `# Errors` section to doc comments on fallible functions |
| `clippy::missing_panics_doc` | Add `# Panics` section or eliminate the panic |
| `clippy::needless_pass_by_value` | Take `&str` instead of `String` for read-only params |
| `clippy::too_many_lines` | Extract helper functions to reduce function length |
| `clippy::cast_possible_truncation` | Use `usize::try_from()` or document the cast |
| `clippy::unwrap_used` | Use `?`, `.context()`, or `.unwrap_or_default()` instead |

### Test Code Exceptions

In `#[cfg(test)]` modules, you may suppress specific lints where they reduce test readability:
```rust
#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests { ... }
```
