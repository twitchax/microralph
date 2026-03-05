---
id: PRD-0040
title: "Control UI: Local Dashboard for Worktree Orchestration"
status: active
owner: twitchax
created: 2026-03-05
updated: 2026-03-05
depends_on:
  - PRD-0039
principles:
  - "Leptos 0.8 + Axum 0.8 + Thaw UI 0.5 — full-stack Rust, no JS build tooling"
  - "Sentry-like layout: left sidebar nav, card-based content, dark theme default"
  - "Design cues from langfuse (trace timelines) and Temporal (workflow state machines)"
  - "Real-time updates via Leptos native WebSocket server functions"
  - "Separate mr-ui crate, feature-gated in the main binary — mr ui command"
  - "SSR + hydration for fast initial loads; islands for selective interactivity"
  - "No authentication — local-only dashboard"
  - "Tracing throughout the UI server for observability"
  - "Constitution-compliant: pedantic clippy, DRY, separation of concerns"
references:
  - name: "Leptos 0.8 Release"
    url: "https://github.com/leptos-rs/leptos/releases/tag/v0.8.0"
  - name: "Thaw UI (Leptos 0.8 compatible)"
    url: "https://github.com/thaw-ui/thaw"
  - name: "Leptos start-axum Template"
    url: "https://github.com/leptos-rs/start-axum"
  - name: "Sentry UI (design reference)"
    url: "https://sentry.io"
  - name: "Langfuse UI (design reference)"
    url: "https://langfuse.com"
  - name: "Temporal UI (design reference)"
    url: "https://temporal.io"
  - name: "cargo-leptos Build Tool"
    url: "https://github.com/leptos-rs/cargo-leptos"
acceptance_tests:
  - id: uat-001
    name: "mr ui starts server and serves dashboard on default port"
    command: cargo make uat
    uat_status: unverified
  - id: uat-002
    name: "Dashboard renders worktree state from state.yaml"
    command: cargo make uat
    uat_status: unverified
  - id: uat-003
    name: "WebSocket pushes real-time state updates to connected clients"
    command: cargo make uat
    uat_status: unverified
  - id: uat-004
    name: "PRD creation form invokes mr new and creates a valid PRD"
    command: cargo make uat
    uat_status: unverified
  - id: uat-005
    name: "Worktree kickoff from UI triggers mr wt run successfully"
    command: cargo make uat
    uat_status: unverified
  - id: uat-006
    name: "Log streaming displays real-time run output for active worktrees"
    command: cargo make uat
    uat_status: unverified
  - id: uat-007
    name: "Dark and light theme toggle works correctly"
    command: cargo make uat
    uat_status: unverified
  - id: uat-008
    name: "UI compiles and passes clippy with pedantic lints"
    command: cargo make ci
    uat_status: unverified
