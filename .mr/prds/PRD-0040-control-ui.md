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
    status: done
    notes: "Use Leptos 0.8 native WebSocket server functions (async Stream) to push state diffs to connected clients. On state.yaml change, broadcast updated WorktreeState to all connected clients. Client receives and updates reactive signals."
  - id: T-005
    title: "Log capture: redirect mr run output to log files"
    priority: 2
    status: done
    notes: "Modify worktree run flow to redirect stdout/stderr of the detached mr run process to .mr/worktrees/<wt-id>/run.log. This enables the UI to tail and stream logs. Ensure log rotation or truncation for long-running agents."
  - id: T-006
    title: "App shell layout: Sentry-style sidebar, dark theme"
    priority: 3
    status: done
    notes: "Build the main layout component with Thaw UI: left sidebar nav (collapsible), top bar with daemon status indicator, main content area. Dark theme by default using Thaw's theming system (CSS custom properties). Light theme toggle. Responsive for different screen sizes. Design cues: Sentry's sidebar structure, langfuse's clean data presentation."
  - id: T-007
    title: "Dashboard home page: overview cards and daemon health"
    priority: 3
    status: done
    notes: "Cards showing: total worktrees by status (active/completed/merged/failed), daemon uptime and health (green/red indicator from last_heartbeat), overlap warnings count with risk badges, recent events timeline (last 10 events across all worktrees). Use Thaw Card, Badge, Tag, and Timeline components."
  - id: T-008
    title: "Worktree list view: table with real-time status"
    priority: 3
    status: done
    notes: "Table of all worktrees with columns: PRD ID, branch, status (color-coded badge), current task, modified files count, last event, age. Sortable and filterable. Click row to navigate to detail. Use Thaw Table, Badge, and Tag components. Real-time updates via WebSocket signal."
  - id: T-009
    title: "Worktree detail view: event timeline, task progress, files"
    priority: 4
    status: done
    notes: "Detailed view for a single worktree. Sections: (1) Status header with PRD title, branch, status badge, PID. (2) Event timeline (Temporal-style) with timestamps and event types. (3) Task progress — list of tasks with status indicators. (4) Modified files list with diff preview link. (5) Merge info — whether merge was ff or agent-resolved, merge target branch. Use Thaw Timeline, Collapse, and List components."
  - id: T-010
    title: "PRD list page: all PRDs with status and dependencies"
    priority: 4
    status: done
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
    status: done
    notes: "Add UiCommand to clap CLI in src/main.rs with flags: --port (default 3939), --host (default 127.0.0.1), --open (auto-open browser). Feature-gated behind #[cfg(feature = \"ui\")]. Starts the Axum server with Leptos SSR. Prints URL to stdout with color."
  - id: T-016
    title: "cargo-make tasks for UI dev and build"
    priority: 3
    status: done
    notes: "Add to Makefile.toml: (1) cargo make ui-dev — runs cargo-leptos watch for hot reload. (2) cargo make ui-build — production build. (3) cargo make ui-test — runs UI-specific tests. Update cargo make ci to include UI lint/test when feature is enabled."
  - id: T-017
    title: "Tracing integration for UI server"
    priority: 4
    status: done
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

## 2026-03-05 — T-004 Completed
- **Task**: WebSocket server function for real-time state push
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/src/ws.rs`: SSR-only Axum WebSocket handler that upgrades HTTP to WebSocket at `/ws/state`. Sends the current `AppState` snapshot immediately on connection, then subscribes to the `broadcast::Sender<AppState>` channel and streams subsequent state updates as JSON. Handles lagged clients gracefully (logs and continues), responds to ping/pong, and cleans up on disconnect. Uses `tokio::select!` for concurrent broadcast recv and socket recv.
  - Updated `crates/mr-ui/src/app.rs`: Added client-side WebSocket connection via `connect_state_ws()` function (behind `hydrate` feature). Uses `web_sys::WebSocket` to connect to `/ws/state`, deserializes incoming JSON into `AppState`, and updates a `RwSignal<Option<AppState>>` provided as Leptos context. `HomePage` component now reactively displays daemon status, worktree count, and PRD count from the WebSocket-fed signal.
  - Updated `crates/mr-ui/src/types.rs`: Added `Serialize, Deserialize` derives to `AppState` for JSON serialization over WebSocket.
  - Updated `crates/mr-ui/src/lib.rs`: Added `pub mod ws` behind `ssr` feature gate.
  - Updated `crates/mr-ui/src/main.rs`: Added `/ws/state` GET route before Leptos routes, imported `state_ws_handler` and `axum::routing::get`.
  - Updated `crates/mr-ui/Cargo.toml`: Added `serde_json` dependency (shared), added `ws` feature to `axum`, added `web-sys` (with `WebSocket`, `MessageEvent`, `ErrorEvent`, `Window`, `Location` features) as optional dependency for `hydrate` feature.
  - All code passes `clippy::pedantic` — one targeted `#[allow(clippy::unused_async)]` on the WebSocket handler (required by Axum's Handler trait).
  - UAT passes: 756 tests, fmt-check, clippy all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes (rule 3), follows existing patterns (rule 4), separation of concerns maintained with ws.rs as a dedicated module (rule 2).

