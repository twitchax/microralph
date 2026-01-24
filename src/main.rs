use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod agents;
mod bootstrap;
mod changelog;
mod config;
mod init;
mod prd;
mod prd_edit;
mod prd_finalize;
mod prd_new;
mod prompt;
mod reindex;
mod run;
mod runner;
mod status;

use runner::Runner;

/// microralph (`mr`) — A tiny CLI for creating and executing PRDs with coding agents.
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
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
    /// Initialize a new repo with `.mr/` structure, templates, prompts, and starter AGENTS.md.
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

    /// Ingest an existing repo into PRDs: generate `.mr/PRDS.md` and starter PRDs.
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

    /// PRD management commands.
    Prd {
        #[command(subcommand)]
        prd_command: PrdCommand,
    },

    /// Run the next task from the active PRD.
    Run {
        /// Explicitly specify a PRD to run.
        #[arg(long)]
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

    /// Show status of PRDs and tasks.
    Status,

    /// Regenerate `.mr/PRDS.md` index and fix inter-PRD/code links in PRDs.
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
}

#[derive(Subcommand, Debug)]
enum PrdCommand {
    /// Create a new PRD via guided Q/A.
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

    /// Edit an existing PRD via runner-assisted modifications.
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

    /// Finalize a PRD after all tasks are complete.
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

    /// List all PRDs.
    List,
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
        Some(Command::Prd { prd_command }) => match prd_command {
            PrdCommand::New {
                slug,
                runner,
                model,
                context,
            } => {
                tracing::info!(slug = %slug, runner = %runner, "Creating new PRD...");
                cmd_prd_new(&slug, &runner, model.as_deref(), context.as_deref())?;
            }
            PrdCommand::Edit {
                prd_id,
                request,
                runner,
                model,
            } => {
                tracing::info!(prd_id = %prd_id, runner = %runner, "Editing PRD...");
                cmd_prd_edit(&prd_id, &request, &runner, model.as_deref())?;
            }
            PrdCommand::List => {
                tracing::info!("Listing PRDs...");
                cmd_prd_list()?;
            }
            PrdCommand::Finalize {
                prd_id,
                runner,
                model,
                stream,
            } => {
                tracing::info!(prd_id = %prd_id, runner = %runner, stream = %stream, "Finalizing PRD...");
                cmd_prd_finalize(&prd_id, &runner, model.as_deref(), stream)?;
            }
        },
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
                "microralph (`mr`) — A tiny CLI for creating and executing PRDs with coding agents."
            );
            println!();
            println!("Run `mr --help` for available commands.");
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

