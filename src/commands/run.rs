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

use crate::config::load_constitution;
use crate::prd::{AcceptanceTest, Prd, PrdStatus, TaskStatus, scan_prds};
use crate::prompt::{
    PlaceholderContext, PromptKind, expand_placeholders, load_prompt_with_fallback,
};
use crate::runner::{Runner, RunnerOutput, TokenUsageInfo};
use crate::util::spinner::start_spinner;

/// Configuration for `mr run`.
#[derive(Debug)]
pub struct RunConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// Explicit PRD ID to run (e.g., "PRD-0001"). If None, asks runner to pick.
    pub prd_id: Option<&'a str>,

    /// Whether to stream runner output in real-time.
    pub stream: bool,

    /// Whether to instruct the agent NOT to commit changes.
    /// When true, prompts will say "Do NOT commit" instead of commit instructions.
    pub no_commit: bool,

    /// Whether the agent is allowed to add new tasks during execution.
    /// When false, the add-task instructions are omitted from prompts.
    pub allow_add_task: bool,
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
        usage: Option<TokenUsageInfo>,
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

    /// Whether the loop was stopped due to `max_iterations` limit.
    pub hit_max_iterations: bool,

    /// Remaining unverified UATs after the loop.
    pub remaining_unverified: usize,

    /// Whether the loop broke early because new incomplete tasks were detected.
    pub has_new_tasks: bool,
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
pub fn pick_prd_via_runner(
    root: &Path,
    runner: &dyn Runner,
    stream: bool,
) -> Result<Option<String>> {
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

    // Print command info before spinner (only when not streaming).
    if !stream && let Some(cmd_display) = runner.format_command_display(&prompt, root) {
        println!("\n🔧 Executing: {cmd_display}");
    }

    // Start spinner when not streaming.
    let spinner = start_spinner(!stream, "Selecting PRD...");

    // Invoke the runner.
    let output: RunnerOutput = if stream {
        let mut stdout = std::io::stdout();
        runner.execute_streaming(&prompt, root, &mut stdout)?
    } else {
        runner.execute(&prompt, root)?
    };

    // Clear spinner.
    spinner.finish_and_clear();

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
    if trimmed.starts_with("PRD-") && trimmed.len() == 8 {
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

/// Finds a PRD by ID, returning an error if not found.
fn require_prd_by_id(root: &Path, prd_id: &str) -> Result<(String, Prd, PathBuf)> {
    find_prd_by_id(root, prd_id)?.ok_or_else(|| {
        anyhow::anyhow!(
            "PRD not found: {prd_id}.\n  Suggestion: Run `mr status` to list available PRDs.",
        )
    })
}

/// Builds the prompt for the runner.
fn build_prompt(
    root: &Path,
    prd: &Prd,
    prd_path: &Path,
    task_id: &str,
    no_commit: bool,
    allow_add_task: bool,
) -> String {
    let prompt_template = load_prompt_with_fallback(root, PromptKind::RunTask);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("prd_path", prd_path.display().to_string());
    ctx.insert("prd_id", prd.id());
    ctx.insert("prd_title", prd.title());
    ctx.insert("next_task_id", task_id);

    // Add commit variable (inverted: commit = !no_commit).
    // When commit is true, prompts include commit instructions.
    // When commit is false, prompts say "Do NOT commit".
    ctx.insert("commit", !no_commit);

    // Add allow_add_task placeholder for conditional prompt sections.
    ctx.insert("allow_add_task", allow_add_task);

    // Add task details if available.
    if let Some(tasks) = prd.tasks()
        && let Some(task) = tasks.iter().find(|t| t.id == task_id)
    {
        ctx.insert("task_title", task.title.as_str());
        ctx.insert("task_priority", task.priority.to_string());

        if let Some(notes) = &task.notes {
            ctx.insert("task_notes", notes.as_str());
        }
    }

    // Load constitution if available.
    if let Ok(Some(constitution)) = load_constitution(root) {
        ctx.insert("constitution", constitution);
    }

    // Load skills manifest if available and non-default.
    let skills_path = root.join(".mr/skills/SKILLS.md");
    if let Ok(content) = std::fs::read_to_string(&skills_path) {
        let trimmed = content.trim();
        let default_trimmed = crate::commands::init::SKILLS_TEMPLATE.trim();
        if !trimmed.is_empty() && trimmed != default_trimmed {
            ctx.insert("skills_manifest", content);
        }
    }

    expand_placeholders(&prompt_template, &ctx)
}

/// Runs the next task from the active PRD.
///
/// The PRD ID must be provided in the config. If you need to pick a PRD,
/// use [`pick_prd_via_runner`] first.
///
/// # Arguments
///
/// * `config` - Configuration for the run (must include `prd_id`)
/// * `runner` - The runner to use for task execution
///
/// # Returns
///
/// A [`RunResult`] describing what happened, or an error.
pub fn run_task(config: &RunConfig, runner: &dyn Runner) -> Result<RunResult> {
    // PRD ID must be provided.
    let prd_id = config.prd_id.ok_or_else(|| {
        anyhow::anyhow!("PRD ID must be provided to run_task. Use pick_prd_via_runner first.")
    })?;

    // Find the PRD.
    let (_filename, prd, prd_path) = find_prd_by_id(config.root, prd_id)?.ok_or_else(|| {
        anyhow::anyhow!(
            "PRD not found: {prd_id}.\n  Suggestion: Run `mr status` to list available PRDs."
        )
    })?;

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
    let prompt = build_prompt(
        config.root,
        &prd,
        &prd_path,
        &task_id,
        config.no_commit,
        config.allow_add_task,
    );

    tracing::info!(
        prompt_len = prompt.len(),
        runner = %runner.name(),
        prd_id = %prd_id,
        task_id = %task_id,
        stream = config.stream,
        "Invoking runner to execute task"
    );

    // Print command info before spinner (only when not streaming).
    if !config.stream
        && let Some(cmd_display) = runner.format_command_display(&prompt, config.root)
    {
        println!("\n🔧 Executing: {cmd_display}");
    }

    // Start spinner when not streaming (streaming already provides visual feedback).
    let spinner = start_spinner(
        !config.stream,
        format!("Running task {current_task_num}/{total_tasks}..."),
    );

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

    // Clear spinner before displaying output.
    spinner.finish_and_clear();

    // Validate PRD frontmatter after agent edits.
    tracing::debug!(prd_path = %prd_path.display(), "Validating PRD frontmatter after agent edit");
    super::validate::validate_prd_frontmatter(&prd_path);

    let output_summary = summarize_output(config.stream, output.text);

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

/// Summarize runner output, truncating if too long. Returns a passthrough message if output was streamed.
fn summarize_output(streamed: bool, text: String) -> String {
    if streamed {
        return "(output was streamed above)".to_string();
    }

    if text.chars().count() > 500 {
        let last_500: String = text
            .chars()
            .rev()
            .take(500)
            .collect::<String>()
            .chars()
            .rev()
            .collect();
        format!("... (truncated)\n{last_500}")
    } else {
        text
    }
}

/// Default max iterations for UAT verification loop if not configured.
const DEFAULT_MAX_UAT_ITERATIONS: u32 = 10;

/// Builds the prompt for UAT verification.
fn build_uat_verify_prompt(
    root: &Path,
    prd: &Prd,
    prd_path: &Path,
    uat: &AcceptanceTest,
    allow_skip_uat: bool,
    allow_add_task: bool,
) -> String {
    let prompt_template = load_prompt_with_fallback(root, PromptKind::RunUatVerify);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("prd_path", prd_path.display().to_string());
    ctx.insert("prd_id", prd.id());
    ctx.insert("uat_id", uat.id.as_str());
    ctx.insert("uat_name", uat.name.as_str());
    ctx.insert("uat_command", uat.command.as_str());

    // Add allow_skip_uat and allow_add_task placeholders for conditional prompt sections.
    ctx.insert("allow_skip_uat", allow_skip_uat);
    ctx.insert("allow_add_task", allow_add_task);

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

/// Configuration for UAT verification loop.
#[derive(Debug)]
pub struct UatVerificationConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// The PRD ID to verify UATs for.
    pub prd_id: &'a str,

    /// Whether to stream runner output in real-time.
    pub stream: bool,

    /// Maximum number of iterations (None = use default).
    pub max_iterations: Option<u32>,

    /// Whether the agent is allowed to skip UATs during verification.
    pub allow_skip_uat: bool,

    /// Whether the agent is allowed to add new tasks during verification.
    pub allow_add_task: bool,
}

/// Outcome of processing a single UAT verification iteration.
enum UatIterationOutcome {
    /// UAT was successfully verified.
    Verified,
    /// UAT was opted out by the agent.
    OptedOut,
    /// Runner executed but outcome unclear (neither opt-out nor success).
    Inconclusive,
}

/// Executes a single UAT verification step.
///
/// Builds the prompt, invokes the runner, and handles output display.
fn execute_uat_verification(
    config: &UatVerificationConfig,
    runner: &dyn Runner,
    prd: &Prd,
    prd_path: &Path,
    uat: &AcceptanceTest,
    current_uat_num: usize,
    all_uats: usize,
) -> Result<RunnerOutput> {
    let prompt = build_uat_verify_prompt(
        config.root,
        prd,
        prd_path,
        uat,
        config.allow_skip_uat,
        config.allow_add_task,
    );

    // Print command info before spinner (only when not streaming).
    if !config.stream
        && let Some(cmd_display) = runner.format_command_display(&prompt, config.root)
    {
        println!("\n🔧 Executing: {cmd_display}");
    }

    let spinner = start_spinner(
        !config.stream,
        format!("Verifying UAT {current_uat_num}/{all_uats}..."),
    );

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

    spinner.finish_and_clear();

    // Display output if not already streamed.
    if !config.stream {
        if output.text.chars().count() > 500 {
            let last_500: String = output
                .text
                .chars()
                .rev()
                .take(500)
                .collect::<String>()
                .chars()
                .rev()
                .collect();
            println!("... (truncated)\n{last_500}");
        } else {
            println!("{}", output.text);
        }
    }

    // Validate PRD frontmatter after agent edits.
    tracing::debug!(prd_path = %prd_path.display(), "Validating PRD frontmatter after UAT verification");
    super::validate::validate_prd_frontmatter(prd_path);

    Ok(output)
}

/// Processes the runner output and determines the outcome.
fn process_uat_verification_response(
    config: &UatVerificationConfig,
    output: &RunnerOutput,
    uat_id: &str,
) -> Result<UatIterationOutcome> {
    if let Some(explanation) = parse_opt_out(&output.text) {
        tracing::info!(
            prd_id = %config.prd_id,
            uat_id = %uat_id,
            explanation = %explanation,
            "UAT verification opted out (agent should have updated PRD)"
        );
        return Ok(UatIterationOutcome::OptedOut);
    }

    if output.success {
        // Check if agent marked UAT as verified.
        let (_f, refreshed_prd, _p) =
            find_prd_by_id(config.root, config.prd_id)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "PRD not found: {}.\n  Suggestion: Run `mr status` to list available PRDs.",
                    config.prd_id
                )
            })?;

        let still_unverified = refreshed_prd
            .unverified_uats()
            .iter()
            .any(|u| u.id == uat_id);

        if still_unverified {
            tracing::warn!(
                prd_id = %config.prd_id,
                uat_id = %uat_id,
                "UAT verification succeeded but agent did not update status in PRD"
            );
        } else {
            tracing::info!(
                prd_id = %config.prd_id,
                uat_id = %uat_id,
                "UAT verified (status updated by agent)"
            );
        }
        return Ok(UatIterationOutcome::Verified);
    }

    Ok(UatIterationOutcome::Inconclusive)
}

