// Deny unwrap_used in production code to ensure proper error handling.
// Test code and mock runner are allowed to use unwrap via #[cfg(test)] and module-level allows.
#![deny(unused)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::correctness)]
#![deny(clippy::complexity)]
#![deny(clippy::pedantic)]

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::Path;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod commands;
mod config;
mod prd;
mod prompt;
mod runner;
mod util;

use commands::{bootstrap, devcontainer, graph, init, refactor, reindex, run, status, suggest};

use runner::Runner;
use util::colors;

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

        /// The runner to use for language adaptation [copilot, claude, codex] (only needed for non-Rust languages).
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,
    },

    /// [0] Ingest an existing repo into PRDs: generate `.mr/PRDS.md` and starter PRDs.
    #[command(display_order = 2)]
    Bootstrap {
        /// The runner to use for bootstrapping [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Target programming language (rust, python, node, go, java).
        /// If unspecified, auto-detects from project files.
        #[arg(long)]
        language: Option<String>,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,

        /// Scaffold mode: skip git history analysis and create an initial PRD for bootstrapping.
        /// Default behavior (without this flag) reconstructs PRDs from git history.
        #[arg(long)]
        scaffold: bool,

        /// Stream runner output to stdout in real-time.
        #[arg(long)]
        stream: bool,
    },

    /// [0] Restore `.mr/prompts/`, `.mr/templates/`, `constitution.md`, and `config.toml` to built-in defaults.
    #[command(display_order = 3)]
    Restore,

    /// [1] Create a new PRD via guided Q/A.
    #[command(display_order = 4)]
    New {
        /// The slug for the new PRD (e.g., "add-user-auth").
        slug: String,

        /// The runner to use for the Q/A session [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
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

        /// Optional upfront context to guide the edit session.
        #[arg(long)]
        context: Option<String>,

        /// The runner to use for the edit session [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,
    },

    /// [2] Run the next task from the active PRD.
    #[command(display_order = 6)]
    Run {
        /// Optional PRD ID to run (e.g., "PRD-0001"). If omitted, runs the highest-priority active PRD.
        prd: Option<String>,

        /// The runner to use for task execution [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Run only one task and exit (default is to loop until all tasks are done).
        #[arg(long)]
        one: bool,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,

        /// Stream runner output to stdout in real-time.
        #[arg(long)]
        stream: bool,

        /// Do not instruct the agent to commit changes.
        /// When set, prompts say "Do NOT commit" instead of commit instructions.
        #[arg(long)]
        no_commit: bool,

        /// Prevent the agent from skipping UATs during verification.
        /// When set, the skip-UAT option is omitted from verification prompts.
        #[arg(long)]
        disallow_skip_uat: bool,

        /// Prevent the agent from adding new tasks during execution.
        /// When set, the add-task instructions are omitted from prompts.
        #[arg(long)]
        disallow_add_task: bool,
    },

    /// [3] Finalize a PRD after all tasks are complete.
    #[command(display_order = 7)]
    Finalize {
        /// The PRD ID to finalize (e.g., "PRD-0001").
        prd_id: String,

        /// The runner to use for finalization [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,

        /// Stream runner output to stdout in real-time.
        #[arg(long)]
        stream: bool,

        /// Do not instruct the agent to commit changes.
        /// When set, prompts say "Do NOT commit" instead of commit instructions.
        #[arg(long)]
        no_commit: bool,
    },

    /// [H] List all PRDs.
    #[command(display_order = 8)]
    List {
        /// Include done PRDs in the listing (hidden by default).
        #[arg(long)]
        done: bool,
    },

    /// [H] Show status of PRDs and tasks.
    #[command(display_order = 9)]
    Status,

    /// [H] Generate AI-driven PRD suggestions based on codebase analysis.
    #[command(display_order = 10)]
    Suggest {
        /// The runner to use for suggestion generation [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,
    },

    /// [H] Run AI-driven iterative refactoring to improve codebase quality.
    #[command(display_order = 11)]
    Refactor {
        /// Maximum number of refactor iterations (default: 3).
        #[arg(long, default_value = "3")]
        max: u32,

        /// Focus hint for the agent (e.g., "improve error handling", "reduce duplication").
        /// When provided, prioritized over constitution-based discovery.
        #[arg(long)]
        context: Option<String>,

        /// Constrain refactoring scope to a specific directory or file pattern.
        #[arg(long)]
        path: Option<String>,

        /// Preview suggested refactors without applying changes.
        #[arg(long)]
        dry_run: bool,

        /// Do not instruct the agent to commit changes.
        #[arg(long)]
        no_commit: bool,

        /// The runner to use for refactoring [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,

        /// Stream runner output to stdout in real-time.
        #[arg(long)]
        stream: bool,
    },

    /// [H] Visualize PRD dependency graph in various formats.
    #[command(display_order = 12)]
    Graph {
        #[command(subcommand)]
        command: GraphCommand,
    },

    /// [C] Dev container management commands.
    #[command(display_order = 13)]
    Devcontainer {
        #[command(subcommand)]
        command: DevcontainerCommand,
    },

    /// [C] Regenerate `.mr/PRDS.md` index and fix inter-PRD/code links in PRDs.
    #[command(display_order = 14)]
    Reindex {
        /// The runner to use for link verification/fixing [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,

        /// Stream runner output to stdout in real-time.
        #[arg(long)]
        stream: bool,
    },

    /// [C] Constitution management commands.
    #[command(display_order = 15)]
    Constitution {
        #[command(subcommand)]
        command: ConstitutionCommand,
    },
}

