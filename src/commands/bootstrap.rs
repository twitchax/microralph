//! Bootstrap command implementation for `mr bootstrap`.
//!
//! Default behavior (reconstruct mode):
//! - Analyzes git history to infer major milestones
//! - Creates PRDs with `status: done` and `reconstructed: true`
//! - Infers `depends_on` relationships from temporal order
//! - Updates `.mr/PRDS.md` index
//!
//! With `--scaffold` flag (scaffold mode):
//! - Ensures `.mr/` structure exists
//! - Invokes runner with `bootstrap_plan.md` to analyze the repo
//! - Invokes runner with `bootstrap_generate_prds.md` to generate PRDs
//! - Updates `.mr/PRDS.md` index

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result, bail};

use crate::init;
use crate::prd::{generate_index_from_root, scan_prd_summaries};
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;
use crate::util::spinner::start_spinner;

/// Default PRD budget when bootstrapping.
const DEFAULT_PRD_BUDGET: u32 = 6;

/// Configuration for the bootstrap command.
#[derive(Debug)]
pub struct BootstrapConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// Maximum number of PRDs to generate.
    pub prd_budget: u32,

    /// Whether to run reconstruct workflow (analyze git history).
    /// Defaults to true; set to false via `--scaffold` flag.
    pub reconstruct: bool,

    /// Whether to stream runner output in real-time.
    pub stream: bool,
}

impl<'a> BootstrapConfig<'a> {
    /// Creates a new bootstrap configuration with defaults.
    ///
    /// Default behavior is reconstruct mode (analyze git history).
    /// Use `--scaffold` flag to enable scaffold mode instead.
    pub fn new(root: &'a Path) -> Self {
        Self {
            root,
            prd_budget: DEFAULT_PRD_BUDGET,
            reconstruct: true,
            stream: false,
        }
    }
}

/// Result from the bootstrap process.
#[derive(Debug)]
pub struct BootstrapResult {
    /// Whether initialization was performed.
    pub initialized: bool,

    /// Whether the bootstrap plan was generated.
    pub plan_generated: bool,

    /// Whether PRDs were generated.
    pub prds_generated: bool,

    /// Number of PRDs created.
    pub prds_created: usize,

    /// Summary of the bootstrap plan.
    pub plan_summary: String,
}

