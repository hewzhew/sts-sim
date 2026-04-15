use super::catalog::{category_for, category_label, DONT_PLAY_POTIONS};
use super::signals::{analyze_combat, CombatSignals};
use super::targets;
use super::{PotionCandidate, PotionCategory, PotionDecisionSnapshot};
use crate::combat::{CombatState, StanceId};
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::potions::{get_potion_definition, PotionId};
use crate::engine::targeting;
use crate::state::core::ClientInput;

#[derive(Clone, Copy)]
enum DecisionMode {
    Immediate,
    Search,
}

enum GateDecision {
    Allowed,
    Forbidden,
}

pub(crate) fn choose_immediate_potion_candidate(combat: &CombatState) -> Option<PotionCandidate> {
    let signals = analyze_combat(combat);
    collect_candidates(combat, &signals, DecisionMode::Immediate)
        .into_iter()
        .find(|candidate| candidate.priority >= minimum_priority(&signals, DecisionMode::Immediate))
}

pub(crate) fn immediate_potion_snapshot(combat: &CombatState) -> PotionDecisionSnapshot {
    let signals = analyze_combat(combat);
    let minimum_priority = minimum_priority(&signals, DecisionMode::Immediate);
    let candidates = collect_candidates(combat, &signals, DecisionMode::Immediate);
    let chosen = candidates
        .iter()
        .find(|candidate| candidate.priority >= minimum_priority)
        .cloned();

    PotionDecisionSnapshot {
        minimum_priority,
        context_summary: format!(
            "turn={} hp={}/{} block={} incoming={} imminent_lethal={} alive={} elite_or_boss={} hand_junk={} expensive_unplayable={} discard_recovery={}",
            combat.turn.turn_count,
            combat.entities.player.current_hp,
            combat.entities.player.max_hp,
            combat.entities.player.block,
            signals.threat.unblocked_incoming,
            signals.threat.imminent_lethal,
            signals.offense.alive_monsters,
            signals.fight.elite_or_boss,
            signals.hand.hand_junk,
            signals.hand.expensive_unplayable_cards,
            signals.hand.discard_recovery_score
        ),
        chosen,
        candidates,
    }
}

pub fn candidate_potion_moves(combat: &CombatState) -> Vec<ClientInput> {
    let signals = analyze_combat(combat);
    collect_candidates(combat, &signals, DecisionMode::Search)
        .into_iter()
        .filter(|candidate| candidate.priority >= minimum_priority(&signals, DecisionMode::Search))
        .map(|candidate| candidate.input)
        .collect()
}

fn collect_candidates(
    combat: &CombatState,
    signals: &CombatSignals,
    mode: DecisionMode,
) -> Vec<PotionCandidate> {
    let mut candidates = combat
        .entities
        .potions
        .iter()
        .enumerate()
        .filter_map(|(potion_index, slot)| {
            let potion = slot.as_ref()?;
            if DONT_PLAY_POTIONS.contains(&potion.id) {
                return None;
            }
            evaluate_potion(combat, signals, mode, potion_index, potion.id)
        })
        .collect::<Vec<_>>();

    candidates.sort_by(|a, b| b.priority.cmp(&a.priority));
    candidates
}

fn evaluate_potion(
    combat: &CombatState,
    signals: &CombatSignals,
    mode: DecisionMode,
    potion_index: usize,
    potion_id: PotionId,
) -> Option<PotionCandidate> {
    if matches!(
        gate_potion_use(combat, signals, mode, potion_id),
        GateDecision::Forbidden
    ) {
        return None;
    }

    let def = get_potion_definition(potion_id);
    let category = category_for(potion_id);
    if matches!(mode, DecisionMode::Immediate)
        && category == PotionCategory::Setup
        && !strong_setup_release_window(combat, signals, potion_id)
    {
        return None;
    }
    let (target, target_score) =
        if let Some(validation) = targeting::validation_for_potion_target(def.target_required) {
            let targets = targeting::candidate_targets(combat, validation);
            let target = targets::best_target(combat, signals, potion_id, &targets)?;
            let score = targets::target_score(combat, signals, potion_id, target);
            (Some(target), score)
        } else {
            (None, 0)
        };

    let base = category_priority(combat, signals, category);
    let special = special_priority(combat, signals, potion_id);
    let reserve_penalty = reserve_penalty(signals, category, potion_id);
    let priority = base + special + target_score - reserve_penalty;
    if priority <= 0 {
        return None;
    }

    Some(PotionCandidate {
        potion_id,
        input: ClientInput::UsePotion {
            potion_index,
            target,
        },
        priority,
        base_priority: base,
        special_priority: special,
        target_priority: target_score,
        reason: candidate_reason(signals, category, potion_id),
        category,
    })
}

