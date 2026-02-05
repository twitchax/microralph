---
id: PRD-0031
title: "Cross-Platform Support: Fix Platform-Specific CLI Checks"
status: active
owner: twitchax
created: 2026-02-05
updated: 2026-02-05

principles:
- "Use cross-platform crates (e.g., `which`) over platform-specific shell commands"
- "Conditional compilation (`#[cfg]`) is acceptable but should be a last resort when a cross-platform crate exists"
- "Minimal changes: only fix platform-specific patterns, do not refactor unrelated code"
- "Assume `git` and other tools are on PATH; only fix the lookup mechanism"

references:
- name: "which crate on crates.io"
  url: https://crates.io/crates/which
- name: "Rust cfg conditional compilation"
  url: https://doc.rust-lang.org/reference/conditional-compilation.html

acceptance_tests:
- id: uat-001
  name: "Existing CI pipeline passes after changes"
  command: cargo make ci
  uat_status: unverified
- id: uat-002
  name: "check_cli_available works without shelling out to `which`"
  command: cargo make test
  uat_status: unverified
- id: uat-003
  name: "Binary compiles for Windows target"
  command: cargo check --target x86_64-pc-windows-msvc
  uat_status: unverified

tasks:
- id: T-001
  title: "Add `which` crate as a dependency in Cargo.toml"
  priority: 1
  status: done
  notes: "Add the `which` crate to [dependencies] in Cargo.toml."
- id: T-002
  title: "Replace `Command::new(\"which\")` with `which::which()` in cli_runner.rs"
  priority: 1
  status: done
  notes: "Rewrite `check_cli_available()` to use `which::which(binary_path).is_ok()` instead of shelling out. This handles Windows (`where.exe`), macOS, and Linux transparently."
- id: T-003
  title: "Update unit tests for check_cli_available"
  priority: 2
  status: done
  notes: "Ensure existing tests in cli_runner.rs still pass with the new implementation. No behavioral change expected."
- id: T-004
  title: "Fix `command -v` in Makefile.toml devcontainer task"
  priority: 3
  status: done
  notes: "Replace `command -v devcontainer` with a cross-platform check. Options include using `which` from the shell (available on most systems with Git Bash), or using cargo-make's built-in condition system. This is lower priority and not required for release."
- id: T-005
  title: "Audit for other platform-specific shell assumptions"
  priority: 3
  status: todo
  notes: "Scan for any other `Command::new` calls or shell-outs that assume Unix-only tooling. Document findings but only fix if trivially resolvable."

---

# Summary

Replace the Unix-specific `which` shell command used in `check_cli_available()` with the cross-platform `which` Rust crate, enabling microralph to correctly detect CLI tool availability on Windows (native `cmd.exe` / PowerShell) in addition to macOS and Linux. Secondarily, address the `command -v` Bash-ism in `Makefile.toml`.

---

# Problem

The `check_cli_available()` function in `src/runner/cli_runner.rs` shells out to `Command::new("which")`, which does not exist on native Windows. This causes all runner availability checks (Copilot CLI, Claude CLI) to fail on Windows, effectively making microralph unusable on that platform. Additionally, the `Makefile.toml` devcontainer task uses `command -v`, a Bash built-in that does not work in `cmd.exe` or PowerShell.

---

# Goals

1. Make `check_cli_available()` work correctly on Windows, macOS, and Linux without platform-specific shell commands.
2. Replace the `which` shell-out with the `which` Rust crate for a single, cross-platform binary lookup.
3. Fix the `command -v` usage in `Makefile.toml` to be cross-platform where feasible.
4. Establish conditional compilation (`#[cfg]`) as an acceptable pattern for future platform-specific code paths.

---

# Technical Approach

The fix is surgical: replace one function body and add one crate dependency.

**Step 1 — Add `which` crate**

Add `which` to `Cargo.toml` dependencies. The crate provides `which::which(binary_name)` which searches `PATH` using the platform-native mechanism (`where.exe` on Windows, path traversal on Unix).

**Step 2 — Rewrite `check_cli_available()`**

```rust
// Before
pub fn check_cli_available(binary_path: &str) -> bool {
    Command::new("which")
        .arg(binary_path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

// After
pub fn check_cli_available(binary_path: &str) -> bool {
    which::which(binary_path).is_ok()
}
```

No changes to callers are needed — the function signature and semantics are identical.

**Step 3 — Fix Makefile.toml (lower priority)**

Replace the `command -v devcontainer` check with `which devcontainer` or use cargo-make's built-in `condition` / `install_crate` features to avoid shell-specific syntax. Since `Makefile.toml` uses `script_runner = "@shell"`, and Windows may not have a POSIX shell, this task may require splitting the script into platform-specific variants or using `script_runner = "@duckscript"` which cargo-make supports cross-platform.

