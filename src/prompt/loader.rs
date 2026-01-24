//! Prompt file loading.

use std::path::Path;

#[cfg(test)]
use anyhow::{Context, Result, bail};

use super::types::PromptKind;
use crate::init;

/// A loader for prompt files from the `.mr/prompts/` directory.
#[derive(Debug)]
pub struct PromptLoader {
    /// The root directory of the repository.
    prompts_dir: std::path::PathBuf,
}

impl PromptLoader {
    /// Creates a new prompt loader for the given repository root.
    ///
    /// # Arguments
    ///
    /// * `root` - The root directory of the repository
    ///
    /// # Returns
    ///
    /// A `PromptLoader` configured to load prompts from `.mr/prompts/`.
    pub fn new(root: impl AsRef<Path>) -> Self {
        let prompts_dir = root.as_ref().join(".mr").join("prompts");

        Self { prompts_dir }
    }

    /// Returns the path to the prompts directory.
    #[cfg(test)]
    pub fn prompts_dir(&self) -> &Path {
        &self.prompts_dir
    }

    /// Loads a prompt file by kind.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of prompt to load
    ///
    /// # Returns
    ///
    /// The content of the prompt file.
    #[cfg(test)]
    pub fn load(&self, kind: PromptKind) -> Result<String> {
        let path = self.prompts_dir.join(kind.filename());

        std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to load prompt file: {}", path.display()))
    }

    /// Loads a prompt file by kind, falling back to the embedded default if not found.
    ///
    /// This is useful for backwards compatibility when prompt files may not exist.
    ///
    /// # Arguments
    ///
    /// * `kind` - The kind of prompt to load
    ///
    /// # Returns
    ///
    /// The content of the prompt file, either from disk or the embedded default.
    pub fn load_with_fallback(&self, kind: PromptKind) -> String {
        let path = self.prompts_dir.join(kind.filename());

        std::fs::read_to_string(&path).unwrap_or_else(|_| get_default_prompt(kind).to_string())
    }

    /// Checks if a prompt file exists.
    #[cfg(test)]
    pub fn exists(&self, kind: PromptKind) -> bool {
        self.prompts_dir.join(kind.filename()).exists()
    }

    /// Checks if all prompt files exist.
    #[cfg(test)]
    pub fn all_exist(&self) -> bool {
        PromptKind::all().iter().all(|kind| self.exists(*kind))
    }

    /// Returns a list of missing prompt files.
    #[cfg(test)]
    pub fn missing_prompts(&self) -> Vec<PromptKind> {
        PromptKind::all()
            .iter()
            .filter(|kind| !self.exists(**kind))
            .copied()
            .collect()
    }
}

/// Returns the embedded default content for a prompt kind.
///
/// These are the same prompts that are written during `mr init`.
fn get_default_prompt(kind: PromptKind) -> &'static str {
    match kind {
        PromptKind::Init => init::PROMPT_INIT,
        PromptKind::BootstrapPlan => init::PROMPT_BOOTSTRAP_PLAN,
        PromptKind::BootstrapGeneratePrds => init::PROMPT_BOOTSTRAP_GENERATE_PRDS,
        PromptKind::PrdNewRound1Questions => init::PROMPT_PRD_NEW_ROUND1,
        PromptKind::PrdNewRoundNQuestions => init::PROMPT_PRD_NEW_ROUNDN,
        PromptKind::PrdNewSynthesizePrd => init::PROMPT_PRD_NEW_SYNTHESIZE,
        PromptKind::RunTask => init::PROMPT_RUN_TASK,
        PromptKind::RunTaskFinalize => init::PROMPT_RUN_TASK_FINALIZE,
        PromptKind::UpdateAgents => init::PROMPT_UPDATE_AGENTS,
        PromptKind::PrdEdit => init::PROMPT_PRD_EDIT,
        PromptKind::AdaptLanguage => init::PROMPT_ADAPT_LANGUAGE,
        PromptKind::Reindex => init::PROMPT_REINDEX,
        PromptKind::PickPrd => init::PROMPT_PICK_PRD,
    }
}

/// Loads a prompt from the given repository root.
///
/// This is a convenience function that creates a `PromptLoader` and loads
/// the specified prompt.
///
/// # Arguments
///
/// * `root` - The root directory of the repository
/// * `kind` - The kind of prompt to load
///
/// # Returns
///
/// The content of the prompt file.
#[cfg(test)]
pub fn load_prompt(root: impl AsRef<Path>, kind: PromptKind) -> Result<String> {
    let prompts_dir = root.as_ref().join(".mr").join("prompts");

    if !prompts_dir.exists() {
        bail!(
            "Prompts directory not found: {}. Run `mr init` first.",
            prompts_dir.display()
        );
    }

    let path = prompts_dir.join(kind.filename());

    std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to load prompt file: {}", path.display()))
}

