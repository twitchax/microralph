//! PRD finalization command.
//!
//! Validates PRD completion, runs final acceptance tests, generates artifacts,
//! updates the index, and marks the PRD as done.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use thiserror::Error;

use crate::prd::types::Task;
use crate::prd::{self, Prd, TaskStatus};
use crate::prompt::{
    PlaceholderContext, PromptKind, expand_placeholders, load_prompt_with_fallback,
};
use crate::runner::Runner;

/// Errors that can occur during PRD finalization.
#[derive(Debug, Error)]
pub enum FinalizeError {
    /// Some tasks are not complete.
    #[error("Cannot finalize PRD: {incomplete_count} task(s) are not done")]
    IncompleteTasks {
        /// Number of incomplete tasks.
        incomplete_count: usize,

        /// Details about the incomplete tasks.
        task_details: Vec<(String, TaskStatus)>,
    },

    /// The PRD was not found.
    #[error("PRD not found: {0}")]
    PrdNotFound(String),
}

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
}

/// Finds a PRD by ID from the scanned PRDs.
fn find_prd_by_id(root: &Path, prd_id: &str) -> Result<(Prd, PathBuf), FinalizeError> {
    let prds_dir = root.join(".mr").join("prds");
    let prds =
        prd::scan_prds(&prds_dir).map_err(|_| FinalizeError::PrdNotFound(prd_id.to_string()))?;

    for (_filename, prd, path) in prds {
        if prd.id() == prd_id {
            return Ok((prd, path));
        }
    }

    Err(FinalizeError::PrdNotFound(prd_id.to_string()))
}

/// Gets all incomplete tasks from a PRD.
fn get_incomplete_tasks(prd: &Prd) -> Vec<(&Task, TaskStatus)> {
    match prd.tasks() {
        Some(tasks) => tasks
            .iter()
            .filter(|t| t.status != TaskStatus::Done)
            .map(|t| (t, t.status))
            .collect(),
        None => vec![],
    }
}

/// Validates that all tasks in the PRD are done.
fn validate_all_tasks_done(prd: &Prd) -> Result<(), FinalizeError> {
    let incomplete = get_incomplete_tasks(prd);

    if incomplete.is_empty() {
        Ok(())
    } else {
        Err(FinalizeError::IncompleteTasks {
            incomplete_count: incomplete.len(),
            task_details: incomplete
                .into_iter()
                .map(|(t, status)| (t.id.clone(), status))
                .collect(),
        })
    }
}

/// Builds the finalization prompt for the runner.
fn build_finalize_prompt(root: &Path, prd: &Prd) -> String {
    let template = load_prompt_with_fallback(root, PromptKind::RunTaskFinalize);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("prd_id", prd.id());
    ctx.insert("prd_summary", prd.body.clone());

    expand_placeholders(&template, &ctx)
}

