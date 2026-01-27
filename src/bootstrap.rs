//! Bootstrap command implementation for `mr bootstrap`.
//!
//! Ingests an existing repository into PRDs:
//! - Ensures `.mr/` structure exists
//! - Invokes runner with `bootstrap_plan.md` to analyze the repo
//! - Invokes runner with `bootstrap_generate_prds.md` to generate PRDs
//! - Updates `.mr/PRDS.md` index
//!
//! Also supports `--reconstruct` mode which:
//! - Analyzes git history to infer major milestones
//! - Creates PRDs with `status: done` and `reconstructed: true`
//! - Infers `depends_on` relationships from temporal order

use std::path::Path;
use std::sync::LazyLock;

use anyhow::{Context, Result, bail};
use regex::Regex;

use crate::init;
use crate::prd::generate_index_from_root;
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;

/// Default PRD budget when bootstrapping.
const DEFAULT_PRD_BUDGET: u32 = 6;

/// Regex pattern for matching PRD identifiers (PRD-NNNN).
static PRD_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"PRD-\d{4}").expect("PRD regex pattern is valid"));

/// Configuration for the bootstrap command.
#[derive(Debug)]
pub struct BootstrapConfig<'a> {
    /// The repository root directory.
    pub root: &'a Path,

    /// Maximum number of PRDs to generate.
    pub prd_budget: u32,

    /// Whether to run reconstruct workflow (analyze git history).
    pub reconstruct: bool,
}

impl<'a> BootstrapConfig<'a> {
    /// Creates a new bootstrap configuration with defaults.
    pub fn new(root: &'a Path) -> Self {
        Self {
            root,
            prd_budget: DEFAULT_PRD_BUDGET,
            reconstruct: false,
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
/// This function:
/// 1. Ensures `.mr/` structure exists (runs init if needed)
/// 2. Invokes runner with `bootstrap_plan.md` to analyze the repo
/// 3. Invokes runner with `bootstrap_generate_prds.md` to generate PRDs
/// 4. Updates `.mr/PRDS.md` index
///
/// If `config.reconstruct` is true, runs the reconstruct workflow instead.
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

    let plan_output = runner
        .execute(&plan_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Runner failed during plan: {e}"))?;

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

    let generate_prompt = build_generate_prompt(config, &plan_output.text);

    tracing::info!(
        runner = %runner.name(),
        prd_budget = config.prd_budget,
        "Invoking runner to generate PRDs from bootstrap plan"
    );

    let generate_output = runner
        .execute(&generate_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Runner failed during PRD generation: {e}"))?;

    if !generate_output.success {
        bail!(
            "Runner failed during PRD generation: {}",
            generate_output.text
        );
    }

    result.prds_generated = true;

    // Count PRDs created (look for PRD-NNNN patterns in output).
    result.prds_created = count_prds_in_output(&generate_output.text);

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

/// Counts PRDs mentioned in the output.
fn count_prds_in_output(output: &str) -> usize {
    // Look for PRD-NNNN patterns using the pre-compiled static regex.
    let matches: std::collections::HashSet<&str> =
        PRD_PATTERN.find_iter(output).map(|m| m.as_str()).collect();

    matches.len()
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

    let reconstruct_prompt = build_reconstruct_prompt(config);

    tracing::info!(
        runner = %runner.name(),
        "Invoking runner for git history reconstruction"
    );

    let reconstruct_output = runner
        .execute(&reconstruct_prompt, config.root)
        .map_err(|e| anyhow::anyhow!("Runner failed during reconstruction: {e}"))?;

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

    // Count PRDs created from the reconstruct output.
    result.prds_created = count_prds_in_output(&reconstruct_output.text);

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
fn build_reconstruct_prompt(config: &BootstrapConfig) -> String {
    let template = load_prompt_with_fallback(config.root, PromptKind::BootstrapReconstruct);

    let mut ctx = PlaceholderContext::new();

    // Add owner placeholder if available (defaults to empty).
    ctx.insert("owner", "");

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

        let config = BootstrapConfig::new(temp.path());

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

        let config = BootstrapConfig::new(temp.path());

        let result = bootstrap(&config, &runner).unwrap();

        assert!(result.prds_generated);
        assert_eq!(result.prds_created, 3);
    }

    #[test]
    fn test_bootstrap_runner_failure_plan() {
        let temp = setup_test_repo();

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::failure(
            "Error analyzing repo",
        )]);

        let config = BootstrapConfig::new(temp.path());

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

        let config = BootstrapConfig::new(temp.path());

        let result = bootstrap(&config, &runner);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("PRD generation"));
    }

    #[test]
    fn test_count_prds_in_output() {
        let output = "Created PRD-0001, PRD-0002, and PRD-0003. Also updated PRD-0001.";

        let count = count_prds_in_output(output);

        assert_eq!(count, 3); // Unique PRDs only.
    }

    #[test]
    fn test_count_prds_in_output_none() {
        let output = "No PRDs created.";

        let count = count_prds_in_output(output);

        assert_eq!(count, 0);
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

        let config = BootstrapConfig::new(temp.path());

        let result = bootstrap(&config, &runner).unwrap();

        // Verify all steps completed.
        assert!(result.initialized);
        assert!(result.plan_generated);
        assert!(result.prds_generated);
        assert_eq!(result.prds_created, 2);

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

        let config = BootstrapConfig::new(temp.path());

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

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = true;

        let result = bootstrap(&config, &runner).unwrap();

        // Verify reconstruct completed.
        assert!(result.initialized);
        assert!(result.plan_generated);
        assert!(result.prds_generated);
        assert_eq!(result.prds_created, 3);

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

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = true;

        let result = bootstrap(&config, &runner).unwrap();

        // Should not have initialized.
        assert!(!result.initialized);
        assert_eq!(result.prds_created, 1);
    }

    #[test]
    fn test_bootstrap_reconstruct_runner_failure() {
        let temp = setup_test_repo();

        let runner = MockRunner::new(vec![crate::runner::RunnerOutput::failure(
            "Error analyzing git",
        )]);

        let mut config = BootstrapConfig::new(temp.path());
        config.reconstruct = true;

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
}