fn gate_potion_use(
    combat: &CombatState,
    signals: &CombatSignals,
    mode: DecisionMode,
    potion_id: PotionId,
) -> GateDecision {
    match potion_id {
        PotionId::DuplicationPotion => {
            if duplication_anchor_score(combat, signals) < 36 {
                GateDecision::Forbidden
            } else {
                GateDecision::Allowed
            }
        }
        PotionId::FearPotion => {
            if signals.offense.playable_attacks <= 0 || all_live_targets_have_artifact(combat) {
                GateDecision::Forbidden
            } else {
                GateDecision::Allowed
            }
        }
        PotionId::WeakenPotion => {
            if all_live_targets_have_artifact(combat) && matches!(mode, DecisionMode::Immediate) {
                GateDecision::Forbidden
            } else {
                GateDecision::Allowed
            }
        }
        PotionId::EntropicBrew => {
            if empty_potion_slots(combat) == 0 {
                GateDecision::Forbidden
            } else {
                GateDecision::Allowed
            }
        }
        PotionId::Ambrosia => {
            if combat.entities.player.stance == StanceId::Divinity {
                GateDecision::Forbidden
            } else {
                GateDecision::Allowed
            }
        }
        _ => GateDecision::Allowed,
    }
}

fn minimum_priority(signals: &CombatSignals, mode: DecisionMode) -> i32 {
    match mode {
        DecisionMode::Immediate => {
            if signals.threat.imminent_lethal {
                34
            } else if signals.fight.elite_or_boss
                && (signals.threat.low_hp || signals.threat.unblocked_incoming > 0)
            {
                52
            } else {
                62
            }
        }
        DecisionMode::Search => {
            if signals.threat.imminent_lethal {
                28
            } else if signals.fight.elite_or_boss {
                40
            } else {
                56
            }
        }
    }
}

fn strong_setup_release_window(
    combat: &CombatState,
    signals: &CombatSignals,
    potion_id: PotionId,
) -> bool {
    if !signals.fight.elite_or_boss || combat.turn.turn_count > 1 {
        return false;
    }
    if signals.threat.imminent_lethal {
        return false;
    }
    if signals.threat.low_hp && signals.threat.unblocked_incoming > 0 {
        return false;
    }

    let long_fight = signals.fight.is_boss
        || signals.offense.likely_long_fight
        || signals.offense.total_enemy_hp >= 110;
    if !long_fight {
        return false;
    }

    match potion_id {
        PotionId::DexterityPotion | PotionId::StrengthPotion | PotionId::AncientPotion => true,
        _ => signals.threat.unblocked_incoming <= signals.threat.player_hp / 3,
    }
}

fn reserve_penalty(signals: &CombatSignals, category: PotionCategory, potion_id: PotionId) -> i32 {
    if matches!(
        potion_id,
        PotionId::FruitJuice | PotionId::FairyPotion | PotionId::SmokeBomb
    ) {
        return 0;
    }
    if signals.threat.imminent_lethal
        || strong_survival_release_window(signals)
        || closeout_release_window(signals, category)
    {
        return 0;
    }

    let mut penalty = if signals.fight.is_boss {
        20
    } else if signals.fight.is_elite {
        30
    } else {
        52
    };

    match category {
        PotionCategory::Setup => {
            if signals.fight.is_boss && signals.fight.early_buff_window {
                penalty = 12;
            } else if signals.fight.is_elite && signals.fight.early_buff_window {
                penalty = 24;
            }
        }
        PotionCategory::Survival => {
            if signals.threat.low_hp && signals.threat.unblocked_incoming > 0 {
                penalty = penalty.min(10);
            }
        }
        PotionCategory::Lethal => {
            if signals.offense.alive_monsters == 1 && signals.offense.total_enemy_hp <= 40 {
                penalty = 0;
            }
        }
        PotionCategory::Recovery => {
            if signals.hand.discard_recovery_score >= 34
                && (signals.fight.elite_or_boss || signals.threat.low_hp)
            {
                penalty = penalty.min(18);
            }
        }
        PotionCategory::Escape | PotionCategory::RandomGeneration => {}
    }

    penalty
}

