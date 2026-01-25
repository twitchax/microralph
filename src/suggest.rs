//! AI-driven PRD suggestion generation.
//!
//! This module implements `mr suggest` which analyzes the codebase,
//! existing PRDs, and external research to generate actionable PRD suggestions.

use anyhow::{Context, Result, bail};
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;

use crate::colors;
use crate::prd::{generate_index_from_root, scan_prd_summaries};
use crate::prd_new::{PrdNewConfig, create_prd};
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;

/// A single PRD suggestion from the AI.
#[derive(Debug, Clone)]
pub struct Suggestion {
    pub number: usize,
    pub title: String,
    pub description: String,
    pub category: String,
    pub effort: String,
    pub rationale: String,
}

/// Runs the PRD suggestion flow.
///
/// This function:
/// 1. Analyzes the codebase and existing PRDs
/// 2. Invokes the runner to generate 5 PRD suggestions
/// 3. Displays a numbered picker for user selection
/// 4. Flows the selected suggestion into `mr new` with pre-filled context
pub fn suggest<R>(root: &Path, runner: &R) -> Result<()>
where
    R: Runner + ?Sized,
{
    println!("{}", colors::info("Analyzing codebase and PRDs..."));
    println!();

    // Gather context: existing PRDs and codebase snapshot.
    let existing_prds = scan_prd_summaries(root)?;
    let codebase_snapshot = analyze_codebase(root)?;

    tracing::debug!(
        prd_count = existing_prds.len(),
        snapshot_len = codebase_snapshot.len(),
        "Context gathered for suggestion generation"
    );

    // Build the prompt.
    let prompt_text = load_prompt_with_fallback(root, PromptKind::SuggestGenerate);
    let expanded_prompt = build_suggestion_prompt(&prompt_text, &existing_prds, &codebase_snapshot);

    tracing::info!(
        runner = %runner.name(),
        "Invoking runner to generate PRD suggestions"
    );

    println!("{}", colors::info("Generating suggestions..."));
    println!();

    // Invoke the runner.
    let result = runner
        .execute(&expanded_prompt, root)
        .context("Runner failed during suggestion generation")?;

    if !result.success {
        bail!("Runner failed to generate suggestions: {}", result.text);
    }

    tracing::debug!("Runner responded with suggestions");

    // Parse the suggestions.
    let suggestions = parse_suggestions(&result.text)?;

    if suggestions.len() != 5 {
        tracing::warn!(
            count = suggestions.len(),
            "Expected exactly 5 suggestions, got different count"
        );
    }

    // Display numbered picker.
    println!("{}", colors::header("Suggested PRDs:"));
    println!();

    for suggestion in &suggestions {
        println!(
            "{}",
            colors::info(&format!("{}. {}", suggestion.number, suggestion.title))
        );
        println!("   {}", colors::dim(&suggestion.description));
        println!(
            "   {}",
            colors::dim(&format!(
                "Category: {} | Effort: {}",
                suggestion.category, suggestion.effort
            ))
        );
        println!(
            "   {}",
            colors::dim(&format!("Rationale: {}", suggestion.rationale))
        );
        println!();
    }

    // Prompt user for selection.
    print!(
        "{}",
        colors::info("Select a suggestion (1-5) or 'q' to quit: ")
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    if input.eq_ignore_ascii_case("q") {
        println!("{}", colors::dim("Cancelled."));
        return Ok(());
    }

    // Parse selection.
    let selection: usize = input
        .parse()
        .context("Invalid selection. Please enter a number between 1 and 5.")?;

    if selection < 1 || selection > suggestions.len() {
        bail!(
            "Selection out of range. Please enter a number between 1 and {}.",
            suggestions.len()
        );
    }

    let selected = &suggestions[selection - 1];

    tracing::info!(
        selection = selection,
        title = %selected.title,
        "User selected suggestion"
    );

    // Generate a slug from the title.
    let slug = generate_slug(&selected.title);

    println!();
    println!(
        "{}",
        colors::success(&format!("Selected: {}", selected.title))
    );
    println!("{}", colors::dim(&format!("Generating slug: {}", slug)));
    println!();

    // Flow into `mr new` with pre-filled context.
    let context = format!(
        "{}\n\nCategory: {}\nEffort: {}\nRationale: {}",
        selected.description, selected.category, selected.effort, selected.rationale
    );

    let config = PrdNewConfig {
        root,
        slug: &slug,
        description: Some(&selected.description),
        context: Some(&context),
    };

    let stdin = io::stdin();
    let mut input_handle = stdin.lock();
    let mut output = io::stdout();

    let result = create_prd(&config, runner, &mut input_handle, &mut output)?;

    println!();
    println!("{}", colors::success("PRD created successfully!"));
    println!("  {}", colors::dim(&format!("ID: {}", result.prd.id())));
    println!(
        "  {}",
        colors::dim(&format!("Path: {}", result.path.display()))
    );

    // Count tasks if available
    let task_count = result.prd.tasks().map(|t| t.len()).unwrap_or(0);
    println!("  {}", colors::dim(&format!("Tasks: {}", task_count)));

    // Regenerate index.
    generate_index_from_root(root)?;

    Ok(())
}

/// Analyzes the codebase to gather context for suggestion generation.
///
/// Returns a string containing:
/// - Repository structure
/// - Detected tools and dependencies
/// - Recent git activity
/// - TODO comments and technical debt indicators
fn analyze_codebase(root: &Path) -> Result<String> {
    let mut analysis = String::new();

    // List key files and directories.
    analysis.push_str("Repository structure:\n");
    for entry in std::fs::read_dir(root).context("Failed to read repository directory")? {
        let entry = entry?;
        let path = entry.path();
        let name = path.file_name().unwrap().to_string_lossy();

        // Skip hidden files and target/build directories.
        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }

        let kind = if path.is_dir() { "dir" } else { "file" };
        analysis.push_str(&format!("- {} ({})\n", name, kind));
    }
    analysis.push('\n');

    // Check for common tools and dependencies.
    let tools = vec![
        ("Cargo.toml", "Rust (cargo)"),
        ("Makefile.toml", "cargo-make"),
        ("package.json", "Node.js (npm/yarn)"),
        ("requirements.txt", "Python (pip)"),
        ("go.mod", "Go modules"),
        (".github/workflows", "GitHub Actions"),
    ];

    analysis.push_str("Tools and dependencies:\n");
    for (file, desc) in tools {
        if root.join(file).exists() {
            analysis.push_str(&format!("- {}\n", desc));
        }
    }
    analysis.push('\n');

    // Get recent git commit messages (last 20).
    if let Ok(output) = Command::new("git")
        .args([
            "log",
            "--all",
            "--oneline",
            "--no-merges",
            "-20",
            "--pretty=format:%s",
        ])
        .current_dir(root)
        .output()
        && output.status.success()
    {
        let log = String::from_utf8_lossy(&output.stdout);
        analysis.push_str("Recent commits (last 20):\n");
        analysis.push_str(&log);
        analysis.push_str("\n\n");
    }

    // Count TODO comments in source files (simple heuristic).
    if let Ok(output) = Command::new("git")
        .args(["grep", "-i", "TODO", "--", "*.rs", "*.py", "*.js", "*.go"])
        .current_dir(root)
        .output()
        && output.status.success()
    {
        let todos = String::from_utf8_lossy(&output.stdout);
        let todo_count = todos.lines().count();
        analysis.push_str(&format!("TODO comments found: {}\n", todo_count));
        if todo_count > 0 && todo_count <= 10 {
            // Include a few examples if count is reasonable.
            analysis.push_str("Examples:\n");
            for (i, line) in todos.lines().take(5).enumerate() {
                analysis.push_str(&format!("  {}. {}\n", i + 1, line));
            }
        }
        analysis.push('\n');
    }

    Ok(analysis)
}

