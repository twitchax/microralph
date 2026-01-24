//! PRD file parser and serializer.
//!
//! Handles parsing PRD files with YAML frontmatter and Markdown body,
//! as well as serializing them back while preserving the body content.

use std::path::Path;

use anyhow::{Context, Result};

use super::types::{Prd, PrdFrontmatter};

/// The delimiter used to mark the start and end of YAML frontmatter.
const FRONTMATTER_DELIMITER: &str = "---";

/// Parses a PRD from a string containing YAML frontmatter and Markdown body.
///
/// The expected format is:
/// ```text
/// ---
/// id: PRD-0001
/// title: My PRD
/// status: active
/// ...
/// ---
///
/// # Summary
///
/// Markdown body content...
/// ```
///
/// # Errors
///
/// Returns an error if:
/// - The frontmatter delimiters are missing or malformed
/// - The YAML frontmatter cannot be parsed
pub fn parse_prd(content: &str) -> Result<Prd> {
    let (frontmatter_str, body) = split_frontmatter(content)?;

    let frontmatter: PrdFrontmatter = serde_yaml::from_str(&frontmatter_str)
        .context("Failed to parse PRD frontmatter as YAML")?;

    Ok(Prd::new(frontmatter, body))
}

/// Parses a PRD from a file path.
///
/// # Errors
///
/// Returns an error if:
/// - The file cannot be read
/// - The PRD content is malformed
pub fn parse_prd_file(path: impl AsRef<Path>) -> Result<Prd> {
    let content = std::fs::read_to_string(path.as_ref())
        .with_context(|| format!("Failed to read PRD file: {}", path.as_ref().display()))?;

    parse_prd(&content)
}

/// Serializes a PRD back to a string with YAML frontmatter and Markdown body.
///
/// The serialization preserves the original body content exactly as provided.
#[allow(dead_code)]
pub fn serialize_prd(prd: &Prd) -> Result<String> {
    let frontmatter_yaml = serde_yaml::to_string(&prd.frontmatter)
        .context("Failed to serialize PRD frontmatter to YAML")?;

    // Build the final document.
    let mut output = String::new();

    output.push_str(FRONTMATTER_DELIMITER);
    output.push('\n');
    output.push_str(&frontmatter_yaml);
    output.push_str(FRONTMATTER_DELIMITER);
    output.push('\n');
    output.push_str(&prd.body);

    Ok(output)
}

