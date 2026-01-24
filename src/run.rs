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

    /// Explicit PRD ID to run (e.g., "PRD-0001"). If None, asks runner to pick.
    pub prd_id: Option<&'a str>,

    /// Whether to stream runner output in real-time.
    pub stream: bool,
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

/// Summary of a PRD for the pick prompt.
struct PrdPickSummary {
    id: String,
    title: String,
    status: String,
    completed: usize,
    total: usize,
    incomplete_tasks: Vec<TaskPickSummary>,
}

/// Summary of an incomplete task for the pick prompt.
struct TaskPickSummary {
    id: String,
    title: String,
    priority: u32,
}

/// Asks the runner to pick the next PRD to work on.
fn pick_prd_via_runner(root: &Path, runner: &dyn Runner, stream: bool) -> Result<Option<String>> {
    let prds_dir = root.join(".mr").join("prds");
    let prds = scan_prds(&prds_dir)?;

    // Collect PRD summaries for the prompt.
    let mut summaries: Vec<PrdPickSummary> = Vec::new();

    for (_filename, prd, _path) in &prds {
        // Only include active and draft PRDs with incomplete tasks.
        if prd.status() != PrdStatus::Active && prd.status() != PrdStatus::Draft {
            continue;
        }

        let tasks = prd.tasks().unwrap_or(&[]);
        let incomplete: Vec<_> = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Todo || t.status == TaskStatus::InProgress)
            .collect();

        if incomplete.is_empty() {
            continue;
        }

        let completed = tasks
            .iter()
            .filter(|t| t.status == TaskStatus::Done)
            .count();

        summaries.push(PrdPickSummary {
            id: prd.id().to_string(),
            title: prd.title().to_string(),
            status: prd.status().to_string(),
            completed,
            total: tasks.len(),
            incomplete_tasks: incomplete
                .iter()
                .map(|t| TaskPickSummary {
                    id: t.id.clone(),
                    title: t.title.clone(),
                    priority: t.priority,
                })
                .collect(),
        });
    }

    if summaries.is_empty() {
        return Ok(None);
    }

    // Build the prompt.
    let prompt_template = load_prompt_with_fallback(root, PromptKind::PickPrd);
    let mut ctx = PlaceholderContext::new();

    let prds_list: Vec<_> = summaries
        .iter()
        .map(|s| {
            let mut prd_map = std::collections::HashMap::new();
            prd_map.insert("id".to_string(), s.id.clone());
            prd_map.insert("title".to_string(), s.title.clone());
            prd_map.insert("status".to_string(), s.status.clone());
            prd_map.insert("completed".to_string(), s.completed.to_string());
            prd_map.insert("total".to_string(), s.total.to_string());

            let tasks_list: Vec<_> = s
                .incomplete_tasks
                .iter()
                .map(|t| {
                    let mut task_map = std::collections::HashMap::new();
                    task_map.insert("id".to_string(), t.id.clone());
                    task_map.insert("title".to_string(), t.title.clone());
                    task_map.insert("priority".to_string(), t.priority.to_string());
                    task_map
                })
                .collect();

            prd_map.insert(
                "incomplete_tasks".to_string(),
                serde_yaml::to_string(&tasks_list).unwrap_or_default(),
            );
            prd_map
        })
        .collect();

    ctx.insert(
        "prds",
        serde_yaml::to_string(&prds_list).unwrap_or_default(),
    );

    let prompt = expand_placeholders(&prompt_template, &ctx);

    tracing::info!("Asking runner to pick the next PRD to work on...");

    // Invoke the runner.
    let output: RunnerOutput = if stream {
        let mut stdout = std::io::stdout();
        runner.execute_streaming(&prompt, root, &mut stdout)?
    } else {
        runner.execute(&prompt, root)?
    };

    // Parse the response to extract the PRD ID.
    let response = output.text.trim();

    if response == "NONE" || response.is_empty() {
        return Ok(None);
    }

    // Extract PRD ID from response (look for PRD-NNNN pattern).
    let prd_id = extract_prd_id(response);

    if let Some(ref id) = prd_id {
        tracing::info!(prd_id = %id, "Runner selected PRD");
    }

    Ok(prd_id)
}

/// Extracts a PRD ID (e.g., PRD-0001) from text.
fn extract_prd_id(text: &str) -> Option<String> {
    // First, try to find it as the entire trimmed line.
    let trimmed = text.trim();
    if trimmed.starts_with("PRD-") && trimmed.len() >= 8 {
        // Check if it looks like a valid PRD ID.
        let id_part = &trimmed[4..];
        if id_part.chars().take(4).all(|c| c.is_ascii_digit()) {
            return Some(trimmed.to_string());
        }
    }

    // Otherwise, search for PRD-NNNN pattern in the text.
    let re = regex::Regex::new(r"PRD-\d{4}").ok()?;
    re.find(text).map(|m| m.as_str().to_string())
}