#[derive(Subcommand, Debug)]
enum DevcontainerCommand {
    /// Generate a dev container configuration from repository analysis.
    Generate {
        /// The runner to use for generation [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
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

        /// The runner to use for the edit session [copilot, claude, codex].
        #[arg(long, default_value = "copilot")]
        runner: String,

        /// Model to use with the runner (e.g., "claude-sonnet-4.5").
        #[arg(long)]
        model: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum GraphCommand {
    /// Render the PRD dependency graph as ASCII art.
    Ascii {
        /// Hide node titles (show only IDs).
        #[arg(long)]
        no_titles: bool,

        /// Maximum title length before truncation.
        #[arg(long, default_value = "40")]
        max_title_len: usize,
    },

    /// Render the PRD dependency graph as Mermaid flowchart syntax.
    Mermaid {
        /// Hide node titles (show only IDs).
        #[arg(long)]
        no_titles: bool,

        /// Maximum title length before truncation.
        #[arg(long, default_value = "40")]
        max_title_len: usize,

        /// Render graph left-to-right instead of top-to-bottom.
        #[arg(long)]
        lr: bool,
    },

    /// Render the PRD dependency graph as Graphviz DOT format.
    Dot {
        /// Hide node titles (show only IDs).
        #[arg(long)]
        no_titles: bool,

        /// Maximum title length before truncation.
        #[arg(long, default_value = "40")]
        max_title_len: usize,

        /// Render graph left-to-right instead of top-to-bottom.
        #[arg(long)]
        lr: bool,
    },
}

/// Arguments for the refactor command.
#[derive(Clone, Copy)]
struct CmdRefactorArgs<'a> {
    max: u32,
    context: Option<&'a str>,
    path: Option<&'a str>,
    dry_run: bool,
    no_commit: bool,
    runner_name: &'a str,
    cli_model: Option<&'a str>,
    stream: bool,
}

#[allow(clippy::too_many_lines)]
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
            scaffold,
            stream,
        }) => {
            tracing::info!(runner = %runner, language = ?language, scaffold = scaffold, stream = stream, "Bootstrapping repo...");
            cmd_bootstrap(
                &runner,
                language.as_deref(),
                model.as_deref(),
                scaffold,
                stream,
            )?;
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
            context,
            runner,
            model,
        }) => {
            let prd_id = normalize_prd_id(&prd_id);
            tracing::info!(prd_id = %prd_id, runner = %runner, "Editing PRD...");
            cmd_prd_edit(&prd_id, context.as_deref(), &runner, model.as_deref())?;
        }
        Some(Command::List { done }) => {
            tracing::info!(done = %done, "Listing PRDs...");
            cmd_prd_list(done)?;
        }
        Some(Command::Finalize {
            prd_id,
            runner,
            model,
            stream,
            no_commit,
        }) => {
            let prd_id = normalize_prd_id(&prd_id);
            tracing::info!(prd_id = %prd_id, runner = %runner, stream = %stream, no_commit = %no_commit, "Finalizing PRD...");
            cmd_prd_finalize(&prd_id, &runner, model.as_deref(), stream, no_commit)?;
        }
        Some(Command::Run {
            prd,
            runner,
            one,
            model,
            stream,
            no_commit,
            disallow_skip_uat,
            disallow_add_task,
        }) => {
            tracing::info!(prd = ?prd, runner = %runner, one = %one, stream = %stream, no_commit = %no_commit, disallow_skip_uat = %disallow_skip_uat, disallow_add_task = %disallow_add_task, "Running next task...");
            cmd_run(&CmdRunOpts {
                prd_id: prd.as_deref(),
                runner_name: &runner,
                one,
                cli_model: model.as_deref(),
                stream,
                cli_no_commit: no_commit,
                disallow_skip_uat,
                disallow_add_task,
            })?;
        }
        Some(Command::Status) => {
            tracing::info!("Showing status...");
            cmd_status()?;
        }
        Some(Command::Suggest { runner, model }) => {
            tracing::info!(runner = %runner, "Generating PRD suggestions...");
            cmd_suggest(&runner, model.as_deref())?;
        }
        Some(Command::Refactor {
            max,
            context,
            path,
            dry_run,
            no_commit,
            runner,
            model,
            stream,
        }) => {
            tracing::info!(
                runner = %runner,
                max = %max,
                context = ?context,
                path = ?path,
                dry_run = %dry_run,
                no_commit = %no_commit,
                stream = %stream,
                "Running refactor loop..."
            );
            cmd_refactor(&CmdRefactorArgs {
                max,
                context: context.as_deref(),
                path: path.as_deref(),
                dry_run,
                no_commit,
                runner_name: &runner,
                cli_model: model.as_deref(),
                stream,
            })?;
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
        Some(Command::Graph { command }) => match command {
            GraphCommand::Ascii {
                no_titles,
                max_title_len,
            } => {
                tracing::info!(no_titles = %no_titles, max_title_len = %max_title_len, "Rendering ASCII graph...");
                cmd_graph_ascii(!no_titles, max_title_len)?;
            }
            GraphCommand::Mermaid {
                no_titles,
                max_title_len,
                lr,
            } => {
                tracing::info!(no_titles = %no_titles, max_title_len = %max_title_len, lr = %lr, "Rendering Mermaid graph...");
                cmd_graph_mermaid(!no_titles, max_title_len, lr)?;
            }
            GraphCommand::Dot {
                no_titles,
                max_title_len,
                lr,
            } => {
                tracing::info!(no_titles = %no_titles, max_title_len = %max_title_len, lr = %lr, "Rendering DOT graph...");
                cmd_graph_dot(!no_titles, max_title_len, lr)?;
            }
        },
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
        "codex" => {
            let codex = runner::CodexRunner::with_model(model);

            if !codex.is_available() {
                anyhow::bail!(
                    "Codex CLI is not available. Install it or use `--runner mock` for testing."
                );
            }

            Ok(Box::new(codex))
        }
        other => anyhow::bail!("Unknown runner: {other}. Supported: copilot, claude, codex, mock"),
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
            colors::info(&format!("Adapting prompts and templates for {lang}..."))
        );

        // Load config for model (config file was just created).
        let cfg = config::Config::load_or_default(&cwd)?;
        let model = cfg.effective_model(cli_model);

        adapt_language(&cwd, lang, runner_name, model.as_deref())?;

        println!("{}", colors::info(&format!("Prompts adapted for {lang}.")));
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
    let runner = create_runner(runner_name, model.map(ToString::to_string))?;

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
fn cmd_bootstrap(
    runner_name: &str,
    language: Option<&str>,
    cli_model: Option<&str>,
    scaffold: bool,
    stream: bool,
) -> Result<()> {
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

    // Default behavior is reconstruct (scaffold=false means reconstruct=true).
    let mut config = bootstrap::BootstrapConfig::new(&cwd);
    config.reconstruct = !scaffold;
    config.stream = stream;

    if scaffold {
        println!("{}", colors::info("Scaffolding repository..."));
    } else {
        println!(
            "{}",
            colors::info("Reconstructing PRDs from git history...")
        );
    }
    println!("{}", colors::info(&format!("Detected language: {lang}")));
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
            colors::info(&format!("Adapting prompts and templates for {lang}..."))
        );

        adapt_language(&cwd, lang, runner_name, model.as_deref())?;

        println!("{}", colors::info(&format!("Prompts adapted for {lang}.")));
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