/// Splits content into frontmatter and body sections.
///
/// Returns a tuple of (frontmatter_yaml, body_markdown).
fn split_frontmatter(content: &str) -> Result<(String, String)> {
    let trimmed = content.trim_start();

    // Check for opening delimiter.
    if !trimmed.starts_with(FRONTMATTER_DELIMITER) {
        anyhow::bail!("PRD file must start with '---' frontmatter delimiter");
    }

    // Find the end of the first line (the opening delimiter).
    let after_open = &trimmed[FRONTMATTER_DELIMITER.len()..];
    let after_open = after_open.strip_prefix('\n').unwrap_or(after_open);

    // Find the closing delimiter.
    let close_pos = after_open
        .find(&format!("\n{FRONTMATTER_DELIMITER}"))
        .ok_or_else(|| {
            anyhow::anyhow!("PRD file is missing closing '---' frontmatter delimiter")
        })?;

    let frontmatter = after_open[..close_pos].to_string();
    let after_close = &after_open[close_pos + 1 + FRONTMATTER_DELIMITER.len()..];

    // The body starts after the closing delimiter.
    // Strip a leading newline if present.
    let body = after_close
        .strip_prefix('\n')
        .unwrap_or(after_close)
        .to_string();

    Ok((frontmatter, body))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prd::types::{PrdStatus, TaskStatus};

    const SAMPLE_PRD: &str = r#"---
id: PRD-0001
title: Test PRD
status: active
owner: Test User
created: "2026-01-23"
tasks:
  - id: T-001
    title: First task
    priority: 1
    status: done
  - id: T-002
    title: Second task
    priority: 2
    status: todo
    notes: Some notes here
---

# Summary

This is the body of the PRD.

## Problem

A description of the problem.

## Goals

1. Goal one
2. Goal two

# History

## 2026-01-23
- Initial creation
"#;

    #[test]
    fn test_parse_prd_basic() {
        let prd = parse_prd(SAMPLE_PRD).unwrap();

        assert_eq!(prd.id(), "PRD-0001");
        assert_eq!(prd.title(), "Test PRD");
        assert_eq!(prd.status(), PrdStatus::Active);
        assert_eq!(prd.frontmatter.owner.as_deref(), Some("Test User"));
    }

    #[test]
    fn test_parse_prd_tasks() {
        let prd = parse_prd(SAMPLE_PRD).unwrap();

        let tasks = prd.tasks().unwrap();

        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "T-001");
        assert_eq!(tasks[0].status, TaskStatus::Done);
        assert_eq!(tasks[1].id, "T-002");
        assert_eq!(tasks[1].status, TaskStatus::Todo);
        assert_eq!(tasks[1].notes.as_deref(), Some("Some notes here"));
    }

    #[test]
    fn test_parse_prd_body() {
        let prd = parse_prd(SAMPLE_PRD).unwrap();

        assert!(prd.body.contains("# Summary"));
        assert!(prd.body.contains("## Problem"));
        assert!(prd.body.contains("# History"));
        assert!(prd.body.contains("Initial creation"));
    }

    #[test]
    fn test_parse_prd_missing_frontmatter() {
        let content = "# Just markdown\n\nNo frontmatter here.";
        let result = parse_prd(content);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("frontmatter"));
    }

    #[test]
    fn test_parse_prd_unclosed_frontmatter() {
        let content = "---\nid: PRD-0001\ntitle: Test\n\n# Body without closing delimiter";
        let result = parse_prd(content);

        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("closing"));
    }

    #[test]
    fn test_serialize_prd() {
        let prd = parse_prd(SAMPLE_PRD).unwrap();
        let serialized = serialize_prd(&prd).unwrap();

        // Should start with frontmatter.
        assert!(serialized.starts_with("---\n"));

        // Should contain the id and title.
        assert!(serialized.contains("id: PRD-0001"));
        assert!(serialized.contains("title: Test PRD"));

        // Should contain the body.
        assert!(serialized.contains("# Summary"));
        assert!(serialized.contains("# History"));
    }

    #[test]
    fn test_roundtrip_preserves_body() {
        let prd = parse_prd(SAMPLE_PRD).unwrap();
        let serialized = serialize_prd(&prd).unwrap();
        let reparsed = parse_prd(&serialized).unwrap();

        // Body should be identical.
        assert_eq!(prd.body, reparsed.body);
    }

    #[test]
    fn test_roundtrip_preserves_frontmatter() {
        let prd = parse_prd(SAMPLE_PRD).unwrap();
        let serialized = serialize_prd(&prd).unwrap();
        let reparsed = parse_prd(&serialized).unwrap();

        // Frontmatter should match.
        assert_eq!(prd.id(), reparsed.id());
        assert_eq!(prd.title(), reparsed.title());
        assert_eq!(prd.status(), reparsed.status());
        assert_eq!(prd.frontmatter.owner, reparsed.frontmatter.owner);
        assert_eq!(
            prd.tasks().map(|t| t.len()),
            reparsed.tasks().map(|t| t.len())
        );
    }

    #[test]
    fn test_serialize_minimal_prd() {
        let frontmatter = PrdFrontmatter {
            id: "PRD-0002".to_string(),
            title: "Minimal PRD".to_string(),
            status: PrdStatus::Draft,
            ..Default::default()
        };

        let prd = Prd::new(frontmatter, "# Body\n\nContent here.\n".to_string());
        let serialized = serialize_prd(&prd).unwrap();

        assert!(serialized.starts_with("---\n"));
        assert!(serialized.contains("id: PRD-0002"));
        assert!(serialized.contains("# Body"));
    }

    #[test]
    fn test_parse_all_task_statuses() {
        let content = r#"---
id: PRD-0001
title: Status Test
status: active
tasks:
  - id: T-001
    title: Todo task
    priority: 1
    status: todo
  - id: T-002
    title: In progress task
    priority: 2
    status: in-progress
  - id: T-003
    title: Done task
    priority: 3
    status: done
  - id: T-004
    title: Blocked task
    priority: 4
    status: blocked
  - id: T-005
    title: Parked task
    priority: 5
    status: parked
---

# Body
"#;

        let prd = parse_prd(content).unwrap();
        let tasks = prd.tasks().unwrap();

        assert_eq!(tasks[0].status, TaskStatus::Todo);
        assert_eq!(tasks[1].status, TaskStatus::InProgress);
        assert_eq!(tasks[2].status, TaskStatus::Done);
        assert_eq!(tasks[3].status, TaskStatus::Blocked);
        assert_eq!(tasks[4].status, TaskStatus::Parked);
    }

    #[test]
    fn test_parse_all_prd_statuses() {
        for (status_str, expected) in [
            ("draft", PrdStatus::Draft),
            ("active", PrdStatus::Active),
            ("done", PrdStatus::Done),
            ("parked", PrdStatus::Parked),
        ] {
            let content = format!(
                r#"---
id: PRD-0001
title: Status Test
status: {status_str}
---

# Body
"#
            );

            let prd = parse_prd(&content).unwrap();

            assert_eq!(prd.status(), expected, "Failed for status: {status_str}");
        }
    }

    #[test]
    fn test_parse_complex_prd() {
        // Parse the actual PRD-0001 from the .mr directory.
        let prd_content = include_str!("../../.mr/prds/PRD-0001-build-micro-ralph-mvp.md");
        let prd = parse_prd(prd_content).unwrap();

        assert_eq!(prd.id(), "PRD-0001");
        assert_eq!(prd.title(), "Build microralph MVP");
        assert_eq!(prd.status(), PrdStatus::Active);

        // Check tasks exist.
        let tasks = prd.tasks().unwrap();

        assert!(!tasks.is_empty());

        // Check first task is done.
        let t001 = tasks.iter().find(|t| t.id == "T-001").unwrap();

        assert_eq!(t001.status, TaskStatus::Done);

        // Body should contain History section.
        assert!(prd.body.contains("# History"));
    }
}