/// Runs the bootstrap process.
///
/// Default behavior (reconstruct=true):
/// 1. Ensures `.mr/` structure exists (runs init if needed)
/// 2. Analyzes git history to infer major milestones
/// 3. Creates PRDs with `status: done` and `reconstructed: true`
/// 4. Updates `.mr/PRDS.md` index
///
/// With `--scaffold` flag (reconstruct=false):
/// 1. Ensures `.mr/` structure exists (runs init if needed)
/// 2. Invokes runner with `bootstrap_plan.md` to analyze the repo
/// 3. Invokes runner with `bootstrap_generate_prds.md` to generate PRDs
/// 4. Updates `.mr/PRDS.md` index
pub fn bootstrap<R>(config: &BootstrapConfig, runner: &R) -> Result<BootstrapResult>
where
    R: Runner + ?Sized,
{
    let mut result = BootstrapResult {
        initialized: false,
        plan_generated: false,
        prds_generated: false,
        prds_created: 0,
        plan_summary: String::new(),
    };

    // Step 1: Ensure .mr/ structure exists.
    if !init::is_initialized(config.root) {
        tracing::info!("Initializing .mr/ structure...");

        init::init(config.root).context("Failed to initialize .mr/ structure")?;

        result.initialized = true;
    }

    // Branch based on reconstruct flag.
    if config.reconstruct {
        return bootstrap_reconstruct(config, runner, result);
    }

    // Normal bootstrap flow continues here.

    // Step 2: Run bootstrap plan.
    tracing::info!("Analyzing repository...");

    let plan_prompt = build_plan_prompt(config);

    tracing::info!(
        runner = %runner.name(),
        prd_budget = config.prd_budget,
        "Invoking runner for bootstrap plan"
    );

    // Print command info before spinner (only when not streaming).
    if !config.stream
        && let Some(cmd_display) = runner.format_command_display(&plan_prompt, config.root)
    {
        println!("\n🔧 Executing: {cmd_display}");
    }

    // Start spinner when not streaming.
    let spinner = start_spinner(!config.stream, "Analyzing repository...");

    let plan_output = if config.stream {
        let mut stdout = std::io::stdout();
        runner
            .execute_streaming(&plan_prompt, config.root, &mut stdout)
            .map_err(|e| anyhow::anyhow!("Runner failed during plan: {e}"))?
    } else {
        runner
            .execute(&plan_prompt, config.root)
            .map_err(|e| anyhow::anyhow!("Runner failed during plan: {e}"))?
    };

    // Clear spinner.
    spinner.finish_and_clear();

    if !plan_output.success {
        bail!("Runner failed during bootstrap plan: {}", plan_output.text);
    }

    result.plan_generated = true;
    result.plan_summary = summarize_plan(&plan_output.text);

    tracing::debug!(
        plan_len = plan_output.text.len(),
        "Bootstrap plan generated"
    );

    // Step 3: Generate PRDs.
    tracing::info!("Generating PRDs...");

    // Count PRDs before generation to calculate delta.
    let prds_before = count_prd_files(config.root);

    let generate_prompt = build_generate_prompt(config, &plan_output.text);

    tracing::info!(
        runner = %runner.name(),
        prd_budget = config.prd_budget,
        "Invoking runner to generate PRDs from bootstrap plan"
    );

    // Print command info before spinner (only when not streaming).
    if !config.stream
        && let Some(cmd_display) = runner.format_command_display(&generate_prompt, config.root)
    {
        println!("\n🔧 Executing: {cmd_display}");
    }

    // Start spinner when not streaming.
    let spinner = start_spinner(!config.stream, "Generating PRDs...");

    let generate_output = if config.stream {
        let mut stdout = std::io::stdout();
        runner
            .execute_streaming(&generate_prompt, config.root, &mut stdout)
            .map_err(|e| anyhow::anyhow!("Runner failed during PRD generation: {e}"))?
    } else {
        runner
            .execute(&generate_prompt, config.root)
            .map_err(|e| anyhow::anyhow!("Runner failed during PRD generation: {e}"))?
    };

    // Clear spinner.
    spinner.finish_and_clear();

    if !generate_output.success {
        bail!(
            "Runner failed during PRD generation: {}",
            generate_output.text
        );
    }

    result.prds_generated = true;

    // Count PRDs after generation and calculate how many were created.
    let prds_after = count_prd_files(config.root);
    result.prds_created = prds_after.saturating_sub(prds_before);

    // Step 4: Regenerate index.
    tracing::info!("Regenerating PRD index...");

    generate_index_from_root(config.root)?;

    Ok(result)
}

/// Builds the bootstrap plan prompt.
fn build_plan_prompt(config: &BootstrapConfig) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::BootstrapPlan);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("prd_budget", config.prd_budget.to_string());

    // Add common heuristics.
    let heuristics: Vec<std::collections::HashMap<String, String>> = vec![
        [(
            "description".to_string(),
            "Detect cargo-make entrypoints and required tasks".to_string(),
        )]
        .into_iter()
        .collect(),
        [(
            "description".to_string(),
            "Detect crates/modules and responsibilities".to_string(),
        )]
        .into_iter()
        .collect(),
        [(
            "description".to_string(),
            "Detect CI workflows and required checks".to_string(),
        )]
        .into_iter()
        .collect(),
        [(
            "description".to_string(),
            "Detect docs that imply features (README/DEVELOPMENT/etc.)".to_string(),
        )]
        .into_iter()
        .collect(),
        [(
            "description".to_string(),
            "Detect TODO/FIXME hotspots".to_string(),
        )]
        .into_iter()
        .collect(),
    ];

    ctx.insert("heuristics", PlaceholderValue::List(heuristics));

    expand_placeholders(&template, &ctx)
}

/// Builds the bootstrap generate PRDs prompt.
fn build_generate_prompt(config: &BootstrapConfig, plan: &str) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::BootstrapGeneratePrds);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("plan", plan);
    ctx.insert("prd_budget", config.prd_budget.to_string());

    expand_placeholders(&template, &ctx)
}