/// Core restore logic that takes an explicit root path.
///
/// This is separated from `cmd_restore` to allow testing without changing cwd.
fn restore_impl(root: &Path) -> Result<()> {
    init::ensure_initialized(root)?;

    println!("{}", colors::info("Restoring prompts and templates..."));

    let mr_dir = root.join(".mr");
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
    let prompts_result = init::init_prompts_and_templates(root)?;

    println!(
        "{}",
        colors::success(&format!(
            "✓ Restored {} prompt and template files",
            prompts_result.files_created
        ))
    );

    // Restore constitution.md and config.toml.
    let config_result = init::init_constitution_and_config(root)?;

    println!(
        "{}",
        colors::success(&format!(
            "✓ Replaced constitution.md and config.toml ({} files)",
            config_result.files_created
        ))
    );

    Ok(())
}

/// Runs the `mr restore` command.
fn cmd_restore() -> Result<()> {
    let cwd = std::env::current_dir()?;
    restore_impl(&cwd)
}

/// Runs the `mr new` command.
fn cmd_prd_new(
    slug: &str,
    runner_name: &str,
    cli_model: Option<&str>,
    context: Option<&str>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    // Show dev container warning for safety.
    devcontainer::show_dev_container_warning();

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    let config = prd::new::PrdNewConfig {
        root: &cwd,
        slug,
        description: None,
        context,
    };

    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let result = prd::new::create_prd(&config, runner.as_ref(), &mut stdout_lock)?;

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

    let task_count = result.prd.tasks().map_or(0, <[_]>::len);
    println!("  {}", colors::dim(&format!("Tasks: {task_count}")));

    Ok(())
}

/// Runs the `mr edit` command.
fn cmd_prd_edit(
    prd_id: &str,
    context: Option<&str>,
    runner_name: &str,
    cli_model: Option<&str>,
) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    // Show dev container warning for safety.
    devcontainer::show_dev_container_warning();

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    let config = prd::edit::PrdEditConfig {
        root: &cwd,
        prd_id,
        context,
    };

    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let result = prd::edit::edit_prd(&config, runner.as_ref(), &mut stdout_lock)?;

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

    Ok(())
}

