//! `mr status` command implementation.
//!
//! Provides an overview of PRDs and tasks, showing the next task to run
//! and the last History entry for context.

use std::fmt::Write;
use std::path::Path;

use anyhow::Result;

use crate::prd::{Prd, PrdStatus, PrdSummary, scan_prds};

/// Status summary for display.
#[derive(Debug)]
pub struct StatusReport {
    /// All PRD summaries.
    pub prds: Vec<PrdSummary>,

    /// The next task to run (if any).
    pub next_task: Option<NextTaskInfo>,

    /// Statistics.
    pub stats: StatusStats,
}

/// Information about the next task.
#[derive(Debug, Clone)]
pub struct NextTaskInfo {
    /// PRD ID.
    pub prd_id: String,

    /// PRD title.
    pub prd_title: String,

    /// Task ID.
    pub task_id: String,

    /// Task title.
    pub task_title: String,

    /// Task priority.
    pub priority: u32,

    /// Task notes (if any).
    pub notes: Option<String>,

    /// Last history entry summary (if any).
    pub last_history: Option<String>,
}

/// Statistics about PRDs and tasks.
#[derive(Debug, Default)]
pub struct StatusStats {
    /// Total PRDs.
    pub total_prds: usize,

    /// Active PRDs.
    pub active_prds: usize,

    /// Draft PRDs.
    pub draft_prds: usize,

    /// Done PRDs.
    pub done_prds: usize,

    /// Parked PRDs.
    pub parked_prds: usize,

    /// Total tasks across all PRDs.
    pub total_tasks: usize,

    /// Completed tasks across all PRDs.
    pub completed_tasks: usize,
}

/// Extracts the last History entry from the PRD body.
///
/// Looks for the History section and returns the most recent entry (last `## YYYY-MM-DD` block).
fn extract_last_history(body: &str) -> Option<String> {
    // Find the History section.
    let history_start = body.find("# History")?;
    let history_section = &body[history_start..];

    // Find all `## ` entries after History header.
    let mut entries: Vec<&str> = Vec::new();
    let mut current_start: Option<usize> = None;

    for (i, line) in history_section.lines().enumerate() {
        if line.starts_with("## ") && i > 0 {
            // Skip the first line which is the History header.
            if let Some(start) = current_start {
                // End of previous entry (we'll collect it when we find the start of the next one).
                let lines: Vec<&str> = history_section.lines().collect();
                let entry: String = lines[start..i].join("\n");
                entries.push(Box::leak(entry.into_boxed_str()));
            }

            current_start = Some(i);
        }
    }

    // Collect the last entry.
    if let Some(start) = current_start {
        let lines: Vec<&str> = history_section.lines().collect();
        let entry: String = lines[start..].join("\n");
        entries.push(Box::leak(entry.into_boxed_str()));
    }

    // Return the last entry (most recent).
    entries.last().map(|s| {
        // Trim and limit to last few lines for summary.
        let lines: Vec<&str> = s.lines().collect();
        let start = lines.len().saturating_sub(6);
        lines[start..].join("\n")
    })
}

/// Finds the next task to run.
///
/// Strategy: First active PRD with incomplete tasks, then pick highest priority task.
fn find_next_task(prds: &[(String, Prd, std::path::PathBuf)]) -> Option<NextTaskInfo> {
    for (_filename, prd, _path) in prds {
        if prd.status() != PrdStatus::Active {
            continue;
        }

        if let Some(task) = prd.next_task() {
            let last_history = extract_last_history(&prd.body);

            return Some(NextTaskInfo {
                prd_id: prd.id().to_string(),
                prd_title: prd.title().to_string(),
                task_id: task.id.clone(),
                task_title: task.title.clone(),
                priority: task.priority,
                notes: task.notes.clone(),
                last_history,
            });
        }
    }

    None
}

