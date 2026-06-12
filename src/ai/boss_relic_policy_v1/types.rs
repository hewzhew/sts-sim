use crate::ai::noncombat_decision_v1::{
    CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1, EvidenceItemV1,
    EvidenceKindV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
    PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
    NONCOMBAT_DECISION_RECORD_SCHEMA_NAME, NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
};
use crate::ai::noncombat_strategy_v1::{
    RunStrategySnapshotV2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::content::relics::RelicId;

#[derive(Clone, Debug, PartialEq)]
pub struct BossRelicDecisionContextV1 {
    pub strategy: RunStrategySnapshotV2,
    pub candidates: Vec<BossRelicCandidateEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossRelicCandidateEvidenceV1 {
    pub index: usize,
    pub relic: RelicId,
    pub class: BossRelicPolicyClassV1,
    pub support_gate: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BossRelicPolicyClassV1 {
    StarterRelicUpgrade,
    DeckCleanup,
    BroadSafeValue,
    RouteDependentValue,
    EnergyWithConstraint,
    TransformAgency,
    CurseDebt,
    StrategicPower,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossRelicPolicyConfigV1 {
    pub allow_starter_upgrade: bool,
    pub allow_empty_cage_when_cleanup_supported: bool,
    pub allow_tiny_house_as_safe_fallback: bool,
}

impl Default for BossRelicPolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_starter_upgrade: true,
            allow_empty_cage_when_cleanup_supported: true,
            allow_tiny_house_as_safe_fallback: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BossRelicDecisionV1 {
    pub action: BossRelicPolicyActionV1,
    pub label_role: &'static str,
    pub context: BossRelicDecisionContextV1,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BossRelicPolicyActionV1 {
    Pick {
        index: usize,
        relic: RelicId,
        confidence: f32,
        reason: String,
    },
    Stop {
        reason: String,
    },
}

impl BossRelicDecisionV1 {
    pub fn to_noncombat_decision_record_v1(&self) -> NonCombatDecisionRecordV1 {
        let selected_candidate_id = match self.action {
            BossRelicPolicyActionV1::Pick { index, relic, .. } => Some(candidate_id(index, relic)),
            BossRelicPolicyActionV1::Stop { .. } => None,
        };

        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::BossRelic,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(vec![
                InformationClassV1::PublicObservation,
                InformationClassV1::Belief,
            ]),
            provenance: PolicyProvenanceV1 {
                source_policy: "boss_relic_policy_v1".to_string(),
                source_schema_name: "BossRelicPolicyConfigV1".to_string(),
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
                    "boss relic automation is a conservative behavior policy, not an optimal-action label"
                        .to_string(),
                    "uncertain, deck-transforming, route-dependent, and high-variance boss relic choices remain human boundaries"
                        .to_string(),
                ],
                warnings: Vec::new(),
            },
            values: Vec::new(),
            selection: match &self.action {
                BossRelicPolicyActionV1::Pick {
                    confidence,
                    reason,
                    ..
                } => PolicySelectionV1 {
                    status: PolicySelectionStatusV1::Selected,
                    selected_candidate_id,
                    reason: reason.clone(),
                    confidence: *confidence,
                    selection_mode: "conservative_approval".to_string(),
                },
                BossRelicPolicyActionV1::Stop { reason } => PolicySelectionV1 {
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

fn candidate_descriptor(candidate: &BossRelicCandidateEvidenceV1) -> CandidateDescriptorV1 {
    CandidateDescriptorV1 {
        candidate_id: candidate_id(candidate.index, candidate.relic),
        site: DecisionSiteKindV1::BossRelic,
        label: format!("{:?}", candidate.relic),
        action_plan: PublicActionPlanV1 {
            summary: format!("choose boss relic {:?}", candidate.relic),
            command: Some(format!("relic {}", candidate.index)),
        },
        information_classes: vec![InformationClassV1::PublicObservation],
        uncertainty_notes: candidate.risks.clone(),
    }
}

fn evidence_items(context: &BossRelicDecisionContextV1) -> Vec<EvidenceItemV1> {
    let mut items = context
        .candidates
        .iter()
        .map(|candidate| EvidenceItemV1 {
            kind: EvidenceKindV1::CandidateFacts,
            candidate_id: Some(candidate_id(candidate.index, candidate.relic)),
            label: format!(
                "{:?}: {:?} gate={:?}",
                candidate.relic, candidate.class, candidate.support_gate
            ),
            information_class: InformationClassV1::PublicObservation,
            components: Vec::new(),
        })
        .collect::<Vec<_>>();

    for id in [
        StrategyPackageIdV2::ShopRemoveWindow,
        StrategyPackageIdV2::CorePlanProtection,
        StrategyPackageIdV2::HpSafety,
        StrategyPackageIdV2::PotionCapacity,
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

fn candidate_id(index: usize, relic: RelicId) -> String {
    format!("boss_relic:{index}:{relic:?}")
}