/// Loads a prompt with fallback to embedded defaults.
///
/// This is a convenience function that creates a `PromptLoader` and loads
/// the specified prompt, falling back to embedded defaults if the file
/// doesn't exist.
///
/// # Arguments
///
/// * `root` - The root directory of the repository
/// * `kind` - The kind of prompt to load
///
/// # Returns
///
/// The content of the prompt file.
pub fn load_prompt_with_fallback(root: impl AsRef<Path>, kind: PromptKind) -> String {
    PromptLoader::new(root).load_with_fallback(kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_prompts_dir(temp: &TempDir) -> std::path::PathBuf {
        let prompts_dir = temp.path().join(".mr").join("prompts");
        std::fs::create_dir_all(&prompts_dir).unwrap();
        prompts_dir
    }

    #[test]
    fn test_prompt_loader_new() {
        let temp = TempDir::new().unwrap();
        let loader = PromptLoader::new(temp.path());
        assert!(loader.prompts_dir().ends_with(".mr/prompts"));
    }

    #[test]
    fn test_prompt_loader_load() {
        let temp = TempDir::new().unwrap();
        let prompts_dir = setup_prompts_dir(&temp);

        let content = "# Test Prompt\n\nThis is a test.";
        std::fs::write(prompts_dir.join("init.md"), content).unwrap();

        let loader = PromptLoader::new(temp.path());
        let loaded = loader.load(PromptKind::Init).unwrap();

        assert_eq!(loaded, content);
    }

    #[test]
    fn test_prompt_loader_load_missing() {
        let temp = TempDir::new().unwrap();
        let loader = PromptLoader::new(temp.path());

        let result = loader.load(PromptKind::Init);
        assert!(result.is_err());
    }

    #[test]
    fn test_prompt_loader_load_with_fallback() {
        let temp = TempDir::new().unwrap();
        let loader = PromptLoader::new(temp.path());

        // Should fall back to embedded default.
        let content = loader.load_with_fallback(PromptKind::Init);
        assert!(content.contains("microralph"));
        assert!(content.contains("Init"));
    }

    #[test]
    fn test_prompt_loader_load_with_fallback_prefers_file() {
        let temp = TempDir::new().unwrap();
        let prompts_dir = setup_prompts_dir(&temp);

        let custom_content = "# Custom Init Prompt\n\nCustom content here.";
        std::fs::write(prompts_dir.join("init.md"), custom_content).unwrap();

        let loader = PromptLoader::new(temp.path());
        let content = loader.load_with_fallback(PromptKind::Init);

        assert_eq!(content, custom_content);
    }

    #[test]
    fn test_prompt_loader_exists() {
        let temp = TempDir::new().unwrap();
        let prompts_dir = setup_prompts_dir(&temp);

        let loader = PromptLoader::new(temp.path());
        assert!(!loader.exists(PromptKind::Init));

        std::fs::write(prompts_dir.join("init.md"), "test").unwrap();
        assert!(loader.exists(PromptKind::Init));
    }

    #[test]
    fn test_prompt_loader_all_exist() {
        let temp = TempDir::new().unwrap();
        let prompts_dir = setup_prompts_dir(&temp);

        let loader = PromptLoader::new(temp.path());
        assert!(!loader.all_exist());

        // Create all prompt files.
        for kind in PromptKind::all() {
            std::fs::write(prompts_dir.join(kind.filename()), "test").unwrap();
        }

        assert!(loader.all_exist());
    }

    #[test]
    fn test_prompt_loader_missing_prompts() {
        let temp = TempDir::new().unwrap();
        let prompts_dir = setup_prompts_dir(&temp);

        let loader = PromptLoader::new(temp.path());
        let missing = loader.missing_prompts();

        // All should be missing initially.
        assert_eq!(missing.len(), 13);

        // Create one prompt file.
        std::fs::write(prompts_dir.join("init.md"), "test").unwrap();

        let missing = loader.missing_prompts();
        assert_eq!(missing.len(), 12);
        assert!(!missing.contains(&PromptKind::Init));
    }

    #[test]
    fn test_load_prompt_convenience() {
        let temp = TempDir::new().unwrap();
        let prompts_dir = setup_prompts_dir(&temp);

        let content = "# Test Prompt";
        std::fs::write(prompts_dir.join("init.md"), content).unwrap();

        let loaded = load_prompt(temp.path(), PromptKind::Init).unwrap();
        assert_eq!(loaded, content);
    }

    #[test]
    fn test_load_prompt_no_init() {
        let temp = TempDir::new().unwrap();

        let result = load_prompt(temp.path(), PromptKind::Init);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("mr init"));
    }

    #[test]
    fn test_load_prompt_with_fallback_convenience() {
        let temp = TempDir::new().unwrap();

        // Should work even without prompts directory.
        let content = load_prompt_with_fallback(temp.path(), PromptKind::RunTask);
        assert!(content.contains("Run Task"));
    }

    #[test]
    fn test_get_default_prompt_all_kinds() {
        // Ensure all prompt kinds have defaults.
        for kind in PromptKind::all() {
            let content = get_default_prompt(*kind);
            assert!(
                !content.is_empty(),
                "Default prompt for {:?} is empty",
                kind
            );
            assert!(
                content.contains("microralph"),
                "Default prompt for {:?} missing header",
                kind
            );
        }
    }
}