fn strong_survival_release_window(signals: &CombatSignals) -> bool {
    signals.threat.critical_hp && signals.threat.unblocked_incoming > 0
        || signals.threat.unblocked_incoming >= signals.threat.player_hp / 2
        || signals.threat.unblocked_incoming >= 24
}

fn closeout_release_window(signals: &CombatSignals, category: PotionCategory) -> bool {
    category == PotionCategory::Lethal
        && signals.offense.alive_monsters == 1
        && (signals.offense.total_enemy_hp <= 35 || signals.offense.fight_almost_over)
}

fn category_priority(
    _combat: &CombatState,
    signals: &CombatSignals,
    category: PotionCategory,
) -> i32 {
    match category {
        PotionCategory::Survival => {
            if signals.threat.imminent_lethal {
                88
            } else if signals.threat.unblocked_incoming > 0 {
                70
            } else if signals.threat.low_hp {
                48
            } else {
                16
            }
        }
        PotionCategory::Lethal => {
            if signals.threat.imminent_lethal {
                74
            } else if signals.offense.alive_monsters == 1 {
                64
            } else if signals.fight.elite_or_boss {
                52
            } else {
                34
            }
        }
        PotionCategory::Setup => {
            if signals.fight.elite_or_boss && signals.fight.early_buff_window {
                72
            } else if signals.offense.total_enemy_hp >= 90 && !signals.threat.imminent_lethal {
                60
            } else if signals.offense.likely_long_fight && !signals.threat.imminent_lethal {
                56
            } else {
                20
            }
        }
        PotionCategory::Recovery => {
            if signals.hand.discard_recovery_score >= 26
                || signals.hand.exhaustable_junk >= 2
                || signals.hand.expensive_unplayable_cards >= 2
            {
                66
            } else if signals.threat.unblocked_incoming > 0 {
                52
            } else {
                24
            }
        }
        PotionCategory::Escape => {
            if signals.fight.is_boss {
                -10_000
            } else if signals.threat.imminent_lethal {
                140
            } else if !signals.fight.is_elite
                && signals.threat.low_hp
                && signals.threat.unblocked_incoming > 0
            {
                110
            } else {
                -10_000
            }
        }
        PotionCategory::RandomGeneration => {
            if signals.threat.imminent_lethal {
                70
            } else if signals.fight.elite_or_boss || signals.threat.unblocked_incoming > 0 {
                56
            } else {
                22
            }
        }
    }
}

fn low_pressure_preserve_potion_window(signals: &CombatSignals) -> bool {
    !signals.fight.elite_or_boss
        && !signals.threat.imminent_lethal
        && !signals.threat.low_hp
        && signals.threat.unblocked_incoming > 0
        && signals.threat.unblocked_incoming <= 10
}