tasks:
  - id: T-001
    title: "Workspace setup: create mr-ui crate with feature gate"
    priority: 1
    status: done
    notes: "Create crates/mr-ui/ with Cargo.toml (leptos 0.8, axum 0.8, thaw 0.5-beta, tracing). Add workspace member to root Cargo.toml. Feature-gate the crate in the main mr binary with a `ui` feature. Install cargo-leptos as a build dependency."
  - id: T-002
    title: "Leptos + Axum app scaffold with cargo-leptos"
    priority: 1
    status: done
    notes: "Set up the Leptos app entrypoint with Axum router, SSR + hydration, and basic cargo-leptos config (Cargo.toml metadata for leptos). Create src/app.rs, src/main.rs (server), src/lib.rs (client hydration). Verify the app compiles and serves a hello-world page."
  - id: T-003
    title: "State service: read and watch state.yaml"
    priority: 2
    status: done
    notes: "Create a server-side StateService that reads .mr/worktrees/state.yaml using the existing StateManager. Use notify or polling (every 2s) to detect changes. Expose state via an Arc<RwLock<WorktreeState>> shared across Axum handlers. Also read .mr/prds/ for PRD metadata."
  - id: T-004
    title: "WebSocket server function for real-time state push"
    priority: 2
    status: todo
    notes: "Use Leptos 0.8 native WebSocket server functions (async Stream) to push state diffs to connected clients. On state.yaml change, broadcast updated WorktreeState to all connected clients. Client receives and updates reactive signals."
  - id: T-005
    title: "Log capture: redirect mr run output to log files"
    priority: 2
    status: todo
    notes: "Modify worktree run flow to redirect stdout/stderr of the detached mr run process to .mr/worktrees/<wt-id>/run.log. This enables the UI to tail and stream logs. Ensure log rotation or truncation for long-running agents."
  - id: T-006
    title: "App shell layout: Sentry-style sidebar, dark theme"
    priority: 3
    status: todo
    notes: "Build the main layout component with Thaw UI: left sidebar nav (collapsible), top bar with daemon status indicator, main content area. Dark theme by default using Thaw's theming system (CSS custom properties). Light theme toggle. Responsive for different screen sizes. Design cues: Sentry's sidebar structure, langfuse's clean data presentation."
  - id: T-007
    title: "Dashboard home page: overview cards and daemon health"
    priority: 3
    status: todo
    notes: "Cards showing: total worktrees by status (active/completed/merged/failed), daemon uptime and health (green/red indicator from last_heartbeat), overlap warnings count with risk badges, recent events timeline (last 10 events across all worktrees). Use Thaw Card, Badge, Tag, and Timeline components."
  - id: T-008
    title: "Worktree list view: table with real-time status"
    priority: 3
    status: todo
    notes: "Table of all worktrees with columns: PRD ID, branch, status (color-coded badge), current task, modified files count, last event, age. Sortable and filterable. Click row to navigate to detail. Use Thaw Table, Badge, and Tag components. Real-time updates via WebSocket signal."
  - id: T-009
    title: "Worktree detail view: event timeline, task progress, files"
    priority: 4
    status: todo
    notes: "Detailed view for a single worktree. Sections: (1) Status header with PRD title, branch, status badge, PID. (2) Event timeline (Temporal-style) with timestamps and event types. (3) Task progress — list of tasks with status indicators. (4) Modified files list with diff preview link. (5) Merge info — whether merge was ff or agent-resolved, merge target branch. Use Thaw Timeline, Collapse, and List components."
  - id: T-010
    title: "PRD list page: all PRDs with status and dependencies"
    priority: 4
    status: todo
    notes: "Table of all PRDs parsed from .mr/prds/. Columns: ID, title, status (badge), depends_on, task count, task completion percentage. Filterable by status. Link to create new PRD. Use existing PRD parsing logic from src/prd/."
  - id: T-011
    title: "PRD creation form: gather context and invoke mr new"
    priority: 5
    status: todo
    notes: "Form with fields: slug, upfront context (textarea), runner selection (dropdown), model override (optional). On submit, invoke mr new --context '<context>' server-side via tokio::process::Command. Show spinner during creation. On success, refresh PRD list and offer to kick off a worktree."
  - id: T-012
    title: "Worktree kickoff: trigger mr wt run from UI"
    priority: 5
    status: todo
    notes: "Action button on PRD detail or PRD list to start a worktree. Invokes mr wt run <prd-id> --runner <runner> --model <model> server-side. Show confirmation dialog with runner/model selection. On success, navigate to worktree list. Auto-refreshes via WebSocket when state.yaml updates."
  - id: T-013
    title: "Log streaming view: real-time log tail via WebSocket"
    priority: 5
    status: todo
    notes: "Dedicated log viewer for a worktree. Server-side tails .mr/worktrees/<wt-id>/run.log and streams via WebSocket server function. Client renders in a scrollable, monospaced container with auto-scroll. Support pause/resume scrolling. Highlight errors in red. Use a pre/code block styled like a terminal."
  - id: T-014
    title: "Overlap risk visualization"
    priority: 6
    status: todo
    notes: "Visualize file overlap warnings from state.yaml. Show as a matrix/heatmap: worktrees on axes, risk level as color intensity. Or as a list of overlap warnings with affected worktrees and files. Use Thaw Table with color-coded cells. Link to affected worktree details."
  - id: T-015
    title: "mr ui CLI command with flags"
    priority: 2
    status: todo
    notes: "Add UiCommand to clap CLI in src/main.rs with flags: --port (default 3939), --host (default 127.0.0.1), --open (auto-open browser). Feature-gated behind #[cfg(feature = \"ui\")]. Starts the Axum server with Leptos SSR. Prints URL to stdout with color."
  - id: T-016
    title: "cargo-make tasks for UI dev and build"
    priority: 3
    status: todo
    notes: "Add to Makefile.toml: (1) cargo make ui-dev — runs cargo-leptos watch for hot reload. (2) cargo make ui-build — production build. (3) cargo make ui-test — runs UI-specific tests. Update cargo make ci to include UI lint/test when feature is enabled."
  - id: T-017
    title: "Tracing integration for UI server"
    priority: 4
    status: todo
    notes: "Use tracing throughout the UI server: request/response logging via tower-http TraceLayer, WebSocket connection events, state.yaml reload events, action invocations (mr new, mr wt run). Use existing tracing-subscriber setup from main binary."
  - id: T-018
    title: "Documentation and AGENTS.md update"
    priority: 6
    status: todo
    notes: "Add UI section to AGENTS.md covering: architecture overview, how to run (mr ui), dev workflow (cargo make ui-dev), component structure, how state flows from daemon to UI. Update README.md with UI screenshot placeholder and usage."