## 2026-03-05 — T-005 Completed
- **Task**: Log capture: redirect mr run output to log files
- **Status**: ✅ Done
- **Changes**:
  - Modified `src/commands/worktree.rs` — `spawn_mr_run()` now accepts a `log_path` parameter. Creates the parent directory (`.mr/worktrees/<wt-id>/`), opens the log file, and redirects both stdout and stderr to it (replacing `Stdio::null()`). Log file is truncated on each new run for simplicity (old logs overwritten).
  - Added `log_file_path()` helper in `src/commands/worktree.rs` that computes `.mr/worktrees/<wt-id>/run.log` from the project root and worktree ID.
  - Updated `cmd_wt_run()` to compute the log path after worktree registration (when wt-id is known), pass it to `spawn_mr_run()`, and store it in state.yaml via the `log_file` field.
  - Added `log_file: Option<String>` field to `WorktreeEntry` in `src/worktree/types.rs` (with `skip_serializing_if = "Option::is_none"` for backward compatibility).
  - Mirrored `log_file` field in `crates/mr-ui/src/types.rs` to keep UI types in sync.
  - Updated `wt status` detail view (`print_worktree_detail`) to display the log file path.
  - Updated CLI output in `cmd_wt_run` to show the actual log file path instead of generic worktree directory.
  - Added `log_file: None` to all existing `WorktreeEntry` struct literal constructions across the codebase (state.rs, daemon.rs, types.rs, worktree.rs, run.rs).
  - Added `log_file_path_builds_correct_path` unit test.
  - UAT passes: 757 tests (756 existing + 1 new), fmt-check, clippy all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes focused on log capture (rule 3), follows existing patterns for optional fields (rule 4), DRY via shared `log_file_path()` helper (rule 1).

## 2026-03-05 — T-015 Completed
- **Task**: mr ui CLI command with flags
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/src/serve.rs`: production server entrypoint with `serve_blocking(host, port)` function. Creates a tokio runtime, constructs `LeptosOptions` via the typed builder with configurable `site_addr`, starts `StateService` for filesystem polling, builds the Axum router with WebSocket and Leptos SSR routes, binds TCP listener, and serves. Includes `find_project_root()` helper that walks up directories looking for `.mr/`.
  - Updated `crates/mr-ui/src/lib.rs`: exported `serve` module behind `ssr` feature gate.
  - Updated `src/main.rs`: added `Ui` command variant to the `Command` enum behind `#[cfg(feature = "ui")]` with `--host` (default `127.0.0.1`), `--port` (default `3939`), and `--open` (auto-open browser) flags. Added `cmd_ui()` handler that prints the server URL with color, optionally opens the browser, then calls `mr_ui::serve::serve_blocking`. Added cross-platform `open_browser()` helper using `xdg-open` (Linux), `open` (macOS), `cmd /c start` (Windows).
  - All code behind `#[cfg(feature = "ui")]` — no impact on default binary size or compilation.
  - UAT passes: 757 tests, fmt-check, clippy all green (both with and without `ui` feature).

