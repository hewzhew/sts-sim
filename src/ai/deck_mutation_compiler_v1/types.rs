use crate::content::cards::CardId;
use crate::state::core::RunPendingChoiceReason;

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledDeckMutationDecisionV1 {
    pub reason: RunPendingChoiceReason,
    pub min_choices: usize,
    pub max_choices: usize,
    pub selected_plan: Option<DeckMutationPlanCandidateV1>,
    pub branch_active_plans: Vec<DeckMutationPlanCandidateV1>,
    pub inspect_only_plans: Vec<DeckMutationPlanCandidateV1>,
    pub blocked_plans: Vec<DeckMutationPlanCandidateV1>,
    pub candidate_plans: Vec<DeckMutationPlanCandidateV1>,
    pub label_role: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeckMutationCompilerModeV1 {
    ExecuteOne,
    BranchTopK { max_active: usize },
    Inspect,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeckMutationKindV1 {
    Remove,
    Upgrade,
    Transform,
    Duplicate,
    Bottle,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum DeckMutationPlanRoleV1 {
    PolicyPreferred,
    SafeAlternative,
    RiskyExploration,
    InspectOnly,
    Blocked,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeckMutationTargetClassV1 {
    Curse,
    StarterStrike,
    StarterDefend,
    Basic,
    Functional,
    UpgradeTarget,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Ord, PartialOrd)]
pub enum DeckMutationTargetLossTierV1 {
    #[default]
    LowValue,
    RedundantFunctional,
    Functional,
    CoreFunctional,
    Unsupported,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DeckMutationTargetLossV1 {
    pub tier: DeckMutationTargetLossTierV1,
    pub same_card_count: usize,
    pub signals: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AllowedDeckMutationConsumersV1 {
    pub execute_autopilot: bool,
    pub branch_active: bool,
    pub branch_frozen: bool,
    pub inspect: bool,
    pub replay: bool,
    pub human_prompt: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckMutationCardSnapshotV1 {
    pub deck_index: usize,
    pub card: CardId,
    pub upgrades: u8,
    pub label: String,
    pub target_class: DeckMutationTargetClassV1,
    pub target_loss: DeckMutationTargetLossV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckMutationPlanStepV1 {
    pub kind: DeckMutationKindV1,
    pub deck_indices: Vec<usize>,
    pub cards: Vec<DeckMutationCardSnapshotV1>,
    pub command: String,
    pub effect_kind: String,
    pub effect_key: String,
    pub effect_label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckMutationPlanCandidateV1 {
    pub plan_id: String,
    pub step: DeckMutationPlanStepV1,
    pub role: DeckMutationPlanRoleV1,
    pub allowed_consumers: AllowedDeckMutationConsumersV1,
    pub representative_count: usize,
    pub suppressed_count: usize,
    pub run_choice_policy_selected: bool,
    pub score_hint: i32,
    pub confidence: f32,
    pub reasons: Vec<String>,
    pub risks: Vec<String>,
}

impl DeckMutationPlanCandidateV1 {
    pub fn deck_indices(&self) -> &[usize] {
        &self.step.deck_indices
    }
}
