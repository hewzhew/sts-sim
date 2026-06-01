use crate::content::cards::{get_card_definition, CardTarget, CardType};
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::state::RunState;

use super::features::route_targets;
use super::types::{
    DeckRouteSummaryV1, PotionRouteSummaryV1, RouteCountersV1, RouteDecisionContextV1,
    RouteRelicSummaryV1, UnknownRoomBeliefV1,
};

pub fn build_route_decision_context_v1(run_state: &RunState) -> RouteDecisionContextV1 {
    let relics = build_relic_summary(run_state);
    RouteDecisionContextV1 {
        act: run_state.act_num,
        floor: run_state.floor_num,
        ascension: run_state.ascension_level,
        class: run_state.player_class.to_string(),
        boss: run_state
            .boss_key
            .map(|boss| debug_words(&format!("{boss:?}"))),
        hp: run_state.current_hp,
        max_hp: run_state.max_hp,
        gold: run_state.gold,
        deck: build_deck_summary(run_state),
        potions: build_potion_summary(run_state),
        current_x: run_state.map.current_x,
        current_y: run_state.map.current_y,
        legal_next_nodes: route_targets(run_state),
        counters: RouteCountersV1 {
            unknown_belief: build_unknown_belief(run_state, &relics),
            wing_boots_charges: relics.wing_boots_charges,
            emerald_key_taken: run_state.keys[2],
            ruby_key_taken: run_state.keys[0],
            sapphire_key_taken: run_state.keys[1],
            normal_fights_remaining_scheduled: run_state.monster_list.len(),
            elite_fights_remaining_scheduled: run_state.elite_monster_list.len(),
        },
        relics,
    }
}

fn build_deck_summary(run_state: &RunState) -> DeckRouteSummaryV1 {
    let mut summary = DeckRouteSummaryV1 {
        deck_size: run_state.master_deck.len(),
        starter_strikes: 0,
        starter_defends: 0,
        curses: 0,
        attacks: 0,
        skills: 0,
        powers: 0,
        frontload_damage_score: 0,
        block_score: 0,
        aoe_score: 0,
        scaling_score: 0,
        draw_score: 0,
        energy_score: 0,
        key_upgrades_available: 0,
        important_cards_unupgraded: 0,
    };
    for card in &run_state.master_deck {
        let def = get_card_definition(card.id);
        summary.observes_card(card.id, def.card_type, card.upgrades);
        if def.card_type == CardType::Attack {
            summary.frontload_damage_score +=
                def.base_damage + def.upgrade_damage * i32::from(card.upgrades);
        }
        if def.base_block > 0 {
            summary.block_score += def.base_block + def.upgrade_block * i32::from(card.upgrades);
        }
        if def.target == CardTarget::AllEnemy || def.is_multi_damage {
            summary.aoe_score += 1;
        }
        if def.card_type == CardType::Power
            || matches!(
                card.id,
                crate::content::cards::CardId::DemonForm
                    | crate::content::cards::CardId::Inflame
                    | crate::content::cards::CardId::Metallicize
                    | crate::content::cards::CardId::LimitBreak
            )
        {
            summary.scaling_score += 1;
        }
        if matches!(
            card.id,
            crate::content::cards::CardId::PommelStrike
                | crate::content::cards::CardId::ShrugItOff
                | crate::content::cards::CardId::BattleTrance
                | crate::content::cards::CardId::Offering
        ) {
            summary.draw_score += 1;
        }
        if matches!(
            card.id,
            crate::content::cards::CardId::SeeingRed
                | crate::content::cards::CardId::Bloodletting
                | crate::content::cards::CardId::Offering
                | crate::content::cards::CardId::Berserk
        ) {
            summary.energy_score += 1;
        }
        if card.upgrades == 0
            && def.card_type != CardType::Curse
            && def.card_type != CardType::Status
        {
            summary.key_upgrades_available = summary.key_upgrades_available.saturating_add(1);
        }
    }
    summary
}