/// Summarizes the bootstrap plan output.
fn summarize_plan(plan: &str) -> String {
    // Take last 500 chars or last few lines as summary.
    let all_lines: Vec<&str> = plan.lines().collect();
    let start_line = all_lines.len().saturating_sub(10);
    let lines = &all_lines[start_line..];
    let summary = lines.join("\n");

    if summary.len() > 500 {
        let start = summary.len() - 500;
        format!("...\n{}", &summary[start..])
    } else {
        summary
    }
}

/// Counts actual PRD files on disk.
fn count_prd_files(root: &Path) -> usize {
    scan_prd_summaries(root).unwrap_or_default().len()
}

/// Runs the reconstruct bootstrap workflow.
///
/// This workflow:
/// 1. Analyzes git history (commits, tags, major changes)
/// 2. Infers major development milestones as PRDs
/// 3. Creates PRDs with `status: done` and `reconstructed: true`
/// 4. Infers `depends_on` relationships from temporal order
/// 5. Regenerates the PRD index
fn bootstrap_reconstruct<R>(
    config: &BootstrapConfig,
    runner: &R,
    mut result: BootstrapResult,
) -> Result<BootstrapResult>
where
    R: Runner + ?Sized,
{
    tracing::info!("Running reconstruct workflow...");

    // Count PRDs before reconstruction to calculate delta.
    let prds_before = count_prd_files(config.root);

    let reconstruct_prompt = build_reconstruct_prompt(config);

    tracing::info!(
        runner = %runner.name(),
        "Invoking runner for git history reconstruction"
    );

    // Print command info before spinner (only when not streaming).
    if !config.stream
        && let Some(cmd_display) = runner.format_command_display(&reconstruct_prompt, config.root)
    {
        println!("\n🔧 Executing: {cmd_display}");
    }

    // Start spinner when not streaming.
    let spinner = start_spinner(!config.stream, "Reconstructing from git history...");

    let reconstruct_output = if config.stream {
        let mut stdout = std::io::stdout();
        runner
            .execute_streaming(&reconstruct_prompt, config.root, &mut stdout)
            .map_err(|e| anyhow::anyhow!("Runner failed during reconstruction: {e}"))?
    } else {
        runner
            .execute(&reconstruct_prompt, config.root)
            .map_err(|e| anyhow::anyhow!("Runner failed during reconstruction: {e}"))?
    };

    // Clear spinner.
    spinner.finish_and_clear();

    if !reconstruct_output.success {
        bail!(
            "Runner failed during git history reconstruction: {}",
            reconstruct_output.text
        );
    }

    // For reconstruct, we skip the plan phase and go directly to PRD generation.
    result.plan_generated = true;
    result.plan_summary = "Reconstructed from git history".to_string();
    result.prds_generated = true;

    // Count PRDs after reconstruction and calculate how many were created.
    let prds_after = count_prd_files(config.root);
    result.prds_created = prds_after.saturating_sub(prds_before);

    tracing::debug!(
        output_len = reconstruct_output.text.len(),
        prds_created = result.prds_created,
        "Reconstruction completed"
    );

    // Regenerate index.
    tracing::info!("Regenerating PRD index...");

    generate_index_from_root(config.root)?;

    Ok(result)
}