fn special_priority(combat: &CombatState, signals: &CombatSignals, potion_id: PotionId) -> i32 {
    match potion_id {
        PotionId::AncientPotion => {
            if signals.threat.player_has_artifact {
                -20
            } else if signals.hand.hand_has_flex || signals.hand.hand_has_battle_trance {
                34
            } else if signals.threat.debuffing_monsters > 0 {
                26
            } else if signals.threat.low_hp && signals.fight.elite_or_boss {
                10
            } else {
                -10
            }
        }
        PotionId::CultistPotion => {
            if signals.offense.boss_stalling_window {
                40
            } else if signals.offense.likely_long_fight
                && signals.fight.early_buff_window
                && !signals.threat.imminent_lethal
            {
                20
            } else if signals.threat.imminent_lethal || signals.offense.alive_monsters >= 3 {
                -30
            } else {
                0
            }
        }
        PotionId::DistilledChaosPotion => {
            if signals.threat.imminent_lethal {
                22
            } else if signals.threat.unblocked_incoming > 0 && signals.hand.playable_blocks == 0 {
                18
            } else if signals.fight.elite_or_boss {
                8
            } else {
                -10
            }
        }
        PotionId::LiquidMemories => {
            if signals.hand.discard_recovery_score >= 34 {
                38
            } else if signals.hand.discard_recovery_score >= 24
                && (signals.fight.elite_or_boss || signals.threat.low_hp)
            {
                22
            } else if signals.hand.discard_recovery_score >= 16 {
                8
            } else {
                -14
            }
        }
        PotionId::SmokeBomb => 0,
        PotionId::EnergyPotion => {
            if signals.hand.hand_has_x_cost && signals.hand.energy_hungry_cards > 0 {
                26
            } else if signals.hand.energy_hungry_cards >= 2 {
                18
            } else if signals.hand.expensive_unplayable_cards > 0
                && signals.threat.unblocked_incoming > 0
            {
                12
            } else {
                -10
            }
        }
        PotionId::GhostInAJar => {
            if signals.threat.max_intent_hits >= 2
                && (signals.threat.low_hp || signals.threat.unblocked_incoming > 0)
            {
                24
            } else if signals.threat.unblocked_incoming >= signals.threat.player_hp / 2 {
                14
            } else {
                0
            }
        }
        PotionId::GamblersBrew => gambler_brew_priority(combat, signals),
        PotionId::Elixir => {
            if signals.hand.exhaustable_junk >= 2 {
                26
            } else if signals.hand.exhaustable_junk >= 1
                && (signals.threat.unblocked_incoming > 0 || signals.fight.elite_or_boss)
            {
                14
            } else {
                -12
            }
        }
        PotionId::BlessingOfTheForge => {
            if signals.fight.elite_or_boss && signals.hand.hand_has_searing_blow {
                30
            } else if signals.fight.elite_or_boss && signals.hand.upgradable_cards_in_hand >= 2 {
                16
            } else if signals.hand.upgradable_cards_in_hand >= 3 {
                8
            } else {
                -12
            }
        }
        PotionId::SneckoOil => {
            if signals.hand.expensive_unplayable_cards >= 2
                || (signals.hand.energy_hungry_cards >= 2 && signals.threat.unblocked_incoming > 0)
            {
                24
            } else if signals.hand.hand_junk >= 2 {
                12
            } else {
                -12
            }
        }
        PotionId::FearPotion => {
            if signals.fight.elite_or_boss || signals.threat.nob_active {
                16
            } else {
                0
            }
        }
        PotionId::WeakenPotion => {
            if signals.threat.imminent_lethal {
                20
            } else if signals.threat.unblocked_incoming > 0 {
                8
            } else {
                0
            }
        }
        PotionId::ExplosivePotion => {
            if signals.offense.alive_monsters >= 3 {
                14
            } else if signals.offense.alive_monsters == 2 {
                8
            } else {
                -8
            }
        }
        PotionId::FirePotion => {
            if signals.offense.alive_monsters == 1 || signals.threat.imminent_lethal {
                12
            } else {
                0
            }
        }
        PotionId::PoisonPotion => {
            if signals.fight.elite_or_boss && signals.offense.alive_monsters == 1 {
                14
            } else {
                0
            }
        }
        PotionId::BlockPotion => {
            if low_pressure_preserve_potion_window(signals) {
                -60
            } else if signals.threat.imminent_lethal {
                22
            } else if signals.threat.unblocked_incoming > 0 || signals.threat.low_hp {
                12
            } else {
                0
            }
        }
        PotionId::RegenPotion => {
            if signals.offense.fight_almost_over {
                -12
            } else if signals.threat.missing_hp >= 18
                && (signals.fight.elite_or_boss || signals.offense.likely_long_fight)
            {
                20
            } else if signals.fight.potions_full && signals.threat.missing_hp >= 10 {
                10
            } else {
                -8
            }
        }
        PotionId::StancePotion => {
            if signals.threat.imminent_lethal
                || (signals.threat.unblocked_incoming > 0 && signals.threat.low_hp)
            {
                18
            } else if signals.fight.elite_or_boss && signals.offense.playable_attacks > 0 {
                8
            } else {
                -10
            }
        }
        PotionId::Ambrosia => {
            if signals.threat.imminent_lethal {
                20
            } else if signals.fight.elite_or_boss && signals.offense.playable_attacks > 0 {
                10
            } else {
                -8
            }
        }
        PotionId::EssenceOfDarkness => {
            if signals.fight.elite_or_boss && signals.offense.likely_long_fight {
                18
            } else {
                -8
            }
        }
        PotionId::BloodPotion => {
            if signals.threat.critical_hp {
                22
            } else if signals.fight.potions_full && signals.threat.missing_hp >= 10 {
                18
            } else if signals.threat.missing_hp >= 16 {
                14
            } else {
                -10
            }
        }
        PotionId::FruitJuice => {
            if signals.fight.potions_full {
                110
            } else if signals.threat.missing_hp >= 1 {
                96
            } else {
                88
            }
        }
        PotionId::DuplicationPotion => duplication_priority(combat, signals),
        PotionId::ColorlessPotion => {
            if low_pressure_preserve_potion_window(signals) {
                -40
            } else {
                0
            }
        }
        PotionId::PowerPotion
        | PotionId::AttackPotion
        | PotionId::SkillPotion
        | PotionId::SwiftPotion
        | PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::HeartOfIron
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::BottledMiracle
        | PotionId::CunningPotion
        | PotionId::PotionOfCapacity
        | PotionId::FocusPotion => 0,
        PotionId::EntropicBrew => {
            let empty_slots = empty_potion_slots(combat);
            if empty_slots >= 2 && signals.fight.elite_or_boss {
                24
            } else if empty_slots >= 1 {
                8
            } else {
                -10_000
            }
        }
        PotionId::FairyPotion => -10_000,
    }
}

