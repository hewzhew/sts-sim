use crate::bot::card_disposition::{
    combat_exhaust_score_for_uuid, combat_retention_score_for_uuid,
};
use crate::bot::monster_belief::build_combat_belief_state;
use crate::bot::strategy_families::{
    apotheosis_hand_shaping_score, apparition_hand_shaping_score, exhaust_fuel_value_score,
    exhaust_future_fuel_reserve_score, hand_shaping_delay_quality_score,
    hand_shaping_next_draw_window_score, hand_shaping_play_now_score, reaper_hand_shaping_score,
    ApparitionTimingContext, SurvivalTimingContext,
};
use crate::combat::CombatState;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::relics::RelicId;

pub(super) fn score_put_on_draw_pile_candidate(combat: &CombatState, uuid: u32) -> i32 {
    let Some(card) = combat.zones.hand.iter().find(|c| c.uuid == uuid) else {
        return i32::MIN / 4;
    };
    let def = get_card_definition(card.id);
    let mut score = 0;

    score += topdeck_badness_for_card(combat, card, &def);
    score += keep_in_hand_urgency(combat, card, &def);
    score
}

pub(super) fn score_exhaust_candidate(combat: &CombatState, uuid: u32) -> i32 {
    let Some(card) = combat.zones.hand.iter().find(|c| c.uuid == uuid) else {
        return i32::MIN / 4;
    };
    let def = get_card_definition(card.id);
    let incoming = total_incoming_damage(combat);
    let safe_block_turn = incoming <= combat.entities.player.block;
    let unblocked_incoming = (incoming - combat.entities.player.block).max(0);
    let missing_hp = (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0);
    let timing_hold_score =
        card_specific_timing_hold_score(combat, card, unblocked_incoming, incoming, missing_hp);
    let can_play_now = crate::content::cards::can_play_card(card, combat).is_ok();
    let mut score = exhaust_fuel_value_score(
        card.id,
        def.card_type,
        def.cost as i32,
        combat.turn.energy as i32,
        safe_block_turn,
        can_play_now,
        timing_hold_score,
        combat.get_power(0, crate::combat::PowerId::FeelNoPain),
        combat.get_power(0, crate::combat::PowerId::DarkEmbrace) > 0,
    );

    score += exhaust_future_fuel_reserve_score(
        remaining_low_value_fuel_after_exhaust(combat, uuid),
        future_exhaust_demand(combat),
    );
    score += combat_exhaust_score_for_uuid(combat, uuid);
    score
}

pub(super) fn score_discard_candidate(combat: &CombatState, uuid: u32) -> i32 {
    let Some(card) = combat.zones.hand.iter().find(|c| c.uuid == uuid) else {
        return i32::MIN / 4;
    };
    let def = get_card_definition(card.id);
    let incoming = total_incoming_damage(combat);
    let unblocked_incoming = (incoming - combat.entities.player.block).max(0);
    let safe_block_turn = incoming <= combat.entities.player.block;
    let can_play_now = crate::content::cards::can_play_card(card, combat).is_ok();
    let missing_hp = (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0);
    let timing_hold_score =
        card_specific_timing_hold_score(combat, card, unblocked_incoming, incoming, missing_hp);
    let duplicate_count = combat
        .zones
        .hand
        .iter()
        .filter(|other| other.id == card.id && other.uuid != uuid)
        .count() as i32;
    let playable_block =
        (def.base_block as i32 + combat.get_power(0, crate::combat::PowerId::Dexterity)).max(0);

    let mut score = hand_shaping_delay_quality_score(
        card.id,
        def.card_type,
        def.cost as i32,
        combat.turn.energy as i32,
        safe_block_turn,
    );
    score -= hand_shaping_play_now_score(can_play_now);
    score -= timing_hold_score * 2;

    match def.card_type {
        CardType::Curse => score += 60_000,
        CardType::Status => {
            score += match card.id {
                CardId::Dazed => 26_000,
                CardId::Slimed => 34_000,
                CardId::Burn => 38_000,
                _ => 28_000,
            };
        }
        CardType::Power => {
            if can_play_now {
                score -= 6_000;
            } else {
                score += 2_000;
            }
        }
        _ => {}
    }

    if !can_play_now {
        score += 2_200;
    }
    if def.cost >= 0 && def.cost as u8 > combat.turn.energy {
        score += 1_400;
    }
    if safe_block_turn && matches!(card.id, CardId::Strike | CardId::Defend | CardId::DefendG) {
        score += 1_500 + duplicate_count * 400;
    }
    if duplicate_count > 0 && matches!(def.card_type, CardType::Attack | CardType::Skill) {
        score += duplicate_count * 500;
    }
    if unblocked_incoming > 0 && playable_block > 0 && can_play_now {
        score -= unblocked_incoming.min(playable_block) * 220 + 1_200;
    }

    match card.id {
        CardId::Apotheosis => score -= 12_000,
        CardId::Apparition => {
            if unblocked_incoming > 0 || combat.entities.player.current_hp <= 35 {
                score -= 20_000;
            } else {
                score -= 5_000;
            }
        }
        CardId::Reaper => {
            if unblocked_incoming > 0 || missing_hp >= 10 {
                score -= 10_000;
            }
        }
        CardId::Corruption
        | CardId::FeelNoPain
        | CardId::DarkEmbrace
        | CardId::Barricade
        | CardId::DemonForm => {
            if can_play_now {
                score -= 8_000;
            }
        }
        CardId::GoodInstincts | CardId::Finesse | CardId::FlashOfSteel | CardId::ShrugItOff => {
            if can_play_now {
                score -= 1_800;
            }
        }
        _ => {}
    }

    score += combat_exhaust_score_for_uuid(combat, uuid) / 2;
    score -= combat_retention_score_for_uuid(combat, uuid) / 2;

    score
}

