use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::Path;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod bootstrap;
mod changelog;
mod colors;
mod config;
mod constitution_edit;
mod devcontainer;
mod init;
mod prd;
mod prd_edit;
mod prd_finalize;
mod prd_new;
mod prompt;
mod qa_workflow;
mod reindex;
mod run;
mod runner;
mod status;
mod suggest;

use runner::Runner;

/// microralph (`mr`) — A tiny CLI for creating and executing PRDs with coding agents.
///
/// Commands are organized by stages:
/// - [0] Initialization
/// - [1] PRD Creation
/// - [2] Task Execution
/// - [3] PRD Finalization
/// - [H] Helper Commands
/// - [C] Configuration/Container Commands
#[derive(Parser, Debug)]
#[command(author, version, long_about, verbatim_doc_comment)]
struct Args {
    /// Enable verbose output.
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Suppress non-essential output.
    #[arg(short, long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// [0] Initialize a new repo with `.mr/` structure, templates, prompts, and starter AGENTS.md.
    #[command(display_order = 1)]
    Init {
        /// Target programming language (rust, python, node, go, java).
        /// If unspecified or "rust", uses default prompts.
        /// Otherwise, invokes runner to adapt prompts/templates.
        #[arg(long)]
        language: Option<String>,

        /// The runner to use for language adaptation (only needed for non-Rust languages).
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,
    },

    /// [0] Ingest an existing repo into PRDs: generate `.mr/PRDS.md` and starter PRDs.
    #[command(display_order = 2)]
    Bootstrap {
        /// The runner to use for bootstrapping.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Target programming language (rust, python, node, go, java).
        /// If unspecified, auto-detects from project files.
        #[arg(long)]
        language: Option<String>,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,
    },

    /// [0] Restore `.mr/prompts/` and `.mr/templates/` to built-in defaults.
    #[command(display_order = 3)]
    Restore,

    /// [1] Create a new PRD via guided Q/A.
    #[command(display_order = 4)]
    New {
        /// The slug for the new PRD (e.g., "add-user-auth").
        slug: String,

        /// The runner to use for the Q/A session.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,

        /// Upfront context to provide before question generation.
        /// This helps the AI ask more relevant, targeted questions.
        #[arg(long)]
        context: Option<String>,
    },

    /// [1] Edit an existing PRD via runner-assisted modifications.
    #[command(display_order = 5)]
    Edit {
        /// The PRD ID to edit (e.g., "PRD-0001").
        prd_id: String,

        /// The edit request (what changes to make).
        request: String,

        /// The runner to use for the edit session.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,
    },

    /// [2] Run the next task from the active PRD.
    #[command(display_order = 6)]
    Run {
        /// Optional PRD ID to run (e.g., "PRD-0001"). If omitted, runs the highest-priority active PRD.
        prd: Option<String>,

        /// The runner to use for task execution.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Run only one task and exit (default is to loop until all tasks are done).
        #[arg(long)]
        one: bool,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,

        /// Stream runner output to stdout in real-time.
        #[arg(long)]
        stream: bool,
    },

    /// [3] Finalize a PRD after all tasks are complete.
    #[command(display_order = 7)]
    Finalize {
        /// The PRD ID to finalize (e.g., "PRD-0001").
        prd_id: String,

        /// The runner to use for finalization.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,

        /// Stream runner output to stdout in real-time.
        #[arg(long)]
        stream: bool,
    },

    /// [H] List all PRDs.
    #[command(display_order = 8)]
    List,

    /// [H] Show status of PRDs and tasks.
    #[command(display_order = 9)]
    Status,

    /// [H] Generate AI-driven PRD suggestions based on codebase analysis.
    #[command(display_order = 10)]
    Suggest {
        /// The runner to use for suggestion generation.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,
    },

    /// [C] Dev container management commands.
    #[command(display_order = 11)]
    Devcontainer {
        #[command(subcommand)]
        command: DevcontainerCommand,
    },

    /// [C] Regenerate `.mr/PRDS.md` index and fix inter-PRD/code links in PRDs.
    #[command(display_order = 12)]
    Reindex {
        /// The runner to use for link verification/fixing.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,

        /// Stream runner output to stdout in real-time.
        #[arg(long)]
        stream: bool,
    },

    /// [C] Constitution management commands.
    #[command(display_order = 13)]
    Constitution {
        #[command(subcommand)]
        command: ConstitutionCommand,
    },
}

