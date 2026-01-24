use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod agents;
mod bootstrap;
mod init;
mod prd;
mod prd_new;
mod prompt;
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
    Init,

    /// Ingest an existing repo into PRDs: generate `.mr/PRDS.md` and starter PRDs.
    Bootstrap {
        /// The runner to use for bootstrapping.
        #[arg(long, default_value = "copilot")]
        runner: String,
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
    },

    /// Show status of PRDs and tasks.
    Status,
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
    },

    /// List all PRDs.
    List,
}

fn main() -> Result<()> {
    let args = Args::parse();

    init_tracing(args.verbose, args.quiet);

    match args.command {
        Some(Command::Init) => {
            tracing::info!("Initializing microralph...");
            cmd_init()?;
        }
        Some(Command::Bootstrap { runner }) => {
            tracing::info!(runner = %runner, "Bootstrapping repo...");
            cmd_bootstrap(&runner)?;
        }
        Some(Command::Prd { prd_command }) => match prd_command {
            PrdCommand::New { slug, runner } => {
                tracing::info!(slug = %slug, runner = %runner, "Creating new PRD...");
                cmd_prd_new(&slug, &runner)?;
            }
            PrdCommand::List => {
                tracing::info!("Listing PRDs...");
                cmd_prd_list()?;
            }
        },
        Some(Command::Run { prd, runner, one }) => {
            tracing::info!(prd = ?prd, runner = %runner, one = %one, "Running next task...");
            cmd_run(prd.as_deref(), &runner, one)?;
        }
        Some(Command::Status) => {
            tracing::info!("Showing status...");
            cmd_status()?;
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
fn cmd_init() -> Result<()> {
    let cwd = std::env::current_dir()?;

    if init::is_initialized(&cwd) {
        println!("microralph is already initialized in this directory.");
        println!("Run `mr status` to see PRD status.");
        return Ok(());
    }

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

    println!();
    println!("Next steps:");
    println!("  1. Review and customize AGENTS.md");
    println!("  2. Create your first PRD: `mr prd new my-feature`");
    println!("  3. Run a task: `mr run`");

    Ok(())
}

/// Runs the `mr bootstrap` command.
fn cmd_bootstrap(runner_name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;

    // Select runner based on name.
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::new();

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
fn cmd_prd_new(slug: &str, runner_name: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Select runner based on name.
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::new();

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

    println!("PRDs:");
    println!();

    for prd_summary in &prds {
        let progress = if prd_summary.total_tasks > 0 {
            format!(
                " [{}/{}]",
                prd_summary.completed_tasks, prd_summary.total_tasks
            )
        } else {
            String::new()
        };

        println!(
            "  {} - {} ({}){}",
            prd_summary.id, prd_summary.title, prd_summary.status, progress
        );
    }

    Ok(())
}

/// Runs the `mr run` command.
fn cmd_run(prd_id: Option<&str>, runner_name: &str, one: bool) -> Result<()> {
    let cwd = std::env::current_dir()?;

    if !init::is_initialized(&cwd) {
        anyhow::bail!("microralph is not initialized. Run `mr init` first.");
    }

    // Select runner based on name.
    let runner: Box<dyn runner::Runner> = match runner_name {
        "mock" => Box::new(runner::MockRunner::empty()),
        "copilot" => {
            let copilot = runner::CopilotRunner::new();

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

    let config = run::RunConfig { root: &cwd, prd_id };

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
        assert!(matches!(args.command, Some(Command::Init)));
    }

    #[test]
    fn test_args_parse_status() {
        let args = Args::try_parse_from(["mr", "status"]).unwrap();
        assert!(matches!(args.command, Some(Command::Status)));
    }

    #[test]
    fn test_args_parse_run_with_runner() {
        let args = Args::try_parse_from(["mr", "run", "--runner", "mock"]).unwrap();
        if let Some(Command::Run { runner, .. }) = args.command {
            assert_eq!(runner, "mock");
        } else {
            panic!("Expected Run command");
        }
    }

    #[test]
    fn test_args_parse_prd_new() {
        let args = Args::try_parse_from(["mr", "prd", "new", "my-feature"]).unwrap();
        if let Some(Command::Prd {
            prd_command: PrdCommand::New { slug, runner },
        }) = args.command
        {
            assert_eq!(slug, "my-feature");
            assert_eq!(runner, "copilot");
        } else {
            panic!("Expected Prd New command");
        }
    }

    #[test]
    fn test_args_parse_bootstrap() {
        let args = Args::try_parse_from(["mr", "bootstrap", "--runner", "mock"]).unwrap();
        if let Some(Command::Bootstrap { runner }) = args.command {
            assert_eq!(runner, "mock");
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
}