---

# Assumptions

- The `which` crate (well-maintained, 50M+ downloads) is an acceptable dependency.
- `git`, `npm`, and other external tools are assumed to be on the system `PATH` and do not need special Windows path resolution beyond what `which` provides.
- Native Windows (`cmd.exe` / PowerShell) is the target; WSL is out of scope.
- Manual verification on Windows is sufficient; no CI matrix changes are required.

---

# Constraints

- The function signature of `check_cli_available()` must not change (constitution rule 5: Public API Stability).
- Only platform-specific patterns should be touched; no unrelated refactoring (constitution rule 3: Minimal Changes).
- The `Makefile.toml` fix is best-effort and not a release blocker, since `cargo make devcontainer` is an optional workflow.

---

# References to Code

- `src/runner/cli_runner.rs:79-86` — `check_cli_available()` function (primary fix target)
- `src/runner/cli_runner.rs:262` — `is_available()` trait method calling `check_cli_available()`
- `src/runner/cli_runner.rs:295-303` — Unit tests for `check_cli_available()`
- `Makefile.toml:538` — `command -v devcontainer` Bash-ism
- `Cargo.toml:17-30` — Dependencies section (add `which` here)

---

# Non-Goals (MVP)

- Adding a Windows CI build matrix or GitHub Actions workflow
- Fixing path separator issues (`/` vs `\`) across the codebase
- Auditing or fixing `Command::new("git")` calls (git is assumed to be on PATH and work identically)
- WSL compatibility testing
- Full Windows end-to-end testing of all commands

---

# History

(Entries appended by `mr run` will go below this line.)

## 2026-02-05 — T-001 Completed
- **Task**: Add `which` crate as a dependency in Cargo.toml
- **Status**: ✅ Done
- **Changes**:
  - Added `which = "8.0.0"` to `[dependencies]` in `Cargo.toml` via `cargo add which`
  - `Cargo.lock` updated with `which v8.0.0`, `env_home v0.1.0`, and `winsafe v0.0.19` (transitive deps)
  - `cargo make uat` passes — all tests pass, fmt and clippy clean
- **Constitution Compliance**: No violations. Minimal change (one dependency added), no public API changes.

---

## 2026-02-05 — T-002 Completed
- **Task**: Replace `Command::new("which")` with `which::which()` in cli_runner.rs
- **Status**: ✅ Done
- **Changes**:
  - Replaced `check_cli_available()` body in `src/runner/cli_runner.rs` from shelling out to `Command::new("which")` to using `which::which(binary_path).is_ok()`
  - Function signature unchanged — no callers affected
  - All 484 tests pass via `cargo make uat`
- **Constitution Compliance**: No violations. Minimal change (one function body), public API preserved, no unrelated refactoring.
- **UAT Opportunistic Verification**: uat-001 and uat-002 feasible (CI passes, `check_cli_available` no longer shells out); uat-003 not verified (requires Windows MSVC target toolchain).

---

## 2026-02-05 — T-003 Completed
- **Task**: Update unit tests for check_cli_available
- **Status**: ✅ Done
- **Changes**:
  - Replaced `test_check_cli_available_echo` with `test_check_cli_available_cargo` in `src/runner/cli_runner.rs` — uses `cargo` instead of `echo` since `cargo` is guaranteed on PATH during `cargo test` and is cross-platform
  - Updated comment to remove Unix-specific language
  - Added `test_is_available_delegates_to_check_cli_available` to verify the `Runner` trait blanket impl routes through `check_cli_available()` for both available and missing binaries
  - All 485 tests pass via `cargo make uat` (net +1 test)
- **Constitution Compliance**: No violations. Minimal test updates, no public API changes, no unrelated refactoring.

---

## 2026-02-05 — T-004 Completed
- **Task**: Fix `command -v` in Makefile.toml devcontainer task
- **Status**: ✅ Done
- **Changes**:
  - Replaced `script_runner = "@shell"` with `script_runner = "@duckscript"` in the `devcontainer` task in `Makefile.toml`
  - Replaced `command -v devcontainer` (Bash built-in) with duckscript's cross-platform `which devcontainer`
  - Replaced `[ ! -f ... ]` file existence check with duckscript's `is_path_exists`
  - Replaced shell subprocess calls with duckscript's `exec` command
  - All 485 tests pass via `cargo make uat`
- **Constitution Compliance**: No violations. Minimal change (one task script rewritten), no public API changes, no unrelated refactoring.