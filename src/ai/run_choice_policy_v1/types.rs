use crate::content::cards::CardId;
use crate::state::core::RunPendingChoiceReason;

#[derive(Clone, Debug, PartialEq)]
pub struct RunChoiceDecisionContextV1 {
    pub reason: RunPendingChoiceReason,
    pub min_choices: usize,
    pub max_choices: usize,
    pub candidates: Vec<RunChoiceCandidateEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunChoiceCandidateEvidenceV1 {
    pub candidate_id: String,
    pub label: String,
    pub deck_index: usize,
    pub card: CardId,
    pub class: RunChoicePolicyClassV1,
    pub selectable: bool,
    pub upgrade_priority: Option<i32>,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunChoicePolicyClassV1 {
    CursePurge,
    StarterStrikeMutation,
    StarterDefendMutation,
    BasicCardMutation,
    OtherDeckMutation,
    UpgradeTarget,
    UnsupportedChoice,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunChoicePolicyConfigV1 {
    pub allow_curse_purge: bool,
    pub allow_low_value_purge: bool,
    pub allow_low_value_transform: bool,
    pub allow_clear_upgrade: bool,
    pub clear_upgrade_priority_threshold: i32,
}

impl Default for RunChoicePolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_curse_purge: true,
            allow_low_value_purge: true,
            allow_low_value_transform: true,
            allow_clear_upgrade: true,
            clear_upgrade_priority_threshold:
                crate::ai::campfire_policy_v1::CampfirePolicyConfigV1::default()
                    .clear_core_smith_priority_threshold,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RunChoiceDecisionV1 {
    pub action: RunChoicePolicyActionV1,
    pub label_role: &'static str,
    pub context: RunChoiceDecisionContextV1,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunChoicePolicyActionV1 {
    SelectDeckIndices {
        indices: Vec<usize>,
        labels: Vec<String>,
        confidence: f32,
        reason: String,
    },
    Stop {
        reason: String,
    },
}

pub(crate) fn candidate_id(deck_index: usize) -> String {
    format!("run_choice:deck:{deck_index}")
}
