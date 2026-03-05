//! Worktree state schema and types.
//!
//! Defines the data structures for worktree orchestration state,
//! YAML-serializable for persistence in `.mr/worktrees/state.yaml`.

// Types are defined now but consumed by later tasks (T-002 .. T-018).
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

// ── Top-level state ─────────────────────────────────────────────────

/// Root state structure persisted to `.mr/worktrees/state.yaml`.
///
/// Includes a `version` field for future schema migration.
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

// ── Daemon ──────────────────────────────────────────────────────────

/// Runtime information about the currently running daemon.
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

/// User-configurable daemon settings (future use in `config.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Tier 1 heartbeat interval in seconds.
    #[serde(default = "default_heartbeat_interval_secs")]
    pub heartbeat_interval_secs: u64,

    /// Hours of idle time before auto-exit.
    #[serde(default = "default_idle_timeout_hours")]
    pub idle_timeout_hours: u32,

    /// Path to the Unix domain socket (relative to `.mr/worktrees/`).
    #[serde(default = "default_socket_name")]
    pub socket_name: String,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            heartbeat_interval_secs: default_heartbeat_interval_secs(),
            idle_timeout_hours: default_idle_timeout_hours(),
            socket_name: default_socket_name(),
        }
    }
}

fn default_heartbeat_interval_secs() -> u64 {
    30
}

fn default_socket_name() -> String {
    String::from("daemon.sock")
}

// ── Worktree entry ──────────────────────────────────────────────────

/// A single registered worktree tied to a PRD.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorktreeEntry {
    /// Unique worktree identifier (e.g., `"wt-001"`).
    pub id: String,

    /// PRD ID this worktree is executing (e.g., `"PRD-0039"`).
    pub prd: String,

    /// Git branch name (e.g., `"microralph-prd-39"`).
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

    /// Target branch for merging (default: `"main"`).
    #[serde(default = "default_merge_target")]
    pub merge_target: String,

    /// Files modified relative to `merge_target` (populated by heartbeat).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub modified_files: Vec<String>,

    /// Ordered lifecycle events.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<WorktreeEvent>,
}

fn default_merge_target() -> String {
    String::from("main")
}

// ── Worktree status ─────────────────────────────────────────────────

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

// ── Events ──────────────────────────────────────────────────────────

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

    /// Merge of the worktree branch into the target started.
    MergeStarted,

    /// Merge completed successfully.
    MergeCompleted,

    /// Merge or UAT verification failed.
    MergeFailed,

    /// Merge produced conflicts requiring resolution.
    Conflicted,

    /// Agent-driven conflict resolution started.
    ConflictResolutionStarted,

    /// Agent successfully resolved conflicts.
    ConflictResolved,
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
        }
    }
}

// ── Overlap warnings ────────────────────────────────────────────────

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
    /// Minimal overlap; unlikely to cause conflicts.
    #[default]
    Low,

    /// Some shared files; review recommended before merging.
    Medium,

    /// Heavy overlap; high conflict probability.
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

// ── IPC message types ───────────────────────────────────────────────

/// Message sent from a worktree `mr run` process to the daemon over IPC.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum IpcMessage {
    /// `mr run` has started in a worktree.
    RunStarted {
        prd: String,
        wt_id: String,
        pid: u32,
    },

    /// A task has started.
    TaskStarted {
        prd: String,
        wt_id: String,
        task: String,
    },

    /// A task has completed.
    TaskCompleted {
        prd: String,
        wt_id: String,
        task: String,
    },

    /// `mr run` completed successfully.
    RunCompleted { prd: String, wt_id: String },

    /// `mr run` failed.
    RunFailed {
        prd: String,
        wt_id: String,
        error: String,
    },

    /// Heartbeat request (daemon queries liveness).
    HeartbeatRequest,
}

