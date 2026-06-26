use crate::ai::block_plan_profile_v1::{block_plan_profile_v1, BlockPlanReadinessV1};
use crate::ai::card_reward_policy_v1::{card_reward_semantic_profile_v1, CardRewardSemanticRoleV1};
use crate::ai::deck_startup_profile_v1::deck_startup_profile_v1;
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyDeckFormationNeedV1,
    StrategyDeckFormationStageV1,
};
use crate::content::cards::{get_card_definition, CardId, CardTag, CardType};
use crate::eval::run_control::RunControlSession;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

use super::model::BranchCampaignBranchV1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignAssessmentSourceV1 {
    SessionState,
    MissingSession,
}

impl BranchCampaignAssessmentSourceV1 {
    fn label(self) -> &'static str {
        match self {
            Self::SessionState => "session_state",
            Self::MissingSession => "missing_session",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignGateStatusV1 {
    Unknown = 0,
    Unsolved = 1,
    AtRisk = 2,
    Passable = 3,
    Solved = 4,
}

impl BranchCampaignGateStatusV1 {
    fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Unsolved => "unsolved",
            Self::AtRisk => "at_risk",
            Self::Passable => "passable",
            Self::Solved => "solved",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignFormationStageV1 {
    Unknown = 0,
    StarterSurvival = 1,
    EarlyStableIncomplete = 2,
    DirectionSeeded = 3,
    PackageForming = 4,
    ScalingOnline = 5,
    EngineOnline = 6,
}

impl BranchCampaignFormationStageV1 {
    fn label(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::StarterSurvival => "starter_survival",
            Self::EarlyStableIncomplete => "early_stable_incomplete",
            Self::DirectionSeeded => "direction_seeded",
            Self::PackageForming => "package_forming",
            Self::ScalingOnline => "scaling_online",
            Self::EngineOnline => "engine_online",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignResourceConversionV1 {
    None = 0,
    BufferOnly = 1,
    PendingConversion = 2,
    StrongConversionWindow = 3,
}

impl BranchCampaignResourceConversionV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::BufferOnly => "buffer_only",
            Self::PendingConversion => "pending_conversion",
            Self::StrongConversionWindow => "strong_conversion_window",
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignJobCoverageV1 {
    pub frontload: BranchCampaignFrontloadProfileV1,
    pub block: BranchCampaignBlockProfileV1,
    pub draw: BranchCampaignDrawProfileV1,
    pub scaling: BranchCampaignScalingProfileV1,
    pub exhaust: BranchCampaignPackageProfileV1,
    pub status: BranchCampaignPackageProfileV1,
    pub debt: BranchCampaignDeckDebtProfileV1,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignFrontloadProfileV1 {
    pub starter_floor: BranchCampaignStarterFrontloadFloorV1,
    pub added_attack_count: u8,
    pub added_attack_quality: BranchCampaignAddedAttackQualityV1,
    pub vulnerable_support: BranchCampaignVulnerableSupportV1,
    pub aoe_support: BranchCampaignAoeSupportV1,
    pub draw_to_damage: BranchCampaignDrawToDamageV1,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignBlockProfileV1 {
    pub starter_floor: BranchCampaignStarterBlockFloorV1,
    pub added_block_count: u8,
    pub added_block_quality: BranchCampaignAddedBlockQualityV1,
    pub skill_liability: BranchCampaignSkillLiabilityV1,
    pub scaling_block: u8,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignDrawProfileV1 {
    pub draw_count: u8,
    pub energy_count: u8,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignScalingProfileV1 {
    pub damage_count: u8,
    pub block_count: u8,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignPackageProfileV1 {
    pub enabler_count: u8,
    pub payoff_count: u8,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignDeckDebtProfileV1 {
    pub deck_size: u8,
    pub starter_strikes: u8,
    pub starter_defends: u8,
    pub starter_unique_cards: u8,
    pub curses: u8,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignStarterFrontloadFloorV1 {
    #[default]
    None = 0,
    Partial = 1,
    FullStarter = 2,
}

impl BranchCampaignStarterFrontloadFloorV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Partial => "partial",
            Self::FullStarter => "full",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignAddedAttackQualityV1 {
    #[default]
    None = 0,
    WeakOne = 1,
    SolidOne = 2,
    MultipleSolid = 3,
    BurstReady = 4,
}

impl BranchCampaignAddedAttackQualityV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::WeakOne => "weak1",
            Self::SolidOne => "solid1",
            Self::MultipleSolid => "multi",
            Self::BurstReady => "burst",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignVulnerableSupportV1 {
    #[default]
    None = 0,
    StarterBash = 1,
    Reliable = 2,
    Premium = 3,
}

impl BranchCampaignVulnerableSupportV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::StarterBash => "bash",
            Self::Reliable => "reliable",
            Self::Premium => "premium",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignAoeSupportV1 {
    #[default]
    None = 0,
    Present = 1,
    Strong = 2,
}

impl BranchCampaignAoeSupportV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Present => "present",
            Self::Strong => "strong",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignDrawToDamageV1 {
    #[default]
    None = 0,
    DrawsMostlyStarter = 1,
    ImprovesGoodTargets = 2,
}

impl BranchCampaignDrawToDamageV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::DrawsMostlyStarter => "starter",
            Self::ImprovesGoodTargets => "targets",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignStarterBlockFloorV1 {
    #[default]
    None = 0,
    Partial = 1,
    FullStarter = 2,
}

impl BranchCampaignStarterBlockFloorV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Partial => "partial",
            Self::FullStarter => "full",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignAddedBlockQualityV1 {
    #[default]
    None = 0,
    LowOne = 1,
    SolidOne = 2,
    MultipleSolid = 3,
    BurstBlock = 4,
}

impl BranchCampaignAddedBlockQualityV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::LowOne => "low1",
            Self::SolidOne => "solid1",
            Self::MultipleSolid => "multi",
            Self::BurstBlock => "burst",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignSkillLiabilityV1 {
    #[default]
    None = 0,
    Low = 1,
    Medium = 2,
    High = 3,
}

impl BranchCampaignSkillLiabilityV1 {
    fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignAssessmentV1 {
    pub source: BranchCampaignAssessmentSourceV1,
    pub formation_stage: BranchCampaignFormationStageV1,
    pub immediate_gate: BranchCampaignGateStatusV1,
    pub act_boss_gate: BranchCampaignGateStatusV1,
    pub transition_gate: BranchCampaignGateStatusV1,
    pub resource_conversion: BranchCampaignResourceConversionV1,
    pub job_coverage: BranchCampaignJobCoverageV1,
    pub missing_critical_jobs: Vec<String>,
}

impl BranchCampaignAssessmentV1 {
    pub fn unknown_v1() -> Self {
        Self {
            source: BranchCampaignAssessmentSourceV1::MissingSession,
            formation_stage: BranchCampaignFormationStageV1::Unknown,
            immediate_gate: BranchCampaignGateStatusV1::Unknown,
            act_boss_gate: BranchCampaignGateStatusV1::Unknown,
            transition_gate: BranchCampaignGateStatusV1::Unknown,
            resource_conversion: BranchCampaignResourceConversionV1::None,
            job_coverage: BranchCampaignJobCoverageV1::default(),
            missing_critical_jobs: Vec::new(),
        }
    }

    pub fn priority_key(&self) -> BranchCampaignAssessmentPriorityKeyV1 {
        (
            i32::from(self.source == BranchCampaignAssessmentSourceV1::SessionState),
            immediate_gate_viability_v1(self.immediate_gate),
            self.act_boss_gate,
            self.transition_gate,
            self.formation_stage,
            -(self.missing_critical_jobs.len() as i32),
            self.resource_conversion,
            self.immediate_gate,
        )
    }
}

pub type BranchCampaignAssessmentPriorityKeyV1 = (
    i32,
    i32,
    BranchCampaignGateStatusV1,
    BranchCampaignGateStatusV1,
    BranchCampaignFormationStageV1,
    i32,
    BranchCampaignResourceConversionV1,
    BranchCampaignGateStatusV1,
);

pub fn campaign_branch_assessment_v1(
    branch: &BranchCampaignBranchV1,
) -> BranchCampaignAssessmentV1 {
    branch
        .assessment
        .clone()
        .unwrap_or_else(BranchCampaignAssessmentV1::unknown_v1)
}

pub fn campaign_branch_assessment_from_session_v1(
    session: &RunControlSession,
) -> BranchCampaignAssessmentV1 {
    let run_state = &session.run_state;
    let strategy = build_run_strategy_snapshot_from_run_state_v2(run_state);
    let formation = strategy.formation_summary();
    let startup = deck_startup_profile_v1(run_state);
    let block_plan = block_plan_profile_v1(run_state);
    let job_coverage = campaign_job_coverage_from_run_state_v1(run_state, &startup, &block_plan);
    let formation_stage = campaign_formation_stage_from_session_v1(
        formation.stage,
        &formation.needs,
        &job_coverage,
        &block_plan.readiness,
    );
    let summary = session_summary_proxy_v1(session);
    let immediate_gate = campaign_immediate_gate_v1(&summary, &job_coverage);
    let act_boss_gate = campaign_act_boss_gate_v1(&summary, &job_coverage, &block_plan.readiness);
    let transition_gate = campaign_transition_gate_v1(&summary, &job_coverage);
    let resource_conversion = campaign_resource_conversion_v1(&summary, session);
    let missing_critical_jobs =
        campaign_missing_critical_jobs_v1(&summary, &job_coverage, &formation.needs);
    BranchCampaignAssessmentV1 {
        source: BranchCampaignAssessmentSourceV1::SessionState,
        formation_stage,
        immediate_gate,
        act_boss_gate,
        transition_gate,
        resource_conversion,
        job_coverage,
        missing_critical_jobs,
    }
}

pub fn render_campaign_branch_assessment_v1(branch: &BranchCampaignBranchV1) -> String {
    let assessment = campaign_branch_assessment_v1(branch);
    render_campaign_assessment_v1(&assessment)
}

fn render_campaign_assessment_v1(assessment: &BranchCampaignAssessmentV1) -> String {
    let missing = if assessment.missing_critical_jobs.is_empty() {
        "-".to_string()
    } else {
        assessment.missing_critical_jobs.join(",")
    };
    format!(
        "assess=[src={} stage={} gate={} boss={} trans={} resource={} jobs=front{{base={} add={} n{} vuln={} aoe={} draw={}}} block{{base={} add={} n{} skill={} scale={}}} d{}/e{} scale{}/{} x{}/{} st{}/{} debt{{size={} str={} def={} uniq={} curse={}}} missing={}]",
        assessment.source.label(),
        assessment.formation_stage.label(),
        assessment.immediate_gate.label(),
        assessment.act_boss_gate.label(),
        assessment.transition_gate.label(),
        assessment.resource_conversion.label(),
        assessment.job_coverage.frontload.starter_floor.label(),
        assessment.job_coverage.frontload.added_attack_quality.label(),
        assessment.job_coverage.frontload.added_attack_count,
        assessment.job_coverage.frontload.vulnerable_support.label(),
        assessment.job_coverage.frontload.aoe_support.label(),
        assessment.job_coverage.frontload.draw_to_damage.label(),
        assessment.job_coverage.block.starter_floor.label(),
        assessment.job_coverage.block.added_block_quality.label(),
        assessment.job_coverage.block.added_block_count,
        assessment.job_coverage.block.skill_liability.label(),
        assessment.job_coverage.block.scaling_block,
        assessment.job_coverage.draw.draw_count,
        assessment.job_coverage.draw.energy_count,
        assessment.job_coverage.scaling.damage_count,
        assessment.job_coverage.scaling.block_count,
        assessment.job_coverage.exhaust.enabler_count,
        assessment.job_coverage.exhaust.payoff_count,
        assessment.job_coverage.status.enabler_count,
        assessment.job_coverage.status.payoff_count,
        assessment.job_coverage.debt.deck_size,
        assessment.job_coverage.debt.starter_strikes,
        assessment.job_coverage.debt.starter_defends,
        assessment.job_coverage.debt.starter_unique_cards,
        assessment.job_coverage.debt.curses,
        missing
    )
}

fn campaign_job_coverage_from_run_state_v1(
    run_state: &RunState,
    startup: &crate::ai::deck_startup_profile_v1::DeckStartupProfileV1,
    block_plan: &crate::ai::block_plan_profile_v1::BlockPlanProfileV1,
) -> BranchCampaignJobCoverageV1 {
    let mut coverage = BranchCampaignJobCoverageV1::default();
    let mut added_attack_best_damage = 0_i32;
    let mut added_attack_solid_count = 0_u8;
    let mut added_block_best = 0_i32;
    let mut added_block_solid_count = 0_u8;
    let mut skill_count = 0_u8;
    for card in &run_state.master_deck {
        let starter_role = starter_card_role_v1(card.id);
        let definition = get_card_definition(card.id);
        let damage = card_damage_v1(card.id, card.upgrades);
        let block = card_block_v1(card.id, card.upgrades);
        let profile = card_reward_semantic_profile_v1(&RewardCard::new(card.id, card.upgrades));
        coverage.debt.deck_size = coverage.debt.deck_size.saturating_add(1);
        if definition.card_type == CardType::Curse {
            coverage.debt.curses = coverage.debt.curses.saturating_add(1);
        }
        if definition.card_type == CardType::Skill {
            skill_count = skill_count.saturating_add(1);
        }
        match starter_role {
            BranchCampaignStarterCardRoleV1::StarterStrike => {
                coverage.debt.starter_strikes = coverage.debt.starter_strikes.saturating_add(1);
            }
            BranchCampaignStarterCardRoleV1::StarterDefend => {
                coverage.debt.starter_defends = coverage.debt.starter_defends.saturating_add(1);
            }
            BranchCampaignStarterCardRoleV1::StarterUnique => {
                coverage.debt.starter_unique_cards =
                    coverage.debt.starter_unique_cards.saturating_add(1);
            }
            BranchCampaignStarterCardRoleV1::NonStarter => {}
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::FrontloadDamage) {
            if starter_role == BranchCampaignStarterCardRoleV1::NonStarter {
                coverage.frontload.added_attack_count =
                    coverage.frontload.added_attack_count.saturating_add(1);
                added_attack_best_damage = added_attack_best_damage.max(damage);
                if damage >= 10 || card.upgrades > 0 || definition.cost == -1 {
                    added_attack_solid_count = added_attack_solid_count.saturating_add(1);
                }
            }
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::AoeDamage) {
            coverage.frontload.aoe_support = coverage.frontload.aoe_support.max(
                if damage >= 18 || matches!(card.id, CardId::Whirlwind | CardId::Immolate) {
                    BranchCampaignAoeSupportV1::Strong
                } else {
                    BranchCampaignAoeSupportV1::Present
                },
            );
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::Vulnerable) {
            coverage.frontload.vulnerable_support = coverage
                .frontload
                .vulnerable_support
                .max(vulnerable_support_for_card_v1(card.id));
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::Block) {
            if starter_role == BranchCampaignStarterCardRoleV1::NonStarter {
                coverage.block.added_block_count =
                    coverage.block.added_block_count.saturating_add(1);
                added_block_best = added_block_best.max(block);
                if block >= 8 {
                    added_block_solid_count = added_block_solid_count.saturating_add(1);
                }
            }
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::CardDraw)
            || has_role_v1(&profile.roles, CardRewardSemanticRoleV1::CycleAccess)
        {
            coverage.draw.draw_count = coverage.draw.draw_count.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::EnergySource) {
            coverage.draw.energy_count = coverage.draw.energy_count.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::ScalingSource)
            || has_role_v1(
                &profile.roles,
                CardRewardSemanticRoleV1::CombatExternalPayoff,
            )
        {
            coverage.scaling.damage_count = coverage.scaling.damage_count.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::BlockRetention)
            || has_role_v1(&profile.roles, CardRewardSemanticRoleV1::BlockMultiplier)
            || block_plan.readiness >= BlockPlanReadinessV1::Supported
        {
            coverage.scaling.block_count = coverage.scaling.block_count.saturating_add(1);
            coverage.block.scaling_block = coverage.block.scaling_block.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::ExhaustGenerator) {
            coverage.exhaust.enabler_count = coverage.exhaust.enabler_count.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::ExhaustPayoff) {
            coverage.exhaust.payoff_count = coverage.exhaust.payoff_count.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::StatusGenerator) {
            coverage.status.enabler_count = coverage.status.enabler_count.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::StatusPayoff) {
            coverage.status.payoff_count = coverage.status.payoff_count.saturating_add(1);
        }
    }
    coverage.frontload.starter_floor = starter_frontload_floor_v1(
        coverage.debt.starter_strikes,
        coverage.debt.starter_unique_cards,
    );
    coverage.block.starter_floor = starter_block_floor_v1(coverage.debt.starter_defends);
    coverage.frontload.added_attack_quality = added_attack_quality_v1(
        coverage.frontload.added_attack_count,
        added_attack_solid_count,
        added_attack_best_damage,
    );
    coverage.block.added_block_quality = added_block_quality_v1(
        coverage.block.added_block_count,
        added_block_solid_count,
        added_block_best,
    );
    coverage.block.skill_liability = skill_liability_v1(skill_count);
    coverage.frontload.draw_to_damage = draw_to_damage_v1(&coverage);
    coverage.exhaust.enabler_count = coverage
        .exhaust
        .enabler_count
        .saturating_add(startup.exhaust_engine_count);
    coverage.exhaust.payoff_count = coverage
        .exhaust
        .payoff_count
        .saturating_add(startup.exhaust_payoff_count);
    coverage
}

fn has_role_v1(roles: &[CardRewardSemanticRoleV1], role: CardRewardSemanticRoleV1) -> bool {
    roles.contains(&role)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum BranchCampaignStarterCardRoleV1 {
    NonStarter,
    StarterStrike,
    StarterDefend,
    StarterUnique,
}

fn starter_card_role_v1(card: CardId) -> BranchCampaignStarterCardRoleV1 {
    let definition = get_card_definition(card);
    if definition.tags.contains(&CardTag::StarterStrike) {
        return BranchCampaignStarterCardRoleV1::StarterStrike;
    }
    if definition.tags.contains(&CardTag::StarterDefend) {
        return BranchCampaignStarterCardRoleV1::StarterDefend;
    }
    if matches!(
        card,
        CardId::Bash
            | CardId::Neutralize
            | CardId::Survivor
            | CardId::Zap
            | CardId::Dualcast
            | CardId::Eruption
            | CardId::Vigilance
    ) {
        return BranchCampaignStarterCardRoleV1::StarterUnique;
    }
    BranchCampaignStarterCardRoleV1::NonStarter
}

fn card_damage_v1(card: CardId, upgrades: u8) -> i32 {
    let definition = get_card_definition(card);
    let upgrades = i32::from(upgrades);
    let damage_per_hit = (definition.base_damage + definition.upgrade_damage * upgrades).max(0);
    match card {
        CardId::TwinStrike => damage_per_hit.saturating_mul(2),
        CardId::SwordBoomerang => damage_per_hit
            .saturating_mul((definition.base_magic + definition.upgrade_magic * upgrades).max(0)),
        CardId::RiddleWithHoles => damage_per_hit.saturating_mul(5),
        _ => damage_per_hit,
    }
}

fn card_block_v1(card: CardId, upgrades: u8) -> i32 {
    let definition = get_card_definition(card);
    (definition.base_block + definition.upgrade_block * i32::from(upgrades)).max(0)
}

fn starter_frontload_floor_v1(
    starter_strikes: u8,
    starter_unique_cards: u8,
) -> BranchCampaignStarterFrontloadFloorV1 {
    if starter_strikes >= 4 && starter_unique_cards > 0 {
        BranchCampaignStarterFrontloadFloorV1::FullStarter
    } else if starter_strikes > 0 || starter_unique_cards > 0 {
        BranchCampaignStarterFrontloadFloorV1::Partial
    } else {
        BranchCampaignStarterFrontloadFloorV1::None
    }
}

fn starter_block_floor_v1(starter_defends: u8) -> BranchCampaignStarterBlockFloorV1 {
    if starter_defends >= 4 {
        BranchCampaignStarterBlockFloorV1::FullStarter
    } else if starter_defends > 0 {
        BranchCampaignStarterBlockFloorV1::Partial
    } else {
        BranchCampaignStarterBlockFloorV1::None
    }
}

fn vulnerable_support_for_card_v1(card: CardId) -> BranchCampaignVulnerableSupportV1 {
    match card {
        CardId::Bash => BranchCampaignVulnerableSupportV1::StarterBash,
        CardId::Shockwave | CardId::Terror | CardId::Uppercut => {
            BranchCampaignVulnerableSupportV1::Premium
        }
        _ => BranchCampaignVulnerableSupportV1::Reliable,
    }
}

fn added_attack_quality_v1(
    added_attack_count: u8,
    solid_count: u8,
    best_damage: i32,
) -> BranchCampaignAddedAttackQualityV1 {
    if added_attack_count == 0 {
        BranchCampaignAddedAttackQualityV1::None
    } else if best_damage >= 20 || solid_count >= 3 {
        BranchCampaignAddedAttackQualityV1::BurstReady
    } else if solid_count >= 2 {
        BranchCampaignAddedAttackQualityV1::MultipleSolid
    } else if solid_count >= 1 {
        BranchCampaignAddedAttackQualityV1::SolidOne
    } else {
        BranchCampaignAddedAttackQualityV1::WeakOne
    }
}

fn added_block_quality_v1(
    added_block_count: u8,
    solid_count: u8,
    best_block: i32,
) -> BranchCampaignAddedBlockQualityV1 {
    if added_block_count == 0 {
        BranchCampaignAddedBlockQualityV1::None
    } else if best_block >= 20 {
        BranchCampaignAddedBlockQualityV1::BurstBlock
    } else if solid_count >= 2 {
        BranchCampaignAddedBlockQualityV1::MultipleSolid
    } else if solid_count >= 1 {
        BranchCampaignAddedBlockQualityV1::SolidOne
    } else {
        BranchCampaignAddedBlockQualityV1::LowOne
    }
}

fn skill_liability_v1(skill_count: u8) -> BranchCampaignSkillLiabilityV1 {
    if skill_count >= 8 {
        BranchCampaignSkillLiabilityV1::High
    } else if skill_count >= 5 {
        BranchCampaignSkillLiabilityV1::Medium
    } else if skill_count > 0 {
        BranchCampaignSkillLiabilityV1::Low
    } else {
        BranchCampaignSkillLiabilityV1::None
    }
}

fn draw_to_damage_v1(coverage: &BranchCampaignJobCoverageV1) -> BranchCampaignDrawToDamageV1 {
    if coverage.draw.draw_count == 0 {
        return BranchCampaignDrawToDamageV1::None;
    }
    if coverage.frontload.added_attack_quality >= BranchCampaignAddedAttackQualityV1::SolidOne
        || coverage.frontload.aoe_support >= BranchCampaignAoeSupportV1::Present
        || coverage.scaling.damage_count > 0
    {
        BranchCampaignDrawToDamageV1::ImprovesGoodTargets
    } else {
        BranchCampaignDrawToDamageV1::DrawsMostlyStarter
    }
}

fn has_frontload_delta_v1(coverage: &BranchCampaignJobCoverageV1) -> bool {
    coverage.frontload.added_attack_quality >= BranchCampaignAddedAttackQualityV1::SolidOne
        || coverage.frontload.aoe_support >= BranchCampaignAoeSupportV1::Present
}

fn has_boss_frontload_plan_v1(coverage: &BranchCampaignJobCoverageV1) -> bool {
    coverage.frontload.added_attack_quality >= BranchCampaignAddedAttackQualityV1::MultipleSolid
        || coverage.frontload.added_attack_quality >= BranchCampaignAddedAttackQualityV1::SolidOne
            && coverage.frontload.vulnerable_support
                >= BranchCampaignVulnerableSupportV1::StarterBash
        || coverage.frontload.aoe_support >= BranchCampaignAoeSupportV1::Strong
}

fn has_added_block_plan_v1(coverage: &BranchCampaignJobCoverageV1) -> bool {
    coverage.block.added_block_quality >= BranchCampaignAddedBlockQualityV1::SolidOne
        || coverage.block.scaling_block > 0
        || coverage.scaling.block_count > 0
}

fn has_engine_or_scaling_plan_v1(coverage: &BranchCampaignJobCoverageV1) -> bool {
    coverage.scaling.damage_count > 0
        || coverage.exhaust.enabler_count > 0 && coverage.exhaust.payoff_count > 0
        || coverage.status.payoff_count > 0
}

fn campaign_formation_stage_from_session_v1(
    stage: StrategyDeckFormationStageV1,
    needs: &[StrategyDeckFormationNeedV1],
    coverage: &BranchCampaignJobCoverageV1,
    block_readiness: &BlockPlanReadinessV1,
) -> BranchCampaignFormationStageV1 {
    if coverage.exhaust.enabler_count > 0 && coverage.exhaust.payoff_count > 0 {
        return BranchCampaignFormationStageV1::EngineOnline;
    }
    if coverage.status.enabler_count > 0 && coverage.status.payoff_count > 0 {
        return BranchCampaignFormationStageV1::EngineOnline;
    }
    if coverage.scaling.damage_count > 0
        && (coverage.draw.draw_count > 0
            || coverage.draw.energy_count > 0
            || coverage.block.added_block_quality
                >= BranchCampaignAddedBlockQualityV1::MultipleSolid)
    {
        return BranchCampaignFormationStageV1::ScalingOnline;
    }
    if *block_readiness >= BlockPlanReadinessV1::Supported {
        return BranchCampaignFormationStageV1::PackageForming;
    }
    if coverage.scaling.damage_count > 0
        || coverage.exhaust.enabler_count > 0
        || coverage.exhaust.payoff_count > 0
        || coverage.status.payoff_count > 0
    {
        return BranchCampaignFormationStageV1::DirectionSeeded;
    }
    match stage {
        StrategyDeckFormationStageV1::StarterShell => {
            BranchCampaignFormationStageV1::StarterSurvival
        }
        StrategyDeckFormationStageV1::Transitional => {
            BranchCampaignFormationStageV1::EarlyStableIncomplete
        }
        StrategyDeckFormationStageV1::PlanSeeded
        | StrategyDeckFormationStageV1::PlanCommitted
        | StrategyDeckFormationStageV1::Mature => {
            if needs.is_empty() {
                BranchCampaignFormationStageV1::DirectionSeeded
            } else {
                BranchCampaignFormationStageV1::EarlyStableIncomplete
            }
        }
    }
}

#[derive(Clone, Copy)]
struct BranchCampaignSessionSummaryProxyV1 {
    act: u8,
    floor: i32,
    hp: i32,
    max_hp: i32,
    gold: i32,
}

fn session_summary_proxy_v1(session: &RunControlSession) -> BranchCampaignSessionSummaryProxyV1 {
    let (hp, max_hp) = session.visible_player_hp();
    BranchCampaignSessionSummaryProxyV1 {
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        hp,
        max_hp,
        gold: session.run_state.gold,
    }
}

fn campaign_immediate_gate_v1(
    summary: &BranchCampaignSessionSummaryProxyV1,
    coverage: &BranchCampaignJobCoverageV1,
) -> BranchCampaignGateStatusV1 {
    if hp_percent_v1(summary) < 20 {
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if summary.act == 1 && summary.floor <= 5 {
        if coverage.frontload.starter_floor >= BranchCampaignStarterFrontloadFloorV1::Partial
            || coverage.frontload.added_attack_quality
                >= BranchCampaignAddedAttackQualityV1::WeakOne
        {
            return BranchCampaignGateStatusV1::Solved;
        }
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if !has_frontload_delta_v1(coverage) {
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if has_added_block_plan_v1(coverage)
        || coverage.frontload.draw_to_damage >= BranchCampaignDrawToDamageV1::ImprovesGoodTargets
        || hp_percent_v1(summary) >= 55
    {
        BranchCampaignGateStatusV1::Passable
    } else {
        BranchCampaignGateStatusV1::AtRisk
    }
}

fn campaign_act_boss_gate_v1(
    summary: &BranchCampaignSessionSummaryProxyV1,
    coverage: &BranchCampaignJobCoverageV1,
    block_readiness: &BlockPlanReadinessV1,
) -> BranchCampaignGateStatusV1 {
    if summary.act == 1 && summary.floor < 7 {
        return BranchCampaignGateStatusV1::Unknown;
    }
    if has_engine_or_scaling_plan_v1(coverage)
        || *block_readiness >= BlockPlanReadinessV1::Supported
    {
        return BranchCampaignGateStatusV1::Passable;
    }
    if has_boss_frontload_plan_v1(coverage) {
        return BranchCampaignGateStatusV1::AtRisk;
    }
    BranchCampaignGateStatusV1::Unsolved
}

fn campaign_transition_gate_v1(
    summary: &BranchCampaignSessionSummaryProxyV1,
    coverage: &BranchCampaignJobCoverageV1,
) -> BranchCampaignGateStatusV1 {
    if summary.act >= 2 {
        if has_engine_or_scaling_plan_v1(coverage)
            || coverage.frontload.aoe_support >= BranchCampaignAoeSupportV1::Present
                && has_added_block_plan_v1(coverage)
        {
            return BranchCampaignGateStatusV1::Passable;
        }
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if summary.floor < 7 {
        return BranchCampaignGateStatusV1::Unknown;
    }
    if has_engine_or_scaling_plan_v1(coverage) {
        BranchCampaignGateStatusV1::Passable
    } else if coverage.frontload.draw_to_damage >= BranchCampaignDrawToDamageV1::ImprovesGoodTargets
        && has_added_block_plan_v1(coverage)
    {
        BranchCampaignGateStatusV1::AtRisk
    } else {
        BranchCampaignGateStatusV1::Unsolved
    }
}

fn campaign_resource_conversion_v1(
    summary: &BranchCampaignSessionSummaryProxyV1,
    session: &RunControlSession,
) -> BranchCampaignResourceConversionV1 {
    let frontier = format!("{:?}", session.engine_state).to_ascii_lowercase();
    if frontier.contains("shop") && summary.gold >= 150 {
        return BranchCampaignResourceConversionV1::StrongConversionWindow;
    }
    if frontier.contains("shop") && summary.gold >= 75 {
        return BranchCampaignResourceConversionV1::PendingConversion;
    }
    if frontier.contains("campfire") && hp_percent_v1(summary) >= 55 {
        return BranchCampaignResourceConversionV1::PendingConversion;
    }
    if hp_percent_v1(summary) >= 60 {
        BranchCampaignResourceConversionV1::BufferOnly
    } else {
        BranchCampaignResourceConversionV1::None
    }
}

fn campaign_missing_critical_jobs_v1(
    summary: &BranchCampaignSessionSummaryProxyV1,
    coverage: &BranchCampaignJobCoverageV1,
    formation_needs: &[StrategyDeckFormationNeedV1],
) -> Vec<String> {
    let mut missing = Vec::new();
    if coverage.frontload.added_attack_quality <= BranchCampaignAddedAttackQualityV1::WeakOne
        && coverage.frontload.aoe_support == BranchCampaignAoeSupportV1::None
    {
        missing.push("frontload".to_string());
    }
    if summary.act >= 2 || summary.floor >= 7 {
        if !has_engine_or_scaling_plan_v1(coverage) {
            missing.push("scaling_or_engine".to_string());
        }
    }
    if summary.act >= 2 && !has_added_block_plan_v1(coverage) {
        missing.push("act2_mitigation".to_string());
    }
    if formation_needs.contains(&StrategyDeckFormationNeedV1::DrawEnergy)
        && coverage.draw.draw_count == 0
        && coverage.draw.energy_count == 0
    {
        missing.push("draw_energy".to_string());
    }
    missing
}

fn hp_percent_v1(summary: &BranchCampaignSessionSummaryProxyV1) -> i32 {
    if summary.max_hp <= 0 {
        return 0;
    }
    summary.hp.max(0).saturating_mul(100) / summary.max_hp
}

fn immediate_gate_viability_v1(status: BranchCampaignGateStatusV1) -> i32 {
    match status {
        BranchCampaignGateStatusV1::Solved | BranchCampaignGateStatusV1::Passable => 2,
        BranchCampaignGateStatusV1::Unknown => 1,
        BranchCampaignGateStatusV1::AtRisk => 0,
        BranchCampaignGateStatusV1::Unsolved => -1,
    }
}
