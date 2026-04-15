use crate::runtime::combat::CombatState;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::powers::PowerId;

#[derive(Clone, Copy, Debug)]
pub struct ThreatSignals {
    pub player_hp: i32,
    pub missing_hp: i32,
    pub unblocked_incoming: i32,
    pub low_hp: bool,
    pub critical_hp: bool,
    pub imminent_lethal: bool,
    pub debuffing_monsters: i32,
    pub max_intent_hits: i32,
    pub player_has_artifact: bool,
    pub nob_active: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct OffenseSignals {
    pub alive_monsters: i32,
    pub total_enemy_hp: i32,
    pub playable_attacks: i32,
    pub likely_long_fight: bool,
    pub boss_stalling_window: bool,
    pub fight_almost_over: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct HandSignals {
    pub hand_junk: i32,
    pub playable_blocks: i32,
    pub expensive_unplayable_cards: i32,
    pub energy_hungry_cards: i32,
    pub upgradable_cards_in_hand: i32,
    pub hand_has_searing_blow: bool,
    pub discard_recovery_score: i32,
    pub exhaustable_junk: i32,
    pub hand_has_flex: bool,
    pub hand_has_battle_trance: bool,
    pub hand_has_x_cost: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct FightSignals {
    pub is_boss: bool,
    pub is_elite: bool,
    pub elite_or_boss: bool,
    pub early_buff_window: bool,
    pub potions_full: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct CombatSignals {
    pub threat: ThreatSignals,
    pub offense: OffenseSignals,
    pub hand: HandSignals,
    pub fight: FightSignals,
}

pub fn analyze_combat(combat: &CombatState) -> CombatSignals {
    let mut incoming_damage = 0;
    let mut debuffing_monsters = 0;
    let mut max_intent_hits = 0;
    let mut alive_monsters = 0;
    let mut total_enemy_hp = 0;

    for monster in &combat.entities.monsters {
        if monster.is_dying || monster.is_escaped || monster.current_hp <= 0 {
            continue;
        }
        alive_monsters += 1;
        total_enemy_hp += monster.current_hp + monster.block;
        let hits = match monster.current_intent {
            crate::runtime::combat::Intent::Attack { hits, .. }
            | crate::runtime::combat::Intent::AttackBuff { hits, .. }
            | crate::runtime::combat::Intent::AttackDebuff { hits, .. }
            | crate::runtime::combat::Intent::AttackDefend { hits, .. } => hits as i32,
            _ => 0,
        };
        if matches!(
            monster.current_intent,
            crate::runtime::combat::Intent::Debuff
                | crate::runtime::combat::Intent::StrongDebuff
                | crate::runtime::combat::Intent::AttackDebuff { .. }
        ) {
            debuffing_monsters += 1;
        }
        if hits > 0 {
            incoming_damage += monster.intent_dmg.max(0) * hits.max(1);
            max_intent_hits = max_intent_hits.max(hits.max(1));
        }
    }

    let mut hand_junk = 0;
    let mut playable_attacks = 0;
    let mut playable_blocks = 0;
    let mut expensive_unplayable_cards = 0;
    let mut energy_hungry_cards = 0;
    let mut hand_has_x_cost = false;
    let mut upgradable_cards_in_hand = 0;
    let mut hand_has_searing_blow = false;
    let mut exhaustable_junk = 0;

    for card in &combat.zones.hand {
        let def = get_card_definition(card.id);
        let can_play_now = crate::content::cards::can_play_card(card, combat).is_ok();
        if card.id == CardId::SearingBlow {
            hand_has_searing_blow = true;
        }
        if !matches!(def.card_type, CardType::Status | CardType::Curse)
            && (card.id == CardId::SearingBlow || card.upgrades == 0)
        {
            upgradable_cards_in_hand += 1;
        }
        if matches!(def.card_type, CardType::Curse | CardType::Status) {
            hand_junk += 1;
            exhaustable_junk += 1;
            continue;
        }
        if can_play_now {
            if def.card_type == CardType::Attack {
                playable_attacks += 1;
            }
            if def.base_block > 0 {
                playable_blocks += 1;
            }
        }
        let cost = card.get_cost() as i32;
        if cost < 0 {
            hand_has_x_cost = true;
            energy_hungry_cards += 1;
        } else if cost > 0 {
            if cost > combat.turn.energy as i32 {
                expensive_unplayable_cards += 1;
            }
            if can_play_now {
                energy_hungry_cards += 1;
            }
        }
        if !can_play_now
            && matches!(
                card.id,
                CardId::Strike | CardId::Defend | CardId::DefendG | CardId::GoodInstincts
            )
        {
            hand_junk += 1;
        }
    }

    let discard_recovery_score = combat
        .zones
        .discard_pile
        .iter()
        .map(|card| match card.id {
            CardId::Offering => 42,
            CardId::Apparition => 38,
            CardId::Impervious => 34,
            CardId::Reaper => 30,
            CardId::SearingBlow => 28 + card.upgrades as i32 * 6,
            CardId::FlameBarrier | CardId::GhostlyArmor | CardId::PowerThrough => 22,
            CardId::Disarm | CardId::Shockwave | CardId::Uppercut => 20,
            CardId::ShrugItOff | CardId::BattleTrance | CardId::BurningPact => 16,
            _ => {
                let def = get_card_definition(card.id);
                let mut score = 0;
                if matches!(def.card_type, CardType::Power) {
                    score += 18;
                }
                if def.base_damage >= 12 {
                    score += 10;
                }
                if def.base_block >= 10 {
                    score += 10;
                }
                score
            }
        })
        .max()
        .unwrap_or(0);

    let hp_per =
        (combat.entities.player.current_hp as f32 / combat.entities.player.max_hp as f32) * 100.0;
    let unblocked_incoming = (incoming_damage - combat.entities.player.block).max(0);
    let missing_hp = (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0);
    let fight_almost_over = alive_monsters <= 1 && total_enemy_hp <= 25;

    CombatSignals {
        threat: ThreatSignals {
            player_hp: combat.entities.player.current_hp,
            missing_hp,
            unblocked_incoming,
            low_hp: hp_per <= 50.0,
            critical_hp: hp_per <= 30.0,
            imminent_lethal: incoming_damage
                >= (combat.entities.player.current_hp + combat.entities.player.block),
            debuffing_monsters,
            max_intent_hits,
            player_has_artifact: combat.get_power(0, PowerId::Artifact) > 0,
            nob_active: combat.entities.monsters.iter().any(|m| {
                !m.is_dying && !m.is_escaped && combat.get_power(m.id, PowerId::Anger) != 0
            }),
        },
        offense: OffenseSignals {
            alive_monsters,
            total_enemy_hp,
            playable_attacks,
            likely_long_fight: combat.meta.is_boss_fight
                || alive_monsters == 1
                    && combat
                        .entities
                        .monsters
                        .iter()
                        .filter(|m| !m.is_dying && !m.is_escaped && m.current_hp > 0)
                        .map(|m| m.current_hp + m.block)
                        .max()
                        .unwrap_or(0)
                        >= 80,
            boss_stalling_window: combat.meta.is_boss_fight
                && alive_monsters == 1
                && incoming_damage <= combat.entities.player.block + 12,
            fight_almost_over,
        },
        hand: HandSignals {
            hand_junk,
            playable_blocks,
            expensive_unplayable_cards,
            energy_hungry_cards,
            upgradable_cards_in_hand,
            hand_has_searing_blow,
            discard_recovery_score,
            exhaustable_junk,
            hand_has_flex: combat.zones.hand.iter().any(|c| c.id == CardId::Flex),
            hand_has_battle_trance: combat
                .zones
                .hand
                .iter()
                .any(|c| c.id == CardId::BattleTrance),
            hand_has_x_cost,
        },
        fight: FightSignals {
            is_boss: combat.meta.is_boss_fight,
            is_elite: combat.meta.is_elite_fight,
            elite_or_boss: combat.meta.is_elite_fight || combat.meta.is_boss_fight,
            early_buff_window: combat.turn.turn_count <= 2,
            potions_full: combat.entities.potions.iter().all(|slot| slot.is_some()),
        },
    }
}
