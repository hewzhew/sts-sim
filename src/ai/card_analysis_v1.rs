use crate::ai::card_reward_policy_v1::{card_reward_semantic_profile_v1, CardRewardSemanticRoleV1};
use crate::content::cards::{get_card_definition, CardId, CardTag, CardType};
use crate::state::rewards::RewardCard;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAnalysisDeckSourceV1 {
    #[default]
    NonStarter = 0,
    StarterStrike = 1,
    StarterDefend = 2,
    StarterUnique = 3,
    Curse = 4,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAnalysisAttackChunkV1 {
    #[default]
    None = 0,
    Weak = 1,
    Solid = 2,
    Burst = 3,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAnalysisBlockChunkV1 {
    #[default]
    None = 0,
    Low = 1,
    Solid = 2,
    Burst = 3,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAnalysisAoeSupportV1 {
    #[default]
    None = 0,
    Present = 1,
    Strong = 2,
}

impl CardAnalysisAoeSupportV1 {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Present => "present",
            Self::Strong => "strong",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CardAnalysisVulnerableSupportV1 {
    #[default]
    None = 0,
    StarterBash = 1,
    Reliable = 2,
    Premium = 3,
}

impl CardAnalysisVulnerableSupportV1 {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::StarterBash => "bash",
            Self::Reliable => "reliable",
            Self::Premium => "premium",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardAnalysisProfileV1 {
    pub card: CardId,
    pub upgrades: u8,
    pub source: CardAnalysisDeckSourceV1,
    pub card_type: CardType,
    pub cost: i8,
    pub damage: i32,
    pub block: i32,
    pub attack_chunk: CardAnalysisAttackChunkV1,
    pub block_chunk: CardAnalysisBlockChunkV1,
    pub aoe_support: CardAnalysisAoeSupportV1,
    pub vulnerable_support: CardAnalysisVulnerableSupportV1,
    pub is_skill: bool,
    pub is_curse: bool,
    pub has_frontload_damage: bool,
    pub has_block: bool,
    pub has_draw_access: bool,
    pub has_energy_access: bool,
    pub has_damage_scaling: bool,
    pub has_block_scaling_hook: bool,
    pub has_exhaust_enabler: bool,
    pub has_exhaust_payoff: bool,
    pub has_status_enabler: bool,
    pub has_status_payoff: bool,
    pub is_non_attack: bool,
    pub is_block_plan_plain_coverage: bool,
    pub is_block_plan_medium_chunk: bool,
    pub is_block_plan_high_quality_chunk: bool,
    pub is_block_retention_source: bool,
    pub is_block_multiplier: bool,
    pub is_block_payoff: bool,
    pub is_feel_no_pain_source: bool,
    pub is_second_wind_source: bool,
    pub is_block_plan_controlled_exhaust_source: bool,
    pub is_block_plan_broad_exhaust_source: bool,
    pub is_block_plan_access_support: bool,
    pub is_stasis_sensitive_key_card: bool,
}

pub fn card_analysis_profile_v1(card: CardId, upgrades: u8) -> CardAnalysisProfileV1 {
    let definition = get_card_definition(card);
    let semantic = card_reward_semantic_profile_v1(&RewardCard::new(card, upgrades));
    let damage = card_damage_v1(card, upgrades);
    let block = card_block_v1(card, upgrades);
    let source = deck_source_v1(card);
    let has_aoe = has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::AoeDamage);
    let has_vulnerable = has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::Vulnerable);

    CardAnalysisProfileV1 {
        card,
        upgrades,
        source,
        card_type: definition.card_type,
        cost: definition.cost,
        damage,
        block,
        attack_chunk: attack_chunk_v1(damage, upgrades, definition.cost),
        block_chunk: block_chunk_v1(block),
        aoe_support: if has_aoe {
            aoe_support_v1(card, damage)
        } else {
            CardAnalysisAoeSupportV1::None
        },
        vulnerable_support: if has_vulnerable {
            vulnerable_support_v1(card)
        } else {
            CardAnalysisVulnerableSupportV1::None
        },
        is_skill: definition.card_type == CardType::Skill,
        is_curse: definition.card_type == CardType::Curse,
        has_frontload_damage: has_role_v1(
            &semantic.roles,
            CardRewardSemanticRoleV1::FrontloadDamage,
        ),
        has_block: has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::Block),
        has_draw_access: has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::CardDraw)
            || has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::CycleAccess),
        has_energy_access: has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::EnergySource),
        has_damage_scaling: has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::ScalingSource)
            || has_role_v1(
                &semantic.roles,
                CardRewardSemanticRoleV1::CombatExternalPayoff,
            ),
        has_block_scaling_hook: has_role_v1(
            &semantic.roles,
            CardRewardSemanticRoleV1::BlockRetention,
        ) || has_role_v1(
            &semantic.roles,
            CardRewardSemanticRoleV1::BlockMultiplier,
        ),
        has_exhaust_enabler: has_role_v1(
            &semantic.roles,
            CardRewardSemanticRoleV1::ExhaustGenerator,
        ),
        has_exhaust_payoff: has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::ExhaustPayoff),
        has_status_enabler: has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::StatusGenerator),
        has_status_payoff: has_role_v1(&semantic.roles, CardRewardSemanticRoleV1::StatusPayoff),
        is_non_attack: definition.card_type != CardType::Attack,
        is_block_plan_plain_coverage: is_block_plan_plain_coverage_v1(card),
        is_block_plan_medium_chunk: is_block_plan_medium_chunk_v1(card),
        is_block_plan_high_quality_chunk: is_block_plan_high_quality_chunk_v1(card),
        is_block_retention_source: matches!(card, CardId::Barricade),
        is_block_multiplier: matches!(card, CardId::Entrench),
        is_block_payoff: matches!(card, CardId::BodySlam | CardId::Juggernaut),
        is_feel_no_pain_source: matches!(card, CardId::FeelNoPain),
        is_second_wind_source: matches!(card, CardId::SecondWind),
        is_block_plan_controlled_exhaust_source: is_block_plan_controlled_exhaust_source_v1(card),
        is_block_plan_broad_exhaust_source: is_block_plan_broad_exhaust_source_v1(card),
        is_block_plan_access_support: is_block_plan_access_support_v1(card),
        is_stasis_sensitive_key_card: is_stasis_sensitive_key_card_v1(card),
    }
}

