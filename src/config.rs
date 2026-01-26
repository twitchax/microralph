//! Configuration file support for microralph.
//!
//! This module provides types and loading logic for `.mr/config.toml`.
//! Configuration values can be overridden by CLI flags.

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// The config file name within the `.mr/` directory.
pub const CONFIG_FILE_NAME: &str = "config.toml";

/// The constitution file name within the `.mr/` directory.
pub const CONSTITUTION_FILE_NAME: &str = "constitution.md";

/// Default configuration content for new repos.
pub const DEFAULT_CONFIG: &str = r#"# microralph configuration
# See README.md for available options.

# Default runner to use for all commands.
# Options: "copilot", "mock"
# runner = "copilot"

# Default model to use with the runner.
# This is passed to the runner CLI (e.g., `copilot --model <model>`).
# model = "claude-sonnet-4.5"

# Permission mode for the runner.
# Options: "yolo" (--allow-all), "manual" (prompt for each)
# permission_mode = "yolo"

# Session timeout in minutes.
# timeout_minutes = 30

# Whether to instruct the agent NOT to commit changes.
# When true, prompts say "Do NOT commit" instead of commit instructions.
# no_commit = false
"#;

/// microralph configuration.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Default runner to use for all commands.
    #[serde(default)]
    pub runner: Option<String>,

    /// Default model to use with the runner.
    #[serde(default)]
    pub model: Option<String>,

    /// Permission mode for the runner.
    #[serde(default)]
    pub permission_mode: Option<String>,

    /// Session timeout in minutes.
    #[serde(default)]
    pub timeout_minutes: Option<u32>,

    /// Whether to instruct the agent NOT to commit changes.
    /// When true, prompts say "Do NOT commit" instead of commit instructions.
    #[serde(default)]
    pub no_commit: Option<bool>,
}

impl Config {
    /// Loads the configuration from `.mr/config.toml` in the given root directory.
    ///
    /// Returns `None` if the config file doesn't exist.
    /// Returns an error if the file exists but can't be parsed.
    pub fn load(root: &Path) -> Result<Option<Self>> {
        let config_path = root.join(".mr").join(CONFIG_FILE_NAME);

        if !config_path.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&config_path)?;
        let config: Config = toml::from_str(&content)?;

        Ok(Some(config))
    }

    /// Loads the configuration, returning defaults if the file doesn't exist.
    pub fn load_or_default(root: &Path) -> Result<Self> {
        Ok(Self::load(root)?.unwrap_or_default())
    }

    /// Returns the effective runner, with CLI flag taking precedence.
    #[cfg(test)]
    pub fn effective_runner(&self, cli_runner: Option<&str>) -> String {
        cli_runner
            .map(|s| s.to_string())
            .or_else(|| self.runner.clone())
            .unwrap_or_else(|| "copilot".to_string())
    }

    /// Returns the effective model, with CLI flag taking precedence.
    pub fn effective_model(&self, cli_model: Option<&str>) -> Option<String> {
        cli_model
            .map(|s| s.to_string())
            .or_else(|| self.model.clone())
    }

    /// Returns the effective no_commit setting, with CLI flag taking precedence.
    ///
    /// Logic:
    /// - If CLI flag is `Some(true)` (--no-commit passed), returns `true`.
    /// - If CLI flag is `Some(false)` (explicit negation, if supported), returns `false`.
    /// - If CLI flag is `None`, falls back to config value.
    /// - If both are `None`, defaults to `false` (commit by default).
    pub fn effective_no_commit(&self, cli_no_commit: Option<bool>) -> bool {
        cli_no_commit.or(self.no_commit).unwrap_or(false)
    }
}

