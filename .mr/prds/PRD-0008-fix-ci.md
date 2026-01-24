---
id: PRD-0008
title: "Fix CI Build Failures"
status: draft
owner: "twitchax"
created: 2026-01-24
updated: 2026-01-24

principles:
  - Keep CI configuration minimal and stable
  - Use latest versions without pinning to specific releases
  - Maintain consistency across all workflow jobs

references:
  - name: cargo-binstall Installation
    url: https://github.com/cargo-bins/cargo-binstall
  - name: Swatinem/rust-cache
    url: https://github.com/Swatinem/rust-cache

acceptance_tests:
  - id: uat-001
    name: CI workflow passes on main branch
    command: gh run list --workflow=build.yml --limit=1 --json conclusion --jq '.[0].conclusion == "success"'
    uat_status: unverified

tasks:
  - id: T-001
    title: Replace cargo-binstall action with inline install script
    priority: 1
    status: todo
    notes: The cargo-bins/cargo-binstall@main action returns 404. Replace with inline curl-based installation in all 5 jobs (test, codecov, build_linux, build_windows, build_macos).

---

# Summary

Fix the broken CI workflow caused by the `cargo-bins/cargo-binstall@main` GitHub Action returning 404 errors when downloading release artifacts. Replace the action with a reliable inline installation method across all workflow jobs.

---

# Problem

The CI workflow is failing because the `cargo-bins/cargo-binstall@main` action cannot download the cargo-binstall binary—the upstream release URL returns a 404 error. This blocks all builds, tests, and code coverage runs. The failure affects all 5 jobs that depend on cargo-binstall: test, codecov, build_linux, build_windows, and build_macos.

---

# Goals

1. Restore CI to a passing state on the main branch
2. Use a reliable installation method for cargo-binstall
3. Apply the fix consistently across all workflow jobs

---

# Non-Goals (MVP)

- Adding retry logic or fallback installation methods
- Pinning to a specific cargo-binstall version
- Caching the binstall binary separately from rust-cache
- Adding new CI features or jobs

---

# History

(Entries appended by `mr run` will go below this line.)

---