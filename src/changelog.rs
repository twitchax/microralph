//! Changelog management for Keep a Changelog format.
//!
//! Creates and updates `CHANGELOG.md` at the project root following the
//! [Keep a Changelog](https://keepachangelog.com/en/1.0.0/) format.

use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

/// The default Keep a Changelog template for a new CHANGELOG.md.
const CHANGELOG_TEMPLATE: &str = r"# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
";

/// Result of ensuring the changelog exists.
#[derive(Debug)]
pub struct EnsureChangelogResult {
    /// Path to the CHANGELOG.md file.
    pub path: std::path::PathBuf,

    /// Whether the file was newly created.
    pub created: bool,
}

/// Ensures a CHANGELOG.md file exists at the project root.
///
/// If the file already exists, this is a no-op. If it doesn't exist,
/// creates a new file with the Keep a Changelog template.
///
/// # Arguments
///
/// * `root` - The project root directory.
///
/// # Returns
///
/// An `EnsureChangelogResult` indicating the path and whether the file was created.
pub fn ensure_changelog_exists(root: impl AsRef<Path>) -> Result<EnsureChangelogResult> {
    let root = root.as_ref();
    let changelog_path = root.join("CHANGELOG.md");

    if changelog_path.exists() {
        tracing::debug!(path = %changelog_path.display(), "CHANGELOG.md already exists");

        return Ok(EnsureChangelogResult {
            path: changelog_path,
            created: false,
        });
    }

    tracing::info!(path = %changelog_path.display(), "Creating CHANGELOG.md");

    fs::write(&changelog_path, CHANGELOG_TEMPLATE).with_context(|| {
        format!(
            "Failed to create CHANGELOG.md at {}",
            changelog_path.display()
        )
    })?;

    Ok(EnsureChangelogResult {
        path: changelog_path,
        created: true,
    })
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_changelog_exists_creates_new() {
        let temp = TempDir::new().unwrap();
        let result = ensure_changelog_exists(temp.path()).unwrap();

        assert!(result.created);
        assert!(result.path.exists());

        let content = fs::read_to_string(&result.path).unwrap();
        assert!(content.contains("# Changelog"));
        assert!(content.contains("[Keep a Changelog]"));
        assert!(content.contains("[Unreleased]"));
    }

    #[test]
    fn test_ensure_changelog_exists_preserves_existing() {
        let temp = TempDir::new().unwrap();
        let changelog_path = temp.path().join("CHANGELOG.md");

        let existing_content = "# My Custom Changelog\n\nExisting content.\n";
        fs::write(&changelog_path, existing_content).unwrap();

        let result = ensure_changelog_exists(temp.path()).unwrap();

        assert!(!result.created);
        assert_eq!(result.path, changelog_path);

        // Content should be unchanged.
        let content = fs::read_to_string(&changelog_path).unwrap();
        assert_eq!(content, existing_content);
    }

    #[test]
    fn test_changelog_template_format() {
        // Verify the template follows Keep a Changelog format.
        assert!(CHANGELOG_TEMPLATE.starts_with("# Changelog"));
        assert!(CHANGELOG_TEMPLATE.contains("All notable changes"));
        assert!(CHANGELOG_TEMPLATE.contains("keepachangelog.com"));
        assert!(CHANGELOG_TEMPLATE.contains("semver.org"));
        assert!(CHANGELOG_TEMPLATE.contains("## [Unreleased]"));
    }
}
