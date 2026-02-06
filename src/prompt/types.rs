//! Prompt type definitions.

use std::fmt;

/// The different kinds of prompts used by microralph.
///
/// Each variant corresponds to a static prompt file in `.mr/prompts/`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PromptKind {
    /// Initialization prompt for `mr init`.
    Init,

    /// Bootstrap planning prompt for analyzing existing repos.
    BootstrapPlan,

    /// Bootstrap PRD generation prompt.
    BootstrapGeneratePrds,

    /// First round of questions for `mr new`.
    PrdNewRound1Questions,

    /// Subsequent rounds of questions for `mr new`.
    PrdNewRoundNQuestions,

    /// Final PRD synthesis prompt for `mr new`.
    PrdNewSynthesizePrd,

    /// Interactive discovery prompt for `mr new`.
    PrdNewDiscovery,

    /// Task execution prompt for `mr run`.
    RunTask,

    /// Final wrap-up task prompt for `mr run`.
    RunTaskFinalize,

    /// UAT verification prompt for verifying a single acceptance test.
    RunUatVerify,

    /// PRD edit prompt for quick modifications.
    PrdEdit,

    /// Constitution edit prompt for updating governance rules.
    ConstitutionEdit,

    /// Language adaptation prompt for rewriting prompts/templates.
    AdaptLanguage,

    /// Reindex prompt for regenerating index and fixing links.
    Reindex,

    /// Pick PRD prompt for determining which PRD to work on next.
    PickPrd,

    /// Devcontainer generation prompt for creating dev container configs.
    DevcontainerGenerate,

    /// Suggest generation prompt for analyzing codebase and proposing PRDs.
    SuggestGenerate,

    /// Refactor prompt for AI-driven code improvements.
    Refactor,

    /// Bootstrap reconstruct prompt for analyzing git history to create PRDs.
    BootstrapReconstruct,

    /// Reindex `depends_on` prompt for inferring and fixing dependency relationships.
    ReindexDependsOn,
}

impl PromptKind {
    /// Returns the filename for this prompt kind.
    ///
    /// This is the name of the file in `.mr/prompts/`.
    pub fn filename(self) -> &'static str {
        match self {
            Self::Init => "init.md",
            Self::BootstrapPlan => "bootstrap_plan.md",
            Self::BootstrapGeneratePrds => "bootstrap_generate_prds.md",
            Self::PrdNewRound1Questions => "prd_new_round1_questions.md",
            Self::PrdNewRoundNQuestions => "prd_new_roundN_questions.md",
            Self::PrdNewSynthesizePrd => "prd_new_synthesize_prd.md",
            Self::PrdNewDiscovery => "prd_new_discovery.md",
            Self::RunTask => "run_task.md",
            Self::RunTaskFinalize => "run_task_finalize.md",
            Self::RunUatVerify => "run_uat_verify.md",
            Self::PrdEdit => "prd_edit.md",
            Self::ConstitutionEdit => "constitution_edit.md",
            Self::AdaptLanguage => "adapt_language.md",
            Self::Reindex => "reindex.md",
            Self::PickPrd => "pick_prd.md",
            Self::DevcontainerGenerate => "devcontainer_generate.md",
            Self::SuggestGenerate => "suggest_generate.md",
            Self::Refactor => "refactor.md",
            Self::BootstrapReconstruct => "bootstrap_reconstruct.md",
            Self::ReindexDependsOn => "reindex_depends_on.md",
        }
    }

    /// Returns all prompt kinds.
    pub fn all() -> &'static [PromptKind] {
        &[
            Self::Init,
            Self::BootstrapPlan,
            Self::BootstrapGeneratePrds,
            Self::PrdNewRound1Questions,
            Self::PrdNewRoundNQuestions,
            Self::PrdNewSynthesizePrd,
            Self::PrdNewDiscovery,
            Self::RunTask,
            Self::RunTaskFinalize,
            Self::RunUatVerify,
            Self::PrdEdit,
            Self::ConstitutionEdit,
            Self::AdaptLanguage,
            Self::Reindex,
            Self::PickPrd,
            Self::DevcontainerGenerate,
            Self::SuggestGenerate,
            Self::Refactor,
            Self::BootstrapReconstruct,
            Self::ReindexDependsOn,
        ]
    }
}

impl fmt::Display for PromptKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.filename())
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_kind_filename() {
        assert_eq!(PromptKind::Init.filename(), "init.md");
        assert_eq!(PromptKind::RunTask.filename(), "run_task.md");
        assert_eq!(
            PromptKind::PrdNewRound1Questions.filename(),
            "prd_new_round1_questions.md"
        );
    }

    #[test]
    fn test_prompt_kind_all() {
        let all = PromptKind::all();
        assert_eq!(all.len(), 20);
        assert!(all.contains(&PromptKind::Init));
        assert!(all.contains(&PromptKind::PrdEdit));
        assert!(all.contains(&PromptKind::AdaptLanguage));
        assert!(all.contains(&PromptKind::Reindex));
        assert!(all.contains(&PromptKind::PickPrd));
        assert!(all.contains(&PromptKind::RunUatVerify));
        assert!(all.contains(&PromptKind::SuggestGenerate));
    }

    #[test]
    fn test_prompt_kind_display() {
        assert_eq!(format!("{}", PromptKind::Init), "init.md");
        assert_eq!(
            format!("{}", PromptKind::RunTaskFinalize),
            "run_task_finalize.md"
        );
    }
}