/// Runs the `mr init` command.
fn cmd_init(language: Option<&str>, runner_name: &str, cli_model: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if init::is_initialized(&cwd) {
        println!("microralph is already initialized in this directory.");
        println!("Run `mr status` to see PRD status.");
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

    println!("Initialized microralph!");
    println!();
    println!(
        "Created {} directories, {} files.",
        result.dirs_created, result.files_created
    );

    if !result.created_paths.is_empty() {
        println!();
        println!("Created files:");
        for path in &result.created_paths {
            println!("  - {path}");
        }
    }

    // Adapt prompts/templates for non-Rust languages.
    if lang != init::Language::Rust {
        println!();
        println!("Adapting prompts and templates for {}...", lang);

        // Load config for model (config file was just created).
        let cfg = config::Config::load_or_default(&cwd)?;
        let model = cfg.effective_model(cli_model);

        adapt_language(&cwd, lang, runner_name, model.as_deref())?;

        println!("Prompts adapted for {}.", lang);
    }

    println!();
    println!("Next steps:");
    println!("  1. Review and customize AGENTS.md");
    println!("  2. Create your first PRD: `mr prd new my-feature`");
    println!("  3. Run a task: `mr run`");

    Ok(())
}

/// Adapts prompts and templates for a specific programming language.
fn adapt_language(
    root: &std::path::Path,
    lang: init::Language,
    runner_name: &str,
    model: Option<&str>,
) -> Result<()> {
    // Select runner based on name.
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => {
            tracing::warn!("Using mock runner for language adaptation - no changes will be made");
            return Ok(());
        }
        "copilot" => {
            let copilot = runner::CopilotRunner::with_model(model.map(|s| s.to_string()));

            if !copilot.is_available() {
                anyhow::bail!(
                    "Copilot CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Box::new(copilot)
        }
        other => {
            anyhow::bail!("Unknown runner: {other}. Available: copilot, mock");
        }
    };

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
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::with_model(model.clone());

            if !copilot.is_available() {
                anyhow::bail!(
                    "Copilot CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Box::new(copilot)
        }
        other => {
            anyhow::bail!("Unknown runner: {other}. Available: copilot, mock");
        }
    };

    let config = bootstrap::BootstrapConfig::new(&cwd);

    println!("Bootstrapping repository...");
    println!("Detected language: {}", lang);
    println!();

    let result = bootstrap::bootstrap(&config, runner.as_ref())?;

    println!();

    if result.initialized {
        println!("Initialized .mr/ structure.");
    }

    if result.plan_generated {
        println!("Bootstrap plan generated.");
    }

    if result.prds_generated {
        println!("Generated {} PRD(s).", result.prds_created);
    }

    // Adapt prompts/templates for non-Rust languages after bootstrap.
    if lang != init::Language::Rust {
        println!();
        println!("Adapting prompts and templates for {}...", lang);

        adapt_language(&cwd, lang, runner_name, model.as_deref())?;

        println!("Prompts adapted for {}.", lang);
    }

    println!();
    println!("Bootstrap complete!");
    println!();
    println!("Next steps:");
    println!("  1. Review generated PRDs in .mr/prds/");
    println!("  2. Check .mr/PRDS.md for the index");
    println!("  3. Run `mr status` to see task summary");
    println!("  4. Run `mr run` to start executing tasks");

    Ok(())
}

/// Runs the `mr prd new` command.
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

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::with_model(model);

            if !copilot.is_available() {
                anyhow::bail!(
                    "Copilot CLI is not available. Install it with `npm install -g @anthropic-ai/copilot-cli` or use `--runner mock` for testing."
                );
            }

            Box::new(copilot)
        }
        other => {
            anyhow::bail!("Unknown runner: {other}. Available: copilot, mock");
        }
    };

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
    println!("PRD created successfully!");
    println!("  ID: {}", result.prd.id());
    println!("  Title: {}", result.prd.title());
    println!("  Path: {}", result.path.display());
    println!("  Q/A Rounds: {}", result.rounds);
    println!("  Questions answered: {}", result.qa_history.len());

    Ok(())
}

/// Runs the `mr prd edit` command.
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
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::with_model(model);

            if !copilot.is_available() {
                anyhow::bail!(
                    "Copilot CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Box::new(copilot)
        }
        other => {
            anyhow::bail!("Unknown runner: {other}. Available: copilot, mock");
        }
    };

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
    println!("PRD edited successfully!");
    println!("  ID: {}", result.prd.id());
    println!("  Title: {}", result.prd.title());
    println!("  Path: {}", result.path.display());
    println!("  Q/A Rounds: {}", result.rounds);

    if !result.qa_history.is_empty() {
        println!("  Questions answered: {}", result.qa_history.len());
    }

    Ok(())
}

/// Runs the `mr prd list` command.
fn cmd_prd_list() -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Regenerate the index file.
    prd::generate_index_from_root(&cwd)?;

    let prds = prd::scan_prd_summaries(&cwd)?;

    if prds.is_empty() {
        println!("No PRDs found.");
        println!();
        println!("Create your first PRD with: `mr prd new my-feature`");
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

    println!("PRDs:");
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
        println!("  Active:");

        for prd_summary in active {
            println!("    {}", format_prd(prd_summary));
        }

        println!();
    }

    if !draft.is_empty() {
        println!("  Draft:");

        for prd_summary in draft {
            println!("    {}", format_prd(prd_summary));
        }

        println!();
    }

    if !done.is_empty() {
        println!("  Done:");

        for prd_summary in done {
            println!("    {}", format_prd(prd_summary));
        }

        println!();
    }

    if !parked.is_empty() {
        println!("  Parked:");

        for prd_summary in parked {
            println!("    {}", format_prd(prd_summary));
        }

        println!();
    }

    Ok(())
}