---

# Summary

A local web dashboard for visualizing and controlling microralph's worktree orchestration system. Built with Leptos 0.8, Axum 0.8, and Thaw UI 0.5, the UI provides a Sentry-inspired dark-themed interface for monitoring worktree status, streaming agent logs, creating PRDs, and kicking off parallel worktree runs — all in real-time via WebSocket.

# Problem

The worktree orchestration daemon (PRD-0039) manages parallel PRD execution via git worktrees, but all interaction is through CLI commands (`mr wt list`, `mr wt status`, etc.). There is no visual overview of what's happening across worktrees: which agents are active, what tasks they're on, whether merges succeeded or required conflict resolution, or which files are at risk of overlap. Operators need to run multiple CLI commands and mentally reconstruct the system state. A visual dashboard would make orchestration observable, actionable, and approachable.

# Goals

1. Provide a real-time dashboard showing all worktree status, daemon health, and overlap warnings at a glance.
2. Enable visual monitoring of individual worktree progress — event timelines, task completion, modified files, and merge outcomes.
3. Stream agent logs in real-time from active worktrees directly in the browser.
4. Allow PRD creation and worktree kickoff from the UI, reducing context switching to the terminal.
5. Deliver a polished, modern dark-themed UI inspired by Sentry's layout, langfuse's data presentation, and Temporal's workflow visualization.
6. Use Leptos 0.8's SSR + hydration for fast initial loads and native WebSocket server functions for real-time updates.
7. Package as a feature-gated crate accessible via `mr ui` with zero additional runtime dependencies for users who don't need it.

# Technical Approach

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Browser                               │
│  ┌────────────┐  ┌──────────────┐  ┌─────────────────────┐  │
│  │  SSR HTML   │  │  Hydrated    │  │  WebSocket Client   │  │
│  │  (initial)  │  │  Islands     │  │  (state + logs)     │  │
│  └────────────┘  └──────────────┘  └─────────┬───────────┘  │
└──────────────────────────────────────────────┼───────────────┘
                                               │ ws://
┌──────────────────────────────────────────────┼───────────────┐
│                   Axum Server (mr ui)         │               │
│  ┌──────────────┐  ┌───────────────┐  ┌─────┴─────────────┐ │
│  │ Leptos SSR   │  │ Server Fns    │  │ WS Server Fns     │ │
│  │ (HTML render)│  │ (actions)     │  │ (state + log push)│ │
│  └──────────────┘  └───────┬───────┘  └─────┬─────────────┘ │
│                            │                │               │
│  ┌─────────────────────────┴────────────────┴─────────────┐ │
│  │                   StateService                          │ │
│  │  - Reads .mr/worktrees/state.yaml (polling/notify)     │ │
│  │  - Reads .mr/prds/ (PRD metadata)                      │ │
│  │  - Tails .mr/worktrees/<wt-id>/run.log                 │ │
│  │  - Shared via Arc<RwLock<AppState>>                     │ │
│  └─────────────────────────────────────────────────────────┘ │
│                            │                                 │
│              ┌─────────────┴──────────────┐                  │
│              │  tokio::process::Command    │                  │
│              │  (mr new, mr wt run, etc.)  │                  │
│              └────────────────────────────┘                  │
└──────────────────────────────────────────────────────────────┘
                             │
                    ┌────────┴────────┐
                    │  Filesystem     │
                    │  state.yaml     │
                    │  .mr/prds/      │
                    │  run.log files  │
                    └─────────────────┘
