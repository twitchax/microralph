//! Dev container detection utilities.
//!
//! This module provides utilities to detect whether the current environment
//! is running inside a dev container, checking common indicators like
//! environment variables and filesystem paths.

use std::env;
use std::path::Path;

/// Checks if the current environment is inside a dev container.
///
/// Detection logic checks for multiple indicators:
/// - `REMOTE_CONTAINERS` environment variable (set by VS Code Dev Containers)
/// - `CODESPACES` environment variable (set by GitHub Codespaces)
/// - `/workspaces` directory (common dev container workspace root)
///
/// Returns `true` if any indicator is present, `false` otherwise.
pub fn is_dev_container() -> bool {
    // Check for VS Code Remote Containers environment variable
    if env::var("REMOTE_CONTAINERS").is_ok() {
        return true;
    }

    // Check for GitHub Codespaces environment variable
    if env::var("CODESPACES").is_ok() {
        return true;
    }

    // Check for /workspaces path (common in dev containers)
    if Path::new("/workspaces").exists() {
        return true;
    }

    false
}

/// Shows a brief warning if not running inside a dev container.
///
/// This function checks if the current environment is a dev container and,
/// if not, prints a non-blocking informational message suggesting the use
/// of dev containers for safety and isolation.
pub fn show_dev_container_warning() {
    if !is_dev_container() {
        eprintln!(
            "\n⚠️  Not running in a dev container. For safety, consider using a dev container."
        );
        eprintln!(
            "   Run `mr devcontainer generate` to create a config, or continue at your own risk.\n"
        );
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_is_dev_container_with_remote_containers_var() {
        unsafe {
            env::set_var("REMOTE_CONTAINERS", "true");
        }
        assert!(is_dev_container());
        unsafe {
            env::remove_var("REMOTE_CONTAINERS");
        }
    }

    #[test]
    fn test_is_dev_container_with_codespaces_var() {
        unsafe {
            env::set_var("CODESPACES", "true");
        }
        assert!(is_dev_container());
        unsafe {
            env::remove_var("CODESPACES");
        }
    }

    #[test]
    fn test_is_dev_container_without_indicators() {
        // Remove any dev container indicators that might be set
        unsafe {
            env::remove_var("REMOTE_CONTAINERS");
            env::remove_var("CODESPACES");
        }

        // The function checks env vars first, then /workspaces directory.
        // After clearing env vars, the result depends on /workspaces existence.
        let workspaces_exists = Path::new("/workspaces").exists();
        let result = is_dev_container();

        // With env vars removed, result should match /workspaces existence check.
        assert_eq!(
            result, workspaces_exists,
            "Without env vars, is_dev_container should return {workspaces_exists} (workspaces_exists={workspaces_exists})"
        );
    }

    #[test]
    fn test_is_dev_container_prioritizes_env_vars_over_filesystem() {
        // Set REMOTE_CONTAINERS, then check that it returns true
        // regardless of /workspaces existence (env check comes first).
        unsafe {
            env::set_var("REMOTE_CONTAINERS", "1");
        }
        assert!(
            is_dev_container(),
            "Should return true when REMOTE_CONTAINERS is set"
        );
        unsafe {
            env::remove_var("REMOTE_CONTAINERS");
        }

        // Same for CODESPACES.
        unsafe {
            env::set_var("CODESPACES", "1");
        }
        assert!(
            is_dev_container(),
            "Should return true when CODESPACES is set"
        );
        unsafe {
            env::remove_var("CODESPACES");
        }
    }
}
