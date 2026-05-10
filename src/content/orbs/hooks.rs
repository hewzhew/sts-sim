use crate::content::powers::{store, PowerId};
use crate::content::relics::RelicId;
use crate::runtime::action::{Action, ActionInfo, AddTo, DamageType};
use crate::runtime::combat::{CombatState, OrbEntity, OrbId};

pub fn at_turn_start(
    state: &CombatState,
) -> smallvec::SmallVec<[crate::runtime::action::ActionInfo; 4]> {
    if state.entities.player.orbs.is_empty() {
        smallvec::smallvec![]
    } else {
        smallvec::smallvec![ActionInfo {
            action: Action::TriggerStartOfTurnOrbs,
            insertion_mode: AddTo::Bottom,
        }]
    }
}

pub fn trigger_end_of_turn_orbs(
    state: &CombatState,
) -> smallvec::SmallVec<[crate::runtime::action::ActionInfo; 4]> {
    if state.entities.player.orbs.is_empty() {
        smallvec::smallvec![]
    } else {
        smallvec::smallvec![ActionInfo {
            action: Action::TriggerEndOfTurnOrbs,
            insertion_mode: AddTo::Bottom,
        }]
    }
}

fn focus_amount(state: &CombatState) -> i32 {
    store::power_amount(state, 0, PowerId::Focus)
}

fn focused_amount(id: OrbId, base_amount: i32, state: &CombatState) -> i32 {
    if matches!(id, OrbId::Empty | OrbId::Plasma) {
        base_amount
    } else {
        (base_amount + focus_amount(state)).max(0)
    }
}

fn passive_amount(orb: &OrbEntity, state: &CombatState) -> i32 {
    focused_amount(orb.id, orb.base_passive_amount, state)
}

fn evoke_amount(orb: &OrbEntity, state: &CombatState) -> i32 {
    match orb.id {
        OrbId::Dark | OrbId::Plasma => orb.evoke_amount,
        OrbId::Empty => 0,
        _ => focused_amount(orb.id, orb.base_evoke_amount, state),
    }
}

pub fn refresh_orb_focus_values(state: &mut CombatState) {
    let focus = focus_amount(state);
    for orb in &mut state.entities.player.orbs {
        match orb.id {
            OrbId::Empty => {
                orb.passive_amount = 0;
                orb.evoke_amount = 0;
            }
            OrbId::Plasma => {
                orb.passive_amount = orb.base_passive_amount;
                orb.evoke_amount = orb.base_evoke_amount;
            }
            OrbId::Dark => {
                orb.passive_amount = (orb.base_passive_amount + focus).max(0);
            }
            _ => {
                orb.passive_amount = (orb.base_passive_amount + focus).max(0);
                orb.evoke_amount = (orb.base_evoke_amount + focus).max(0);
            }
        }
    }
}

fn queue_orb_action(state: &mut CombatState, action: Action, to_front: bool) {
    if to_front {
        state.queue_action_front(action);
    } else {
        state.queue_action_back(action);
    }
}

fn queue_lightning_damage(state: &mut CombatState, amount: i32, to_front: bool) {
    if amount <= 0 {
        return;
    }
    if store::power_amount(state, 0, PowerId::Electro) > 0 {
        let damages = state
            .entities
            .monsters
            .iter()
            .map(|_| amount)
            .collect::<smallvec::SmallVec<[i32; 5]>>();
        queue_orb_action(
            state,
            Action::DamageAllEnemies {
                source: 0,
                damages,
                damage_type: DamageType::Thorns,
                is_modified: false,
            },
            to_front,
        );
    } else {
        queue_orb_action(
            state,
            Action::AttackDamageRandomEnemy {
                base_damage: amount,
                damage_type: DamageType::Thorns,
                applies_target_modifiers: false,
            },
            to_front,
        );
    }
}

fn queue_dark_damage(state: &mut CombatState, amount: i32, to_front: bool) {
    if amount <= 0 {
        return;
    }
    let Some(target) = state
        .entities
        .monsters
        .iter()
        .filter(|m| m.current_hp > 0 && !m.is_dying && !m.is_escaped)
        .min_by_key(|m| m.current_hp)
        .map(|m| m.id)
    else {
        return;
    };
    queue_orb_action(
        state,
        Action::Damage(crate::runtime::action::DamageInfo {
            source: 0,
            target,
            base: amount,
            output: amount,
            damage_type: DamageType::Thorns,
            is_modified: false,
        }),
        to_front,
    );
}

