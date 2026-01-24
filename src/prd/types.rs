//! PRD data types and structures.

use serde::{Deserialize, Serialize};

/// Status of a PRD document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PrdStatus {
    /// PRD is in draft state, not yet active.
    #[default]
    Draft,

    /// PRD is active and being worked on.
    Active,

    /// PRD is complete.
    Done,

    /// PRD is parked/on-hold.
    Parked,
}

impl std::fmt::Display for PrdStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Draft => write!(f, "draft"),
            Self::Active => write!(f, "active"),
            Self::Done => write!(f, "done"),
            Self::Parked => write!(f, "parked"),
        }
    }
}

impl PrdStatus {
    /// Returns a sort order for display purposes.
    ///
    /// Order: Active (0), Draft (1), Done (2), Parked (3).
    pub fn sort_order(&self) -> u8 {
        match self {
            Self::Active => 0,
            Self::Draft => 1,
            Self::Done => 2,
            Self::Parked => 3,
        }
    }
}

/// Status of a task within a PRD.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task has not been started.
    #[default]
    Todo,

    /// Task is currently in progress.
    #[serde(rename = "in-progress")]
    InProgress,

    /// Task is complete.
    Done,

    /// Task is blocked.
    Blocked,

    /// Task is parked/on-hold.
    Parked,
}

impl std::fmt::Display for TaskStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Todo => write!(f, "todo"),
            Self::InProgress => write!(f, "in-progress"),
            Self::Done => write!(f, "done"),
            Self::Blocked => write!(f, "blocked"),
            Self::Parked => write!(f, "parked"),
        }
    }
}

/// A task within a PRD.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    /// Unique identifier for the task (e.g., "T-001").
    pub id: String,

    /// Human-readable title of the task.
    pub title: String,

    /// Priority of the task (lower = higher priority).
    pub priority: u32,

    /// Current status of the task.
    pub status: TaskStatus,

    /// Optional notes about the task.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

/// UAT verification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UatStatus {
    /// UAT has not been verified to exist as a real test.
    #[default]
    Unverified,

    /// UAT has been verified to exist (a real test exists or has been manually verified).
    Verified,
}

impl std::fmt::Display for UatStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unverified => write!(f, "unverified"),
            Self::Verified => write!(f, "verified"),
        }
    }
}

/// An acceptance test for a PRD.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AcceptanceTest {
    /// Unique identifier for the test (e.g., "uat-001").
    pub id: String,

    /// Human-readable name of the test.
    pub name: String,

    /// Command to run the test.
    pub command: String,

    /// Whether this UAT has been verified to exist as a real test.
    #[serde(default)]
    pub uat_status: UatStatus,
}

/// Git configuration for the PRD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct GitConfig {
    /// Branch mode: "current" or "feature".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_mode: Option<String>,

    /// Prefix for feature branches.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub feature_branch_prefix: Option<String>,

    /// Commit policy: "never", "auto_clean", or "always".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_policy: Option<String>,
}

/// Runner configuration for the PRD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct RunnerConfig {
    /// Default runner to use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Allowed runners.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allow_runners: Option<Vec<String>>,

    /// Permissions mode: "yolo", "allow_all", or "manual".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permissions_mode: Option<String>,

    /// Fallback flags for the runner.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_flags: Option<Vec<String>>,
}

/// Loop configuration for the PRD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct LoopConfig {
    /// PRD pick strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd_pick: Option<String>,

    /// Task pick strategy.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task_pick: Option<String>,

    /// Maximum iterations per run.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_iterations: Option<u32>,

    /// Maximum task attempts across history entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_task_attempts: Option<u32>,

    /// Maximum session time in minutes.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_session_minutes: Option<u32>,

    /// Maximum transcript size in KB.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_transcript_kb: Option<u32>,
}

/// Bootstrap configuration for the PRD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct BootstrapConfig {
    /// Whether to generate the index.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_index: Option<bool>,

    /// Whether to generate PRDs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generate_prds: Option<bool>,

    /// Maximum number of PRDs to generate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd_budget: Option<u32>,

    /// Heuristics to use for bootstrap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristics: Option<Vec<String>>,
}

/// Prompts configuration for the PRD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PromptsConfig {
    /// Init prompt path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub init: Option<String>,

    /// Bootstrap plan prompt path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_plan: Option<String>,

    /// Bootstrap generate PRDs prompt path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap_generate_prds: Option<String>,

    /// PRD new round 1 questions prompt path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd_new_round1_questions: Option<String>,

    /// PRD new round N questions prompt path.
    #[serde(
        rename = "prd_new_roundN_questions",
        skip_serializing_if = "Option::is_none"
    )]
    pub prd_new_round_n_questions: Option<String>,

    /// PRD new synthesize PRD prompt path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prd_new_synthesize_prd: Option<String>,

    /// Run task prompt path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_task: Option<String>,

    /// Run task finalize prompt path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_task_finalize: Option<String>,

    /// Update agents prompt path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub update_agents: Option<String>,
}

/// Dev configuration for the PRD.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct DevConfig {
    /// Command router (e.g., "cargo make").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command_router: Option<String>,

    /// Required make tasks.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub make_tasks_required: Option<Vec<String>>,
}

/// A reference link.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Reference {
    /// Name of the reference.
    pub name: String,

    /// URL of the reference.
    pub url: String,
}