fn gambler_brew_priority(combat: &CombatState, signals: &CombatSignals) -> i32 {
    let hand_liability = gambler_hand_liability(combat, signals);
    let dead_turn = gambler_dead_turn_score(combat, signals);
    let redraw_upside = gambler_redraw_upside_score(combat, signals);

    if hand_liability <= 0 && dead_turn <= 0 && redraw_upside <= 0 {
        return -12;
    }

    let mut score = hand_liability + dead_turn + redraw_upside;
    if signals.fight.elite_or_boss {
        score += 8;
    }
    if signals.threat.imminent_lethal {
        score += 10;
    }
    score
}

fn duplication_priority(combat: &CombatState, signals: &CombatSignals) -> i32 {
    duplication_anchor_score(combat, signals).max(0)
}

fn duplication_anchor_score(combat: &CombatState, signals: &CombatSignals) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .map(|card| duplication_follow_up_score(card, combat, signals))
        .max()
        .unwrap_or(-40)
}

fn duplication_follow_up_score(
    card: &crate::combat::CombatCard,
    combat: &CombatState,
    signals: &CombatSignals,
) -> i32 {
    if crate::content::cards::can_play_card(card, combat).is_err() {
        return -40;
    }

    let def = get_card_definition(card.id);
    let damage = if card.base_damage_mut > 0 {
        card.base_damage_mut
    } else {
        def.base_damage
    };
    let block = if card.base_block_mut > 0 {
        card.base_block_mut
    } else {
        def.base_block
    };

    match card.id {
        CardId::Apotheosis
        | CardId::Corruption
        | CardId::DarkEmbrace
        | CardId::FeelNoPain
        | CardId::Evolve
        | CardId::FireBreathing
        | CardId::Barricade => return -40,
        _ => {}
    }

    if def.card_type == CardType::Power && def.cost >= 2 {
        return if signals.fight.elite_or_boss { 44 } else { 34 };
    }

    if damage >= 24
        || matches!(
            card.id,
            CardId::Bludgeon
                | CardId::Immolate
                | CardId::FiendFire
                | CardId::Reaper
                | CardId::Feed
                | CardId::Impervious
        )
    {
        return 40;
    }

    if block >= 20
        || matches!(
            card.id,
            CardId::Shockwave | CardId::Uppercut | CardId::FlameBarrier
        )
    {
        return if signals.threat.unblocked_incoming > 0 {
            36
        } else {
            28
        };
    }

    -20
}

fn all_live_targets_have_artifact(combat: &CombatState) -> bool {
    let mut any_live_target = false;
    for monster in &combat.entities.monsters {
        if monster.current_hp <= 0 || monster.is_dying || monster.is_escaped || monster.half_dead {
            continue;
        }
        any_live_target = true;
        if combat.get_power(monster.id, crate::combat::PowerId::Artifact) <= 0 {
            return false;
        }
    }
    any_live_target
}

fn empty_potion_slots(combat: &CombatState) -> usize {
    combat
        .entities
        .potions
        .iter()
        .filter(|slot| slot.is_none())
        .count()
}