/// Builds the bootstrap reconstruct prompt.
///
/// Includes existing PRD information to ensure idempotency (T-007).
fn build_reconstruct_prompt(config: &BootstrapConfig) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::BootstrapReconstruct);

    let mut ctx = PlaceholderContext::new();

    // Add owner placeholder if available (defaults to empty).
    ctx.insert("owner", "");

    // Add existing PRDs for idempotency (skip/merge with existing PRDs).
    let existing_prds = scan_prd_summaries(config.root).unwrap_or_default();

    let prd_list: Vec<HashMap<String, String>> = existing_prds
        .iter()
        .map(|p| {
            [
                ("id".to_string(), p.id.clone()),
                ("title".to_string(), p.title.clone()),
                ("status".to_string(), p.status.to_string()),
            ]
            .into_iter()
            .collect()
        })
        .collect();

    ctx.insert("existing_prds", PlaceholderValue::List(prd_list));

    expand_placeholders(&template, &ctx)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::runner::MockRunner;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        TempDir::new().unwrap()
    }

    #[test]
    fn test_bootstrap_config_defaults() {
        let temp = setup_test_repo();
        let config = BootstrapConfig::new(temp.path());

        assert_eq!(config.prd_budget, DEFAULT_PRD_BUDGET);
        // PRD-0027 uat-001: Default behavior is reconstruct (not scaffold).
        assert!(config.reconstruct, "Default should be reconstruct mode");
    }

    #[test]
    fn test_bootstrap_initializes_if_needed() {
        let temp = setup_test_repo();

        // Verify not initialized.
        assert!(!init::is_initialized(temp.path()));

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Plan: Generate 2 PRDs..."),
            crate::runner::RunnerOutput::success("Created PRD-0001 and PRD-0002."),
        ]);

        let config = BootstrapConfig::new(temp.path());

        let result = bootstrap(&config, &runner).unwrap();

        // Verify initialized.
        assert!(result.initialized);
        assert!(init::is_initialized(temp.path()));
    }

    #[test]
    fn test_bootstrap_skips_init_if_exists() {
        let temp = setup_test_repo();

        // Initialize first.
        init::init(temp.path()).unwrap();

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Plan: Generate 1 PRD..."),
            crate::runner::RunnerOutput::success("Created PRD-0001."),
        ]);

        let config = BootstrapConfig::new(temp.path());

        let result = bootstrap(&config, &runner).unwrap();

        // Should not have initialized.
        assert!(!result.initialized);
    }

    #[test]
    fn test_bootstrap_plan_generated() {
        let temp = setup_test_repo();

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success(
                "Bootstrap Plan:\n1. PRD for feature A\n2. PRD for feature B",
            ),
            crate::runner::RunnerOutput::success("Created PRD-0001 and PRD-0002."),
        ]);

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = false; // Test scaffold behavior

        let result = bootstrap(&config, &runner).unwrap();

        assert!(result.plan_generated);
        assert!(result.plan_summary.contains("Bootstrap Plan"));
    }

    #[test]
    fn test_bootstrap_prds_generated() {
        let temp = setup_test_repo();

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Plan complete."),
            crate::runner::RunnerOutput::success("Generated PRD-0001, PRD-0002, and PRD-0003."),
        ]);

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = false; // Test scaffold behavior

        let result = bootstrap(&config, &runner).unwrap();

        assert!(result.prds_generated);
        // MockRunner doesn't create actual files, so prds_created is 0.
        assert_eq!(result.prds_created, 0);
    }

    #[test]
    fn test_bootstrap_runner_failure_plan() {
        let temp = setup_test_repo();

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::failure(
            "Error analyzing repo",
        )]);

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = false; // Test scaffold behavior

        let result = bootstrap(&config, &runner);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("bootstrap plan"));
    }

    #[test]
    fn test_bootstrap_runner_failure_generate() {
        let temp = setup_test_repo();

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Plan complete."),
            crate::runner::RunnerOutput::failure("Error generating PRDs"),
        ]);

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = false; // Test scaffold behavior

        let result = bootstrap(&config, &runner);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("PRD generation"));
    }

    #[test]
    fn test_count_prd_files() {
        let temp = setup_test_repo();

        // Initialize to create .mr structure.
        init::init(temp.path()).unwrap();

        // Initially no PRDs.
        assert_eq!(count_prd_files(temp.path()), 0);

        // Create a PRD file.
        let prd_content = r#"---
id: PRD-0001
title: "Test PRD"
status: draft
owner: Test
created: 2026-01-01
updated: 2026-01-01
tasks:
- id: T-001
  title: "Task"
  priority: 1
  status: todo
---

# Summary

Test PRD.
"#;
        let prds_dir = temp.path().join(".mr/prds");
        std::fs::write(prds_dir.join("PRD-0001-test.md"), prd_content).unwrap();

        // Now one PRD.
        assert_eq!(count_prd_files(temp.path()), 1);
    }

    #[test]
    fn test_summarize_plan() {
        let plan = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";

        let summary = summarize_plan(plan);

        assert!(summary.contains("Line 1"));
        assert!(summary.contains("Line 5"));
    }

    #[test]
    fn test_build_plan_prompt() {
        let temp = setup_test_repo();

        // Initialize to have prompts available.
        init::init(temp.path()).unwrap();

        let config = BootstrapConfig::new(temp.path());

        let prompt = build_plan_prompt(&config);

        // Should contain content from the bootstrap_plan.md template.
        assert!(prompt.contains("Objective") || prompt.contains("objective"));
    }

    #[test]
    fn test_build_generate_prompt() {
        let temp = setup_test_repo();

        // Initialize to have prompts available.
        init::init(temp.path()).unwrap();

        let config = BootstrapConfig::new(temp.path());
        let plan = "Generate 2 PRDs for features A and B.";

        let prompt = build_generate_prompt(&config, plan);

        // Should contain the plan.
        assert!(prompt.contains("Generate 2 PRDs"));
    }

    #[test]
    fn test_full_bootstrap_flow() {
        let temp = setup_test_repo();

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success(
                "Bootstrap Plan:\n- PRD-0001: Core feature\n- PRD-0002: Extended feature",
            ),
            crate::runner::RunnerOutput::success("Created PRD-0001 and PRD-0002 successfully."),
        ]);

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = false; // Test scaffold behavior

        let result = bootstrap(&config, &runner).unwrap();

        // Verify all steps completed.
        assert!(result.initialized);
        assert!(result.plan_generated);
        assert!(result.prds_generated);
        // MockRunner doesn't create actual files, so prds_created is 0.
        assert_eq!(result.prds_created, 0);

        // Verify runner was called 2 times (plan, generate).
        assert_eq!(runner.recorded_prompts().len(), 2);

        // Verify .mr/ structure exists.
        assert!(temp.path().join(".mr/prds").exists());
        assert!(temp.path().join(".mr/PRDS.md").exists());
        assert!(temp.path().join("AGENTS.md").exists());
    }

    #[test]
    fn test_bootstrap_creates_constitution() {
        let temp = setup_test_repo();

        // Verify constitution doesn't exist yet.
        assert!(!temp.path().join(".mr/constitution.md").exists());

        let runner = MockRunner::new(vec![
            crate::runner::RunnerOutput::success("Plan: Generate 1 PRD..."),
            crate::runner::RunnerOutput::success("Created PRD-0001."),
        ]);

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = false; // Test scaffold behavior

        let result = bootstrap(&config, &runner).unwrap();

        // Verify bootstrap initialized (and thus created constitution).
        assert!(result.initialized);

        // Verify constitution.md was created.
        assert!(temp.path().join(".mr/constitution.md").exists());

        // Verify constitution contains expected content.
        let constitution_content =
            std::fs::read_to_string(temp.path().join(".mr/constitution.md")).unwrap();
        assert!(constitution_content.contains("# Constitution"));
        assert!(constitution_content.contains("## Purpose"));
        assert!(constitution_content.contains("## Rules"));
    }

    #[test]
    fn test_bootstrap_reconstruct_workflow() {
        let temp = setup_test_repo();

        // Mock runner for reconstruct (single call).
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(
            "Reconstructed PRD-0001, PRD-0002, and PRD-0003 from git history.",
        )]);

        let config = BootstrapConfig::new(temp.path());
        // reconstruct is true by default, no need to set explicitly

        let result = bootstrap(&config, &runner).unwrap();

        // Verify reconstruct completed.
        assert!(result.initialized);
        assert!(result.plan_generated);
        assert!(result.prds_generated);
        // MockRunner doesn't create actual files, so prds_created is 0.
        assert_eq!(result.prds_created, 0);

        // Verify summary is set for reconstruct.
        assert!(result.plan_summary.contains("git history"));

        // Verify runner was called once (reconstruct is single call).
        assert_eq!(runner.recorded_prompts().len(), 1);
    }

    #[test]
    fn test_bootstrap_reconstruct_skips_init_if_exists() {
        let temp = setup_test_repo();

        // Initialize first.
        init::init(temp.path()).unwrap();

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(
            "Reconstructed PRD-0001 from commits.",
        )]);

        let config = BootstrapConfig::new(temp.path());
        // reconstruct is true by default, no need to set explicitly

        let result = bootstrap(&config, &runner).unwrap();

        // Should not have initialized.
        assert!(!result.initialized);
        // MockRunner doesn't create actual files, so prds_created is 0.
        assert_eq!(result.prds_created, 0);
    }

    #[test]
    fn test_bootstrap_reconstruct_runner_failure() {
        let temp = setup_test_repo();

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::failure(
            "Error analyzing git",
        )]);

        let config = BootstrapConfig::new(temp.path());
        // reconstruct is true by default, no need to set explicitly

        let result = bootstrap(&config, &runner);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("reconstruction"));
    }

    #[test]
    fn test_build_reconstruct_prompt() {
        let temp = setup_test_repo();

        // Initialize to have prompts available.
        init::init(temp.path()).unwrap();

        let config = BootstrapConfig::new(temp.path());

        let prompt = build_reconstruct_prompt(&config);

        // Should contain content from the bootstrap_reconstruct.md template.
        assert!(prompt.contains("Objective") || prompt.contains("objective"));
        assert!(prompt.contains("git") || prompt.contains("Git"));
    }

    #[test]
    fn test_build_reconstruct_prompt_includes_existing_prds() {
        let temp = setup_test_repo();

        // Initialize to have prompts and prds directory available.
        init::init(temp.path()).unwrap();

        // Create a test PRD file.
        let prd_content = r#"---
id: PRD-0001
title: "Test Existing PRD"
status: done
owner: Test
created: 2026-01-01
updated: 2026-01-01
tasks:
- id: T-001
  title: "Task"
  priority: 1
  status: done
---

# Summary

Test PRD for idempotency testing.
"#;

        let prds_dir = temp.path().join(".mr/prds");
        std::fs::create_dir_all(&prds_dir).unwrap();
        std::fs::write(prds_dir.join("PRD-0001-test.md"), prd_content).unwrap();

        let config = BootstrapConfig::new(temp.path());

        let prompt = build_reconstruct_prompt(&config);

        // Should contain the existing PRD information.
        assert!(prompt.contains("PRD-0001"));
        assert!(prompt.contains("Test Existing PRD"));
        assert!(prompt.contains("done"));
        // Should contain the "Do Not Duplicate" section.
        assert!(prompt.contains("Do Not Duplicate") || prompt.contains("Do NOT create"));
    }

    #[test]
    fn test_build_reconstruct_prompt_no_existing_prds() {
        let temp = setup_test_repo();

        // Initialize to have prompts available (but no PRDs).
        init::init(temp.path()).unwrap();

        let config = BootstrapConfig::new(temp.path());

        let prompt = build_reconstruct_prompt(&config);

        // Should contain the "No existing PRDs" message.
        assert!(prompt.contains("No existing PRDs found"));
    }

    // ========================================================================
    // Integration Tests for Reconstruct Workflow (T-017)
    // ========================================================================

    #[test]
    fn test_reconstruct_integration_creates_prds_from_git_history() {
        // Integration test: Verify reconstruct workflow creates PRDs and updates index.
        let temp = setup_test_repo();

        // Initialize a git repository to simulate real environment.
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to initialize git repo");

        // Configure git user for commits.
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to configure git email");
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to configure git name");

        // Mock runner simulates LLM creating PRD files.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(
            "Analyzed git history. Created PRD-0001 and PRD-0002 with depends_on relationships.",
        )]);

        let config = BootstrapConfig::new(temp.path());
        // reconstruct is true by default, no need to set explicitly

        let result = bootstrap(&config, &runner).unwrap();

        // Verify reconstruct completed.
        assert!(result.initialized);
        assert!(result.plan_generated);
        assert!(result.prds_generated);
        // MockRunner doesn't create actual files, so prds_created is 0.
        assert_eq!(result.prds_created, 0);

        // Verify .mr/ structure exists.
        assert!(temp.path().join(".mr/prds").exists());
        assert!(temp.path().join(".mr/PRDS.md").exists());

        // Verify reconstruct prompt was called once.
        let prompts = runner.recorded_prompts();
        assert_eq!(prompts.len(), 1);
        assert!(
            prompts[0].contains("git") || prompts[0].contains("history"),
            "Reconstruct prompt should mention git or history"
        );
    }

    #[test]
    fn test_reconstruct_integration_idempotent_with_existing_prds() {
        // Integration test: Verify reconstruct skips existing PRDs.
        let temp = setup_test_repo();

        // Initialize the repository first.
        init::init(temp.path()).unwrap();

        // Create an existing PRD.
        let prd_content = r#"---
id: PRD-0001
title: "Existing Feature"
status: done
owner: Test
created: 2026-01-01
updated: 2026-01-01
reconstructed: true
tasks:
- id: T-001
  title: "Initial task"
  priority: 1
  status: done
---

# Summary

An existing PRD that should not be duplicated.
"#;
        let prds_dir = temp.path().join(".mr/prds");
        std::fs::write(prds_dir.join("PRD-0001-existing.md"), prd_content).unwrap();

        // Mock runner that should receive context about existing PRDs.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(
            "Created PRD-0002 and PRD-0003. Skipped existing PRD as it already exists.",
        )]);

        let config = BootstrapConfig::new(temp.path());
        // reconstruct is true by default, no need to set explicitly

        let result = bootstrap(&config, &runner).unwrap();

        // Verify reconstruct completed without re-initialization.
        assert!(!result.initialized);
        // MockRunner doesn't create actual files. Started with 1 PRD, still 1 PRD.
        assert_eq!(result.prds_created, 0);

        // Verify the prompt included existing PRD info.
        let prompts = runner.recorded_prompts();
        assert_eq!(prompts.len(), 1);
        assert!(
            prompts[0].contains("PRD-0001"),
            "Prompt should include existing PRD ID"
        );
        assert!(
            prompts[0].contains("Existing Feature"),
            "Prompt should include existing PRD title"
        );
    }

    #[test]
    fn test_reconstruct_integration_with_depends_on_inference() {
        // Integration test: Verify reconstruct prompt supports depends_on inference.
        let temp = setup_test_repo();

        // Initialize a git repo with some commits to analyze.
        std::process::Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to initialize git repo");

        // Create a file and commit it.
        std::fs::write(temp.path().join("README.md"), "# Test Project").unwrap();
        std::process::Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to configure git");
        std::process::Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to configure git");
        std::process::Command::new("git")
            .args(["add", "."])
            .current_dir(temp.path())
            .output()
            .expect("Failed to stage files");
        std::process::Command::new("git")
            .args(["commit", "-m", "Initial commit"])
            .current_dir(temp.path())
            .output()
            .expect("Failed to commit");

        // Mock runner that simulates creating PRDs with depends_on.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(
            r"Created PRD-0001 (status: done, reconstructed: true, depends_on: []).
Created PRD-0002 (status: done, reconstructed: true, depends_on: [PRD-0001]).
Created PRD-0003 (status: done, reconstructed: true, depends_on: [PRD-0001, PRD-0002]).",
        )]);

        let config = BootstrapConfig::new(temp.path());
        // reconstruct is true by default, no need to set explicitly

        let result = bootstrap(&config, &runner).unwrap();

        // MockRunner doesn't create actual files, so prds_created is 0.
        assert_eq!(result.prds_created, 0);

        // Verify the reconstruct prompt contains guidance about depends_on.
        let prompts = runner.recorded_prompts();
        assert!(
            prompts[0].contains("depends_on") || prompts[0].contains("dependency"),
            "Prompt should mention depends_on or dependency relationships"
        );
    }

    #[test]
    fn test_reconstruct_integration_full_workflow_with_index_regeneration() {
        // Integration test: Full workflow from reconstruct to index generation.
        let temp = setup_test_repo();

        // Mock runner simulates LLM creating PRD files.
        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::success(
            "Reconstructed project history. Created PRD-0001, PRD-0002.",
        )]);

        let config = BootstrapConfig::new(temp.path());
        // reconstruct is true by default, no need to set explicitly

        let result = bootstrap(&config, &runner).unwrap();

        // Verify the PRDS.md index was generated.
        let index_path = temp.path().join(".mr/PRDS.md");
        assert!(index_path.exists(), "PRDS.md should be created");

        let index_content = std::fs::read_to_string(&index_path).unwrap();
        assert!(
            index_content.contains("# PRD Index")
                || index_content.contains("PRDs")
                || index_content.contains("No active PRDs"),
            "Index should contain PRD-related content"
        );

        // Verify result summary.
        assert!(result.plan_summary.contains("git history"));
    }
}