/// YAML frontmatter for a PRD document.
///
/// This struct represents all the structured metadata that can appear
/// in the YAML frontmatter of a PRD file.
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PrdFrontmatter {
    /// Unique identifier for the PRD (e.g., "PRD-0001").
    pub id: String,

    /// Human-readable title of the PRD.
    pub title: String,

    /// Current status of the PRD.
    #[serde(default)]
    pub status: PrdStatus,

    /// Owner of the PRD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,

    /// Creation date (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created: Option<String>,

    /// Last updated date (ISO 8601).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated: Option<String>,

    /// Product name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_name: Option<String>,

    /// Binary name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binary_name: Option<String>,

    /// State directory path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state_dir: Option<String>,

    /// PRDs directory path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prds_dir: Option<String>,

    /// Index file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index_file: Option<String>,

    /// Templates directory path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates_dir: Option<String>,

    /// Prompts directory path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts_dir: Option<String>,

    /// Agents file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agents_file: Option<String>,

    /// Guiding principles.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub principles: Option<Vec<String>>,

    /// Reference links.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<Reference>>,

    /// Runner configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runner: Option<RunnerConfig>,

    /// Git configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git: Option<GitConfig>,

    /// Loop configuration.
    #[serde(rename = "loop", skip_serializing_if = "Option::is_none")]
    pub loop_config: Option<LoopConfig>,

    /// Bootstrap configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bootstrap: Option<BootstrapConfig>,

    /// Prompts configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts: Option<PromptsConfig>,

    /// Dev configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dev: Option<DevConfig>,

    /// Tags for categorization.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,

    /// Acceptance tests.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub acceptance_tests: Option<Vec<AcceptanceTest>>,

    /// Tasks within the PRD.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tasks: Option<Vec<Task>>,
}

/// A complete PRD document with frontmatter and body.
#[derive(Debug, Clone, PartialEq)]
pub struct Prd {
    /// The structured frontmatter metadata.
    pub frontmatter: PrdFrontmatter,

    /// The raw Markdown body content (everything after the frontmatter).
    pub body: String,
}

impl Prd {
    /// Creates a new PRD with the given frontmatter and body.
    pub fn new(frontmatter: PrdFrontmatter, body: String) -> Self {
        Self { frontmatter, body }
    }

    /// Returns the PRD ID.
    pub fn id(&self) -> &str {
        &self.frontmatter.id
    }

    /// Returns the PRD title.
    pub fn title(&self) -> &str {
        &self.frontmatter.title
    }

    /// Returns the PRD status.
    pub fn status(&self) -> PrdStatus {
        self.frontmatter.status
    }

    /// Returns the tasks in the PRD, if any.
    pub fn tasks(&self) -> Option<&[Task]> {
        self.frontmatter.tasks.as_deref()
    }

    /// Returns the next incomplete task (by priority).
    pub fn next_task(&self) -> Option<&Task> {
        self.frontmatter.tasks.as_ref().and_then(|tasks| {
            tasks
                .iter()
                .filter(|t| t.status == TaskStatus::Todo || t.status == TaskStatus::InProgress)
                .min_by_key(|t| t.priority)
        })
    }

    /// Returns all incomplete tasks.
    #[cfg(test)]
    pub fn incomplete_tasks(&self) -> Vec<&Task> {
        self.frontmatter
            .tasks
            .as_ref()
            .map(|tasks| {
                tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Todo || t.status == TaskStatus::InProgress)
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Returns all completed tasks.
    pub fn completed_tasks(&self) -> Vec<&Task> {
        self.frontmatter
            .tasks
            .as_ref()
            .map(|tasks| {
                tasks
                    .iter()
                    .filter(|t| t.status == TaskStatus::Done)
                    .collect()
            })
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prd_status_display() {
        assert_eq!(PrdStatus::Draft.to_string(), "draft");
        assert_eq!(PrdStatus::Active.to_string(), "active");
        assert_eq!(PrdStatus::Done.to_string(), "done");
        assert_eq!(PrdStatus::Parked.to_string(), "parked");
    }

    #[test]
    fn test_task_status_display() {
        assert_eq!(TaskStatus::Todo.to_string(), "todo");
        assert_eq!(TaskStatus::InProgress.to_string(), "in-progress");
        assert_eq!(TaskStatus::Done.to_string(), "done");
        assert_eq!(TaskStatus::Blocked.to_string(), "blocked");
        assert_eq!(TaskStatus::Parked.to_string(), "parked");
    }

    #[test]
    fn test_prd_next_task() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            tasks: Some(vec![
                Task {
                    id: "T-001".to_string(),
                    title: "First task".to_string(),
                    priority: 2,
                    status: TaskStatus::Done,
                    notes: None,
                },
                Task {
                    id: "T-002".to_string(),
                    title: "Second task".to_string(),
                    priority: 1,
                    status: TaskStatus::Todo,
                    notes: None,
                },
                Task {
                    id: "T-003".to_string(),
                    title: "Third task".to_string(),
                    priority: 3,
                    status: TaskStatus::Todo,
                    notes: None,
                },
            ]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, String::new());
        let next = prd.next_task().unwrap();

        assert_eq!(next.id, "T-002");
    }

    #[test]
    fn test_prd_incomplete_tasks() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            tasks: Some(vec![
                Task {
                    id: "T-001".to_string(),
                    title: "First task".to_string(),
                    priority: 1,
                    status: TaskStatus::Done,
                    notes: None,
                },
                Task {
                    id: "T-002".to_string(),
                    title: "Second task".to_string(),
                    priority: 2,
                    status: TaskStatus::Todo,
                    notes: None,
                },
                Task {
                    id: "T-003".to_string(),
                    title: "Third task".to_string(),
                    priority: 3,
                    status: TaskStatus::InProgress,
                    notes: None,
                },
            ]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, String::new());
        let incomplete = prd.incomplete_tasks();

        assert_eq!(incomplete.len(), 2);
    }
}
