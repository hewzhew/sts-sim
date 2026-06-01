use crate::content::powers::{store, PowerId};
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;
pub fn handle_spot_weakness(target: usize, amount: i32, state: &mut CombatState) {
    let Some(target_monster) = state.entities.monsters.iter().find(|m| m.id == target) else {
        return;
    };

    if monster_has_java_attack_intent_base_damage(state, target_monster) {
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Strength,
            amount,
        });
    }
}

pub fn handle_apply_weak_if_target_attacking(target: usize, amount: i32, state: &mut CombatState) {
    let Some(target_monster) = state.entities.monsters.iter().find(|m| m.id == target) else {
        return;
    };

    if monster_has_java_attack_intent_base_damage(state, target_monster) {
        state.queue_action_front(Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Weak,
            amount,
        });
    }
}

pub fn handle_doppelganger(
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let effect = x_cost_effect(state, upgraded, energy_on_use);
    if effect > 0 {
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::Energized,
            amount: effect,
        });
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::DrawCardNextTurn,
            amount: effect,
        });
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_malaise(
    target: usize,
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let effect = x_cost_effect(state, upgraded, energy_on_use);
    if effect > 0 {
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Strength,
            amount: -effect,
        });
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target,
            power_id: PowerId::Weak,
            amount: effect,
        });
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

pub fn handle_collect(
    upgraded: bool,
    free_to_play_once: bool,
    energy_on_use: i32,
    state: &mut CombatState,
) {
    let effect = x_cost_effect(state, upgraded, energy_on_use);
    if effect > 0 {
        state.queue_action_back(Action::ApplyPower {
            source: 0,
            target: 0,
            power_id: PowerId::CollectPower,
            amount: effect,
        });
        if !free_to_play_once {
            state.turn.spend_energy(state.turn.energy as i32);
        }
    }
}

fn x_cost_effect(state: &CombatState, upgraded: bool, energy_on_use: i32) -> i32 {
    let base_effect = if energy_on_use != -1 {
        energy_on_use
    } else {
        state.turn.energy as i32
    };
    let mut effect = crate::content::relics::hooks::on_calculate_x_cost(state, base_effect);
    if upgraded {
        effect += 1;
    }
    effect
}

fn monster_has_java_attack_intent_base_damage(
    state: &CombatState,
    monster: &crate::runtime::combat::MonsterEntity,
) -> bool {
    let current_plan = crate::content::monsters::resolve_monster_turn_plan(state, monster);
    if current_plan.attack().is_some() {
        return true;
    }

    if !monster.is_dead_or_escaped() {
        return false;
    }

    if monster
        .move_state
        .planned_visible_spec
        .as_ref()
        .and_then(|spec| spec.attack())
        .is_some()
    {
        return true;
    }

    monster
        .move_state
        .planned_steps
        .as_ref()
        .is_some_and(|steps| {
            steps
                .iter()
                .any(|step| matches!(step, crate::runtime::monster_move::MoveStep::Attack(_)))
        })
}

pub fn handle_trigger_time_warp_end_turn(owner: usize, state: &mut CombatState) {
    let current_amount = store::power_amount(state, owner, PowerId::TimeWarp);
    if current_amount != 0 {
        let _ = store::with_power_mut(state, owner, PowerId::TimeWarp, |power| {
            power.amount = 0;
            power.just_applied = false;
        });
    }

    // Java TimeWarpPower.callEndTurnEarlySequence clears cardQueue but preserves
    // autoplay cards as dontTriggerOnUseCard cleanup actions.
    crate::engine::action_handlers::cards::handle_queue_early_end_turn(state);

    let alive_monster_ids: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| m.is_random_target_candidate())
        .map(|m| m.id)
        .collect();
    for monster_id in alive_monster_ids {
        state.queue_action_back(Action::ApplyPower {
            source: monster_id,
            target: monster_id,
            power_id: PowerId::Strength,
            amount: 2,
        });
    }
}

fn random_alive_monster(state: &mut CombatState) -> Option<usize> {
    let alive: Vec<usize> = state
        .entities
        .monsters
        .iter()
        .filter(|m| m.is_random_target_candidate())
        .map(|m| m.id)
        .collect();
    if alive.is_empty() {
        None
    } else {
        let idx = state.rng.card_random_rng.random(alive.len() as i32 - 1) as usize;
        Some(alive[idx])
    }
}

pub fn handle_bouncing_flask(
    target: Option<usize>,
    amount: i32,
    num_times: u8,
    state: &mut CombatState,
) {
    let Some(target_id) = target.or_else(|| random_alive_monster(state)) else {
        return;
    };

    if state.are_monsters_basically_dead_java() {
        state.clear_post_combat_actions();
        return;
    }

    if num_times > 1 {
        let next_target = random_alive_monster(state);
        state.queue_action_front(Action::BouncingFlask {
            target: next_target,
            amount,
            num_times: num_times - 1,
        });
    }

    if state
        .entities
        .monsters
        .iter()
        .any(|m| m.id == target_id && m.is_alive_for_action())
    {
        state.queue_action_front(Action::ApplyPower {
            source: 0,
            target: target_id,
            power_id: PowerId::Poison,
            amount,
        });
    }
}

pub fn handle_apply_stasis(target_id: usize, state: &mut CombatState) {
    if state.zones.draw_pile.is_empty() && state.zones.discard_pile.is_empty() {
        return;
    }

    let source_pile_draw = !state.zones.draw_pile.is_empty();
    let source_pile = if source_pile_draw {
        &state.zones.draw_pile
    } else {
        &state.zones.discard_pile
    };

    let rarities_to_check = [
        crate::content::cards::CardRarity::Rare,
        crate::content::cards::CardRarity::Uncommon,
        crate::content::cards::CardRarity::Common,
    ];

    let mut candidates = Vec::new();
    for expected_rarity in rarities_to_check {
        for (i, card) in source_pile.iter().enumerate() {
            let def = crate::content::cards::get_card_definition(card.id);
            if def.rarity == expected_rarity {
                candidates.push(i);
            }
        }
        if !candidates.is_empty() {
            candidates.sort_by(|left, right| {
                crate::content::cards::java_id(source_pile[*left].id)
                    .cmp(crate::content::cards::java_id(source_pile[*right].id))
            });
            break;
        }
    }

    if candidates.is_empty() {
        for i in 0..source_pile.len() {
            candidates.push(i);
        }
    }

    let pick_idx = if candidates.len() > 1 {
        let r = state
            .rng
            .card_random_rng
            .random(candidates.len() as i32 - 1) as usize;
        candidates[r]
    } else {
        candidates[0]
    };

    let card = if source_pile_draw {
        state.zones.draw_pile.remove(pick_idx)
    } else {
        state.zones.discard_pile.remove(pick_idx)
    };

    let uuid = card.uuid as i32;
    state.zones.limbo.push(card);

    state.queue_action_front(Action::UpdatePowerExtraData {
        target: target_id,
        power_id: PowerId::Stasis,
        value: uuid,
    });
    state.queue_action_front(Action::ApplyPower {
        source: target_id,
        target: target_id,
        power_id: PowerId::Stasis,
        amount: -1,
    });
}
