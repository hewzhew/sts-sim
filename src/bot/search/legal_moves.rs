use crate::combat::CombatState;
use crate::engine::targeting;
use crate::state::core::{ClientInput, HandSelectReason, PendingChoice};
use crate::state::EngineState;

use super::hand_select::{
    score_discard_candidate, score_discard_to_hand_candidate, score_exhaust_candidate,
    score_put_on_draw_pile_candidate,
};

pub(crate) fn get_legal_moves(engine: &EngineState, combat: &CombatState) -> Vec<ClientInput> {
    let mut moves = Vec::new();

    match engine {
        EngineState::CombatPlayerTurn => {
            moves.push(ClientInput::EndTurn);

            let hp_percent =
                (combat.entities.player.current_hp * 100) / combat.entities.player.max_hp.max(1);
            let mut expected_inc_damage = 0;
            for m in &combat.entities.monsters {
                if !m.is_dying && !m.is_escaped && !m.half_dead {
                    match m.current_intent {
                        crate::combat::Intent::Attack { hits, .. }
                        | crate::combat::Intent::AttackBuff { hits, .. }
                        | crate::combat::Intent::AttackDebuff { hits, .. }
                        | crate::combat::Intent::AttackDefend { hits, .. } => {
                            expected_inc_damage += (m.intent_dmg * (hits as i32)).max(0);
                        }
                        _ => {}
                    }
                }
            }
            let unblocked = expected_inc_damage - combat.entities.player.block as i32;
            let mut potion_allowed = false;

            if combat.meta.is_boss_fight && combat.turn.turn_count == 1 {
                potion_allowed = true;
            } else if combat.meta.is_elite_fight && hp_percent <= 50 && combat.turn.turn_count == 1
            {
                potion_allowed = true;
            }

            if unblocked >= 15 || (hp_percent <= 30 && expected_inc_damage > 0) {
                potion_allowed = true;
            }

            if potion_allowed {
                for (i, opt_p) in combat.entities.potions.iter().enumerate() {
                    if let Some(p) = opt_p {
                        use crate::content::potions::PotionId;
                        if p.id == PotionId::SmokeBomb && combat.meta.is_boss_fight {
                            continue;
                        }
                        let def = crate::content::potions::get_potion_definition(p.id);
                        if let Some(validation) =
                            targeting::validation_for_potion_target(def.target_required)
                        {
                            for target in targeting::candidate_targets(combat, validation) {
                                moves.push(ClientInput::UsePotion {
                                    potion_index: i,
                                    target: Some(target),
                                });
                            }
                        } else {
                            moves.push(ClientInput::UsePotion {
                                potion_index: i,
                                target: None,
                            });
                        }
                    }
                }
            }

            for (i, card) in combat.zones.hand.iter().enumerate() {
                if crate::content::cards::can_play_card(card, combat).is_ok() {
                    let target_type = crate::content::cards::get_card_definition(card.id).target;
                    if let Some(validation) = targeting::validation_for_card_target(target_type) {
                        for target in targeting::candidate_targets(combat, validation) {
                            moves.push(ClientInput::PlayCard {
                                card_index: i,
                                target: Some(target),
                            });
                        }
                    } else {
                        moves.push(ClientInput::PlayCard {
                            card_index: i,
                            target: None,
                        });
                    }
                }
            }
        }
        EngineState::PendingChoice(choice) => match choice {
            PendingChoice::HandSelect {
                min_cards,
                max_cards,
                candidate_uuids,
                reason,
                ..
            } => {
                if matches!(
                    reason,
                    HandSelectReason::Discard
                        | HandSelectReason::GamblingChip
                        | HandSelectReason::Exhaust
                ) && *min_cards == 0
                {
                    let mut ordered = candidate_uuids.clone();
                    match reason {
                        HandSelectReason::Exhaust => ordered.sort_by_key(|uuid| {
                            std::cmp::Reverse(score_exhaust_candidate(combat, *uuid))
                        }),
                        _ => ordered.sort_by_key(|uuid| {
                            std::cmp::Reverse(score_discard_candidate(combat, *uuid))
                        }),
                    }

                    moves.push(ClientInput::SubmitHandSelect(Vec::new()));

                    let mut positive = Vec::new();
                    for uuid in ordered {
                        let candidate_score = match reason {
                            HandSelectReason::Exhaust => score_exhaust_candidate(combat, uuid),
                            _ => score_discard_candidate(combat, uuid),
                        };
                        if candidate_score <= 0 {
                            continue;
                        }
                        positive.push(uuid);
                    }

                    let max_prefix = positive.len().min((*max_cards as usize).min(4));
                    for take in 1..=max_prefix {
                        moves.push(ClientInput::SubmitHandSelect(positive[..take].to_vec()));
                    }
                    if positive.len() > max_prefix && positive.len() <= *max_cards as usize {
                        moves.push(ClientInput::SubmitHandSelect(positive));
                    }
                } else if *min_cards == 1 {
                    let mut ordered = candidate_uuids.clone();
                    match reason {
                        HandSelectReason::PutOnDrawPile => {
                            ordered.sort_by_key(|uuid| {
                                std::cmp::Reverse(score_put_on_draw_pile_candidate(combat, *uuid))
                            });
                        }
                        HandSelectReason::Exhaust => {
                            ordered.sort_by_key(|uuid| {
                                std::cmp::Reverse(score_exhaust_candidate(combat, *uuid))
                            });
                        }
                        HandSelectReason::Discard | HandSelectReason::GamblingChip => {
                            ordered.sort_by_key(|uuid| {
                                std::cmp::Reverse(score_discard_candidate(combat, *uuid))
                            });
                        }
                        _ => {}
                    }
                    for uuid in ordered {
                        moves.push(ClientInput::SubmitHandSelect(vec![uuid]));
                    }
                } else if *min_cards > 1 {
                    let mut ordered = candidate_uuids.clone();
                    match reason {
                        HandSelectReason::PutOnDrawPile => {
                            ordered.sort_by_key(|uuid| {
                                std::cmp::Reverse(score_put_on_draw_pile_candidate(combat, *uuid))
                            });
                        }
                        HandSelectReason::Exhaust => {
                            ordered.sort_by_key(|uuid| {
                                std::cmp::Reverse(score_exhaust_candidate(combat, *uuid))
                            });
                        }
                        HandSelectReason::Discard | HandSelectReason::GamblingChip => {
                            ordered.sort_by_key(|uuid| {
                                std::cmp::Reverse(score_discard_candidate(combat, *uuid))
                            });
                        }
                        _ => {}
                    }
                    moves.push(ClientInput::SubmitHandSelect(
                        ordered.into_iter().take(*min_cards as usize).collect(),
                    ));
                } else {
                    moves.push(ClientInput::SubmitHandSelect(Vec::new()));
                }
            }
            PendingChoice::GridSelect {
                min_cards,
                candidate_uuids,
                max_cards,
                reason,
                ..
            } => {
                if *min_cards == 0 {
                    moves.push(ClientInput::SubmitGridSelect(Vec::new()));
                }
                let mut ordered = candidate_uuids.clone();
                if *reason == crate::state::GridSelectReason::DiscardToHand {
                    ordered.sort_by_key(|uuid| {
                        std::cmp::Reverse(score_discard_to_hand_candidate(combat, *uuid))
                    });
                }
                let mut selected = Vec::new();
                for i in 0..*min_cards {
                    if let Some(&uuid) = ordered.get(i as usize) {
                        selected.push(uuid);
                    }
                }
                moves.push(ClientInput::SubmitGridSelect(selected));
                if *reason == crate::state::GridSelectReason::DiscardToHand && *max_cards >= 2 {
                    let alt: Vec<u32> = ordered.iter().copied().take(2).collect();
                    if alt.len() >= 2 {
                        moves.push(ClientInput::SubmitGridSelect(alt));
                    }
                }
            }
            PendingChoice::DiscoverySelect(_) => {
                moves.push(ClientInput::SubmitDiscoverChoice(0));
                moves.push(ClientInput::SubmitDiscoverChoice(1));
                moves.push(ClientInput::SubmitDiscoverChoice(2));
            }
            PendingChoice::CardRewardSelect { .. } => {
                moves.push(ClientInput::SubmitCardChoice(vec![0]));
            }
            PendingChoice::StanceChoice => {
                let hp_percent = (combat.entities.player.current_hp * 100)
                    / combat.entities.player.max_hp.max(1);
                let expected_inc_damage: i32 = combat
                    .entities
                    .monsters
                    .iter()
                    .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead)
                    .map(|m| match m.current_intent {
                        crate::combat::Intent::Attack { hits, .. }
                        | crate::combat::Intent::AttackBuff { hits, .. }
                        | crate::combat::Intent::AttackDebuff { hits, .. }
                        | crate::combat::Intent::AttackDefend { hits, .. } => {
                            (m.intent_dmg * (hits as i32)).max(0)
                        }
                        _ => 0,
                    })
                    .sum();
                let unblocked = (expected_inc_damage - combat.entities.player.block).max(0);
                let playable_attacks = combat
                    .zones
                    .hand
                    .iter()
                    .filter(|card| crate::content::cards::can_play_card(card, combat).is_ok())
                    .filter(|card| {
                        crate::content::cards::get_card_definition(card.id).card_type
                            == crate::content::cards::CardType::Attack
                    })
                    .count();

                let prefer_calm = unblocked > 0 || hp_percent <= 45 || playable_attacks == 0;
                if prefer_calm {
                    moves.push(ClientInput::SubmitDiscoverChoice(1));
                    moves.push(ClientInput::SubmitDiscoverChoice(0));
                } else {
                    moves.push(ClientInput::SubmitDiscoverChoice(0));
                    moves.push(ClientInput::SubmitDiscoverChoice(1));
                }
            }
            _ => {
                moves.push(ClientInput::Proceed);
            }
        },
        _ => {
            moves.push(ClientInput::Proceed);
        }
    }

    moves
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{
        CombatCard, CombatMeta, CombatPhase, CombatRng, EngineRuntime, EntityState, Intent,
        MonsterEntity, PlayerEntity, RelicBuses, StanceId, TurnRuntime,
    };
    use crate::state::core::{GridSelectReason, PileType};
    use std::collections::{HashMap, VecDeque};

    fn test_combat() -> CombatState {
        CombatState {
            meta: CombatMeta {
                ascension_level: 0,
                is_boss_fight: false,
                is_elite_fight: false,
                meta_changes: Vec::new(),
            },
            turn: TurnRuntime {
                turn_count: 1,
                current_phase: CombatPhase::PlayerTurn,
                energy: 2,
                turn_start_draw_modifier: 0,
                counters: Default::default(),
            },
            zones: crate::combat::CardZones {
                draw_pile: Vec::new(),
                hand: Vec::new(),
                discard_pile: Vec::new(),
                exhaust_pile: Vec::new(),
                limbo: Vec::new(),
                queued_cards: VecDeque::new(),
                card_uuid_counter: 100,
            },
            entities: EntityState {
                player: PlayerEntity {
                    id: 0,
                    current_hp: 40,
                    max_hp: 80,
                    block: 0,
                    gold_delta_this_combat: 0,
                    gold: 99,
                    max_orbs: 0,
                    orbs: Vec::new(),
                    stance: StanceId::Neutral,
                    relics: Vec::new(),
                    relic_buses: RelicBuses::default(),
                    energy_master: 3,
                },
                monsters: vec![MonsterEntity {
                    id: 1,
                    monster_type: crate::content::monsters::EnemyId::JawWorm as usize,
                    current_hp: 36,
                    max_hp: 36,
                    block: 0,
                    slot: 0,
                    is_dying: false,
                    is_escaped: false,
                    half_dead: false,
                    next_move_byte: 0,
                    current_intent: Intent::Attack {
                        damage: 12,
                        hits: 1,
                    },
                    move_history: VecDeque::new(),
                    intent_dmg: 12,
                    logical_position: 0,
                    hexaghost: Default::default(),
                    darkling: Default::default(),
                }],
                potions: vec![None, None, None],
                power_db: HashMap::new(),
            },
            engine: EngineRuntime {
                action_queue: VecDeque::new(),
            },
            rng: CombatRng::new(crate::rng::RngPool::new(123)),
        }
    }

    #[test]
    fn stance_choice_exposes_wrath_and_calm_instead_of_proceed() {
        let combat = test_combat();
        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::StanceChoice),
            &combat,
        );
        assert!(moves
            .iter()
            .any(|m| matches!(m, ClientInput::SubmitDiscoverChoice(0))));
        assert!(moves
            .iter()
            .any(|m| matches!(m, ClientInput::SubmitDiscoverChoice(1))));
        assert!(!moves.iter().any(|m| matches!(m, ClientInput::Proceed)));
    }

    #[test]
    fn liquid_memories_prioritizes_offering_from_discard() {
        let mut combat = test_combat();
        let offering = CombatCard::new(crate::content::cards::CardId::Offering, 11);
        let strike = CombatCard::new(crate::content::cards::CardId::Strike, 12);
        combat.zones.discard_pile = vec![strike.clone(), offering.clone()];
        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::GridSelect {
                source_pile: PileType::Discard,
                candidate_uuids: vec![strike.uuid, offering.uuid],
                min_cards: 1,
                max_cards: 1,
                can_cancel: false,
                reason: GridSelectReason::DiscardToHand,
            }),
            &combat,
        );
        assert!(matches!(
            moves.first(),
            Some(ClientInput::SubmitGridSelect(selected)) if selected == &vec![offering.uuid]
        ));
    }

    #[test]
    fn zero_min_exhaust_selection_offers_positive_exhaust_choices() {
        let mut combat = test_combat();
        let slimed = CombatCard::new(crate::content::cards::CardId::Slimed, 21);
        let strike = CombatCard::new(crate::content::cards::CardId::Strike, 22);
        combat.zones.hand = vec![strike, slimed.clone()];
        let moves = get_legal_moves(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids: vec![21, 22],
                min_cards: 0,
                max_cards: 99,
                can_cancel: true,
                reason: crate::state::core::HandSelectReason::Exhaust,
            }),
            &combat,
        );
        assert!(moves.iter().any(|m| matches!(
            m,
            ClientInput::SubmitHandSelect(selected) if selected == &vec![slimed.uuid]
        )));
    }
}