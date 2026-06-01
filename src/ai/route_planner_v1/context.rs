use crate::state::RunState;

use super::features::route_targets;
use super::types::{RouteCountersV1, RouteDecisionContextV1};

mod deck;
mod potions;
mod relics;
mod unknown;
mod util;

use deck::build_deck_summary;
use potions::build_potion_summary;
use relics::build_relic_summary;
use unknown::build_unknown_belief;
use util::debug_words;

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
