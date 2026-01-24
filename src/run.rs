//! `mr run` command implementation.
//!
//! Picks the next task from an active PRD and invokes the runner to execute it.
//! The runner is responsible for:
//! - Implementing the task
//! - Running `cargo make uat`
//! - Updating the task status in the PRD frontmatter
//! - Appending to the History section
//! - Regenerating `.mr/PRDS.md`

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::agents::{RecentChange, update_agents_md};
use crate::prd::{Prd, PrdStatus, TaskStatus, scan_prds};
use crate::prompt::{
    PlaceholderContext, PromptKind, expand_placeholders, load_prompt_with_fallback,
};
use crate::runner::{Runner, RunnerOutput};

/// Configuration for `mr run`.
#[derive(Debug)]
pub struct RunConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// Explicit PRD ID to run (e.g., "PRD-0001"). If None, picks first active PRD.
    pub prd_id: Option<&'a str>,
}

/// Result from running a task.
#[derive(Debug)]
pub struct RunResult {
    /// The PRD that was run.
    pub prd_id: String,

    /// The task that was attempted.
    pub task_id: String,

    /// The task title.
    pub task_title: String,

    /// Path to the PRD file.
    pub prd_path: PathBuf,

    /// Whether the runner reported success.
    pub runner_success: bool,

    /// Runner output summary.
    pub output_summary: String,
}

/// Picks the next PRD to run.
///
/// Strategy: First active PRD with incomplete tasks, sorted by ID.
fn pick_prd(root: &Path, explicit_id: Option<&str>) -> Result<Option<(String, Prd, PathBuf)>> {
    let prds_dir = root.join(".mr").join("prds");
    let prds = scan_prds(&prds_dir)?;

    if let Some(id) = explicit_id {
        // Find the explicit PRD.
        for (filename, prd, path) in prds {
            if prd.id() == id {
                return Ok(Some((filename, prd, path)));
            }
        }

        anyhow::bail!("PRD not found: {id}");
    }

    // Find first active PRD with incomplete tasks.
    for (filename, prd, path) in prds {
        if prd.status() == PrdStatus::Active {
            let has_incomplete = prd
                .tasks()
                .map(|tasks| {
                    tasks
                        .iter()
                        .any(|t| t.status == TaskStatus::Todo || t.status == TaskStatus::InProgress)
                })
                .unwrap_or(false);

            if has_incomplete {
                return Ok(Some((filename, prd, path)));
            }
        }
    }

    Ok(None)
}

/// Builds the prompt for the runner.
fn build_prompt(root: &Path, prd: &Prd, prd_path: &Path, task_id: &str) -> String {
    let prompt_template = load_prompt_with_fallback(root, PromptKind::RunTask);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("prd_path", prd_path.display().to_string());
    ctx.insert("prd_id", prd.id());
    ctx.insert("prd_title", prd.title());
    ctx.insert("next_task_id", task_id);

    // Add task details if available.
    if let Some(tasks) = prd.tasks()
        && let Some(task) = tasks.iter().find(|t| t.id == task_id)
    {
        ctx.insert("task_title", task.title.clone());
        ctx.insert("task_priority", task.priority.to_string());

        if let Some(notes) = &task.notes {
            ctx.insert("task_notes", notes.clone());
        }
    }

    expand_placeholders(&prompt_template, &ctx)
}