/// Finds a PRD by ID.
fn find_prd_by_id(root: &Path, prd_id: &str) -> Result<Option<(String, Prd, PathBuf)>> {
    let prds_dir = root.join(".mr").join("prds");
    let prds = scan_prds(&prds_dir)?;

    for (filename, prd, path) in prds {
        if prd.id() == prd_id {
            return Ok(Some((filename, prd, path)));
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
/// If no PRD ID is specified, the runner is first asked to determine which PRD
/// to work on next (two-pass approach). Then the normal task execution proceeds.
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
    // Determine which PRD to work on.
    let (_filename, prd, prd_path) = if let Some(explicit_id) = config.prd_id {
        // Explicit PRD ID provided.
        find_prd_by_id(config.root, explicit_id)?
            .ok_or_else(|| anyhow::anyhow!("PRD not found: {explicit_id}"))?
    } else {
        // Ask the runner to pick the next PRD.
        let picked_id =
            pick_prd_via_runner(config.root, runner, config.stream)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "No active PRD with incomplete tasks found. Create a PRD with `mr prd new`."
                )
            })?;

        find_prd_by_id(config.root, &picked_id)?
            .ok_or_else(|| anyhow::anyhow!("Runner picked PRD {picked_id}, but it was not found"))?
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

    tracing::debug!(prompt_len = prompt.len(), runner = %runner.name(), "Invoking runner");

    let output: RunnerOutput = if config.stream {
        // Stream output to stdout in real-time.
        let mut stdout = std::io::stdout();

        runner
            .execute_streaming(&prompt, config.root, &mut stdout)
            .with_context(|| format!("Runner failed for task {task_id}"))?
    } else {
        runner
            .execute(&prompt, config.root)
            .with_context(|| format!("Runner failed for task {task_id}"))?
    };

    // Summarize output (truncate if too long). Skip summary if we already streamed.
    let output_summary = if config.stream {
        "(output was streamed above)".to_string()
    } else if output.text.len() > 500 {
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
    fn test_find_prd_by_id() {
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

        let result = find_prd_by_id(&root, "PRD-0002").unwrap().unwrap();

        assert_eq!(result.1.id(), "PRD-0002");
    }

    #[test]
    fn test_find_prd_by_id_not_found() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        let result = find_prd_by_id(&root, "PRD-9999").unwrap();

        assert!(result.is_none());
    }

    #[test]
    fn test_extract_prd_id_simple() {
        assert_eq!(extract_prd_id("PRD-0001"), Some("PRD-0001".to_string()));
        assert_eq!(extract_prd_id("PRD-0002"), Some("PRD-0002".to_string()));
        assert_eq!(extract_prd_id("  PRD-0003  "), Some("PRD-0003".to_string()));
    }

    #[test]
    fn test_extract_prd_id_in_text() {
        assert_eq!(
            extract_prd_id("I recommend PRD-0002 because..."),
            Some("PRD-0002".to_string())
        );
        assert_eq!(
            extract_prd_id("The next PRD is PRD-0001."),
            Some("PRD-0001".to_string())
        );
    }

    #[test]
    fn test_extract_prd_id_none() {
        assert_eq!(extract_prd_id("NONE"), None);
        assert_eq!(extract_prd_id(""), None);
        assert_eq!(extract_prd_id("No PRDs available"), None);
    }

    #[test]
    fn test_run_task_success() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create pick_prd.md prompt.
        std::fs::write(root.join(".mr/prompts/pick_prd.md"), "Pick the next PRD").unwrap();

        create_test_prd(
            &prds_dir,
            "PRD-0001",
            PrdStatus::Active,
            vec![
                make_task("T-001", 2, TaskStatus::Done),
                make_task("T-002", 1, TaskStatus::Todo),
            ],
        );

        // Two responses: one for pick_prd, one for run_task.
        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("PRD-0001"),
            crate::runner::RunnerOutput::success("Task executed successfully."),
        ]);

        let config = RunConfig {
            root: &root,
            prd_id: None,
            stream: false,
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

        // Only one response needed since we're using explicit PRD ID.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success("Done.")]);

        let config = RunConfig {
            root: &root,
            prd_id: Some("PRD-0002"),
            stream: false,
        };

        let result = run_task(&config, &runner).unwrap();

        assert_eq!(result.prd_id, "PRD-0002");
    }

    #[test]
    fn test_run_task_no_active_prd() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        // Create pick_prd.md prompt.
        std::fs::write(root.join(".mr/prompts/pick_prd.md"), "Pick the next PRD").unwrap();

        // Runner returns NONE (no PRDs available).
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success("NONE")]);

        let config = RunConfig {
            root: &root,
            prd_id: None,
            stream: false,
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
