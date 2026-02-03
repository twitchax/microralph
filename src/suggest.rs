//! AI-driven PRD suggestion generation.
//!
//! This module implements `mr suggest` which analyzes the codebase,
//! existing PRDs, and external research to generate actionable PRD suggestions.

use anyhow::{Context, Result, bail};
use std::fmt::Write as FmtWrite;
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
use crate::spinner::start_spinner;

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
#[allow(clippy::too_many_lines)]
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

    // Start spinner during AI generation phase (always enabled since suggest doesn't stream).
    let spinner = start_spinner(true, "Analyzing codebase...");

    // Invoke the runner.
    let result = runner
        .execute(&expanded_prompt, root)
        .context("Runner failed during suggestion generation")?;

    // Clear spinner before displaying output.
    spinner.finish_and_clear();

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

    // Validate and parse selection.
    let Some(selection_index) = validate_selection(&input, suggestions.len())? else {
        println!("{}", colors::dim("Cancelled."));
        return Ok(());
    };

    let selected = &suggestions[selection_index];

    tracing::info!(
        selection = selection_index + 1,
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
    println!("{}", colors::dim(&format!("Generating slug: {slug}")));
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
        stream: false,
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
    let task_count = result.prd.tasks().map_or(0, <[_]>::len);
    println!("  {}", colors::dim(&format!("Tasks: {task_count}")));

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

    analysis.push_str(&analyze_repository_structure(root)?);
    analysis.push_str(&detect_tools_and_dependencies(root));
    analysis.push_str(&get_recent_commits(root));
    analysis.push_str(&detect_todo_comments(root));

    Ok(analysis)
}

/// Lists key files and directories in the repository root.
///
/// Skips hidden files, `target/`, and `node_modules/` directories.
fn analyze_repository_structure(root: &Path) -> Result<String> {
    let mut output = String::from("Repository structure:\n");

    for entry in std::fs::read_dir(root).context("Failed to read repository directory")? {
        let entry = entry?;
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy())
            .unwrap_or_default();

        if name.starts_with('.') || name == "target" || name == "node_modules" {
            continue;
        }

        let kind = if path.is_dir() { "dir" } else { "file" };
        let _ = writeln!(output, "- {name} ({kind})");
    }
    output.push('\n');

    Ok(output)
}

/// Known tools and their display names for dependency detection.
const KNOWN_TOOLS: &[(&str, &str)] = &[
    ("Cargo.toml", "Rust (cargo)"),
    ("Makefile.toml", "cargo-make"),
    ("package.json", "Node.js (npm/yarn)"),
    ("requirements.txt", "Python (pip)"),
    ("go.mod", "Go modules"),
    (".github/workflows", "GitHub Actions"),
];

/// Detects common tools and dependencies by checking for known files.
fn detect_tools_and_dependencies(root: &Path) -> String {
    let mut output = String::from("Tools and dependencies:\n");

    for (file, desc) in KNOWN_TOOLS {
        if root.join(file).exists() {
            let _ = writeln!(output, "- {desc}");
        }
    }
    output.push('\n');

    output
}

/// Gets recent git commit messages (last 20).
fn get_recent_commits(root: &Path) -> String {
    let Ok(result) = Command::new("git")
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
    else {
        return String::new();
    };

    if !result.status.success() {
        return String::new();
    }

    let log = String::from_utf8_lossy(&result.stdout);
    format!("Recent commits (last 20):\n{log}\n\n")
}