```

## Crate Structure

The UI lives in a separate crate within the workspace:

```
crates/
  mr-ui/
    Cargo.toml          # leptos 0.8, axum 0.8, thaw 0.5, tower-http, tracing
    src/
      lib.rs            # Client-side hydration entrypoint
      main.rs           # Server-side Axum entrypoint
      app.rs            # Root App component, router, layout
      state.rs          # StateService: reads state.yaml + PRDs, broadcasts changes
      log_stream.rs     # Log file tailing + WebSocket streaming
      pages/
        dashboard.rs    # Home: overview cards, daemon health, recent events
        worktrees.rs    # Worktree list table
        worktree.rs     # Worktree detail: timeline, tasks, files, merge info
        prds.rs         # PRD list
        prd_new.rs      # PRD creation form
        logs.rs         # Log streaming viewer
      components/
        layout.rs       # App shell: sidebar, topbar, content area
        sidebar.rs      # Collapsible sidebar nav
        status_badge.rs # Color-coded status badges
        event_timeline.rs # Temporal-style event timeline
        overlap_matrix.rs # File overlap risk visualization
        theme.rs        # Dark/light theme provider and toggle
    style/
      main.css          # Custom CSS overrides for Thaw theme tokens
```

Feature-gate in root `Cargo.toml`:
```toml
[features]
ui = ["dep:mr-ui"]

