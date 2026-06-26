use crate::ai::block_plan_profile_v1::{block_plan_profile_v1, BlockPlanReadinessV1};
use crate::ai::card_reward_policy_v1::{card_reward_semantic_profile_v1, CardRewardSemanticRoleV1};
use crate::ai::deck_startup_profile_v1::deck_startup_profile_v1;
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyDeckFormationNeedV1,
    StrategyDeckFormationStageV1,
};
use crate::content::cards::{get_card_definition, CardId, CardTag};
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
    pub frontload_damage: u8,
    pub real_frontload_damage: u8,
    pub starter_frontload_damage: u8,
    pub aoe_damage: u8,
    pub block: u8,
    pub real_block: u8,
    pub starter_block: u8,
    pub starter_utility: u8,
    pub draw: u8,
    pub energy: u8,
    pub scaling_damage: u8,
    pub scaling_block: u8,
    pub exhaust_enabler: u8,
    pub exhaust_payoff: u8,
    pub status_enabler: u8,
    pub status_payoff: u8,
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
        "assess=[src={} stage={} gate={} boss={} trans={} resource={} jobs=f{}/{}+{} a{} b{}/{}+{} u{} d{} e{} s{} x{}/{} missing={}]",
        assessment.source.label(),
        assessment.formation_stage.label(),
        assessment.immediate_gate.label(),
        assessment.act_boss_gate.label(),
        assessment.transition_gate.label(),
        assessment.resource_conversion.label(),
        assessment.job_coverage.frontload_damage,
        assessment.job_coverage.real_frontload_damage,
        assessment.job_coverage.starter_frontload_damage,
        assessment.job_coverage.aoe_damage,
        assessment.job_coverage.block,
        assessment.job_coverage.real_block,
        assessment.job_coverage.starter_block,
        assessment.job_coverage.starter_utility,
        assessment.job_coverage.draw,
        assessment.job_coverage.energy,
        assessment.job_coverage.scaling_damage,
        assessment.job_coverage.exhaust_enabler,
        assessment.job_coverage.exhaust_payoff,
        missing
    )
}

fn campaign_job_coverage_from_run_state_v1(
    run_state: &RunState,
    startup: &crate::ai::deck_startup_profile_v1::DeckStartupProfileV1,
    block_plan: &crate::ai::block_plan_profile_v1::BlockPlanProfileV1,
) -> BranchCampaignJobCoverageV1 {
    let mut coverage = BranchCampaignJobCoverageV1::default();
    for card in &run_state.master_deck {
        let starter_role = starter_card_role_v1(card.id);
        let profile = card_reward_semantic_profile_v1(&RewardCard::new(card.id, card.upgrades));
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::FrontloadDamage) {
            coverage.frontload_damage = coverage.frontload_damage.saturating_add(1);
            if starter_role == BranchCampaignStarterCardRoleV1::StarterStrike {
                coverage.starter_frontload_damage =
                    coverage.starter_frontload_damage.saturating_add(1);
            } else if starter_role == BranchCampaignStarterCardRoleV1::StarterUtility {
                coverage.starter_utility = coverage.starter_utility.saturating_add(1);
            } else {
                coverage.real_frontload_damage = coverage.real_frontload_damage.saturating_add(1);
            }
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::AoeDamage) {
            coverage.aoe_damage = coverage.aoe_damage.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::Block) {
            coverage.block = coverage.block.saturating_add(1);
            if starter_role == BranchCampaignStarterCardRoleV1::StarterDefend {
                coverage.starter_block = coverage.starter_block.saturating_add(1);
            } else {
                coverage.real_block = coverage.real_block.saturating_add(1);
            }
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::CardDraw)
            || has_role_v1(&profile.roles, CardRewardSemanticRoleV1::CycleAccess)
        {
            coverage.draw = coverage.draw.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::EnergySource) {
            coverage.energy = coverage.energy.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::ScalingSource)
            || has_role_v1(
                &profile.roles,
                CardRewardSemanticRoleV1::CombatExternalPayoff,
            )
        {
            coverage.scaling_damage = coverage.scaling_damage.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::BlockRetention)
            || has_role_v1(&profile.roles, CardRewardSemanticRoleV1::BlockMultiplier)
            || block_plan.readiness >= BlockPlanReadinessV1::Supported
        {
            coverage.scaling_block = coverage.scaling_block.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::ExhaustGenerator) {
            coverage.exhaust_enabler = coverage.exhaust_enabler.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::ExhaustPayoff) {
            coverage.exhaust_payoff = coverage.exhaust_payoff.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::StatusGenerator) {
            coverage.status_enabler = coverage.status_enabler.saturating_add(1);
        }
        if has_role_v1(&profile.roles, CardRewardSemanticRoleV1::StatusPayoff) {
            coverage.status_payoff = coverage.status_payoff.saturating_add(1);
        }
    }
    coverage.exhaust_enabler = coverage
        .exhaust_enabler
        .saturating_add(startup.exhaust_engine_count);
    coverage.exhaust_payoff = coverage
        .exhaust_payoff
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
    StarterUtility,
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
        return BranchCampaignStarterCardRoleV1::StarterUtility;
    }
    BranchCampaignStarterCardRoleV1::NonStarter
}

