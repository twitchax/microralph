//! `mr run` command implementation.
//!
//! Picks the next task from an active PRD and invokes the runner to execute it.
//! The runner is responsible for:
//! - Implementing the task
//! - Running `cargo make uat`
//! - Updating the task status in the PRD frontmatter
//! - Appending to the History section
//! - Regenerating `.mr/PRDS.md`

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::config::load_constitution;
use crate::prd::{AcceptanceTest, Prd, PrdStatus, TaskStatus, scan_prds};
use crate::prompt::{
    PlaceholderContext, PromptKind, expand_placeholders, load_prompt_with_fallback,
};
use crate::runner::{Runner, RunnerOutput, UsageInfo};

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

/// Result from running a task or checking run status.
#[derive(Debug)]
pub enum RunResult {
    /// A task was executed.
    TaskExecuted {
        /// The PRD that was run.
        prd_id: String,

        /// The task that was attempted.
        task_id: String,

        /// The task title.
        task_title: String,

        /// Path to the PRD file.
        prd_path: PathBuf,

        /// Whether the runner reported success.
        runner_success: bool,

        /// Runner output summary.
        output_summary: String,

        /// Optional usage information from the underlying agent.
        usage: Option<UsageInfo>,
    },

    /// All tasks are done but there are unverified UATs that need verification.
    NeedsUatVerification {
        /// The PRD that needs UAT verification.
        prd_id: String,

        /// Path to the PRD file.
        prd_path: PathBuf,

        /// Number of unverified UATs.
        unverified_count: usize,
    },

    /// PRD is fully complete (all tasks done, no unverified UATs).
    PrdComplete {
        /// The PRD that is complete.
        prd_id: String,

        /// Path to the PRD file.
        prd_path: PathBuf,
    },
}

/// Result from running the UAT verification loop.
#[derive(Debug)]
pub struct UatVerificationLoopResult {
    /// The PRD ID.
    #[allow(dead_code)] // Used for display purposes.
    pub prd_id: String,

    /// Path to the PRD file.
    #[allow(dead_code)] // Used for display purposes.
    pub prd_path: PathBuf,

    /// Number of UATs verified.
    pub verified_count: usize,

    /// Number of UATs opted out.
    pub opted_out_count: usize,

    /// Total iterations performed.
    pub iterations: usize,

    /// Whether the loop was stopped due to max_iterations limit.
    pub hit_max_iterations: bool,

    /// Remaining unverified UATs after the loop.
    pub remaining_unverified: usize,
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

    // Display the command being invoked (without the prompt)
    if let Some(cmd_display) = runner.format_command_display(&prompt, root) {
        println!("\n🔧 Executing: {}", cmd_display);
    }

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

    // Load constitution if available.
    if let Ok(Some(constitution)) = load_constitution(root) {
        ctx.insert("constitution", constitution);
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
                    "No active PRD with incomplete tasks found. Create a PRD with `mr new`."
                )
            })?;

        find_prd_by_id(config.root, &picked_id)?
            .ok_or_else(|| anyhow::anyhow!("Runner picked PRD {picked_id}, but it was not found"))?
    };

    // Pick the next task.
    let Some(task) = prd.next_task() else {
        // All tasks are done. Check if there are unverified UATs.
        let prd_id = prd.id().to_string();

        if prd.has_unverified_uats() {
            let unverified_count = prd.unverified_uats().len();

            tracing::info!(
                prd_id = %prd_id,
                unverified_count = unverified_count,
                "All tasks done but UATs need verification"
            );

            return Ok(RunResult::NeedsUatVerification {
                prd_id,
                prd_path,
                unverified_count,
            });
        }

        tracing::info!(
            prd_id = %prd_id,
            "PRD is complete (all tasks done, all UATs verified)"
        );

        return Ok(RunResult::PrdComplete { prd_id, prd_path });
    };

    let task_id = task.id.clone();
    let task_title = task.title.clone();
    let prd_id = prd.id().to_string();

    // Calculate task progress.
    let tasks = prd.tasks().unwrap_or(&[]);
    let total_tasks = tasks.len();
    let completed_tasks = tasks
        .iter()
        .filter(|t| t.status == TaskStatus::Done)
        .count();
    let current_task_num = completed_tasks + 1; // This is the task we're working on.

    tracing::info!(
        prd_id = %prd_id,
        task_id = %task_id,
        task_title = %task_title,
        current_task = current_task_num,
        total_tasks = total_tasks,
        "Running task"
    );

    // Build and execute the prompt.
    let prompt = build_prompt(config.root, &prd, &prd_path, &task_id);

    tracing::info!(
        prompt_len = prompt.len(),
        runner = %runner.name(),
        prd_id = %prd_id,
        task_id = %task_id,
        stream = config.stream,
        "Invoking runner to execute task"
    );

    // Display the command being invoked (without the prompt)
    if let Some(cmd_display) = runner.format_command_display(&prompt, config.root) {
        println!("\n🔧 Executing: {}", cmd_display);
    }

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
        let start = output.text.len() - 500;
        format!("... (truncated)\n{}", &output.text[start..])
    } else {
        output.text.clone()
    };

    Ok(RunResult::TaskExecuted {
        prd_id,
        task_id,
        task_title,
        prd_path,
        runner_success: output.success,
        output_summary,
        usage: output.usage,
    })
}

