use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

mod init;
mod prd;

/// Micro Ralph (`mr`) — A tiny CLI for creating and executing PRDs with coding agents.
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
            tracing::info!("Initializing Micro Ralph...");
            cmd_init()?;
        }
        Some(Command::Bootstrap { runner }) => {
            tracing::info!(runner = %runner, "Bootstrapping repo...");
            println!("mr bootstrap --runner {runner}: not yet implemented");
        }
        Some(Command::Prd { prd_command }) => match prd_command {
            PrdCommand::New { slug, runner } => {
                tracing::info!(slug = %slug, runner = %runner, "Creating new PRD...");
                println!("mr prd new {slug} --runner {runner}: not yet implemented");
            }
            PrdCommand::List => {
                tracing::info!("Listing PRDs...");
                println!("mr prd list: not yet implemented");
            }
        },
        Some(Command::Run { prd, runner }) => {
            tracing::info!(prd = ?prd, runner = %runner, "Running next task...");
            println!("mr run: not yet implemented");
        }
        Some(Command::Status) => {
            tracing::info!("Showing status...");
            println!("mr status: not yet implemented");
        }
        None => {
            println!(
                "Micro Ralph (`mr`) — A tiny CLI for creating and executing PRDs with coding agents."
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
        println!("Micro Ralph is already initialized in this directory.");
        println!("Run `mr status` to see PRD status.");
        return Ok(());
    }

    let result = init::init(&cwd)?;

    println!("Initialized Micro Ralph!");
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
