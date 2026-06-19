use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::relics::RelicId;
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BlockPlanReadinessV1 {
    #[default]
    Absent,
    PlainCoverage,
    Latent,
    Supported,
    Ready,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct BlockPlanProfileV1 {
    pub plain_block_cards: u8,
    pub medium_block_chunks: u8,
    pub high_quality_block_chunks: u8,
    pub retention_sources: u8,
    pub multipliers: u8,
    pub payoffs: u8,
    pub feel_no_pain_sources: u8,
    pub controlled_exhaust_sources: u8,
    pub broad_exhaust_sources: u8,
    pub second_wind_sources: u8,
    pub non_attack_cards: u8,
    pub setup_debt: u8,
    pub access_support: u8,
    pub stasis_sensitive_key_cards: u8,
    pub readiness: BlockPlanReadinessV1,
    pub diagnosis: Vec<String>,
}

impl BlockPlanProfileV1 {
    pub fn has_fnp_exhaust_engine(&self) -> bool {
        self.feel_no_pain_sources > 0 && self.controlled_exhaust_sources > 0
    }

    pub fn has_second_wind_block_engine(&self) -> bool {
        self.second_wind_sources > 0 && self.non_attack_cards >= 4
    }

    pub fn has_supported_retention_plan(&self) -> bool {
        self.retention_sources > 0
            && (self.multipliers > 0
                || self.high_quality_block_chunks > 0
                || self.has_fnp_exhaust_engine()
                || self.has_second_wind_block_engine())
    }

    pub fn has_execute_block_plan(&self) -> bool {
        self.high_quality_block_chunks > 0
            || self.has_fnp_exhaust_engine()
            || self.has_second_wind_block_engine()
            || self.has_supported_retention_plan()
    }

    pub fn has_dark_echo_block_plan(&self) -> bool {
        self.has_execute_block_plan()
    }

    pub fn has_hyperbeam_block_plan(&self) -> bool {
        self.high_quality_block_chunks > 0 || self.has_supported_retention_plan()
    }

    pub fn engine_support_score(&self) -> usize {
        match self.readiness {
            BlockPlanReadinessV1::Absent | BlockPlanReadinessV1::PlainCoverage => 0,
            BlockPlanReadinessV1::Latent => 1,
            BlockPlanReadinessV1::Supported => 2,
            BlockPlanReadinessV1::Ready => 3,
        }
    }
}

pub fn block_plan_profile_v1(run_state: &RunState) -> BlockPlanProfileV1 {
    let mut profile = BlockPlanProfileV1::default();

    for relic in &run_state.relics {
        if matches!(relic.id, RelicId::Calipers) {
            profile.retention_sources = profile.retention_sources.saturating_add(1);
        }
    }

    for card in &run_state.master_deck {
        let id = card.id;
        if is_plain_block_card(id) {
            profile.plain_block_cards = profile.plain_block_cards.saturating_add(1);
        }
        if is_medium_block_chunk(id) {
            profile.medium_block_chunks = profile.medium_block_chunks.saturating_add(1);
        }
        if is_high_quality_block_chunk(id) {
            profile.high_quality_block_chunks = profile.high_quality_block_chunks.saturating_add(1);
        }
        if is_retention_source(id) {
            profile.retention_sources = profile.retention_sources.saturating_add(1);
            profile.setup_debt = profile.setup_debt.saturating_add(1);
        }
        if is_multiplier(id) {
            profile.multipliers = profile.multipliers.saturating_add(1);
        }
        if is_payoff(id) {
            profile.payoffs = profile.payoffs.saturating_add(1);
        }
        if id == CardId::FeelNoPain {
            profile.feel_no_pain_sources = profile.feel_no_pain_sources.saturating_add(1);
            profile.setup_debt = profile.setup_debt.saturating_add(1);
        }
        if id == CardId::SecondWind {
            profile.second_wind_sources = profile.second_wind_sources.saturating_add(1);
        }
        if is_controlled_exhaust_source(id) {
            profile.controlled_exhaust_sources =
                profile.controlled_exhaust_sources.saturating_add(1);
        }
        if is_broad_exhaust_source(id) {
            profile.broad_exhaust_sources = profile.broad_exhaust_sources.saturating_add(1);
        }
        if get_card_definition(id).card_type != CardType::Attack {
            profile.non_attack_cards = profile.non_attack_cards.saturating_add(1);
        }
        if is_access_support(id) {
            profile.access_support = profile.access_support.saturating_add(1);
        }
        if is_stasis_sensitive_key_card(id) {
            profile.stasis_sensitive_key_cards =
                profile.stasis_sensitive_key_cards.saturating_add(1);
        }
    }

    profile.readiness = block_plan_readiness_v1(&profile);
    profile.diagnosis = block_plan_diagnosis_v1(&profile);
    profile
}

fn block_plan_readiness_v1(profile: &BlockPlanProfileV1) -> BlockPlanReadinessV1 {
    if profile.has_supported_retention_plan()
        && (profile.multipliers > 0
            || profile.has_fnp_exhaust_engine()
            || profile.has_second_wind_block_engine())
    {
        return BlockPlanReadinessV1::Ready;
    }
    if profile.has_supported_retention_plan()
        || profile.has_fnp_exhaust_engine()
        || profile.high_quality_block_chunks >= 2
    {
        return BlockPlanReadinessV1::Supported;
    }
    if profile.retention_sources > 0
        || profile.multipliers > 0
        || profile.payoffs > 0
        || profile.feel_no_pain_sources > 0
    {
        return BlockPlanReadinessV1::Latent;
    }
    if profile.plain_block_cards >= 3
        || profile.medium_block_chunks > 0
        || profile.high_quality_block_chunks > 0
    {
        return BlockPlanReadinessV1::PlainCoverage;
    }
    BlockPlanReadinessV1::Absent
}

fn block_plan_diagnosis_v1(profile: &BlockPlanProfileV1) -> Vec<String> {
    let mut diagnosis = Vec::new();
    if profile.retention_sources > 0 && !profile.has_supported_retention_plan() {
        diagnosis.push("retention_without_block_engine_support".to_string());
    }
    if profile.multipliers > 0
        && profile.retention_sources == 0
        && profile.high_quality_block_chunks == 0
    {
        diagnosis.push("multiplier_without_retained_or_large_block".to_string());
    }
    if profile.payoffs > 0 && profile.readiness < BlockPlanReadinessV1::Supported {
        diagnosis.push("block_payoff_without_supported_block_plan".to_string());
    }
    if profile.feel_no_pain_sources > 0 && profile.controlled_exhaust_sources == 0 {
        diagnosis.push("fnp_without_controlled_exhaust_access".to_string());
    }
    if profile.medium_block_chunks > 0 && profile.high_quality_block_chunks == 0 {
        diagnosis.push("medium_block_chunk_not_terminal_plan".to_string());
    }
    if profile.stasis_sensitive_key_cards > 0 {
        diagnosis.push("stasis_can_target_key_block_or_engine_cards".to_string());
    }
    diagnosis
}

fn is_plain_block_card(card: CardId) -> bool {
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

fn is_medium_block_chunk(card: CardId) -> bool {
    matches!(card, CardId::FlameBarrier | CardId::GhostlyArmor)
}

fn is_high_quality_block_chunk(card: CardId) -> bool {
    matches!(card, CardId::Impervious | CardId::PowerThrough)
}

fn is_retention_source(card: CardId) -> bool {
    matches!(card, CardId::Barricade)
}

fn is_multiplier(card: CardId) -> bool {
    matches!(card, CardId::Entrench)
}

fn is_payoff(card: CardId) -> bool {
    matches!(card, CardId::BodySlam | CardId::Juggernaut)
}

fn is_controlled_exhaust_source(card: CardId) -> bool {
    matches!(
        card,
        CardId::BurningPact | CardId::TrueGrit | CardId::SecondWind | CardId::FiendFire
    )
}

fn is_broad_exhaust_source(card: CardId) -> bool {
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

fn is_access_support(card: CardId) -> bool {
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

fn is_stasis_sensitive_key_card(card: CardId) -> bool {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn barricade_alone_is_latent_not_supported_block_plan() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::Barricade);

        let profile = block_plan_profile_v1(&run_state);

        assert_eq!(profile.readiness, BlockPlanReadinessV1::Latent);
        assert!(!profile.has_execute_block_plan());
        assert!(profile
            .diagnosis
            .contains(&"retention_without_block_engine_support".to_string()));
    }

    #[test]
    fn barricade_with_multiplier_and_big_block_is_ready() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::Barricade);
        run_state.add_card_to_deck(CardId::Entrench);
        run_state.add_card_to_deck(CardId::Impervious);

        let profile = block_plan_profile_v1(&run_state);

        assert_eq!(profile.readiness, BlockPlanReadinessV1::Ready);
        assert!(profile.has_execute_block_plan());
        assert!(profile.has_hyperbeam_block_plan());
    }

    #[test]
    fn flame_barrier_is_medium_block_not_terminal_plan() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::FlameBarrier);

        let profile = block_plan_profile_v1(&run_state);

        assert_eq!(profile.readiness, BlockPlanReadinessV1::PlainCoverage);
        assert_eq!(profile.medium_block_chunks, 1);
        assert_eq!(profile.high_quality_block_chunks, 0);
        assert!(!profile.has_hyperbeam_block_plan());
    }
}