/// Default max iterations for UAT verification loop if not configured.
const DEFAULT_MAX_UAT_ITERATIONS: u32 = 10;

/// Builds the prompt for UAT verification.
fn build_uat_verify_prompt(
    root: &Path,
    prd: &Prd,
    prd_path: &Path,
    uat: &AcceptanceTest,
) -> String {
    let prompt_template = load_prompt_with_fallback(root, PromptKind::RunUatVerify);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("prd_path", prd_path.display().to_string());
    ctx.insert("prd_id", prd.id());
    ctx.insert("uat_id", uat.id.clone());
    ctx.insert("uat_name", uat.name.clone());
    ctx.insert("uat_command", uat.command.clone());

    expand_placeholders(&prompt_template, &ctx)
}

/// Checks if a runner response contains an OPT-OUT.
fn parse_opt_out(text: &str) -> Option<String> {
    // Look for "OPT-OUT:" pattern in the response.
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("OPT-OUT:") {
            let explanation = trimmed.strip_prefix("OPT-OUT:").unwrap_or("").trim();
            return Some(explanation.to_string());
        }
    }

    None
}

/// Appends an opt-out History entry to the PRD.
///
/// This ensures that even if the runner doesn't append a History entry, the opt-out is recorded.
fn append_opt_out_history(
    prd_path: &Path,
    uat_id: &str,
    uat_name: &str,
    explanation: &str,
) -> Result<()> {
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    let history_entry = format!(
        "\n## {} — {} Opt-Out\n- **UAT**: {}\n- **Status**: ⏭️ Opted-out\n- **Reason**: {}\n",
        today, uat_id, uat_name, explanation
    );

    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(prd_path)
        .with_context(|| {
            format!(
                "Failed to open PRD file for appending opt-out history: {}",
                prd_path.display()
            )
        })?;

    write!(file, "{}", history_entry)?;

    tracing::debug!(
        prd_path = %prd_path.display(),
        uat_id = %uat_id,
        "Appended opt-out History entry"
    );

    Ok(())
}

/// Updates a UAT's status to verified in the PRD frontmatter.
///
/// This reads the PRD, finds the matching UAT by ID, updates its status to verified,
/// and writes the PRD back to disk.
///
/// # Arguments
///
/// * `prd_path` - Path to the PRD file
/// * `uat_id` - ID of the UAT to update (e.g., "uat-001")
///
/// # Errors
///
/// Returns an error if:
/// - The PRD file cannot be read or parsed
/// - The UAT ID is not found in the PRD
/// - The updated PRD cannot be written back
pub fn update_uat_status(prd_path: &Path, uat_id: &str) -> Result<()> {
    use crate::prd::{UatStatus, parse_prd_file, serialize_prd};

    // Read and parse the PRD.
    let mut prd = parse_prd_file(prd_path).with_context(|| {
        format!(
            "Failed to read PRD for UAT status update: {}",
            prd_path.display()
        )
    })?;

    // Find and update the UAT.
    let acceptance_tests = prd.frontmatter.acceptance_tests.as_mut().ok_or_else(|| {
        anyhow::anyhow!(
            "PRD has no acceptance_tests section: {}",
            prd_path.display()
        )
    })?;

    let uat = acceptance_tests
        .iter_mut()
        .find(|t| t.id == uat_id)
        .ok_or_else(|| anyhow::anyhow!("UAT not found: {} in {}", uat_id, prd_path.display()))?;

    // Update status to verified.
    uat.uat_status = UatStatus::Verified;

    // Serialize and write back.
    let content = serialize_prd(&prd).with_context(|| {
        format!(
            "Failed to serialize PRD after UAT update: {}",
            prd_path.display()
        )
    })?;

    fs::write(prd_path, &content).with_context(|| {
        format!(
            "Failed to write PRD after UAT update: {}",
            prd_path.display()
        )
    })?;

    tracing::info!(
        prd_path = %prd_path.display(),
        uat_id = %uat_id,
        "Updated UAT status to verified"
    );

    Ok(())
}

/// Configuration for UAT verification loop.
#[derive(Debug)]
pub struct UatVerificationConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// The PRD ID to verify UATs for.
    pub prd_id: &'a str,

    /// Path to the PRD file (for display purposes).
    #[allow(dead_code)]
    pub prd_path: &'a Path,

    /// Whether to stream runner output in real-time.
    pub stream: bool,

    /// Maximum number of iterations (None = use default).
    pub max_iterations: Option<u32>,
}

