use crate::runtime::combat::{CombatState, OrbEntity, RelicBuses};

use super::super::types::{
    CombatExactPlayerKey, CombatOrbKey, CombatPlayerFutureKey, CombatRelicBusesKey, CombatRelicKey,
};

pub(super) fn player_exact_key(combat: &CombatState) -> CombatExactPlayerKey {
    let player = &combat.entities.player;
    CombatExactPlayerKey {
        current_hp: player.current_hp,
        block: player.block,
        future_relevant: player_future_key(combat),
    }
}

pub(super) fn player_future_key(combat: &CombatState) -> CombatPlayerFutureKey {
    let player = &combat.entities.player;
    CombatPlayerFutureKey {
        entity_id: player.id,
        max_hp: player.max_hp,
        facing_left: player.facing_left,
        gold_delta_this_combat: player.gold_delta_this_combat,
        gold: player.gold,
        max_orbs: player.max_orbs,
        orbs: player.orbs.iter().map(orb_key).collect(),
        stance: player.stance,
        relics: player
            .relics
            .iter()
            .map(|relic| CombatRelicKey {
                id: relic.id,
                counter: relic.counter,
                used_up: relic.used_up,
                amount: relic.amount,
            })
            .collect(),
        relic_buses: relic_buses_key(&player.relic_buses),
        energy_master: player.energy_master,
    }
}

fn orb_key(orb: &OrbEntity) -> CombatOrbKey {
    CombatOrbKey {
        id: orb.id,
        base_passive_amount: orb.base_passive_amount,
        base_evoke_amount: orb.base_evoke_amount,
        passive_amount: orb.passive_amount,
        evoke_amount: orb.evoke_amount,
    }
}

fn relic_buses_key(buses: &RelicBuses) -> CombatRelicBusesKey {
    CombatRelicBusesKey::from_slices([
        &buses.at_pre_battle,
        &buses.at_battle_start_pre_draw,
        &buses.at_battle_start,
        &buses.at_turn_start,
        &buses.at_turn_start_post_draw,
        &buses.on_use_card,
        &buses.on_shuffle,
        &buses.on_exhaust,
        &buses.on_lose_hp,
        &buses.on_victory,
        &buses.on_apply_power,
        &buses.on_monster_death,
        &buses.on_spawn_monster,
        &buses.at_end_of_turn,
        &buses.on_use_potion,
        &buses.on_discard,
        &buses.on_change_stance,
        &buses.on_attacked_to_change_damage,
        &buses.on_lose_hp_last,
        &buses.on_calculate_heal,
        &buses.on_calculate_x_cost,
        &buses.on_calculate_block_retained,
        &buses.on_calculate_energy_retained,
        &buses.on_scry,
        &buses.on_receive_power_modify,
        &buses.on_calculate_vulnerable_multiplier,
    ])
}
