mod monster;
mod player;
mod power;
mod relic;

use serde_json::Value;

use super::build::{
    build_draw_pile_from_snapshot, build_hand_from_snapshot, build_limbo_from_snapshot,
    build_pile_from_ids, build_runtime_hints_from_snapshots,
};
use super::rng::sync_rng;

pub fn sync_state(cs: &mut crate::runtime::combat::CombatState, snapshot: &Value) {
    sync_state_from_snapshots(cs, snapshot, snapshot);
}

pub fn sync_state_from_snapshots(
    cs: &mut crate::runtime::combat::CombatState,
    truth_snapshot: &Value,
    observation_snapshot: &Value,
) {
    player::sync_player_from_snapshot(cs, truth_snapshot);

    cs.zones.hand = build_hand_from_snapshot(truth_snapshot);

    monster::sync_monsters_from_snapshots(cs, truth_snapshot, observation_snapshot);

    cs.update_hand_cards();

    cs.zones.draw_pile = build_draw_pile_from_snapshot(truth_snapshot);
    cs.zones.discard_pile = build_pile_from_ids("discard_pile_ids", truth_snapshot, 3000);
    cs.zones.exhaust_pile = build_pile_from_ids("exhaust_pile_ids", truth_snapshot, 4000);
    cs.zones.limbo = build_limbo_from_snapshot(truth_snapshot);

    sync_rng(&mut cs.rng, truth_snapshot);

    relic::sync_player_potions_from_snapshot(cs, truth_snapshot);
    relic::sync_player_relics_from_snapshot(cs, truth_snapshot);
    crate::content::relics::restore_combat_energy_master(cs);

    cs.clear_pending_actions();
    cs.turn.begin_player_phase();
    cs.runtime = build_runtime_hints_from_snapshots(truth_snapshot, observation_snapshot);
}
