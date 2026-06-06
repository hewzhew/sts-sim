use crate::content::cards::CardId;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyPlanIdV1 {
    FrontloadSurvival,
    WeakControl,
    StrengthScaling,
    UpgradeSink,
    ExhaustEngine,
    BlockEngine,
    StrikeDensity,
    StatusPackage,
    SelfDamage,
    EnergyDraw,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyPlanSupportV1 {
    Blocked,
    Weak,
    Plausible,
    Strong,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub enum StrategyPlanPressureV1 {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct StrategyDeckFactsV1 {
    pub deck_size: usize,
    pub attacks: u8,
    pub skills: u8,
    pub powers: u8,
    pub starter_strikes: u8,
    pub starter_defends: u8,
    pub strength_sources: u8,
    pub strength_payoffs: u8,
    pub weak_sources: u8,
    pub draw_sources: u8,
    pub energy_sources: u8,
    pub vulnerable_sources: u8,
    pub route_upgrade_payoffs: u8,
    pub important_cards_unupgraded: u8,
    pub exhaust_generators: u8,
    pub exhaust_payoffs: u8,
    pub status_generators: u8,
    pub status_payoffs: u8,
    pub block_retention_sources: u8,
    pub block_payoffs: u8,
    pub block_multipliers: u8,
    pub total_attack_damage: i32,
    pub total_block: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategyRouteFutureV1 {
    pub min_fires: usize,
    pub max_fires: usize,
    pub first_fire_floor: Option<i32>,
    pub max_early_pressure: usize,
    pub need_heal: f32,
    pub avoid_damage: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RunStrategySnapshotV1 {
    pub deck: StrategyDeckFactsV1,
    pub route: Option<StrategyRouteFutureV1>,
    pub plans: Vec<DeckPlanHypothesisV1>,
    pub formation: StrategyDeckFormationV1,
    pub route_packages: Vec<StrategyRoutePackageV1>,
}

impl RunStrategySnapshotV1 {
    pub fn plan(&self, id: StrategyPlanIdV1) -> Option<&DeckPlanHypothesisV1> {
        self.plans.iter().find(|plan| plan.id == id)
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct DeckPlanHypothesisV1 {
    pub id: StrategyPlanIdV1,
    pub support: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub opportunity_costs: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyDeckFormationStageV1 {
    StarterShell,
    Transitional,
    PlanSeeded,
    PlanCommitted,
    Mature,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyDeckFormationNeedV1 {
    Frontload,
    Block,
    Scaling,
    DrawEnergy,
    Consistency,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategyDeckFormationV1 {
    pub stage: StrategyDeckFormationStageV1,
    pub needs: Vec<StrategyDeckFormationNeedV1>,
    pub strengths: Vec<StrategyPlanIdV1>,
    pub blockers: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyRoutePackageIdV1 {
    CombatPatchWindow,
    UpgradeCommitment,
    CorePlanProtection,
    RecoveryPressure,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategyRoutePackageV1 {
    pub id: StrategyRoutePackageIdV1,
    pub support: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyPlanEffectV1 {
    UpgradeSink,
    UpgradeBudgetConsumer,
    StrengthPayoff,
    FrontloadDamage,
    WeakCoverage,
    DamageMitigation,
    BlockRetention,
    BlockPayoff,
    BlockMultiplier,
    ExhaustGenerator,
    ExhaustPayoff,
    StatusGenerator,
    StatusPayoff,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategyCandidatePlanDeltaV1 {
    pub effects: Vec<StrategyPlanEffectV1>,
    pub support: StrategyPlanSupportV1,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategyCandidateFactsV1 {
    pub card: CardId,
    pub damage_total: i32,
    pub weak: i32,
    pub strength_gain: i32,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyPackageDomainV2 {
    Archetype,
    Route,
    Resource,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyPackageIdV2 {
    FrontloadSurvival,
    WeakControl,
    StrengthScaling,
    UpgradeSink,
    ExhaustEngine,
    BlockEngine,
    StrikeDensity,
    StatusPackage,
    SelfDamage,
    EnergyDraw,
    CombatPatchWindow,
    UpgradeCommitment,
    CorePlanProtection,
    RecoveryPressure,
    GoldPlan,
    PotionCapacity,
    HpSafety,
    ShopRemoveWindow,
    RelicConstraints,
}

impl StrategyPackageIdV2 {
    pub(crate) fn from_plan_v1(id: StrategyPlanIdV1) -> Self {
        match id {
            StrategyPlanIdV1::FrontloadSurvival => Self::FrontloadSurvival,
            StrategyPlanIdV1::WeakControl => Self::WeakControl,
            StrategyPlanIdV1::StrengthScaling => Self::StrengthScaling,
            StrategyPlanIdV1::UpgradeSink => Self::UpgradeSink,
            StrategyPlanIdV1::ExhaustEngine => Self::ExhaustEngine,
            StrategyPlanIdV1::BlockEngine => Self::BlockEngine,
            StrategyPlanIdV1::StrikeDensity => Self::StrikeDensity,
            StrategyPlanIdV1::StatusPackage => Self::StatusPackage,
            StrategyPlanIdV1::SelfDamage => Self::SelfDamage,
            StrategyPlanIdV1::EnergyDraw => Self::EnergyDraw,
        }
    }

    pub(crate) fn from_route_package_v1(id: StrategyRoutePackageIdV1) -> Self {
        match id {
            StrategyRoutePackageIdV1::CombatPatchWindow => Self::CombatPatchWindow,
            StrategyRoutePackageIdV1::UpgradeCommitment => Self::UpgradeCommitment,
            StrategyRoutePackageIdV1::CorePlanProtection => Self::CorePlanProtection,
            StrategyRoutePackageIdV1::RecoveryPressure => Self::RecoveryPressure,
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategyPackageV2 {
    pub id: StrategyPackageIdV2,
    pub domain: StrategyPackageDomainV2,
    pub support: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategyResourceFactsV2 {
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub estimated_purge_cost: i32,
    pub potion_slots: usize,
    pub potion_count: usize,
    pub empty_potion_slots: usize,
    pub curses: usize,
    pub removable_curses: usize,
    pub starter_cards: usize,
    pub relic_constraints: Vec<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyThreatTagV1 {
    HighIncomingDamage,
    MultiHit,
    StrengthDebuffValuable,
    WeakValuable,
    AoEValuable,
    ArtifactBlocksDebuff,
    StatusFlood,
    SplitThreshold,
    ModeShiftThreshold,
    SkillPunish,
    PowerPunish,
    CardPlayLimit,
    LongFightScaling,
    SetupWindow,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum StrategyThreatSourceV1 {
    ActBoss,
    ActElitePool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategyThreatSourceRecordV1 {
    pub tag: StrategyThreatTagV1,
    pub source: StrategyThreatSourceV1,
    pub subject: String,
    pub evidence: String,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct StrategyThreatProfileV1 {
    pub boss: Option<String>,
    pub tags: Vec<StrategyThreatTagV1>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<StrategyThreatSourceRecordV1>,
    pub evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RunStrategySnapshotV2 {
    pub(crate) v1: RunStrategySnapshotV1,
    pub resources: StrategyResourceFactsV2,
    pub threats: StrategyThreatProfileV1,
    pub packages: Vec<StrategyPackageV2>,
}

impl RunStrategySnapshotV2 {
    pub fn package(&self, id: StrategyPackageIdV2) -> Option<&StrategyPackageV2> {
        self.packages.iter().find(|package| package.id == id)
    }

    pub fn support(&self, id: StrategyPackageIdV2) -> StrategyPlanSupportV1 {
        self.package(id)
            .map(|package| package.support)
            .unwrap_or(StrategyPlanSupportV1::Blocked)
    }

    pub fn has_formation_strength(&self, id: StrategyPackageIdV2) -> bool {
        self.v1
            .formation
            .strengths
            .iter()
            .any(|strength| StrategyPackageIdV2::from_plan_v1(*strength) == id)
    }
}
