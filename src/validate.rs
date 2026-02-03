//! YAML frontmatter validation for PRDs and Constitution.
//!
//! This module provides validation for YAML frontmatter in PRD files
//! and Constitution files after agent edits. Validation emits warnings
//! rather than blocking execution, ensuring agents can proceed even if
//! files are malformed.

use std::path::Path;

/// Validates the YAML frontmatter of a PRD file.
///
/// Attempts to parse the PRD file and emits warnings if the frontmatter
/// is malformed. Returns `true` if valid, `false` if invalid.
///
/// # Arguments
///
/// * `path` - Path to the PRD file
///
/// # Behavior
///
/// - If parsing succeeds, returns `true`
/// - If parsing fails, emits a warning and returns `false`
/// - Does NOT return an error (validation warnings should not block execution)
pub fn validate_prd_frontmatter(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();

    match parse_prd_file_for_validation(path) {
        Ok(()) => {
            tracing::debug!(path = %path.display(), "PRD frontmatter validation passed");
            true
        }
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "PRD frontmatter validation failed"
            );
            eprintln!(
                "⚠️  Warning: PRD file has malformed YAML frontmatter: {}\n   Error: {}",
                path.display(),
                e
            );
            false
        }
    }
}

/// Validates the YAML frontmatter of a Constitution file.
///
/// Attempts to parse the Constitution file's frontmatter and emits warnings
/// if malformed. Returns `true` if valid, `false` if invalid.
///
/// # Arguments
///
/// * `path` - Path to the Constitution file
///
/// # Behavior
///
/// - If parsing succeeds, returns `true`
/// - If parsing fails, emits a warning and returns `false`
/// - Does NOT return an error (validation warnings should not block execution)
pub fn validate_constitution_frontmatter(path: impl AsRef<Path>) -> bool {
    let path = path.as_ref();

    // Constitution files may or may not have YAML frontmatter.
    // For now, we just check if the file can be read.
    // In the future, if Constitution files gain structured frontmatter,
    // we can add proper parsing here.

    match std::fs::read_to_string(path) {
        Ok(content) => {
            // Check if content starts with YAML frontmatter delimiter
            if content.trim_start().starts_with("---") {
                // Has frontmatter - try to parse it
                match try_parse_generic_frontmatter(&content) {
                    Ok(()) => {
                        tracing::debug!(path = %path.display(), "Constitution frontmatter validation passed");
                        true
                    }
                    Err(e) => {
                        tracing::warn!(
                            path = %path.display(),
                            error = %e,
                            "Constitution frontmatter validation failed"
                        );
                        eprintln!(
                            "⚠️  Warning: Constitution file has malformed YAML frontmatter: {}\n   Error: {}",
                            path.display(),
                            e
                        );
                        false
                    }
                }
            } else {
                // No frontmatter - that's fine for Constitution
                tracing::debug!(path = %path.display(), "Constitution has no frontmatter (valid)");
                true
            }
        }
        Err(e) => {
            tracing::warn!(
                path = %path.display(),
                error = %e,
                "Failed to read Constitution file"
            );
            eprintln!(
                "⚠️  Warning: Failed to read Constitution file: {}\n   Error: {}",
                path.display(),
                e
            );
            false
        }
    }
}

/// Helper to parse a PRD file (wrapper around [`parse_prd_file`]).
fn parse_prd_file_for_validation(path: &Path) -> anyhow::Result<()> {
    crate::prd::parse_prd_file(path)?;
    Ok(())
}

/// Tries to parse generic YAML frontmatter from content.
///
/// This is used for Constitution files which may have frontmatter
/// but don't follow the PRD schema.
fn try_parse_generic_frontmatter(content: &str) -> anyhow::Result<()> {
    const FRONTMATTER_DELIMITER: &str = "---";

    let trimmed = content.trim_start();

    if !trimmed.starts_with(FRONTMATTER_DELIMITER) {
        anyhow::bail!("Content does not start with frontmatter delimiter");
    }

    let after_open = &trimmed[FRONTMATTER_DELIMITER.len()..];
    let after_open = after_open.strip_prefix('\n').unwrap_or(after_open);

    let close_pos = after_open
        .find(&format!("\n{FRONTMATTER_DELIMITER}"))
        .ok_or_else(|| anyhow::anyhow!("Missing closing frontmatter delimiter"))?;

    let frontmatter = &after_open[..close_pos];

    // Try to parse as generic YAML (serde_yaml::Value)
    serde_yaml::from_str::<serde_yaml::Value>(frontmatter)
        .map_err(|e| anyhow::anyhow!("Failed to parse YAML frontmatter: {e}"))?;

    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_validate_prd_frontmatter_valid() {
        let temp = TempDir::new().unwrap();
        let prd_path = temp.path().join("PRD-0001.md");

        let content = r#"---
id: PRD-0001
title: Test PRD
status: active
---

# Summary

Test content.
"#;

        std::fs::write(&prd_path, content).unwrap();

        assert!(validate_prd_frontmatter(&prd_path));
    }

    #[test]
    fn test_validate_prd_frontmatter_invalid_yaml() {
        let temp = TempDir::new().unwrap();
        let prd_path = temp.path().join("PRD-0002.md");

        let content = r#"---
id: PRD-0002
title: Test PRD
status: [invalid yaml structure
---

# Summary
"#;

        std::fs::write(&prd_path, content).unwrap();

        assert!(!validate_prd_frontmatter(&prd_path));
    }

    #[test]
    fn test_validate_prd_frontmatter_missing_delimiter() {
        let temp = TempDir::new().unwrap();
        let prd_path = temp.path().join("PRD-0003.md");

        let content = r#"---
id: PRD-0003
title: Test PRD
status: active

# No closing delimiter!
"#;

        std::fs::write(&prd_path, content).unwrap();

        assert!(!validate_prd_frontmatter(&prd_path));
    }

    #[test]
    fn test_validate_constitution_no_frontmatter() {
        let temp = TempDir::new().unwrap();
        let const_path = temp.path().join("constitution.md");

        let content = "# Constitution\n\nRules go here.";
        std::fs::write(&const_path, content).unwrap();

        assert!(validate_constitution_frontmatter(&const_path));
    }

    #[test]
    fn test_validate_constitution_with_valid_frontmatter() {
        let temp = TempDir::new().unwrap();
        let const_path = temp.path().join("constitution.md");

        let content = r#"---
version: 1
updated: 2026-01-26
---

# Constitution

Rules go here.
"#;

        std::fs::write(&const_path, content).unwrap();

        assert!(validate_constitution_frontmatter(&const_path));
    }

    #[test]
    fn test_validate_constitution_with_invalid_frontmatter() {
        let temp = TempDir::new().unwrap();
        let const_path = temp.path().join("constitution.md");

        let content = r#"---
version: [malformed
updated: 2026-01-26
---

# Constitution
"#;

        std::fs::write(&const_path, content).unwrap();

        assert!(!validate_constitution_frontmatter(&const_path));
    }
}
