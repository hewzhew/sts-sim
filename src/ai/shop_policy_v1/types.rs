use crate::ai::noncombat_decision_v1::{
    CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1, EvidenceItemV1,
    EvidenceKindV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
    PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
    NONCOMBAT_DECISION_RECORD_SCHEMA_NAME, NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
};
use crate::ai::noncombat_strategy_v1::{
    RunStrategySnapshotV2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::content::cards::CardId;

#[derive(Clone, Debug, PartialEq)]
pub struct ShopDecisionContextV1 {
    pub strategy: RunStrategySnapshotV2,
    pub candidates: Vec<ShopCandidateEvidenceV1>,
    pub affordable_purchase_exists: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopCandidateEvidenceV1 {
    pub candidate_id: String,
    pub label: String,
    pub class: ShopPolicyClassV1,
    pub deck_index: Option<usize>,
    pub card: Option<CardId>,
    pub support_gate: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPolicyClassV1 {
    CursePurge,
    StarterStrikePurge,
    PurchaseOpportunity,
    Leave,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPolicyConfigV1 {
    pub allow_curse_purge: bool,
    pub allow_starter_strike_purge_when_core_plan_protected: bool,
}

impl Default for ShopPolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_curse_purge: true,
            allow_starter_strike_purge_when_core_plan_protected: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopDecisionV1 {
    pub action: ShopPolicyActionV1,
    pub label_role: &'static str,
    pub context: ShopDecisionContextV1,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShopPolicyActionV1 {
    Purge {
        deck_index: usize,
        card: CardId,
        confidence: f32,
        reason: String,
    },
    Stop {
        reason: String,
    },
}

impl ShopDecisionV1 {
    pub fn to_noncombat_decision_record_v1(&self) -> NonCombatDecisionRecordV1 {
        let selected_candidate_id = match self.action {
            ShopPolicyActionV1::Purge { deck_index, .. } => Some(purge_candidate_id(deck_index)),
            ShopPolicyActionV1::Stop { .. } => None,
        };

        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::Shop,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(vec![
                InformationClassV1::PublicObservation,
                InformationClassV1::Belief,
            ]),
            provenance: PolicyProvenanceV1 {
                source_policy: "shop_policy_v1".to_string(),
                source_schema_name: "ShopPolicyConfigV1".to_string(),
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
                    "shop automation only handles conservative purge certificates".to_string(),
                    "shop automation is a behavior policy, not a teacher label".to_string(),
                ],
                warnings: Vec::new(),
            },
            values: Vec::new(),
            selection: match &self.action {
                ShopPolicyActionV1::Purge {
                    confidence, reason, ..
                } => PolicySelectionV1 {
                    status: PolicySelectionStatusV1::Selected,
                    selected_candidate_id,
                    reason: reason.clone(),
                    confidence: *confidence,
                    selection_mode: "conservative_shop_certificate".to_string(),
                },
                ShopPolicyActionV1::Stop { reason } => PolicySelectionV1 {
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

fn candidate_descriptor(candidate: &ShopCandidateEvidenceV1) -> CandidateDescriptorV1 {
    CandidateDescriptorV1 {
        candidate_id: candidate.candidate_id.clone(),
        site: DecisionSiteKindV1::Shop,
        label: candidate.label.clone(),
        action_plan: PublicActionPlanV1 {
            summary: candidate.label.clone(),
            command: candidate
                .deck_index
                .map(|idx| format!("purge {idx}"))
                .or_else(|| {
                    Some("leave".to_string())
                        .filter(|_| candidate.class == ShopPolicyClassV1::Leave)
                }),
        },
        information_classes: vec![InformationClassV1::PublicObservation],
        uncertainty_notes: candidate.risks.clone(),
    }
}

fn evidence_items(context: &ShopDecisionContextV1) -> Vec<EvidenceItemV1> {
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
        StrategyPackageIdV2::ShopRemoveWindow,
        StrategyPackageIdV2::CorePlanProtection,
        StrategyPackageIdV2::CombatPatchWindow,
        StrategyPackageIdV2::GoldPlan,
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

pub(crate) fn purge_candidate_id(deck_index: usize) -> String {
    format!("shop:purge:{deck_index}")
}
