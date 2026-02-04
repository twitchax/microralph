//! Reindex logic for `mr reindex`.
//!
//! Regenerates the `.mr/PRDS.md` index, verifies/fixes inter-PRD and code links,
//! and auto-fixes `depends_on` relationships using LLM analysis.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::prd::{generate_index_from_root, scan_prds};
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;
use crate::util::spinner::start_spinner;

/// Result of reindexing operation.
#[derive(Debug, Default)]
pub struct ReindexResult {
    /// Number of PRDs indexed.
    pub prds_indexed: usize,

    /// Number of links verified.
    pub links_verified: usize,

    /// Number of links fixed.
    pub links_fixed: usize,

    /// Number of `depends_on` relationships added.
    pub depends_on_added: usize,

    /// Number of `depends_on` relationships fixed (invalid refs removed).
    pub depends_on_fixed: usize,
}

/// PRD file info for prompt context.
#[derive(Debug, Clone)]
struct PrdFileInfo {
    filename: String,
    id: String,
    title: String,
}

/// Extended PRD info for `depends_on` analysis.
#[derive(Debug, Clone)]
struct PrdDependsOnInfo {
    filename: String,
    id: String,
    title: String,
    status: String,
    created: String,
    depends_on: String,
    summary: String,
}

/// Runs the reindex operation.
///
/// 1. Regenerates `.mr/PRDS.md` index.
/// 2. Invokes runner to scan and fix links in PRDs.
/// 3. Invokes runner to auto-fix `depends_on` relationships using LLM analysis.
///
/// # Arguments
///
/// * `root` - Repository root directory
/// * `runner` - The runner to use for link verification/fixing
/// * `stream` - Whether to stream runner output
pub fn reindex(root: impl AsRef<Path>, runner: &dyn Runner, stream: bool) -> Result<ReindexResult> {
    let root = root.as_ref();
    let prds_dir = root.join(".mr").join("prds");

    // Step 1: Regenerate the index.
    let prds_indexed = generate_index_from_root(root)?;
    tracing::info!(prds_indexed, "Regenerated PRD index");

    // If no PRDs, we're done.
    if prds_indexed == 0 {
        return Ok(ReindexResult {
            prds_indexed,
            ..Default::default()
        });
    }

    // Step 2: Collect PRD info for prompt context.
    let prds = scan_prds(&prds_dir)?;
    let prd_files: Vec<PrdFileInfo> = prds
        .iter()
        .map(|(filename, prd, _)| PrdFileInfo {
            filename: filename.clone(),
            id: prd.id().to_string(),
            title: prd.title().to_string(),
        })
        .collect();

    // Step 3: Build prompt context for link verification.
    let prompt_template = load_prompt_with_fallback(root, PromptKind::Reindex);

    // Build list for {{#each prd_files}}.
    let prd_files_list: Vec<HashMap<String, String>> = prd_files
        .iter()
        .map(|info| {
            let mut map = HashMap::new();
            map.insert("filename".to_string(), info.filename.clone());
            map.insert("id".to_string(), info.id.clone());
            map.insert("title".to_string(), info.title.clone());
            map
        })
        .collect();

    let mut context = PlaceholderContext::new();
    context.insert("prds_dir", prds_dir.display().to_string());
    context.insert("repo_root", root.display().to_string());
    context.insert("prd_files", PlaceholderValue::List(prd_files_list));

    let prompt = expand_placeholders(&prompt_template, &context);

    // Step 4: Run the runner with the link verification prompt.
    tracing::info!("Invoking runner to verify and fix links...");

    // Print command info before spinner (only when not streaming).
    if !stream && let Some(cmd_display) = runner.format_command_display(&prompt, root) {
        println!("\n🔧 Executing: {cmd_display}");
    }

    let spinner = start_spinner(!stream, "Verifying links...");

    let result = if stream {
        let mut stdout = std::io::stdout();
        runner.execute_streaming(&prompt, root, &mut stdout)?
    } else {
        runner.execute(&prompt, root)?
    };

    spinner.finish_and_clear();

    let output = result.text;

    // Step 5: Parse output to extract link counts (best effort).
    let (links_verified, links_fixed) = parse_link_counts(&output);

    // Step 6: Run depends_on auto-fix phase.
    let (depends_on_added, depends_on_fixed) =
        run_depends_on_fix(root, &prds_dir, &prds, runner, stream)?;

    Ok(ReindexResult {
        prds_indexed,
        links_verified,
        links_fixed,
        depends_on_added,
        depends_on_fixed,
    })
}

