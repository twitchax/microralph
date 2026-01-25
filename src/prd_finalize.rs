//! PRD finalization command.
//!
//! Validates PRD completion, runs final acceptance tests, generates artifacts,
//! updates the index, and marks the PRD as done.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Local, Utc};
use thiserror::Error;

use crate::changelog::{ensure_changelog_exists, read_changelog};
use crate::prd::types::{AcceptanceTest, Task};
use crate::prd::{self, Prd, PrdStatus, TaskStatus, serialize_prd};
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

    /// Some UATs are not verified.
    #[error("Cannot finalize PRD: {unverified_count} UAT(s) are not verified")]
    UnverifiedUats {
        /// Number of unverified UATs.
        unverified_count: usize,

        /// Details about the unverified UATs (id, name).
        uat_details: Vec<(String, String)>,
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
#[allow(dead_code)]
pub struct PrdFinalizeResult {
    /// The PRD ID.
    pub prd_id: String,

    /// The PRD title.
    pub prd_title: String,

    /// Path to the PRD file.
    pub path: PathBuf,

    /// Path to the CHANGELOG.md file.
    pub changelog_path: PathBuf,

    /// Whether the changelog was newly created.
    pub changelog_created: bool,

    /// The summary report content.
    pub summary_report: String,
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

/// Gets all unverified acceptance tests from a PRD.
fn get_unverified_uats(prd: &Prd) -> Vec<&AcceptanceTest> {
    prd.unverified_uats()
}

/// Validates that all acceptance tests in the PRD are verified.
fn validate_all_uats_verified(prd: &Prd) -> Result<(), FinalizeError> {
    let unverified = get_unverified_uats(prd);

    if unverified.is_empty() {
        Ok(())
    } else {
        Err(FinalizeError::UnverifiedUats {
            unverified_count: unverified.len(),
            uat_details: unverified
                .into_iter()
                .map(|t| (t.id.clone(), t.name.clone()))
                .collect(),
        })
    }
}

/// Formats completed tasks as a bullet list.
fn format_completed_tasks(prd: &Prd) -> String {
    let tasks = prd.completed_tasks();

    if tasks.is_empty() {
        return "(No completed tasks)".to_string();
    }

    tasks
        .iter()
        .map(|t| format!("- **{}**: {}", t.id, t.title))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Builds the finalization prompt for the runner.
fn build_finalize_prompt(root: &Path, prd: &Prd) -> String {
    let template = load_prompt_with_fallback(root, PromptKind::RunTaskFinalize);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("prd_id", prd.id());
    ctx.insert("prd_title", prd.title());
    ctx.insert("prd_summary", prd.body.clone());
    ctx.insert("completed_tasks", format_completed_tasks(prd));

    // Read the current changelog content for context.
    let changelog_content =
        read_changelog(root).unwrap_or_else(|| "(Changelog not found)".to_string());
    ctx.insert("changelog_content", changelog_content);

    // Load constitution if it exists.
    if let Ok(Some(constitution)) = crate::config::load_constitution(root) {
        ctx.insert("constitution", constitution);
    }

    expand_placeholders(&template, &ctx)
}

/// Generates a summary report for a finalized PRD.
fn generate_summary_report(prd: &Prd) -> String {
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ");
    let local_date = Local::now().format("%Y-%m-%d");
    let tasks = prd.completed_tasks();
    let task_count = tasks.len();

    let mut report = String::new();

    report.push_str(&format!("## {} — Finalization Complete\n", local_date));
    report.push_str(&format!("- **PRD**: {} — {}\n", prd.id(), prd.title()));
    report.push_str(&format!("- **Finalized**: {}\n", timestamp));
    report.push_str(&format!("- **Tasks Completed**: {}\n", task_count));
    report.push_str("- **Summary**:\n");

    for task in tasks {
        report.push_str(&format!("  - {}: {}\n", task.id, task.title));
    }

    report.push_str("- **Status**: ✅ All acceptance tests passed\n");

    report
}

/// Appends a summary report to the PRD file.
fn append_to_prd(prd_path: &Path, summary: &str) -> Result<()> {
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(prd_path)
        .with_context(|| {
            format!(
                "Failed to open PRD file for appending: {}",
                prd_path.display()
            )
        })?;

    // Ensure we start on a new line.
    writeln!(file)?;
    write!(file, "{}", summary)?;

    Ok(())
}

/// Updates the PRD status to done and saves the file.
///
/// This re-reads the PRD from disk (to capture any runner modifications),
/// updates the status, and writes it back.
fn update_prd_status_to_done(prd_path: &Path) -> Result<()> {
    // Re-read the PRD from disk to capture any changes made by the runner.
    let mut updated_prd = prd::parse_prd_file(prd_path)
        .with_context(|| format!("Failed to re-read PRD file: {}", prd_path.display()))?;

    // Update the status.
    updated_prd.frontmatter.status = PrdStatus::Done;

    // Serialize and write.
    let content = serialize_prd(&updated_prd)
        .with_context(|| format!("Failed to serialize PRD: {}", updated_prd.id()))?;

    fs::write(prd_path, &content)
        .with_context(|| format!("Failed to write updated PRD file: {}", prd_path.display()))?;

    tracing::info!(prd_id = updated_prd.id(), "Updated PRD status to done");

    Ok(())
}

/// Finalizes a PRD.
///
/// This function:
/// 1. Finds the PRD by ID
/// 2. Validates all tasks are done (returns error if not)
/// 3. Validates all UATs are verified (returns error if not)
/// 4. Updates PRD status to done
/// 5. Runs finalization prompt (runner appends history, updates changelog, regenerates PRDS.md, commits)
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
/// Returns `FinalizeError::UnverifiedUats` if any UAT is not verified.
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

    // Validate all UATs are verified - this returns an error if any are unverified.
    validate_all_uats_verified(&prd).with_context(|| {
        format!(
            "PRD {} cannot be finalized: unverified UATs remain",
            config.prd_id
        )
    })?;

    tracing::info!(
        prd_id = config.prd_id,
        "All tasks done and UATs verified, running acceptance test verification"
    );

    // Update PRD status to done BEFORE running the finalization prompt.
    // This ensures when the runner executes `cargo run -- list`, it sees the correct status.
    update_prd_status_to_done(&path)
        .with_context(|| format!("Failed to update PRD status to done: {}", config.prd_id))?;

    // Re-read the PRD after status update for the prompt context.
    let prd = prd::parse_prd_file(&path)
        .with_context(|| format!("Failed to re-read PRD after status update: {}", path.display()))?;

    // Build and execute the finalization prompt.
    let prompt = build_finalize_prompt(config.root, &prd);

    tracing::info!(
        prompt_len = prompt.len(),
        runner = %runner.name(),
        prd_id = %config.prd_id,
        stream = config.stream,
        "Invoking runner for finalization"
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

    // Ensure CHANGELOG.md exists at the project root.
    let changelog_result = ensure_changelog_exists(config.root)
        .with_context(|| format!("Failed to ensure CHANGELOG.md for {}", config.prd_id))?;

    if changelog_result.created {
        tracing::info!(
            path = %changelog_result.path.display(),
            "Created CHANGELOG.md"
        );
    } else {
        tracing::debug!(
            path = %changelog_result.path.display(),
            "CHANGELOG.md already exists"
        );
    }

    // The runner has already:
    // - Appended finalization history entry to PRD
    // - Updated CHANGELOG.md  
    // - Regenerated PRDS.md via `cargo run -- list`
    // - Committed all changes via git

    Ok(PrdFinalizeResult {
        prd_id: prd.id().to_string(),
        prd_title: prd.title().to_string(),
        path,
        changelog_path: changelog_result.path,
        changelog_created: changelog_result.created,
        summary_report: String::new(), // Runner handles reporting via prompt
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prd::types::{PrdFrontmatter, Task, TaskStatus, UatStatus};

    fn make_test_prd(id: &str, tasks: Vec<Task>) -> Prd {
        let frontmatter = PrdFrontmatter {
            id: id.to_string(),
            title: format!("Test PRD {}", id),
            tasks: if tasks.is_empty() { None } else { Some(tasks) },
            ..Default::default()
        };

        Prd::new(frontmatter, "# Body\n".to_string())
    }

    fn make_test_prd_with_uats(
        id: &str,
        tasks: Vec<Task>,
        acceptance_tests: Vec<AcceptanceTest>,
    ) -> Prd {
        let frontmatter = PrdFrontmatter {
            id: id.to_string(),
            title: format!("Test PRD {}", id),
            tasks: if tasks.is_empty() { None } else { Some(tasks) },
            acceptance_tests: if acceptance_tests.is_empty() {
                None
            } else {
                Some(acceptance_tests)
            },
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

    fn make_uat(id: &str, uat_status: UatStatus) -> AcceptanceTest {
        AcceptanceTest {
            id: id.to_string(),
            name: format!("Test {}", id),
            command: "cargo test".to_string(),
            uat_status,
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
            prompt.contains("Test PRD"),
            "Prompt should contain PRD title"
        );
        assert!(
            prompt.contains("Test PRD Summary"),
            "Prompt should contain PRD body"
        );
        assert!(
            prompt.contains("This is the PRD body content"),
            "Prompt should contain full body"
        );
        assert!(
            prompt.contains("**T-001**"),
            "Prompt should contain completed task ID"
        );
        assert!(
            prompt.contains("Task T-001"),
            "Prompt should contain completed task title"
        );
    }

    #[test]
    fn test_build_finalize_prompt_with_changelog() {
        let temp = tempfile::TempDir::new().unwrap();

        // Create a changelog file.
        std::fs::write(
            temp.path().join("CHANGELOG.md"),
            "# Changelog\n\n## [Unreleased]\n",
        )
        .unwrap();

        let frontmatter = PrdFrontmatter {
            id: "PRD-0002".to_string(),
            title: "Second PRD".to_string(),
            tasks: Some(vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Done),
            ]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Summary\n".to_string());

        let prompt = build_finalize_prompt(temp.path(), &prd);

        // Verify changelog content is included.
        assert!(
            prompt.contains("# Changelog"),
            "Prompt should contain changelog content"
        );
        assert!(
            prompt.contains("[Unreleased]"),
            "Prompt should contain Unreleased section"
        );

        // Verify both completed tasks are listed.
        assert!(
            prompt.contains("**T-001**"),
            "Prompt should contain first task"
        );
        assert!(
            prompt.contains("**T-002**"),
            "Prompt should contain second task"
        );
    }

    #[test]
    fn test_format_completed_tasks() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            tasks: Some(vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Todo),
                make_task("T-003", TaskStatus::Done),
            ]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, String::new());
        let formatted = format_completed_tasks(&prd);

        // Should only include done tasks.
        assert!(formatted.contains("**T-001**"));
        assert!(!formatted.contains("T-002"));
        assert!(formatted.contains("**T-003**"));
    }

    #[test]
    fn test_format_completed_tasks_empty() {
        let prd = make_test_prd("PRD-0001", vec![make_task("T-001", TaskStatus::Todo)]);
        let formatted = format_completed_tasks(&prd);

        assert_eq!(formatted, "(No completed tasks)");
    }

    #[test]
    fn test_generate_summary_report() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            tasks: Some(vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Done),
            ]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, String::new());
        let report = generate_summary_report(&prd);

        // Verify structure.
        assert!(report.contains("Finalization Complete"));
        assert!(report.contains("PRD-0001"));
        assert!(report.contains("Test PRD"));
        assert!(report.contains("Tasks Completed**: 2"));
        assert!(report.contains("T-001: Task T-001"));
        assert!(report.contains("T-002: Task T-002"));
        assert!(report.contains("✅ All acceptance tests passed"));
    }

    #[test]
    fn test_generate_summary_report_no_tasks() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0002".to_string(),
            title: "Empty PRD".to_string(),
            tasks: None,
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, String::new());
        let report = generate_summary_report(&prd);

        assert!(report.contains("PRD-0002"));
        assert!(report.contains("Tasks Completed**: 0"));
    }

    #[test]
    fn test_append_to_prd() {
        let temp = tempfile::TempDir::new().unwrap();
        let prd_path = temp.path().join("test.md");

        // Create a minimal PRD file.
        std::fs::write(&prd_path, "---\nid: PRD-0001\ntitle: Test\n---\n\n# Body\n").unwrap();

        let summary = "## Summary\n- Test entry\n";
        append_to_prd(&prd_path, summary).unwrap();

        let content = std::fs::read_to_string(&prd_path).unwrap();

        assert!(content.contains("# Body"));
        assert!(content.contains("## Summary"));
        assert!(content.contains("- Test entry"));
    }

    #[test]
    fn test_append_to_prd_preserves_existing() {
        let temp = tempfile::TempDir::new().unwrap();
        let prd_path = temp.path().join("test.md");

        let original = "---\nid: PRD-0001\ntitle: Test\n---\n\n# Body\n\n## History\n\n## 2026-01-01 — T-001\n- First entry\n";
        std::fs::write(&prd_path, original).unwrap();

        let summary = "## 2026-01-24 — Finalization Complete\n- Second entry\n";
        append_to_prd(&prd_path, summary).unwrap();

        let content = std::fs::read_to_string(&prd_path).unwrap();

        // Original content preserved.
        assert!(content.contains("## 2026-01-01 — T-001"));
        assert!(content.contains("- First entry"));

        // New content appended.
        assert!(content.contains("## 2026-01-24 — Finalization Complete"));
        assert!(content.contains("- Second entry"));
    }

    #[test]
    fn test_update_prd_status_to_done() {
        let temp = tempfile::TempDir::new().unwrap();
        let prd_path = temp.path().join("test.md");

        // Create a PRD file with draft status.
        let original = "---\nid: PRD-0001\ntitle: Test PRD\nstatus: draft\n---\n\n# Body\n\nSome content here.\n";
        std::fs::write(&prd_path, original).unwrap();

        // Update the status (re-reads from disk).
        update_prd_status_to_done(&prd_path).unwrap();

        // Read the updated file.
        let content = std::fs::read_to_string(&prd_path).unwrap();

        // Status should be updated to done.
        assert!(content.contains("status: done"), "Status should be 'done'");
        assert!(
            !content.contains("status: draft"),
            "Status should not be 'draft'"
        );

        // PRD ID and title should be preserved.
        assert!(content.contains("id: PRD-0001"));
        assert!(content.contains("title: Test PRD"));
    }

    #[test]
    fn test_update_prd_status_preserves_tasks() {
        let temp = tempfile::TempDir::new().unwrap();
        let prd_path = temp.path().join("test.md");

        // Create a PRD with tasks.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0002".to_string(),
            title: "Task Test".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![
                make_task("T-001", TaskStatus::Done),
                make_task("T-002", TaskStatus::Done),
            ]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n\nTest content.\n".to_string());

        // Serialize and write the original.
        let original = serialize_prd(&prd).unwrap();
        std::fs::write(&prd_path, &original).unwrap();

        // Update the status (re-reads from disk).
        update_prd_status_to_done(&prd_path).unwrap();

        // Parse the updated file.
        let updated = prd::parse_prd_file(&prd_path).unwrap();

        // Status should be done.
        assert_eq!(updated.status(), PrdStatus::Done);

        // Tasks should be preserved.
        let tasks = updated.tasks().unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "T-001");
        assert_eq!(tasks[1].id, "T-002");

        // Body should be preserved.
        assert!(updated.body.contains("Test content."));
    }

    #[test]
    fn test_validate_all_uats_verified_with_all_verified() {
        let prd = make_test_prd_with_uats(
            "PRD-0001",
            vec![make_task("T-001", TaskStatus::Done)],
            vec![
                make_uat("uat-001", UatStatus::Verified),
                make_uat("uat-002", UatStatus::Verified),
            ],
        );

        assert!(validate_all_uats_verified(&prd).is_ok());
    }

    #[test]
    fn test_validate_all_uats_verified_with_unverified() {
        let prd = make_test_prd_with_uats(
            "PRD-0001",
            vec![make_task("T-001", TaskStatus::Done)],
            vec![
                make_uat("uat-001", UatStatus::Verified),
                make_uat("uat-002", UatStatus::Unverified),
            ],
        );

        let result = validate_all_uats_verified(&prd);
        assert!(result.is_err());

        if let Err(FinalizeError::UnverifiedUats {
            unverified_count,
            uat_details,
        }) = result
        {
            assert_eq!(unverified_count, 1);
            assert_eq!(uat_details.len(), 1);
            assert_eq!(uat_details[0].0, "uat-002");
            assert_eq!(uat_details[0].1, "Test uat-002");
        } else {
            panic!("Expected UnverifiedUats error");
        }
    }

    #[test]
    fn test_validate_all_uats_verified_with_no_uats() {
        let prd = make_test_prd("PRD-0001", vec![make_task("T-001", TaskStatus::Done)]);

        // No UATs means validation passes.
        assert!(validate_all_uats_verified(&prd).is_ok());
    }

    #[test]
    fn test_validate_multiple_unverified_uats() {
        let prd = make_test_prd_with_uats(
            "PRD-0001",
            vec![make_task("T-001", TaskStatus::Done)],
            vec![
                make_uat("uat-001", UatStatus::Verified),
                make_uat("uat-002", UatStatus::Unverified),
                make_uat("uat-003", UatStatus::Unverified),
            ],
        );

        let result = validate_all_uats_verified(&prd);
        assert!(result.is_err());

        if let Err(FinalizeError::UnverifiedUats {
            unverified_count,
            uat_details,
        }) = result
        {
            assert_eq!(unverified_count, 2);
            assert_eq!(uat_details.len(), 2);
        } else {
            panic!("Expected UnverifiedUats error");
        }
    }

    #[test]
    fn test_validate_all_unverified_uats() {
        let prd = make_test_prd_with_uats(
            "PRD-0001",
            vec![make_task("T-001", TaskStatus::Done)],
            vec![
                make_uat("uat-001", UatStatus::Unverified),
                make_uat("uat-002", UatStatus::Unverified),
            ],
        );

        let result = validate_all_uats_verified(&prd);
        assert!(result.is_err());

        if let Err(FinalizeError::UnverifiedUats {
            unverified_count,
            uat_details: _,
        }) = result
        {
            assert_eq!(unverified_count, 2);
        } else {
            panic!("Expected UnverifiedUats error");
        }
    }

    #[test]
    fn finalize_unverified_blocks() {
        // Integration test: Unverified UATs block PRD finalization.
        // This test verifies that when all tasks are done but UATs remain unverified,
        // the finalize_prd validation logic correctly prevents finalization.

        let prd = make_test_prd_with_uats(
            "PRD-0001",
            vec![make_task("T-001", TaskStatus::Done)],
            vec![
                make_uat("uat-001", UatStatus::Verified),
                make_uat("uat-002", UatStatus::Unverified),
                make_uat("uat-003", UatStatus::Unverified),
            ],
        );

        // Validate that finalization is blocked by unverified UATs.
        let result = validate_all_uats_verified(&prd);
        assert!(
            result.is_err(),
            "Finalization should be blocked when UATs are unverified"
        );

        // Verify the error details.
        match result {
            Err(FinalizeError::UnverifiedUats {
                unverified_count,
                uat_details,
            }) => {
                assert_eq!(unverified_count, 2, "Should report 2 unverified UATs");
                assert_eq!(uat_details.len(), 2, "Should provide details for 2 UATs");
                assert_eq!(uat_details[0].0, "uat-002");
                assert_eq!(uat_details[1].0, "uat-003");
            }
            _ => panic!("Expected UnverifiedUats error"),
        }

        // Verify that PRDs with all UATs verified can proceed.
        let verified_prd = make_test_prd_with_uats(
            "PRD-0002",
            vec![make_task("T-001", TaskStatus::Done)],
            vec![
                make_uat("uat-001", UatStatus::Verified),
                make_uat("uat-002", UatStatus::Verified),
            ],
        );

        let verified_result = validate_all_uats_verified(&verified_prd);
        assert!(
            verified_result.is_ok(),
            "Finalization should proceed when all UATs are verified"
        );
    }

    #[test]
    fn test_constitution_prd_finalize() {
        // UAT: constitution_prd_finalize — Verify prd finalize reads and respects constitution
        // This test verifies that when a constitution file exists, its content
        // is loaded and included in the finalization prompt.

        let temp = tempfile::TempDir::new().unwrap();
        let mr_dir = temp.path().join(".mr");
        let prompts_dir = mr_dir.join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create constitution file
        let constitution_content = r#"# Constitution

## Purpose
Project governance rules.

## Rules
1. **Acceptance tests must be codified** — No one-off manual tests.
2. **Use semantic versioning** — All releases follow semver.
"#;
        std::fs::write(mr_dir.join("constitution.md"), constitution_content).unwrap();

        // Create a minimal finalize prompt template that includes constitution placeholder
        std::fs::write(
            prompts_dir.join("run_task_finalize.md"),
            "Finalize PRD {{prd_id}}{{#if constitution}}\n\nConstitution:\n{{constitution}}{{/if}}",
        )
        .unwrap();

        // Create a test PRD with all tasks done
        let prd = make_test_prd_with_uats(
            "PRD-0001",
            vec![make_task("T-001", TaskStatus::Done)],
            vec![make_uat("uat-001", UatStatus::Verified)],
        );

        // Build the finalization prompt
        let prompt = build_finalize_prompt(temp.path(), &prd);

        // Verify constitution was loaded and included in the prompt
        assert!(
            prompt.contains("Acceptance tests must be codified"),
            "Finalize prompt should contain constitution content"
        );
        assert!(
            prompt.contains("Use semantic versioning"),
            "Finalize prompt should contain full constitution content"
        );
    }
}