## 2026-03-05 — T-006 Completed
- **Task**: App shell layout: Sentry-style sidebar, dark theme
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/src/components/mod.rs`: module index exporting `layout`, `sidebar`, and `theme` submodules.
  - Created `crates/mr-ui/src/components/layout.rs`: `AppShell` component using Thaw's `Layout`, `LayoutHeader`, `LayoutSider`. Renders a persistent left sidebar with brand header, a top bar with `DaemonStatusIndicator` (colored `Badge` showing online/offline from `AppState` context) and `ThemeToggle`, and a scrollable main content area. Sentry-inspired structure.
  - Created `crates/mr-ui/src/components/sidebar.rs`: `Sidebar` component using Thaw's `NavDrawer` with `NavItem` entries for Dashboard (`/`), Worktrees (`/worktrees`), and PRDs (`/prds`). Uses `icondata_ai` icons (`AiDashboardOutlined`, `AiForkOutlined`, `AiFileOutlined`). Navigation values and hrefs wired for client-side routing.
  - Created `crates/mr-ui/src/components/theme.rs`: `ThemeProvider` wrapping Thaw's `ConfigProvider` with `Theme::dark()` by default, and `ThemeToggle` component using Thaw's `Switch` to flip between dark and light themes with emoji indicators (🌙/☀️).
  - Updated `crates/mr-ui/src/app.rs`: wrapped `Router` in `ThemeProvider` and routes in `AppShell`. Added placeholder routes for `/worktrees` (`WorktreesPage`) and `/prds` (`PrdsPage`). Moved daemon status from `HomePage` to the top bar. Simplified `HomePage` to show just worktree and PRD counts.
  - Updated `crates/mr-ui/src/lib.rs`: exported `components` module (shared, not feature-gated).
  - Updated `crates/mr-ui/Cargo.toml`: added `icondata_ai = "0.0.10"` dependency for sidebar nav icons.
  - Updated `crates/mr-ui/style/main.css`: replaced minimal placeholder styles with Sentry-inspired layout CSS. Full-height sidebar (220px), top bar with flex layout, scrollable content area. Uses Thaw CSS custom properties (`--colorNeutralBackground1`, etc.) for theme-aware colors with dark fallbacks. Responsive breakpoint at 768px collapses sidebar to 56px icon-only mode.
  - All code passes `clippy::pedantic` with targeted allows matching existing patterns (`clippy::must_use_candidate` for component fns, `clippy::wildcard_imports` for `leptos::prelude::*`).
  - UAT passes: 757 tests, fmt-check, clippy all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes (rule 3), follows existing patterns (rule 4), separation of concerns with dedicated component modules (rule 2), DRY via shared layout wrapping all routes (rule 1).

## 2026-03-05 — T-007 Completed
- **Task**: Dashboard home page: overview cards and daemon health
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/src/components/dashboard.rs`: Full dashboard home page with five components — `DashboardHome` (orchestrator), `StatusCards` (worktree count breakdown by status: total, active, completed, merged, failed using Thaw Card/Badge), `DaemonHealthCard` (online/offline badge, PID, started_at, last_heartbeat display), `OverlapWarningsCard` (warning count with risk-colored Thaw Tags for high/medium/low), `RecentEventsTimeline` (last 10 events across all worktrees sorted by timestamp, with color-coded dots for success/danger/active/neutral event types).
  - Updated `crates/mr-ui/src/components/mod.rs`: exported `dashboard` module.
  - Updated `crates/mr-ui/src/app.rs`: replaced placeholder `HomePage` (simple worktree/PRD count) with `DashboardHome` component. Added import for `DashboardHome`.
  - Updated `crates/mr-ui/style/main.css`: replaced temporary `dashboard-summary` styles with full dashboard CSS — card grid layout (`grid-template-columns: repeat(auto-fit, minmax(280px, 1fr))`), status item grid, daemon health detail rows, overlap summary with risk-colored tag overrides, and custom timeline CSS (vertical line with colored dots, event headers with type/meta/detail/timestamp).
  - Since Thaw 0.5.0-beta does not include a Timeline component, built a custom CSS timeline with vertical connector line and colored dot indicators per event type.
  - All code passes `clippy::pedantic` with targeted allows matching existing patterns.
  - UAT passes: 757 tests, fmt-check, clippy all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes (rule 3), follows existing patterns (rule 4), separation of concerns with dedicated dashboard module (rule 2).