/// Parses the runner output to extract link verification/fix counts.
///
/// This is a best-effort extraction. If the runner output doesn't contain
/// parseable counts, returns (0, 0).
fn parse_link_counts(output: &str) -> (usize, usize) {
    let mut verified = 0;
    let mut fixed = 0;

    // Look for patterns like "Links verified: 5" or "verified 5 links"
    for line in output.lines() {
        let lower = line.to_lowercase();

        if lower.contains("verified")
            && let Some(num) = extract_first_number(line)
        {
            verified = num;
        }

        if lower.contains("fixed")
            && let Some(num) = extract_first_number(line)
        {
            fixed = num;
        }
    }

    (verified, fixed)
}

/// Extracts the first number from a string.
fn extract_first_number(s: &str) -> Option<usize> {
    let mut num_str = String::new();
    let mut in_number = false;

    for c in s.chars() {
        if c.is_ascii_digit() {
            in_number = true;
            num_str.push(c);
        } else if in_number {
            break;
        }
    }

    num_str.parse().ok()
}

use crate::prd::Prd;
use std::path::PathBuf;

/// Runs the `depends_on` auto-fix phase.
///
/// Invokes the runner with the `ReindexDependsOn` prompt to analyze PRDs
/// and infer/fix `depends_on` relationships.
fn run_depends_on_fix(
    root: &Path,
    prds_dir: &Path,
    prds: &[(String, Prd, PathBuf)],
    runner: &dyn Runner,
    stream: bool,
) -> Result<(usize, usize)> {
    tracing::info!("Invoking runner to auto-fix depends_on relationships...");

    // Build extended PRD info for depends_on analysis.
    let prd_info: Vec<PrdDependsOnInfo> = prds
        .iter()
        .map(|(filename, prd, _)| {
            // Get depends_on as comma-separated string.
            let depends_on = prd
                .frontmatter
                .depends_on
                .as_ref()
                .map(|deps| deps.join(", "))
                .unwrap_or_default();

            // Extract a brief summary from the body (first non-empty paragraph).
            let summary = extract_summary(&prd.body);

            PrdDependsOnInfo {
                filename: filename.clone(),
                id: prd.id().to_string(),
                title: prd.title().to_string(),
                status: prd.status().to_string(),
                created: prd
                    .frontmatter
                    .created
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string()),
                depends_on,
                summary,
            }
        })
        .collect();

    // Build prompt context for depends_on analysis.
    let prompt_template = load_prompt_with_fallback(root, PromptKind::ReindexDependsOn);

    let prd_files_list: Vec<HashMap<String, String>> = prd_info
        .iter()
        .map(|info| {
            let mut map = HashMap::new();
            map.insert("filename".to_string(), info.filename.clone());
            map.insert("id".to_string(), info.id.clone());
            map.insert("title".to_string(), info.title.clone());
            map.insert("status".to_string(), info.status.clone());
            map.insert("created".to_string(), info.created.clone());
            map.insert("depends_on".to_string(), info.depends_on.clone());
            map.insert("summary".to_string(), info.summary.clone());
            map
        })
        .collect();

    let mut context = PlaceholderContext::new();
    context.insert("prds_dir", prds_dir.display().to_string());
    context.insert("repo_root", root.display().to_string());
    context.insert("prd_files", PlaceholderValue::List(prd_files_list));

    let prompt = expand_placeholders(&prompt_template, &context);

    // Print command info before spinner (only when not streaming).
    if !stream && let Some(cmd_display) = runner.format_command_display(&prompt, root) {
        println!("\n🔧 Executing: {cmd_display}");
    }

    let spinner = start_spinner(!stream, "Analyzing depends_on relationships...");

    let result = if stream {
        let mut stdout = std::io::stdout();
        runner.execute_streaming(&prompt, root, &mut stdout)?
    } else {
        runner.execute(&prompt, root)?
    };

    spinner.finish_and_clear();

    let output = result.text;

    // Parse output to extract depends_on counts (best effort).
    let (added, fixed) = parse_depends_on_counts(&output);

    Ok((added, fixed))
}

