---
id: PRD-0008
title: "Fix CI cargo-make Not Found Error"
status: active
owner: "twitchax"
created: 2026-01-24
updated: 2026-01-25

principles:
- Fix must be simple and targeted; avoid over-engineering
- CI should work reliably on cache hit and cache miss scenarios
- Prefer adjusting existing configuration over adding new steps

references:
- name: Failed GitHub Actions Run
  url: https://github.com/twitchax/microralph/actions/runs/21312622613/job/61352818340
- name: Swatinem/rust-cache Documentation
  url: https://github.com/Swatinem/rust-cache

acceptance_tests:
- id: uat-001
  name: CI passes on push with cache hit
  command: cargo make ci
  uat_status: unverified

tasks:
- id: T-001
  title: Investigate rust-cache bin caching interaction with binstall
  priority: 1
  status: done
  notes: The cache restores ~/.cargo/bin, binstall sees cargo-make as "already installed" but the binary isn't properly available to cargo.  Weirdly, this works for kord (https://github.com/twitchax/kord/actions/runs/21278722168/job/61243513376).
- id: T-002
  title: Fix CI workflow to ensure cargo-make is available
  priority: 2
  status: todo
  notes: Options include forcing reinstall with --force, disabling cache-bin in rust-cache, or reordering steps. Try the simplest fix first.

---

# Summary

The CI workflow fails with `error: no such command: make` even though `cargo binstall cargo-make` reports the tool as "already installed". This appears to be a cache interaction issue where the rust-cache action restores the cargo bin directory, but the binary isn't properly detected by cargo afterward.

---

# Problem

When CI runs with a cache hit, the following sequence occurs:
1. `cargo-bins/cargo-binstall@main` installs binstall
2. `Swatinem/rust-cache@v2` restores the cache, including `~/.cargo/bin` and `.crates.toml`/.crates2.json`
3. `cargo binstall cargo-make --no-confirm` sees cargo-make v0.37.24 as "already installed" and skips installation
4. `cargo make ci` fails with "no such command: make"

The root cause is likely that the cached cargo-make binary or its metadata is stale or corrupted, causing cargo to not recognize it as a valid subcommand despite binstall detecting it as installed.

---

# Goals

1. Make CI reliably pass when the cache is restored
2. Ensure cargo-make is properly available to cargo after cache restoration
3. Keep the fix minimal and avoid unnecessary complexity

---

# Non-Goals (MVP)

- Auditing other binstalled tools for similar issues
- Adding verification steps like `cargo make --version`
- Comprehensive cache debugging or optimization

---

# History

## 2026-01-25 — T-001 Completed
- **Task**: Investigate rust-cache bin caching interaction with binstall
- **Status**: ✅ Done
- **Changes**:
  - Added `cache-bin: "false"` to all Swatinem/rust-cache@v2 uses in `.github/workflows/build.yml`
  - This prevents caching of `~/.cargo/bin`, which was causing binstall to detect cargo-make as "already installed" but the binary wasn't properly available to cargo
  - Applied to all 5 jobs: test, codecov, build_linux, build_windows, build_macos
  - UAT passed: `cargo make uat` runs successfully (all tests pass)
- **Root Cause**: The rust-cache action by default caches `~/.cargo/bin`. When restored, binstall sees tools as installed but cargo doesn't recognize them as valid subcommands. Disabling bin caching forces fresh reinstalls, avoiding the issue.

---