//! UI-specific data types for worktree state and PRD summaries.
//!
//! These types mirror the YAML schemas from the root crate's `src/worktree/types.rs`
//! and `src/prd/types.rs`. They are duplicated here because the UI crate cannot depend
//! on the root binary crate (which would create a circular dependency). Keep these types
//! in sync with the canonical definitions when the YAML schema changes.

use serde::{Deserialize, Serialize};

// ── Worktree state (mirrors src/worktree/types.rs) ─────────────────

/// Root worktree state from `.mr/worktrees/state.yaml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeState {
    /// Schema version for forward-compatible migrations.
    pub version: u32,

    /// Daemon lifecycle information (present when daemon is running).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub daemon: Option<DaemonInfo>,

    /// Registered worktrees.
    #[serde(default)]
    pub worktrees: Vec<WorktreeEntry>,

    /// Cross-worktree file-overlap warnings.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub overlap_warnings: Vec<OverlapWarning>,
}

impl Default for WorktreeState {
    fn default() -> Self {
        Self {
            version: 1,
            daemon: None,
            worktrees: Vec::new(),
            overlap_warnings: Vec::new(),
        }
    }
}

/// Daemon runtime information.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonInfo {
    /// PID of the daemon process.
    pub pid: u32,

    /// ISO 8601 timestamp when the daemon started.
    pub started_at: String,

    /// Hours of idle time before the daemon auto-exits.
    #[serde(default = "default_idle_timeout_hours")]
    pub idle_timeout_hours: u32,

    /// ISO 8601 timestamp of the last heartbeat.
    pub last_heartbeat: String,
}

fn default_idle_timeout_hours() -> u32 {
    3
}

/// A registered worktree entry tied to a PRD.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeEntry {
    /// Unique worktree identifier (e.g., `"wt-001"`).
    pub id: String,

    /// PRD ID this worktree is executing (e.g., `"PRD-0039"`).
    pub prd: String,

    /// Git branch name.
    pub branch: String,

    /// Absolute filesystem path to the worktree directory.
    pub path: String,

    /// Current lifecycle status.
    pub status: WorktreeStatus,

    /// PID of the `mr run` process inside this worktree.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_pid: Option<u32>,

    /// ISO 8601 creation timestamp.
    pub created_at: String,

    /// ISO 8601 last-update timestamp.
    pub updated_at: String,

    /// Target branch for merging.
    #[serde(default = "default_merge_target")]
    pub merge_target: String,

    /// Files modified relative to `merge_target`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modified_files: Vec<String>,

    /// Ordered lifecycle events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<WorktreeEvent>,
}

fn default_merge_target() -> String {
    String::from("main")
}

/// Lifecycle status of a worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorktreeStatus {
    /// Agent is actively executing tasks.
    #[default]
    Active,

    /// All tasks finished; awaiting merge.
    Completed,

    /// Merge in progress.
    Merging,

    /// Successfully merged into target branch.
    Merged,

    /// Merge or UAT verification failed.
    MergeFailed,

    /// Merge produced conflicts requiring agent resolution.
    Conflicted,

    /// Manually or automatically abandoned.
    Abandoned,
}

impl std::fmt::Display for WorktreeStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Active => write!(f, "active"),
            Self::Completed => write!(f, "completed"),
            Self::Merging => write!(f, "merging"),
            Self::Merged => write!(f, "merged"),
            Self::MergeFailed => write!(f, "merge_failed"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::Abandoned => write!(f, "abandoned"),
        }
    }
}

/// A lifecycle event recorded against a worktree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeEvent {
    /// ISO 8601 timestamp of the event.
    pub timestamp: String,

    /// Kind of event.
    #[serde(rename = "type")]
    pub event_type: EventType,

    /// Optional detail (e.g., task ID, error message).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// Discriminator for worktree lifecycle events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    /// Worktree was created.
    Created,
    /// `mr run` started in the worktree.
    RunStarted,
    /// `mr run` completed successfully.
    RunCompleted,
    /// `mr run` failed.
    RunFailed,
    /// A specific task started.
    TaskStarted,
    /// A specific task completed.
    TaskCompleted,
    /// Merge started.
    MergeStarted,
    /// Merge completed successfully.
    MergeCompleted,
    /// Merge failed.
    MergeFailed,
    /// Merge produced conflicts.
    Conflicted,
    /// Agent conflict resolution started.
    ConflictResolutionStarted,
    /// Agent resolved conflicts.
    ConflictResolved,
    /// State committed to repo.
    StateCommitted,
    /// Worktree removed.
    Removed,
    /// Daemon crash recovery action.
    RecoveryPerformed,
}

impl std::fmt::Display for EventType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Created => write!(f, "created"),
            Self::RunStarted => write!(f, "run_started"),
            Self::RunCompleted => write!(f, "run_completed"),
            Self::RunFailed => write!(f, "run_failed"),
            Self::TaskStarted => write!(f, "task_started"),
            Self::TaskCompleted => write!(f, "task_completed"),
            Self::MergeStarted => write!(f, "merge_started"),
            Self::MergeCompleted => write!(f, "merge_completed"),
            Self::MergeFailed => write!(f, "merge_failed"),
            Self::Conflicted => write!(f, "conflicted"),
            Self::ConflictResolutionStarted => write!(f, "conflict_resolution_started"),
            Self::ConflictResolved => write!(f, "conflict_resolved"),
            Self::StateCommitted => write!(f, "state_committed"),
            Self::Removed => write!(f, "removed"),
            Self::RecoveryPerformed => write!(f, "recovery_performed"),
        }
    }
}

/// A file-overlap warning between two or more worktrees.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OverlapWarning {
    /// IDs of the worktrees that share modified files.
    pub worktrees: Vec<String>,

    /// Shared file paths.
    pub files: Vec<String>,

    /// Computed risk level based on overlap extent.
    pub risk: OverlapRisk,
}

/// Risk level for file-overlap between worktrees.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OverlapRisk {
    /// Minimal overlap.
    #[default]
    Low,
    /// Some shared files.
    Medium,
    /// Heavy overlap.
    High,
}

impl std::fmt::Display for OverlapRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Low => write!(f, "low"),
            Self::Medium => write!(f, "medium"),
            Self::High => write!(f, "high"),
        }
    }
}

// ── PRD summary ─────────────────────────────────────────────────────

/// Summary of a PRD for display in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PrdSummary {
    /// PRD ID (e.g., `"PRD-0001"`).
    pub id: String,

    /// Human-readable title.
    pub title: String,

    /// Status string (e.g., `"active"`, `"done"`).
    pub status: String,

    /// Number of completed tasks.
    pub completed_tasks: usize,

    /// Total number of tasks.
    pub total_tasks: usize,

    /// PRD IDs this PRD depends on.
    pub depends_on: Vec<String>,
}

// ── Combined application state ──────────────────────────────────────

/// Combined UI state shared across Axum handlers via `Arc<RwLock<...>>`.
#[derive(Debug, Clone, Default)]
pub struct AppState {
    /// Current worktree orchestration state.
    pub worktree_state: WorktreeState,

    /// Summaries of all PRDs in `.mr/prds/`.
    pub prds: Vec<PrdSummary>,
}