/// Extracts a brief summary from PRD body text.
///
/// Returns the first non-empty paragraph (up to 200 chars).
fn extract_summary(body: &str) -> String {
    // Find first substantial line after any headers.
    for line in body.lines() {
        let trimmed = line.trim();

        // Skip empty lines and headers.
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with("---") {
            continue;
        }

        // Return first 200 chars of first real content.
        if trimmed.len() > 200 {
            return format!("{}...", &trimmed[..197]);
        }

        return trimmed.to_string();
    }

    "(no summary available)".to_string()
}

/// Parses the runner output to extract `depends_on` fix counts.
///
/// This is a best-effort extraction. If the runner output doesn't contain
/// parseable counts, returns (0, 0).
fn parse_depends_on_counts(output: &str) -> (usize, usize) {
    let mut added = 0;
    let mut fixed = 0;

    for line in output.lines() {
        let lower = line.to_lowercase();

        // Look for patterns like "depends_on added: 5" or "added 5 depends_on"
        if (lower.contains("added") || lower.contains("relationships added"))
            && lower.contains("depends")
            && let Some(num) = extract_first_number(line)
        {
            added = num;
        }

        // Look for patterns like "depends_on fixed: 3" or "fixed 3 invalid refs"
        if (lower.contains("fixed") || lower.contains("removed"))
            && (lower.contains("depends") || lower.contains("invalid"))
            && let Some(num) = extract_first_number(line)
        {
            fixed = num;
        }
    }

    (added, fixed)
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_link_counts() {
        let output = r"
Reindex complete!
Links verified: 12
Links fixed: 3
";
        let (verified, fixed) = parse_link_counts(output);
        assert_eq!(verified, 12);
        assert_eq!(fixed, 3);
    }

    #[test]
    fn test_parse_link_counts_alternative_format() {
        let output = "Verified 5 inter-PRD links.\nFixed 2 broken links.";
        let (verified, fixed) = parse_link_counts(output);
        assert_eq!(verified, 5);
        assert_eq!(fixed, 2);
    }

    #[test]
    fn test_parse_link_counts_no_matches() {
        let output = "Done processing.";
        let (verified, fixed) = parse_link_counts(output);
        assert_eq!(verified, 0);
        assert_eq!(fixed, 0);
    }

    #[test]
    fn test_extract_first_number() {
        assert_eq!(extract_first_number("verified 42 links"), Some(42));
        assert_eq!(extract_first_number("Links: 123"), Some(123));
        assert_eq!(extract_first_number("no numbers here"), None);
        assert_eq!(extract_first_number(""), None);
    }

    #[test]
    fn test_parse_depends_on_counts() {
        let output = r"
Reindex depends_on complete!
depends_on relationships added: 5
depends_on relationships fixed: 2
";
        let (added, fixed) = parse_depends_on_counts(output);
        assert_eq!(added, 5);
        assert_eq!(fixed, 2);
    }

    #[test]
    fn test_parse_depends_on_counts_alternative_format() {
        let output = "Added 3 depends_on entries.\nFixed 1 invalid ref.";
        let (added, fixed) = parse_depends_on_counts(output);
        assert_eq!(added, 3);
        assert_eq!(fixed, 1);
    }

    #[test]
    fn test_parse_depends_on_counts_no_matches() {
        let output = "Done processing.";
        let (added, fixed) = parse_depends_on_counts(output);
        assert_eq!(added, 0);
        assert_eq!(fixed, 0);
    }

    #[test]
    fn test_extract_summary() {
        let body = r"
# Summary

This is the first paragraph of content that should be extracted as the summary.

More content follows here.
";
        let summary = extract_summary(body);
        assert_eq!(
            summary,
            "This is the first paragraph of content that should be extracted as the summary."
        );
    }

    #[test]
    fn test_extract_summary_long_text() {
        let body = "A".repeat(250);
        let summary = extract_summary(&body);
        assert!(summary.len() <= 200);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_extract_summary_empty_body() {
        let summary = extract_summary("");
        assert_eq!(summary, "(no summary available)");
    }

    #[test]
    fn test_extract_summary_headers_only() {
        let body = "# Header\n## Another Header\n---";
        let summary = extract_summary(body);
        assert_eq!(summary, "(no summary available)");
    }

    #[test]
    fn test_reindex_integration_depends_on_autofix() {
        // Integration test: Verify reindex invokes depends_on auto-fix via runner.
        use crate::runner::{MockRunner, RunnerOutput};
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Initialize .mr structure.
        crate::init::init(root).unwrap();

        // Create test PRDs without depends_on for the runner to fix.
        let prd1 = r#"---
id: PRD-0001
title: "Initial Setup"
status: done
owner: Test
created: 2026-01-01
updated: 2026-01-01
tasks:
- id: T-001
  title: "Setup task"
  priority: 1
  status: done
---

# Summary

Initial project setup that other PRDs depend on.
"#;

        let prd2 = r#"---
id: PRD-0002
title: "Add Auth"
status: done
owner: Test
created: 2026-01-02
updated: 2026-01-02
tasks:
- id: T-001
  title: "Auth task"
  priority: 1
  status: done
---

# Summary

Authentication feature that builds on initial setup.
"#;

        let prds_dir = root.join(".mr/prds");
        std::fs::write(prds_dir.join("PRD-0001-initial-setup.md"), prd1).unwrap();
        std::fs::write(prds_dir.join("PRD-0002-add-auth.md"), prd2).unwrap();

        // Mock runner returns responses for both link verification and depends_on fix.
        // First call: link verification. Second call: depends_on auto-fix.
        let runner = MockRunner::new(vec![
            RunnerOutput::success("Links verified: 3\nLinks fixed: 0"),
            RunnerOutput::success(
                "Analyzed PRD dependencies.\ndepends_on relationships added: 1\ndepends_on fixed: 0",
            ),
        ]);

        let result = reindex(root, &runner, false).unwrap();

        // Verify reindex completed with expected counts.
        assert_eq!(result.prds_indexed, 2);
        assert_eq!(result.links_verified, 3);
        assert_eq!(result.links_fixed, 0);
        assert_eq!(result.depends_on_added, 1);
        assert_eq!(result.depends_on_fixed, 0);

        // Verify runner was called twice (link + depends_on phases).
        let prompts = runner.recorded_prompts();
        assert_eq!(prompts.len(), 2);

        // Second prompt should be for depends_on analysis.
        assert!(
            prompts[1].contains("depends_on") || prompts[1].contains("dependency"),
            "Second prompt should be for depends_on analysis"
        );
    }

    #[test]
    fn test_reindex_integration_depends_on_with_existing_deps() {
        // Integration test: Verify reindex handles PRDs that already have depends_on.
        use crate::runner::{MockRunner, RunnerOutput};
        use tempfile::TempDir;

        let temp = TempDir::new().unwrap();
        let root = temp.path();

        // Initialize .mr structure.
        crate::init::init(root).unwrap();

        // Create a PRD with existing depends_on.
        let prd_with_deps = r#"---
id: PRD-0003
title: "Feature with Dependencies"
status: active
owner: Test
created: 2026-01-03
updated: 2026-01-03
depends_on:
- PRD-0001
- PRD-0002
tasks:
- id: T-001
  title: "Feature task"
  priority: 1
  status: todo
---

# Summary

A feature that depends on PRD-0001 and PRD-0002.
"#;

        let prds_dir = root.join(".mr/prds");
        std::fs::write(prds_dir.join("PRD-0003-feature.md"), prd_with_deps).unwrap();

        // Mock runner: link phase + depends_on phase (may fix invalid refs).
        let runner = MockRunner::new(vec![
            RunnerOutput::success("Links verified: 1\nLinks fixed: 0"),
            RunnerOutput::success(
                "Analyzed dependencies.\ndepends_on relationships added: 0\nRemoved 2 invalid depends_on refs.",
            ),
        ]);

        let result = reindex(root, &runner, false).unwrap();

        // Verify the result includes depends_on fixed count.
        assert_eq!(result.prds_indexed, 1);
        assert_eq!(result.depends_on_added, 0);
        assert_eq!(result.depends_on_fixed, 2);

        // Verify the depends_on prompt includes existing dependency info.
        let prompts = runner.recorded_prompts();
        assert!(
            prompts[1].contains("PRD-0001") && prompts[1].contains("PRD-0002"),
            "Depends_on prompt should include existing dependency info"
        );
    }
}