/// Builds the suggestion generation prompt with placeholders expanded.
fn build_suggestion_prompt(
    template: &str,
    existing_prds: &[crate::prd::PrdSummary],
    codebase_snapshot: &str,
) -> String {
    let mut ctx = PlaceholderContext::new();

    // Format existing PRDs.
    let prds_text = if existing_prds.is_empty() {
        "No existing PRDs.".to_string()
    } else {
        existing_prds
            .iter()
            .map(|p| format!("- {} ({}) - {}", p.id, p.status, p.title))
            .collect::<Vec<_>>()
            .join("\n")
    };

    ctx.insert("existing_prds", PlaceholderValue::String(prds_text));
    ctx.insert(
        "codebase_snapshot",
        PlaceholderValue::String(codebase_snapshot.to_string()),
    );

    expand_placeholders(template, &ctx)
}

/// Parses the runner's response to extract exactly 5 suggestions.
///
/// Expected format:
/// ```
/// 1. [Title] — [Description]
///    Category: [category]
///    Effort: [effort]
///    Rationale: [rationale]
/// ```
fn parse_suggestions(text: &str) -> Result<Vec<Suggestion>> {
    let mut suggestions = Vec::new();
    let lines: Vec<&str> = text.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i].trim();

        // Look for numbered entries (e.g., "1. ").
        if let Some(rest) = line
            .strip_prefix(|c: char| c.is_numeric())
            .and_then(|s| s.strip_prefix(". "))
        {
            let number = line.chars().next().unwrap().to_digit(10).unwrap() as usize;

            // Parse title and description from "Title — Description".
            let (title, description) = if let Some(sep_idx) = rest.find(" — ") {
                (
                    rest[..sep_idx].trim().to_string(),
                    rest[sep_idx + 3..].trim().to_string(),
                )
            } else {
                (rest.trim().to_string(), String::new())
            };

            // Parse subsequent lines for Category, Effort, Rationale.
            let mut category = String::new();
            let mut effort = String::new();
            let mut rationale = String::new();

            i += 1;
            while i < lines.len() {
                let meta_line = lines[i].trim();

                if meta_line.is_empty() {
                    i += 1;
                    break;
                }

                if let Some(cat) = meta_line.strip_prefix("Category:") {
                    category = cat.trim().to_string();
                } else if let Some(eff) = meta_line.strip_prefix("Effort:") {
                    effort = eff.trim().to_string();
                } else if let Some(rat) = meta_line.strip_prefix("Rationale:") {
                    rationale = rat.trim().to_string();
                }

                i += 1;

                // Stop if we hit the next numbered entry.
                if lines
                    .get(i)
                    .and_then(|l| l.trim().chars().next())
                    .is_some_and(|c| c.is_numeric() && lines[i].trim().contains(". "))
                {
                    break;
                }
            }

            suggestions.push(Suggestion {
                number,
                title,
                description,
                category,
                effort,
                rationale,
            });
        } else {
            i += 1;
        }
    }

    if suggestions.is_empty() {
        bail!("No suggestions found in runner output");
    }

    Ok(suggestions)
}

/// Generates a URL-friendly slug from a title.
///
/// Converts to lowercase, replaces spaces/punctuation with hyphens,
/// and removes consecutive hyphens.
fn generate_slug(title: &str) -> String {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}