fn gambler_hand_liability(combat: &CombatState, signals: &CombatSignals) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .map(|card| gambler_card_liability(card, combat, signals))
        .sum()
}

fn gambler_card_liability(
    card: &crate::combat::CombatCard,
    combat: &CombatState,
    signals: &CombatSignals,
) -> i32 {
    let def = get_card_definition(card.id);
    let can_play = crate::content::cards::can_play_card(card, combat).is_ok();
    let cost = card.get_cost() as i32;
    let damage = if card.base_damage_mut > 0 {
        card.base_damage_mut
    } else {
        def.base_damage
    };
    let block = if card.base_block_mut > 0 {
        card.base_block_mut
    } else {
        def.base_block
    };

    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return 18;
    }

    if !can_play {
        return if cost >= 0 && cost > combat.turn.energy as i32 {
            12
        } else {
            4
        };
    }

    if signals.threat.unblocked_incoming > 0 {
        if block > 0 {
            return 0;
        }
        return match def.card_type {
            CardType::Power => 14,
            CardType::Attack if damage < 10 => 10,
            CardType::Skill => {
                if matches!(
                    card.id,
                    CardId::BattleTrance
                        | CardId::BurningPact
                        | CardId::Offering
                        | CardId::SeeingRed
                        | CardId::Disarm
                        | CardId::Shockwave
                        | CardId::Intimidate
                ) {
                    0
                } else {
                    8
                }
            }
            _ => 4,
        };
    }

    if cost >= 0 && cost > combat.turn.energy as i32 {
        return 6;
    }

    if matches!(card.id, CardId::Strike | CardId::Defend | CardId::DefendG) {
        return 3;
    }

    0
}

fn gambler_dead_turn_score(combat: &CombatState, signals: &CombatSignals) -> i32 {
    let playable_cards = combat
        .zones
        .hand
        .iter()
        .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
        .count() as i32;
    let current_turn_value: i32 = combat
        .zones
        .hand
        .iter()
        .map(|card| gambler_card_turn_value(card, combat, signals))
        .sum();

    if playable_cards == 0 {
        30
    } else if combat.turn.energy >= 2 && current_turn_value <= 10 {
        22
    } else if combat.turn.energy >= 1 && current_turn_value <= 18 {
        12
    } else {
        0
    }
}

fn gambler_redraw_upside_score(combat: &CombatState, signals: &CombatSignals) -> i32 {
    if combat.zones.hand.is_empty() || combat.zones.draw_pile.is_empty() {
        return 0;
    }

    let current_value: i32 = combat
        .zones
        .hand
        .iter()
        .map(|card| gambler_card_turn_value(card, combat, signals))
        .sum();
    let draw_total: i32 = combat
        .zones
        .draw_pile
        .iter()
        .map(|card| gambler_card_turn_value(card, combat, signals))
        .sum();
    let expected_redraw_value =
        draw_total * combat.zones.hand.len() as i32 / combat.zones.draw_pile.len() as i32;
    let upgrade = expected_redraw_value - current_value;

    if upgrade >= 24 {
        upgrade / 2
    } else if upgrade >= 12 {
        upgrade / 3
    } else {
        0
    }
}

fn gambler_card_turn_value(
    card: &crate::combat::CombatCard,
    combat: &CombatState,
    signals: &CombatSignals,
) -> i32 {
    let def = get_card_definition(card.id);
    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return -12;
    }
    if crate::content::cards::can_play_card(card, combat).is_err() {
        return -6;
    }

    let damage = if card.base_damage_mut > 0 {
        card.base_damage_mut
    } else {
        def.base_damage
    };
    let block = if card.base_block_mut > 0 {
        card.base_block_mut
    } else {
        def.base_block
    };

    let mut score = 0;
    if signals.threat.unblocked_incoming > 0 {
        score += block.max(0) * 3;
    } else {
        score += block.max(0);
    }
    score += damage.max(0).min(18);

    if matches!(
        card.id,
        CardId::BattleTrance
            | CardId::BurningPact
            | CardId::Offering
            | CardId::SeeingRed
            | CardId::ShrugItOff
            | CardId::PommelStrike
            | CardId::Disarm
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::Impervious
    ) {
        score += 12;
    }

    if def.card_type == CardType::Power
        && signals.fight.elite_or_boss
        && signals.fight.early_buff_window
    {
        score += 10;
    }

    score
}