pub(super) fn score_discard_to_hand_candidate(combat: &CombatState, uuid: u32) -> i32 {
    let Some(card) = combat.zones.discard_pile.iter().find(|c| c.uuid == uuid) else {
        return i32::MIN / 4;
    };
    let def = get_card_definition(card.id);
    let incoming = total_incoming_damage(combat);
    let unblocked_incoming = (incoming - combat.entities.player.block).max(0);
    let missing_hp = (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0);
    let safe_block_turn = incoming <= combat.entities.player.block;
    let cost = card.get_cost() as i32;
    let can_play_now = cost <= combat.turn.energy as i32 || cost == 0 || cost == -1;

    let mut score = 0;
    score -= hand_shaping_delay_quality_score(
        card.id,
        def.card_type,
        cost,
        combat.turn.energy as i32,
        safe_block_turn,
    );

    match card.id {
        CardId::Offering => score += 42_000,
        CardId::Apparition => {
            if unblocked_incoming > 0 || combat.entities.player.current_hp <= 30 {
                score += 34_000;
            } else {
                score += 18_000;
            }
        }
        CardId::Impervious => score += 28_000 + unblocked_incoming.min(25) * 400,
        CardId::Reaper => score += 24_000 + missing_hp.min(20) * 250,
        CardId::SearingBlow => score += 18_000 + card.upgrades as i32 * 3_000,
        CardId::FlameBarrier | CardId::GhostlyArmor | CardId::PowerThrough => {
            score += 18_000 + unblocked_incoming.min(20) * 320
        }
        CardId::Disarm | CardId::Shockwave | CardId::Uppercut => {
            score += 16_000 + incoming.min(20) * 180
        }
        CardId::ShrugItOff | CardId::BattleTrance | CardId::BurningPact => score += 12_000,
        _ => {
            if def.card_type == CardType::Power {
                score += 14_000;
            }
            if def.base_damage >= 12 {
                score += 7_000;
            }
            if def.base_block >= 10 {
                score += 8_000 + unblocked_incoming.min(def.base_block as i32) * 120;
            }
        }
    }

    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        score -= 20_000;
    }
    if can_play_now {
        score += 2_000;
    }
    if cost > combat.turn.energy as i32 && cost > 0 {
        score -= 2_200;
    }
    score
}

fn total_incoming_damage(combat: &CombatState) -> i32 {
    build_combat_belief_state(combat)
        .expected_incoming_damage
        .round() as i32
}

fn topdeck_badness_for_card(
    combat: &CombatState,
    card: &crate::combat::CombatCard,
    def: &crate::content::cards::CardDefinition,
) -> i32 {
    let incoming = total_incoming_damage(combat);
    let safe_block_turn = incoming <= combat.entities.player.block;
    let mut score = hand_shaping_next_draw_window_score(5, true)
        + hand_shaping_delay_quality_score(
            card.id,
            def.card_type,
            def.cost as i32,
            combat.turn.energy as i32,
            safe_block_turn,
        );

    if crate::content::cards::is_ethereal(card) {
        score += 3_000;
    }

    score
}

fn keep_in_hand_urgency(
    combat: &CombatState,
    card: &crate::combat::CombatCard,
    def: &crate::content::cards::CardDefinition,
) -> i32 {
    let incoming = total_incoming_damage(combat);
    let can_play_now = crate::content::cards::can_play_card(card, combat).is_ok();
    let missing_hp = (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0);
    let unblocked_incoming = (incoming - combat.entities.player.block).max(0);
    let mut score = 0;

    score += hand_shaping_play_now_score(can_play_now);
    score +=
        card_specific_timing_hold_score(combat, card, unblocked_incoming, incoming, missing_hp);

    if def.card_type == CardType::Power {
        score -= 1_800;
    }

    score
}

