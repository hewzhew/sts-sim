use sts_simulator::ai::strategy::boss_relic_admission::BossRelicAdmission;
use sts_simulator::ai::strategy::decision_pipeline::{CandidateEvaluation, CleanupTarget};
use sts_simulator::ai::strategy::reward_admission::RewardAdmission;
use sts_simulator::eval::run_control::{
    DecisionCandidateKey, RunDecisionAction, RunForcedTransitionKindV1,
};

pub(super) type DecisionKey = DecisionCandidateKey;

pub(super) enum OwnerDecision {
    Candidates(Vec<OwnerChoice>),
    Routine(OwnerRoutine),
    Gap(String),
}

pub(super) enum OwnerRoutine {
    Candidate {
        candidate_id: String,
        action: RunDecisionAction,
    },
    RewardPolicyStep,
    ForcedTransition(RunForcedTransitionKindV1),
}

#[derive(Clone)]
pub(super) struct OwnerChoice {
    pub(super) candidate_id: String,
    pub(super) key: Option<DecisionKey>,
    pub(super) action: RunDecisionAction,
    pub(super) label: String,
    pub(super) annotation: ChoiceAnnotation,
    pub(super) expansion: OwnerChoiceExpansion,
}

#[derive(Clone)]
pub(super) enum ChoiceAnnotation {
    None,
    Candidate(OwnerCandidateDecision),
    BossRelic(BossRelicAdmission),
}

#[derive(Clone)]
pub(super) struct OwnerCandidateDecision {
    pub(super) evaluation: CandidateEvaluation,
    pub(super) admission: Option<RewardAdmission>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnerChoiceExpansion {
    AutoAllowed,
    InspectOnly(&'static str),
}

impl OwnerChoice {
    pub(super) fn auto_expand_allowed(&self) -> bool {
        matches!(self.expansion, OwnerChoiceExpansion::AutoAllowed)
    }

    pub(super) fn inspect_only_reason(&self) -> Option<&'static str> {
        match self.expansion {
            OwnerChoiceExpansion::InspectOnly(reason) => Some(reason),
            OwnerChoiceExpansion::AutoAllowed => None,
        }
    }
}

impl ChoiceAnnotation {
    pub(super) fn admission(&self) -> Option<&RewardAdmission> {
        match self {
            ChoiceAnnotation::Candidate(decision) => decision.admission.as_ref(),
            _ => None,
        }
    }

    pub(super) fn evaluation(&self) -> Option<&CandidateEvaluation> {
        match self {
            ChoiceAnnotation::Candidate(decision) => Some(&decision.evaluation),
            _ => None,
        }
    }

    pub(super) fn candidate(&self) -> Option<&OwnerCandidateDecision> {
        match self {
            ChoiceAnnotation::Candidate(decision) => Some(decision),
            _ => None,
        }
    }

    pub(super) fn boss_relic(&self) -> Option<&BossRelicAdmission> {
        match self {
            ChoiceAnnotation::BossRelic(admission) => Some(admission),
            _ => None,
        }
    }
}

pub(super) fn cleanup_target_label(target: CleanupTarget) -> &'static str {
    match target {
        CleanupTarget::Curse => "curse",
        CleanupTarget::Status => "status",
        CleanupTarget::StarterStrike => "starter-attack",
        CleanupTarget::StarterDefend => "starter-skill",
        CleanupTarget::OtherStarter => "starter",
        CleanupTarget::Other => "other",
    }
}
