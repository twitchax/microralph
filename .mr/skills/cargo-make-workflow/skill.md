# cargo-make-workflow

**Summary**: Use `cargo make` tasks for all dev operations — never raw `cargo` commands for CI, testing, or releases.

## When to Use

- Running tests, linting, formatting, or CI checks
- Building for specific platforms
- Publishing releases or generating changelogs
- Any dev workflow beyond `cargo build`

## Key Commands

| Task | Purpose |
|------|---------|
| `cargo make fmt` | Format code |
| `cargo make clippy` | Run clippy with `--all-targets --all-features -- -D warnings` |
| `cargo make test` | Run tests via cargo-nextest |
| `cargo make ci` | Full CI pipeline: fmt-check → clippy → test |
| `cargo make uat` | The one true gate: runs CI, optionally filters tests |

## Important Details

- Tests use **cargo-nextest**, not `cargo test`. The `test` task auto-installs nextest via cargo-binstall.
- `cargo make uat` is the acceptance gate — always run this before committing.
- `cargo make ci` runs `fmt-check` (not `fmt`), so format first if needed.
- Platform builds: `build-linux`, `build-macos`, `build-windows`, `build-wasm`.
- Release flow: `cargo make release` (automated) or individual tasks for manual control.
