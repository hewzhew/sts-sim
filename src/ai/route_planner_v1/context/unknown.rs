use crate::state::RunState;

use super::super::types::{RouteRelicSummaryV1, UnknownRoomBeliefV1};

pub(super) fn build_unknown_belief(
    run_state: &RunState,
    relics: &RouteRelicSummaryV1,
) -> UnknownRoomBeliefV1 {
    let monster_chance = if relics.has_juzu_bracelet {
        0.0
    } else {
        run_state.event_generator.monster_chance
    };
    let shop_chance = run_state.event_generator.shop_chance;
    let treasure_chance = if relics.has_tiny_chest {
        run_state.event_generator.treasure_chance.max(0.02)
    } else {
        run_state.event_generator.treasure_chance
    };
    let used = monster_chance + shop_chance + treasure_chance;
    UnknownRoomBeliefV1 {
        monster_chance,
        shop_chance,
        treasure_chance,
        event_chance: (1.0 - used).clamp(0.0, 1.0),
        elite_chance: 0.0,
        has_juzu_bracelet: relics.has_juzu_bracelet,
        has_tiny_chest: relics.has_tiny_chest,
        deadly_events: false,
    }
}
