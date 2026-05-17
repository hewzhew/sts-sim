use super::node::{Map, RoomType};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct MapState {
    pub graph: Map,
    /// current_y = -1 indicates the player has not selected a starting node at the bottom yet.
    /// current_y = 14 indicates fighting the Boss.
    pub current_y: i32,
    pub current_x: i32,
    pub boss_node_available: bool,
    pub has_emerald_key: bool,
}

impl MapState {
    pub fn new(graph: Map) -> Self {
        Self {
            graph,
            current_y: -1,
            current_x: -1,
            boss_node_available: false,
            has_emerald_key: false, // In a real run, this is injected from the Act / Run manager
        }
    }

    /// Checks if the player can legally travel to the target point based on edges.
    /// If `has_flight` is true, allows traveling to any valid node on the next
    /// row reached by the current node's outgoing edges, not to arbitrary later
    /// rows.
    ///
    /// Java: `MapRoomNode.wingedIsConnectedTo()` checks `node.y == edge.dstY`
    /// and ignores `dstX` while Winged Greaves has charges.
    pub fn can_travel_to(&self, target_x: i32, target_y: i32, has_flight: bool) -> bool {
        // Handle initial map entry at y=0
        if self.current_y == -1 {
            if target_y == 0 && target_x >= 0 && (target_x as usize) < self.graph[0].len() {
                // You can enter any node at the bottom row (y=0) that natively has an edge or parent
                return !self.graph[0][target_x as usize].edges.is_empty();
            }
            return false;
        }

        // Handle normal edge traversal
        if target_y == self.current_y + 1 {
            if self.current_y >= 0 && (self.current_y as usize) < self.graph.len() {
                let current_node = &self.graph[self.current_y as usize][self.current_x as usize];
                for edge in current_node.edges.iter() {
                    if edge.dst_x == target_x && edge.dst_y == target_y {
                        return true;
                    }
                }
            }
        }

        // WingBoots flight: allow any valid node on a row the current node can
        // already reach vertically. Java compares only the target row to each
        // outgoing edge's dstY; it does not allow skipping multiple rows.
        if has_flight
            && self.current_y >= 0
            && (self.current_y as usize) < self.graph.len()
            && target_y == self.current_y + 1
            && (target_y as usize) < self.graph.len()
            && target_x >= 0
            && (target_x as usize) < self.graph[target_y as usize].len()
        {
            let current_node = &self.graph[self.current_y as usize][self.current_x as usize];
            let target_node = &self.graph[target_y as usize][target_x as usize];
            if current_node.edges.iter().any(|edge| edge.dst_y == target_y)
                && (!target_node.edges.is_empty() || target_node.class.is_some())
            {
                return true;
            }
        }

        // Handling Boss Node Phase
        if target_y == 15 && self.current_y == 14 {
            return true;
        }

        false
    }

    pub fn travel_to(
        &mut self,
        target_x: i32,
        target_y: i32,
        has_flight: bool,
    ) -> Result<(), &'static str> {
        if !self.can_travel_to(target_x, target_y, has_flight) {
            return Err("Invalid map traversal path");
        }
        self.current_y = target_y;
        self.current_x = target_x;
        Ok(())
    }

    pub fn get_current_room_type(&self) -> Option<RoomType> {
        if self.current_y == 15 {
            return Some(RoomType::MonsterRoomBoss);
        }
        self.get_current_node().and_then(|n| n.class)
    }

    pub fn get_current_node(&self) -> Option<&super::node::MapRoomNode> {
        if self.current_y >= 0
            && self.current_x >= 0
            && (self.current_y as usize) < self.graph.len()
        {
            let row = &self.graph[self.current_y as usize];
            if (self.current_x as usize) < row.len() {
                return Some(&row[self.current_x as usize]);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::node::{MapEdge, MapRoomNode, RoomType};

    fn node(x: i32, y: i32, class: Option<RoomType>) -> MapRoomNode {
        let mut node = MapRoomNode::new(x, y);
        node.class = class;
        node
    }

    #[test]
    fn wing_boots_matches_java_next_row_only_semantics() {
        let mut start = node(0, 0, Some(RoomType::MonsterRoom));
        start.edges.insert(MapEdge::new(0, 0, 0, 1));

        let graph = vec![
            vec![start, node(1, 0, None)],
            vec![
                node(0, 1, Some(RoomType::MonsterRoom)),
                node(1, 1, Some(RoomType::ShopRoom)),
            ],
            vec![
                node(0, 2, Some(RoomType::RestRoom)),
                node(1, 2, Some(RoomType::MonsterRoom)),
            ],
        ];
        let mut map = MapState::new(graph);
        map.current_x = 0;
        map.current_y = 0;

        assert!(map.can_travel_to(0, 1, false));
        assert!(!map.can_travel_to(1, 1, false));
        assert!(
            map.can_travel_to(1, 1, true),
            "Java Winged Greaves ignores dstX but keeps the target on an outgoing edge row"
        );
        assert!(
            !map.can_travel_to(0, 2, true),
            "Java Winged Greaves does not skip arbitrary future rows"
        );
    }
}