fn card_specific_timing_hold_score(
    combat: &CombatState,
    card: &crate::combat::CombatCard,
    unblocked_incoming: i32,
    incoming: i32,
    missing_hp: i32,
) -> i32 {
    match card.id {
        CardId::Defend | CardId::DefendG => {
            let junk_fuel_count = combat
                .zones
                .hand
                .iter()
                .filter(|other| {
                    other.uuid != card.uuid
                        && matches!(
                            get_card_definition(other.id).card_type,
                            CardType::Curse | CardType::Status
                        )
                })
                .count() as i32;
            if unblocked_incoming > 0 {
                let mut value = 4_200 + unblocked_incoming.min(20) * 260;
                if junk_fuel_count == 0 {
                    value += 1_800;
                } else {
                    value -= junk_fuel_count.min(2) * 450;
                }
                value
            } else {
                0
            }
        }
        CardId::Inflame => {
            if crate::content::cards::can_play_card(card, combat).is_ok()
                && combat.get_power(0, crate::combat::PowerId::Strength) <= 3
            {
                4_500
            } else {
                0
            }
        }
        CardId::Apotheosis => {
            let upgrade_targets = combat
                .zones
                .hand
                .iter()
                .filter(|c| c.uuid != card.uuid)
                .filter(|c| {
                    c.upgrades == 0
                        && !matches!(
                            get_card_definition(c.id).card_type,
                            CardType::Curse | CardType::Status
                        )
                })
                .count() as i32;
            apotheosis_hand_shaping_score(upgrade_targets, unblocked_incoming)
        }
        CardId::Reaper => reaper_hand_shaping_score(&SurvivalTimingContext {
            current_hp: combat.entities.player.current_hp,
            imminent_unblocked_damage: unblocked_incoming,
            missing_hp,
        }),
        CardId::Apparition => apparition_hand_shaping_score(&ApparitionTimingContext {
            current_hp: combat.entities.player.current_hp,
            current_intangible: combat
                .get_power(0, crate::combat::PowerId::Intangible)
                .max(combat.get_power(0, crate::combat::PowerId::IntangiblePlayer)),
            imminent_unblocked_damage: unblocked_incoming,
            total_incoming_damage: incoming,
            apparitions_in_hand: combat
                .zones
                .hand
                .iter()
                .filter(|c| c.id == CardId::Apparition)
                .count() as i32,
            remaining_apparitions_total: combat
                .zones
                .hand
                .iter()
                .chain(combat.zones.draw_pile.iter())
                .chain(combat.zones.discard_pile.iter())
                .filter(|c| c.id == CardId::Apparition)
                .count() as i32,
            upgraded: card.upgrades > 0,
            has_runic_pyramid: combat.entities.player.has_relic(RelicId::RunicPyramid),
            encounter_pressure: combat
                .entities
                .monsters
                .iter()
                .filter(|m| !m.is_dying && !m.is_escaped && m.current_hp > 0)
                .map(|m| {
                    combat
                        .get_power(m.id, crate::combat::PowerId::Strength)
                        .max(0)
                        * 2
                        + 2
                })
                .sum::<i32>()
                + if combat.meta.is_boss_fight {
                    6
                } else if combat.meta.is_elite_fight {
                    3
                } else {
                    0
                },
        }),
        _ => 0,
    }
}

fn future_exhaust_demand(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .filter(|card| {
            matches!(
                card.id,
                CardId::SecondWind
                    | CardId::SeverSoul
                    | CardId::FiendFire
                    | CardId::BurningPact
                    | CardId::TrueGrit
            )
        })
        .count() as i32
        - 1
}

fn remaining_low_value_fuel_after_exhaust(combat: &CombatState, exhausted_uuid: u32) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .filter(|card| card.uuid != exhausted_uuid)
        .filter(|card| {
            let def = get_card_definition(card.id);
            let incoming = total_incoming_damage(combat);
            let safe_block_turn = incoming <= combat.entities.player.block;
            let can_play_now = crate::content::cards::can_play_card(card, combat).is_ok();
            let timing_hold_score = card_specific_timing_hold_score(
                combat,
                card,
                (incoming - combat.entities.player.block).max(0),
                incoming,
                (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0),
            );
            exhaust_fuel_value_score(
                card.id,
                def.card_type,
                def.cost as i32,
                combat.turn.energy as i32,
                safe_block_turn,
                can_play_now,
                timing_hold_score,
                combat.get_power(0, crate::combat::PowerId::FeelNoPain),
                combat.get_power(0, crate::combat::PowerId::DarkEmbrace) > 0,
            ) + combat_exhaust_score_for_uuid(combat, card.uuid)
                > 0
        })
        .count() as i32
}
