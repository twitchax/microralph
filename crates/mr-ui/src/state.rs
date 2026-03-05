//! Server-side state service that reads and watches worktree state.
//!
//! Polls `.mr/worktrees/state.yaml` and `.mr/prds/` every 2 seconds,
//! broadcasting changes to connected WebSocket clients via a
//! `tokio::sync::broadcast` channel.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use tokio::sync::{RwLock, broadcast};
use tokio::time;

use crate::types::{AppState, PrdSummary, TaskSummary, WorktreeState};

// ── Constants ───────────────────────────────────────────────────────

/// Polling interval for filesystem changes.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// Broadcast channel capacity for state updates.
const BROADCAST_CAPACITY: usize = 16;

// ── Private PRD parsing types ───────────────────────────────────────

/// Minimal PRD frontmatter for extracting summary information.
///
/// Only the fields needed for [`PrdSummary`] are included.
#[derive(serde::Deserialize)]
struct PrdFrontmatter {
    id: String,
    title: String,
    #[serde(default)]
    status: String,
    #[serde(default)]
    depends_on: Option<Vec<String>>,
    #[serde(default)]
    tasks: Option<Vec<PrdTask>>,
}

/// Minimal task entry for counting completion progress.
#[derive(serde::Deserialize)]
struct PrdTask {
    #[serde(default)]
    id: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    status: String,
}

// ── State service ───────────────────────────────────────────────────

/// Server-side service that reads `.mr/worktrees/state.yaml` and `.mr/prds/`,
/// exposing the combined state via an `Arc<RwLock<AppState>>`.
///
/// The service polls the filesystem every 2 seconds and broadcasts changes
/// to subscribers (used by WebSocket server functions in future tasks).
pub struct StateService {
    root: PathBuf,
    shared: Arc<RwLock<AppState>>,
    tx: broadcast::Sender<AppState>,
}

impl StateService {
    /// Creates a new `StateService` rooted at the given project directory.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);

        Self {
            root,
            shared: Arc::new(RwLock::new(AppState::default())),
            tx,
        }
    }

    /// Returns a clone of the shared state handle.
    #[must_use]
    pub fn shared(&self) -> Arc<RwLock<AppState>> {
        Arc::clone(&self.shared)
    }

    /// Returns a new broadcast receiver for state change notifications.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<AppState> {
        self.tx.subscribe()
    }

    /// Returns a clone of the broadcast sender for state change notifications.
    #[must_use]
    pub fn sender(&self) -> broadcast::Sender<AppState> {
        self.tx.clone()
    }

    /// Performs the initial state load and starts the polling loop.
    ///
    /// This method runs indefinitely. Spawn it as a background tokio task.
    pub async fn run(self) {
        // Perform initial load.
        let initial = load_app_state(&self.root).await;
        {
            let mut guard = self.shared.write().await;
            *guard = initial.clone();
        }
        tracing::info!(
            worktrees = initial.worktree_state.worktrees.len(),
            prds = initial.prds.len(),
            "state service: initial load complete"
        );

        let mut last_state_mtime: Option<SystemTime> = None;
        let mut last_prds_scan_mtime: Option<SystemTime> = None;
        let mut interval = time::interval(POLL_INTERVAL);

        loop {
            interval.tick().await;

            let state_path = self.root.join(".mr/worktrees/state.yaml");
            let prds_dir = self.root.join(".mr/prds");

            let state_mtime = file_mtime(&state_path).await;
            let prds_mtime = dir_latest_mtime(&prds_dir).await;

            let state_changed = state_mtime != last_state_mtime;
            let prds_changed = prds_mtime != last_prds_scan_mtime;

            if state_changed || prds_changed {
                last_state_mtime = state_mtime;
                last_prds_scan_mtime = prds_mtime;

                let new_state = load_app_state(&self.root).await;
                {
                    let mut guard = self.shared.write().await;
                    *guard = new_state.clone();
                }

                // Broadcast to WebSocket subscribers (OK if nobody is listening).
                let _ = self.tx.send(new_state.clone());

                tracing::debug!(
                    state_changed,
                    prds_changed,
                    worktrees = new_state.worktree_state.worktrees.len(),
                    prds = new_state.prds.len(),
                    "state service: detected changes, state reloaded"
                );
            }
        }
    }
}

// ── Loaders ─────────────────────────────────────────────────────────

/// Loads the combined application state from disk.
async fn load_app_state(root: &Path) -> AppState {
    let worktree_state = load_worktree_state(root).await;
    let prds = load_prd_summaries(root).await;

    AppState {
        worktree_state,
        prds,
    }
}

