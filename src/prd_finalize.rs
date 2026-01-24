//! PRD finalization command.
//!
//! Validates PRD completion, runs final acceptance tests, generates artifacts,
//! updates the index, and marks the PRD as done.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::prd::{self, Prd, TaskStatus};
use crate::runner::Runner;

/// Configuration for PRD finalization.
pub struct PrdFinalizeConfig<'a> {
    /// Root directory of the repository.
    pub root: &'a Path,

    /// The PRD ID to finalize (e.g., "PRD-0001").
    pub prd_id: &'a str,

    /// Whether to stream runner output.
    pub stream: bool,
}

/// Result of PRD finalization.
pub struct PrdFinalizeResult {
    /// The PRD ID.
    pub prd_id: String,

    /// The PRD title.
    pub prd_title: String,

    /// Path to the PRD file.
    pub path: PathBuf,

    /// Whether all tasks are done.
    pub all_tasks_done: bool,
}

/// Finds a PRD by ID from the scanned PRDs.
fn find_prd_by_id(root: &Path, prd_id: &str) -> Result<(Prd, PathBuf)> {
    let prds_dir = root.join(".mr").join("prds");
    let prds = prd::scan_prds(&prds_dir)?;

    for (_filename, prd, path) in prds {
        if prd.id() == prd_id {
            return Ok((prd, path));
        }
    }

    anyhow::bail!("PRD not found: {}", prd_id)
}

/// Checks if all tasks in a PRD are done.
fn all_tasks_done(prd: &Prd) -> bool {
    match prd.tasks() {
        Some(tasks) => tasks.iter().all(|t| t.status == TaskStatus::Done),
        None => true, // No tasks means nothing to do
    }
}

/// Finalizes a PRD.
///
/// This function:
/// 1. Finds the PRD by ID
/// 2. Validates all tasks are done
/// 3. (Future: runs acceptance tests via runner)
/// 4. (Future: generates changelog entry)
/// 5. (Future: updates PRD status to done)
/// 6. (Future: refreshes the index)
///
/// # Arguments
///
/// * `config` - Configuration for finalization
/// * `_runner` - The runner to use for acceptance test verification
///
/// # Returns
///
/// A `PrdFinalizeResult` with the outcome of finalization.
pub fn finalize_prd(config: &PrdFinalizeConfig, _runner: &dyn Runner) -> Result<PrdFinalizeResult> {
    tracing::debug!(
        prd_id = config.prd_id,
        stream = config.stream,
        "Starting PRD finalization"
    );

    let (prd, path) = find_prd_by_id(config.root, config.prd_id)
        .with_context(|| format!("Failed to find PRD: {}", config.prd_id))?;

    let tasks_done = all_tasks_done(&prd);

    // For now, just validate and report. Actual finalization logic will be
    // added in subsequent tasks (T-002 through T-011).
    if !tasks_done {
        tracing::warn!(
            prd_id = config.prd_id,
            "Cannot finalize PRD: not all tasks are done"
        );
    }

    Ok(PrdFinalizeResult {
        prd_id: prd.id().to_string(),
        prd_title: prd.title().to_string(),
        path,
        all_tasks_done: tasks_done,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prd::types::{PrdFrontmatter, Task, TaskStatus};

    fn make_test_prd(id: &str, tasks: Vec<Task>) -> Prd {
        let frontmatter = PrdFrontmatter {
            id: id.to_string(),
            title: format!("Test PRD {}", id),
            tasks: if tasks.is_empty() { None } else { Some(tasks) },
            ..Default::default()
        };

        Prd::new(frontmatter, "# Body\n".to_string())
    }

    fn make_task(id: &str, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {}", id),
            priority: 1,
            status,
            notes: None,
        }
    }

    #[test]
    fn test_all_tasks_done_with_all_done() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Done),
            ],
        );

        assert!(all_tasks_done(&prd));
    }

    #[test]
    fn test_all_tasks_done_with_incomplete() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Todo),
            ],
        );

        assert!(!all_tasks_done(&prd));
    }

    #[test]
    fn test_all_tasks_done_with_in_progress() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::InProgress),
            ],
        );

        assert!(!all_tasks_done(&prd));
    }

    #[test]
    fn test_all_tasks_done_with_no_tasks() {
        let prd = make_test_prd("PRD-0001", vec![]);

        assert!(all_tasks_done(&prd));
    }

    #[test]
    fn test_all_tasks_done_with_parked() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Parked),
            ],
        );

        assert!(!all_tasks_done(&prd));
    }

    #[test]
    fn test_all_tasks_done_with_blocked() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Blocked),
            ],
        );

        assert!(!all_tasks_done(&prd));
    }
}