/// Response from the daemon to an IPC message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IpcResponse {
    /// `"ok"` on success, `"error"` on failure.
    pub status: String,

    /// Error description when `status` is `"error"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl IpcResponse {
    /// Create a successful response.
    #[must_use]
    pub fn ok() -> Self {
        Self {
            status: String::from("ok"),
            message: None,
        }
    }

    /// Create an error response.
    #[must_use]
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            status: String::from("error"),
            message: Some(message.into()),
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn default_state_has_version_1() {
        let state = WorktreeState::default();
        assert_eq!(state.version, 1);
        assert!(state.daemon.is_none());
        assert!(state.worktrees.is_empty());
        assert!(state.overlap_warnings.is_empty());
    }

    #[test]
    fn state_roundtrips_through_yaml() {
        let state = WorktreeState {
            version: 1,
            daemon: Some(DaemonInfo {
                pid: 12345,
                started_at: "2026-03-04T22:00:00Z".to_string(),
                idle_timeout_hours: 3,
                last_heartbeat: "2026-03-04T22:30:00Z".to_string(),
            }),
            worktrees: vec![WorktreeEntry {
                id: "wt-001".to_string(),
                prd: "PRD-0039".to_string(),
                branch: "microralph-prd-39".to_string(),
                path: "/tmp/microralph-prd-39".to_string(),
                status: WorktreeStatus::Active,
                run_pid: Some(54321),
                created_at: "2026-03-04T22:00:00Z".to_string(),
                updated_at: "2026-03-04T22:30:00Z".to_string(),
                merge_target: "main".to_string(),
                modified_files: vec!["src/main.rs".to_string()],
                events: vec![
                    WorktreeEvent {
                        timestamp: "2026-03-04T22:00:00Z".to_string(),
                        event_type: EventType::Created,
                        detail: None,
                    },
                    WorktreeEvent {
                        timestamp: "2026-03-04T22:01:00Z".to_string(),
                        event_type: EventType::RunStarted,
                        detail: Some("T-001".to_string()),
                    },
                ],
            }],
            overlap_warnings: vec![OverlapWarning {
                worktrees: vec!["wt-001".to_string(), "wt-002".to_string()],
                files: vec!["src/main.rs".to_string()],
                risk: OverlapRisk::High,
            }],
        };

        let yaml = serde_yaml::to_string(&state).unwrap();
        let deserialized: WorktreeState = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(state, deserialized);
    }

    #[test]
    fn ipc_message_serializes_with_type_tag() {
        let msg = IpcMessage::RunStarted {
            prd: "PRD-0039".to_string(),
            wt_id: "wt-001".to_string(),
            pid: 54321,
        };

        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains(r#""type":"run_started""#));

        let deserialized: IpcMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(msg, deserialized);
    }

    #[test]
    fn ipc_response_helpers() {
        let ok = IpcResponse::ok();
        assert_eq!(ok.status, "ok");
        assert!(ok.message.is_none());

        let err = IpcResponse::error("something broke");
        assert_eq!(err.status, "error");
        assert_eq!(err.message.as_deref(), Some("something broke"));
    }

    #[test]
    fn worktree_status_display() {
        assert_eq!(WorktreeStatus::Active.to_string(), "active");
        assert_eq!(WorktreeStatus::MergeFailed.to_string(), "merge_failed");
        assert_eq!(WorktreeStatus::Abandoned.to_string(), "abandoned");
    }

    #[test]
    fn overlap_risk_display() {
        assert_eq!(OverlapRisk::Low.to_string(), "low");
        assert_eq!(OverlapRisk::Medium.to_string(), "medium");
        assert_eq!(OverlapRisk::High.to_string(), "high");
    }

    #[test]
    fn event_type_display() {
        assert_eq!(EventType::Created.to_string(), "created");
        assert_eq!(EventType::RunStarted.to_string(), "run_started");
        assert_eq!(EventType::TaskCompleted.to_string(), "task_completed");
        assert_eq!(EventType::MergeStarted.to_string(), "merge_started");
        assert_eq!(EventType::MergeCompleted.to_string(), "merge_completed");
        assert_eq!(EventType::MergeFailed.to_string(), "merge_failed");
        assert_eq!(EventType::Conflicted.to_string(), "conflicted");
        assert_eq!(
            EventType::ConflictResolutionStarted.to_string(),
            "conflict_resolution_started"
        );
        assert_eq!(EventType::ConflictResolved.to_string(), "conflict_resolved");
    }

    #[test]
    fn default_daemon_config() {
        let cfg = DaemonConfig::default();
        assert_eq!(cfg.heartbeat_interval_secs, 30);
        assert_eq!(cfg.idle_timeout_hours, 3);
        assert_eq!(cfg.socket_name, "daemon.sock");
    }

    #[test]
    fn minimal_yaml_deserializes_with_defaults() {
        let yaml = "version: 1\nworktrees: []\n";
        let state: WorktreeState = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(state.version, 1);
        assert!(state.daemon.is_none());
        assert!(state.worktrees.is_empty());
        assert!(state.overlap_warnings.is_empty());
    }
}