/// Runs the `mr constitution edit` command.
fn cmd_constitution_edit(request: &str, runner_name: &str, cli_model: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    let edit_config = config::constitution::ConstitutionEditConfig {
        root: &cwd,
        request,
    };

    let stdin = std::io::stdin();
    let mut stdin_lock = stdin.lock();
    let stdout = std::io::stdout();
    let mut stdout_lock = stdout.lock();

    let result = config::constitution::edit_constitution(
        &edit_config,
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

/// Formats a PRD summary for display.
fn format_prd_summary(prd_summary: &prd::PrdSummary) -> String {
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
}

/// Prints a group of PRDs with a header.
fn print_prd_group(header: &str, prds: &[&prd::PrdSummary]) {
    if prds.is_empty() {
        return;
    }

    println!("  {}", colors::header(header));

    for prd_summary in prds {
        println!("    {}", format_prd_summary(prd_summary));
    }

    println!();
}

/// Runs the `mr list` command.
fn cmd_prd_list(include_done: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    // Regenerate the index file.
    prd::generate_index_from_root(&cwd)?;

    let prds = prd::scan_prd_summaries(&cwd)?;

    // Filter out done PRDs unless --done flag is set.
    let prds: Vec<_> = if include_done {
        prds
    } else {
        prds.into_iter()
            .filter(|p| p.status != prd::PrdStatus::Done)
            .collect()
    };

    if prds.is_empty() {
        if include_done {
            println!("{}", colors::info("No PRDs found."));
        } else {
            println!("{}", colors::info("No active PRDs found."));
            println!(
                "{}",
                colors::dim("Use `mr list --done` to include done PRDs.")
            );
        }
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

    print_prd_group("Active:", &active);
    print_prd_group("Draft:", &draft);
    print_prd_group("Done:", &done);
    print_prd_group("Parked:", &parked);

    Ok(())
}

/// Runs the `mr finalize` command.
fn cmd_prd_finalize(
    prd_id: &str,
    runner_name: &str,
    cli_model: Option<&str>,
    stream: bool,
    no_commit: bool,
) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    let config = prd::finalize::PrdFinalizeConfig {
        root: &cwd,
        prd_id,
        stream,
        no_commit,
    };

    let result = prd::finalize::finalize_prd(&config, runner.as_ref())?;

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

/// Prints the result of a task execution.
fn print_task_result(
    task_id: &str,
    task_title: &str,
    prd_id: &str,
    prd_path: &Path,
    runner_success: bool,
    output_summary: &str,
    usage: Option<&runner::TokenUsageInfo>,
) {
    println!();

    if runner_success {
        println!(
            "{}",
            colors::success(&format!("Task {task_id} completed successfully!"))
        );
    } else {
        println!(
            "{}",
            colors::error(&format!("Task {task_id} did not complete successfully."))
        );
    }

    println!();
    println!(
        "  {}",
        colors::dim(&format!("PRD: {} ({})", prd_id, prd_path.display()))
    );
    println!(
        "  {}",
        colors::dim(&format!("Task: {task_id} — {task_title}"))
    );
    println!();

    if !output_summary.is_empty() {
        println!("{}", colors::header("Runner output:"));
        println!("{output_summary}");
    }

    // Display usage information if available.
    if let Some(usage_info) = usage
        && usage_info.has_data()
    {
        println!();
        println!("{}", colors::dim("Token usage:"));

        if let Some(input) = usage_info.input {
            print!("{}", colors::dim(&format!("  Input: {input}")));
        }

        if let Some(output) = usage_info.output {
            if usage_info.input.is_some() {
                print!("{}", colors::dim(", "));
            } else {
                print!("{}", colors::dim("  "));
            }
            print!("{}", colors::dim(&format!("Output: {output}")));
        }

        if let Some(total) = usage_info.total {
            if usage_info.input.is_some() || usage_info.output.is_some() {
                print!("{}", colors::dim(", "));
            } else {
                print!("{}", colors::dim("  "));
            }
            print!("{}", colors::dim(&format!("Total: {total}")));
        }

        println!(); // Newline after usage info.
    }
}

/// Prints the result of UAT verification loop.
fn print_uat_result(result: &run::UatVerificationLoopResult) {
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

    if result.has_new_tasks {
        println!(
            "  {}",
            colors::warning("New incomplete tasks detected during UAT verification.")
        );
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

/// Prints the UAT verification introduction header.
fn print_uat_verification_header(prd_id: &str, prd_path: &Path, unverified_count: usize) {
    println!();
    println!(
        "{}",
        colors::warning(&format!(
            "All tasks done for {prd_id} but {unverified_count} UAT(s) need verification."
        ))
    );
    println!("  {}", colors::dim(&format!("PRD: {}", prd_path.display())));
    println!();
    println!("{}", colors::info("Starting UAT verification loop..."));
    println!();
}

/// Prints the PRD complete message.
fn print_prd_complete(prd_id: &str, prd_path: &Path) {
    println!();
    println!("{}", colors::success(&format!("PRD {prd_id} is complete!")));
    println!("  {}", colors::dim("All tasks done, all UATs verified."));
    println!("  {}", colors::dim(&format!("PRD: {}", prd_path.display())));
}

/// Handles `run_task` errors: `Ok(None)` to break, `Ok(Some(result))` to continue, `Err` to propagate.
fn handle_run_task_error(e: anyhow::Error, tasks_completed: u32) -> Result<Option<run::RunResult>> {
    let err_msg = e.to_string();
    if err_msg.contains("No active PRD") || err_msg.contains("no incomplete tasks") {
        if tasks_completed == 0 {
            return Err(e);
        }
        return Ok(None); // Break gracefully.
    }
    Err(e)
}

/// Runs the `mr run` command.
/// Options for the `mr run` CLI command.
#[allow(clippy::struct_excessive_bools)]
struct CmdRunOpts<'a> {
    prd_id: Option<&'a str>,
    runner_name: &'a str,
    one: bool,
    cli_model: Option<&'a str>,
    stream: bool,
    cli_no_commit: bool,
    disallow_skip_uat: bool,
    disallow_add_task: bool,
}

#[allow(clippy::too_many_lines)]
fn cmd_run(opts: &CmdRunOpts) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    // Show dev container warning for safety.
    devcontainer::show_dev_container_warning();

    // Normalize PRD ID if provided (e.g., "5" -> "PRD-0005").
    let normalized_prd_id = opts.prd_id.map(normalize_prd_id);

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(opts.cli_model);

    // Compute effective no_commit setting (CLI flag supersedes config).
    let no_commit = cfg.effective_no_commit(opts.cli_no_commit.then_some(true));

    // Select runner based on name.
    let runner = create_runner(opts.runner_name, model)?;

    // If no PRD ID was provided, ask the runner to pick one.
    let active_prd_id = normalized_prd_id.map_or_else(
        || {
            run::pick_prd_via_runner(&cwd, runner.as_ref(), opts.stream)?.ok_or_else(|| {
                anyhow::anyhow!(
                    "No active PRD with incomplete tasks found. Create a PRD with `mr new`."
                )
            })
        },
        Ok,
    )?;

    let max_uat_cycles = 10;
    let mut tasks_completed = 0;
    let mut uat_cycles = 0;

    loop {
        let config = run::RunConfig {
            root: &cwd,
            prd_id: Some(&active_prd_id),
            stream: opts.stream,
            no_commit,
            allow_add_task: !opts.disallow_add_task,
        };

        let result = match run::run_task(&config, runner.as_ref()) {
            Ok(result) => result,
            Err(e) => match handle_run_task_error(e, tasks_completed)? {
                Some(result) => result,
                None => break,
            },
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

                print_task_result(
                    &task_id,
                    &task_title,
                    &prd_id,
                    &prd_path,
                    runner_success,
                    &output_summary,
                    usage.as_ref(),
                );

                // Exit if --one flag is set or if the task failed.
                if opts.one || !runner_success {
                    break;
                }

                println!("---\n{}", colors::info("Continuing to next task..."));
            }

            run::RunResult::NeedsUatVerification {
                prd_id,
                prd_path,
                unverified_count,
            } => {
                print_uat_verification_header(&prd_id, &prd_path, unverified_count);

                // Run the UAT verification loop.
                let uat_config = run::UatVerificationConfig {
                    root: &cwd,
                    prd_id: &prd_id,
                    stream: opts.stream,
                    max_iterations: None, // Use PRD config or default.
                    allow_skip_uat: !opts.disallow_skip_uat,
                    allow_add_task: !opts.disallow_add_task,
                };

                match run::run_uat_verification_loop(&uat_config, runner.as_ref()) {
                    Ok(result) => {
                        print_uat_result(&result);

                        // If new tasks were added during UAT verification, re-enter
                        // task execution (with a safety counter to prevent infinite loops).
                        if result.has_new_tasks {
                            uat_cycles += 1;
                            if uat_cycles >= max_uat_cycles {
                                println!(
                                    "{}",
                                    colors::warning(&format!(
                                        "Safety limit reached ({max_uat_cycles} task→UAT cycles). Stopping."
                                    ))
                                );
                                break;
                            }

                            println!(
                                "---\n{}",
                                colors::info("New tasks detected; re-entering task execution...")
                            );
                            continue;
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
                print_prd_complete(&prd_id, &prd_path);
                break;
            }
        }
    }

    if tasks_completed > 1 {
        println!("---");
        println!(
            "{}",
            colors::info(&format!("Completed {tasks_completed} tasks total."))
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
    println!("{}", colors::info(&format!("Detected language: {lang}")));
    println!();

    // Call the testable implementation.
    generate_devcontainer_config(&cwd, lang, runner.as_ref())?;

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
    runner: &dyn runner::Runner,
) -> Result<()> {
    use crate::prompt::{
        PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
        load_prompt_with_fallback,
    };

    // Analyze repository for dev container context.
    let analysis = analyze_repo_for_devcontainer(root, lang);

    tracing::debug!("Repository analysis complete");

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
fn analyze_repo_for_devcontainer(root: &Path, lang: init::Language) -> String {
    use std::fmt::Write;
    use std::process::Command;

    let mut analysis = String::new();

    // Language and typical tools.
    let _ = writeln!(analysis, "Language: {lang}");
    let _ = writeln!(
        analysis,
        "Typical build commands: {:?}\n",
        lang.build_commands()
    );

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
        .map(|(_, desc)| format!("- {desc}\n"))
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

    analysis
}

/// Runs the `mr status` command.
fn cmd_status() -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    let report = status::get_status(&cwd)?;
    let output = status::format_status(&report);

    print!("{output}");

    Ok(())
}

/// Runs the `mr suggest` command.
fn cmd_suggest(runner_name: &str, cli_model: Option<&str>) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    // Run suggest logic.
    suggest::suggest(&cwd, runner.as_ref())?;

    Ok(())
}

/// Runs the `mr refactor` command.
///
/// Executes AI-driven iterative refactoring up to `max` iterations.
/// Each iteration identifies one impactful refactor, applies it, verifies UATs, and commits.
fn cmd_refactor(args: &CmdRefactorArgs<'_>) -> Result<()> {
    let CmdRefactorArgs {
        max,
        context,
        path,
        dry_run,
        no_commit,
        runner_name,
        cli_model,
        stream,
    } = *args;

    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    // Load config for model settings.
    let cfg = config::Config::load_or_default(&cwd)?;
    let model = cfg.effective_model(cli_model);

    // Select runner based on name.
    let runner = create_runner(runner_name, model)?;

    // Build refactor config.
    let config = refactor::RefactorConfig {
        root: &cwd,
        max_iterations: max,
        context,
        path,
        dry_run,
        no_commit,
        stream,
    };

    // Display header.
    println!("{}", colors::header("Refactor Command"));
    println!();
    println!("{}", colors::info(&format!("Max iterations: {max}")));
    if let Some(ctx) = context {
        println!("{}", colors::info(&format!("Focus: {ctx}")));
    }
    if let Some(p) = path {
        println!("{}", colors::info(&format!("Path constraint: {p}")));
    }
    if dry_run {
        println!(
            "{}",
            colors::dim("Mode: dry-run (no changes will be applied)")
        );
    }
    if no_commit {
        println!(
            "{}",
            colors::dim("Mode: no-commit (changes will not be committed)")
        );
    }
    println!();

    // Run the refactor loop.
    let result = refactor::refactor(&config, runner.as_ref())?;

    // Display summary.
    println!();
    println!("{}", colors::header("Refactor Summary"));
    println!();
    println!(
        "  {}",
        colors::info(&format!("Iterations: {}", result.iterations))
    );

    if dry_run {
        println!(
            "  {}",
            colors::info(&format!("Suggestions: {}", result.suggested_count))
        );
    } else {
        println!(
            "  {}",
            colors::success(&format!("Refactors applied: {}", result.applied_count))
        );
    }

    if result.early_termination {
        println!(
            "  {}",
            colors::dim("Early termination: no more impactful refactors found")
        );
    }

    if let Some(usage) = &result.total_usage {
        println!();
        println!(
            "  {}",
            colors::dim(&format!(
                "Tokens: {} in / {} out / {} total",
                usage.input.unwrap_or(0),
                usage.output.unwrap_or(0),
                usage.total.unwrap_or(0)
            ))
        );
    }

    Ok(())
}

/// Runs the `mr reindex` command.
fn cmd_reindex(runner_name: &str, cli_model: Option<&str>, stream: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

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
    println!(
        "  {}",
        colors::dim(&format!("depends_on added: {}", result.depends_on_added))
    );
    println!(
        "  {}",
        colors::dim(&format!("depends_on fixed: {}", result.depends_on_fixed))
    );

    Ok(())
}

/// Renders the PRD dependency graph as ASCII art.
fn cmd_graph_ascii(show_titles: bool, max_title_len: usize) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    let prd_graph = graph::build_graph(&cwd)?;

    let config = graph::AsciiConfig {
        display: graph::NodeDisplayConfig {
            show_titles,
            max_title_len,
        },
    };

    let output = graph::render_ascii(&prd_graph, Some(config));
    print!("{output}");

    // Print warnings if there are missing references.
    if prd_graph.has_missing_refs() {
        println!();
        println!(
            "{}",
            colors::warning(&format!(
                "⚠ {} missing PRD reference(s) detected",
                prd_graph.missing_refs.len()
            ))
        );
    }

    Ok(())
}

/// Renders the PRD dependency graph as Mermaid flowchart syntax.
fn cmd_graph_mermaid(show_titles: bool, max_title_len: usize, lr: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    let prd_graph = graph::build_graph(&cwd)?;

    let direction = if lr {
        graph::MermaidDirection::LeftRight
    } else {
        graph::MermaidDirection::TopDown
    };

    let config = graph::MermaidConfig {
        display: graph::NodeDisplayConfig {
            show_titles,
            max_title_len,
        },
        direction,
    };

    let output = graph::render_mermaid(&prd_graph, Some(config));
    print!("{output}");

    // Print warnings if there are missing references.
    if prd_graph.has_missing_refs() {
        eprintln!();
        eprintln!(
            "{}",
            colors::warning(&format!(
                "⚠ {} missing PRD reference(s) detected",
                prd_graph.missing_refs.len()
            ))
        );
    }

    Ok(())
}

/// Renders the PRD dependency graph as Graphviz DOT format.
fn cmd_graph_dot(show_titles: bool, max_title_len: usize, lr: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    init::ensure_initialized(&cwd)?;

    let prd_graph = graph::build_graph(&cwd)?;

    let direction = if lr {
        graph::DotDirection::LeftRight
    } else {
        graph::DotDirection::TopBottom
    };

    let config = graph::DotConfig {
        display: graph::NodeDisplayConfig {
            show_titles,
            max_title_len,
        },
        direction,
    };

    let output = graph::render_dot(&prd_graph, Some(config));
    print!("{output}");

    // Print warnings if there are missing references.
    if prd_graph.has_missing_refs() {
        eprintln!();
        eprintln!(
            "{}",
            colors::warning(&format!(
                "⚠ {} missing PRD reference(s) detected",
                prd_graph.missing_refs.len()
            ))
        );
    }

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
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
    fn test_args_parse_run_with_runner_codex() {
        let args = Args::try_parse_from(["mr", "run", "--runner", "codex"]).unwrap();
        if let Some(Command::Run { runner, .. }) = args.command {
            assert_eq!(runner, "codex");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_parse_run_with_model() {
        let args = Args::try_parse_from(["mr", "run", "--model", "claude-sonnet-4.5"]).unwrap();
        if let Some(Command::Run { runner, model, .. }) = args.command {
            assert_eq!(runner, "copilot");
            assert_eq!(model, Some("claude-sonnet-4.5".to_string()));
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
    fn test_args_parse_run_with_disallow_skip_uat() {
        let args = Args::try_parse_from(["mr", "run", "--disallow-skip-uat"]).unwrap();
        if let Some(Command::Run {
            disallow_skip_uat, ..
        }) = args.command
        {
            assert!(disallow_skip_uat);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_parse_run_with_disallow_add_task() {
        let args = Args::try_parse_from(["mr", "run", "--disallow-add-task"]).unwrap();
        if let Some(Command::Run {
            disallow_add_task, ..
        }) = args.command
        {
            assert!(disallow_add_task);
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_parse_run_default_disallow_flags_off() {
        let args = Args::try_parse_from(["mr", "run"]).unwrap();
        if let Some(Command::Run {
            disallow_skip_uat,
            disallow_add_task,
            ..
        }) = args.command
        {
            assert!(!disallow_skip_uat);
            assert!(!disallow_add_task);
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
            scaffold,
            stream,
        }) = args.command
        {
            assert_eq!(runner, "mock");
            assert!(language.is_none());
            assert!(model.is_none());
            assert!(!scaffold);
            assert!(!stream);
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
            scaffold,
            stream,
        }) = args.command
        {
            assert_eq!(runner, "mock");
            assert_eq!(language, Some("node".to_string()));
            assert!(model.is_none());
            assert!(!scaffold);
            assert!(!stream);
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
            scaffold,
            stream,
        }) = args.command
        {
            assert_eq!(runner, "copilot");
            assert!(language.is_none());
            assert_eq!(model, Some("claude-opus-4".to_string()));
            assert!(!scaffold);
            assert!(!stream);
        } else {
            panic!("Expected Bootstrap command");
        }
    }

    #[test]
    fn test_args_parse_bootstrap_with_scaffold() {
        let args =
            Args::try_parse_from(["mr", "bootstrap", "--runner", "mock", "--scaffold"]).unwrap();
        if let Some(Command::Bootstrap {
            runner,
            language,
            model,
            scaffold,
            stream,
        }) = args.command
        {
            assert_eq!(runner, "mock");
            assert!(language.is_none());
            assert!(model.is_none());
            assert!(scaffold);
            assert!(!stream);
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
            no_commit,
        }) = args.command
        {
            assert_eq!(prd_id, "PRD-0001");
            assert_eq!(runner, "copilot");
            assert!(model.is_none());
            assert!(!stream);
            assert!(!no_commit);
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
            no_commit,
        }) = args.command
        {
            assert_eq!(prd_id, "PRD-0002");
            assert_eq!(runner, "mock");
            assert_eq!(model, Some("gpt-4o".to_string()));
            assert!(stream);
            assert!(!no_commit);
        } else {
            panic!("Expected Prd Finalize command");
        }
    }

    #[test]
    fn test_args_parse_prd_finalize_no_commit() {
        let args = Args::try_parse_from(["mr", "finalize", "PRD-0003", "--no-commit"]).unwrap();
        if let Some(Command::Finalize {
            prd_id, no_commit, ..
        }) = args.command
        {
            assert_eq!(prd_id, "PRD-0003");
            assert!(no_commit);
        } else {
            panic!("Expected Finalize command with no_commit");
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
        if let Some(Command::List { done }) = args.command {
            assert!(!done, "Default should not include done PRDs");
        } else {
            panic!("Expected List command");
        }
    }

    #[test]
    fn test_args_parse_list_with_done_flag() {
        // Verify list command accepts --done flag
        let args = Args::try_parse_from(["mr", "list", "--done"]).unwrap();
        if let Some(Command::List { done }) = args.command {
            assert!(done, "--done flag should be true");
        } else {
            panic!("Expected List command with --done flag");
        }
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
        // Verify edit command works at top level with prd_id argument
        let args = Args::try_parse_from(["mr", "edit", "PRD-0001"]).unwrap();
        if let Some(Command::Edit {
            prd_id, context, ..
        }) = args.command
        {
            assert_eq!(prd_id, "PRD-0001");
            assert!(context.is_none());
        } else {
            panic!("Expected Edit command with prd_id 'PRD-0001'");
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

        let prompt_content = r"# Dev Container Generation

Generate a devcontainer.json configuration for:
- Language: {{language}}
- Analysis: {{analysis}}

**Create the file `.devcontainer/devcontainer.json` directly** with the generated configuration.
";
        std::fs::write(prompts_dir.join("devcontainer_generate.md"), prompt_content).unwrap();

        // Mock response - LLM will create the file itself, so response doesn't matter.
        let mock_response = "File created successfully.";

        // Set up mock runner with successful response.
        let mock_runner = MockRunner::new(vec![RunnerOutput::success(mock_response)]);

        // Call the core generation function.
        let result = generate_devcontainer_config(temp_path, init::Language::Rust, &mock_runner);

        // Verify success - the LLM is responsible for creating the file, not microralph.
        assert!(result.is_ok(), "Generation should succeed: {result:?}");
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

        // Run restore (using restore_impl to avoid cwd changes).
        let result = restore_impl(root);

        // Verify success.
        assert!(result.is_ok(), "Restore should succeed: {result:?}");

        // Verify files still exist after restore.
        assert!(root.join(".mr/prompts/init.md").exists());
        assert!(root.join(".mr/templates/prd.md").exists());
        assert!(root.join(".mr/prompts/run_task.md").exists());
        assert!(root.join(".mr/prompts/suggest_generate.md").exists());
        assert!(root.join(".mr/constitution.md").exists());
        assert!(root.join(".mr/config.toml").exists());
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

        // Also customize constitution.md.
        let constitution_path = root.join(".mr/constitution.md");
        let custom_constitution = "# CUSTOM CONSTITUTION";
        std::fs::write(&constitution_path, custom_constitution).unwrap();

        // Run restore (using restore_impl to avoid cwd changes).
        let result = restore_impl(root);

        // Verify success.
        assert!(result.is_ok(), "Restore should succeed: {result:?}");

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

        // Verify constitution was also restored.
        let constitution_after = std::fs::read_to_string(&constitution_path).unwrap();
        assert_ne!(
            constitution_after, custom_constitution,
            "Constitution should be restored to built-in default"
        );
        assert!(
            constitution_after.contains("Constitution"),
            "Restored constitution should contain built-in text"
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

        // Run restore first time (using restore_impl to avoid cwd changes).
        let result1 = restore_impl(root);
        assert!(result1.is_ok(), "First restore should succeed");

        // Capture file contents after first restore.
        let init_content1 = std::fs::read_to_string(root.join(".mr/prompts/init.md")).unwrap();
        let prd_template1 = std::fs::read_to_string(root.join(".mr/templates/prd.md")).unwrap();
        let constitution1 = std::fs::read_to_string(root.join(".mr/constitution.md")).unwrap();
        let config1 = std::fs::read_to_string(root.join(".mr/config.toml")).unwrap();

        // Run restore second time.
        let result2 = restore_impl(root);
        assert!(result2.is_ok(), "Second restore should succeed");

        // Capture file contents after second restore.
        let init_content2 = std::fs::read_to_string(root.join(".mr/prompts/init.md")).unwrap();
        let prd_template2 = std::fs::read_to_string(root.join(".mr/templates/prd.md")).unwrap();
        let constitution2 = std::fs::read_to_string(root.join(".mr/constitution.md")).unwrap();
        let config2 = std::fs::read_to_string(root.join(".mr/config.toml")).unwrap();

        // Run restore third time to ensure it still works.
        let result3 = restore_impl(root);
        assert!(result3.is_ok(), "Third restore should succeed");

        // Verify idempotency: contents should be identical.
        assert_eq!(
            init_content1, init_content2,
            "Init prompt should be identical after multiple restores"
        );
        assert_eq!(
            prd_template1, prd_template2,
            "PRD template should be identical after multiple restores"
        );
        assert_eq!(
            constitution1, constitution2,
            "Constitution should be identical after multiple restores"
        );
        assert_eq!(
            config1, config2,
            "Config should be identical after multiple restores"
        );
    }

    #[test]
    fn test_restore_fails_if_not_initialized() {
        // Test scenario: Restore should fail if .mr/ directory doesn't exist
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Don't initialize - leave directory empty.

        // Run restore (using restore_impl to avoid cwd changes).
        let result = restore_impl(root);

        // Verify failure.
        assert!(result.is_err(), "Restore should fail if not initialized");
        assert!(
            result.unwrap_err().to_string().contains("not initialized"),
            "Error message should mention not initialized"
        );
    }

    #[test]
    fn test_args_parse_refactor_defaults() {
        // Verify refactor command parses with default values
        let args = Args::try_parse_from(["mr", "refactor"]).unwrap();
        if let Some(Command::Refactor {
            max,
            context,
            path,
            dry_run,
            no_commit,
            runner,
            model,
            stream,
        }) = args.command
        {
            assert_eq!(max, 3, "Default max should be 3");
            assert!(context.is_none(), "Context should be None by default");
            assert!(path.is_none(), "Path should be None by default");
            assert!(!dry_run, "Dry run should be false by default");
            assert!(!no_commit, "No commit should be false by default");
            assert_eq!(runner, "copilot", "Default runner should be copilot");
            assert!(model.is_none(), "Model should be None by default");
            assert!(!stream, "Stream should be false by default");
        } else {
            panic!("Expected Refactor command");
        }
    }

    #[test]
    fn test_args_parse_refactor_with_all_flags() {
        // Verify refactor command parses with all flags set
        let args = Args::try_parse_from([
            "mr",
            "refactor",
            "--max",
            "5",
            "--context",
            "improve error handling",
            "--path",
            "src/",
            "--dry-run",
            "--no-commit",
            "--runner",
            "claude",
            "--model",
            "claude-sonnet-4.5",
            "--stream",
        ])
        .unwrap();
        if let Some(Command::Refactor {
            max,
            context,
            path,
            dry_run,
            no_commit,
            runner,
            model,
            stream,
        }) = args.command
        {
            assert_eq!(max, 5, "Max should be 5");
            assert_eq!(
                context,
                Some("improve error handling".to_string()),
                "Context should match"
            );
            assert_eq!(path, Some("src/".to_string()), "Path should match");
            assert!(dry_run, "Dry run should be true");
            assert!(no_commit, "No commit should be true");
            assert_eq!(runner, "claude", "Runner should be claude");
            assert_eq!(
                model,
                Some("claude-sonnet-4.5".to_string()),
                "Model should match"
            );
            assert!(stream, "Stream should be true");
        } else {
            panic!("Expected Refactor command with all flags");
        }
    }

    #[test]
    fn test_args_parse_graph_ascii_defaults() {
        let args = Args::try_parse_from(["mr", "graph", "ascii"]).unwrap();
        if let Some(Command::Graph { command }) = args.command {
            if let GraphCommand::Ascii {
                no_titles,
                max_title_len,
            } = command
            {
                assert!(!no_titles, "no_titles should be false by default");
                assert_eq!(max_title_len, 40, "max_title_len should be 40 by default");
            } else {
                panic!("Expected Ascii subcommand");
            }
        } else {
            panic!("Expected Graph command");
        }
    }

    #[test]
    fn test_args_parse_graph_ascii_with_flags() {
        let args = Args::try_parse_from([
            "mr",
            "graph",
            "ascii",
            "--no-titles",
            "--max-title-len",
            "20",
        ])
        .unwrap();
        if let Some(Command::Graph { command }) = args.command {
            if let GraphCommand::Ascii {
                no_titles,
                max_title_len,
            } = command
            {
                assert!(no_titles, "no_titles should be true");
                assert_eq!(max_title_len, 20, "max_title_len should be 20");
            } else {
                panic!("Expected Ascii subcommand");
            }
        } else {
            panic!("Expected Graph command");
        }
    }

    #[test]
    fn test_args_parse_graph_mermaid_defaults() {
        let args = Args::try_parse_from(["mr", "graph", "mermaid"]).unwrap();
        if let Some(Command::Graph { command }) = args.command {
            if let GraphCommand::Mermaid {
                no_titles,
                max_title_len,
                lr,
            } = command
            {
                assert!(!no_titles, "no_titles should be false by default");
                assert_eq!(max_title_len, 40, "max_title_len should be 40 by default");
                assert!(!lr, "lr should be false by default");
            } else {
                panic!("Expected Mermaid subcommand");
            }
        } else {
            panic!("Expected Graph command");
        }
    }

    #[test]
    fn test_args_parse_graph_mermaid_with_lr() {
        let args = Args::try_parse_from(["mr", "graph", "mermaid", "--lr"]).unwrap();
        if let Some(Command::Graph { command }) = args.command {
            if let GraphCommand::Mermaid {
                no_titles,
                max_title_len,
                lr,
            } = command
            {
                assert!(!no_titles, "no_titles should be false");
                assert_eq!(max_title_len, 40, "max_title_len should be 40");
                assert!(lr, "lr should be true");
            } else {
                panic!("Expected Mermaid subcommand");
            }
        } else {
            panic!("Expected Graph command");
        }
    }

    #[test]
    fn test_args_parse_graph_dot_defaults() {
        let args = Args::try_parse_from(["mr", "graph", "dot"]).unwrap();
        if let Some(Command::Graph { command }) = args.command {
            if let GraphCommand::Dot {
                no_titles,
                max_title_len,
                lr,
            } = command
            {
                assert!(!no_titles, "no_titles should be false by default");
                assert_eq!(max_title_len, 40, "max_title_len should be 40 by default");
                assert!(!lr, "lr should be false by default");
            } else {
                panic!("Expected Dot subcommand");
            }
        } else {
            panic!("Expected Graph command");
        }
    }

    #[test]
    fn test_args_parse_graph_dot_with_all_flags() {
        let args = Args::try_parse_from([
            "mr",
            "graph",
            "dot",
            "--no-titles",
            "--max-title-len",
            "60",
            "--lr",
        ])
        .unwrap();
        if let Some(Command::Graph { command }) = args.command {
            if let GraphCommand::Dot {
                no_titles,
                max_title_len,
                lr,
            } = command
            {
                assert!(no_titles, "no_titles should be true");
                assert_eq!(max_title_len, 60, "max_title_len should be 60");
                assert!(lr, "lr should be true");
            } else {
                panic!("Expected Dot subcommand");
            }
        } else {
            panic!("Expected Graph command");
        }
    }
}