fn has_role_v1(roles: &[CardRewardSemanticRoleV1], role: CardRewardSemanticRoleV1) -> bool {
    roles.contains(&role)
}

fn deck_source_v1(card: CardId) -> CardAnalysisDeckSourceV1 {
    let definition = get_card_definition(card);
    if definition.card_type == CardType::Curse {
        return CardAnalysisDeckSourceV1::Curse;
    }
    if definition.tags.contains(&CardTag::StarterStrike) {
        return CardAnalysisDeckSourceV1::StarterStrike;
    }
    if definition.tags.contains(&CardTag::StarterDefend) {
        return CardAnalysisDeckSourceV1::StarterDefend;
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
        return CardAnalysisDeckSourceV1::StarterUnique;
    }
    CardAnalysisDeckSourceV1::NonStarter
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

fn attack_chunk_v1(damage: i32, upgrades: u8, cost: i8) -> CardAnalysisAttackChunkV1 {
    if damage <= 0 {
        CardAnalysisAttackChunkV1::None
    } else if damage >= 20 {
        CardAnalysisAttackChunkV1::Burst
    } else if damage >= 10 || upgrades > 0 || cost == -1 {
        CardAnalysisAttackChunkV1::Solid
    } else {
        CardAnalysisAttackChunkV1::Weak
    }
}

fn block_chunk_v1(block: i32) -> CardAnalysisBlockChunkV1 {
    if block <= 0 {
        CardAnalysisBlockChunkV1::None
    } else if block >= 20 {
        CardAnalysisBlockChunkV1::Burst
    } else if block >= 8 {
        CardAnalysisBlockChunkV1::Solid
    } else {
        CardAnalysisBlockChunkV1::Low
    }
}

fn aoe_support_v1(card: CardId, damage: i32) -> CardAnalysisAoeSupportV1 {
    if damage >= 18 || matches!(card, CardId::Whirlwind | CardId::Immolate) {
        CardAnalysisAoeSupportV1::Strong
    } else {
        CardAnalysisAoeSupportV1::Present
    }
}

fn vulnerable_support_v1(card: CardId) -> CardAnalysisVulnerableSupportV1 {
    match card {
        CardId::Bash => CardAnalysisVulnerableSupportV1::StarterBash,
        CardId::Shockwave | CardId::Terror | CardId::Uppercut => {
            CardAnalysisVulnerableSupportV1::Premium
        }
        _ => CardAnalysisVulnerableSupportV1::Reliable,
    }
}

fn is_block_plan_plain_coverage_v1(card: CardId) -> bool {
    matches!(
        card,
        CardId::Defend
            | CardId::Armaments
            | CardId::ShrugItOff
            | CardId::TrueGrit
            | CardId::IronWave
            | CardId::GhostlyArmor
            | CardId::FlameBarrier
            | CardId::Impervious
            | CardId::PowerThrough
    )
}

fn is_block_plan_medium_chunk_v1(card: CardId) -> bool {
    matches!(card, CardId::FlameBarrier | CardId::GhostlyArmor)
}

fn is_block_plan_high_quality_chunk_v1(card: CardId) -> bool {
    matches!(card, CardId::Impervious | CardId::PowerThrough)
}

fn is_block_plan_controlled_exhaust_source_v1(card: CardId) -> bool {
    matches!(
        card,
        CardId::BurningPact | CardId::TrueGrit | CardId::SecondWind | CardId::FiendFire
    )
}

fn is_block_plan_broad_exhaust_source_v1(card: CardId) -> bool {
    matches!(
        card,
        CardId::BurningPact
            | CardId::TrueGrit
            | CardId::SecondWind
            | CardId::FiendFire
            | CardId::SeverSoul
            | CardId::Corruption
            | CardId::Havoc
    )
}

fn is_block_plan_access_support_v1(card: CardId) -> bool {
    matches!(
        card,
        CardId::BattleTrance
            | CardId::BurningPact
            | CardId::PommelStrike
            | CardId::ShrugItOff
            | CardId::Offering
            | CardId::DeepBreath
            | CardId::Warcry
            | CardId::MasterOfStrategy
    )
}

fn is_stasis_sensitive_key_card_v1(card: CardId) -> bool {
    matches!(
        card,
        CardId::Barricade
            | CardId::Entrench
            | CardId::Impervious
            | CardId::PowerThrough
            | CardId::FeelNoPain
            | CardId::Corruption
            | CardId::Offering
            | CardId::LimitBreak
            | CardId::DemonForm
            | CardId::Shockwave
    )
}