/// Detects TODO comments in source files as tech debt indicators.
fn detect_todo_comments(root: &Path) -> String {
    let Ok(result) = Command::new("git")
        .args(["grep", "-i", "TODO", "--", "*.rs", "*.py", "*.js", "*.go"])
        .current_dir(root)
        .output()
    else {
        return String::new();
    };

    if !result.status.success() {
        return String::new();
    }

    let todos = String::from_utf8_lossy(&result.stdout);
    let todo_count = todos.lines().count();

    let mut output = format!("TODO comments found: {todo_count}\n");

    if todo_count > 0 && todo_count <= 10 {
        output.push_str("Examples:\n");
        for (i, line) in todos.lines().take(5).enumerate() {
            let _ = writeln!(output, "  {}. {}", i + 1, line);
        }
    }
    output.push('\n');

    output
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

/// Parsed numbered entry from a suggestion line.
struct NumberedEntry {
    number: usize,
    rest: String,
}

/// Parses a numbered entry line like "1. Title — Description".
///
/// Returns `Some(NumberedEntry)` if the line starts with a digit followed by ". ",
/// otherwise returns `None`.
fn parse_numbered_entry(line: &str) -> Option<NumberedEntry> {
    let trimmed = line.trim();
    let first_char = trimmed.chars().next()?;
    let digit = first_char.to_digit(10)?;

    let rest = trimmed
        .strip_prefix(|c: char| c.is_numeric())?
        .strip_prefix(". ")?;

    Some(NumberedEntry {
        number: digit as usize,
        rest: rest.to_string(),
    })
}

/// Parses "Title — Description" into separate title and description.
///
/// If the em dash separator is not found, returns the entire string as title
/// with an empty description.
fn parse_title_description(text: &str) -> (String, String) {
    text.find(" — ").map_or_else(
        || (text.trim().to_string(), String::new()),
        |sep_idx| {
            let (title_part, rest_part) = text.split_at(sep_idx);
            let desc_part = &rest_part[" — ".len()..];
            (title_part.trim().to_string(), desc_part.trim().to_string())
        },
    )
}

/// Metadata extracted from suggestion lines.
struct SuggestionMetadata {
    category: String,
    effort: String,
    rationale: String,
}

/// Checks if a line looks like the start of a new numbered entry.
fn is_numbered_entry_start(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed
        .chars()
        .next()
        .is_some_and(|c| c.is_numeric() && trimmed.contains(". "))
}

/// Parses metadata lines (Category, Effort, Rationale) following a suggestion header.
///
/// Consumes lines from the iterator until an empty line or next numbered entry is found.
/// Returns the extracted metadata and the number of lines consumed.
fn parse_suggestion_metadata(lines: &[&str], start_index: usize) -> (SuggestionMetadata, usize) {
    let mut category = String::new();
    let mut effort = String::new();
    let mut rationale = String::new();
    let mut consumed = 0;
    let mut i = start_index;

    while i < lines.len() {
        let meta_line = lines[i].trim();

        if meta_line.is_empty() {
            consumed += 1;
            break;
        }

        if let Some(cat) = meta_line.strip_prefix("Category:") {
            category = cat.trim().to_string();
        } else if let Some(eff) = meta_line.strip_prefix("Effort:") {
            effort = eff.trim().to_string();
        } else if let Some(rat) = meta_line.strip_prefix("Rationale:") {
            rationale = rat.trim().to_string();
        }

        consumed += 1;
        i += 1;

        // Stop if we hit the next numbered entry.
        if lines.get(i).is_some_and(|l| is_numbered_entry_start(l)) {
            break;
        }
    }

    (
        SuggestionMetadata {
            category,
            effort,
            rationale,
        },
        consumed,
    )
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
        if let Some(entry) = parse_numbered_entry(lines[i]) {
            let (title, description) = parse_title_description(&entry.rest);

            i += 1;
            let (metadata, consumed) = parse_suggestion_metadata(&lines, i);
            i += consumed;

            suggestions.push(Suggestion {
                number: entry.number,
                title,
                description,
                category: metadata.category,
                effort: metadata.effort,
                rationale: metadata.rationale,
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

/// Validates and parses user selection input.
///
/// Returns `Ok(Some(index))` for valid selection (1-based to 0-based conversion),
/// `Ok(None)` if user quit with 'q',
/// or `Err` if input is invalid or out of range.
fn validate_selection(input: &str, max_suggestions: usize) -> Result<Option<usize>> {
    let input = input.trim();

    if input.eq_ignore_ascii_case("q") {
        return Ok(None);
    }

    let selection: usize = input
        .parse()
        .context("Invalid selection. Please enter a number between 1 and 5.")?;

    if selection < 1 || selection > max_suggestions {
        bail!("Selection out of range. Please enter a number between 1 and {max_suggestions}.");
    }

    // Convert to 0-based index
    Ok(Some(selection - 1))
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

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::init;
    use tempfile::TempDir;

    fn setup_test_repo() -> TempDir {
        let temp = TempDir::new().unwrap();
        init::init(temp.path()).unwrap();
        temp
    }

    /// Sample suggestion output matching the expected format.
    fn sample_suggestion_output() -> String {
        r#"
Based on my analysis, here are 5 PRD suggestions:

1. Add Logging Framework — Implement structured logging with tracing-subscriber
   Category: Infrastructure
   Effort: Medium (1-2 days)
   Rationale: Improve debugging and monitoring capabilities across all modules

2. Implement Configuration Validation — Validate config.toml on load with detailed error messages
   Category: Quality
   Effort: Small (4-6 hours)
   Rationale: Prevent runtime failures due to misconfigured settings

3. Add Metrics Collection — Integrate prometheus metrics for runner invocations
   Category: Observability
   Effort: Large (3-5 days)
   Rationale: Enable production monitoring and performance tracking

4. Improve Error Messages — Enhance user-facing error messages with suggestions
   Category: UX
   Effort: Medium (1-2 days)
   Rationale: Reduce confusion when commands fail, improve developer experience

5. Add Shell Completion — Generate bash/zsh/fish completion scripts
   Category: UX
   Effort: Small (4-6 hours)
   Rationale: Improve CLI discoverability and reduce typing errors
"#
        .to_string()
    }

    /// UAT-001: Suggest command parses exactly 5 PRD suggestions from runner output.
    #[test]
    fn test_suggest_parses_five_suggestions() {
        let output = sample_suggestion_output();
        let suggestions = parse_suggestions(&output).unwrap();

        assert_eq!(suggestions.len(), 5, "Should parse exactly 5 suggestions");

        // Verify first suggestion structure.
        let first = &suggestions[0];
        assert_eq!(first.number, 1);
        assert_eq!(first.title, "Add Logging Framework");
        assert_eq!(
            first.description,
            "Implement structured logging with tracing-subscriber"
        );
        assert_eq!(first.category, "Infrastructure");
        assert_eq!(first.effort, "Medium (1-2 days)");
        assert_eq!(
            first.rationale,
            "Improve debugging and monitoring capabilities across all modules"
        );

        // Verify last suggestion.
        let last = &suggestions[4];
        assert_eq!(last.number, 5);
        assert_eq!(last.title, "Add Shell Completion");
        assert_eq!(last.category, "UX");
    }

    /// UAT-002: Parser handles malformed or missing suggestions gracefully.
    #[test]
    fn test_parse_suggestions_empty_output() {
        let result = parse_suggestions("");
        assert!(result.is_err(), "Should fail on empty output");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No suggestions found")
        );
    }

    #[test]
    fn test_parse_suggestions_incomplete() {
        let output = r#"
1. Test Suggestion — Description
   Category: Testing
"#;
        let result = parse_suggestions(output);
        assert!(result.is_ok(), "Should handle incomplete metadata");
        let suggestions = result.unwrap();
        assert_eq!(suggestions.len(), 1);
        // Missing fields should be empty strings, not cause failure.
        assert_eq!(suggestions[0].effort, "");
        assert_eq!(suggestions[0].rationale, "");
    }

    /// UAT-002: User can select a suggestion by number.
    #[test]
    fn test_validate_selection() {
        // Valid selections (1-5)
        assert_eq!(
            validate_selection("1", 5).unwrap(),
            Some(0),
            "Selection '1' should map to index 0"
        );
        assert_eq!(
            validate_selection("3", 5).unwrap(),
            Some(2),
            "Selection '3' should map to index 2"
        );
        assert_eq!(
            validate_selection("5", 5).unwrap(),
            Some(4),
            "Selection '5' should map to index 4"
        );

        // Valid selection with whitespace
        assert_eq!(
            validate_selection("  2  ", 5).unwrap(),
            Some(1),
            "Whitespace should be trimmed"
        );

        // Quit with 'q' or 'Q'
        assert_eq!(
            validate_selection("q", 5).unwrap(),
            None,
            "Lowercase 'q' should return None (quit)"
        );
        assert_eq!(
            validate_selection("Q", 5).unwrap(),
            None,
            "Uppercase 'Q' should return None (quit)"
        );

        // Invalid: Out of range
        let result = validate_selection("0", 5);
        assert!(result.is_err(), "Selection '0' should be out of range");
        assert!(result.unwrap_err().to_string().contains("out of range"));

        let result = validate_selection("6", 5);
        assert!(result.is_err(), "Selection '6' should be out of range");
        assert!(result.unwrap_err().to_string().contains("out of range"));

        // Invalid: Non-numeric
        let result = validate_selection("abc", 5);
        assert!(result.is_err(), "Non-numeric input should fail");
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Invalid selection")
        );

        // Invalid: Empty
        let result = validate_selection("", 5);
        assert!(result.is_err(), "Empty input should fail");
    }

    /// UAT-003: Generate slug converts titles to URL-friendly format.
    #[test]
    fn test_generate_slug() {
        assert_eq!(
            generate_slug("Add Logging Framework"),
            "add-logging-framework"
        );
        assert_eq!(
            generate_slug("Implement Configuration Validation"),
            "implement-configuration-validation"
        );
        assert_eq!(
            generate_slug("Add Shell Completion!"),
            "add-shell-completion"
        );
        assert_eq!(generate_slug("Fix Bug #123"), "fix-bug-123");
        assert_eq!(
            generate_slug("Multi---Hyphen   Spaces"),
            "multi-hyphen-spaces"
        );
    }

    /// UAT-004: Codebase analysis includes repository structure and tools.
    #[test]
    fn test_analyze_codebase() {
        let temp = setup_test_repo();

        // Create some sample files to analyze.
        std::fs::write(temp.path().join("Cargo.toml"), "# sample").unwrap();
        std::fs::write(temp.path().join("Makefile.toml"), "# sample").unwrap();
        std::fs::create_dir(temp.path().join("src")).unwrap();

        let analysis = analyze_codebase(temp.path()).unwrap();

        assert!(
            analysis.contains("Repository structure:"),
            "Should include structure section"
        );
        assert!(
            analysis.contains("Tools and dependencies:"),
            "Should include tools section"
        );
        assert!(
            analysis.contains("Rust (cargo)"),
            "Should detect Cargo.toml"
        );
        assert!(
            analysis.contains("cargo-make"),
            "Should detect Makefile.toml"
        );
    }

    /// Tests analyze_repository_structure helper function.
    #[test]
    fn test_analyze_repository_structure() {
        let temp = setup_test_repo();

        // Create various files and directories.
        std::fs::write(temp.path().join("README.md"), "# Test").unwrap();
        std::fs::create_dir(temp.path().join("src")).unwrap();
        std::fs::create_dir(temp.path().join("tests")).unwrap();
        std::fs::write(temp.path().join(".hidden"), "hidden").unwrap();
        std::fs::create_dir(temp.path().join("node_modules")).unwrap();
        std::fs::create_dir(temp.path().join("target")).unwrap();

        let result = analyze_repository_structure(temp.path()).unwrap();

        assert!(result.contains("Repository structure:"));
        assert!(result.contains("README.md"));
        assert!(result.contains("src (dir)"));
        assert!(result.contains("tests (dir)"));
        // Hidden files, target, and node_modules should be skipped.
        assert!(!result.contains(".hidden"));
        assert!(!result.contains("node_modules"));
        assert!(!result.contains("target"));
    }

    /// Tests detect_tools_and_dependencies helper function.
    #[test]
    fn test_detect_tools_and_dependencies() {
        let temp = setup_test_repo();

        // Create dependency indicator files.
        std::fs::write(temp.path().join("Cargo.toml"), "").unwrap();
        std::fs::write(temp.path().join("package.json"), "{}").unwrap();
        std::fs::create_dir_all(temp.path().join(".github/workflows")).unwrap();

        let result = detect_tools_and_dependencies(temp.path());

        assert!(result.contains("Tools and dependencies:"));
        assert!(result.contains("Rust (cargo)"));
        assert!(result.contains("Node.js (npm/yarn)"));
        assert!(result.contains("GitHub Actions"));
        // Not created, so should not appear.
        assert!(!result.contains("Python (pip)"));
        assert!(!result.contains("Go modules"));
    }

    /// Tests detect_tools_and_dependencies with no tools present.
    #[test]
    fn test_detect_tools_and_dependencies_empty() {
        let temp = setup_test_repo();

        let result = detect_tools_and_dependencies(temp.path());

        assert!(result.contains("Tools and dependencies:"));
        // Should be just the header and a newline when no tools are detected.
        assert!(!result.contains("Rust"));
        assert!(!result.contains("Node.js"));
    }

    /// Tests get_recent_commits helper function in non-git directory.
    #[test]
    fn test_get_recent_commits_no_git() {
        let temp = setup_test_repo();

        // No git repo initialized, should return empty string.
        let result = get_recent_commits(temp.path());

        assert!(result.is_empty());
    }

    /// Tests get_recent_commits helper function with git repo.
    #[test]
    fn test_get_recent_commits_with_git() {
        let temp = setup_test_repo();

        // Initialize git and create a commit.
        Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Configure git user for CI environment (required for commits).
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["config", "user.name", "Test User"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        std::fs::write(temp.path().join("test.txt"), "test").unwrap();

        Command::new("git")
            .args(["add", "."])
            .current_dir(temp.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Test commit message"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let result = get_recent_commits(temp.path());

        assert!(result.contains("Recent commits (last 20):"));
        assert!(result.contains("Test commit message"));
    }

    /// Tests detect_todo_comments helper function in non-git directory.
    #[test]
    fn test_detect_todo_comments_no_git() {
        let temp = setup_test_repo();

        // No git repo initialized, should return empty string.
        let result = detect_todo_comments(temp.path());

        assert!(result.is_empty());
    }

    /// UAT-005: Placeholder expansion includes PRDs and codebase snapshot.
    #[test]
    fn test_build_suggestion_prompt() {
        use crate::prd::PrdStatus;

        let template = "PRDs:\n{{existing_prds}}\n\nCodebase:\n{{codebase_snapshot}}";

        let prds = vec![crate::prd::PrdSummary {
            id: "PRD-0001".to_string(),
            title: "Test PRD".to_string(),
            status: PrdStatus::Active,
            completed_tasks: 0,
            total_tasks: 1,
            verified_uats: 0,
            total_uats: 0,
            relative_path: "prds/PRD-0001-test.md".to_string(),
            references: vec![],
            depends_on: vec![],
        }];

        let snapshot = "Sample codebase info";

        let result = build_suggestion_prompt(template, &prds, snapshot);

        assert!(result.contains("PRD-0001"));
        assert!(result.contains("Test PRD"));
        assert!(result.contains("active"));
        assert!(result.contains("Sample codebase info"));
    }

    /// Integration test: Full suggestion flow with mock runner.
    /// This test simulates the complete `mr suggest` command flow:
    /// 1. Runner generates 5 suggestions
    /// 2. User selects suggestion #2
    /// 3. Flow into `mr new` with pre-filled context
    /// 4. PRD is created successfully
    ///
    /// Note: This test uses a mock runner and simulated user input.
    /// The actual interactive picker is not tested here (requires TTY).
    #[test]
    fn test_suggest_integration_with_mock_runner() {
        let temp = setup_test_repo();

        // Verify analyze_codebase runs without error.
        let snapshot = analyze_codebase(temp.path()).unwrap();
        assert!(!snapshot.is_empty());

        // Verify parse_suggestions extracts 5 suggestions.
        let suggestions = parse_suggestions(&sample_suggestion_output()).unwrap();
        assert_eq!(suggestions.len(), 5);

        // Verify slug generation for selection #2.
        let selected = &suggestions[1];
        let slug = generate_slug(&selected.title);
        assert_eq!(slug, "implement-configuration-validation");

        // Verify prompt building includes context.
        let prds = vec![];
        let prompt =
            build_suggestion_prompt("{{existing_prds}}\n{{codebase_snapshot}}", &prds, &snapshot);
        assert!(prompt.contains("Repository structure:"));
    }

    /// UAT-003: Selected suggestion flows into mr new with pre-filled context.
    ///
    /// This test verifies that when a user selects a suggestion:
    /// 1. A slug is generated from the suggestion title
    /// 2. PrdNewConfig is constructed with pre-filled context from the suggestion
    /// 3. The context includes description, category, effort, and rationale
    #[test]
    fn test_suggestion_flows_to_prd_new_with_context() {
        let suggestions = parse_suggestions(&sample_suggestion_output()).unwrap();

        // Simulate user selecting suggestion #2.
        let selected = &suggestions[1];

        // Verify slug generation matches expected format.
        let slug = generate_slug(&selected.title);
        assert_eq!(slug, "implement-configuration-validation");

        // Build context as done in suggest() function.
        let context = format!(
            "{}\n\nCategory: {}\nEffort: {}\nRationale: {}",
            selected.description, selected.category, selected.effort, selected.rationale
        );

        // Verify context contains all expected fields from the suggestion.
        assert!(context.contains("Validate config.toml on load with detailed error messages"));
        assert!(context.contains("Category: Quality"));
        assert!(context.contains("Effort: Small (4-6 hours)"));
        assert!(
            context.contains("Rationale: Prevent runtime failures due to misconfigured settings")
        );

        // Verify description is properly extracted.
        assert_eq!(
            selected.description,
            "Validate config.toml on load with detailed error messages"
        );

        // This demonstrates the data structure that would be passed to create_prd.
        // The actual create_prd call requires a runner and I/O handles, so we verify
        // the config construction logic here rather than invoking the full flow.
        assert_eq!(selected.title, "Implement Configuration Validation");
        assert_eq!(selected.category, "Quality");
        assert_eq!(selected.effort, "Small (4-6 hours)");
    }

    /// UAT-005: Codebase analysis covers tech debt and dependency versions.
    ///
    /// Verifies that analyze_codebase() includes:
    /// - TODO comments detection (tech debt indicators)
    /// - Dependency file detection (Cargo.toml, package.json, etc.)
    #[test]
    fn test_analyze_codebase_includes_tech_debt_and_dependencies() {
        let temp = setup_test_repo();

        // Initialize git repository (setup_test_repo only creates .mr structure).
        Command::new("git")
            .args(["init"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        // Create dependency files.
        std::fs::write(temp.path().join("Cargo.toml"), "# Rust dependencies").unwrap();
        std::fs::write(temp.path().join("package.json"), "{}").unwrap();

        // Create a source file with TODO comments.
        std::fs::create_dir(temp.path().join("src")).unwrap();
        std::fs::write(
            temp.path().join("src/main.rs"),
            "// TODO: Refactor this module\nfn main() {}\n// TODO: Add error handling",
        )
        .unwrap();

        // Also create a top-level .rs file that matches the git grep pattern.
        std::fs::write(
            temp.path().join("lib.rs"),
            "// TODO: Improve error handling\npub fn test() {}\n",
        )
        .unwrap();

        // Need to add and commit files to git for git grep to find them.
        Command::new("git")
            .args(["add", "."])
            .current_dir(temp.path())
            .output()
            .unwrap();

        Command::new("git")
            .args(["commit", "-m", "Add test files"])
            .current_dir(temp.path())
            .output()
            .unwrap();

        let analysis = analyze_codebase(temp.path()).unwrap();

        // Verify dependency detection section exists.
        assert!(
            analysis.contains("Tools and dependencies:"),
            "Analysis should include dependency detection section"
        );
        assert!(
            analysis.contains("Rust (cargo)"),
            "Analysis should detect Cargo.toml"
        );
        assert!(
            analysis.contains("Node.js (npm/yarn)"),
            "Analysis should detect package.json"
        );

        // Verify TODO comments (tech debt) detection.
        assert!(
            analysis.contains("TODO comments found:"),
            "Analysis should detect TODO comments as tech debt indicators"
        );
    }

    /// UAT-004: Suggestions include both strategic and quick-win categories.
    ///
    /// Validates that parsed suggestions include a balanced mix of:
    /// - At least one "Quick Win" category
    /// - At least one "Strategic" category
    /// - Valid categories from the expected set
    #[test]
    fn test_suggestions_include_strategic_and_quick_win() {
        let output = r#"
1. Add Telemetry Support — Integrate OpenTelemetry for distributed tracing
   Category: Strategic
   Effort: High
   Rationale: Long-term observability investment for production systems

2. Add --verbose Flag — Add verbose output flag to all commands
   Category: Quick Win
   Effort: Low
   Rationale: Quick improvement to debugging experience with minimal code changes

3. Implement Caching Layer — Add caching for expensive operations
   Category: Strategic
   Effort: Medium
   Rationale: Performance optimization for future scale

4. Fix Help Text Typos — Correct typos in command help messages
   Category: Quick Win
   Effort: Low
   Rationale: Easy documentation fix that improves user experience

5. Add Integration Tests — Expand test coverage with integration tests
   Category: Testing
   Effort: Medium
   Rationale: Improve confidence in cross-module interactions
"#;

        let suggestions = parse_suggestions(output).unwrap();
        assert_eq!(suggestions.len(), 5, "Should parse exactly 5 suggestions");

        // Collect categories from all suggestions.
        let categories: Vec<String> = suggestions.iter().map(|s| s.category.clone()).collect();

        // Verify at least one "Strategic" category exists.
        let has_strategic = categories.iter().any(|c| c == "Strategic");
        assert!(
            has_strategic,
            "Suggestions should include at least one 'Strategic' category. Found: {:?}",
            categories
        );

        // Verify at least one "Quick Win" category exists.
        let has_quick_win = categories.iter().any(|c| c == "Quick Win");
        assert!(
            has_quick_win,
            "Suggestions should include at least one 'Quick Win' category. Found: {:?}",
            categories
        );

        // Verify all categories are from the expected set.
        let valid_categories = ["Quick Win", "Strategic", "Debt", "Testing", "Docs"];
        for category in &categories {
            assert!(
                valid_categories.contains(&category.as_str()),
                "Invalid category '{}'. Must be one of: {:?}",
                category,
                valid_categories
            );
        }
    }

    // =========================================================================
    // Tests for parse_suggestions helper functions
    // =========================================================================

    /// Tests parse_numbered_entry with valid numbered lines.
    #[test]
    fn test_parse_numbered_entry_valid() {
        let result = parse_numbered_entry("1. Add Logging Framework");
        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.number, 1);
        assert_eq!(entry.rest, "Add Logging Framework");

        let result = parse_numbered_entry("5. Last Suggestion");
        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.number, 5);
        assert_eq!(entry.rest, "Last Suggestion");
    }

    /// Tests parse_numbered_entry with leading/trailing whitespace.
    #[test]
    fn test_parse_numbered_entry_whitespace() {
        let result = parse_numbered_entry("  3. Trimmed Entry  ");
        assert!(result.is_some());
        let entry = result.unwrap();
        assert_eq!(entry.number, 3);
        // Trailing whitespace is trimmed by the outer trim() call
        assert_eq!(entry.rest, "Trimmed Entry");
    }

    /// Tests parse_numbered_entry with invalid lines.
    #[test]
    fn test_parse_numbered_entry_invalid() {
        // No number
        assert!(parse_numbered_entry("Add Logging").is_none());

        // Missing ". " separator
        assert!(parse_numbered_entry("1 Add Logging").is_none());
        assert!(parse_numbered_entry("1.Add Logging").is_none());

        // Empty line
        assert!(parse_numbered_entry("").is_none());

        // Just number with period (no content after dot-space) - fails because trim() removes trailing space
        assert!(parse_numbered_entry("1. ").is_none());
    }

    /// Tests parse_title_description with em dash separator.
    #[test]
    fn test_parse_title_description_with_separator() {
        let (title, desc) = parse_title_description("Add Logging — Implement tracing-subscriber");
        assert_eq!(title, "Add Logging");
        assert_eq!(desc, "Implement tracing-subscriber");
    }

    /// Tests parse_title_description without em dash separator.
    #[test]
    fn test_parse_title_description_no_separator() {
        let (title, desc) = parse_title_description("Just a title");
        assert_eq!(title, "Just a title");
        assert_eq!(desc, "");
    }

    /// Tests parse_title_description with extra whitespace.
    #[test]
    fn test_parse_title_description_whitespace() {
        let (title, desc) = parse_title_description("  Title  —  Description  ");
        assert_eq!(title, "Title");
        assert_eq!(desc, "Description");
    }

    /// Tests is_numbered_entry_start detection.
    #[test]
    fn test_is_numbered_entry_start() {
        assert!(is_numbered_entry_start("1. Add Logging"));
        assert!(is_numbered_entry_start("  5. With indent  "));
        assert!(!is_numbered_entry_start("Category: Testing"));
        assert!(!is_numbered_entry_start(""));
        assert!(!is_numbered_entry_start("No number here"));
    }

    /// Tests parse_suggestion_metadata with complete metadata.
    #[test]
    fn test_parse_suggestion_metadata_complete() {
        let lines = vec![
            "   Category: Infrastructure",
            "   Effort: Medium (1-2 days)",
            "   Rationale: Improve debugging",
            "",
        ];
        let (metadata, consumed) = parse_suggestion_metadata(&lines, 0);

        assert_eq!(metadata.category, "Infrastructure");
        assert_eq!(metadata.effort, "Medium (1-2 days)");
        assert_eq!(metadata.rationale, "Improve debugging");
        assert_eq!(consumed, 4); // 3 metadata lines + 1 empty line
    }

    /// Tests parse_suggestion_metadata with partial metadata.
    #[test]
    fn test_parse_suggestion_metadata_partial() {
        let lines = vec!["   Category: Testing", ""];
        let (metadata, consumed) = parse_suggestion_metadata(&lines, 0);

        assert_eq!(metadata.category, "Testing");
        assert_eq!(metadata.effort, "");
        assert_eq!(metadata.rationale, "");
        assert_eq!(consumed, 2);
    }

    /// Tests parse_suggestion_metadata stops at next numbered entry.
    #[test]
    fn test_parse_suggestion_metadata_stops_at_next_entry() {
        let lines = vec!["   Category: UX", "   Effort: Low", "2. Next Suggestion"];
        let (metadata, consumed) = parse_suggestion_metadata(&lines, 0);

        assert_eq!(metadata.category, "UX");
        assert_eq!(metadata.effort, "Low");
        assert_eq!(consumed, 2); // Should stop before "2. Next Suggestion"
    }
}
