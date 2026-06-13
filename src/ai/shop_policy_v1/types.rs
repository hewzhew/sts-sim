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
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Debug, PartialEq)]
pub struct ShopDecisionContextV1 {
    pub strategy: RunStrategySnapshotV2,
    pub need: ShopNeedProfileV1,
    pub candidates: Vec<ShopCandidateEvidenceV1>,
    pub affordable_purchase_exists: bool,
    pub conversion_pressure: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopNeedProfileV1 {
    pub act: u8,
    pub floor: i32,
    pub gold: i32,
    pub floors_to_boss: i32,
    pub near_boss: bool,
    pub has_curse: bool,
    pub starter_count: usize,
    pub strike_count: usize,
    pub defend_count: usize,
    pub empty_potion_slots: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopCandidateEvidenceV1 {
    pub candidate_id: String,
    pub label: String,
    pub class: ShopPolicyClassV1,
    pub deck_index: Option<usize>,
    pub card: Option<CardId>,
    pub purchase_target: Option<ShopPurchaseTargetV1>,
    pub purchase_priority: Option<i32>,
    pub gold_cost: Option<i32>,
    pub support_gate: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPurchaseTargetV1 {
    Card { index: usize, card: CardId },
    Relic { index: usize, relic: RelicId },
    Potion { index: usize, potion: PotionId },
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
    pub allow_high_impact_purchase: bool,
    pub high_impact_card_purchase_priority_threshold: i32,
    pub high_impact_relic_purchase_priority_threshold: i32,
    pub high_impact_potion_purchase_priority_threshold: i32,
}

impl Default for ShopPolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_curse_purge: true,
            allow_starter_strike_purge_when_core_plan_protected: true,
            allow_high_impact_purchase: true,
            high_impact_card_purchase_priority_threshold: 650,
            high_impact_relic_purchase_priority_threshold: 900,
            high_impact_potion_purchase_priority_threshold: 780,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopDecisionV1 {
    pub action: ShopPolicyActionV1,
    pub label_role: &'static str,
    pub context: ShopDecisionContextV1,
    pub strategic_trace: crate::ai::strategic::StrategicDecisionTrace,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopDecisionSourceV1 {
    LegacyWrapped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanSourceV1 {
    LegacyWrapped,
    LegacyShopPortfolioSource,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopCompileModeV1 {
    ExecuteOne,
    BranchTopK { max_plans: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanKindV1 {
    Execute,
    Stop,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledShopDecisionV1 {
    pub selected_plan: ShopPlanV1,
    pub alternatives: Vec<ShopPlanV1>,
    pub candidate_plans: Vec<ShopPlanCandidateV1>,
    pub strategic_trace: crate::ai::strategic::StrategicDecisionTrace,
    pub source: ShopDecisionSourceV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanCandidateV1 {
    pub plan: ShopPlanV1,
    pub role: ShopPlanCandidateRoleV1,
    pub evaluation: ShopPlanEvaluationV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanCandidateRoleV1 {
    SingleAction,
    PortfolioAlternative,
    StopFallback,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanEvaluationV1 {
    pub verdict: ShopPlanVerdictV1,
    pub tier: i32,
    pub score: i32,
    pub confidence: f32,
    pub reasons: Vec<String>,
    pub legacy_priority: Option<i32>,
    pub components: Vec<ShopPlanComponentV1>,
    pub component_score: ShopPlanComponentScoreV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanVerdictV1 {
    Allow,
    Stop,
    Block,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanComponentV1 {
    pub kind: ShopPlanComponentKindV1,
    pub amount: f32,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanComponentScoreV1 {
    pub positive: f32,
    pub negative: f32,
    pub net: f32,
    pub confidence: f32,
    pub explanation: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanComponentKindV1 {
    DeckCleanup,
    RelicValue,
    PotionFill,
    DeckBloatCost,
    GoldSpend,
    LegacyEstimate,
    BranchExploration,
    BossAnswer,
    StopReason,
}

impl ShopPlanEvaluationV1 {
    pub(crate) fn pending() -> Self {
        Self {
            verdict: ShopPlanVerdictV1::Block,
            tier: 0,
            score: 0,
            confidence: 0.0,
            reasons: vec!["pending shop plan evaluation".to_string()],
            legacy_priority: None,
            components: Vec::new(),
            component_score: ShopPlanComponentScoreV1::neutral(
                "component score pending shop plan evaluation",
            ),
        }
    }

    pub(crate) fn allow(
        tier: i32,
        score: i32,
        confidence: f32,
        legacy_priority: Option<i32>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            verdict: ShopPlanVerdictV1::Allow,
            tier,
            score,
            confidence,
            reasons: vec![reason.into()],
            legacy_priority,
            components: Vec::new(),
            component_score: ShopPlanComponentScoreV1::neutral("component score not attached yet"),
        }
    }

    pub(crate) fn stop(reason: impl Into<String>) -> Self {
        Self {
            verdict: ShopPlanVerdictV1::Stop,
            tier: 0,
            score: 0,
            confidence: 0.0,
            reasons: vec![reason.into()],
            legacy_priority: None,
            components: Vec::new(),
            component_score: ShopPlanComponentScoreV1::neutral("component score not attached yet"),
        }
    }

    pub(crate) fn block(legacy_priority: Option<i32>, reason: impl Into<String>) -> Self {
        Self {
            verdict: ShopPlanVerdictV1::Block,
            tier: 0,
            score: legacy_priority.unwrap_or_default(),
            confidence: 0.0,
            reasons: vec![reason.into()],
            legacy_priority,
            components: Vec::new(),
            component_score: ShopPlanComponentScoreV1::neutral("component score not attached yet"),
        }
    }
}

impl ShopPlanComponentScoreV1 {
    pub(crate) fn neutral(explanation: impl Into<String>) -> Self {
        Self {
            positive: 0.0,
            negative: 0.0,
            net: 0.0,
            confidence: 0.0,
            explanation: explanation.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanV1 {
    pub plan_id: String,
    pub label: String,
    pub kind: ShopPlanKindV1,
    pub steps: Vec<ShopPlanStepV1>,
    pub total_gold_spent: i32,
    pub candidate_ids: Vec<String>,
    pub source: ShopPlanSourceV1,
    pub legacy_priority: Option<i32>,
    pub legacy_confidence: Option<f32>,
    pub suppressed_count: usize,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShopPlanStepV1 {
    BuyCard {
        index: usize,
        card: CardId,
        cost: i32,
    },
    BuyRelic {
        index: usize,
        relic: RelicId,
        cost: i32,
    },
    BuyPotion {
        index: usize,
        potion: PotionId,
        cost: i32,
    },
    RemoveCard {
        deck_index: usize,
        card: CardId,
        cost: i32,
    },
    LeaveShop,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ShopPolicyActionV1 {
    Purge {
        deck_index: usize,
        card: CardId,
        confidence: f32,
        reason: String,
    },
    Purchase {
        target: ShopPurchaseTargetV1,
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
            ShopPolicyActionV1::Purchase { target, .. } => Some(purchase_candidate_id(target)),
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
                    "shop automation handles conservative purge and high-impact purchase approvals"
                        .to_string(),
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
                    selection_mode: "conservative_shop_approval".to_string(),
                },
                ShopPolicyActionV1::Purchase {
                    confidence, reason, ..
                } => PolicySelectionV1 {
                    status: PolicySelectionStatusV1::Selected,
                    selected_candidate_id,
                    reason: reason.clone(),
                    confidence: *confidence,
                    selection_mode: "conservative_shop_approval".to_string(),
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
                    candidate.purchase_target.map(purchase_command).or_else(|| {
                        Some("leave".to_string())
                            .filter(|_| candidate.class == ShopPolicyClassV1::Leave)
                    })
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

pub(crate) fn purchase_candidate_id(target: ShopPurchaseTargetV1) -> String {
    match target {
        ShopPurchaseTargetV1::Card { index, .. } => format!("shop:card-{index}"),
        ShopPurchaseTargetV1::Relic { index, .. } => format!("shop:relic-{index}"),
        ShopPurchaseTargetV1::Potion { index, .. } => format!("shop:potion-{index}"),
    }
}

fn purchase_command(target: ShopPurchaseTargetV1) -> String {
    match target {
        ShopPurchaseTargetV1::Card { index, .. } => format!("card {index}"),
        ShopPurchaseTargetV1::Relic { index, .. } => format!("relic {index}"),
        ShopPurchaseTargetV1::Potion { index, .. } => format!("potion {index}"),
    }
}