/// Reads and parses `.mr/worktrees/state.yaml`.
///
/// Returns [`WorktreeState::default()`] if the file does not exist or is malformed.
async fn load_worktree_state(root: &Path) -> WorktreeState {
    let path = root.join(".mr/worktrees/state.yaml");

    if let Ok(contents) = tokio::fs::read_to_string(&path).await {
        match serde_yaml::from_str(&contents) {
            Ok(state) => state,
            Err(e) => {
                tracing::warn!("failed to parse state.yaml: {e}");
                WorktreeState::default()
            }
        }
    } else {
        tracing::debug!("state.yaml not found, using defaults");
        WorktreeState::default()
    }
}

/// Scans `.mr/prds/` for PRD files and extracts summary metadata.
async fn load_prd_summaries(root: &Path) -> Vec<PrdSummary> {
    let prds_dir = root.join(".mr/prds");

    let Ok(mut entries) = tokio::fs::read_dir(&prds_dir).await else {
        tracing::debug!("prds directory not found");
        return Vec::new();
    };

    let mut summaries = Vec::new();

    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();

        if path.extension().is_some_and(|ext| ext == "md")
            && let Some(summary) = parse_prd_summary(&path).await
        {
            summaries.push(summary);
        }
    }

    summaries.sort_by(|a, b| a.id.cmp(&b.id));
    summaries
}

/// Parses a single PRD file and extracts summary information.
///
/// Returns `None` if the file is unreadable or the frontmatter is malformed.
async fn parse_prd_summary(path: &Path) -> Option<PrdSummary> {
    let contents = tokio::fs::read_to_string(path).await.ok()?;

    // Extract YAML frontmatter between --- delimiters.
    let trimmed = contents.trim_start();

    if !trimmed.starts_with("---") {
        return None;
    }

    let after_open = trimmed.strip_prefix("---")?.strip_prefix('\n')?;
    let close_pos = after_open.find("\n---")?;
    let frontmatter_str = &after_open[..close_pos];

    let fm: PrdFrontmatter = serde_yaml::from_str(frontmatter_str).ok()?;

    let tasks = fm.tasks.as_deref().unwrap_or_default();
    let total_tasks = tasks.len();
    let completed_tasks = tasks.iter().filter(|t| t.status == "done").count();
    let task_summaries = tasks
        .iter()
        .map(|t| TaskSummary {
            id: t.id.clone(),
            title: t.title.clone(),
            status: t.status.clone(),
        })
        .collect();

    Some(PrdSummary {
        id: fm.id,
        title: fm.title,
        status: fm.status,
        completed_tasks,
        total_tasks,
        depends_on: fm.depends_on.unwrap_or_default(),
        tasks: task_summaries,
    })
}

// ── Filesystem helpers ──────────────────────────────────────────────

/// Returns the modification time of a file, if available.
async fn file_mtime(path: &Path) -> Option<SystemTime> {
    tokio::fs::metadata(path)
        .await
        .ok()
        .and_then(|m| m.modified().ok())
}