/// Runs the next task from the active PRD.
///
/// # Arguments
///
/// * `config` - Configuration for the run
/// * `runner` - The runner to use for task execution
///
/// # Returns
///
/// A `RunResult` describing what happened, or an error.
pub fn run_task(config: &RunConfig, runner: &dyn Runner) -> Result<RunResult> {
    // Pick the PRD.
    let Some((_filename, prd, prd_path)) = pick_prd(config.root, config.prd_id)? else {
        anyhow::bail!("No active PRD with incomplete tasks found. Create a PRD with `mr prd new`.");
    };

    // Pick the next task.
    let Some(task) = prd.next_task() else {
        anyhow::bail!(
            "PRD {} has no incomplete tasks. All tasks are done!",
            prd.id()
        );
    };

    let task_id = task.id.clone();
    let task_title = task.title.clone();
    let prd_id = prd.id().to_string();

    tracing::info!(
        prd_id = %prd_id,
        task_id = %task_id,
        task_title = %task_title,
        "Running task"
    );

    // Build and execute the prompt.
    let prompt = build_prompt(config.root, &prd, &prd_path, &task_id);

    tracing::debug!(prompt_len = prompt.len(), "Invoking runner");

    let output: RunnerOutput = runner
        .execute(&prompt, config.root)
        .with_context(|| format!("Runner failed for task {task_id}"))?;

    // Summarize output (truncate if too long).
    let output_summary = if output.text.len() > 500 {
        format!("{}... (truncated)", &output.text[..500])
    } else {
        output.text.clone()
    };

    // Update AGENTS.md with task completion info (only if runner succeeded).
    if output.success {
        let changes = vec![RecentChange {
            file: prd_path.display().to_string(),
            description: format!("Completed task {task_id}: {task_title}"),
        }];

        match update_agents_md(config.root, runner, &changes) {
            Ok(result) if result.modified => {
                let summary = result
                    .new_content
                    .as_ref()
                    .map(|c| if c.len() > 100 { &c[..100] } else { c.as_str() })
                    .unwrap_or("(empty)");
                tracing::info!(summary = %summary, "Updated AGENTS.md auto-managed section");
            }
            Ok(_) => {
                tracing::debug!("No changes needed for AGENTS.md");
            }
            Err(e) => {
                tracing::warn!("Failed to update AGENTS.md: {e}");
            }
        }
    }

    Ok(RunResult {
        prd_id,
        task_id,
        task_title,
        prd_path,
        runner_success: output.success,
        output_summary,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prd::{PrdFrontmatter, Task};
    use crate::runner::MockRunner;
    use tempfile::TempDir;

    fn setup_test_repo(temp: &TempDir) -> PathBuf {
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create a minimal run_task.md prompt.
        std::fs::write(
            prompts_dir.join("run_task.md"),
            "Execute task {{next_task_id}} from {{prd_path}}",
        )
        .unwrap();

        root
    }

    fn create_test_prd(prds_dir: &Path, id: &str, status: PrdStatus, tasks: Vec<Task>) {
        let frontmatter = PrdFrontmatter {
            id: id.to_string(),
            title: format!("Test PRD {}", id),
            status,
            tasks: if tasks.is_empty() { None } else { Some(tasks) },
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let filename = format!("{}-test.md", id);

        std::fs::write(prds_dir.join(filename), content).unwrap();
    }

    fn make_task(id: &str, priority: u32, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {}", id),
            priority,
            status,
            notes: None,
        }
    }

    #[test]
    fn test_pick_prd_explicit() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(
            &prds_dir,
            "PRD-0001",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
        );
        create_test_prd(
            &prds_dir,
            "PRD-0002",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
        );

        let result = pick_prd(&root, Some("PRD-0002")).unwrap().unwrap();

        assert_eq!(result.1.id(), "PRD-0002");
    }

    #[test]
    fn test_pick_prd_first_active() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(&prds_dir, "PRD-0001", PrdStatus::Draft, vec![]);
        create_test_prd(
            &prds_dir,
            "PRD-0002",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
        );

        let result = pick_prd(&root, None).unwrap().unwrap();

        assert_eq!(result.1.id(), "PRD-0002");
    }

    #[test]
    fn test_pick_prd_skips_done_tasks() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // PRD with all done tasks should be skipped.
        create_test_prd(
            &prds_dir,
            "PRD-0001",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Done)],
        );
        create_test_prd(
            &prds_dir,
            "PRD-0002",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
        );

        let result = pick_prd(&root, None).unwrap().unwrap();

        assert_eq!(result.1.id(), "PRD-0002");
    }

    #[test]
    fn test_pick_prd_none_available() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(&prds_dir, "PRD-0001", PrdStatus::Draft, vec![]);

        let result = pick_prd(&root, None).unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_pick_prd_not_found() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        let result = pick_prd(&root, Some("PRD-9999"));

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not found"));
    }

    #[test]
    fn test_run_task_success() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(
            &prds_dir,
            "PRD-0001",
            PrdStatus::Active,
            vec![
                make_task("T-001", 2, TaskStatus::Done),
                make_task("T-002", 1, TaskStatus::Todo),
            ],
        );

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(
            "Task executed successfully.",
        )]);

        let config = RunConfig {
            root: &root,
            prd_id: None,
        };

        let result = run_task(&config, &runner).unwrap();

        assert_eq!(result.prd_id, "PRD-0001");
        assert_eq!(result.task_id, "T-002"); // Lower priority = higher precedence.
        assert!(result.runner_success);
    }

    #[test]
    fn test_run_task_explicit_prd() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(
            &prds_dir,
            "PRD-0001",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
        );
        create_test_prd(
            &prds_dir,
            "PRD-0002",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
        );

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success("Done.")]);

        let config = RunConfig {
            root: &root,
            prd_id: Some("PRD-0002"),
        };

        let result = run_task(&config, &runner).unwrap();

        assert_eq!(result.prd_id, "PRD-0002");
    }

    #[test]
    fn test_run_task_no_active_prd() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        let runner = MockRunner::empty();

        let config = RunConfig {
            root: &root,
            prd_id: None,
        };

        let result = run_task(&config, &runner);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No active PRD"));
    }

    #[test]
    fn test_build_prompt() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Implement feature".to_string(),
                priority: 1,
                status: TaskStatus::Todo,
                notes: Some("Use existing patterns".to_string()),
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let prd_path = root.join(".mr/prds/PRD-0001.md");

        let prompt = build_prompt(&root, &prd, &prd_path, "T-001");

        assert!(prompt.contains("T-001"));
        assert!(prompt.contains("PRD-0001.md"));
    }
}