/// Runs the UAT verification loop.
///
/// Iterates over unverified UATs, invoking the runner for each one.
/// Respects max_iterations limit and handles OPT-OUT responses.
///
/// # Arguments
///
/// * `config` - Configuration for the verification loop
/// * `runner` - The runner to use for verification
///
/// # Returns
///
/// A `UatVerificationLoopResult` describing what happened.
pub fn run_uat_verification_loop(
    config: &UatVerificationConfig,
    runner: &dyn Runner,
) -> Result<UatVerificationLoopResult> {
    // Load the PRD to get current state.
    let (_filename, prd, prd_path) = find_prd_by_id(config.root, config.prd_id)?
        .ok_or_else(|| anyhow::anyhow!("PRD not found: {}", config.prd_id))?;

    // Get max_iterations from PRD config or use default.
    let max_iterations = config.max_iterations.unwrap_or_else(|| {
        prd.frontmatter
            .loop_config
            .as_ref()
            .and_then(|lc| lc.max_iterations)
            .unwrap_or(DEFAULT_MAX_UAT_ITERATIONS)
    });

    let mut verified_count = 0;
    let mut opted_out_count = 0;
    let mut iterations = 0;

    tracing::info!(
        prd_id = %config.prd_id,
        max_iterations = max_iterations,
        "Starting UAT verification loop"
    );

    loop {
        // Reload PRD to get current unverified UATs (they may have been updated by runner).
        let (_filename, current_prd, current_prd_path) =
            find_prd_by_id(config.root, config.prd_id)?
                .ok_or_else(|| anyhow::anyhow!("PRD not found: {}", config.prd_id))?;

        let unverified = current_prd.unverified_uats();

        if unverified.is_empty() {
            tracing::info!(
                prd_id = %config.prd_id,
                verified = verified_count,
                opted_out = opted_out_count,
                "All UATs verified or opted out"
            );
            break;
        }

        if iterations >= max_iterations as usize {
            tracing::info!(
                prd_id = %config.prd_id,
                iterations = iterations,
                max_iterations = max_iterations,
                remaining = unverified.len(),
                "Hit max iterations limit"
            );

            return Ok(UatVerificationLoopResult {
                prd_id: config.prd_id.to_string(),
                prd_path: prd_path.clone(),
                verified_count,
                opted_out_count,
                iterations,
                hit_max_iterations: true,
                remaining_unverified: unverified.len(),
            });
        }

        // Get the next unverified UAT.
        let uat = unverified[0];

        // Calculate UAT progress.
        let all_uats = current_prd
            .frontmatter
            .acceptance_tests
            .as_ref()
            .map(|tests| tests.len())
            .unwrap_or(0);
        let current_uat_num = all_uats - unverified.len() + 1; // 1-indexed position.

        tracing::info!(
            prd_id = %config.prd_id,
            uat_id = %uat.id,
            uat_name = %uat.name,
            current_uat = current_uat_num,
            total_uats = all_uats,
            iteration = iterations + 1,
            max_iterations = max_iterations,
            "Verifying UAT"
        );

        // Build and execute the verification prompt.
        let prompt = build_uat_verify_prompt(config.root, &current_prd, &current_prd_path, uat);

        // Display the command being invoked (without the prompt)
        if let Some(cmd_display) = runner.format_command_display(&prompt, config.root) {
            println!("\n🔧 Executing: {}", cmd_display);
        }

        let output = if config.stream {
            let mut stdout = std::io::stdout();
            runner
                .execute_streaming(&prompt, config.root, &mut stdout)
                .with_context(|| format!("Runner failed for UAT {}", uat.id))?
        } else {
            runner
                .execute(&prompt, config.root)
                .with_context(|| format!("Runner failed for UAT {}", uat.id))?
        };

        // Summarize output (truncate if too long). Skip summary if we already streamed.
        if !config.stream {
            if output.text.len() > 500 {
                let start = output.text.len() - 500;
                println!("... (truncated)\n{}", &output.text[start..]);
            } else {
                println!("{}", output.text);
            }
        }

        iterations += 1;

        // Check for OPT-OUT in response.
        if let Some(explanation) = parse_opt_out(&output.text) {
            tracing::info!(
                prd_id = %config.prd_id,
                uat_id = %uat.id,
                explanation = %explanation,
                "UAT verification opted out"
            );
            opted_out_count += 1;

            // Append opt-out History entry to the PRD.
            append_opt_out_history(&current_prd_path, &uat.id, &uat.name, &explanation)?;

            // Reload to check if UAT is still unverified (runner might have updated it).
            let (_f, refreshed_prd, _p) = find_prd_by_id(config.root, config.prd_id)?
                .ok_or_else(|| anyhow::anyhow!("PRD not found: {}", config.prd_id))?;

            // If UAT is still unverified after opt-out, we need to continue to next UAT.
            // The OPT-OUT means the runner decided not to verify this specific UAT.
            // The loop should still try to verify remaining UATs.
            if refreshed_prd
                .unverified_uats()
                .iter()
                .any(|u| u.id == uat.id)
            {
                // UAT is still unverified after opt-out - continue to see if others can be verified.
                continue;
            }
        } else if output.success {
            // Runner completed successfully - check if UAT was marked verified.
            let (_f, refreshed_prd, _p) = find_prd_by_id(config.root, config.prd_id)?
                .ok_or_else(|| anyhow::anyhow!("PRD not found: {}", config.prd_id))?;

            let still_unverified = refreshed_prd
                .unverified_uats()
                .iter()
                .any(|u| u.id == uat.id);

            if still_unverified {
                // Runner succeeded but didn't update UAT status - update it ourselves.
                update_uat_status(&current_prd_path, &uat.id)?;
                tracing::info!(
                    prd_id = %config.prd_id,
                    uat_id = %uat.id,
                    "UAT verified (status updated by microralph)"
                );
            } else {
                tracing::info!(
                    prd_id = %config.prd_id,
                    uat_id = %uat.id,
                    "UAT verified (status updated by runner)"
                );
            }
            verified_count += 1;
        }
    }

    // Final state check.
    let (_filename, final_prd, _final_path) = find_prd_by_id(config.root, config.prd_id)?
        .ok_or_else(|| anyhow::anyhow!("PRD not found: {}", config.prd_id))?;

    let remaining = final_prd.unverified_uats().len();

    Ok(UatVerificationLoopResult {
        prd_id: config.prd_id.to_string(),
        prd_path,
        verified_count,
        opted_out_count,
        iterations,
        hit_max_iterations: false,
        remaining_unverified: remaining,
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

        match result {
            RunResult::TaskExecuted {
                prd_id,
                task_id,
                runner_success,
                ..
            } => {
                assert_eq!(prd_id, "PRD-0001");
                assert_eq!(task_id, "T-002"); // Lower priority = higher precedence.
                assert!(runner_success);
            }
            _ => panic!("Expected TaskExecuted result"),
        }
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

        match result {
            RunResult::TaskExecuted { prd_id, .. } => {
                assert_eq!(prd_id, "PRD-0002");
            }
            _ => panic!("Expected TaskExecuted result"),
        }
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

    #[test]
    fn test_run_task_all_done_with_unverified_uats() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create a PRD with all tasks done but unverified UATs.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Task 1".to_string(),
                priority: 1,
                status: TaskStatus::Done,
                notes: None,
            }]),
            acceptance_tests: Some(vec![AcceptanceTest {
                id: "uat-001".to_string(),
                name: "Test 1".to_string(),
                command: "cargo test".to_string(),
                uat_status: UatStatus::Unverified,
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        std::fs::write(prds_dir.join("PRD-0001-test.md"), content).unwrap();

        let runner = MockRunner::new(vec![]);

        let config = RunConfig {
            root: &root,
            prd_id: Some("PRD-0001"),
            stream: false,
        };

        let result = run_task(&config, &runner).unwrap();

        match result {
            RunResult::NeedsUatVerification {
                prd_id,
                unverified_count,
                ..
            } => {
                assert_eq!(prd_id, "PRD-0001");
                assert_eq!(unverified_count, 1);
            }
            _ => panic!("Expected NeedsUatVerification result"),
        }
    }

    #[test]
    fn test_run_task_all_done_and_verified() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create a PRD with all tasks done and all UATs verified.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Task 1".to_string(),
                priority: 1,
                status: TaskStatus::Done,
                notes: None,
            }]),
            acceptance_tests: Some(vec![AcceptanceTest {
                id: "uat-001".to_string(),
                name: "Test 1".to_string(),
                command: "cargo test".to_string(),
                uat_status: UatStatus::Verified,
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        std::fs::write(prds_dir.join("PRD-0001-test.md"), content).unwrap();

        let runner = MockRunner::new(vec![]);

        let config = RunConfig {
            root: &root,
            prd_id: Some("PRD-0001"),
            stream: false,
        };

        let result = run_task(&config, &runner).unwrap();

        match result {
            RunResult::PrdComplete { prd_id, .. } => {
                assert_eq!(prd_id, "PRD-0001");
            }
            _ => panic!("Expected PrdComplete result"),
        }
    }

    #[test]
    fn test_run_task_all_done_no_uats() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create a PRD with all tasks done and no UATs.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Task 1".to_string(),
                priority: 1,
                status: TaskStatus::Done,
                notes: None,
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        std::fs::write(prds_dir.join("PRD-0001-test.md"), content).unwrap();

        let runner = MockRunner::new(vec![]);

        let config = RunConfig {
            root: &root,
            prd_id: Some("PRD-0001"),
            stream: false,
        };

        let result = run_task(&config, &runner).unwrap();

        match result {
            RunResult::PrdComplete { prd_id, .. } => {
                assert_eq!(prd_id, "PRD-0001");
            }
            _ => panic!("Expected PrdComplete result when no UATs defined"),
        }
    }

    #[test]
    fn test_parse_opt_out() {
        // Test basic OPT-OUT detection.
        assert_eq!(
            parse_opt_out("OPT-OUT: This requires manual testing"),
            Some("This requires manual testing".to_string())
        );

        // Test OPT-OUT with extra whitespace.
        assert_eq!(
            parse_opt_out("  OPT-OUT:  Too complex  "),
            Some("Too complex".to_string())
        );

        // Test OPT-OUT in multi-line response.
        let response = "I tried to verify this UAT but couldn't.\nOPT-OUT: Requires external API\nSee history for details.";
        assert_eq!(
            parse_opt_out(response),
            Some("Requires external API".to_string())
        );

        // Test no OPT-OUT.
        assert_eq!(parse_opt_out("Task completed successfully"), None);

        // Test partial match (not a real OPT-OUT).
        assert_eq!(parse_opt_out("OPT-OUT-ish"), None);
    }

    #[test]
    fn test_append_opt_out_history() {
        let temp = TempDir::new().unwrap();
        let prd_file = temp.path().join("PRD-0001-test.md");

        // Create a minimal PRD file.
        let prd_content = r#"---
id: PRD-0001
title: Test PRD
status: active
acceptance_tests:
  - id: uat-001
    name: Test UAT
    command: cargo test
    uat_status: unverified
---

# Summary

Test PRD.

# History
"#;
        std::fs::write(&prd_file, prd_content).unwrap();

        // Append opt-out history.
        append_opt_out_history(
            &prd_file,
            "uat-001",
            "Test UAT",
            "Requires external API access",
        )
        .unwrap();

        // Verify the History entry was appended.
        let updated_content = std::fs::read_to_string(&prd_file).unwrap();

        assert!(updated_content.contains("## "));
        assert!(updated_content.contains("uat-001 Opt-Out"));
        assert!(updated_content.contains("**UAT**: Test UAT"));
        assert!(updated_content.contains("⏭️ Opted-out"));
        assert!(updated_content.contains("**Reason**: Requires external API access"));
    }

    #[test]
    fn test_build_uat_verify_prompt() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        // Create the UAT verify prompt.
        std::fs::write(
            root.join(".mr/prompts/run_uat_verify.md"),
            "Verify UAT {{uat_id}}: {{uat_name}} for {{prd_id}} at {{prd_path}} using {{uat_command}}",
        )
        .unwrap();

        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            acceptance_tests: Some(vec![AcceptanceTest {
                id: "uat-001".to_string(),
                name: "Test 1".to_string(),
                command: "cargo test".to_string(),
                uat_status: UatStatus::Unverified,
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let prd_path = root.join(".mr/prds/PRD-0001.md");

        let uat = AcceptanceTest {
            id: "uat-001".to_string(),
            name: "Test 1".to_string(),
            command: "cargo test".to_string(),
            uat_status: UatStatus::Unverified,
        };

        let prompt = build_uat_verify_prompt(&root, &prd, &prd_path, &uat);

        assert!(prompt.contains("uat-001"));
        assert!(prompt.contains("Test 1"));
        assert!(prompt.contains("PRD-0001"));
        assert!(prompt.contains("cargo test"));
    }

    #[test]
    fn test_uat_verification_loop_all_verified_by_runner() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create the UAT verify prompt.
        std::fs::write(
            root.join(".mr/prompts/run_uat_verify.md"),
            "Verify UAT {{uat_id}}",
        )
        .unwrap();

        // Create a PRD with an already verified UAT (simulates runner having updated it).
        // This tests the loop correctly exits when no unverified UATs remain.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Task 1".to_string(),
                priority: 1,
                status: TaskStatus::Done,
                notes: None,
            }]),
            acceptance_tests: Some(vec![AcceptanceTest {
                id: "uat-001".to_string(),
                name: "Test 1".to_string(),
                command: "cargo test".to_string(),
                uat_status: UatStatus::Verified, // Already verified.
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let prd_file = prds_dir.join("PRD-0001-test.md");
        std::fs::write(&prd_file, content).unwrap();

        // Runner shouldn't be called since no unverified UATs exist.
        let runner = MockRunner::new(vec![]);

        let config = UatVerificationConfig {
            root: &root,
            prd_id: "PRD-0001",
            prd_path: &prd_file,
            stream: false,
            max_iterations: Some(5),
        };

        let result = run_uat_verification_loop(&config, &runner).unwrap();

        assert_eq!(result.prd_id, "PRD-0001");
        assert_eq!(result.verified_count, 0); // None verified in this loop - already was verified.
        assert_eq!(result.opted_out_count, 0);
        assert_eq!(result.iterations, 0); // No iterations needed.
        assert!(!result.hit_max_iterations);
        assert_eq!(result.remaining_unverified, 0);
    }

    #[test]
    fn test_uat_verification_loop_opt_out() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create the UAT verify prompt.
        std::fs::write(
            root.join(".mr/prompts/run_uat_verify.md"),
            "Verify UAT {{uat_id}}",
        )
        .unwrap();

        // Create a PRD with unverified UAT.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Task 1".to_string(),
                priority: 1,
                status: TaskStatus::Done,
                notes: None,
            }]),
            acceptance_tests: Some(vec![AcceptanceTest {
                id: "uat-001".to_string(),
                name: "Test 1".to_string(),
                command: "cargo test".to_string(),
                uat_status: UatStatus::Unverified,
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let prd_file = prds_dir.join("PRD-0001-test.md");
        std::fs::write(&prd_file, &content).unwrap();

        // Runner returns OPT-OUT without modifying the PRD.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(
            "OPT-OUT: Requires manual testing",
        )]);

        let config = UatVerificationConfig {
            root: &root,
            prd_id: "PRD-0001",
            prd_path: &prd_file,
            stream: false,
            max_iterations: Some(1), // Only 1 iteration allowed.
        };

        let result = run_uat_verification_loop(&config, &runner).unwrap();

        assert_eq!(result.prd_id, "PRD-0001");
        assert_eq!(result.opted_out_count, 1);
        assert_eq!(result.iterations, 1);
        assert!(result.hit_max_iterations);
        assert_eq!(result.remaining_unverified, 1);

        // Verify History entry was appended.
        let updated_content = std::fs::read_to_string(&prd_file).unwrap();
        assert!(updated_content.contains("uat-001 Opt-Out"));
        assert!(updated_content.contains("Requires manual testing"));
    }

    #[test]
    fn test_uat_verification_loop_max_iterations() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create the UAT verify prompt.
        std::fs::write(
            root.join(".mr/prompts/run_uat_verify.md"),
            "Verify UAT {{uat_id}}",
        )
        .unwrap();

        // Create a PRD with multiple unverified UATs.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Task 1".to_string(),
                priority: 1,
                status: TaskStatus::Done,
                notes: None,
            }]),
            acceptance_tests: Some(vec![
                AcceptanceTest {
                    id: "uat-001".to_string(),
                    name: "Test 1".to_string(),
                    command: "cargo test".to_string(),
                    uat_status: UatStatus::Unverified,
                },
                AcceptanceTest {
                    id: "uat-002".to_string(),
                    name: "Test 2".to_string(),
                    command: "cargo test".to_string(),
                    uat_status: UatStatus::Unverified,
                },
            ]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let prd_file = prds_dir.join("PRD-0001-test.md");
        std::fs::write(&prd_file, &content).unwrap();

        // Runner always succeeds - now microralph will update UAT status automatically.
        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Verified test 1"),
            crate::runner::RunnerOutput::success("Verified test 2"),
        ]);

        let config = UatVerificationConfig {
            root: &root,
            prd_id: "PRD-0001",
            prd_path: &prd_file,
            stream: false,
            max_iterations: Some(2),
        };

        let result = run_uat_verification_loop(&config, &runner).unwrap();

        assert_eq!(result.prd_id, "PRD-0001");
        assert_eq!(result.iterations, 2);
        assert!(!result.hit_max_iterations); // All UATs verified before hitting limit.
        assert_eq!(result.verified_count, 2); // Both UATs verified.
        assert_eq!(result.remaining_unverified, 0);

        // Verify the PRD was actually updated.
        let updated_prd = crate::prd::parse_prd_file(&prd_file).unwrap();
        let uats = updated_prd.frontmatter.acceptance_tests.unwrap();
        assert!(uats.iter().all(|u| u.uat_status == UatStatus::Verified));
    }

    #[test]
    fn test_update_uat_status() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");

        std::fs::create_dir_all(&prds_dir).unwrap();

        // Create a PRD with unverified UATs.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            acceptance_tests: Some(vec![
                AcceptanceTest {
                    id: "uat-001".to_string(),
                    name: "Test 1".to_string(),
                    command: "cargo test".to_string(),
                    uat_status: UatStatus::Unverified,
                },
                AcceptanceTest {
                    id: "uat-002".to_string(),
                    name: "Test 2".to_string(),
                    command: "cargo test".to_string(),
                    uat_status: UatStatus::Unverified,
                },
            ]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let prd_file = prds_dir.join("PRD-0001-test.md");
        std::fs::write(&prd_file, &content).unwrap();

        // Update uat-001 to verified.
        update_uat_status(&prd_file, "uat-001").unwrap();

        // Reload and verify.
        let updated = crate::prd::parse_prd_file(&prd_file).unwrap();
        let uats = updated.frontmatter.acceptance_tests.unwrap();

        let uat1 = uats.iter().find(|u| u.id == "uat-001").unwrap();
        let uat2 = uats.iter().find(|u| u.id == "uat-002").unwrap();

        assert_eq!(uat1.uat_status, UatStatus::Verified);
        assert_eq!(uat2.uat_status, UatStatus::Unverified);
    }

    #[test]
    fn test_update_uat_status_not_found() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");

        std::fs::create_dir_all(&prds_dir).unwrap();

        // Create a PRD with UAT.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            acceptance_tests: Some(vec![AcceptanceTest {
                id: "uat-001".to_string(),
                name: "Test 1".to_string(),
                command: "cargo test".to_string(),
                uat_status: UatStatus::Unverified,
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let prd_file = prds_dir.join("PRD-0001-test.md");
        std::fs::write(&prd_file, &content).unwrap();

        // Try to update non-existent UAT.
        let result = update_uat_status(&prd_file, "uat-999");

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("UAT not found"));
    }

    /// Integration test for the full UAT verification flow:
    /// 1. run_task() returns NeedsUatVerification when all tasks done but UATs unverified
    /// 2. run_uat_verification_loop() processes the unverified UATs
    /// 3. Loop respects max_iterations and correctly updates UAT status
    #[test]
    fn test_uat_verification_integration_flow() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create the UAT verify prompt.
        std::fs::write(
            root.join(".mr/prompts/run_uat_verify.md"),
            "Verify UAT {{uat_id}}: {{uat_name}}",
        )
        .unwrap();

        // Create a PRD with all tasks done but multiple unverified UATs.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Integration Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![
                Task {
                    id: "T-001".to_string(),
                    title: "Task 1".to_string(),
                    priority: 1,
                    status: TaskStatus::Done,
                    notes: None,
                },
                Task {
                    id: "T-002".to_string(),
                    title: "Task 2".to_string(),
                    priority: 2,
                    status: TaskStatus::Done,
                    notes: None,
                },
            ]),
            acceptance_tests: Some(vec![
                AcceptanceTest {
                    id: "uat-001".to_string(),
                    name: "Build passes".to_string(),
                    command: "cargo build".to_string(),
                    uat_status: UatStatus::Unverified,
                },
                AcceptanceTest {
                    id: "uat-002".to_string(),
                    name: "Tests pass".to_string(),
                    command: "cargo test".to_string(),
                    uat_status: UatStatus::Unverified,
                },
                AcceptanceTest {
                    id: "uat-003".to_string(),
                    name: "Lint passes".to_string(),
                    command: "cargo clippy".to_string(),
                    uat_status: UatStatus::Unverified,
                },
            ]),
            ..Default::default()
        };

        let prd = Prd::new(
            frontmatter,
            "# Summary\n\nIntegration test.\n\n# History\n".to_string(),
        );
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let prd_file = prds_dir.join("PRD-0001-integration-test.md");
        std::fs::write(&prd_file, &content).unwrap();

        // Step 1: run_task should return NeedsUatVerification.
        let run_runner = MockRunner::new(vec![]);
        let run_config = RunConfig {
            root: &root,
            prd_id: Some("PRD-0001"),
            stream: false,
        };

        let run_result = run_task(&run_config, &run_runner).unwrap();

        let (prd_id, prd_path) = match run_result {
            RunResult::NeedsUatVerification {
                prd_id,
                prd_path,
                unverified_count,
            } => {
                assert_eq!(prd_id, "PRD-0001");
                assert_eq!(unverified_count, 3);
                (prd_id, prd_path)
            }
            other => panic!("Expected NeedsUatVerification, got {:?}", other),
        };

        // Step 2: Run the UAT verification loop with max_iterations = 2.
        // Runner verifies first UAT, opts out of second UAT.
        let uat_runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Verified: build passes"),
            crate::runner::RunnerOutput::success("OPT-OUT: Requires CI environment"),
        ]);

        let uat_config = UatVerificationConfig {
            root: &root,
            prd_id: &prd_id,
            prd_path: &prd_path,
            stream: false,
            max_iterations: Some(2),
        };

        let uat_result = run_uat_verification_loop(&uat_config, &uat_runner).unwrap();

        // Verify loop behavior.
        assert_eq!(uat_result.prd_id, "PRD-0001");
        assert_eq!(uat_result.verified_count, 1); // First UAT verified.
        assert_eq!(uat_result.opted_out_count, 1); // Second UAT opted out.
        assert_eq!(uat_result.iterations, 2); // Ran for 2 iterations.
        assert!(uat_result.hit_max_iterations); // Hit the limit.
        assert_eq!(uat_result.remaining_unverified, 2); // uat-002 (opted out) + uat-003 (not reached).

        // Step 3: Verify PRD was updated correctly.
        let updated_prd = crate::prd::parse_prd_file(&prd_file).unwrap();
        let uats = updated_prd.frontmatter.acceptance_tests.unwrap();

        let uat1 = uats.iter().find(|u| u.id == "uat-001").unwrap();
        let uat2 = uats.iter().find(|u| u.id == "uat-002").unwrap();
        let uat3 = uats.iter().find(|u| u.id == "uat-003").unwrap();

        assert_eq!(uat1.uat_status, UatStatus::Verified); // Verified by loop.
        assert_eq!(uat2.uat_status, UatStatus::Unverified); // Opted out, still unverified.
        assert_eq!(uat3.uat_status, UatStatus::Unverified); // Not reached due to max_iterations.

        // Step 4: Verify History entry was appended for opt-out.
        let prd_content = std::fs::read_to_string(&prd_file).unwrap();
        assert!(prd_content.contains("uat-002 Opt-Out"));
        assert!(prd_content.contains("Requires CI environment"));
    }

    #[test]
    fn test_uat_verification_history_appending() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create the UAT verify prompt.
        std::fs::write(
            root.join(".mr/prompts/run_uat_verify.md"),
            "Verify UAT {{uat_id}}: {{uat_name}}",
        )
        .unwrap();

        // Create a PRD with unverified UATs.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0099".to_string(),
            title: "History Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Task 1".to_string(),
                priority: 1,
                status: TaskStatus::Done,
                notes: None,
            }]),
            acceptance_tests: Some(vec![
                AcceptanceTest {
                    id: "uat-001".to_string(),
                    name: "Successful verification".to_string(),
                    command: "cargo test".to_string(),
                    uat_status: UatStatus::Unverified,
                },
                AcceptanceTest {
                    id: "uat-002".to_string(),
                    name: "Opt-out verification".to_string(),
                    command: "cargo test".to_string(),
                    uat_status: UatStatus::Unverified,
                },
            ]),
            ..Default::default()
        };

        let prd = Prd::new(
            frontmatter,
            "# Summary\n\nHistory test.\n\n# History\n".to_string(),
        );
        let content = crate::prd::serialize_prd(&prd).unwrap();
        let prd_file = prds_dir.join("PRD-0099-history-test.md");
        std::fs::write(&prd_file, &content).unwrap();

        // Simulate verification loop: first UAT succeeds, second UAT opts out.
        // Set max_iterations=2 to stop after processing both UATs once.
        let uat_runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Verified via existing test"),
            crate::runner::RunnerOutput::success("OPT-OUT: Requires manual testing"),
        ]);

        let uat_config = UatVerificationConfig {
            root: &root,
            prd_id: "PRD-0099",
            prd_path: &prd_file,
            stream: false,
            max_iterations: Some(2), // Stop after 2 iterations to avoid retrying opted-out UAT.
        };

        let uat_result = run_uat_verification_loop(&uat_config, &uat_runner).unwrap();

        assert_eq!(uat_result.verified_count, 1);
        assert_eq!(uat_result.opted_out_count, 1);
        assert_eq!(uat_result.iterations, 2);
        assert!(uat_result.hit_max_iterations); // Loop stopped due to max_iterations, not because all UATs verified.
        assert_eq!(uat_result.remaining_unverified, 1); // uat-002 still unverified after opt-out.

        // Verify PRD frontmatter updates.
        let updated_prd = crate::prd::parse_prd_file(&prd_file).unwrap();
        let uats = updated_prd.frontmatter.acceptance_tests.unwrap();
        let uat1 = uats.iter().find(|u| u.id == "uat-001").unwrap();
        let uat2 = uats.iter().find(|u| u.id == "uat-002").unwrap();
        assert_eq!(uat1.uat_status, UatStatus::Verified);
        assert_eq!(uat2.uat_status, UatStatus::Unverified); // Opted out, still unverified.

        // Verify History entries.
        let prd_content = std::fs::read_to_string(&prd_file).unwrap();

        // Opt-out History entries ARE automatically appended by append_opt_out_history().
        assert!(
            prd_content.contains("uat-002 Opt-Out"),
            "Opt-out History entries should be automatically appended"
        );
        assert!(prd_content.contains("Requires manual testing"));

        // Successful verification History entries are NOT automatically appended.
        // The prompt instructs the runner (AI agent) to manually append them.
        // This is by design: the runner has context to write meaningful History entries.
        assert!(
            !prd_content.contains("uat-001 Verification"),
            "Successful verification History entries are manually appended by the runner, not by the loop"
        );
    }

    #[test]
    fn test_constitution_violation_logging() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create a run_task.md prompt that includes constitution placeholder.
        let prompt_content = r#"Execute task {{next_task_id}} from {{prd_path}}