#[derive(Subcommand, Debug)]
enum DevcontainerCommand {
    /// Generate a dev container configuration from repository analysis.
    Generate {
        /// The runner to use for generation.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ConstitutionCommand {
    /// Edit the constitution via LLM-assisted modifications.
    Edit {
        /// The edit request (what changes to make).
        request: String,

        /// The runner to use for the edit session.
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4-20250514").
        #[arg(long)]
        model: Option<String>,
    },
}

fn main() -> Result<()> {
    let args = Args::parse();

    init_tracing(args.verbose, args.quiet);

    match args.command {
        Some(Command::Init {
            language,
            runner,
            model,
        }) => {
            tracing::info!(language = ?language, "Initializing microralph...");
            cmd_init(language.as_deref(), &runner, model.as_deref())?;
        }
        Some(Command::Bootstrap {
            runner,
            language,
            model,
        }) => {
            tracing::info!(runner = %runner, language = ?language, "Bootstrapping repo...");
            cmd_bootstrap(&runner, language.as_deref(), model.as_deref())?;
        }
        Some(Command::Restore) => {
            tracing::info!("Restoring prompts and templates...");
            cmd_restore()?;
        }
        Some(Command::New {
            slug,
            runner,
            model,
            context,
        }) => {
            tracing::info!(slug = %slug, runner = %runner, "Creating new PRD...");
            cmd_prd_new(&slug, &runner, model.as_deref(), context.as_deref())?;
        }
        Some(Command::Edit {
            prd_id,
            request,
            runner,
            model,
        }) => {
            let prd_id = normalize_prd_id(&prd_id);
            tracing::info!(prd_id = %prd_id, runner = %runner, "Editing PRD...");
            cmd_prd_edit(&prd_id, &request, &runner, model.as_deref())?;
        }
        Some(Command::List) => {
            tracing::info!("Listing PRDs...");
            cmd_prd_list()?;
        }
        Some(Command::Finalize {
            prd_id,
            runner,
            model,
            stream,
        }) => {
            let prd_id = normalize_prd_id(&prd_id);
            tracing::info!(prd_id = %prd_id, runner = %runner, stream = %stream, "Finalizing PRD...");
            cmd_prd_finalize(&prd_id, &runner, model.as_deref(), stream)?;
        }
        Some(Command::Run {
            prd,
            runner,
            one,
            model,
            stream,
        }) => {
            tracing::info!(prd = ?prd, runner = %runner, one = %one, stream = %stream, "Running next task...");
            cmd_run(prd.as_deref(), &runner, one, model.as_deref(), stream)?;
        }
        Some(Command::Status) => {
            tracing::info!("Showing status...");
            cmd_status()?;
        }
        Some(Command::Suggest { runner, model }) => {
            tracing::info!(runner = %runner, "Generating PRD suggestions...");
            cmd_suggest(&runner, model.as_deref())?;
        }
        Some(Command::Devcontainer { command }) => match command {
            DevcontainerCommand::Generate { runner, model } => {
                tracing::info!(runner = %runner, "Generating dev container config...");
                cmd_devcontainer_generate(&runner, model.as_deref())?;
            }
        },
        Some(Command::Constitution { command }) => match command {
            ConstitutionCommand::Edit {
                request,
                runner,
                model,
            } => {
                tracing::info!(runner = %runner, "Editing constitution...");
                cmd_constitution_edit(&request, &runner, model.as_deref())?;
            }
        },
        Some(Command::Reindex {
            runner,
            model,
            stream,
        }) => {
            tracing::info!(runner = %runner, stream = %stream, "Reindexing PRDs...");
            cmd_reindex(&runner, model.as_deref(), stream)?;
        }
        None => {
            println!(
                "{}",
                colors::info(
                    "microralph (`mr`) — A tiny CLI for creating and executing PRDs with coding agents."
                )
            );
            println!();
            println!("{}", colors::dim("Run `mr --help` for available commands."));
        }
    }

    Ok(())
}

fn init_tracing(verbose: bool, quiet: bool) {
    let filter = if quiet {
        EnvFilter::new("error")
    } else if verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(filter)
        .init();
}

/// Creates a runner based on the runner name and model.
fn create_runner(runner_name: &str, model: Option<String>) -> Result<Box<dyn runner::Runner>> {
    match runner_name {
        "mock" => Ok(Box::new(runner::MockRunner::empty())),
        "copilot" => {
            let copilot = runner::CopilotRunner::with_model(model);

            if !copilot.is_available() {
                anyhow::bail!(
                    "Copilot CLI is not available. Install it or use `--runner copilot` for testing."
                );
            }

            Ok(Box::new(copilot))
        }
        "claude" => {
            let claude = runner::ClaudeRunner::with_model(model);

            if !claude.is_available() {
                anyhow::bail!(
                    "Claude CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Ok(Box::new(claude))
        }
        other => anyhow::bail!("Unknown runner: {other}. Supported: copilot, claude, mock"),
    }
}

/// Runs the `mr init` command.
fn cmd_init(language: Option<&str>, runner_name: &str, cli_model: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if init::is_initialized(&cwd) {
        println!(
            "{}",
            colors::info("microralph is already initialized in this directory.")
        );
        println!("{}", colors::dim("Run `mr status` to see PRD status."));
        return Ok(());
    }

    // Parse language if provided.
    let lang = match language {
        Some(l) => l
            .parse::<init::Language>()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        None => init::Language::Rust,
    };

    let result = init::init(&cwd)?;

    println!("{}", colors::success("Initialized microralph!"));
    println!();
    println!(
        "{}",
        colors::info(&format!(
            "Created {} directories, {} files.",
            result.dirs_created, result.files_created
        ))
    );

    if !result.created_paths.is_empty() {
        println!();
        println!("{}", colors::header("Created files:"));
        for path in &result.created_paths {
            println!("  - {}", colors::dim(path));
        }
    }

    // Adapt prompts/templates for non-Rust languages.
    if lang != init::Language::Rust {
        println!();
        println!(
            "{}",
            colors::info(&format!("Adapting prompts and templates for {}...", lang))
        );

        // Load config for model (config file was just created).
        let cfg = config::Config::load_or_default(&cwd)?;
        let model = cfg.effective_model(cli_model);

        adapt_language(&cwd, lang, runner_name, model.as_deref())?;

        println!(
            "{}",
            colors::info(&format!("Prompts adapted for {}.", lang))
        );
    }

    println!();
    println!("{}", colors::header("Next steps:"));
    println!("  {}", colors::dim("1. Review and customize AGENTS.md"));
    println!(
        "  {}",
        colors::dim("2. Create your first PRD: `mr new my-feature`")
    );
    println!("  {}", colors::dim("3. Run a task: `mr run`"));

    Ok(())
}

/// Adapts prompts and templates for a specific programming language.
fn adapt_language(
    root: &std::path::Path,
    lang: init::Language,
    runner_name: &str,
    model: Option<&str>,
) -> Result<()> {
    // Special case for mock runner - skip adaptation
    if runner_name == "mock" {
        tracing::warn!("Using mock runner for language adaptation - no changes will be made");
        return Ok(());
    }

    // Select runner based on name.
    let runner = create_runner(runner_name, model.map(|s| s.to_string()))?;

    // Build the language adaptation prompt.
    let template = prompt::load_prompt_with_fallback(root, prompt::PromptKind::AdaptLanguage);

    let mut ctx = prompt::PlaceholderContext::new();
    ctx.insert("language", lang.to_string());

    // Add build commands as a list.
    let build_commands: Vec<std::collections::HashMap<String, String>> = lang
        .build_commands()
        .iter()
        .map(|cmd| {
            [("command".to_string(), cmd.to_string())]
                .into_iter()
                .collect()
        })
        .collect();

    ctx.insert(
        "build_commands",
        prompt::PlaceholderValue::List(build_commands),
    );

    let prompt_text = prompt::expand_placeholders(&template, &ctx);

    // Execute the runner with the adaptation prompt.
    let output = runner
        .execute(&prompt_text, root)
        .map_err(|e| anyhow::anyhow!("Runner failed during language adaptation: {e}"))?;

    if !output.success {
        anyhow::bail!("Language adaptation failed: {}", output.text);
    }

    tracing::debug!(
        output_len = output.text.len(),
        "Language adaptation completed"
    );

    Ok(())
}

/// Runs the `mr bootstrap` command.
fn cmd_bootstrap(runner_name: &str, language: Option<&str>, cli_model: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Show dev container warning for safety.
    devcontainer::show_dev_container_warning();

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Detect or parse language.
    let lang = match language {
        Some(l) => l
            .parse::<init::Language>()
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        None => {
            // Auto-detect language from project files.
            init::detect_language(&cwd).unwrap_or(init::Language::Rust)
        }
    };

    tracing::info!(language = %lang, "Detected/specified language");

    // Select runner based on name.
    let runner = create_runner(runner_name, model.clone())?;

    let config = bootstrap::BootstrapConfig::new(&cwd);

    println!("{}", colors::info("Bootstrapping repository..."));
    println!("{}", colors::info(&format!("Detected language: {}", lang)));
    println!();

    let result = bootstrap::bootstrap(&config, runner.as_ref())?;

    println!();

    if result.initialized {
        println!("{}", colors::info("Initialized .mr/ structure."));
    }

    if result.plan_generated {
        println!("{}", colors::info("Bootstrap plan generated."));
    }

    if result.prds_generated {
        println!(
            "{}",
            colors::info(&format!("Generated {} PRD(s).", result.prds_created))
        );
    }

    // Adapt prompts/templates for non-Rust languages after bootstrap.
    if lang != init::Language::Rust {
        println!();
        println!(
            "{}",
            colors::info(&format!("Adapting prompts and templates for {}...", lang))
        );

        adapt_language(&cwd, lang, runner_name, model.as_deref())?;

        println!(
            "{}",
            colors::info(&format!("Prompts adapted for {}.", lang))
        );
    }

    println!();
    println!("{}", colors::success("Bootstrap complete!"));
    println!();
    println!("{}", colors::header("Next steps:"));
    println!("  {}", colors::dim("1. Review generated PRDs in .mr/prds/"));
    println!("  {}", colors::dim("2. Check .mr/PRDS.md for the index"));
    println!(
        "  {}",
        colors::dim("3. Run `mr status` to see task summary")
    );
    println!(
        "  {}",
        colors::dim("4. Run `mr run` to start executing tasks")
    );

    Ok(())
}

/// Runs the `mr restore` command.
fn cmd_restore() -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    println!("{}", colors::info("Restoring prompts and templates..."));

    let mr_dir = cwd.join(".mr");
    let prompts_dir = mr_dir.join("prompts");
    let templates_dir = mr_dir.join("templates");

    // Delete existing directories if they exist.
    if prompts_dir.exists() {
        std::fs::remove_dir_all(&prompts_dir)
            .with_context(|| format!("Failed to remove directory: {}", prompts_dir.display()))?;
        tracing::debug!(path = %prompts_dir.display(), "Removed prompts directory");
    }

    if templates_dir.exists() {
        std::fs::remove_dir_all(&templates_dir)
            .with_context(|| format!("Failed to remove directory: {}", templates_dir.display()))?;
        tracing::debug!(path = %templates_dir.display(), "Removed templates directory");
    }

    println!(
        "{}",
        colors::success("✓ Deleted existing prompts and templates")
    );

    // Recreate prompts and templates with built-in defaults.
    let result = init::init_prompts_and_templates(&cwd)?;

    println!(
        "{}",
        colors::success(&format!(
            "✓ Restored {} prompt and template files",
            result.files_created
        ))
    );

    Ok(())
}

/// Runs the `mr new` command.
fn cmd_prd_new(
    slug: &str,
    runner_name: &str,
    cli_model: Option<&str>,
    context: Option<&str>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Show dev container warning for safety.
    devcontainer::show_dev_container_warning();

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    let config = prd_new::PrdNewConfig {
        root: &cwd,
        slug,
        description: None,
        context,
    };

    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let result = prd_new::create_prd(&config, runner.as_ref(), &mut stdin_lock, &mut stdout_lock)?;

    println!();
    println!("{}", colors::success("PRD created successfully!"));
    println!("  {}", colors::dim(&format!("ID: {}", result.prd.id())));
    println!(
        "  {}",
        colors::dim(&format!("Title: {}", result.prd.title()))
    );
    println!(
        "  {}",
        colors::dim(&format!("Path: {}", result.path.display()))
    );
    println!(
        "  {}",
        colors::dim(&format!("Q/A Rounds: {}", result.rounds))
    );
    println!(
        "  {}",
        colors::dim(&format!("Questions answered: {}", result.qa_history.len()))
    );

    Ok(())
}

/// Runs the `mr edit` command.
fn cmd_prd_edit(
    prd_id: &str,
    request: &str,
    runner_name: &str,
    cli_model: Option<&str>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    let config = prd_edit::PrdEditConfig {
        root: &cwd,
        prd_id,
        request,
    };

    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let result = prd_edit::edit_prd(&config, runner.as_ref(), &mut stdin_lock, &mut stdout_lock)?;

    println!();
    println!("{}", colors::success("PRD edited successfully!"));
    println!("  {}", colors::dim(&format!("ID: {}", result.prd.id())));
    println!(
        "  {}",
        colors::dim(&format!("Title: {}", result.prd.title()))
    );
    println!(
        "  {}",
        colors::dim(&format!("Path: {}", result.path.display()))
    );
    println!(
        "  {}",
        colors::dim(&format!("Q/A Rounds: {}", result.rounds))
    );

    if !result.qa_history.is_empty() {
        println!(
            "  {}",
            colors::dim(&format!("Questions answered: {}", result.qa_history.len()))
        );
    }

    Ok(())
}

/// Runs the `mr constitution edit` command.
fn cmd_constitution_edit(request: &str, runner_name: &str, cli_model: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    let config = constitution_edit::ConstitutionEditConfig {
        root: &cwd,
        request,
    };

    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let result = constitution_edit::edit_constitution(
        &config,
        runner.as_ref(),
        &mut stdin_lock,
        &mut stdout_lock,
    )?;

    println!();
    println!("{}", colors::success("Constitution edited successfully!"));
    println!(
        "  {}",
        colors::dim(&format!("Path: {}", result.path.display()))
    );
    println!(
        "  {}",
        colors::dim(&format!("Q/A Rounds: {}", result.rounds))
    );

    if !result.qa_history.is_empty() {
        println!(
            "  {}",
            colors::dim(&format!("Questions answered: {}", result.qa_history.len()))
        );
    }

    Ok(())
}

/// Runs the `mr list` command.
fn cmd_prd_list() -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Regenerate the index file.
    prd::generate_index_from_root(&cwd)?;

    let prds = prd::scan_prd_summaries(&cwd)?;

    if prds.is_empty() {
        println!("{}", colors::info("No PRDs found."));
        println!();
        println!(
            "{}",
            colors::dim("Create your first PRD with: `mr new my-feature`")
        );
        return Ok(());
    }

    // Group by status.
    let active: Vec<_> = prds
        .iter()
        .filter(|p| p.status == prd::PrdStatus::Active)
        .collect();

    let draft: Vec<_> = prds
        .iter()
        .filter(|p| p.status == prd::PrdStatus::Draft)
        .collect();

    let done: Vec<_> = prds
        .iter()
        .filter(|p| p.status == prd::PrdStatus::Done)
        .collect();

    let parked: Vec<_> = prds
        .iter()
        .filter(|p| p.status == prd::PrdStatus::Parked)
        .collect();

    println!("{}", colors::header("PRDs:"));
    println!();

    let format_prd = |prd_summary: &prd::PrdSummary| {
        // Task status with emoji.
        let task_status = if prd_summary.total_tasks > 0 {
            let emoji = if prd_summary.completed_tasks == prd_summary.total_tasks {
                "✅"
            } else {
                "📋"
            };
            format!(
                "{} {}/{}",
                emoji, prd_summary.completed_tasks, prd_summary.total_tasks
            )
        } else {
            String::new()
        };

        // UAT status with emoji.
        let uat_status = if prd_summary.total_uats > 0 {
            let emoji = if prd_summary.verified_uats == prd_summary.total_uats {
                "🧪"
            } else {
                "⚠️"
            };
            format!(
                "{} {}/{}",
                emoji, prd_summary.verified_uats, prd_summary.total_uats
            )
        } else {
            String::new()
        };

        // Combine status parts.
        let status_parts: Vec<&str> = [task_status.as_str(), uat_status.as_str()]
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect();

        let status_str = if status_parts.is_empty() {
            String::new()
        } else {
            format!(" [{}]", status_parts.join(" | "))
        };

        format!("{} - {}{}", prd_summary.id, prd_summary.title, status_str)
    };

    if !active.is_empty() {
        println!("  {}", colors::header("Active:"));

        for prd_summary in active {
            println!("    {}", format_prd(prd_summary));
        }

        println!();
    }

    if !draft.is_empty() {
        println!("  {}", colors::header("Draft:"));

        for prd_summary in draft {
            println!("    {}", format_prd(prd_summary));
        }

        println!();
    }

    if !done.is_empty() {
        println!("  {}", colors::header("Done:"));

        for prd_summary in done {
            println!("    {}", format_prd(prd_summary));
        }

        println!();
    }

    if !parked.is_empty() {
        println!("  {}", colors::header("Parked:"));

        for prd_summary in parked {
            println!("    {}", format_prd(prd_summary));
        }

        println!();
    }

    Ok(())
}

/// Runs the `mr finalize` command.
fn cmd_prd_finalize(
    prd_id: &str,
    runner_name: &str,
    cli_model: Option<&str>,
    stream: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    let config = prd_finalize::PrdFinalizeConfig {
        root: &cwd,
        prd_id,
        stream,
    };

    let result = prd_finalize::finalize_prd(&config, runner.as_ref())?;

    // Output summary report to stdout.
    println!();
    println!(
        "{}",
        colors::info("═══════════════════════════════════════════════════════════════")
    );
    println!(
        "                    {}",
        colors::header("FINALIZATION SUMMARY")
    );
    println!(
        "{}",
        colors::info("═══════════════════════════════════════════════════════════════")
    );
    println!();
    print!("{}", result.summary_report);
    println!();
    println!(
        "{}",
        colors::info("───────────────────────────────────────────────────────────────")
    );
    println!(
        "  {}",
        colors::dim(&format!("PRD Path: {}", result.path.display()))
    );

    if result.changelog_created {
        println!(
            "  {}",
            colors::dim(&format!(
                "Changelog: Created at {}",
                result.changelog_path.display()
            ))
        );
    } else {
        println!(
            "  {}",
            colors::dim(&format!("Changelog: {}", result.changelog_path.display()))
        );
    }

    println!("  {}", colors::dim("Summary Report: Appended to PRD"));
    println!("  {}", colors::dim("PRD Status: Updated to done"));
    println!("  {}", colors::dim("Index: PRDS.md regenerated"));
    println!(
        "{}",
        colors::info("═══════════════════════════════════════════════════════════════")
    );

    Ok(())
}

/// Normalizes a PRD identifier.
///
/// Accepts either a full PRD ID like "PRD-0005" or just a number like "5" or "13".
/// Returns the normalized form "PRD-NNNN".
fn normalize_prd_id(input: &str) -> String {
    let trimmed = input.trim();

    // If it already looks like a PRD ID, return as-is.
    if trimmed.starts_with("PRD-") {
        return trimmed.to_string();
    }

    // Try to parse as a number and format as PRD-NNNN.
    if let Ok(num) = trimmed.parse::<u32>() {
        return format!("PRD-{num:04}");
    }

    // Fall back to the original input.
    trimmed.to_string()
}

/// Runs the `mr run` command.
fn cmd_run(
    prd_id: Option<&str>,
    runner_name: &str,
    one: bool,
    cli_model: Option<&str>,
    stream: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Show dev container warning for safety.
    devcontainer::show_dev_container_warning();

    // Normalize PRD ID if provided (e.g., "5" -> "PRD-0005").
    let normalized_prd_id = prd_id.map(normalize_prd_id);

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    // If no PRD ID was provided, ask the runner to pick one.
    let active_prd_id = if let Some(id) = normalized_prd_id {
        id
    } else {
        // Ask runner to pick the PRD once, then use it for all task executions.
        run::pick_prd_via_runner(&cwd, runner.as_ref(), stream)?.ok_or_else(|| {
            anyhow::anyhow!(
                "No active PRD with incomplete tasks found. Create a PRD with `mr new`."
            )
        })?
    };

    let mut tasks_completed = 0;
    let mut last_failed = false;

    loop {
        let config = run::RunConfig {
            root: &cwd,
            prd_id: Some(&active_prd_id),
            stream,
        };

        let result = match run::run_task(&config, runner.as_ref()) {
            Ok(result) => result,
            Err(e) => {
                // Check if it's the "no active PRD" error, which means we're done.
                let err_msg = e.to_string();
                if err_msg.contains("No active PRD") || err_msg.contains("no incomplete tasks") {
                    if tasks_completed == 0 {
                        // No tasks were run, propagate original error.
                        return Err(e);
                    }

                    // We completed some tasks, exit gracefully.
                    break;
                }

                return Err(e);
            }
        };

        match result {
            run::RunResult::TaskExecuted {
                prd_id,
                task_id,
                task_title,
                prd_path,
                runner_success,
                output_summary,
                usage,
            } => {
                tasks_completed += 1;

                println!();

                if runner_success {
                    println!(
                        "{}",
                        colors::success(&format!("Task {} completed successfully!", task_id))
                    );
                } else {
                    println!(
                        "{}",
                        colors::error(&format!("Task {} did not complete successfully.", task_id))
                    );
                    last_failed = true;
                }

                println!();
                println!(
                    "  {}",
                    colors::dim(&format!("PRD: {} ({})", prd_id, prd_path.display()))
                );
                println!(
                    "  {}",
                    colors::dim(&format!("Task: {} — {}", task_id, task_title))
                );
                println!();

                if !output_summary.is_empty() {
                    println!("{}", colors::header("Runner output:"));
                    println!("{}", output_summary);
                }

                // Display usage information if available.
                if let Some(usage_info) = usage
                    && usage_info.has_data()
                {
                    println!();
                    println!("{}", colors::dim("Token usage:"));

                    if let Some(input) = usage_info.input_tokens {
                        print!("{}", colors::dim(&format!("  Input: {}", input)));
                    }

                    if let Some(output) = usage_info.output_tokens {
                        if usage_info.input_tokens.is_some() {
                            print!("{}", colors::dim(", "));
                        } else {
                            print!("{}", colors::dim("  "));
                        }
                        print!("{}", colors::dim(&format!("Output: {}", output)));
                    }

                    if let Some(total) = usage_info.total_tokens {
                        if usage_info.input_tokens.is_some() || usage_info.output_tokens.is_some() {
                            print!("{}", colors::dim(", "));
                        } else {
                            print!("{}", colors::dim("  "));
                        }
                        print!("{}", colors::dim(&format!("Total: {}", total)));
                    }

                    println!(); // Newline after usage info.
                }

                // Exit if --one flag is set or if the task failed.
                if one || last_failed {
                    break;
                }

                println!("---");
                println!("{}", colors::info("Continuing to next task..."));
            }

            run::RunResult::NeedsUatVerification {
                prd_id,
                prd_path,
                unverified_count,
            } => {
                println!();
                println!(
                    "{}",
                    colors::warning(&format!(
                        "All tasks done for {} but {} UAT(s) need verification.",
                        prd_id, unverified_count
                    ))
                );
                println!("  {}", colors::dim(&format!("PRD: {}", prd_path.display())));
                println!();
                println!("{}", colors::info("Starting UAT verification loop..."));
                println!();

                // Run the UAT verification loop.
                let uat_config = run::UatVerificationConfig {
                    root: &cwd,
                    prd_id: &prd_id,
                    stream,
                    max_iterations: None, // Use PRD config or default.
                };

                match run::run_uat_verification_loop(&uat_config, runner.as_ref()) {
                    Ok(result) => {
                        println!();
                        println!("{}", colors::info("UAT verification loop completed:"));
                        println!(
                            "  {}",
                            colors::dim(&format!("Verified: {}", result.verified_count))
                        );
                        println!(
                            "  {}",
                            colors::dim(&format!("Opted out: {}", result.opted_out_count))
                        );
                        println!(
                            "  {}",
                            colors::dim(&format!("Iterations: {}", result.iterations))
                        );

                        if result.hit_max_iterations {
                            println!("  {}", colors::warning("Hit max iterations limit."));
                        }

                        if result.remaining_unverified > 0 {
                            println!(
                                "  {}",
                                colors::dim(&format!(
                                    "Remaining unverified: {}",
                                    result.remaining_unverified
                                ))
                            );
                            println!();
                            println!(
                                "{}",
                                colors::dim("Run `mr run` again to continue verification.")
                            );
                        } else {
                            println!();
                            println!(
                                "{}",
                                colors::info("All UATs verified or opted out. PRD is complete!")
                            );
                        }
                    }
                    Err(e) => {
                        eprintln!(
                            "{}",
                            colors::error(&format!("UAT verification loop failed: {e}"))
                        );
                        return Err(e);
                    }
                }

                break;
            }

            run::RunResult::PrdComplete { prd_id, prd_path } => {
                println!();
                println!(
                    "{}",
                    colors::success(&format!("PRD {} is complete!", prd_id))
                );
                println!("  {}", colors::dim("All tasks done, all UATs verified."));
                println!("  {}", colors::dim(&format!("PRD: {}", prd_path.display())));
                break;
            }
        }
    }

    if tasks_completed > 1 {
        println!("---");
        println!(
            "{}",
            colors::info(&format!("Completed {} tasks total.", tasks_completed))
        );
    }

    Ok(())
}

/// Runs the `mr devcontainer generate` command.
fn cmd_devcontainer_generate(runner_name: &str, cli_model: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Show dev container warning for safety.
    devcontainer::show_dev_container_warning();

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Detect language from project files.
    let lang = init::detect_language(&cwd).unwrap_or(init::Language::Rust);
    tracing::info!(language = %lang, "Detected language");

    // Select runner based on name.
    let runner = create_runner(runner_name, model.clone())?;

    println!("{}", colors::info("Analyzing repository..."));
    println!("{}", colors::info(&format!("Detected language: {}", lang)));
    println!();

    // Call the testable implementation.
    generate_devcontainer_config(&cwd, lang, runner)?;

    println!("{}", colors::success("Dev container config generated!"));
    println!(
        "  {}",
        colors::dim(&format!(
            "Created: {}",
            cwd.join(".devcontainer/devcontainer.json").display()
        ))
    );
    println!();
    println!("{}", colors::header("Next steps:"));
    println!(
        "  {}",
        colors::dim("1. Review .devcontainer/devcontainer.json")
    );
    println!(
        "  {}",
        colors::dim(
            "2. Reopen project in dev container (VSCode: Cmd+Shift+P → 'Reopen in Container')"
        )
    );
    println!(
        "  {}",
        colors::dim("3. Or use GitHub Codespaces for cloud-based development")
    );

    Ok(())
}

/// Core implementation for dev container generation (testable).
///
/// This function contains the main logic for generating a devcontainer.json file,
/// separated from CLI handling for easier testing.
fn generate_devcontainer_config(
    root: &Path,
    lang: init::Language,
    runner: Box<dyn runner::Runner>,
) -> Result<()> {
    // Analyze repository for dev container context.
    let analysis = analyze_repo_for_devcontainer(root, lang)?;

    tracing::debug!("Repository analysis complete");

    // Load the devcontainer generation prompt.
    use crate::prompt::{
        PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
        load_prompt_with_fallback,
    };

    let prompt_text = load_prompt_with_fallback(root, PromptKind::DevcontainerGenerate);

    // Build placeholder context.
    let mut context = PlaceholderContext::new();
    context.insert("language", PlaceholderValue::String(lang.to_string()));
    context.insert("analysis", PlaceholderValue::String(analysis));

    let expanded_prompt = expand_placeholders(&prompt_text, &context);

    tracing::debug!("Invoking runner for devcontainer generation");

    // Invoke runner to generate devcontainer.json content.
    let result = runner.execute(&expanded_prompt, root)?;

    if !result.success {
        eprintln!(
            "{}",
            colors::warning("Runner encountered an issue. Please review the generated file.")
        );
    }

    tracing::debug!("Runner completed devcontainer generation");

    Ok(())
}

/// Analyzes the repository to gather context for dev container generation.
///
/// Returns a string containing:
/// - Detected language and frameworks
/// - Development tools found in git history
/// - Tools referenced in PRDs
/// - Current dependencies from manifest files
fn analyze_repo_for_devcontainer(root: &Path, lang: init::Language) -> Result<String> {
    use std::process::Command;

    let mut analysis = String::new();

    // Language and typical tools.
    analysis.push_str(&format!("Language: {}\n", lang));
    analysis.push_str(&format!(
        "Typical build commands: {:?}\n\n",
        lang.build_commands()
    ));

    // Check for common tools in the project.
    let tools = [
        ("Cargo.toml", "Rust (cargo)"),
        ("Makefile.toml", "cargo-make"),
        ("package.json", "Node.js (npm/yarn)"),
        ("requirements.txt", "Python (pip)"),
        ("go.mod", "Go modules"),
        (".github/workflows", "GitHub Actions"),
    ];

    analysis.push_str("Project files found:\n");
    let found_tools: Vec<_> = tools
        .iter()
        .filter(|(file, _)| root.join(file).exists())
        .map(|(_, desc)| format!("- {}\n", desc))
        .collect();
    analysis.push_str(&found_tools.join(""));
    analysis.push('\n');

    // Check git log for recently added tools (last 50 commits).
    if let Ok(output) = Command::new("git")
        .args([
            "log",
            "--all",
            "--oneline",
            "--no-merges",
            "-50",
            "--pretty=format:%s",
        ])
        .current_dir(root)
        .output()
        && output.status.success()
    {
        let log = String::from_utf8_lossy(&output.stdout);
        analysis.push_str("Recent commit messages (last 50):\n");
        analysis.push_str(&log);
        analysis.push_str("\n\n");
    }

    // Scan PRDs for tool references.
    let mr_dir = root.join(".mr");
    if mr_dir.exists() {
        let prds_dir = mr_dir.join("prds");
        if prds_dir.exists() {
            analysis.push_str("PRDs directory exists - tools may be referenced in PRDs.\n");
        }
    }

    Ok(analysis)
}

/// Runs the `mr status` command.
fn cmd_status() -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    let report = status::get_status(&cwd)?;
    let output = status::format_status(&report);