/// Runs the UAT verification loop.
///
/// Iterates over unverified UATs, invoking the runner for each one.
/// Respects `max_iterations` limit and handles `OPT-OUT` responses.
///
/// # Arguments
///
/// * `config` - Configuration for the verification loop
/// * `runner` - The runner to use for verification
///
/// # Returns
///
/// A [`UatVerificationLoopResult`] describing what happened.
///
/// # State Machine Flow
///
/// This function implements a UAT verification state machine with the following states:
///
/// 1. **Load**: Read PRD and get `max_iterations` config
/// 2. **Loop Start**: Reload PRD to get current unverified UATs (may change between iterations)
/// 3. **Check Completion**: If no unverified UATs remain → Success exit
/// 4. **Check Iteration Limit**: If `max_iterations` reached → Early exit with remaining UATs
/// 5. **Pick UAT**: Select the first unverified UAT to process
/// 6. **Execute Runner**: Invoke runner with UAT verification prompt
/// 7. **Parse Response**: Check for `OPT-OUT` signal or success
/// 8. **Update State**:
///    - `OPT-OUT`: Append history entry, continue to next UAT
///    - Success: Update UAT status to verified if not already done by runner
/// 9. **Loop**: Return to step 2
///
/// The loop ensures eventual termination via `max_iterations` and allows the runner to modify
/// the PRD (e.g., marking UATs as verified) between iterations. This state machine handles
/// three exit conditions: all verified, iteration limit, or error.
pub fn run_uat_verification_loop(
    config: &UatVerificationConfig,
    runner: &dyn Runner,
) -> Result<UatVerificationLoopResult> {
    let (_filename, prd, prd_path) = require_prd_by_id(config.root, config.prd_id)?;

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

    tracing::info!(prd_id = %config.prd_id, max_iterations, "Starting UAT verification loop");

    loop {
        let (_filename, current_prd, current_prd_path) =
            require_prd_by_id(config.root, config.prd_id)?;

        let unverified = current_prd.unverified_uats();

        if unverified.is_empty() {
            tracing::info!(prd_id = %config.prd_id, verified_count, opted_out_count, "All UATs verified or opted out");
            break;
        }

        if iterations >= max_iterations as usize {
            tracing::info!(prd_id = %config.prd_id, iterations, max_iterations, remaining = unverified.len(), "Hit max iterations limit");
            return Ok(UatVerificationLoopResult {
                prd_id: config.prd_id.to_string(),
                prd_path: prd_path.clone(),
                verified_count,
                opted_out_count,
                iterations,
                hit_max_iterations: true,
                remaining_unverified: unverified.len(),
                has_new_tasks: false,
            });
        }

        let uat = unverified[0];
        let all_uats = current_prd
            .frontmatter
            .acceptance_tests
            .as_ref()
            .map_or(0, Vec::len);
        let current_uat_num = all_uats - unverified.len() + 1;

        tracing::info!(
            prd_id = %config.prd_id, uat_id = %uat.id, uat_name = %uat.name,
            current_uat = current_uat_num, total_uats = all_uats,
            iteration = iterations + 1, max_iterations, "Verifying UAT"
        );

        let output = execute_uat_verification(
            config,
            runner,
            &current_prd,
            &current_prd_path,
            uat,
            current_uat_num,
            all_uats,
        )?;

        iterations += 1;

        match process_uat_verification_response(config, &output, &uat.id)? {
            UatIterationOutcome::OptedOut => opted_out_count += 1,
            UatIterationOutcome::Verified => verified_count += 1,
            UatIterationOutcome::Inconclusive => {}
        }

        // Check if the agent added new incomplete tasks during this iteration.
        let (_filename, post_iteration_prd, _path) = require_prd_by_id(config.root, config.prd_id)?;

        if post_iteration_prd.has_incomplete_tasks() {
            tracing::info!(
                prd_id = %config.prd_id,
                "New incomplete tasks detected during UAT verification; breaking early"
            );

            return Ok(UatVerificationLoopResult {
                prd_id: config.prd_id.to_string(),
                prd_path,
                verified_count,
                opted_out_count,
                iterations,
                hit_max_iterations: false,
                remaining_unverified: post_iteration_prd.unverified_uats().len(),
                has_new_tasks: true,
            });
        }
    }

    let (_filename, final_prd, _final_path) = require_prd_by_id(config.root, config.prd_id)?;

    Ok(UatVerificationLoopResult {
        prd_id: config.prd_id.to_string(),
        prd_path,
        verified_count,
        opted_out_count,
        iterations,
        hit_max_iterations: false,
        remaining_unverified: final_prd.unverified_uats().len(),
        has_new_tasks: false,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(clippy::similar_names)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Mutex;

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
            title: format!("Test PRD {id}"),
            status,
            tasks: if tasks.is_empty() { None } else { Some(tasks) },
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n".to_string());
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
    fn test_extract_prd_id_with_trailing_content() {
        // Regression: extract_prd_id used to return "PRD-0001-slug" because the
        // fast path checked `len() >= 8` instead of `== 8`, accepting trailing chars.
        assert_eq!(
            extract_prd_id("PRD-0001-build-something"),
            Some("PRD-0001".to_string())
        );
        assert_eq!(extract_prd_id("PRD-0002."), Some("PRD-0002".to_string()));
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

        // Runner picks PRD, then executes task.
        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("PRD-0001"),
            crate::runner::RunnerOutput::success("Task executed successfully."),
        ]);

        // First, pick the PRD.
        let picked_prd = pick_prd_via_runner(&root, &runner, false)
            .unwrap()
            .expect("Should pick a PRD");

        assert_eq!(picked_prd, "PRD-0001");

        // Then run task with the picked PRD.
        let config = RunConfig {
            root: &root,
            prd_id: Some(&picked_prd),
            stream: false,
            no_commit: false,
            allow_add_task: true,
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
            no_commit: false,
            allow_add_task: true,
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
    fn test_pick_prd_selects_active_with_tasks() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);
        let prds_dir = root.join(".mr").join("prds");

        // Create pick_prd.md prompt.
        std::fs::write(root.join(".mr/prompts/pick_prd.md"), "Pick the next PRD").unwrap();

        // Create multiple PRDs with different states.
        create_test_prd(
            &prds_dir,
            "PRD-0001",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Done)], // All done.
        );
        create_test_prd(
            &prds_dir,
            "PRD-0002",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)], // Has incomplete task.
        );
        create_test_prd(
            &prds_dir,
            "PRD-0003",
            PrdStatus::Draft,
            vec![make_task("T-001", 1, TaskStatus::Todo)], // Draft with task.
        );

        // Runner picks PRD-0002.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success("PRD-0002")]);

        let picked = pick_prd_via_runner(&root, &runner, false)
            .unwrap()
            .expect("Should pick a PRD");

        assert_eq!(picked, "PRD-0002");
    }

    #[test]
    fn test_pick_prd_no_active_prd() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        // Create pick_prd.md prompt.
        std::fs::write(root.join(".mr/prompts/pick_prd.md"), "Pick the next PRD").unwrap();

        // Runner returns NONE (no PRDs available).
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success("NONE")]);

        let result = pick_prd_via_runner(&root, &runner, false);

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), None);
    }

    #[test]
    fn test_run_task_requires_prd_id() {
        let temp = TempDir::new().unwrap();
        let root = setup_test_repo(&temp);

        let runner = MockRunner::new(vec![]);

        let config = RunConfig {
            root: &root,
            prd_id: None,
            stream: false,
            no_commit: false,
            allow_add_task: true,
        };

        let result = run_task(&config, &runner);

        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("PRD ID must be provided")
        );
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

        let prompt = build_prompt(&root, &prd, &prd_path, "T-001", false, true);

        assert!(prompt.contains("T-001"));
        assert!(prompt.contains("PRD-0001.md"));
    }

    #[test]
    fn test_build_prompt_commit_true() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create a prompt template with commit conditional.
        std::fs::write(
            prompts_dir.join("run_task.md"),
            r"Execute task {{next_task_id}} from {{prd_path}}
{{#if commit}}
9. **Commit your work** with a descriptive commit message.
{{else}}
9. **Do NOT commit your work** — leave changes staged or unstaged for manual review.
{{/if}}",
        )
        .unwrap();

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
        let prd_path = root.join(".mr/prds/PRD-0001.md");

        // no_commit=false means commit=true.
        let prompt = build_prompt(&root, &prd, &prd_path, "T-001", false, true);

        // When commit=true, prompt should include commit instructions.
        assert!(
            prompt.contains("Commit your work"),
            "Prompt should include commit instructions when no_commit=false"
        );
        // And should NOT contain "Do NOT commit".
        assert!(
            !prompt.contains("Do NOT commit"),
            "Prompt should not contain 'Do NOT commit' when no_commit=false"
        );
    }

    #[test]
    fn test_build_prompt_commit_false() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create a prompt template with commit conditional.
        std::fs::write(
            prompts_dir.join("run_task.md"),
            r"Execute task {{next_task_id}} from {{prd_path}}
{{#if commit}}
9. **Commit your work** with a descriptive commit message.
{{else}}
9. **Do NOT commit your work** — leave changes staged or unstaged for manual review.
{{/if}}",
        )
        .unwrap();

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
        let prd_path = root.join(".mr/prds/PRD-0001.md");

        // no_commit=true means commit=false.
        let prompt = build_prompt(&root, &prd, &prd_path, "T-001", true, true);

        // When commit=false, prompt should include "Do NOT commit" instructions.
        assert!(
            prompt.contains("Do NOT commit"),
            "Prompt should contain 'Do NOT commit' when no_commit=true"
        );
        // And should NOT contain commit instructions.
        assert!(
            !prompt.contains("Commit your work"),
            "Prompt should not contain 'Commit your work' when no_commit=true"
        );
    }

    #[test]
    fn test_build_prompt_allow_add_task_true() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create a prompt template with allow_add_task conditional.
        std::fs::write(
            prompts_dir.join("run_task.md"),
            r"Execute task {{next_task_id}}
{{#if allow_add_task}}
### Adding New Tasks (Dynamic Task Addition)
You may add new tasks.
{{/if}}",
        )
        .unwrap();

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
        let prd_path = root.join(".mr/prds/PRD-0001.md");

        let prompt = build_prompt(&root, &prd, &prd_path, "T-001", false, true);

        assert!(
            prompt.contains("Adding New Tasks"),
            "Prompt should include add-task section when allow_add_task=true"
        );
    }

    #[test]
    fn test_build_prompt_allow_add_task_false() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create a prompt template with allow_add_task conditional.
        std::fs::write(
            prompts_dir.join("run_task.md"),
            r"Execute task {{next_task_id}}
{{#if allow_add_task}}
### Adding New Tasks (Dynamic Task Addition)
You may add new tasks.
{{/if}}",
        )
        .unwrap();

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
        let prd_path = root.join(".mr/prds/PRD-0001.md");

        let prompt = build_prompt(&root, &prd, &prd_path, "T-001", false, false);

        assert!(
            !prompt.contains("Adding New Tasks"),
            "Prompt should not include add-task section when allow_add_task=false"
        );
    }

    #[test]
    fn test_run_task_with_no_commit_sends_correct_prompt() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create a run_task.md prompt with commit conditional.
        std::fs::write(
            prompts_dir.join("run_task.md"),
            r"Execute task {{next_task_id}} from {{prd_path}}
{{#if commit}}
**Commit your work** with a descriptive commit message.
{{else}}
**Do NOT commit your work** — leave changes staged or unstaged for manual review.
{{/if}}",
        )
        .unwrap();

        // Create a PRD with a task.
        create_test_prd(
            &prds_dir,
            "PRD-0001",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
        );

        // Create a mock runner that records prompts.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success("Done.")]);

        // Run the task with no_commit=true.
        let config = RunConfig {
            root: &root,
            prd_id: Some("PRD-0001"),
            stream: false,
            no_commit: true,
            allow_add_task: true,
        };

        let result = run_task(&config, &runner).unwrap();

        // Verify task was executed.
        match result {
            RunResult::TaskExecuted {
                prd_id, task_id, ..
            } => {
                assert_eq!(prd_id, "PRD-0001");
                assert_eq!(task_id, "T-001");
            }
            _ => panic!("Expected TaskExecuted result"),
        }

        // Verify the prompt sent to the runner contains "Do NOT commit".
        let prompts = runner.recorded_prompts();
        assert_eq!(prompts.len(), 1);
        assert!(
            prompts[0].contains("Do NOT commit"),
            "Prompt should contain 'Do NOT commit' when no_commit=true. Got: {}",
            prompts[0]
        );
        assert!(
            !prompts[0].contains("Commit your work"),
            "Prompt should not contain 'Commit your work' when no_commit=true"
        );
    }

    #[test]
    fn test_run_task_without_no_commit_sends_commit_instructions() {
        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        // Create a run_task.md prompt with commit conditional.
        std::fs::write(
            prompts_dir.join("run_task.md"),
            r"Execute task {{next_task_id}} from {{prd_path}}
{{#if commit}}
**Commit your work** with a descriptive commit message.
{{else}}
**Do NOT commit your work** — leave changes staged or unstaged for manual review.
{{/if}}",
        )
        .unwrap();

        // Create a PRD with a task.
        create_test_prd(
            &prds_dir,
            "PRD-0001",
            PrdStatus::Active,
            vec![make_task("T-001", 1, TaskStatus::Todo)],
        );

        // Create a mock runner that records prompts.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success("Done.")]);

        // Run the task with no_commit=false (default behavior).
        let config = RunConfig {
            root: &root,
            prd_id: Some("PRD-0001"),
            stream: false,
            no_commit: false,
            allow_add_task: true,
        };

        let result = run_task(&config, &runner).unwrap();

        // Verify task was executed.
        match result {
            RunResult::TaskExecuted {
                prd_id, task_id, ..
            } => {
                assert_eq!(prd_id, "PRD-0001");
                assert_eq!(task_id, "T-001");
            }
            _ => panic!("Expected TaskExecuted result"),
        }

        // Verify the prompt sent to the runner contains commit instructions.
        let prompts = runner.recorded_prompts();
        assert_eq!(prompts.len(), 1);
        assert!(
            prompts[0].contains("Commit your work"),
            "Prompt should contain 'Commit your work' when no_commit=false. Got: {}",
            prompts[0]
        );
        assert!(
            !prompts[0].contains("Do NOT commit"),
            "Prompt should not contain 'Do NOT commit' when no_commit=false"
        );
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
            no_commit: false,
            allow_add_task: true,
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
            no_commit: false,
            allow_add_task: true,
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
            no_commit: false,
            allow_add_task: true,
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

        let prompt = build_uat_verify_prompt(&root, &prd, &prd_path, &uat, true, true);

        assert!(prompt.contains("uat-001"));
        assert!(prompt.contains("Test 1"));
        assert!(prompt.contains("PRD-0001"));
        assert!(prompt.contains("cargo test"));
    }

    #[test]
    fn test_build_uat_verify_prompt_allow_skip_uat_true() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("run_uat_verify.md"),
            r"Verify UAT {{uat_id}}
{{#if allow_skip_uat}}
### Option D: Mark as Skipped
You may skip this UAT.
{{/if}}
{{#if allow_add_task}}
### Option E: Add a Task
You may add a task.
{{/if}}",
        )
        .unwrap();

        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
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

        let prompt = build_uat_verify_prompt(&root, &prd, &prd_path, &uat, true, true);

        assert!(
            prompt.contains("Option D: Mark as Skipped"),
            "Prompt should include skip section when allow_skip_uat=true"
        );
        assert!(
            prompt.contains("Option E: Add a Task"),
            "Prompt should include add-task section when allow_add_task=true"
        );
    }

    #[test]
    fn test_build_uat_verify_prompt_allow_skip_uat_false() {
        use crate::prd::types::{AcceptanceTest, UatStatus};

        let temp = TempDir::new().unwrap();
        let root = temp.path().to_path_buf();
        let prds_dir = root.join(".mr").join("prds");
        let prompts_dir = root.join(".mr").join("prompts");

        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("run_uat_verify.md"),
            r"Verify UAT {{uat_id}}
{{#if allow_skip_uat}}
### Option D: Mark as Skipped
You may skip this UAT.
{{/if}}
{{#if allow_add_task}}
### Option E: Add a Task
You may add a task.
{{/if}}",
        )
        .unwrap();

        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
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

        let prompt = build_uat_verify_prompt(&root, &prd, &prd_path, &uat, false, false);

        assert!(
            !prompt.contains("Option D: Mark as Skipped"),
            "Prompt should not include skip section when allow_skip_uat=false"
        );
        assert!(
            !prompt.contains("Option E: Add a Task"),
            "Prompt should not include add-task section when allow_add_task=false"
        );
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
            stream: false,
            max_iterations: Some(5),
            allow_skip_uat: true,
            allow_add_task: true,
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
            stream: false,
            max_iterations: Some(1), // Only 1 iteration allowed.
            allow_skip_uat: true,
            allow_add_task: true,
        };

        let result = run_uat_verification_loop(&config, &runner).unwrap();

        assert_eq!(result.prd_id, "PRD-0001");
        assert_eq!(result.opted_out_count, 1);
        assert_eq!(result.iterations, 1);
        assert!(result.hit_max_iterations);
        assert_eq!(result.remaining_unverified, 1);

        // Note: History entries are now the agent's responsibility.
        // Rust code no longer automatically appends opt-out history.
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

        // Runner always succeeds. The agent should update UAT status, but we're not testing that here.
        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Verified test 1"),
            crate::runner::RunnerOutput::success("Verified test 2"),
        ]);

        let config = UatVerificationConfig {
            root: &root,
            prd_id: "PRD-0001",
            stream: false,
            max_iterations: Some(2),
            allow_skip_uat: true,
            allow_add_task: true,
        };

        let result = run_uat_verification_loop(&config, &runner).unwrap();

        assert_eq!(result.prd_id, "PRD-0001");
        assert_eq!(result.iterations, 2);
        assert!(result.hit_max_iterations); // Hit limit because UATs remain unverified (agent didn't update).
        assert_eq!(result.verified_count, 2); // Both reported as verified by runner.

        // Note: UAT status in PRD is not updated by Rust code anymore.
        // The agent is responsible for updating UAT status when it verifies them.
        // This test verifies the loop counts and iteration behavior correctly.
    }

    /// Integration test for the full UAT verification flow:
    /// 1. `run_task()` returns `NeedsUatVerification` when all tasks done but UATs unverified
    /// 2. `run_uat_verification_loop()` processes the unverified UATs
    /// 3. Loop respects `max_iterations` and correctly updates UAT status
    #[test]
    #[allow(clippy::too_many_lines)]
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
            no_commit: false,
            allow_add_task: true,
        };

        let run_result = run_task(&run_config, &run_runner).unwrap();

        let (prd_id, _prd_path) = match run_result {
            RunResult::NeedsUatVerification {
                prd_id,
                prd_path,
                unverified_count,
            } => {
                assert_eq!(prd_id, "PRD-0001");
                assert_eq!(unverified_count, 3);
                (prd_id, prd_path)
            }
            other => panic!("Expected NeedsUatVerification, got {other:?}"),
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
            stream: false,
            max_iterations: Some(2),
            allow_skip_uat: true,
            allow_add_task: true,
        };

        let uat_result = run_uat_verification_loop(&uat_config, &uat_runner).unwrap();

        // Verify loop behavior.
        assert_eq!(uat_result.prd_id, "PRD-0001");
        assert_eq!(uat_result.verified_count, 1); // First UAT verified.
        assert_eq!(uat_result.opted_out_count, 1); // Second UAT opted out.
        assert_eq!(uat_result.iterations, 2); // Ran for 2 iterations.
        assert!(uat_result.hit_max_iterations); // Hit the limit.

        // Note: Remaining unverified count reflects that agent didn't update PRD
        // In real usage, agent would update UAT status when verifying
        assert_eq!(uat_result.remaining_unverified, 3); // All 3 still unverified (agent didn't update PRD).

        // Step 3: Verify behavior - UAT status is NOT updated by Rust code anymore.
        let updated_prd = crate::prd::parse_prd_file(&prd_file).unwrap();
        let uats = updated_prd.frontmatter.acceptance_tests.unwrap();

        let uat1 = uats.iter().find(|u| u.id == "uat-001").unwrap();
        let uat2 = uats.iter().find(|u| u.id == "uat-002").unwrap();
        let uat3 = uats.iter().find(|u| u.id == "uat-003").unwrap();

        // All remain unverified because the agent is responsible for updating status
        assert_eq!(uat1.uat_status, UatStatus::Unverified);
        assert_eq!(uat2.uat_status, UatStatus::Unverified);
        assert_eq!(uat3.uat_status, UatStatus::Unverified);

        // Note: History entries for opt-out and verification are now the agent's responsibility.
        // The agent should append them when it updates the PRD, but Rust code no longer does this automatically.
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
            stream: false,
            max_iterations: Some(2), // Stop after 2 iterations to avoid retrying opted-out UAT.
            allow_skip_uat: true,
            allow_add_task: true,
        };

        let uat_result = run_uat_verification_loop(&uat_config, &uat_runner).unwrap();

        assert_eq!(uat_result.verified_count, 1);
        assert_eq!(uat_result.opted_out_count, 1);
        assert_eq!(uat_result.iterations, 2);
        assert!(uat_result.hit_max_iterations); // Loop stopped due to max_iterations, not because all UATs verified.
        assert_eq!(uat_result.remaining_unverified, 2); // Both UATs still unverified (agent didn't update PRD).

        // Verify PRD frontmatter - UAT status is not automatically updated anymore.
        let updated_prd = crate::prd::parse_prd_file(&prd_file).unwrap();
        let uats = updated_prd.frontmatter.acceptance_tests.unwrap();
        let uat1 = uats.iter().find(|u| u.id == "uat-001").unwrap();
        let uat2 = uats.iter().find(|u| u.id == "uat-002").unwrap();
        assert_eq!(uat1.uat_status, UatStatus::Unverified); // Agent should update, but Rust doesn't.
        assert_eq!(uat2.uat_status, UatStatus::Unverified); // Opted out, still unverified.

        // Note: History entries are now the agent's responsibility.
        // The agent should append opt-out and verification History entries when appropriate.
        // Rust code no longer automatically appends these entries.
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
        let constitution_content = r"# Constitution

## Purpose
Project governance and best practices.

## Rules
1. **Acceptance tests must be codified** — One-off acceptance tests are unacceptable.
2. **Use semantic versioning** — All releases must follow semver.
";
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
        let prompt = build_prompt(&root, &prd, &prd_path, "T-001", false, true);

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

    // ─── PRD-0037 T-004: Tests for UAT loop early-break and outer loop re-entry ───

    /// A runner that modifies the PRD file as a side effect on the first `execute()` call,
    /// simulating an agent adding a new task during UAT verification.
    struct SideEffectRunner {
        prd_file: PathBuf,
        updated_content: Mutex<Option<String>>,
        responses: Mutex<VecDeque<RunnerOutput>>,
    }

    impl SideEffectRunner {
        fn new(prd_file: PathBuf, updated_content: String, responses: Vec<RunnerOutput>) -> Self {
            Self {
                prd_file,
                updated_content: Mutex::new(Some(updated_content)),
                responses: Mutex::new(responses.into()),
            }
        }
    }

    impl Runner for SideEffectRunner {
        fn name(&self) -> &'static str {
            "side-effect-mock"
        }

        fn execute(
            &self,
            _prompt: &str,
            _working_dir: &std::path::Path,
        ) -> Result<RunnerOutput, crate::runner::RunnerError> {
            // Apply the side effect (modify PRD) on the first call only.
            if let Some(content) = self.updated_content.lock().unwrap().take() {
                std::fs::write(&self.prd_file, content).unwrap();
            }

            let response = self
                .responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| RunnerOutput::success("Mock response"));

            Ok(response)
        }
    }

    #[test]
    fn test_uat_loop_breaks_early_when_new_tasks_detected() {
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

        // Create a PRD with all tasks done and unverified UATs.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Early Break Test PRD".to_string(),
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

        // Build the updated PRD content: same PRD but with a new incomplete task added.
        let mut updated_frontmatter = prd.frontmatter.clone();
        updated_frontmatter.tasks.as_mut().unwrap().push(Task {
            id: "T-002".to_string(),
            title: "Newly added task".to_string(),
            priority: 2,
            status: TaskStatus::Todo,
            notes: None,
        });
        let updated_prd = Prd::new(updated_frontmatter, "# Body\n".to_string());
        let updated_content = crate::prd::serialize_prd(&updated_prd).unwrap();

        // Use SideEffectRunner: on first execute(), it writes the updated PRD (adding new task).
        let runner = SideEffectRunner::new(
            prd_file.clone(),
            updated_content,
            vec![RunnerOutput::success("Verified test 1")],
        );

        let config = UatVerificationConfig {
            root: &root,
            prd_id: "PRD-0001",
            stream: false,
            max_iterations: Some(5),
            allow_skip_uat: true,
            allow_add_task: true,
        };

        let result = run_uat_verification_loop(&config, &runner).unwrap();

        // Should have broken early after 1 iteration with has_new_tasks = true.
        assert!(result.has_new_tasks, "Should detect new incomplete tasks");
        assert_eq!(result.iterations, 1, "Should break after first iteration");
        assert!(!result.hit_max_iterations, "Should not hit max iterations");
        assert_eq!(result.verified_count, 1);
    }

    #[test]
    fn test_uat_loop_no_new_tasks_means_has_new_tasks_false() {
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

        // All tasks done, one unverified UAT.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Convergence Test PRD".to_string(),
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
        std::fs::write(&prd_file, content).unwrap();

        // Runner does NOT modify the PRD — no side effects.
        let runner = MockRunner::new(vec![RunnerOutput::success("Verified test 1")]);

        let config = UatVerificationConfig {
            root: &root,
            prd_id: "PRD-0001",
            stream: false,
            max_iterations: Some(1),
            allow_skip_uat: true,
            allow_add_task: true,
        };

        let result = run_uat_verification_loop(&config, &runner).unwrap();

        // No new tasks added, so has_new_tasks should be false (even though loop hit max).
        assert!(!result.has_new_tasks, "Should NOT report new tasks");
        assert!(result.hit_max_iterations);
        assert_eq!(result.iterations, 1);
    }

    #[test]
    fn test_uat_loop_convergence_all_uats_verified() {
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

        // All tasks done, all UATs already verified — loop should terminate immediately.
        let frontmatter = PrdFrontmatter {
            id: "PRD-0001".to_string(),
            title: "Already Verified PRD".to_string(),
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

        // Runner should not be called at all.
        let runner = MockRunner::new(vec![]);

        let config = UatVerificationConfig {
            root: &root,
            prd_id: "PRD-0001",
            stream: false,
            max_iterations: Some(5),
            allow_skip_uat: true,
            allow_add_task: true,
        };

        let result = run_uat_verification_loop(&config, &runner).unwrap();

        // Loop terminates immediately with no iterations.
        assert_eq!(result.iterations, 0);
        assert!(!result.has_new_tasks);
        assert!(!result.hit_max_iterations);
        assert_eq!(result.remaining_unverified, 0);
    }

    #[test]
    fn test_uat_result_has_new_tasks_field_default() {
        // Verify that UatVerificationLoopResult correctly reports has_new_tasks
        // when constructed directly (unit test of the struct).
        let result_no_tasks = UatVerificationLoopResult {
            prd_id: "PRD-0001".to_string(),
            prd_path: PathBuf::from("test.md"),
            verified_count: 0,
            opted_out_count: 0,
            iterations: 0,
            hit_max_iterations: false,
            remaining_unverified: 0,
            has_new_tasks: false,
        };
        assert!(!result_no_tasks.has_new_tasks);

        let result_with_tasks = UatVerificationLoopResult {
            prd_id: "PRD-0001".to_string(),
            prd_path: PathBuf::from("test.md"),
            verified_count: 1,
            opted_out_count: 0,
            iterations: 1,
            hit_max_iterations: false,
            remaining_unverified: 2,
            has_new_tasks: true,
        };
        assert!(result_with_tasks.has_new_tasks);
    }

    #[test]
    fn test_has_incomplete_tasks_with_all_done() {
        let prd = Prd::new(
            PrdFrontmatter {
                id: "PRD-0001".to_string(),
                title: "Test".to_string(),
                status: PrdStatus::Active,
                tasks: Some(vec![
                    make_task("T-001", 1, TaskStatus::Done),
                    make_task("T-002", 2, TaskStatus::Done),
                ]),
                ..Default::default()
            },
            "# Body\n".to_string(),
        );

        assert!(!prd.has_incomplete_tasks());
    }

    #[test]
    fn test_has_incomplete_tasks_with_todo() {
        let prd = Prd::new(
            PrdFrontmatter {
                id: "PRD-0001".to_string(),
                title: "Test".to_string(),
                status: PrdStatus::Active,
                tasks: Some(vec![
                    make_task("T-001", 1, TaskStatus::Done),
                    make_task("T-002", 2, TaskStatus::Todo),
                ]),
                ..Default::default()
            },
            "# Body\n".to_string(),
        );

        assert!(prd.has_incomplete_tasks());
    }

    #[test]
    fn test_has_incomplete_tasks_with_in_progress() {
        let prd = Prd::new(
            PrdFrontmatter {
                id: "PRD-0001".to_string(),
                title: "Test".to_string(),
                status: PrdStatus::Active,
                tasks: Some(vec![
                    make_task("T-001", 1, TaskStatus::Done),
                    make_task("T-002", 2, TaskStatus::InProgress),
                ]),
                ..Default::default()
            },
            "# Body\n".to_string(),
        );

        assert!(prd.has_incomplete_tasks());
    }

    #[test]
    fn test_has_incomplete_tasks_with_no_tasks() {
        let prd = Prd::new(
            PrdFrontmatter {
                id: "PRD-0001".to_string(),
                title: "Test".to_string(),
                status: PrdStatus::Active,
                tasks: None,
                ..Default::default()
            },
            "# Body\n".to_string(),
        );

        assert!(!prd.has_incomplete_tasks());
    }
}