## 2026-03-05 — T-008 Completed
- **Task**: Worktree list view: table with real-time status
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/src/components/worktrees.rs`: Full worktree list page with `WorktreeList` (orchestrator), `StatusFilter` (Thaw `Select` dropdown for filtering by worktree status), `WorktreeTable` (Thaw `Table`/`TableHeader`/`TableBody`/`TableRow`/`TableCell` with columns: PRD ID, Branch, Status, Current Task, Modified Files, Last Event, Age), `SortableHeader` (click-to-sort headers with arrow indicators), `WorktreeRow` (per-row rendering with color-coded `StatusBadge`), `StatusBadge` (maps `WorktreeStatus` to Thaw `Badge` with color), `derive_current_task()` (extracts active task from event history), `compute_age()` (formats creation date). Extracted `filter_and_sort()` helper to keep component under pedantic line limit.
  - Updated `crates/mr-ui/src/components/mod.rs`: exported `worktrees` module.
  - Updated `crates/mr-ui/src/app.rs`: replaced placeholder `WorktreesPage` with `WorktreeList` component import and usage.
  - Updated `crates/mr-ui/style/main.css`: added worktree list CSS — toolbar with filter label, table styles with sortable header highlighting, hover rows, monospaced branch/task/age columns, color-coded PRD ID, and file count tag.
  - All code passes `clippy::pedantic` with targeted allows matching existing patterns (`clippy::must_use_candidate` for component fns, `clippy::wildcard_imports` for `leptos::prelude::*`).
  - UAT passes: 757 tests, fmt-check, clippy all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes (rule 3), follows existing patterns (rule 4), separation of concerns with dedicated worktrees module (rule 2), DRY via extracted `filter_and_sort` helper (rule 1).

## 2026-03-05 — T-016 Completed
- **Task**: cargo-make tasks for UI dev and build
- **Status**: ✅ Done
- **Changes**:
  - Added `ui-dev` task to `Makefile.toml`: depends on `install-cargo-leptos`, runs `cargo leptos watch` in `crates/mr-ui` for hot-reload development.
  - Added `ui-build` task: depends on `install-cargo-leptos`, runs `cargo leptos build --release` in `crates/mr-ui` for production SSR binary + hydrated WASM bundle.
  - Added `ui-test` task: depends on `install-nextest`, runs `cargo nextest run -p mr-ui --all-features` to execute UI-specific tests (requires SSR feature for server-side modules).
  - Added `ui-clippy` task: runs `cargo clippy -p mr-ui --all-features --all-targets -- -D warnings` for pedantic linting of the UI crate.
  - Added `ui-ci` task: convenience task combining `ui-clippy` + `ui-test`.
  - Updated `ci` task to depend on `ui-clippy` and `ui-test`, ensuring the UI crate is always linted and tested as part of the full CI pipeline.
  - Fixed pre-existing clippy `collapsible_if` warning in `crates/mr-ui/src/app.rs` (collapsed nested `if let` into combined `if let && let` form).
  - Fixed pre-existing clippy `needless_raw_string_hashes` warning in `crates/mr-ui/src/state.rs` test (changed `r#"..."#` to `r"..."`).
  - Added `#![recursion_limit = "256"]` to `crates/mr-ui/src/lib.rs` to handle deep type recursion when compiling with `--all-features` (both `ssr` and `hydrate` combined).
  - UAT passes: 757 root crate tests + 11 mr-ui tests, fmt-check, clippy (including ui-clippy) all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes (rule 3), follows existing patterns for cargo-make tasks (rule 4).