/// Runs the `mr prd finalize` command.
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
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::with_model(model);

            if !copilot.is_available() {
                anyhow::bail!(
                    "Copilot CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Box::new(copilot)
        }
        other => {
            anyhow::bail!("Unknown runner: {other}. Available: copilot, mock");
        }
    };

    let config = prd_finalize::PrdFinalizeConfig {
        root: &cwd,
        prd_id,
        stream,
    };

    let result = prd_finalize::finalize_prd(&config, runner.as_ref())?;

    // Output summary report to stdout.
    println!();
    println!("═══════════════════════════════════════════════════════════════");
    println!("                    FINALIZATION SUMMARY");
    println!("═══════════════════════════════════════════════════════════════");
    println!();
    print!("{}", result.summary_report);
    println!();
    println!("───────────────────────────────────────────────────────────────");
    println!("  PRD Path: {}", result.path.display());

    if result.changelog_created {
        println!(
            "  Changelog: Created at {}",
            result.changelog_path.display()
        );
    } else {
        println!("  Changelog: {}", result.changelog_path.display());
    }

    println!("  Summary Report: Appended to PRD");
    println!("  PRD Status: Updated to done");
    println!("  Index: PRDS.md regenerated");
    println!("═══════════════════════════════════════════════════════════════");

    Ok(())
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

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::with_model(model);

            if !copilot.is_available() {
                anyhow::bail!(
                    "Copilot CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Box::new(copilot)
        }
        other => {
            anyhow::bail!("Unknown runner: {other}. Available: copilot, mock");
        }
    };

    let config = run::RunConfig {
        root: &cwd,
        prd_id,
        stream,
    };

    let mut tasks_completed = 0;
    let mut last_failed = false;

    loop {
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

        tasks_completed += 1;

        println!();

        if result.runner_success {
            println!("Task {} completed successfully!", result.task_id);
        } else {
            println!("Task {} did not complete successfully.", result.task_id);
            last_failed = true;
        }

        println!();
        println!("  PRD: {} ({})", result.prd_id, result.prd_path.display());
        println!("  Task: {} — {}", result.task_id, result.task_title);
        println!();

        if !result.output_summary.is_empty() {
            println!("Runner output:");
            println!("{}", result.output_summary);
        }

        // Exit if --one flag is set or if the task failed.
        if one || last_failed {
            break;
        }

        println!("---");
        println!("Continuing to next task...");
    }

    if tasks_completed > 1 {
        println!("---");
        println!("Completed {} tasks total.", tasks_completed);
    }

    Ok(())
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
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::with_model(model);
            Box::new(copilot)
        }
        other => anyhow::bail!("Unknown runner: {other}. Supported: copilot, mock"),
    };

    // Run reindex.
    let result = reindex::reindex(&cwd, runner.as_ref(), stream)?;

    println!();
    println!("Reindex complete!");
    println!("  PRDs indexed: {}", result.prds_indexed);
    println!("  Links verified: {}", result.links_verified);
    println!("  Links fixed: {}", result.links_fixed);

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
        let args = Args::try_parse_from(["mr", "prd", "new", "my-feature"]).unwrap();
        if let Some(Command::Prd {
            prd_command:
                PrdCommand::New {
                    slug,
                    runner,
                    model,
                    context,
                },
        }) = args.command
        {
            assert_eq!(slug, "my-feature");
            assert_eq!(runner, "copilot");
            assert!(model.is_none());
            assert!(context.is_none());
        } else {
            panic!("Expected Prd New command");
        }
    }

    #[test]
    fn test_args_parse_prd_new_with_model() {
        let args =
            Args::try_parse_from(["mr", "prd", "new", "my-feature", "--model", "gpt-4o"]).unwrap();
        if let Some(Command::Prd {
            prd_command:
                PrdCommand::New {
                    slug,
                    runner,
                    model,
                    context,
                },
        }) = args.command
        {
            assert_eq!(slug, "my-feature");
            assert_eq!(runner, "copilot");
            assert_eq!(model, Some("gpt-4o".to_string()));
            assert!(context.is_none());
        } else {
            panic!("Expected Prd New command");
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
        let args = Args::try_parse_from(["mr", "prd", "finalize", "PRD-0001"]).unwrap();
        if let Some(Command::Prd {
            prd_command:
                PrdCommand::Finalize {
                    prd_id,
                    runner,
                    model,
                    stream,
                },
        }) = args.command
        {
            assert_eq!(prd_id, "PRD-0001");
            assert_eq!(runner, "copilot");
            assert!(model.is_none());
            assert!(!stream);
        } else {
            panic!("Expected Prd Finalize command");
        }
    }

    #[test]
    fn test_args_parse_prd_finalize_with_options() {
        let args = Args::try_parse_from([
            "mr", "prd", "finalize", "PRD-0002", "--runner", "mock", "--model", "gpt-4o",
            "--stream",
        ])
        .unwrap();
        if let Some(Command::Prd {
            prd_command:
                PrdCommand::Finalize {
                    prd_id,
                    runner,
                    model,
                    stream,
                },
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
}