/// Finalizes a PRD.
///
/// This function:
/// 1. Finds the PRD by ID
/// 2. Validates all tasks are done (returns error if not)
/// 3. Runs acceptance tests via the finalization prompt
/// 4. (Future: generates changelog entry)
/// 5. (Future: updates PRD status to done)
/// 6. (Future: refreshes the index)
///
/// # Arguments
///
/// * `config` - Configuration for finalization
/// * `runner` - The runner to use for acceptance test verification
///
/// # Returns
///
/// A `PrdFinalizeResult` with the outcome of finalization.
///
/// # Errors
///
/// Returns `FinalizeError::IncompleteTasks` if any task is not done.
/// Returns `FinalizeError::PrdNotFound` if the PRD doesn't exist.
/// Returns an error if the runner fails.
pub fn finalize_prd(config: &PrdFinalizeConfig, runner: &dyn Runner) -> Result<PrdFinalizeResult> {
    tracing::debug!(
        prd_id = config.prd_id,
        stream = config.stream,
        "Starting PRD finalization"
    );

    let (prd, path) = find_prd_by_id(config.root, config.prd_id)
        .with_context(|| format!("Failed to find PRD: {}", config.prd_id))?;

    // Validate all tasks are done - this returns an error if any are incomplete.
    validate_all_tasks_done(&prd).with_context(|| {
        format!(
            "PRD {} cannot be finalized: incomplete tasks remain",
            config.prd_id
        )
    })?;

    tracing::info!(
        prd_id = config.prd_id,
        "All tasks done, running acceptance test verification"
    );

    // Build and execute the finalization prompt.
    let prompt = build_finalize_prompt(config.root, &prd);

    tracing::debug!(
        prompt_len = prompt.len(),
        runner = %runner.name(),
        "Invoking runner for acceptance test verification"
    );

    let output = if config.stream {
        let mut stdout = std::io::stdout();

        runner
            .execute_streaming(&prompt, config.root, &mut stdout)
            .with_context(|| format!("Runner failed during finalization of {}", config.prd_id))?
    } else {
        runner
            .execute(&prompt, config.root)
            .with_context(|| format!("Runner failed during finalization of {}", config.prd_id))?
    };

    if !output.success {
        anyhow::bail!(
            "Finalization verification failed for {}: {}",
            config.prd_id,
            output.text
        );
    }

    tracing::info!(
        prd_id = config.prd_id,
        "Finalization verification completed successfully"
    );

    Ok(PrdFinalizeResult {
        prd_id: prd.id().to_string(),
        prd_title: prd.title().to_string(),
        path,
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
    fn test_validate_all_tasks_done_with_all_done() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Done),
            ],
        );

        assert!(validate_all_tasks_done(&prd).is_ok());
    }

    #[test]
    fn test_validate_all_tasks_done_with_incomplete() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Todo),
            ],
        );

        let result = validate_all_tasks_done(&prd);
        assert!(result.is_err());

        if let Err(FinalizeError::IncompleteTasks {
            incomplete_count,
            task_details,
        }) = result
        {
            assert_eq!(incomplete_count, 1);
            assert_eq!(task_details.len(), 1);
            assert_eq!(task_details[0].0, "T-002");
            assert_eq!(task_details[0].1, TaskStatus::Todo);
        } else {
            panic!("Expected IncompleteTasks error");
        }
    }

    #[test]
    fn test_validate_all_tasks_done_with_in_progress() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::InProgress),
            ],
        );

        let result = validate_all_tasks_done(&prd);
        assert!(result.is_err());

        if let Err(FinalizeError::IncompleteTasks {
            incomplete_count,
            task_details,
        }) = result
        {
            assert_eq!(incomplete_count, 1);
            assert_eq!(task_details[0].1, TaskStatus::InProgress);
        } else {
            panic!("Expected IncompleteTasks error");
        }
    }

    #[test]
    fn test_validate_all_tasks_done_with_no_tasks() {
        let prd = make_test_prd("PRD-0001", vec![]);

        assert!(validate_all_tasks_done(&prd).is_ok());
    }

    #[test]
    fn test_validate_all_tasks_done_with_parked() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Parked),
            ],
        );

        let result = validate_all_tasks_done(&prd);
        assert!(result.is_err());

        if let Err(FinalizeError::IncompleteTasks {
            incomplete_count,
            task_details,
        }) = result
        {
            assert_eq!(incomplete_count, 1);
            assert_eq!(task_details[0].1, TaskStatus::Parked);
        } else {
            panic!("Expected IncompleteTasks error");
        }
    }

    #[test]
    fn test_validate_all_tasks_done_with_blocked() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Blocked),
            ],
        );

        let result = validate_all_tasks_done(&prd);
        assert!(result.is_err());

        if let Err(FinalizeError::IncompleteTasks {
            incomplete_count,
            task_details,
        }) = result
        {
            assert_eq!(incomplete_count, 1);
            assert_eq!(task_details[0].1, TaskStatus::Blocked);
        } else {
            panic!("Expected IncompleteTasks error");
        }
    }

    #[test]
    fn test_validate_multiple_incomplete_tasks() {
        let prd = make_test_prd(
            "PRD-0001",
            vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Todo),
                make_task("T-003", TaskStatus::InProgress),
                make_task("T-004", TaskStatus::Parked),
            ],
        );

        let result = validate_all_tasks_done(&prd);
        assert!(result.is_err());

        if let Err(FinalizeError::IncompleteTasks {
            incomplete_count,
            task_details,
        }) = result
        {
            assert_eq!(incomplete_count, 3);
            assert_eq!(task_details.len(), 3);
        } else {
            panic!("Expected IncompleteTasks error");
        }
    }

    #[test]
    fn test_build_finalize_prompt() {
        let temp = tempfile::TempDir::new().unwrap();

        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            tasks: Some(vec![make_task("T-001", TaskStatus::Done)]),
            ..Default::default()
        };

        let prd = Prd::new(
            frontmatter,
            "# Test PRD Summary\n\nThis is the PRD body content.".to_string(),
        );

        let prompt = build_finalize_prompt(temp.path(), &prd);

        // Verify placeholders are expanded.
        assert!(prompt.contains("PRD-0001"), "Prompt should contain PRD ID");
        assert!(
            prompt.contains("Test PRD Summary"),
            "Prompt should contain PRD body"
        );
        assert!(
            prompt.contains("This is the PRD body content"),
            "Prompt should contain full body"
        );
    }
}
