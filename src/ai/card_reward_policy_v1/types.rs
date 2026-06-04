use crate::content::cards::{CardId, CardRarity, CardType};

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardPolicyConfigV1 {
    /// Enables automatic picks only when a candidate receives an explicit
    /// certificate from the evidence gate. This does not enable score fallback.
    pub allow_automatic_pick_certificates: bool,
}

impl Default for CardRewardPolicyConfigV1 {
    fn default() -> Self {
        Self {
            allow_automatic_pick_certificates: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardDecisionContextV1 {
    pub run: CardRewardRunContextV1,
    pub deck: DeckProfileV1,
    pub route: Option<CardRewardRouteEvidenceV1>,
    pub plans: CardRewardStrategicPlansV1,
    pub candidates: Vec<CardRewardCandidateEvidenceV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardRunContextV1 {
    pub act: u8,
    pub floor: i32,
    pub ascension: u8,
    pub class: &'static str,
    pub boss: Option<String>,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardRouteEvidenceV1 {
    pub route_policy: &'static str,
    pub selected_route: Option<CardRewardSelectedRouteV1>,
    pub candidate_count: usize,
    pub need_card_rewards: f32,
    pub need_upgrade: f32,
    pub need_heal: f32,
    pub can_take_elite: f32,
    pub avoid_damage: f32,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
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
}

#[derive(Clone, Debug, PartialEq)]
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

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardCandidateEvidenceV1 {
    pub index: usize,
    pub card: CardId,
    pub name: &'static str,
    pub card_type: CardType,
    pub facts: CardRewardFactsV1,
    pub impact: CardRewardCandidateImpactV1,
    pub plan_delta: CardRewardCandidatePlanDeltaV1,
}

pub type CardRewardStrategicPlansV1 = crate::ai::noncombat_strategy_v1::RunStrategySnapshotV2;
pub type CardRewardCandidatePlanDeltaV1 =
    crate::ai::noncombat_strategy_v1::StrategyCandidatePlanDeltaV1;
pub type CardRewardPlanEffectV1 = crate::ai::noncombat_strategy_v1::StrategyPlanEffectV1;
pub type CardRewardPlanSupportV1 = crate::ai::noncombat_strategy_v1::StrategyPlanSupportV1;

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardFactsV1 {
    pub card: CardId,
    pub name: &'static str,
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
    pub unsupported_mechanics: Vec<&'static str>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CardRewardDamageFactsV1 {
    pub damage_per_hit: i32,
    pub hit_count: i32,
    pub total_damage: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardCandidateImpactV1 {
    pub added_deck_size: i32,
    pub frontload_damage_delta: i32,
    pub block_delta: i32,
    pub draw_delta: i32,
    pub energy_delta: i32,
    pub scaling_signals: Vec<CardRewardScalingSignalV1>,
    pub dependency_assessments: Vec<CardRewardDependencyAssessmentV1>,
    pub certification_blockers: Vec<CardRewardEvidenceGapV1>,
    pub evidence_notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardRewardScalingSignalV1 {
    StrengthGain,
    StrengthPayoff,
    Vulnerable,
    Weak,
    EnemyStrengthDown,
    ExhaustPayoff,
    StatusPayoff,
    BlockEngine,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardDependencyAssessmentV1 {
    pub dependency: CardRewardPickDependencyV1,
    pub status: CardRewardDependencyStatusV1,
    pub reason: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardRewardDependencyStatusV1 {
    Satisfied,
    Unsatisfied,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardRewardEvidenceGapV1 {
    MissingRouteEvidence,
    MissingValueEstimate,
    UncalibratedValueEstimate,
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
    NoAutoPickCertificate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardRewardValueSourceV1 {
    ImpactPrior,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardRewardValueStatusV1 {
    UncalibratedPrior,
    CounterfactualProbe,
    OutcomeCalibrated,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardValueEstimateV1 {
    pub index: usize,
    pub card: CardId,
    pub source: CardRewardValueSourceV1,
    pub status: CardRewardValueStatusV1,
    pub survival_delta: f32,
    pub progress_delta: f32,
    pub deck_consistency_delta: f32,
    pub uncertainty: f32,
    pub components: Vec<CardRewardValueComponentV1>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardValueComponentV1 {
    pub name: &'static str,
    pub value: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardPickCertificateV1 {
    pub index: usize,
    pub card: CardId,
    pub confidence: f32,
    pub reasons: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardRewardDecisionV1 {
    pub action: CardRewardPolicyActionV1,
    pub context: CardRewardDecisionContextV1,
    pub candidates: Vec<CardRewardCandidateEvidenceV1>,
    pub value_estimates: Vec<CardRewardValueEstimateV1>,
    pub evidence_gaps: Vec<CardRewardEvidenceGapV1>,
    pub pick_certificate: Option<CardRewardPickCertificateV1>,
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
    },
}
