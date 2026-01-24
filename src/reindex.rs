//! Reindex logic for `mr reindex`.
//!
//! Regenerates the `.mr/PRDS.md` index and verifies/fixes inter-PRD and code links.

use std::collections::HashMap;
use std::path::Path;

use anyhow::Result;

use crate::prd::{generate_index_from_root, scan_prds};
use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;

/// Result of reindexing operation.
#[derive(Debug, Default)]
pub struct ReindexResult {
    /// Number of PRDs indexed.
    pub prds_indexed: usize,

    /// Number of links verified.
    pub links_verified: usize,

    /// Number of links fixed.
    pub links_fixed: usize,
}

/// PRD file info for prompt context.
#[derive(Debug, Clone)]
struct PrdFileInfo {
    filename: String,
    id: String,
    title: String,
}

/// Runs the reindex operation.
///
/// 1. Regenerates `.mr/PRDS.md` index.
/// 2. Invokes runner to scan and fix links in PRDs.
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

    // Step 3: Build prompt context.
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

    // Step 4: Run the runner with the prompt.
    tracing::info!("Invoking runner to verify and fix links...");

    let result = if stream {
        let mut stdout = std::io::stdout();
        runner.execute_streaming(&prompt, root, &mut stdout)?
    } else {
        runner.execute(&prompt, root)?
    };

    let output = result.text;

    // Step 5: Parse output to extract counts (best effort).
    let (links_verified, links_fixed) = parse_link_counts(&output);

    Ok(ReindexResult {
        prds_indexed,
        links_verified,
        links_fixed,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_link_counts() {
        let output = r#"
Reindex complete!
Links verified: 12
Links fixed: 3
"#;
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
}
