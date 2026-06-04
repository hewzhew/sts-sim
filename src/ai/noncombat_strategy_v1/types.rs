use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategyPlanSupportV1 {
    Blocked,
    Weak,
    Plausible,
    Strong,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum StrategyPlanPressureV1 {
    Low,
    Medium,
    High,
}

#[derive(Clone, Debug, PartialEq)]
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
    pub total_attack_damage: i32,
    pub total_block: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrategyRouteFutureV1 {
    pub min_fires: usize,
    pub max_fires: usize,
    pub first_fire_floor: Option<i32>,
    pub max_early_pressure: usize,
    pub need_heal: f32,
    pub avoid_damage: f32,
}

#[derive(Clone, Debug, PartialEq)]
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

    pub fn route_package(&self, id: StrategyRoutePackageIdV1) -> Option<&StrategyRoutePackageV1> {
        self.route_packages.iter().find(|package| package.id == id)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeckPlanHypothesisV1 {
    pub id: StrategyPlanIdV1,
    pub support: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub blockers: Vec<String>,
    pub opportunity_costs: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategyDeckFormationStageV1 {
    StarterShell,
    Transitional,
    PlanSeeded,
    PlanCommitted,
    Mature,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategyDeckFormationNeedV1 {
    Frontload,
    Block,
    Scaling,
    DrawEnergy,
    Consistency,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrategyDeckFormationV1 {
    pub stage: StrategyDeckFormationStageV1,
    pub needs: Vec<StrategyDeckFormationNeedV1>,
    pub strengths: Vec<StrategyPlanIdV1>,
    pub blockers: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategyRoutePackageIdV1 {
    CombatPatchWindow,
    UpgradeCommitment,
    CorePlanProtection,
    RecoveryPressure,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrategyRoutePackageV1 {
    pub id: StrategyRoutePackageIdV1,
    pub support: StrategyPlanSupportV1,
    pub evidence: Vec<String>,
    pub risks: Vec<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StrategyPlanEffectV1 {
    UpgradeSink,
    UpgradeBudgetConsumer,
    StrengthPayoff,
    FrontloadDamage,
    WeakCoverage,
    DamageMitigation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrategyCandidatePlanDeltaV1 {
    pub effects: Vec<StrategyPlanEffectV1>,
    pub support: StrategyPlanSupportV1,
    pub notes: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StrategyCandidateFactsV1 {
    pub card: CardId,
    pub damage_total: i32,
    pub weak: i32,
    pub strength_gain: i32,
}