    print!("{output}");

    Ok(())
}

/// Runs the `mr suggest` command.
fn cmd_suggest(runner_name: &str, cli_model: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    // Run suggest logic.
    suggest::suggest(&cwd, runner.as_ref())?;

    Ok(())
}

/// Runs the `mr reindex` command.
fn cmd_reindex(runner_name: &str, cli_model: Option<&str>, stream: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    // Run reindex.
    let result = reindex::reindex(&cwd, runner.as_ref(), stream)?;

    println!();
    println!("{}", colors::success("Reindex complete!"));
    println!(
        "  {}",
        colors::dim(&format!("PRDs indexed: {}", result.prds_indexed))
    );
    println!(
        "  {}",
        colors::dim(&format!("Links verified: {}", result.links_verified))
    );
    println!(
        "  {}",
        colors::dim(&format!("Links fixed: {}", result.links_fixed))
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_args_parse_no_command() {
        let args = Args::try_parse_from(["mr"]).unwrap();
        assert!(args.command.is_none());
    }

    #[test]
    fn test_args_parse_init() {
        let args = Args::try_parse_from(["mr", "init"]).unwrap();
        assert!(matches!(args.command, Some(Command::Init { .. })));
    }

    #[test]
    fn test_args_parse_init_with_language() {
        let args = Args::try_parse_from(["mr", "init", "--language", "python"]).unwrap();
        if let Some(Command::Init {
            language,
            runner,
            model,
        }) = args.command
        {
            assert_eq!(language, Some("python".to_string()));
            assert_eq!(runner, "copilot");
            assert!(model.is_none());
        } else {
            panic!("Expected Init command");
        }
    }

    #[test]
    fn test_args_parse_status() {
        let args = Args::try_parse_from(["mr", "status"]).unwrap();
        assert!(matches!(args.command, Some(Command::Status)));
    }

    #[test]
    fn test_args_parse_run_with_runner() {
        let args = Args::try_parse_from(["mr", "run", "--runner", "mock"]).unwrap();
        if let Some(Command::Run { runner, model, .. }) = args.command {
            assert_eq!(runner, "mock");
            assert!(model.is_none());
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_parse_run_with_model() {
        let args =
            Args::try_parse_from(["mr", "run", "--model", "claude-sonnet-4-20250514"]).unwrap();
        if let Some(Command::Run { runner, model, .. }) = args.command {
            assert_eq!(runner, "copilot");
            assert_eq!(model, Some("claude-sonnet-4-20250514".to_string()));
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_parse_run_with_stream() {
        let args = Args::try_parse_from(["mr", "run", "--stream"]).unwrap();
        if let Some(Command::Run { stream, .. }) = args.command {
            assert!(stream);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_parse_run_default_stream_off() {
        let args = Args::try_parse_from(["mr", "run"]).unwrap();
        if let Some(Command::Run { stream, .. }) = args.command {
            assert!(!stream);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_parse_prd_new() {
        let args = Args::try_parse_from(["mr", "new", "my-feature"]).unwrap();
        if let Some(Command::New {
            slug,
            runner,
            model,
            context,
        }) = args.command
        {
            assert_eq!(slug, "my-feature");
            assert_eq!(runner, "copilot");
            assert!(model.is_none());
            assert!(context.is_none());
        } else {
            panic!("Expected New command");
        }
    }

    #[test]
    fn test_args_parse_prd_new_with_model() {
        let args = Args::try_parse_from(["mr", "new", "my-feature", "--model", "gpt-4o"]).unwrap();
        if let Some(Command::New {
            slug,
            runner,
            model,
            context,
        }) = args.command
        {
            assert_eq!(slug, "my-feature");
            assert_eq!(runner, "copilot");
            assert_eq!(model, Some("gpt-4o".to_string()));
            assert!(context.is_none());
        } else {
            panic!("Expected New command");
        }
    }

    #[test]
    fn test_args_parse_bootstrap() {
        let args = Args::try_parse_from(["mr", "bootstrap", "--runner", "mock"]).unwrap();
        if let Some(Command::Bootstrap {
            runner,
            language,
            model,
        }) = args.command
        {
            assert_eq!(runner, "mock");
            assert!(language.is_none());
            assert!(model.is_none());
        } else {
            panic!("Expected Bootstrap command");
        }
    }

    #[test]
    fn test_args_parse_bootstrap_with_language() {
        let args =
            Args::try_parse_from(["mr", "bootstrap", "--runner", "mock", "--language", "node"])
                .unwrap();
        if let Some(Command::Bootstrap {
            runner,
            language,
            model,
        }) = args.command
        {
            assert_eq!(runner, "mock");
            assert_eq!(language, Some("node".to_string()));
            assert!(model.is_none());
        } else {
            panic!("Expected Bootstrap command");
        }
    }

    #[test]
    fn test_args_parse_bootstrap_with_model() {
        let args = Args::try_parse_from([
            "mr",
            "bootstrap",
            "--runner",
            "copilot",
            "--model",
            "claude-opus-4",
        ])
        .unwrap();
        if let Some(Command::Bootstrap {
            runner,
            language,
            model,
        }) = args.command
        {
            assert_eq!(runner, "copilot");
            assert!(language.is_none());
            assert_eq!(model, Some("claude-opus-4".to_string()));
        } else {
            panic!("Expected Bootstrap command");
        }
    }

    #[test]
    fn test_verbose_flag() {
        let args = Args::try_parse_from(["mr", "-v", "status"]).unwrap();
        assert!(args.verbose);
        assert!(!args.quiet);
    }

    #[test]
    fn test_quiet_flag() {
        let args = Args::try_parse_from(["mr", "-q", "init"]).unwrap();
        assert!(args.quiet);
        assert!(!args.verbose);
    }

    #[test]
    fn test_args_parse_prd_finalize() {
        let args = Args::try_parse_from(["mr", "finalize", "PRD-0001"]).unwrap();
        if let Some(Command::Finalize {
            prd_id,
            runner,
            model,
            stream,
        }) = args.command
        {
            assert_eq!(prd_id, "PRD-0001");
            assert_eq!(runner, "copilot");
            assert!(model.is_none());
            assert!(!stream);
        } else {
            panic!("Expected Finalize command");
        }
    }

    #[test]
    fn test_args_parse_prd_finalize_with_options() {
        let args = Args::try_parse_from([
            "mr", "finalize", "PRD-0002", "--runner", "mock", "--model", "gpt-4o", "--stream",
        ])
        .unwrap();
        if let Some(Command::Finalize {
            prd_id,
            runner,
            model,
            stream,
        }) = args.command
        {
            assert_eq!(prd_id, "PRD-0002");
            assert_eq!(runner, "mock");
            assert_eq!(model, Some("gpt-4o".to_string()));
            assert!(stream);
        } else {
            panic!("Expected Prd Finalize command");
        }
    }

    #[test]
    fn test_normalize_prd_id_full_id() {
        assert_eq!(normalize_prd_id("PRD-0001"), "PRD-0001");
        assert_eq!(normalize_prd_id("PRD-0042"), "PRD-0042");
        assert_eq!(normalize_prd_id("  PRD-0005  "), "PRD-0005");
    }

    #[test]
    fn test_normalize_prd_id_number() {
        assert_eq!(normalize_prd_id("5"), "PRD-0005");
        assert_eq!(normalize_prd_id("13"), "PRD-0013");
        assert_eq!(normalize_prd_id("1"), "PRD-0001");
        assert_eq!(normalize_prd_id("9999"), "PRD-9999");
        assert_eq!(normalize_prd_id("  42  "), "PRD-0042");
    }

    #[test]
    fn test_normalize_prd_id_fallback() {
        // Non-numeric, non-PRD strings fall back to original.
        assert_eq!(normalize_prd_id("my-feature"), "my-feature");
        assert_eq!(normalize_prd_id("abc"), "abc");
    }

    #[test]
    fn test_args_parse_run_with_positional_prd() {
        // UAT-001: Verify run command accepts optional positional PRD argument
        let args = Args::try_parse_from(["mr", "run", "PRD-0001"]).unwrap();
        if let Some(Command::Run { prd, .. }) = args.command {
            assert_eq!(prd, Some("PRD-0001".to_string()));
        } else {
            panic!("Expected Run command with positional PRD argument");
        }
    }

    #[test]
    fn test_args_parse_run_without_positional_prd() {
        // Verify run command works without positional PRD argument (interactive mode)
        let args = Args::try_parse_from(["mr", "run"]).unwrap();
        if let Some(Command::Run { prd, .. }) = args.command {
            assert!(prd.is_none());
        } else {
            panic!("Expected Run command without positional PRD argument");
        }
    }

    #[test]
    fn test_args_parse_list() {
        // Verify list command works at top level
        let args = Args::try_parse_from(["mr", "list"]).unwrap();
        assert!(matches!(args.command, Some(Command::List)));
    }

    #[test]
    fn test_args_parse_new() {
        // Verify new command works at top level with slug argument
        let args = Args::try_parse_from(["mr", "new", "test-slug"]).unwrap();
        if let Some(Command::New { slug, .. }) = args.command {
            assert_eq!(slug, "test-slug");
        } else {
            panic!("Expected New command with slug 'test-slug'");
        }
    }

    #[test]
    fn test_args_parse_edit() {
        // Verify edit command works at top level with prd_id and request arguments
        let args = Args::try_parse_from(["mr", "edit", "PRD-0001", "test edit"]).unwrap();
        if let Some(Command::Edit {
            prd_id, request, ..
        }) = args.command
        {
            assert_eq!(prd_id, "PRD-0001");
            assert_eq!(request, "test edit");
        } else {
            panic!("Expected Edit command with prd_id 'PRD-0001' and request 'test edit'");
        }
    }

    #[test]
    fn test_args_parse_finalize() {
        // Verify finalize command works at top level with prd_id argument
        let args = Args::try_parse_from(["mr", "finalize", "PRD-0001"]).unwrap();
        if let Some(Command::Finalize { prd_id, .. }) = args.command {
            assert_eq!(prd_id, "PRD-0001");
        } else {
            panic!("Expected Finalize command with prd_id 'PRD-0001'");
        }
    }

    #[test]
    fn test_devcontainer_generate_with_mock_runner() {
        // UAT-001: Test devcontainer generate invokes runner with correct prompt
        use crate::runner::{MockRunner, RunnerOutput};
        use tempfile::TempDir;

        // Create temporary directory for testing.
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create .mr/prompts directory with devcontainer_generate.md prompt.
        let prompts_dir = temp_path.join(".mr/prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        let prompt_content = r#"# Dev Container Generation

Generate a devcontainer.json configuration for:
- Language: {{language}}
- Analysis: {{analysis}}

**Create the file `.devcontainer/devcontainer.json` directly** with the generated configuration.
"#;
        std::fs::write(prompts_dir.join("devcontainer_generate.md"), prompt_content).unwrap();

        // Mock response - LLM will create the file itself, so response doesn't matter.
        let mock_response = "File created successfully.";

        // Set up mock runner with successful response.
        let mock_runner = MockRunner::new(vec![RunnerOutput::success(mock_response)]);

        // Call the core generation function.
        let result =
            generate_devcontainer_config(temp_path, init::Language::Rust, Box::new(mock_runner));

        // Verify success - the LLM is responsible for creating the file, not microralph.
        assert!(result.is_ok(), "Generation should succeed: {:?}", result);
    }

    #[test]
    fn test_restore_fresh() {
        // Test scenario: Fresh restore on an initialized repository
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Initialize first.
        init::init(root).unwrap();

        // Verify files exist after init.
        assert!(root.join(".mr/prompts/init.md").exists());
        assert!(root.join(".mr/templates/prd.md").exists());

        // Save current directory and change to temp dir.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(root).unwrap();

        // Run restore.
        let result = cmd_restore();

        // Restore current directory.
        std::env::set_current_dir(&original_dir).unwrap();

        // Verify success.
        assert!(result.is_ok(), "Restore should succeed: {:?}", result);

        // Verify files still exist after restore.
        assert!(root.join(".mr/prompts/init.md").exists());
        assert!(root.join(".mr/templates/prd.md").exists());
        assert!(root.join(".mr/prompts/run_task.md").exists());
        assert!(root.join(".mr/prompts/suggest_generate.md").exists());
    }

    #[test]
    fn test_restore_after_customization() {
        // Test scenario: Restore after customizing prompt files
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Initialize first.
        init::init(root).unwrap();

        // Customize a prompt file.
        let init_prompt_path = root.join(".mr/prompts/init.md");
        let custom_content = "# CUSTOM INIT PROMPT\n\nThis is customized.";
        std::fs::write(&init_prompt_path, custom_content).unwrap();

        // Verify customization.
        let content_before = std::fs::read_to_string(&init_prompt_path).unwrap();
        assert_eq!(content_before, custom_content);

        // Save current directory and change to temp dir.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(root).unwrap();

        // Run restore.
        let result = cmd_restore();

        // Restore current directory.
        std::env::set_current_dir(&original_dir).unwrap();

        // Verify success.
        assert!(result.is_ok(), "Restore should succeed: {:?}", result);

        // Verify the file was restored to built-in default (not custom content).
        let content_after = std::fs::read_to_string(&init_prompt_path).unwrap();
        assert_ne!(
            content_after, custom_content,
            "Content should be restored to built-in default"
        );
        assert!(
            content_after.contains("microralph"),
            "Restored content should contain built-in text"
        );
    }

    #[test]
    fn test_restore_idempotency() {
        // Test scenario: Multiple restore operations produce the same result
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Initialize first.
        init::init(root).unwrap();

        // Save current directory and change to temp dir.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(root).unwrap();

        // Run restore first time.
        let result1 = cmd_restore();
        assert!(result1.is_ok(), "First restore should succeed");

        // Capture file contents after first restore.
        let init_content1 = std::fs::read_to_string(root.join(".mr/prompts/init.md")).unwrap();
        let prd_template1 = std::fs::read_to_string(root.join(".mr/templates/prd.md")).unwrap();

        // Run restore second time.
        let result2 = cmd_restore();
        assert!(result2.is_ok(), "Second restore should succeed");

        // Capture file contents after second restore.
        let init_content2 = std::fs::read_to_string(root.join(".mr/prompts/init.md")).unwrap();
        let prd_template2 = std::fs::read_to_string(root.join(".mr/templates/prd.md")).unwrap();

        // Run restore third time to ensure it still works.
        let result3 = cmd_restore();
        assert!(result3.is_ok(), "Third restore should succeed");

        // Restore current directory before cleanup.
        std::env::set_current_dir(&original_dir).unwrap();

        // Verify idempotency: contents should be identical.
        assert_eq!(
            init_content1, init_content2,
            "Init prompt should be identical after multiple restores"
        );
        assert_eq!(
            prd_template1, prd_template2,
            "PRD template should be identical after multiple restores"
        );
    }

    #[test]
    fn test_restore_fails_if_not_initialized() {
        // Test scenario: Restore should fail if .mr/ directory doesn't exist
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Don't initialize - leave directory empty.

        // Save current directory and change to temp dir.
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(root).unwrap();

        // Run restore.
        let result = cmd_restore();

        // Restore current directory.
        std::env::set_current_dir(&original_dir).unwrap();

        // Verify failure.
        assert!(result.is_err(), "Restore should fail if not initialized");
        assert!(
            result.unwrap_err().to_string().contains("not initialized"),
            "Error message should mention not initialized"
        );
    }
}
