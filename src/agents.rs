//! AGENTS.md updater for microralph.
//!
//! This module provides functionality to safely update the auto-managed section
//! of AGENTS.md after PRD creation or task completion.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{Context, Result};

use crate::prompt::{
    PlaceholderContext, PlaceholderValue, PromptKind, expand_placeholders,
    load_prompt_with_fallback,
};
use crate::runner::Runner;

/// Marker for the start of the auto-managed section.
const AUTO_MANAGED_START: &str = "<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->";

/// Marker for the end of the auto-managed section.
const AUTO_MANAGED_END: &str = "<!-- END MICRORALPH AUTO-MANAGED SECTION -->";

/// Signal from the runner that no changes are needed.
const NO_CHANGES_SIGNAL: &str = "NO_CHANGES";

/// Represents a recent change to be included in the AGENTS.md update prompt.
#[derive(Debug, Clone)]
pub struct RecentChange {
    /// The file that was changed.
    pub file: String,

    /// A brief description of the change.
    pub description: String,
}

/// Result of an AGENTS.md update operation.
#[derive(Debug)]
pub struct AgentsUpdateResult {
    /// Whether the file was modified.
    pub modified: bool,

    /// The new content of the auto-managed section (if modified).
    pub new_content: Option<String>,
}

/// Reads the current AGENTS.md content.
///
/// Returns an empty string if the file doesn't exist.
pub fn read_agents_file(root: &Path) -> Result<String> {
    let agents_path = root.join("AGENTS.md");

    if !agents_path.exists() {
        return Ok(String::new());
    }

    std::fs::read_to_string(&agents_path).context("Failed to read AGENTS.md")
}

/// Extracts the auto-managed section content from AGENTS.md.
///
/// Returns the content between the start and end markers, or None if not found.
pub fn extract_auto_managed_section(content: &str) -> Option<&str> {
    let start = content.find(AUTO_MANAGED_START)?;
    let end = content.find(AUTO_MANAGED_END)?;

    if start >= end {
        return None;
    }

    // Get content after the start marker.
    let after_start = start + AUTO_MANAGED_START.len();

    // Trim leading newline if present.
    let section_start = if content[after_start..].starts_with('\n') {
        after_start + 1
    } else {
        after_start
    };

    // Trim trailing whitespace before end marker.
    let section_end = content[..end].trim_end().len();

    if section_start >= section_end {
        return Some("");
    }

    Some(&content[section_start..section_end])
}

/// Patches the auto-managed section in AGENTS.md content.
///
/// Replaces content between the start and end markers with new content.
/// Preserves everything outside the markers.
pub fn patch_auto_managed_section(content: &str, new_section: &str) -> Option<String> {
    let start = content.find(AUTO_MANAGED_START)?;
    let end = content.find(AUTO_MANAGED_END)?;

    if start >= end {
        return None;
    }

    // Build the new content.
    let before = &content[..start + AUTO_MANAGED_START.len()];
    let after = &content[end..];

    // Format the new section with proper spacing.
    let new_section_trimmed = new_section.trim();

    let patched = if new_section_trimmed.is_empty() {
        format!("{before}\n{after}")
    } else {
        format!("{before}\n{new_section_trimmed}\n{after}")
    };

    Some(patched)
}

/// Builds the prompt for updating AGENTS.md.
fn build_update_agents_prompt(
    root: &Path,
    agents_content: &str,
    changes: &[RecentChange],
) -> String {
    let template = load_prompt_with_fallback(root, PromptKind::UpdateAgents);

    let mut ctx = PlaceholderContext::new();

    ctx.insert("agents_content", agents_content);

    // Build recent changes list.
    let changes_list: Vec<HashMap<String, String>> = changes
        .iter()
        .map(|c| {
            [
                ("file".to_string(), c.file.clone()),
                ("description".to_string(), c.description.clone()),
            ]
            .into_iter()
            .collect()
        })
        .collect();

    ctx.insert("recent_changes", PlaceholderValue::List(changes_list));

    expand_placeholders(&template, &ctx)
}

/// Parses the runner output to extract new section content.
///
/// Handles:
/// - "NO_CHANGES" signal
/// - Markdown code blocks
/// - Plain text
fn parse_update_response(output: &str) -> Option<String> {
    let trimmed = output.trim();

    // Check for no changes signal.
    if trimmed.contains(NO_CHANGES_SIGNAL) {
        return None;
    }

    // Check for markdown code block.
    if let Some(start) = trimmed.find("```markdown")
        && let Some(end) = trimmed[start + 11..].find("```")
    {
        return Some(trimmed[start + 11..start + 11 + end].trim().to_string());
    }

    if let Some(start) = trimmed.find("```") {
        let after_first = start + 3;

        // Skip the language identifier if present.
        let content_start = trimmed[after_first..]
            .find('\n')
            .map(|i| after_first + i + 1)
            .unwrap_or(after_first);

        if let Some(end) = trimmed[content_start..].find("```") {
            return Some(
                trimmed[content_start..content_start + end]
                    .trim()
                    .to_string(),
            );
        }
    }

    // Use the whole output as the new section.
    Some(trimmed.to_string())
}