fn campaign_formation_stage_from_session_v1(
    stage: StrategyDeckFormationStageV1,
    needs: &[StrategyDeckFormationNeedV1],
    coverage: &BranchCampaignJobCoverageV1,
    block_readiness: &BlockPlanReadinessV1,
) -> BranchCampaignFormationStageV1 {
    if coverage.exhaust_enabler > 0 && coverage.exhaust_payoff > 0 {
        return BranchCampaignFormationStageV1::EngineOnline;
    }
    if coverage.status_enabler > 0 && coverage.status_payoff > 0 {
        return BranchCampaignFormationStageV1::EngineOnline;
    }
    if coverage.scaling_damage > 0
        && (coverage.draw > 0 || coverage.energy > 0 || coverage.block >= 3)
    {
        return BranchCampaignFormationStageV1::ScalingOnline;
    }
    if *block_readiness >= BlockPlanReadinessV1::Supported {
        return BranchCampaignFormationStageV1::PackageForming;
    }
    if coverage.scaling_damage > 0
        || coverage.exhaust_enabler > 0
        || coverage.exhaust_payoff > 0
        || coverage.status_payoff > 0
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
        if coverage.real_frontload_damage >= 1 || coverage.starter_frontload_damage >= 4 {
            return BranchCampaignGateStatusV1::Solved;
        }
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if coverage.real_frontload_damage <= 1 && coverage.aoe_damage == 0 {
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if coverage.real_block > 0 || coverage.draw > 0 || hp_percent_v1(summary) >= 55 {
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
    if coverage.scaling_damage > 0
        || coverage.exhaust_enabler > 0 && coverage.exhaust_payoff > 0
        || *block_readiness >= BlockPlanReadinessV1::Supported
    {
        return BranchCampaignGateStatusV1::Passable;
    }
    if coverage.real_frontload_damage >= 2
        && (coverage.real_block > 0 || coverage.starter_utility > 0)
    {
        return BranchCampaignGateStatusV1::AtRisk;
    }
    BranchCampaignGateStatusV1::Unsolved
}

fn campaign_transition_gate_v1(
    summary: &BranchCampaignSessionSummaryProxyV1,
    coverage: &BranchCampaignJobCoverageV1,
) -> BranchCampaignGateStatusV1 {
    if summary.act >= 2 {
        if coverage.scaling_damage > 0
            || coverage.exhaust_enabler > 0 && coverage.exhaust_payoff > 0
            || coverage.aoe_damage > 0 && coverage.real_block > 0
        {
            return BranchCampaignGateStatusV1::Passable;
        }
        return BranchCampaignGateStatusV1::AtRisk;
    }
    if summary.floor < 7 {
        return BranchCampaignGateStatusV1::Unknown;
    }
    if coverage.scaling_damage > 0 || coverage.exhaust_enabler > 0 && coverage.exhaust_payoff > 0 {
        BranchCampaignGateStatusV1::Passable
    } else if coverage.draw > 0 && coverage.real_block > 0 {
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
    if coverage.real_frontload_damage <= 1 && coverage.aoe_damage == 0 {
        missing.push("frontload".to_string());
    }
    if summary.act >= 2 || summary.floor >= 7 {
        if coverage.scaling_damage == 0
            && !(coverage.exhaust_enabler > 0 && coverage.exhaust_payoff > 0)
            && coverage.status_payoff == 0
        {
            missing.push("scaling_or_engine".to_string());
        }
    }
    if summary.act >= 2 && coverage.real_block == 0 && coverage.scaling_block == 0 {
        missing.push("act2_mitigation".to_string());
    }
    if formation_needs.contains(&StrategyDeckFormationNeedV1::DrawEnergy)
        && coverage.draw == 0
        && coverage.energy == 0
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