/// Generates a status report for the repository.
pub fn get_status(root: &Path) -> Result<StatusReport> {
    let prds_dir = root.join(".mr").join("prds");
    let prds = scan_prds(&prds_dir)?;

    // Build summaries.
    let summaries: Vec<PrdSummary> = prds
        .iter()
        .map(|(filename, prd, _abs_path)| {
            let relative_path = format!("prds/{filename}");
            PrdSummary::from_prd(prd, relative_path)
        })
        .collect();

    // Calculate statistics.
    let mut stats = StatusStats {
        total_prds: summaries.len(),
        ..Default::default()
    };

    for summary in &summaries {
        match summary.status {
            PrdStatus::Active => stats.active_prds += 1,
            PrdStatus::Draft => stats.draft_prds += 1,
            PrdStatus::Done => stats.done_prds += 1,
            PrdStatus::Parked => stats.parked_prds += 1,
        }

        stats.total_tasks += summary.total_tasks;
        stats.completed_tasks += summary.completed_tasks;
    }

    // Find next task.
    let next_task = find_next_task(&prds);

    Ok(StatusReport {
        prds: summaries,
        next_task,
        stats,
    })
}

/// Formats the status report for display.
#[allow(clippy::too_many_lines)]
pub fn format_status(report: &StatusReport) -> String {
    let mut output = String::new();

    // Header.
    output.push_str("microralph Status\n");
    output.push_str("==================\n\n");

    // Next task (most important info).
    if let Some(next) = &report.next_task {
        output.push_str("## Next Task\n\n");
        output.push_str("  PRD: ");
        output.push_str(&next.prd_id);
        output.push_str(" — ");
        output.push_str(&next.prd_title);
        output.push_str("\n  Task: ");
        output.push_str(&next.task_id);
        output.push_str(" — ");
        output.push_str(&next.task_title);
        output.push_str(" (priority ");
        output.push_str(&next.priority.to_string());
        output.push_str(")\n");

        if let Some(notes) = &next.notes {
            output.push_str("  Notes: ");
            output.push_str(notes);
            output.push('\n');
        }

        output.push('\n');

        if let Some(history) = &next.last_history {
            output.push_str("## Last History Entry\n\n");

            for line in history.lines() {
                output.push_str("  ");
                output.push_str(line);
                output.push('\n');
            }

            output.push('\n');
        }
    } else {
        output.push_str("## Next Task\n\n");
        output.push_str("  No active PRDs with incomplete tasks.\n\n");
    }

    // PRD Summary.
    output.push_str("## PRDs\n\n");

    if report.prds.is_empty() {
        output.push_str("  No PRDs found. Create one with `mr new <slug>`.\n");
    } else {
        // Group by status.
        let active: Vec<_> = report
            .prds
            .iter()
            .filter(|p| p.status == PrdStatus::Active)
            .collect();

        let draft: Vec<_> = report
            .prds
            .iter()
            .filter(|p| p.status == PrdStatus::Draft)
            .collect();

        let done: Vec<_> = report
            .prds
            .iter()
            .filter(|p| p.status == PrdStatus::Done)
            .collect();

        let parked: Vec<_> = report
            .prds
            .iter()
            .filter(|p| p.status == PrdStatus::Parked)
            .collect();

        if !active.is_empty() {
            output.push_str("  Active:\n");

            for prd in active {
                output.push_str("    ");
                output.push_str(&prd.id);
                output.push_str(" — ");
                output.push_str(&prd.title);
                output.push_str(" [");
                output.push_str(&prd.completed_tasks.to_string());
                output.push('/');
                output.push_str(&prd.total_tasks.to_string());
                output.push_str("]\n");
            }
        }

        if !draft.is_empty() {
            output.push_str("  Draft:\n");

            for prd in draft {
                output.push_str("    ");
                output.push_str(&prd.id);
                output.push_str(" — ");
                output.push_str(&prd.title);
                output.push_str(" [");
                output.push_str(&prd.completed_tasks.to_string());
                output.push('/');
                output.push_str(&prd.total_tasks.to_string());
                output.push_str("]\n");
            }
        }

        if !done.is_empty() {
            output.push_str("  Done:\n");

            for prd in done {
                output.push_str("    ");
                output.push_str(&prd.id);
                output.push_str(" — ");
                output.push_str(&prd.title);
                output.push_str(" [");
                output.push_str(&prd.completed_tasks.to_string());
                output.push('/');
                output.push_str(&prd.total_tasks.to_string());
                output.push_str("]\n");
            }
        }

        if !parked.is_empty() {
            output.push_str("  Parked:\n");

            for prd in parked {
                output.push_str("    ");
                output.push_str(&prd.id);
                output.push_str(" — ");
                output.push_str(&prd.title);
                output.push_str(" [");
                output.push_str(&prd.completed_tasks.to_string());
                output.push('/');
                output.push_str(&prd.total_tasks.to_string());
                output.push_str("]\n");
            }
        }
    }

    output.push('\n');

    // Statistics.
    output.push_str("## Statistics\n\n");
    output.push_str("  PRDs: ");
    output.push_str(&report.stats.total_prds.to_string());
    output.push_str(" total");

    if report.stats.total_prds > 0 {
        output.push_str(" (");
        output.push_str(&report.stats.active_prds.to_string());
        output.push_str(" active, ");
        output.push_str(&report.stats.draft_prds.to_string());
        output.push_str(" draft, ");
        output.push_str(&report.stats.done_prds.to_string());
        output.push_str(" done, ");
        output.push_str(&report.stats.parked_prds.to_string());
        output.push_str(" parked)");
    }

    output.push('\n');

    output.push_str("  Tasks: ");
    output.push_str(&report.stats.completed_tasks.to_string());
    output.push('/');
    output.push_str(&report.stats.total_tasks.to_string());
    output.push_str(" completed");

    if report.stats.total_tasks > 0 {
        // Task counts are small enough that precision loss is negligible.
        #[allow(clippy::cast_precision_loss)]
        let pct = (report.stats.completed_tasks as f64 / report.stats.total_tasks as f64) * 100.0;
        output.push_str(" (");
        let _ = write!(output, "{pct:.0}");
        output.push_str("%)");
    }

    output.push('\n');

    output
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::prd::{PrdFrontmatter, Task, TaskStatus};
    use tempfile::TempDir;

    fn setup_test_repo(temp: &TempDir) -> std::path::PathBuf {
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");

        std::fs::create_dir_all(&prds_dir).unwrap();

        root
    }

    fn create_test_prd(
        prds_dir: &Path,
        id: &str,
        title: &str,
        status: PrdStatus,
        tasks: Vec<Task>,
        history: Option<&str>,
    ) {
        let frontmatter = PrdFrontmatter {
            id: id.to_string(),
            title: title.to_string(),
            status,
            tasks: if tasks.is_empty() { None } else { Some(tasks) },
            ..Default::default()
        };

        let body = if let Some(hist) = history {
            format!("# Summary\n\nTest PRD.\n\n# History\n\n{hist}\n")
        } else {
            "# Summary\n\nTest PRD.\n".to_string()
        };

        let prd = Prd::new(frontmatter, body);
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let filename = format!("{id}-test.md");

        std::fs::write(prds_dir.join(filename), content).unwrap();
    }

    fn make_task(id: &str, priority: u32, status: TaskStatus) -> Task {
        Task {
            id: id.to_string(),
            title: format!("Task {id}"),
            priority,
            status,
            notes: None,
        }
    }

    #[test]
    fn test_get_status_empty() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        let report = get_status(&root).unwrap();

        assert!(report.prds.is_empty());
        assert!(report.next_task.is_none());
        assert_eq!(report.stats.total_prds, 0);
    }

    #[test]
    fn test_get_status_with_prds() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(
            &prds_dir,
            "PRD-0001",
            "First PRD",
            PrdStatus::Active,
            vec![
                make_task("T-001", 1, TaskStatus::Done),
                make_task("T-002", 2, TaskStatus::Todo),
            ],
            None,
        );

        create_test_prd(
            &prds_dir,
            "PRD-0002",
            "Second PRD",
            PrdStatus::Draft,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
            None,
        );

        let report = get_status(&root).unwrap();

        assert_eq!(report.prds.len(), 2);
        assert_eq!(report.stats.total_prds, 2);
        assert_eq!(report.stats.active_prds, 1);
        assert_eq!(report.stats.draft_prds, 1);
        assert_eq!(report.stats.total_tasks, 3);
        assert_eq!(report.stats.completed_tasks, 1);
    }

    #[test]
    fn test_find_next_task() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(
            &prds_dir,
            "PRD-0001",
            "Test PRD",
            PrdStatus::Active,
            vec![
                make_task("T-001", 2, TaskStatus::Done),
                make_task("T-002", 1, TaskStatus::Todo),
            ],
            None,
        );

        let report = get_status(&root).unwrap();

        assert!(report.next_task.is_some());

        let next = report.next_task.unwrap();

        assert_eq!(next.prd_id, "PRD-0001");
        assert_eq!(next.task_id, "T-002");
    }

    #[test]
    fn test_next_task_skips_draft_prds() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(
            &prds_dir,
            "PRD-0001",
            "Draft PRD",
            PrdStatus::Draft,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
            None,
        );

        let report = get_status(&root).unwrap();

        assert!(report.next_task.is_none());
    }

    #[test]
    fn test_extract_last_history() {
        let body = r"# Summary

Test PRD.

# History

## 2026-01-23 — Initial
- First entry

## 2026-01-24 — T-001 Completed
- **Task**: Do something
- **Status**: ✅ Done
- **Changes**:
  - Changed something
";

        let history = extract_last_history(body).unwrap();

        assert!(history.contains("2026-01-24"));
        assert!(history.contains("T-001 Completed"));
    }

    #[test]
    fn test_extract_last_history_none() {
        let body = "# Summary\n\nNo history here.\n";

        let history = extract_last_history(body);

        assert!(history.is_none());
    }

    #[test]
    fn test_format_status() {
        let report = StatusReport {
            prds: vec![PrdSummary {
                id: "PRD-0001".to_string(),
                title: "Test PRD".to_string(),
                status: PrdStatus::Active,
                completed_tasks: 2,
                total_tasks: 5,
                verified_uats: 0,
                total_uats: 0,
                relative_path: "prds/PRD-0001.md".to_string(),
                references: vec![],
                depends_on: vec![],
            }],
            next_task: Some(NextTaskInfo {
                prd_id: "PRD-0001".to_string(),
                prd_title: "Test PRD".to_string(),
                task_id: "T-003".to_string(),
                task_title: "Do something".to_string(),
                priority: 3,
                notes: Some("Be careful".to_string()),
                last_history: None,
            }),
            stats: StatusStats {
                total_prds: 1,
                active_prds: 1,
                draft_prds: 0,
                done_prds: 0,
                parked_prds: 0,
                total_tasks: 5,
                completed_tasks: 2,
            },
        };

        let output = format_status(&report);

        assert!(output.contains("microralph Status"));
        assert!(output.contains("PRD-0001"));
        assert!(output.contains("T-003"));
        assert!(output.contains("Do something"));
        assert!(output.contains("Be careful"));
        assert!(output.contains("2/5"));
        assert!(output.contains("40%"));
    }

    #[test]
    fn test_format_status_empty() {
        let report = StatusReport {
            prds: vec![],
            next_task: None,
            stats: StatusStats::default(),
        };

        let output = format_status(&report);

        assert!(output.contains("No active PRDs with incomplete tasks"));
        assert!(output.contains("No PRDs found"));
    }

    #[test]
    fn test_status_with_history() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        create_test_prd(
            &prds_dir,
            "PRD-0001",
            "Test PRD",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
            Some(
                r"## 2026-01-24 — T-000 Completed
- **Task**: Setup
- **Status**: ✅ Done",
            ),
        );

        let report = get_status(&root).unwrap();
        let next = report.next_task.unwrap();

        assert!(next.last_history.is_some());

        let history = next.last_history.unwrap();

        assert!(history.contains("2026-01-24"));
    }
}