fn candidate_reason(
    signals: &CombatSignals,
    category: PotionCategory,
    potion_id: PotionId,
) -> String {
    match potion_id {
        PotionId::CultistPotion if signals.offense.boss_stalling_window => {
            "stallable boss setup".to_string()
        }
        PotionId::SmokeBomb => "hopeless lethal escape".to_string(),
        PotionId::FruitJuice => "permanent max hp and free slot".to_string(),
        PotionId::AncientPotion if signals.hand.hand_has_battle_trance => {
            "artifact shield for battle trance".to_string()
        }
        PotionId::AncientPotion if signals.threat.debuffing_monsters > 0 => {
            "artifact shield for incoming debuff".to_string()
        }
        PotionId::DistilledChaosPotion if signals.threat.imminent_lethal => {
            "panic tempo in lethal window".to_string()
        }
        PotionId::LiquidMemories if signals.hand.discard_recovery_score >= 24 => {
            "recover premium discard".to_string()
        }
        PotionId::DuplicationPotion => "duplicate high-impact follow-up".to_string(),
        PotionId::GamblersBrew if signals.threat.unblocked_incoming > 0 => {
            "redraw to stabilize under pressure".to_string()
        }
        PotionId::GamblersBrew => "redraw weak hand and dig for payoff".to_string(),
        PotionId::GhostInAJar if signals.threat.imminent_lethal => {
            "intangible survival spike".to_string()
        }
        PotionId::EntropicBrew => "refill potion slots".to_string(),
        PotionId::Ambrosia => "divinity burst window".to_string(),
        _ => match category {
            PotionCategory::Survival => "survival margin".to_string(),
            PotionCategory::Lethal => "damage or lethal window".to_string(),
            PotionCategory::Setup => "boss or elite setup window".to_string(),
            PotionCategory::Recovery => "hand or discard recovery".to_string(),
            PotionCategory::Escape => "escape line".to_string(),
            PotionCategory::RandomGeneration => "high pressure random generation".to_string(),
        },
    }
}

#[allow(dead_code)]
fn _debug_label(category: PotionCategory) -> &'static str {
    category_label(category)
}

#[cfg(test)]
mod tests {
    use super::choose_immediate_potion_candidate;
    use crate::combat::Intent;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::{Potion, PotionId};
    use crate::testing::support::test_support::{combat_with_hand, CombatTestExt};

    #[test]
    fn block_potion_is_not_used_on_low_pressure_chip_damage_turn() {
        let mut combat = combat_with_hand(&[
            CardId::Intimidate,
            CardId::Defend,
            CardId::Strike,
            CardId::Defend,
            CardId::Strike,
        ])
        .with_player_hp(80)
        .with_energy(3)
        .with_monster_type(1, EnemyId::LouseNormal)
        .with_monster_max_hp(1, 13)
        .with_monster_hp(1, 13)
        .with_monster_intent(1, Intent::Attack { damage: 5, hits: 1 });
        let mut second = combat.entities.monsters[0].clone();
        second.id = 2;
        second.slot = 1;
        second.logical_position = 1;
        second.monster_type = EnemyId::LouseDefensive as usize;
        second.max_hp = 11;
        second.current_hp = 11;
        second.current_intent = Intent::Attack { damage: 5, hits: 1 };
        second.intent_dmg = 5;
        combat.entities.monsters.push(second);
        combat.entities.potions[0] = Some(Potion::new(PotionId::BlockPotion, 1));

        assert!(choose_immediate_potion_candidate(&combat).is_none());
    }

    #[test]
    fn colorless_potion_is_not_used_on_light_pressure_single_cultist_turn() {
        let mut combat = combat_with_hand(&[
            CardId::FeelNoPain,
            CardId::Strike,
            CardId::Bash,
            CardId::PommelStrike,
            CardId::Strike,
        ])
        .with_player_hp(80)
        .with_energy(3)
        .with_monster_type(1, EnemyId::Cultist)
        .with_monster_max_hp(1, 49)
        .with_monster_hp(1, 37)
        .with_monster_intent(1, Intent::Attack { damage: 6, hits: 1 });
        combat.entities.potions[0] = Some(Potion::new(PotionId::ColorlessPotion, 1));
        combat.zones.draw_pile = vec![
            crate::combat::CombatCard::new(CardId::Defend, 100),
            crate::combat::CombatCard::new(CardId::Defend, 101),
        ];
        combat.zones.discard_pile = vec![
            crate::combat::CombatCard::new(CardId::Strike, 102),
            crate::combat::CombatCard::new(CardId::Defend, 103),
            crate::combat::CombatCard::new(CardId::Strike, 104),
            crate::combat::CombatCard::new(CardId::Defend, 105),
        ];

        assert!(choose_immediate_potion_candidate(&combat).is_none());
    }

