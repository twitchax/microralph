---
id: PRD-0002
title: Prepare for Releases
status: draft                 # draft | active | done | parked
owner: Aaron Roney
created: 2026-01-24
updated: 2026-01-24

principles:
- Follow kord patterns for release infrastructure and CI/CD.
- All release workflows should route through cargo-make tasks.
- Prefer GitHub Actions for CI, crates.io for Rust distribution.

references:
- name: kord (release pipeline reference)
  url: https://github.com/twitchax/kord
- name: Keep a Changelog
  url: https://keepachangelog.com/en/1.0.0/
- name: Semantic Versioning
  url: https://semver.org/spec/v2.0.0.html

acceptance_tests:
- id: uat-001
  name: Code coverage runs successfully
  command: cargo make codecov
- id: uat-002
  name: Linux binary builds
  command: cargo make build-linux
- id: uat-003
  name: macOS binary builds
  command: cargo make build-macos
- id: uat-004
  name: Windows binary builds
  command: cargo make build-windows
- id: uat-005
  name: WASM binary builds
  command: cargo make build-wasm
- id: uat-006
  name: Changelog generation works
  command: cargo make changelog
- id: uat-007
  name: Release version bump works
  command: cargo make release --dry-run
- id: uat-008
  name: crates.io publish works (dry-run)
  command: cargo make publish-crates --dry-run

tasks:
- id: T-001
  title: Add code coverage with cargo-llvm-cov + Codecov integration
  priority: 1
  status: todo
  notes: Add codecov task to Makefile.toml, update CI workflow to run coverage and upload to Codecov.
- id: T-002
  title: Add cross-platform build jobs to CI (Linux x86_64, macOS ARM, Windows, WASM32-WASIP2)
  priority: 2
  status: todo
  notes: Add build-linux, build-macos, build-windows, build-wasm jobs to build.yml. Upload artifacts to GitHub Artifacts. Only run on main branch.
- id: T-003
  title: Add cargo-make build tasks for each target platform
  priority: 3
  status: todo
  notes: Add build-linux, build-macos, build-windows, build-wasm tasks to Makefile.toml mirroring kord patterns.
- id: T-004
  title: Set up cargo-release for version management
  priority: 4
  status: todo
  notes: Add install-cargo-release task, configure release.toml if needed, add release task to Makefile.toml.
- id: T-005
  title: Add changelog generation with git-cliff
  priority: 5
  status: todo
  notes: Install git-cliff, add cliff.toml config, add changelog task to Makefile.toml.
- id: T-006
  title: Add publish-crates task for crates.io publishing
  priority: 6
  status: todo
  notes: Add task to Makefile.toml that runs cargo publish. Include pre-publish checks.
- id: T-007
  title: Add GitHub Release creation workflow/task
  priority: 7
  status: todo
  notes: Create cargo-make task or script to create GitHub releases with attached binaries from artifacts. May use gh CLI.
- id: T-008
  title: Add unified release task orchestrating full pipeline
  priority: 8
  status: todo
  notes: "Add 'release' and 'publish-all' tasks that orchestrate: version bump, changelog generation, build, publish to crates.io, create GitHub release."
---

## Summary

Set up a complete release pipeline for microralph including:
- Code coverage with Codecov
- Multi-platform binary builds (Linux x86_64, macOS ARM, Windows, WASM32-WASIP2)
- SemVer versioning with cargo-release
- Automated changelog generation with git-cliff
- crates.io publishing
- GitHub Releases with pre-built binaries

## Problem

microralph currently has no release infrastructure. To distribute the tool to users, we need:
1. A way to build binaries for multiple platforms
2. A way to publish to crates.io
3. A way to create GitHub releases with attached binaries
4. Version management and changelog automation
5. Code coverage to track test quality

## Goals

1. **Code Coverage**: Integrate cargo-llvm-cov with Codecov for visibility into test coverage.
2. **Multi-platform Builds**: Build binaries for Linux x86_64, macOS ARM, Windows, and WASM32-WASIP2 in CI.
3. **Artifact Storage**: Upload build artifacts to GitHub Artifacts during CI runs.
4. **Version Management**: Use cargo-release for SemVer version bumping.
5. **Changelog**: Auto-generate changelogs with git-cliff from conventional commits.
6. **crates.io Publishing**: Add cargo-make task to publish to crates.io.
7. **GitHub Releases**: Create releases with attached binaries via cargo-make task using gh CLI.
8. **Unified Release Flow**: Single `cargo make release` or `cargo make publish-all` command for full pipeline.

## Non-Goals

- Homebrew formula creation (future work)
- AUR packaging (future work)
- Docker image publishing (not needed for CLI tool)
- Automated release on tag push (prefer manual cargo-make driven releases for now)

## History