fn build_relic_summary(run_state: &RunState) -> RouteRelicSummaryV1 {
    let relics = run_state
        .relics
        .iter()
        .map(|relic| relic.id)
        .collect::<Vec<_>>();
    let has_juzu_bracelet = has_relic(&relics, RelicId::JuzuBracelet);
    let has_tiny_chest = has_relic(&relics, RelicId::TinyChest);
    let has_preserved_insect = has_relic(&relics, RelicId::PreservedInsect);
    let has_peace_pipe = has_relic(&relics, RelicId::PeacePipe);
    let has_shovel = has_relic(&relics, RelicId::Shovel);
    let has_girya = has_relic(&relics, RelicId::Girya);
    let has_smiling_mask = has_relic(&relics, RelicId::SmilingMask);
    let has_membership_card = has_relic(&relics, RelicId::MembershipCard);
    let has_courier = has_relic(&relics, RelicId::Courier);
    let wing_boots_charges = run_state
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::WingBoots)
        .map(|relic| relic.counter.max(0) as u8)
        .unwrap_or(0);
    RouteRelicSummaryV1 {
        relic_count: run_state.relics.len(),
        relics,
        wing_boots_charges,
        has_juzu_bracelet,
        has_tiny_chest,
        has_preserved_insect,
        has_peace_pipe,
        has_shovel,
        has_girya,
        has_smiling_mask,
        has_membership_card,
        has_courier,
    }
}

fn has_relic(relics: &[RelicId], id: RelicId) -> bool {
    relics.iter().any(|&relic| relic == id)
}

fn debug_words(raw: &str) -> String {
    let mut out = String::new();
    for (idx, ch) in raw.chars().enumerate() {
        if idx > 0 && ch.is_ascii_uppercase() {
            out.push(' ');
        }
        out.push(ch);
    }
    out
}

fn build_potion_summary(run_state: &RunState) -> PotionRouteSummaryV1 {
    let potions = run_state
        .potions
        .iter()
        .filter_map(|slot| slot.as_ref().map(|potion| potion.id))
        .collect::<Vec<_>>();
    let has_elite_potion_signal = potions.iter().any(|id| {
        matches!(
            id,
            PotionId::FirePotion
                | PotionId::ExplosivePotion
                | PotionId::AttackPotion
                | PotionId::StrengthPotion
                | PotionId::SteroidPotion
                | PotionId::DuplicationPotion
                | PotionId::LiquidMemories
                | PotionId::EntropicBrew
        )
    });
    let has_defensive_potion_signal = potions.iter().any(|id| {
        matches!(
            id,
            PotionId::BlockPotion
                | PotionId::DexterityPotion
                | PotionId::SpeedPotion
                | PotionId::EssenceOfSteel
                | PotionId::LiquidBronze
                | PotionId::RegenPotion
                | PotionId::FairyPotion
                | PotionId::FruitJuice
                | PotionId::BloodPotion
        )
    });
    PotionRouteSummaryV1 {
        slots: run_state.potions.len(),
        filled: potions.len(),
        potions,
        has_elite_potion_signal,
        has_defensive_potion_signal,
    }
}

fn build_unknown_belief(run_state: &RunState, relics: &RouteRelicSummaryV1) -> UnknownRoomBeliefV1 {
    let monster_chance = if relics.has_juzu_bracelet {
        0.0
    } else {
        run_state.event_generator.monster_chance
    };
    let shop_chance = run_state.event_generator.shop_chance;
    let treasure_chance = if relics.has_tiny_chest {
        run_state.event_generator.treasure_chance.max(0.02)
    } else {
        run_state.event_generator.treasure_chance
    };
    let used = monster_chance + shop_chance + treasure_chance;
    UnknownRoomBeliefV1 {
        monster_chance,
        shop_chance,
        treasure_chance,
        event_chance: (1.0 - used).clamp(0.0, 1.0),
        elite_chance: 0.0,
        has_juzu_bracelet: relics.has_juzu_bracelet,
        has_tiny_chest: relics.has_tiny_chest,
        deadly_events: false,
    }
}
