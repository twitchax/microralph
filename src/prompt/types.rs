//! Prompt type definitions.

use std::fmt;

/// The different kinds of prompts used by Micro Ralph.
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

    /// First round of questions for `mr prd new`.
    PrdNewRound1Questions,

    /// Subsequent rounds of questions for `mr prd new`.
    PrdNewRoundNQuestions,

    /// Final PRD synthesis prompt for `mr prd new`.
    PrdNewSynthesizePrd,

    /// Task execution prompt for `mr run`.
    RunTask,

    /// Final wrap-up task prompt for `mr run`.
    RunTaskFinalize,

    /// AGENTS.md update prompt.
    UpdateAgents,
}

impl PromptKind {
    /// Returns the filename for this prompt kind.
    ///
    /// This is the name of the file in `.mr/prompts/`.
    pub fn filename(&self) -> &'static str {
        match self {
            Self::Init => "init.md",
            Self::BootstrapPlan => "bootstrap_plan.md",
            Self::BootstrapGeneratePrds => "bootstrap_generate_prds.md",
            Self::PrdNewRound1Questions => "prd_new_round1_questions.md",
            Self::PrdNewRoundNQuestions => "prd_new_roundN_questions.md",
            Self::PrdNewSynthesizePrd => "prd_new_synthesize_prd.md",
            Self::RunTask => "run_task.md",
            Self::RunTaskFinalize => "run_task_finalize.md",
            Self::UpdateAgents => "update_agents.md",
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
            Self::RunTask,
            Self::RunTaskFinalize,
            Self::UpdateAgents,
        ]
    }
}

impl fmt::Display for PromptKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.filename())
    }
}

#[cfg(test)]
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
        assert_eq!(all.len(), 9);
        assert!(all.contains(&PromptKind::Init));
        assert!(all.contains(&PromptKind::UpdateAgents));
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
