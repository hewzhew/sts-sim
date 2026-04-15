mod monster;
mod player;
mod power;
mod relic;

use serde_json::Value;

use crate::runtime::combat::CombatPhase;

use super::build::{
    build_draw_pile_from_snapshot, build_hand_from_snapshot, build_limbo_from_snapshot,
    build_pile_from_ids, build_runtime_hints_from_snapshot,
};
use super::rng::sync_rng;

pub fn sync_state(cs: &mut crate::runtime::combat::CombatState, snapshot: &Value) {
    player::sync_player_from_snapshot(cs, snapshot);

    cs.zones.hand = build_hand_from_snapshot(snapshot);

    monster::sync_monsters_from_snapshot(cs, snapshot);

    cs.update_hand_cards();

    cs.zones.draw_pile = build_draw_pile_from_snapshot(snapshot);
    cs.zones.discard_pile = build_pile_from_ids("discard_pile_ids", snapshot, 3000);
    cs.zones.exhaust_pile = build_pile_from_ids("exhaust_pile_ids", snapshot, 4000);
    cs.zones.limbo = build_limbo_from_snapshot(snapshot);

    sync_rng(&mut cs.rng, snapshot);

    relic::sync_player_potions_from_snapshot(cs, snapshot);
    relic::sync_player_relics_from_snapshot(cs, snapshot);
    crate::content::relics::restore_combat_energy_master(cs);

    cs.engine.action_queue.clear();
    cs.turn.current_phase = CombatPhase::PlayerTurn;
    cs.runtime = build_runtime_hints_from_snapshot(snapshot);
}
