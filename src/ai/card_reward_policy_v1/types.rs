use crate::content::cards::{CardId, CardRarity, CardType};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardPolicyConfigV1 {
    /// Enables automatic picks only when the generic value gate accepts a
    /// calibrated estimate. This does not enable score fallback.
    pub allow_autopilot_value_gate: bool,
    /// Enables behavior-policy picks from unpromoted but structured public
    /// estimates. These picks are diagnostic autoplay, not teacher labels.
    pub allow_behavior_autopick_gate: bool,
    pub behavior_min_total_delta: f32,
    pub behavior_min_margin: f32,
    pub behavior_max_uncertainty: f32,
}

impl Default for CardRewardPolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_autopilot_value_gate: true,
            allow_behavior_autopick_gate: false,
            behavior_min_total_delta: 0.45,
            behavior_min_margin: 0.35,
            behavior_max_uncertainty: 0.82,
        }
    }
}

impl CardRewardPolicyConfigV1 {
    pub fn behavior_autopick() -> Self {
        Self {
            allow_behavior_autopick_gate: true,
            ..Default::default()
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CardRewardEstimatorInputsV1 {
    pub external_value_estimates: Vec<CardRewardValueEstimateV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardDecisionContextV1 {
    pub run: CardRewardRunContextV1,
    pub deck: DeckProfileV1,
    #[serde(default)]
    pub startup: crate::ai::deck_startup_profile_v1::DeckStartupProfileV1,
    #[serde(default)]
    pub deck_shape: crate::ai::deck_shape_v1::DeckShapeProfileV1,
    #[serde(default)]
    pub run_debt: crate::ai::strategic::RunDebtLedgerV1,
    pub route: Option<CardRewardRouteEvidenceV1>,
    pub strategy: CardRewardStrategySnapshotV2,
    pub has_singing_bowl: bool,
    pub candidates: Vec<CardRewardCandidateEvidenceV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardRunContextV1 {
    pub act: u8,
    pub floor: i32,
    pub ascension: u8,
    pub class: String,
    pub boss: Option<String>,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardRouteEvidenceV1 {
    pub route_policy: String,
    pub selected_route: Option<CardRewardSelectedRouteV1>,
    pub candidate_count: usize,
    pub need_card_rewards: f32,
    pub need_upgrade: f32,
    pub need_heal: f32,
    pub can_take_elite: f32,
    pub avoid_damage: f32,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct CardRewardSelectedRouteV1 {
    pub next_x: i32,
    pub next_y: i32,
    pub min_fires: usize,
    pub max_fires: usize,
    pub first_fire_floor: Option<i32>,
    pub min_elites: usize,
    pub max_elites: usize,
    pub min_early_pressure: usize,
    pub max_early_pressure: usize,
    #[serde(default)]
    pub first_elite_forced: bool,
    #[serde(default)]
    pub max_hallways_before_first_elite: usize,
    #[serde(default)]
    pub can_bail_to_rest_before_first_elite: bool,
    #[serde(default)]
    pub can_bail_to_shop_before_first_elite: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DeckProfileV1 {
    pub deck_size: usize,
    pub attacks: u8,
    pub skills: u8,
    pub powers: u8,
    pub curses: u8,
    pub starter_strikes: u8,
    pub starter_defends: u8,
    pub total_attack_damage: i32,
    pub total_block: i32,
    pub draw_cards: u8,
    pub energy_sources: u8,
    pub strength_sources: u8,
    #[serde(default)]
    pub temporary_strength_bursts: u8,
    #[serde(default)]
    pub strength_converters: u8,
    #[serde(default)]
    pub convertible_strength_sources: u8,
    pub strength_payoffs: u8,
    pub vulnerable_sources: u8,
    pub weak_sources: u8,
    pub exhaust_generators: u8,
    pub exhaust_payoffs: u8,
    pub status_generators: u8,
    pub status_payoffs: u8,
    pub route_upgrade_payoffs: u8,
    pub important_cards_unupgraded: u8,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardCandidateEvidenceV1 {
    pub index: usize,
    pub card: CardId,
    #[serde(default)]
    pub same_card_count: usize,
    pub name: String,
    pub card_type: CardType,
    pub facts: CardRewardFactsV1,
    pub impact: CardRewardCandidateImpactV1,
    pub plan_delta: CardRewardCandidatePlanDeltaV1,
}

pub type CardRewardStrategySnapshotV2 = crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2;
pub type CardRewardCandidatePlanDeltaV1 =
    crate::ai::noncombat_strategy_v1::StrategyCandidatePlanDeltaV1;
pub type CardRewardPlanEffectV1 = crate::ai::noncombat_strategy_v1::StrategyPlanEffectV1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardFactsV1 {
    pub card: CardId,
    #[serde(default)]
    pub upgrades: u8,
    pub name: String,
    pub card_type: CardType,
    pub rarity: CardRarity,
    pub cost: i8,
    pub damage: CardRewardDamageFactsV1,
    pub block: i32,
    pub draw_cards: i32,
    pub energy_gain: i32,
    pub vulnerable: i32,
    pub weak: i32,
    pub strength_gain: i32,
    pub enemy_strength_down: i32,
    pub exhausts: bool,
    pub exhausts_other_cards: bool,
    pub adds_status_cards: i32,
    pub upgrades_cards: bool,
    pub is_random_output: bool,
    pub has_conditional_playability: bool,
    pub is_aoe: bool,
    pub pick_dependencies: Vec<CardRewardPickDependencyV1>,
    pub unsupported_mechanics: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardSemanticProfileV1 {
    pub card: CardId,
    pub name: String,
    pub roles: Vec<CardRewardSemanticRoleV1>,
    pub dependencies: Vec<CardRewardPickDependencyV1>,
    pub unsupported_mechanics: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
pub enum CardRewardSemanticRoleV1 {
    FrontloadDamage,
    AoeDamage,
    Block,
    BlockRetention,
    BlockMultiplier,
    CardDraw,
    EnergySource,
    Vulnerable,
    Weak,
    EnemyStrengthDown,
    ScalingSource,
    TemporaryStrengthBurst,
    StrengthPayoff,
    BlockPayoff,
    StrikePayoff,
    UpgradePayoff,
    ExhaustGenerator,
    ExhaustReuse,
    ExhaustPayoff,
    StatusGenerator,
    StatusPayoff,
    SelfDamagePayoff,
    PackagePayoff,
    RandomOutput,
    ConditionalPlayability,
    UnsupportedMechanics,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct CardRewardDamageFactsV1 {
    pub damage_per_hit: i32,
    pub hit_count: i32,
    pub total_damage: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardCandidateImpactV1 {
    pub added_deck_size: i32,
    pub frontload_damage_delta: i32,
    pub block_delta: i32,
    pub draw_delta: i32,
    pub energy_delta: i32,
    pub scaling_signals: Vec<CardRewardScalingSignalV1>,
    pub dependency_assessments: Vec<CardRewardDependencyAssessmentV1>,
    pub approval_blockers: Vec<CardRewardEvidenceGapV1>,
    pub evidence_notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CardRewardScalingSignalV1 {
    StrengthGain,
    TemporaryStrengthBurst,
    StrengthPayoff,
    Vulnerable,
    Weak,
    EnemyStrengthDown,
    ExhaustPayoff,
    StatusPayoff,
    BlockEngine,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardDependencyAssessmentV1 {
    pub dependency: CardRewardPickDependencyV1,
    pub status: CardRewardDependencyStatusV1,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CardRewardDependencyStatusV1 {
    Satisfied,
    Unsatisfied,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CardRewardPickDependencyV1 {
    RouteUpgradeDensity,
    StrengthScaling,
    BlockDensity,
    StrikeDensity,
    ExhaustPackage,
    StatusPackage,
    SelfDamagePackage,
    RandomOutputPolicy,
    ConditionalPlayabilityPolicy,
    UnsupportedMechanics,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CardRewardEvidenceGapV1 {
    MissingRouteEvidence,
    MissingValueEstimate,
    UncalibratedValueEstimate,
    IneligibleValueSource,
    ValueNotPositive,
    ValueMarginTooSmall,
    ValueUncertaintyTooHigh,
    UnresolvedCandidateDependencies,
    StrategicCompilerRejectedCandidate,
    MissingStrategicPlanEvidence,
    UnsatisfiedRouteUpgradeEvidence,
    UnsatisfiedStrengthScalingEvidence,
    UnsatisfiedBlockDensityEvidence,
    UnsatisfiedStrikeDensityEvidence,
    UnsatisfiedExhaustPackageEvidence,
    UnsatisfiedStatusPackageEvidence,
    UnsupportedCardMechanics,
    RandomOutcomeRequiresPolicy,
    ConditionalPlayabilityRequiresPolicy,
    SingingBowlAddsMaxHpChoice,
    NoDecisionApproval,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CardRewardValueSourceV1 {
    UncalibratedImpactPrior,
    StrategyPackage,
    OutcomeCalibration,
    PublicCombatHeuristic,
    CombatProbe,
    RouteRisk,
    LearnedValue,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum CardRewardValueStatusV1 {
    UncalibratedPrior,
    StrategyPackageEstimate,
    StrategyPackageCalibrated,
    PublicCombatHeuristic,
    CounterfactualProbe,
    OutcomeCalibrated,
    RouteRiskEstimate,
    RouteRiskCalibrated,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardValueEstimateV1 {
    pub index: usize,
    pub card: CardId,
    pub source: CardRewardValueSourceV1,
    pub status: CardRewardValueStatusV1,
    pub survival_delta: f32,
    pub progress_delta: f32,
    pub deck_consistency_delta: f32,
    pub uncertainty: f32,
    #[serde(default)]
    pub eligibility: CardRewardValueEligibilityV1,
    pub components: Vec<CardRewardValueComponentV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardRewardValueEligibilityV1 {
    pub usable_for_value_estimate: bool,
    pub usable_for_autopilot_gate: bool,
    pub reasons: Vec<CardRewardValueEligibilityReasonV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bucket_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<CardRewardValueHorizonV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome_sample_count: Option<usize>,
}

impl Default for CardRewardValueEligibilityV1 {
    fn default() -> Self {
        Self {
            usable_for_value_estimate: false,
            usable_for_autopilot_gate: false,
            reasons: vec![CardRewardValueEligibilityReasonV1::MissingEligibilityMetadata],
            bucket_key: None,
            horizon: None,
            outcome_sample_count: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardValueEligibilityReasonV1 {
    MissingEligibilityMetadata,
    UncalibratedPriorNeverGateEligible,
    OutcomeCalibrationBucketNotGateEligible,
    MissingDistinctSeedCount,
    MissingRulesetVersion,
    MissingDataRoleProvenance,
    HiddenSimulatorStateUsed,
    ShortHorizonMetricOnly,
    StrategyPackageEstimateNotPromoted,
    StrategyPackageCalibrationNotGateEligible,
    PublicCombatHeuristicNotGateEligible,
    CounterfactualProbeNotGateEligible,
    RouteRiskEstimateNotPromoted,
    RouteRiskCalibrationNotGateEligible,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardRewardValueHorizonV1 {
    NextCombatHpLoss,
    NextCombatPublicProbe,
    NextCombatCounterfactualProbe,
    VisibleRouteRisk,
    CurrentStrategyPackage,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardEstimatorArbitrationV1 {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub label_role: &'static str,
    pub input_estimate_count: usize,
    pub gate_value_estimates: Vec<CardRewardValueEstimateV1>,
    pub candidate_reports: Vec<CardRewardEstimatorCandidateArbitrationV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardEstimatorCandidateArbitrationV1 {
    pub index: usize,
    pub card: CardId,
    pub input_estimate_count: usize,
    pub selected_source: Option<CardRewardValueSourceV1>,
    pub selected_status: Option<CardRewardValueStatusV1>,
    pub selected_for_gate: bool,
    pub autopilot_source_eligible: bool,
    pub selected_estimate_gate_eligible: bool,
    pub rejected_reasons: Vec<CardRewardEvidenceGapV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CardRewardValueComponentV1 {
    pub name: String,
    pub value: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardDecisionApprovalV1 {
    pub index: usize,
    pub card: CardId,
    pub confidence: f32,
    pub selection_mode: &'static str,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardAutopilotGateReportV1 {
    pub hidden_free: bool,
    pub candidate_coverage_complete: bool,
    pub value_source_eligible: bool,
    pub calibration_status_allowed: bool,
    pub value_vs_skip_positive: bool,
    pub margin_sufficient: bool,
    pub uncertainty_below_limit: bool,
    pub unresolved_dependencies_empty: bool,
    pub selected_candidate_index: Option<usize>,
    pub blocked_reasons: Vec<CardRewardEvidenceGapV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardDecisionV1 {
    pub action: CardRewardPolicyActionV1,
    pub context: CardRewardDecisionContextV1,
    pub candidates: Vec<CardRewardCandidateEvidenceV1>,
    pub value_estimates: Vec<CardRewardValueEstimateV1>,
    pub value_arbitration: CardRewardEstimatorArbitrationV1,
    pub autopilot_gate: CardRewardAutopilotGateReportV1,
    pub evidence_gaps: Vec<CardRewardEvidenceGapV1>,
    pub decision_approval: Option<CardRewardDecisionApprovalV1>,
    pub strategic_trace: crate::ai::strategic::StrategicDecisionTrace,
    pub label_role: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CardRewardPolicyActionV1 {
    Pick {
        index: usize,
        card: CardId,
        confidence: f32,
        reason: String,
    },
    Stop {
        reason: String,
        disposition: CardRewardStopDispositionV1,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardRewardStopDispositionV1 {
    MayOpenRewardItem,
    KeepRewardItemClosed,
}
