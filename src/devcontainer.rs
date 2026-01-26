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

        // This test can't reliably test the /workspaces check
        // since we might actually be in a dev container during testing.
        // Just verify the function is callable.
        let _ = is_dev_container();
    }
}