[dependencies]
mr-ui = { path = "crates/mr-ui", optional = true }
```

## Key Technology Choices

- **Leptos 0.8**: SSR + hydration with native WebSocket server functions. Islands architecture for selective hydration of interactive components (e.g., the log viewer, real-time tables) while static content stays server-rendered.
- **Axum 0.8**: HTTP server with tower-http middleware for tracing, compression, and static file serving.
- **Thaw UI 0.5-beta**: Fluent Design component library for Leptos 0.8. Provides Table, Card, Badge, Tag, Timeline, Dialog, Form, Button, and theming primitives.
- **Native WebSocket Server Functions**: Leptos 0.8 supports defining server functions that communicate over WebSocket using async Streams. Used for state push and log streaming — no external WebSocket crate needed.
- **cargo-leptos**: Build tool for coordinating server and client (WASM) compilation, asset bundling, and hot reload during development.

## Real-Time Data Flow

1. `StateService` polls `state.yaml` every 2 seconds (or uses `notify` for filesystem events).
2. On change, computes a diff and pushes the updated state via a broadcast channel.
3. WebSocket server function subscribes to the broadcast channel and yields state updates as a Stream.
4. Client receives updates and patches reactive signals — Leptos fine-grained reactivity updates only affected DOM nodes.

## Log Streaming

1. `mr wt run` is modified to redirect stdout/stderr to `.mr/worktrees/<wt-id>/run.log`.
2. The UI server tails the log file using async file I/O (seek to end, poll for new bytes).
3. New log lines are pushed to the client via a dedicated WebSocket server function.
4. Client renders in a terminal-styled container with auto-scroll and pause/resume.

## Design Language

- **Layout**: Sentry-inspired — persistent left sidebar with icon+label nav items, top bar with daemon status pill, main content area with card-based sections.
- **Data Presentation**: Langfuse-inspired — clean tables with sortable columns, timeline traces for events, structured metadata cards.
- **Workflow Visualization**: Temporal-inspired — event timeline with state transitions, clear status progression from Active → Completed → Merging → Merged.
- **Theme**: Dark by default using Thaw's CSS custom property theming. Light mode toggle in the top bar. Colors encode status: green (active/merged), yellow (merging), red (failed/conflicted), gray (abandoned).

# Assumptions

1. PRD-0039 (Worktree Orchestration) is at least partially complete — `state.yaml` exists and the daemon is functional.
2. The host machine has a browser available for viewing the UI (local development use case).
3. `cargo-leptos` is available or can be installed as a build dependency.
4. The nightly Rust toolchain (already configured) supports Leptos 0.8 WASM compilation.
5. Users are comfortable with `mr ui` starting a local server (no deployment target for v1).

# Constraints

1. **Feature-gated**: The UI must not affect binary size or compilation time when the `ui` feature is disabled. All UI dependencies are behind the feature gate.
2. **No external JS**: The UI is pure Rust (Leptos compiles to WASM). No npm, webpack, or JS build tooling.
3. **Local only**: No authentication, no HTTPS, no multi-user support. Binds to 127.0.0.1 by default.
4. **Read-mostly**: The UI reads state from the daemon's state.yaml. Write operations (create PRD, kick off worktree) shell out to existing `mr` CLI commands rather than reimplementing logic.
5. **Daemon independence**: The UI server is a separate process from the daemon. It reads the same state file but does not interfere with daemon operation.
6. **Clippy pedantic**: All production code in the UI crate must pass `clippy::pedantic`.

# References to Code

- `src/worktree/types.rs` — `WorktreeState`, `WorktreeEntry`, `WorktreeStatus`, `WorktreeEvent`, `OverlapWarning` structs (shared types for the UI)
- `src/worktree/state.rs` — `StateManager` for reading/writing state.yaml (reuse for the state service)
- `src/worktree/ipc.rs` — `IpcMessage`, `IpcResponse` types (reference for event types)
- `src/worktree/daemon.rs` — Daemon heartbeat and auto-merge logic (informs what the UI should display)
- `src/commands/worktree.rs` — CLI command handlers for `mr wt` (actions the UI will invoke)
- `src/prd/` — PRD parsing, indexing, and types (reuse for PRD list page)
- `src/main.rs` — Clap CLI structure (add `mr ui` subcommand here)
- `Makefile.toml` — Build tasks (add UI tasks here)

# Non-Goals (MVP)

1. **Remote access / deployment**: No support for exposing the UI over the network, TLS, or authentication.
2. **PRD editing from UI**: Only creation via `mr new --context`. Full interactive editing remains CLI-only.
3. **Merge controls**: Merges are automatic via the daemon. No manual merge trigger from the UI.
4. **Conflict resolution UI**: Conflicts are resolved by agents. The UI shows status but doesn't provide a merge conflict editor.
5. **Mobile-optimized layout**: Responsive but not optimized for small screens.
6. **Persistent UI state**: No user preferences, bookmarks, or session persistence across browser reloads.
7. **Multi-repo support**: Dashboard shows the current repo's worktrees only.

# History

## 2026-03-05 — T-001 Completed
- **Task**: Workspace setup: create mr-ui crate with feature gate
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/` directory with `Cargo.toml` (leptos 0.8, axum 0.8, thaw 0.5.0-beta, tracing, tower-http, tokio — server deps behind `ssr` feature, hydrate support behind `hydrate` feature)
  - Created `crates/mr-ui/src/lib.rs` with clippy::pedantic denials matching root crate style
  - Added `[workspace]` section to root `Cargo.toml` with `members = [".", "crates/mr-ui"]` and `default-members = ["."]` to preserve existing CI behavior
  - Added `ui` feature and `mr-ui` optional dependency (with `ssr` feature) to root `Cargo.toml`
  - Added `install-cargo-leptos` task to `Makefile.toml` for build tooling
  - UAT passes: 756 tests, fmt-check, clippy all green

