use crate::bot::agent::Agent;
use crate::state::core::ClientInput;
use crate::state::run::RunState;

impl Agent {
    pub(crate) fn decide_map(&mut self, rs: &RunState) -> ClientInput {
        if rs.map.current_y < 0 {
            self.map_path = self.compute_map_path_with_target(rs, self.active_curiosity_target());
            let archetypes = crate::bot::evaluator::CardEvaluator::archetype_tags(
                &crate::bot::evaluator::CardEvaluator::deck_profile(rs),
            );
            eprintln!(
                "  [BOT] Computed map path: {:?} | Archetypes: {:?}",
                self.map_path, archetypes
            );
        }

        let path_idx = (rs.map.current_y + 1) as usize;
        if path_idx < self.map_path.len() {
            let target_x = self.map_path[path_idx];
            let next_y = rs.map.current_y + 1;
            if rs.map.can_travel_to(target_x, next_y, false) {
                ClientInput::SelectMapNode(target_x as usize)
            } else {
                for x in 0..7 {
                    if rs.map.can_travel_to(x, next_y, false) {
                        return ClientInput::SelectMapNode(x as usize);
                    }
                }
                ClientInput::SelectMapNode(0)
            }
        } else {
            let next_y = rs.map.current_y + 1;
            for x in 0..7 {
                if rs.map.can_travel_to(x, next_y, false) {
                    return ClientInput::SelectMapNode(x as usize);
                }
            }
            ClientInput::SelectMapNode(0)
        }
    }

    pub(crate) fn compute_map_path_with_target(
        &self,
        rs: &RunState,
        curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    ) -> Vec<i32> {
        let graph = &rs.map.graph;
        let weights = self.map_room_weights(rs, curiosity_target);

        let mut paths_a: Vec<(Vec<i32>, i32)> = vec![(vec![], 0); 7];
        let mut paths_b: Vec<(Vec<i32>, i32)> = vec![(vec![], 0); 7];

        if !graph.is_empty() {
            for x in 0..7 {
                if x < graph[0].len() {
                    let node = &graph[0][x];
                    if !node.edges.is_empty() {
                        let w = node
                            .class
                            .map(|rt| weights[Self::room_type_to_weight_index(rt)])
                            .unwrap_or(0);
                        paths_a[x] = (vec![x as i32], w);
                    }
                }
            }
        }

        let max_y = graph.len().min(15);
        for y in 0..max_y.saturating_sub(1) {
            for slot in paths_b.iter_mut().take(7) {
                *slot = (vec![], 0);
            }

            for x in 0..7 {
                if x >= graph[y].len() {
                    continue;
                }
                let node = &graph[y][x];
                if node.edges.is_empty() {
                    continue;
                }
                let cur_path = &paths_a[x];

                for edge in &node.edges {
                    let next_x = edge.dst_x as usize;
                    let next_y = edge.dst_y as usize;
                    if next_y >= graph.len() || next_x >= graph[next_y].len() {
                        continue;
                    }

                    let next_node = &graph[next_y][next_x];
                    let room_w = next_node
                        .class
                        .map(|rt| weights[Self::room_type_to_weight_index(rt)])
                        .unwrap_or(0);
                    let new_weight = cur_path.1 + room_w;

                    let dest = &paths_b[next_x];
                    if dest.0.len() < cur_path.0.len() + 1 || dest.1 < new_weight {
                        let mut new_route = cur_path.0.clone();
                        new_route.push(next_x as i32);
                        paths_b[next_x] = (new_route, new_weight);
                    }
                }
            }

            std::mem::swap(&mut paths_a, &mut paths_b);
        }

        let mut best_x = 0;
        let mut best_weight = i32::MIN;
        for (x, path) in paths_a.iter().enumerate().take(7) {
            if path.1 > best_weight && !path.0.is_empty() {
                best_weight = path.1;
                best_x = x;
            }
        }

        let mut route = paths_a[best_x].0.clone();
        route.push(0);
        route
    }