## 2026-03-05 — T-009 Completed
- **Task**: Worktree detail view: event timeline, task progress, files
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/src/components/worktree_detail.rs`: Full worktree detail page with five sections — `StatusHeader` (PRD title, branch, status badge, PID, created timestamp as metadata row), `EventTimeline` (Temporal-style reverse-chronological event list with color-coded dots reusing dashboard pattern), `TaskProgress` (individual task list with status icons and progress bar derived from matching PRD's task summaries), `ModifiedFilesList` (file paths in monospaced list), `MergeInfo` (merge target branch and derived merge summary — clean/agent-resolved/failed/pending).
  - Added `TaskSummary` struct to `crates/mr-ui/src/types.rs` with `id`, `title`, `status` fields. Extended `PrdSummary` with `tasks: Vec<TaskSummary>` for individual task detail display.
  - Updated `crates/mr-ui/src/state.rs`: extended `PrdTask` frontmatter parser to capture `id` and `title`. Updated `parse_prd_summary` to populate `TaskSummary` instances in the `PrdSummary.tasks` field.
  - Updated `crates/mr-ui/src/components/mod.rs`: exported `worktree_detail` module.
  - Updated `crates/mr-ui/src/app.rs`: added `ParamSegment` import, `/worktrees/:id` route pointing to `WorktreeDetailPage` component that renders `WorktreeDetail`.
  - Updated `crates/mr-ui/src/components/worktrees.rs`: made PRD ID column in worktree table a clickable link (`<a>`) navigating to `/worktrees/{wt_id}`.
  - Updated `crates/mr-ui/style/main.css`: added comprehensive CSS for detail view — status header, metadata row, two-column grid layout (timeline + tasks), task progress bar, task list items with status icons, file list, merge info section. Responsive breakpoint at 900px collapses grid to single column.
  - All code passes `clippy::pedantic` with targeted allows matching existing patterns (`clippy::must_use_candidate`, `clippy::wildcard_imports`).
  - UAT passes: 757 root crate tests + 11 mr-ui tests, fmt-check, clippy all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes (rule 3), follows existing patterns (rule 4), separation of concerns with dedicated worktree_detail module (rule 2), DRY via reuse of event dot color mapping pattern from dashboard (rule 1).

## 2026-03-05 — T-010 Completed
- **Task**: PRD list page: all PRDs with status and dependencies
- **Status**: ✅ Done
- **Changes**:
  - Created `crates/mr-ui/src/components/prd_list.rs`: full PRD list page component with sortable table (ID, Title, Status, Dependencies, Tasks, Completion columns), status filter dropdown (All/Draft/Active/Done/Parked), color-coded status badges, dependency tags, task count tags, and completion progress bars. Includes a "+ New PRD" link placeholder for T-011.
  - Updated `crates/mr-ui/src/components/mod.rs`: registered new `prd_list` module.
  - Updated `crates/mr-ui/src/app.rs`: replaced placeholder `PrdsPage` with `PrdList` component, added import.
  - Updated `crates/mr-ui/style/main.css`: added comprehensive CSS for PRD list — toolbar layout, filter styling, new-link button with hover effect, table with sortable headers, PRD ID/title/status styling, dependency tag layout, task count tag, completion bar with fill animation, and text alignment.
  - All code passes `clippy::pedantic` with targeted allows matching existing patterns (`clippy::must_use_candidate`, `clippy::wildcard_imports`).
  - UAT passes: 11 mr-ui tests + root crate tests, fmt-check, clippy all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes (rule 3), follows existing worktree list patterns (rule 4), separation of concerns with dedicated prd_list module (rule 2), DRY via reuse of sort/filter pattern from worktrees.rs (rule 1).

## 2026-03-05 — T-017 Completed
- **Task**: Tracing integration for UI server
- **Status**: ✅ Done
- **Changes**:
  - Updated `crates/mr-ui/src/serve.rs`: added `tower_http::trace::TraceLayer` import and `.layer(TraceLayer::new_for_http())` to the Axum router middleware stack for automatic request/response logging (method, URI, status, latency).
  - Updated `crates/mr-ui/src/main.rs` (dev server): added matching `TraceLayer` import and layer to the dev server router for parity with production.
  - Updated `crates/mr-ui/src/ws.rs`: added `tracing::info!("WebSocket client connected")` at the start of `handle_state_ws` to log new WebSocket connections. Existing disconnect and lag tracing preserved.
  - Updated `crates/mr-ui/src/state.rs`: enhanced initial load tracing to include `worktrees` and `prds` counts as structured fields. Enhanced reload tracing to include `state_changed`, `prds_changed` flags and updated counts.
  - Action invocation tracing (mr new, mr wt run) deferred to T-011/T-012 when those server functions are implemented — tracing calls will be added at that point.
  - UAT passes: 11 mr-ui tests + root crate tests, fmt-check, clippy all green.
- **Constitution Compliance**: No violations. Pedantic clippy enforced (rule 8), minimal changes — only added tracing middleware and log statements (rule 3), follows existing tracing patterns already in the codebase (rule 4).