fn trigger_orb_start_of_turn(state: &mut CombatState, orb_index: usize) {
    let Some(orb) = state.entities.player.orbs.get(orb_index).cloned() else {
        return;
    };
    if orb.id == OrbId::Plasma {
        let amount = passive_amount(&orb, state);
        if amount > 0 {
            state.queue_action_back(Action::GainEnergy { amount });
        }
    }
}

fn trigger_orb_end_of_turn(state: &mut CombatState, orb_index: usize) {
    let Some(orb) = state.entities.player.orbs.get(orb_index).cloned() else {
        return;
    };
    match orb.id {
        OrbId::Lightning => queue_lightning_damage(state, passive_amount(&orb, state), false),
        OrbId::Frost => {
            let amount = passive_amount(&orb, state);
            if amount > 0 {
                state.queue_action_back(Action::GainBlock { target: 0, amount });
            }
        }
        OrbId::Dark => {
            let amount = passive_amount(&orb, state);
            if amount > 0 {
                if let Some(current) = state.entities.player.orbs.get_mut(orb_index) {
                    current.passive_amount = amount;
                    current.evoke_amount += amount;
                }
            }
        }
        OrbId::Empty | OrbId::Plasma => {}
    }
}

fn trigger_cables_start_of_turn(state: &mut CombatState) {
    if state.entities.player.has_relic(RelicId::GoldPlatedCables)
        && state
            .entities
            .player
            .orbs
            .first()
            .is_some_and(|orb| orb.id != OrbId::Empty)
    {
        trigger_orb_start_of_turn(state, 0);
    }
}

fn trigger_cables_end_of_turn(state: &mut CombatState) {
    if state.entities.player.has_relic(RelicId::GoldPlatedCables)
        && state
            .entities
            .player
            .orbs
            .first()
            .is_some_and(|orb| orb.id != OrbId::Empty)
    {
        trigger_orb_end_of_turn(state, 0);
    }
}

pub fn trigger_start_of_turn_orbs_now(state: &mut CombatState) {
    refresh_orb_focus_values(state);
    let len = state.entities.player.orbs.len();
    for i in 0..len {
        trigger_orb_start_of_turn(state, i);
    }
    trigger_cables_start_of_turn(state);
}

pub fn trigger_end_of_turn_orbs_now(state: &mut CombatState) {
    refresh_orb_focus_values(state);
    let len = state.entities.player.orbs.len();
    for i in 0..len {
        trigger_orb_end_of_turn(state, i);
    }
    trigger_cables_end_of_turn(state);
}

pub fn trigger_impulse_orbs_now(state: &mut CombatState) {
    refresh_orb_focus_values(state);
    let len = state.entities.player.orbs.len();
    for i in 0..len {
        trigger_orb_start_of_turn(state, i);
        trigger_orb_end_of_turn(state, i);
    }
    if state.entities.player.has_relic(RelicId::GoldPlatedCables)
        && state
            .entities
            .player
            .orbs
            .first()
            .is_some_and(|orb| orb.id != OrbId::Empty)
    {
        trigger_orb_start_of_turn(state, 0);
        trigger_orb_end_of_turn(state, 0);
    }
}

pub fn evoke_next_orb_now(state: &mut CombatState) {
    refresh_orb_focus_values(state);
    let Some(orb) = state.entities.player.orbs.first().cloned() else {
        return;
    };
    if orb.id == OrbId::Empty {
        return;
    }
    match orb.id {
        OrbId::Lightning => queue_lightning_damage(state, evoke_amount(&orb, state), true),
        OrbId::Frost => {
            let amount = evoke_amount(&orb, state);
            if amount > 0 {
                state.queue_action_front(Action::GainBlock { target: 0, amount });
            }
        }
        OrbId::Dark => queue_dark_damage(state, evoke_amount(&orb, state), true),
        OrbId::Plasma => {
            let amount = evoke_amount(&orb, state);
            if amount > 0 {
                state.queue_action_front(Action::GainEnergy { amount });
            }
        }
        OrbId::Empty => {}
    }

    if !state.entities.player.orbs.is_empty() {
        state.entities.player.orbs.remove(0);
        state
            .entities
            .player
            .orbs
            .push(OrbEntity::new(OrbId::Empty));
    }
}
