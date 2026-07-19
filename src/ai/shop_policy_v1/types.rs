use crate::ai::block_plan_profile_v1::BlockPlanProfileV1;
use crate::ai::deck_startup_profile_v1::DeckStartupProfileV1;
use crate::ai::noncombat_decision_v1::{
    CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1, EvidenceItemV1,
    EvidenceKindV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
    PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
    ValueComponentV1, ValueEstimateV1, NONCOMBAT_DECISION_RECORD_SCHEMA_NAME,
    NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
};
use crate::ai::noncombat_strategy_v1::{RunStrategySnapshotV2, StrategyPlanSupportV1};
use crate::ai::strategic::RunDebtLedgerV1;
use crate::ai::strength_profile_v1::StrengthProfileV1;
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Clone, Debug, PartialEq)]
pub struct ShopDecisionContextV1 {
    pub strategy: RunStrategySnapshotV2,
    pub strength: StrengthProfileV1,
    pub block_plan: BlockPlanProfileV1,
    pub startup: DeckStartupProfileV1,
    pub run_debt: RunDebtLedgerV1,
    pub boss_matchup_pressures: Vec<crate::ai::boss_matchup::BossMatchupShadowPressureV1>,
    pub upgrade_need: ShopUpgradeNeedProfileV1,
    pub need: ShopNeedProfileV1,
    pub visit: ShopVisitFactsV1,
    pub candidates: Vec<ShopCandidateEvidenceV1>,
    pub affordable_purchase_exists: bool,
    pub conversion_pressure: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShopVisitFactsV1 {
    pub entry_gold: i32,
    pub spent_gold_in_visit: bool,
    pub maw_bank: ShopMawBankStateV1,
    pub future_shop: ShopFutureShopV1,
    pub next_threat: ShopThreatWindowV1,
    /// Exact scheduled elite identity is oracle-only evidence. Generic callers
    /// leave this as `None` and retain pool-level reasoning.
    pub next_elite_encounter: Option<EncounterId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopMawBankStateV1 {
    Absent,
    LiveUnspent,
    BrokenThisVisit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopFutureShopV1 {
    Unknown,
    NotVisible,
    VisibleIn(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopThreatWindowV1 {
    Unknown,
    NoVisibleHardFight,
    EliteIn(u8),
    BossIn(i32),
}

impl ShopDecisionContextV1 {
    pub fn with_visit_facts(mut self, visit: ShopVisitFactsV1) -> Self {
        self.visit = visit;
        self
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShopUpgradeNeedProfileV1 {
    pub unpaid_core_count: usize,
    pub pressure: f32,
    pub evidence: Vec<String>,
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
    pub same_card_count: usize,
    pub purchase_target: Option<ShopPurchaseTargetV1>,
    /// Legacy purchase estimate retained as an input signal. This is not a
    /// final priority; rollout/frontier admission must go through ShopPlanEvaluation.
    pub legacy_estimate: Option<i32>,
    pub gold_cost: Option<i32>,
    pub support_gate: StrategyPlanSupportV1,
    pub signals: Vec<ShopPurchaseSignalV1>,
    pub risk_kinds: Vec<ShopPurchaseRiskV1>,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopPurchaseSignalV1 {
    BossAnswer,
    EngineClosure,
    StartupAccess,
    CoreDefenseOrSurvival,
    CoreCardAccess,
    ImmediateRecovery,
    CombatShapeChange,
    DigestCapacity,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopPurchaseRiskV1 {
    BossEnemyStrengthMultiHit,
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
    FunctionalRepairPurge,
    PurchaseOpportunity,
    Leave,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPolicyConfigV1 {
    pub allow_curse_purge: bool,
    pub allow_starter_strike_purge_when_core_plan_protected: bool,
    pub allow_functional_repair_purge: bool,
    pub allow_high_impact_purchase: bool,
    /// Legacy estimate threshold for relic purchases, which do not yet have
    /// the typed strategic evaluator used by cards and potions.
    pub high_impact_relic_legacy_estimate_threshold: i32,
}

impl Default for ShopPolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_curse_purge: true,
            allow_starter_strike_purge_when_core_plan_protected: true,
            allow_functional_repair_purge: true,
            allow_high_impact_purchase: true,
            high_impact_relic_legacy_estimate_threshold: 900,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopDecisionSourceV1 {
    PlanEvaluationCompiler,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopCompileModeV1 {
    ExecuteOne,
    ExecutePlanHead,
    BranchTopK { max_plans: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanKindV1 {
    Execute,
    Stop,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledShopDecisionV1 {
    pub frontier: ShopPlanFrontierV1,
    pub rollout_head: Option<ShopPlanProjectionV1>,
    pub branch_frontier: Vec<ShopPlanProjectionV1>,
    /// Compatibility projection for older single-action consumers. New code
    /// should read rollout_head/frontier instead of treating this as a claim
    /// that the plan is globally best.
    pub compat_selected_plan: ShopPlanV1,
    /// Compatibility projection for older branch consumers. New branch code
    /// should read branch_frontier/frontier instead of assuming everything
    /// not in compat_selected_plan is merely an alternative.
    pub compat_alternatives: Vec<ShopPlanV1>,
    pub candidate_plans: Vec<ShopPlanCandidateV1>,
    pub strategic_trace: crate::ai::strategic::StrategicDecisionTrace,
    pub source: ShopDecisionSourceV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanFrontierV1 {
    pub plans: Vec<ShopPlanCandidateV1>,
    pub lanes: Vec<ShopPlanLaneGroupV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanLaneGroupV1 {
    pub lane: ShopPlanLaneV1,
    pub plan_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanProjectionV1 {
    pub plan_id: String,
    pub lane: ShopPlanLaneV1,
    pub role: ShopPlanProjectionRoleV1,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanProjectionRoleV1 {
    RolloutHead,
    BranchExplore,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ShopPlanLaneV1 {
    Purge,
    BuyRelic,
    BuyPotion,
    BuyCardBossAnswer,
    BuyCardMissingCeiling,
    BuyCardFutureSustain,
    BuyCardScalingEngine,
    BuyCardDrawAccess,
    BuyCardExhaustAccess,
    BuyCardDefense,
    BuyCardFrontload,
    BuyCardGeneric,
    Leave,
    Stop,
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
    StopFallback,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShopPlanEvaluationV1 {
    /// Legacy compatibility verdict. This is no longer the single source of
    /// truth for both automation and branch exploration; use
    /// rollout_admission and branch_admission at new call sites.
    pub verdict: ShopPlanVerdictV1,
    pub rollout_admission: ShopPlanRolloutAdmissionV1,
    pub branch_admission: ShopPlanBranchAdmissionV1,
    pub tier: i32,
    pub score: i32,
    pub confidence: f32,
    pub reasons: Vec<String>,
    /// Legacy estimate copied into traces for auditability. The component
    /// scorer deliberately does not add this amount to plan value.
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopPlanRolloutAdmissionV1 {
    pub status: ShopPlanRolloutAdmissionStatusV1,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanRolloutAdmissionStatusV1 {
    Admit,
    Reject,
}

impl ShopPlanRolloutAdmissionV1 {
    pub(crate) fn admit(reason: impl Into<String>) -> Self {
        Self {
            status: ShopPlanRolloutAdmissionStatusV1::Admit,
            reason: reason.into(),
        }
    }

    pub(crate) fn reject(reason: impl Into<String>) -> Self {
        Self {
            status: ShopPlanRolloutAdmissionStatusV1::Reject,
            reason: reason.into(),
        }
    }

    pub fn is_admitted(&self) -> bool {
        self.status == ShopPlanRolloutAdmissionStatusV1::Admit
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShopPlanBranchAdmissionV1 {
    pub status: ShopPlanBranchAdmissionStatusV1,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShopPlanBranchAdmissionStatusV1 {
    Admit,
    Reject,
}

impl ShopPlanBranchAdmissionV1 {
    pub(crate) fn admit(reason: impl Into<String>) -> Self {
        Self {
            status: ShopPlanBranchAdmissionStatusV1::Admit,
            reason: reason.into(),
        }
    }

    pub(crate) fn reject(reason: impl Into<String>) -> Self {
        Self {
            status: ShopPlanBranchAdmissionStatusV1::Reject,
            reason: reason.into(),
        }
    }

    pub fn is_admitted(&self) -> bool {
        self.status == ShopPlanBranchAdmissionStatusV1::Admit
    }
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
    BossAnswer,
    ImmediateThreatCoverage,
    MawBankOpportunityCost,
    StopReason,
}

impl ShopPlanEvaluationV1 {
    pub(crate) fn pending() -> Self {
        Self {
            verdict: ShopPlanVerdictV1::Block,
            rollout_admission: ShopPlanRolloutAdmissionV1::reject("pending shop plan evaluation"),
            branch_admission: ShopPlanBranchAdmissionV1::reject("pending shop plan evaluation"),
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
            rollout_admission: ShopPlanRolloutAdmissionV1::admit(
                "shop plan admitted as a default rollout candidate",
            ),
            branch_admission: ShopPlanBranchAdmissionV1::admit(
                "shop plan admitted for branch exploration",
            ),
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
            rollout_admission: ShopPlanRolloutAdmissionV1::admit(
                "shop stop/leave plan admitted as a default rollout candidate",
            ),
            branch_admission: ShopPlanBranchAdmissionV1::admit(
                "shop stop/leave plan admitted for branch exploration",
            ),
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
            rollout_admission: ShopPlanRolloutAdmissionV1::reject(
                "shop plan rejected as a default rollout candidate",
            ),
            branch_admission: ShopPlanBranchAdmissionV1::reject(
                "shop plan rejected for branch exploration",
            ),
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
    /// Legacy estimate copied from candidate/evaluation for trace continuity.
    /// It must not be used as a direct rollout path.
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

impl ShopPlanStepV1 {
    pub(crate) fn strategic_candidate_id_v1(&self) -> String {
        match *self {
            Self::BuyCard { index, card, .. } => format!("shop:buy_card:{index}:{card:?}"),
            Self::BuyRelic { index, relic, .. } => {
                format!("shop:buy_relic:{index}:{relic:?}")
            }
            Self::BuyPotion { index, potion, .. } => {
                format!("shop:buy_potion:{index}:{potion:?}")
            }
            Self::RemoveCard {
                deck_index, card, ..
            } => format!("shop:remove:{deck_index}:{card:?}"),
            Self::LeaveShop => "shop:leave".to_string(),
        }
    }
}

impl CompiledShopDecisionV1 {
    pub fn to_noncombat_decision_record_v1(&self) -> NonCombatDecisionRecordV1 {
        let rollout_plan = self
            .rollout_head
            .as_ref()
            .and_then(|projection| {
                self.candidate_plans
                    .iter()
                    .find(|candidate| candidate.plan.plan_id == projection.plan_id)
                    .map(|candidate| &candidate.plan)
            })
            .unwrap_or(&self.compat_selected_plan);
        let selected_candidate_id = (rollout_plan.kind == ShopPlanKindV1::Execute
            && !rollout_plan.steps.is_empty())
        .then(|| rollout_plan.plan_id.clone());
        let selected_evaluation = self
            .candidate_plans
            .iter()
            .find(|candidate| candidate.plan.plan_id == rollout_plan.plan_id)
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
                    "legacy priority is retained as estimate evidence, not as a rollout entry point"
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
                reason: rollout_plan.reason.clone(),
                confidence: selected_evaluation
                    .map(|evaluation| evaluation.confidence)
                    .or(rollout_plan.legacy_confidence)
                    .unwrap_or(0.0),
                selection_mode: "compiled_shop_rollout_head_v1".to_string(),
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
            command: compiled_shop_plan_command(&candidate.plan),
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
        ValueComponentV1::new(
            format!("rollout_{:?}", evaluation.rollout_admission.status),
            1.0,
        ),
        ValueComponentV1::new(
            format!("branch_{:?}", evaluation.branch_admission.status),
            1.0,
        ),
        ValueComponentV1::new("tier", evaluation.tier as f32),
        ValueComponentV1::new("score", evaluation.score as f32),
        ValueComponentV1::new("confidence", evaluation.confidence),
        ValueComponentV1::new("component_net", evaluation.component_score.net),
    ];
    if let Some(priority) = evaluation.legacy_priority {
        components.push(ValueComponentV1::new("legacy_estimate", priority as f32));
    }

    EvidenceItemV1 {
        kind: EvidenceKindV1::PolicyGate,
        candidate_id: Some(candidate.plan.plan_id.clone()),
        label: format!(
            "shop plan role={:?} verdict={:?} rollout={:?} branch={:?} tier={} score={}",
            candidate.role,
            evaluation.verdict,
            evaluation.rollout_admission.status,
            evaluation.branch_admission.status,
            evaluation.tier,
            evaluation.score
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

fn compiled_shop_plan_command(plan: &ShopPlanV1) -> Option<String> {
    match plan.steps.as_slice() {
        [] => None,
        [step] => Some(compiled_shop_step_command(step)),
        // A multi-step plan cannot be executed truthfully from one observed
        // shop state because each purchase may change prices, inventory, or
        // follow-up interactions. Such a plan must be decomposed and
        // recompiled after the exact engine transition.
        _ => None,
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