/// Returns the latest modification time across all entries in a directory.
async fn dir_latest_mtime(dir: &Path) -> Option<SystemTime> {
    let mut entries = tokio::fs::read_dir(dir).await.ok()?;
    let mut latest: Option<SystemTime> = None;

    while let Ok(Some(entry)) = entries.next_entry().await {
        if let Ok(meta) = entry.metadata().await
            && let Ok(mtime) = meta.modified()
        {
            latest = Some(latest.map_or(mtime, |l: SystemTime| l.max(mtime)));
        }
    }

    latest
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_temp_root(tmp: &TempDir) -> PathBuf {
        let root = tmp.path().to_path_buf();
        std::fs::create_dir_all(root.join(".mr/worktrees")).unwrap();
        std::fs::create_dir_all(root.join(".mr/prds")).unwrap();
        root
    }

    #[tokio::test]
    async fn load_worktree_state_returns_default_when_missing() {
        let tmp = TempDir::new().unwrap();
        let root = setup_temp_root(&tmp);

        let state = load_worktree_state(&root).await;
        assert_eq!(state, WorktreeState::default());
    }

    #[tokio::test]
    async fn load_worktree_state_parses_yaml() {
        let tmp = TempDir::new().unwrap();
        let root = setup_temp_root(&tmp);

        let yaml = r#"version: 1
daemon:
  pid: 12345
  started_at: "2026-03-04T22:00:00Z"
  idle_timeout_hours: 3
  last_heartbeat: "2026-03-04T22:30:00Z"
worktrees:
  - id: wt-001
    prd: PRD-0039
    branch: microralph-prd-39
    path: /tmp/test
    status: active
    created_at: "2026-03-04T22:00:00Z"
    updated_at: "2026-03-04T22:00:00Z"
"#;

        std::fs::write(root.join(".mr/worktrees/state.yaml"), yaml).unwrap();

        let state = load_worktree_state(&root).await;
        assert_eq!(state.version, 1);
        assert!(state.daemon.is_some());
        assert_eq!(state.worktrees.len(), 1);
        assert_eq!(state.worktrees[0].id, "wt-001");
    }

    #[tokio::test]
    async fn load_prd_summaries_returns_empty_when_no_dir() {
        let tmp = TempDir::new().unwrap();
        // Don't create .mr/prds/.
        let root = tmp.path().to_path_buf();

        let summaries = load_prd_summaries(&root).await;
        assert!(summaries.is_empty());
    }

    #[tokio::test]
    async fn load_prd_summaries_parses_prds() {
        let tmp = TempDir::new().unwrap();
        let root = setup_temp_root(&tmp);

        let prd_content = r"---
id: PRD-0001
title: Test PRD
status: active
depends_on:
  - PRD-0000
tasks:
  - id: T-001
    title: Task one
    priority: 1
    status: done
  - id: T-002
    title: Task two
    priority: 2
    status: todo
---

# Summary

Test body.
";

        std::fs::write(root.join(".mr/prds/PRD-0001-test.md"), prd_content).unwrap();

        let summaries = load_prd_summaries(&root).await;
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, "PRD-0001");
        assert_eq!(summaries[0].title, "Test PRD");
        assert_eq!(summaries[0].status, "active");
        assert_eq!(summaries[0].completed_tasks, 1);
        assert_eq!(summaries[0].total_tasks, 2);
        assert_eq!(summaries[0].depends_on, vec!["PRD-0000"]);
    }

    #[tokio::test]
    async fn load_prd_summaries_sorts_by_id() {
        let tmp = TempDir::new().unwrap();
        let root = setup_temp_root(&tmp);

        for (id, title) in [
            ("PRD-0003", "Third"),
            ("PRD-0001", "First"),
            ("PRD-0002", "Second"),
        ] {
            let content = format!("---\nid: {id}\ntitle: {title}\nstatus: active\n---\n\n# Body\n");
            std::fs::write(root.join(format!(".mr/prds/{id}-test.md")), content).unwrap();
        }

        let summaries = load_prd_summaries(&root).await;
        assert_eq!(summaries.len(), 3);
        assert_eq!(summaries[0].id, "PRD-0001");
        assert_eq!(summaries[1].id, "PRD-0002");
        assert_eq!(summaries[2].id, "PRD-0003");
    }

    #[tokio::test]
    async fn parse_prd_summary_returns_none_for_malformed() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.md");
        std::fs::write(&path, "not a valid PRD").unwrap();

        let result = parse_prd_summary(&path).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn parse_prd_summary_returns_none_for_missing_frontmatter_close() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("unclosed.md");
        std::fs::write(&path, "---\nid: PRD-0001\ntitle: Test\n\n# Body").unwrap();

        let result = parse_prd_summary(&path).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn state_service_initial_load() {
        let tmp = TempDir::new().unwrap();
        let root = setup_temp_root(&tmp);

        let service = StateService::new(root);
        let shared = service.shared();

        let handle = tokio::spawn(service.run());
        tokio::time::sleep(Duration::from_millis(200)).await;

        let state = shared.read().await;
        assert_eq!(state.worktree_state, WorktreeState::default());
        assert!(state.prds.is_empty());

        handle.abort();
    }

    #[tokio::test]
    async fn state_service_broadcasts_on_change() {
        let tmp = TempDir::new().unwrap();
        let root = setup_temp_root(&tmp);

        let service = StateService::new(root.clone());
        let shared = service.shared();
        let mut rx = service.subscribe();

        let handle = tokio::spawn(service.run());

        // Wait for initial load.
        tokio::time::sleep(Duration::from_millis(200)).await;

        // Write a state file to trigger a change on the next poll.
        let yaml = "version: 1\nworktrees: []\n";
        std::fs::write(root.join(".mr/worktrees/state.yaml"), yaml).unwrap();

        // Wait for the polling interval to pick up the change.
        let update = tokio::time::timeout(Duration::from_secs(5), rx.recv()).await;
        assert!(update.is_ok(), "should have received a broadcast");

        let state = shared.read().await;
        assert_eq!(state.worktree_state.version, 1);

        handle.abort();
    }

    /// Writes a realistic state.yaml and PRD, then returns the loaded `AppState`.
    async fn setup_dashboard_state() -> (TempDir, AppState) {
        use crate::types::WorktreeStatus;
        let tmp = TempDir::new().unwrap();
        let root = setup_temp_root(&tmp);

        let yaml = r#"version: 1
daemon:
  pid: 99999
  started_at: "2026-03-04T20:00:00Z"
  idle_timeout_hours: 3
  last_heartbeat: "2026-03-04T22:30:00Z"
worktrees:
  - id: wt-001
    prd: PRD-0001
    branch: microralph-prd-1
    path: /tmp/wt1
    status: active
    created_at: "2026-03-04T20:00:00Z"
    updated_at: "2026-03-04T22:00:00Z"
    modified_files: [src/main.rs]
    events:
      - { timestamp: "2026-03-04T20:00:00Z", type: created }
      - { timestamp: "2026-03-04T20:01:00Z", type: run_started, detail: "T-001" }
  - id: wt-002
    prd: PRD-0002
    branch: microralph-prd-2
    path: /tmp/wt2
    status: merged
    created_at: "2026-03-04T18:00:00Z"
    updated_at: "2026-03-04T21:00:00Z"
    modified_files: [src/main.rs, src/lib.rs]
    events:
      - { timestamp: "2026-03-04T18:00:00Z", type: created }
      - { timestamp: "2026-03-04T21:00:00Z", type: merge_completed }
  - id: wt-003
    prd: PRD-0003
    branch: microralph-prd-3
    path: /tmp/wt3
    status: merge_failed
    created_at: "2026-03-04T19:00:00Z"
    updated_at: "2026-03-04T22:30:00Z"
overlap_warnings:
  - worktrees: [wt-001, wt-002]
    files: [src/main.rs]
    risk: high
"#;
        std::fs::write(root.join(".mr/worktrees/state.yaml"), yaml).unwrap();

        let prd = "---\nid: PRD-0001\ntitle: Initial Setup\nstatus: active\ntasks:\n  \
                   - { id: T-001, title: Do thing, priority: 1, status: done }\n  \
                   - { id: T-002, title: Do other, priority: 2, status: todo }\n---\n\n# Body\n";
        std::fs::write(root.join(".mr/prds/PRD-0001-setup.md"), prd).unwrap();

        let state = load_app_state(&root).await;

        // Quick sanity: statuses parse as expected.
        let counts: Vec<usize> = [
            WorktreeStatus::Active,
            WorktreeStatus::Merged,
            WorktreeStatus::MergeFailed,
        ]
        .iter()
        .map(|s| {
            state
                .worktree_state
                .worktrees
                .iter()
                .filter(|wt| &wt.status == s)
                .count()
        })
        .collect();
        assert_eq!(counts, vec![1, 1, 1]);

        (tmp, state)
    }

    #[tokio::test]
    async fn load_app_state_combines_worktrees_and_prds_for_dashboard() {
        let (_tmp, state) = setup_dashboard_state().await;

        // Daemon health (rendered by DaemonHealthCard).
        let daemon = state.worktree_state.daemon.as_ref().unwrap();
        assert_eq!(daemon.pid, 99999);

        // Worktree counts (rendered by StatusCards).
        assert_eq!(state.worktree_state.worktrees.len(), 3);

        // Events across worktrees (rendered by RecentEventsTimeline).
        let total_events: usize = state
            .worktree_state
            .worktrees
            .iter()
            .map(|wt| wt.events.len())
            .sum();
        assert_eq!(total_events, 4);

        // Overlap warnings (rendered by OverlapWarningsCard).
        assert_eq!(state.worktree_state.overlap_warnings.len(), 1);
        assert_eq!(
            state.worktree_state.overlap_warnings[0].risk,
            crate::types::OverlapRisk::High
        );

        // PRD summaries (rendered by PrdList).
        assert_eq!(state.prds.len(), 1);
        assert_eq!(state.prds[0].id, "PRD-0001");
        assert_eq!(state.prds[0].completed_tasks, 1);
        assert_eq!(state.prds[0].total_tasks, 2);
    }

    #[tokio::test]
    async fn file_mtime_returns_none_for_missing() {
        let result = file_mtime(Path::new("/nonexistent/path")).await;
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn dir_latest_mtime_returns_none_for_missing() {
        let result = dir_latest_mtime(Path::new("/nonexistent/dir")).await;
        assert!(result.is_none());
    }
}