    #[test]
    fn energy_potion_is_not_used_on_moderate_pressure_hallway_turn() {
        let mut combat = combat_with_hand(&[
            CardId::Strike,
            CardId::ShrugItOff,
            CardId::Defend,
            CardId::Defend,
            CardId::Defend,
        ])
        .with_player_hp(57)
        .with_player_max_hp(88)
        .with_energy(3)
        .with_monster_type(1, EnemyId::SlaverRed)
        .with_monster_max_hp(1, 50)
        .with_monster_hp(1, 50)
        .with_monster_intent(
            1,
            Intent::Attack {
                damage: 13,
                hits: 1,
            },
        );
        combat.entities.potions[0] = Some(Potion::new(PotionId::EnergyPotion, 1));

        assert!(choose_immediate_potion_candidate(&combat).is_none());
    }

    #[test]
    fn liquid_memories_is_not_used_on_zero_incoming_hallway_turn() {
        let mut combat = combat_with_hand(&[
            CardId::ShrugItOff,
            CardId::Strike,
            CardId::Strike,
            CardId::Bludgeon,
        ])
        .with_player_hp(58)
        .with_player_max_hp(98)
        .with_player_block(11)
        .with_energy(0)
        .with_monster_type(1, EnemyId::SnakePlant)
        .with_monster_max_hp(1, 79)
        .with_monster_hp(1, 60)
        .with_monster_intent(1, Intent::StrongDebuff);
        combat.entities.potions[0] = Some(Potion::new(PotionId::LiquidMemories, 1));
        combat.zones.discard_pile = vec![
            crate::combat::CombatCard::new(CardId::ShrugItOff, 100),
            crate::combat::CombatCard::new(CardId::Anger, 101),
            crate::combat::CombatCard::new(CardId::Strike, 102),
            crate::combat::CombatCard::new(CardId::Strike, 103),
        ];

        assert!(choose_immediate_potion_candidate(&combat).is_none());
    }

    #[test]
    fn dexterity_potion_is_still_allowed_on_guardian_setup_turn() {
        let mut combat = combat_with_hand(&[
            CardId::Defend,
            CardId::PommelStrike,
            CardId::Bash,
            CardId::Strike,
            CardId::Cleave,
        ])
        .with_player_hp(46)
        .with_player_max_hp(88)
        .with_energy(3)
        .with_boss_fight(true)
        .with_monster_type(1, EnemyId::TheGuardian)
        .with_monster_max_hp(1, 240)
        .with_monster_hp(1, 240)
        .with_monster_intent(1, Intent::Defend);
        combat.entities.potions[0] = Some(Potion::new(PotionId::DexterityPotion, 1));

        let chosen =
            choose_immediate_potion_candidate(&combat).expect("dex potion should remain allowed");
        assert_eq!(chosen.potion_id, PotionId::DexterityPotion);
    }

    #[test]
    fn setup_potion_waits_after_opening_turn_even_in_boss_fight() {
        let mut combat = combat_with_hand(&[
            CardId::Defend,
            CardId::PommelStrike,
            CardId::Bash,
            CardId::Strike,
            CardId::Cleave,
        ])
        .with_player_hp(70)
        .with_player_max_hp(88)
        .with_energy(3)
        .with_boss_fight(true)
        .with_monster_type(1, EnemyId::TheGuardian)
        .with_monster_max_hp(1, 220)
        .with_monster_hp(1, 220)
        .with_monster_intent(
            1,
            Intent::Attack {
                damage: 12,
                hits: 1,
            },
        );
        combat.turn.turn_count = 2;
        combat.entities.potions[0] = Some(Potion::new(PotionId::StrengthPotion, 1));

        assert!(choose_immediate_potion_candidate(&combat).is_none());
    }
}