/// Loads the constitution from `.mr/constitution.md`.
///
/// Returns `None` if the constitution file doesn't exist.
/// Returns an error if the file exists but can't be read.
pub fn load_constitution(root: &Path) -> Result<Option<String>> {
    let constitution_path = root.join(".mr").join(CONSTITUTION_FILE_NAME);

    if !constitution_path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(&constitution_path).with_context(|| {
        format!(
            "Failed to read constitution from {}",
            constitution_path.display()
        )
    })?;

    Ok(Some(content))
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_config_default() {
        let config = Config::default();

        assert!(config.runner.is_none());
        assert!(config.model.is_none());
        assert!(config.permission_mode.is_none());
        assert!(config.timeout_minutes.is_none());
        assert!(config.no_commit.is_none());
    }

    #[test]
    fn test_config_load_missing_file() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".mr")).unwrap();

        let config = Config::load(temp.path()).unwrap();

        assert!(config.is_none());
    }

    #[test]
    fn test_config_load_empty_file() {
        let temp = TempDir::new().unwrap();
        let mr_dir = temp.path().join(".mr");
        std::fs::create_dir_all(&mr_dir).unwrap();
        std::fs::write(mr_dir.join("config.toml"), "").unwrap();

        let config = Config::load(temp.path()).unwrap().unwrap();

        assert!(config.runner.is_none());
        assert!(config.model.is_none());
    }

    #[test]
    fn test_config_load_full() {
        let temp = TempDir::new().unwrap();
        let mr_dir = temp.path().join(".mr");
        std::fs::create_dir_all(&mr_dir).unwrap();
        std::fs::write(
            mr_dir.join("config.toml"),
            r#"
runner = "mock"
model = "gpt-4o"
permission_mode = "manual"
timeout_minutes = 60
"#,
        )
        .unwrap();

        let config = Config::load(temp.path()).unwrap().unwrap();

        assert_eq!(config.runner, Some("mock".to_string()));
        assert_eq!(config.model, Some("gpt-4o".to_string()));
        assert_eq!(config.permission_mode, Some("manual".to_string()));
        assert_eq!(config.timeout_minutes, Some(60));
    }

    #[test]
    fn test_config_load_partial() {
        let temp = TempDir::new().unwrap();
        let mr_dir = temp.path().join(".mr");
        std::fs::create_dir_all(&mr_dir).unwrap();
        std::fs::write(mr_dir.join("config.toml"), r#"model = "claude-sonnet-4""#).unwrap();

        let config = Config::load(temp.path()).unwrap().unwrap();

        assert!(config.runner.is_none());
        assert_eq!(config.model, Some("claude-sonnet-4".to_string()));
    }

    #[test]
    fn test_config_load_or_default_missing() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".mr")).unwrap();

        let config = Config::load_or_default(temp.path()).unwrap();

        assert!(config.runner.is_none());
        assert!(config.model.is_none());
    }

    #[test]
    fn test_effective_runner_cli_override() {
        let config = Config {
            runner: Some("copilot".to_string()),
            ..Default::default()
        };

        assert_eq!(config.effective_runner(Some("mock")), "mock");
    }

    #[test]
    fn test_effective_runner_config_value() {
        let config = Config {
            runner: Some("mock".to_string()),
            ..Default::default()
        };

        assert_eq!(config.effective_runner(None), "mock");
    }

    #[test]
    fn test_effective_runner_default() {
        let config = Config::default();

        assert_eq!(config.effective_runner(None), "copilot");
    }

    #[test]
    fn test_effective_model_cli_override() {
        let config = Config {
            model: Some("gpt-4".to_string()),
            ..Default::default()
        };

        assert_eq!(
            config.effective_model(Some("claude-sonnet-4")),
            Some("claude-sonnet-4".to_string())
        );
    }

    #[test]
    fn test_effective_model_config_value() {
        let config = Config {
            model: Some("gpt-4".to_string()),
            ..Default::default()
        };

        assert_eq!(config.effective_model(None), Some("gpt-4".to_string()));
    }

    #[test]
    fn test_effective_model_none() {
        let config = Config::default();

        assert!(config.effective_model(None).is_none());
    }

    #[test]
    fn test_default_config_parses() {
        let _config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
    }

    #[test]
    fn test_effective_no_commit_cli_override() {
        let config = Config {
            no_commit: Some(false),
            ..Default::default()
        };

        // CLI flag (true) supersedes config (false).
        assert!(config.effective_no_commit(Some(true)));
    }

    #[test]
    fn test_effective_no_commit_config_value() {
        let config = Config {
            no_commit: Some(true),
            ..Default::default()
        };

        // No CLI flag, uses config value.
        assert!(config.effective_no_commit(None));
    }

    #[test]
    fn test_effective_no_commit_default_false() {
        let config = Config::default();

        // No CLI flag, no config value, defaults to false (commit by default).
        assert!(!config.effective_no_commit(None));
    }

    #[test]
    fn test_config_load_with_no_commit() {
        let temp = TempDir::new().unwrap();
        let mr_dir = temp.path().join(".mr");
        std::fs::create_dir_all(&mr_dir).unwrap();
        std::fs::write(mr_dir.join("config.toml"), r#"no_commit = true"#).unwrap();

        let config = Config::load(temp.path()).unwrap().unwrap();

        assert_eq!(config.no_commit, Some(true));
    }

    #[test]
    fn test_load_constitution_missing() {
        let temp = TempDir::new().unwrap();
        std::fs::create_dir_all(temp.path().join(".mr")).unwrap();

        let constitution = super::load_constitution(temp.path()).unwrap();

        assert!(constitution.is_none());
    }

    #[test]
    fn test_load_constitution_exists() {
        let temp = TempDir::new().unwrap();
        let mr_dir = temp.path().join(".mr");
        std::fs::create_dir_all(&mr_dir).unwrap();
        let test_content = "# Constitution\n\n## Rules\n\n1. Test rule";
        std::fs::write(mr_dir.join("constitution.md"), test_content).unwrap();

        let constitution = super::load_constitution(temp.path()).unwrap().unwrap();

        assert_eq!(constitution, test_content);
    }
}