{{#if constitution}}
# Constitution

{{constitution}}

**Note**: When appending History entries, include a "Constitution Compliance" section if any rules were violated.
{{/if}}"#;
        std::fs::write(prompts_dir.join("run_task.md"), prompt_content).unwrap();

        // Create a constitution file with example rules.
        let constitution_content = r#"# Constitution

## Purpose
Project governance and best practices.

## Rules
1. **Acceptance tests must be codified** — One-off acceptance tests are unacceptable.
2. **Use semantic versioning** — All releases must follow semver.
"#;
        std::fs::write(
            root.join(".mr").join("constitution.md"),
            constitution_content,
        )
        .unwrap();

        // Create a test PRD with a task.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            tasks: Some(vec![Task {
                id: "T-001".to_string(),
                title: "Implement feature".to_string(),
                priority: 1,
                status: TaskStatus::Todo,
                notes: None,
            }]),
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
        let prd_path = root.join(".mr/prds/PRD-0001-test.md");

        // Build the prompt for the task.
        let prompt = build_prompt(&root, &prd, &prd_path, "T-001");

        // Verify the constitution is included in the prompt.
        assert!(
            prompt.contains("Acceptance tests must be codified"),
            "Constitution content should be included in task execution prompt"
        );
        assert!(
            prompt.contains("Use semantic versioning"),
            "Constitution content should be included in task execution prompt"
        );

        // Verify the prompt instructs the runner to log violations.
        assert!(
            prompt.contains("Constitution Compliance"),
            "Prompt should instruct runner to log constitution violations"
        );
    }
}
