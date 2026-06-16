use crate::ai::noncombat_decision_v1::{
    CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1, EvidenceItemV1,
    EvidenceKindV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
    PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
    ValueComponentV1, ValueEstimateV1, NONCOMBAT_DECISION_RECORD_SCHEMA_NAME,
    NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
};
use crate::ai::noncombat_strategy_v1::{RunStrategySnapshotV2, StrategyPlanSupportV1};
use crate::ai::strength_profile_v1::StrengthProfileV1;
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Debug, PartialEq)]
pub struct ShopDecisionContextV1 {
    pub strategy: RunStrategySnapshotV2,
    pub strength: StrengthProfileV1,
    pub need: ShopNeedProfileV1,
    pub candidates: Vec<ShopCandidateEvidenceV1>,
    pub affordable_purchase_exists: bool,
    pub conversion_pressure: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopNeedProfileV1 {
    pub act: u8,
    pub floor: i32,
    pub boss: Option<EncounterId>,
    pub hp: i32,
    pub max_hp: i32,
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
    StarterDefendPurge,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopDecisionSourceV1 {
    PlanEvaluationCompiler,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanSourceV1 {
    CandidateEvidence,
    PortfolioCandidate,
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

impl CompiledShopDecisionV1 {
    pub fn to_noncombat_decision_record_v1(&self) -> NonCombatDecisionRecordV1 {
        let selected_candidate_id = (self.selected_plan.kind == ShopPlanKindV1::Execute
            && !self.selected_plan.steps.is_empty())
        .then(|| self.selected_plan.plan_id.clone());
        let selected_evaluation = self
            .candidate_plans
            .iter()
            .find(|candidate| candidate.plan.plan_id == self.selected_plan.plan_id)
            .map(|candidate| &candidate.evaluation);

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
                source_policy: "shop_compiler_v1".to_string(),
                source_schema_name: "CompiledShopDecisionV1".to_string(),
                source_schema_version: 1,
            },
            candidates: self
                .candidate_plans
                .iter()
                .map(compiled_shop_candidate_descriptor)
                .collect(),
            evidence: EvidenceBundleV1 {
                items: self
                    .candidate_plans
                    .iter()
                    .map(compiled_shop_evidence_item)
                    .collect(),
                assumptions: vec![
                    "shop compiler evaluates plan candidates before selecting an action"
                        .to_string(),
                    "legacy priority is retained as estimate evidence, not as an execution entry point"
                        .to_string(),
                    "shop automation is a behavior policy, not a teacher label".to_string(),
                ],
                warnings: Vec::new(),
            },
            values: self
                .candidate_plans
                .iter()
                .map(compiled_shop_value_estimate)
                .collect(),
            selection: PolicySelectionV1 {
                status: if selected_candidate_id.is_some() {
                    PolicySelectionStatusV1::Selected
                } else {
                    PolicySelectionStatusV1::Stopped
                },
                selected_candidate_id,
                reason: self.selected_plan.reason.clone(),
                confidence: selected_evaluation
                    .map(|evaluation| evaluation.confidence)
                    .or(self.selected_plan.legacy_confidence)
                    .unwrap_or(0.0),
                selection_mode: "compiled_shop_decision_v1".to_string(),
            },
        }
    }
}

fn compiled_shop_candidate_descriptor(candidate: &ShopPlanCandidateV1) -> CandidateDescriptorV1 {
    CandidateDescriptorV1 {
        candidate_id: candidate.plan.plan_id.clone(),
        site: DecisionSiteKindV1::Shop,
        label: candidate.plan.label.clone(),
        action_plan: PublicActionPlanV1 {
            summary: candidate.plan.label.clone(),
            command: candidate.plan.steps.first().map(compiled_shop_step_command),
        },
        information_classes: vec![
            InformationClassV1::PublicObservation,
            InformationClassV1::Belief,
        ],
        uncertainty_notes: candidate.evaluation.reasons.clone(),
    }
}

fn compiled_shop_evidence_item(candidate: &ShopPlanCandidateV1) -> EvidenceItemV1 {
    let evaluation = &candidate.evaluation;
    let mut components = vec![
        ValueComponentV1::new(format!("role_{:?}", candidate.role), 1.0),
        ValueComponentV1::new(format!("verdict_{:?}", evaluation.verdict), 1.0),
        ValueComponentV1::new("tier", evaluation.tier as f32),
        ValueComponentV1::new("score", evaluation.score as f32),
        ValueComponentV1::new("confidence", evaluation.confidence),
        ValueComponentV1::new("component_net", evaluation.component_score.net),
    ];
    if let Some(priority) = evaluation.legacy_priority {
        components.push(ValueComponentV1::new("legacy_priority", priority as f32));
    }

    EvidenceItemV1 {
        kind: EvidenceKindV1::PolicyGate,
        candidate_id: Some(candidate.plan.plan_id.clone()),
        label: format!(
            "shop plan role={:?} verdict={:?} tier={} score={} source={:?}",
            candidate.role,
            evaluation.verdict,
            evaluation.tier,
            evaluation.score,
            candidate.plan.source
        ),
        information_class: InformationClassV1::Belief,
        components,
    }
}

fn compiled_shop_value_estimate(candidate: &ShopPlanCandidateV1) -> ValueEstimateV1 {
    let evaluation = &candidate.evaluation;
    ValueEstimateV1 {
        candidate_id: candidate.plan.plan_id.clone(),
        mean_utility: evaluation.score as f32,
        risk_adjusted_utility: evaluation.score as f32 + evaluation.component_score.net,
        confidence: evaluation.confidence,
        components: vec![
            ValueComponentV1::new("tier", evaluation.tier as f32),
            ValueComponentV1::new("score", evaluation.score as f32),
            ValueComponentV1::new("component_positive", evaluation.component_score.positive),
            ValueComponentV1::new("component_negative", evaluation.component_score.negative),
            ValueComponentV1::new("component_net", evaluation.component_score.net),
        ],
        evidence_refs: Vec::new(),
    }
}

fn compiled_shop_step_command(step: &ShopPlanStepV1) -> String {
    match *step {
        ShopPlanStepV1::BuyCard { index, .. } => format!("buy-card {index}"),
        ShopPlanStepV1::BuyRelic { index, .. } => format!("buy-relic {index}"),
        ShopPlanStepV1::BuyPotion { index, .. } => format!("buy-potion {index}"),
        ShopPlanStepV1::RemoveCard { deck_index, .. } => format!("purge {deck_index}"),
        ShopPlanStepV1::LeaveShop => "leave".to_string(),
    }
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
