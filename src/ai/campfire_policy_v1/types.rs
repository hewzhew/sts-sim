use crate::ai::noncombat_decision_v1::{
    CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1, EvidenceItemV1,
    EvidenceKindV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
    PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
    NONCOMBAT_DECISION_RECORD_SCHEMA_NAME, NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
};
use crate::ai::noncombat_strategy_v1::{
    RunStrategySnapshotV2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::state::core::CampfireChoice;

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireDecisionContextV1 {
    pub strategy: RunStrategySnapshotV2,
    pub current_hp: i32,
    pub max_hp: i32,
    pub candidates: Vec<CampfireCandidateEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireCandidateEvidenceV1 {
    pub candidate_id: String,
    pub label: String,
    pub choice: CampfireChoice,
    pub class: CampfirePolicyClassV1,
    pub upgrade_priority: Option<i32>,
    pub support_gate: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfirePolicyClassV1 {
    RestRecovery,
    UpgradeAgency,
    RelicAction,
    KeyRecall,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfirePolicyConfigV1 {
    pub allow_rest_under_recovery_pressure: bool,
    pub allow_clear_core_smith_when_healthy: bool,
    pub allow_combat_patch_smith_when_safe: bool,
    pub clear_core_smith_priority_threshold: i32,
    pub combat_patch_smith_priority_threshold: i32,
    pub combat_patch_smith_min_hp_percent: i32,
}

impl Default for CampfirePolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_rest_under_recovery_pressure: true,
            allow_clear_core_smith_when_healthy: true,
            allow_combat_patch_smith_when_safe: true,
            clear_core_smith_priority_threshold: 180,
            combat_patch_smith_priority_threshold: 180,
            combat_patch_smith_min_hp_percent: 70,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireDecisionV1 {
    pub action: CampfirePolicyActionV1,
    pub selected_plan: CampfirePlanCandidateV1,
    pub candidate_plans: Vec<CampfirePlanCandidateV1>,
    pub label_role: &'static str,
    pub context: CampfireDecisionContextV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfirePlanCandidateV1 {
    pub plan_id: String,
    pub choice: Option<CampfireChoice>,
    pub action: CampfirePolicyActionV1,
    pub role: CampfirePlanRoleV1,
    pub score_hint: i32,
    pub confidence: f32,
    pub reasons: Vec<String>,
    pub execute_autopilot: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfirePlanRoleV1 {
    PolicyPreferred,
    InspectOnly,
    StopFallback,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CampfirePolicyActionV1 {
    Rest {
        confidence: f32,
        reason: String,
    },
    Smith {
        deck_index: usize,
        confidence: f32,
        reason: String,
    },
    Stop {
        reason: String,
    },
}

impl CampfireDecisionV1 {
    pub fn to_noncombat_decision_record_v1(&self) -> NonCombatDecisionRecordV1 {
        let selected_candidate_id = match self.action {
            CampfirePolicyActionV1::Rest { .. } => Some(candidate_id(CampfireChoice::Rest)),
            CampfirePolicyActionV1::Smith { deck_index, .. } => {
                Some(candidate_id(CampfireChoice::Smith(deck_index)))
            }
            CampfirePolicyActionV1::Stop { .. } => None,
        };

        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::Campfire,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(vec![
                InformationClassV1::PublicObservation,
                InformationClassV1::Belief,
            ]),
            provenance: PolicyProvenanceV1 {
                source_policy: "campfire_policy_v1".to_string(),
                source_schema_name: "CampfirePolicyConfigV1".to_string(),
                source_schema_version: 1,
            },
            candidates: self
                .context
                .candidates
                .iter()
                .map(candidate_descriptor)
                .collect(),
            evidence: EvidenceBundleV1 {
                items: evidence_items(&self.context),
                assumptions: vec![
                    "campfire automation handles conservative rest and clear-priority smith approvals"
                        .to_string(),
                    "healthy smith automation requires a clear core upgrade priority threshold"
                        .to_string(),
                    "campfire automation is a behavior policy, not a teacher label".to_string(),
                ],
                warnings: Vec::new(),
            },
            values: Vec::new(),
            selection: match &self.action {
                CampfirePolicyActionV1::Rest { confidence, reason } => PolicySelectionV1 {
                    status: PolicySelectionStatusV1::Selected,
                    selected_candidate_id,
                    reason: reason.clone(),
                    confidence: *confidence,
                    selection_mode: "conservative_campfire_approval".to_string(),
                },
                CampfirePolicyActionV1::Smith {
                    confidence, reason, ..
                } => PolicySelectionV1 {
                    status: PolicySelectionStatusV1::Selected,
                    selected_candidate_id,
                    reason: reason.clone(),
                    confidence: *confidence,
                    selection_mode: "conservative_campfire_approval".to_string(),
                },
                CampfirePolicyActionV1::Stop { reason } => PolicySelectionV1 {
                    status: PolicySelectionStatusV1::Stopped,
                    selected_candidate_id,
                    reason: reason.clone(),
                    confidence: 0.0,
                    selection_mode: "human_required".to_string(),
                },
            },
        }
    }
}

fn candidate_descriptor(candidate: &CampfireCandidateEvidenceV1) -> CandidateDescriptorV1 {
    CandidateDescriptorV1 {
        candidate_id: candidate.candidate_id.clone(),
        site: DecisionSiteKindV1::Campfire,
        label: candidate.label.clone(),
        action_plan: PublicActionPlanV1 {
            summary: candidate.label.clone(),
            command: Some(command_for_choice(candidate.choice)),
        },
        information_classes: vec![InformationClassV1::PublicObservation],
        uncertainty_notes: candidate.risks.clone(),
    }
}

fn evidence_items(context: &CampfireDecisionContextV1) -> Vec<EvidenceItemV1> {
    let mut items = context
        .candidates
        .iter()
        .map(|candidate| EvidenceItemV1 {
            kind: EvidenceKindV1::CandidateFacts,
            candidate_id: Some(candidate.candidate_id.clone()),
            label: format!(
                "{}: {:?} gate={:?}",
                candidate.label, candidate.class, candidate.support_gate
            ),
            information_class: InformationClassV1::PublicObservation,
            components: Vec::new(),
        })
        .collect::<Vec<_>>();

    for id in [
        StrategyPackageIdV2::RecoveryPressure,
        StrategyPackageIdV2::HpSafety,
        StrategyPackageIdV2::UpgradeCommitment,
        StrategyPackageIdV2::RelicConstraints,
    ] {
        if let Some(package) = context.strategy.package(id) {
            items.push(EvidenceItemV1 {
                kind: EvidenceKindV1::PolicyGate,
                candidate_id: None,
                label: format!("strategy package: {:?}/{:?}", package.domain, package.id),
                information_class: InformationClassV1::Belief,
                components: Vec::new(),
            });
        }
    }

    items
}

pub(crate) fn candidate_id(choice: CampfireChoice) -> String {
    match choice {
        CampfireChoice::Rest => "campfire:rest".to_string(),
        CampfireChoice::Smith(idx) => format!("campfire:smith:{idx}"),
        CampfireChoice::Dig => "campfire:dig".to_string(),
        CampfireChoice::Lift => "campfire:lift".to_string(),
        CampfireChoice::Toke(idx) => format!("campfire:toke:{idx}"),
        CampfireChoice::Recall => "campfire:recall".to_string(),
    }
}

fn command_for_choice(choice: CampfireChoice) -> String {
    match choice {
        CampfireChoice::Rest => "rest".to_string(),
        CampfireChoice::Smith(idx) => format!("smith {idx}"),
        CampfireChoice::Dig => "dig".to_string(),
        CampfireChoice::Lift => "lift".to_string(),
        CampfireChoice::Toke(idx) => format!("toke {idx}"),
        CampfireChoice::Recall => "recall".to_string(),
    }
}