    pub(crate) fn room_type_to_weight_index(rt: crate::map::node::RoomType) -> usize {
        use crate::map::node::RoomType;
        match rt {
            RoomType::ShopRoom => 0,
            RoomType::RestRoom => 1,
            RoomType::EventRoom => 2,
            RoomType::MonsterRoomElite => 3,
            RoomType::MonsterRoom => 4,
            RoomType::TreasureRoom => 5,
            _ => 4,
        }
    }

    pub(crate) fn map_room_weights(
        &self,
        rs: &RunState,
        curiosity_target: Option<&crate::bot::coverage::CuriosityTarget>,
    ) -> [i32; 6] {
        let act_idx = ((rs.act_num as usize).saturating_sub(1)).min(2);
        let mut weights: [i32; 6] = match act_idx {
            0 => [100, 1000, 100, 10, 1, 0],
            1 => [10, 1000, 10, 100, 1, 0],
            _ => [100, 1000, 100, 1, 10, 0],
        };
        let need = self.build_noncombat_need_snapshot(rs);

        weights[0] += need.purge_value / 8 + need.survival_pressure / 10;
        weights[1] += need.survival_pressure / 2 + need.best_upgrade_value / 5
            - need.long_term_meta_value / 6
            + need.key_urgency / 6;
        weights[2] += need.purge_value / 12 + need.long_term_meta_value / 8
            - need.survival_pressure / 10;
        weights[3] += need.long_term_meta_value / 7 + need.best_upgrade_value / 10
            - need.survival_pressure / 4
            - need.key_urgency / 5;
        weights[4] += need.best_upgrade_value / 12 + need.long_term_meta_value / 10
            - need.survival_pressure / 8;
        weights[5] += 12 + need.long_term_meta_value / 12;

        if need.route.nearby_recovery_windows == 0 {
            weights[1] += 45;
            weights[0] += 25;
        }
        if need.route.upcoming_elite_pressure > 0 && need.survival_pressure < need.best_upgrade_value {
            weights[3] += 35;
        }

        if let Some(crate::bot::coverage::CuriosityTarget::Archetype(target)) = curiosity_target {
            let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
            let target = Self::normalize_lookup_name(target);
            let target_online = crate::bot::evaluator::CardEvaluator::archetype_tags(&profile)
                .iter()
                .any(|tag| Self::normalize_lookup_name(tag) == target);
            if !target_online {
                weights[0] += 45;
                weights[2] += 35;
                weights[3] -= 35;
                if target == "block" {
                    weights[1] += 25;
                }
            } else {
                weights[3] += 30;
            }
        }

        weights
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bot::Agent;
    use crate::content::cards::CardId;
    use crate::map::node::{Map, MapEdge, MapRoomNode, RoomType};
    use crate::map::state::MapState;

    #[test]
    fn map_room_weights_respond_to_shared_need_snapshot() {
        let agent = Agent::new();
        let mut safe = RunState::new(13, 0, true, "Ironclad");
        safe.current_hp = 70;
        safe.max_hp = 80;
        safe.map = linear_map_state(
            &[
                RoomType::MonsterRoom,
                RoomType::ShopRoom,
                RoomType::RestRoom,
            ],
            0,
        );

        let mut pressured = safe.clone();
        pressured.current_hp = 18;
        pressured.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            13_001,
        ));
        pressured.map = linear_map_state(
            &[
                RoomType::MonsterRoomElite,
                RoomType::MonsterRoom,
                RoomType::RestRoom,
            ],
            0,
        );

        let safe_weights = agent.map_room_weights(&safe, None);
        let pressured_weights = agent.map_room_weights(&pressured, None);

        assert!(pressured_weights[1] > safe_weights[1]);
        assert!(pressured_weights[0] >= safe_weights[0]);
    }

    fn linear_map_state(rooms: &[RoomType], current_y: i32) -> MapState {
        let mut graph: Map = Vec::new();
        for (y, room_type) in rooms.iter().enumerate() {
            let mut node = MapRoomNode::new(0, y as i32);
            node.class = Some(*room_type);
            if y + 1 < rooms.len() {
                node.edges.insert(MapEdge::new(0, y as i32, 0, y as i32 + 1));
            }
            graph.push(vec![node]);
        }
        let mut map = MapState::new(graph);
        map.current_x = 0;
        map.current_y = current_y;
        map
    }
}