## 2026-03-05 — T-002 Completed
- **Task**: Leptos + Axum app scaffold with cargo-leptos
- **Status**: ✅ Done
- **Changes**:
  - Updated `crates/mr-ui/Cargo.toml`: added `[lib]` section with `crate-type = ["cdylib", "rlib"]`, added `leptos_meta`, `console_error_panic_hook`, `wasm-bindgen` dependencies, added `leptos_meta/ssr` and `leptos_router/ssr` to `ssr` feature, added hydrate feature deps, added `[package.metadata.leptos]` section with port 3939, output-name, style-file, and cargo-leptos build config
  - Created `crates/mr-ui/src/app.rs`: root `App` component with `leptos_router` (`Router`/`Routes`/`Route`), `shell()` function for SSR HTML rendering with `AutoReload`/`HydrationScripts`/`MetaTags`, `HomePage` component with hello-world content, Thaw-compatible dark theme CSS reference
  - Created `crates/mr-ui/src/main.rs`: server-side Axum entrypoint behind `ssr` feature, sets up `LeptosRoutes` with SSR shell, includes `file_and_error_handler` fallback, empty `main()` for non-ssr (hydration handled in lib.rs)
  - Updated `crates/mr-ui/src/lib.rs`: added `pub mod app` export and `hydrate()` function behind `hydrate` feature using `wasm_bindgen` + `console_error_panic_hook` + `leptos::mount::hydrate_body`
  - Created `crates/mr-ui/style/main.css`: minimal dark-theme base styles (background, text color, typography)
  - Added `[profile.wasm-release]` to root `Cargo.toml` for optimized WASM bundle (opt-level z, LTO, single codegen unit)
  - All code passes `clippy::pedantic` with targeted `#![allow(clippy::must_use_candidate)]` in app.rs (Leptos component functions don't benefit from `#[must_use]`) and `#[allow(clippy::wildcard_imports)]` for `leptos::prelude::*` (canonical Leptos import pattern)
  - UAT passes: 756 tests, fmt-check, clippy all green
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes (rule 3), follows existing patterns (rule 4).

## 2026-03-05 — T-003 Completed
- **Task**: State service: read and watch state.yaml
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/src/types.rs`: UI-specific data types mirroring the worktree state YAML schema (`WorktreeState`, `DaemonInfo`, `WorktreeEntry`, `WorktreeStatus`, `WorktreeEvent`, `EventType`, `OverlapWarning`, `OverlapRisk`) and PRD summary types (`PrdSummary`, `AppState`). These are duplicated from the root crate because the UI crate cannot depend on the root binary crate (circular dependency). Clearly documented to stay in sync.
  - Created `crates/mr-ui/src/state.rs`: server-side `StateService` that polls `.mr/worktrees/state.yaml` and `.mr/prds/` every 2 seconds. Detects changes via file modification timestamps. Exposes combined state via `Arc<RwLock<AppState>>`. Includes `tokio::sync::broadcast` channel for future WebSocket push (T-004). Graceful degradation on missing/malformed files. 9 comprehensive async tests covering YAML parsing, PRD scanning, sort order, malformed input handling, initial load, and broadcast notification.
  - Updated `crates/mr-ui/Cargo.toml`: added `serde` (with `derive` feature), `serde_yaml` (SSR-only), `tempfile` (dev-dependency). Updated `tokio` features to include `time`, `fs`, `sync`, `macros`. Added `dep:serde_yaml` to `ssr` feature list.
  - Updated `crates/mr-ui/src/lib.rs`: added `pub mod types` (shared) and `#[cfg(feature = "ssr")] pub mod state` (server-only).
  - Updated `crates/mr-ui/src/main.rs`: integrated `StateService` — starts polling on server boot, shared state and broadcast sender injected as Axum `Extension` layers alongside existing `LeptosOptions` state.
  - All code passes `clippy::pedantic` with no suppressions in new modules.
  - UAT passes: 756 tests (root crate), 11 tests (mr-ui with ssr), fmt-check, clippy all green.
- **Constitution Compliance**: Rule 1 (DRY) — worktree/PRD types are duplicated in the UI crate due to the circular dependency constraint (root binary depends on mr-ui, so mr-ui cannot depend on root). This is documented in the types module header with a clear note to keep in sync. A shared types crate would resolve this but was deemed too large a refactor for this task (rule 3, minimal changes). All other rules fully compliant.