/// Updates the AGENTS.md file with new content for the auto-managed section.
///
/// # Arguments
///
/// * `root` - The repository root directory
/// * `runner` - The runner to use for generating updates
/// * `changes` - Recent changes to include in the prompt
///
/// # Returns
///
/// An `AgentsUpdateResult` indicating whether the file was modified.
pub fn update_agents_md<R>(
    root: &Path,
    runner: &R,
    changes: &[RecentChange],
) -> Result<AgentsUpdateResult>
where
    R: Runner + ?Sized,
{
    let agents_path = root.join("AGENTS.md");

    // Read current content.
    let current_content = read_agents_file(root)?;

    if current_content.is_empty() {
        tracing::debug!("AGENTS.md not found, skipping update");
        return Ok(AgentsUpdateResult {
            modified: false,
            new_content: None,
        });
    }

    // Check if auto-managed section exists.
    if extract_auto_managed_section(&current_content).is_none() {
        tracing::debug!("No auto-managed section found in AGENTS.md, skipping update");
        return Ok(AgentsUpdateResult {
            modified: false,
            new_content: None,
        });
    }

    // Build and execute the prompt.
    let prompt = build_update_agents_prompt(root, &current_content, changes);

    tracing::debug!(
        prompt_len = prompt.len(),
        "Invoking runner for AGENTS.md update"
    );

    let output = runner
        .execute(&prompt, root)
        .map_err(|e| anyhow::anyhow!("Runner failed during AGENTS.md update: {e}"))?;

    if !output.success {
        tracing::warn!("Runner failed during AGENTS.md update, skipping");
        return Ok(AgentsUpdateResult {
            modified: false,
            new_content: None,
        });
    }

    // Parse the response.
    let Some(new_section) = parse_update_response(&output.text) else {
        tracing::debug!("No changes needed for AGENTS.md");
        return Ok(AgentsUpdateResult {
            modified: false,
            new_content: None,
        });
    };

    // Patch the content.
    let Some(patched_content) = patch_auto_managed_section(&current_content, &new_section) else {
        tracing::warn!("Failed to patch AGENTS.md auto-managed section");
        return Ok(AgentsUpdateResult {
            modified: false,
            new_content: None,
        });
    };

    // Write the updated content.
    std::fs::write(&agents_path, &patched_content).context("Failed to write AGENTS.md")?;

    tracing::info!("Updated AGENTS.md auto-managed section");

    Ok(AgentsUpdateResult {
        modified: true,
        new_content: Some(new_section),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::{MockRunner, RunnerOutput};
    use tempfile::TempDir;

    fn create_agents_file(root: &Path, content: &str) {
        std::fs::write(root.join("AGENTS.md"), content).unwrap();
    }

    fn create_prompts_dir(root: &Path) {
        let prompts_dir = root.join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();

        std::fs::write(
            prompts_dir.join("update_agents.md"),
            "Update AGENTS.md with {{agents_content}}",
        )
        .unwrap();
    }

    #[test]
    fn test_extract_auto_managed_section() {
        let content = r#"# AGENTS.md

Some content.

<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
This is the auto-managed content.
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
"#;

        let section = extract_auto_managed_section(content);
        assert_eq!(section, Some("This is the auto-managed content."));
    }

    #[test]
    fn test_extract_auto_managed_section_empty() {
        let content = r#"# AGENTS.md

<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
"#;

        let section = extract_auto_managed_section(content);
        assert_eq!(section, Some(""));
    }

    #[test]
    fn test_extract_auto_managed_section_not_found() {
        let content = "# AGENTS.md\n\nNo markers here.";

        let section = extract_auto_managed_section(content);
        assert!(section.is_none());
    }

    #[test]
    fn test_patch_auto_managed_section() {
        let content = r#"# AGENTS.md

Before section.

<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
Old content.
<!-- END MICRORALPH AUTO-MANAGED SECTION -->

After section.
"#;

        let patched = patch_auto_managed_section(content, "New content here.").unwrap();

        assert!(patched.contains("Before section."));
        assert!(patched.contains("After section."));
        assert!(patched.contains("New content here."));
        assert!(!patched.contains("Old content."));
    }

    #[test]
    fn test_patch_auto_managed_section_empty() {
        let content = r#"<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
<!-- END MICRORALPH AUTO-MANAGED SECTION -->"#;

        let patched = patch_auto_managed_section(content, "New content.").unwrap();

        assert!(patched.contains("New content."));
    }

    #[test]
    fn test_parse_update_response_no_changes() {
        let output = "NO_CHANGES";
        assert!(parse_update_response(output).is_none());

        let output = "After review, NO_CHANGES are needed.";
        assert!(parse_update_response(output).is_none());
    }

    #[test]
    fn test_parse_update_response_code_block() {
        let output = r#"Here's the new content:

```markdown
## New Section

Some new content.
```
"#;

        let parsed = parse_update_response(output).unwrap();
        assert!(parsed.contains("## New Section"));
        assert!(parsed.contains("Some new content."));
    }

    #[test]
    fn test_parse_update_response_plain_text() {
        let output = "## New Section\n\nPlain text content.";

        let parsed = parse_update_response(output).unwrap();
        assert!(parsed.contains("## New Section"));
    }

    #[test]
    fn test_update_agents_md_no_file() {
        let temp = TempDir::new().unwrap();
        create_prompts_dir(temp.path());

        let runner = MockRunner::empty();
        let result = update_agents_md(temp.path(), &runner, &[]).unwrap();

        assert!(!result.modified);
    }

    #[test]
    fn test_update_agents_md_no_section() {
        let temp = TempDir::new().unwrap();
        create_prompts_dir(temp.path());
        create_agents_file(temp.path(), "# AGENTS.md\n\nNo markers.");

        let runner = MockRunner::empty();
        let result = update_agents_md(temp.path(), &runner, &[]).unwrap();

        assert!(!result.modified);
    }

    #[test]
    fn test_update_agents_md_no_changes() {
        let temp = TempDir::new().unwrap();
        create_prompts_dir(temp.path());

        let content = r#"# AGENTS.md

<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
Current content.
<!-- END MICRORALPH AUTO-MANAGED SECTION -->
"#;
        create_agents_file(temp.path(), content);

        let runner = MockRunner::new(vec![RunnerOutput::success("NO_CHANGES")]);
        let result = update_agents_md(temp.path(), &runner, &[]).unwrap();

        assert!(!result.modified);
    }

    #[test]
    fn test_update_agents_md_with_changes() {
        let temp = TempDir::new().unwrap();
        create_prompts_dir(temp.path());

        let content = r#"# AGENTS.md

Before.

<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
Old content.
<!-- END MICRORALPH AUTO-MANAGED SECTION -->

After.
"#;
        create_agents_file(temp.path(), content);

        let runner = MockRunner::new(vec![RunnerOutput::success("## Updated\n\nNew content.")]);

        let changes = vec![RecentChange {
            file: "src/lib.rs".to_string(),
            description: "Added new module".to_string(),
        }];

        let result = update_agents_md(temp.path(), &runner, &changes).unwrap();

        assert!(result.modified);
        assert!(result.new_content.unwrap().contains("Updated"));

        // Verify file was updated.
        let updated = std::fs::read_to_string(temp.path().join("AGENTS.md")).unwrap();
        assert!(updated.contains("Before."));
        assert!(updated.contains("After."));
        assert!(updated.contains("Updated"));
        assert!(!updated.contains("Old content."));
    }

    #[test]
    fn test_update_agents_md_runner_failure() {
        let temp = TempDir::new().unwrap();
        create_prompts_dir(temp.path());

        let content = r#"<!-- BEGIN MICRORALPH AUTO-MANAGED SECTION -->
<!-- END MICRORALPH AUTO-MANAGED SECTION -->"#;
        create_agents_file(temp.path(), content);

        let runner = MockRunner::new(vec![RunnerOutput::failure("Error!")]);
        let result = update_agents_md(temp.path(), &runner, &[]).unwrap();

        assert!(!result.modified);
    }

    #[test]
    fn test_recent_change_struct() {
        let change = RecentChange {
            file: "src/main.rs".to_string(),
            description: "Added CLI arguments".to_string(),
        };

        assert_eq!(change.file, "src/main.rs");
        assert_eq!(change.description, "Added CLI arguments");
    }

    #[test]
    fn test_build_update_agents_prompt() {
        let temp = TempDir::new().unwrap();
        create_prompts_dir(temp.path());

        let changes = vec![RecentChange {
            file: "src/lib.rs".to_string(),
            description: "Added feature".to_string(),
        }];

        let prompt = build_update_agents_prompt(temp.path(), "# AGENTS\nContent", &changes);

        // Should contain the agents content.
        assert!(prompt.contains("AGENTS") || prompt.contains("Content"));
    }
}
