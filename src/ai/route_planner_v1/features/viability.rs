use crate::ai::route_window_facts::RouteWindowPath;
use crate::state::map::node::RoomType;

use super::super::types::{RoutePathViabilityV1, RoutePlannerConfigV1, UnknownRoomBeliefV1};

const HALLWAY_HP_LOSS_P90: f32 = 14.0;
const ELITE_HP_LOSS_P90: f32 = 40.0;
const BOSS_HP_LOSS_P90: f32 = 60.0;

pub(in crate::ai::route_planner_v1) fn project_route_path_viability(
    path: &RouteWindowPath,
    current_hp: i32,
    belief: &UnknownRoomBeliefV1,
    config: &RoutePlannerConfigV1,
) -> RoutePathViabilityV1 {
    let mut cumulative_hp_loss_p90 = 0.0;
    let mut elite_included_before_recovery = false;
    let mut campfire_reached_before_elite = false;
    let mut shop_seen_before_segment_end = false;

    for node in &path.nodes {
        match node.room_type {
            Some(RoomType::RestRoom) => {
                campfire_reached_before_elite = true;
                break;
            }
            Some(RoomType::ShopRoom) => shop_seen_before_segment_end = true,
            Some(RoomType::MonsterRoom) => cumulative_hp_loss_p90 += HALLWAY_HP_LOSS_P90,
            Some(RoomType::MonsterRoomElite) => {
                cumulative_hp_loss_p90 += ELITE_HP_LOSS_P90;
                elite_included_before_recovery = true;
                break;
            }
            Some(RoomType::MonsterRoomBoss) => {
                cumulative_hp_loss_p90 += BOSS_HP_LOSS_P90;
                break;
            }
            Some(RoomType::EventRoom) => {
                cumulative_hp_loss_p90 += unknown_room_hp_loss_p90(belief, config);
            }
            _ => {}
        }
    }

    let projected_hp_after_segment = current_hp as f32 - cumulative_hp_loss_p90;
    RoutePathViabilityV1 {
        cumulative_hp_loss_p90,
        projected_hp_after_segment,
        elite_included_before_recovery,
        campfire_reached_before_elite,
        shop_seen_before_segment_end,
        survives_projected_segment: projected_hp_after_segment > 0.0,
    }
}

fn unknown_room_hp_loss_p90(belief: &UnknownRoomBeliefV1, config: &RoutePlannerConfigV1) -> f32 {
    (belief.monster_chance * config.base_monster_hp_loss
        + belief.elite_chance * config.base_elite_hp_loss)
        * 1.8
}

#[cfg(test)]
mod tests {
    use crate::ai::route_planner_v1::{RoutePlannerConfigV1, UnknownRoomBeliefV1};
    use crate::ai::route_window_facts::{RouteWindowNode, RouteWindowPath};
    use crate::state::map::node::RoomType;

    use super::project_route_path_viability;

    fn viability(current_hp: i32, rooms: &[RoomType]) -> super::RoutePathViabilityV1 {
        let path = RouteWindowPath {
            nodes: rooms
                .iter()
                .enumerate()
                .map(|(index, room_type)| RouteWindowNode {
                    x: 0,
                    y: index as i32,
                    room_type: Some(*room_type),
                })
                .collect(),
        };
        project_route_path_viability(
            &path,
            current_hp,
            &UnknownRoomBeliefV1 {
                monster_chance: 0.25,
                shop_chance: 0.10,
                treasure_chance: 0.05,
                event_chance: 0.60,
                elite_chance: 0.0,
                has_juzu_bracelet: false,
                has_tiny_chest: false,
                deadly_events: false,
            },
            &RoutePlannerConfigV1::default(),
        )
    }

    #[test]
    fn forced_damage_cannot_improve_projected_hp() {
        let direct = viability(44, &[RoomType::MonsterRoomElite]);
        let hallway_then_elite =
            viability(44, &[RoomType::MonsterRoom, RoomType::MonsterRoomElite]);

        assert!(hallway_then_elite.cumulative_hp_loss_p90 >= direct.cumulative_hp_loss_p90);
        assert!(hallway_then_elite.projected_hp_after_segment <= direct.projected_hp_after_segment);
    }

    #[test]
    fn raising_current_hp_cannot_reduce_viability() {
        let low = viability(30, &[RoomType::MonsterRoom, RoomType::MonsterRoomElite]);
        let high = viability(60, &[RoomType::MonsterRoom, RoomType::MonsterRoomElite]);

        assert!(!low.survives_projected_segment || high.survives_projected_segment);
        assert!(high.projected_hp_after_segment >= low.projected_hp_after_segment);
    }

    #[test]
    fn shop_is_liquidity_not_recovery() {
        let projection = viability(
            44,
            &[
                RoomType::MonsterRoom,
                RoomType::ShopRoom,
                RoomType::MonsterRoomElite,
            ],
        );

        assert_eq!(projection.cumulative_hp_loss_p90, 54.0);
        assert!(projection.shop_seen_before_segment_end);
        assert!(!projection.campfire_reached_before_elite);
        assert!(!projection.survives_projected_segment);
    }

    #[test]
    fn campfire_ends_the_current_danger_segment() {
        let projection = viability(
            44,
            &[
                RoomType::MonsterRoom,
                RoomType::RestRoom,
                RoomType::MonsterRoomElite,
            ],
        );

        assert_eq!(projection.cumulative_hp_loss_p90, 14.0);
        assert!(projection.campfire_reached_before_elite);
        assert!(!projection.elite_included_before_recovery);
        assert!(projection.survives_projected_segment);
    }
}
