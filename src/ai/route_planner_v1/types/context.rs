use serde::{Deserialize, Serialize};

use crate::content::cards::{CardId, CardType};
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

use super::features::MapRouteTargetV1;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteDecisionContextV1 {
    pub act: u8,
    pub floor: i32,
    pub ascension: u8,
    pub class: String,
    pub boss: Option<String>,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck: DeckRouteSummaryV1,
    pub relics: RouteRelicSummaryV1,
    pub potions: PotionRouteSummaryV1,
    pub current_x: i32,
    pub current_y: i32,
    pub legal_next_nodes: Vec<MapRouteTargetV1>,
    pub counters: RouteCountersV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeckRouteSummaryV1 {
    pub deck_size: usize,
    pub starter_strikes: u8,
    pub starter_defends: u8,
    pub curses: u8,
    pub attacks: u8,
    pub skills: u8,
    pub powers: u8,
    pub frontload_damage_score: i32,
    pub block_score: i32,
    pub aoe_score: i32,
    pub scaling_score: i32,
    #[serde(default)]
    pub debuff_score: i32,
    pub draw_score: i32,
    pub energy_score: i32,
    pub key_upgrades_available: u8,
    pub important_cards_unupgraded: u8,
}

impl DeckRouteSummaryV1 {
    pub(crate) fn observes_card(&mut self, card_id: CardId, card_type: CardType, upgrades: u8) {
        match card_type {
            CardType::Attack => self.attacks += 1,
            CardType::Skill => self.skills += 1,
            CardType::Power => self.powers += 1,
            CardType::Curse => self.curses += 1,
            CardType::Status => {}
        }
        match card_id {
            CardId::Strike | CardId::StrikeG | CardId::StrikeB | CardId::StrikeP => {
                self.starter_strikes += 1
            }
            CardId::Defend | CardId::DefendG | CardId::DefendB | CardId::DefendP => {
                self.starter_defends += 1
            }
            _ => {}
        }
        if upgrades == 0
            && matches!(
                card_id,
                CardId::Bash | CardId::Neutralize | CardId::Eruption
            )
        {
            self.important_cards_unupgraded += 1;
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RouteRelicSummaryV1 {
    pub relic_count: usize,
    pub relics: Vec<RelicId>,
    pub wing_boots_charges: u8,
    pub has_juzu_bracelet: bool,
    pub has_tiny_chest: bool,
    pub has_preserved_insect: bool,
    pub has_peace_pipe: bool,
    pub has_shovel: bool,
    pub has_girya: bool,
    pub has_smiling_mask: bool,
    pub has_membership_card: bool,
    pub has_courier: bool,
    #[serde(default)]
    pub has_cursed_key: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PotionRouteSummaryV1 {
    pub slots: usize,
    pub filled: usize,
    pub potions: Vec<PotionId>,
    pub has_elite_potion_signal: bool,
    pub has_defensive_potion_signal: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RouteCountersV1 {
    pub unknown_belief: UnknownRoomBeliefV1,
    pub wing_boots_charges: u8,
    pub emerald_key_taken: bool,
    pub ruby_key_taken: bool,
    pub sapphire_key_taken: bool,
    pub normal_fights_remaining_scheduled: usize,
    pub elite_fights_remaining_scheduled: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct UnknownRoomBeliefV1 {
    pub monster_chance: f32,
    pub shop_chance: f32,
    pub treasure_chance: f32,
    pub event_chance: f32,
    pub elite_chance: f32,
    pub has_juzu_bracelet: bool,
    pub has_tiny_chest: bool,
    pub deadly_events: bool,
}
