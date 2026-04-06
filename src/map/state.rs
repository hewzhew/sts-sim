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
    /// If `has_flight` is true, allows traveling to any node at y > current_y (WingBoots),
    /// not just adjacent rows — matching Java's MapRoomNode.wingedIsConnected().
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

        // WingBoots flight: allow traveling to any valid node above current position
        // Java: MapRoomNode.wingedIsConnected() — allows non-adjacent y as long as node exists
        if has_flight && target_y > self.current_y && target_y <= 14 {
            if (target_y as usize) < self.graph.len() && (target_x as usize) < self.graph[target_y as usize].len() {
                let target_node = &self.graph[target_y as usize][target_x as usize];
                // Node must have edges or be a valid room (not an empty/disconnected node)
                if !target_node.edges.is_empty() || target_node.class.is_some() {
                    return true;
                }
            }
        }
        
        // Handling Boss Node Phase 
        if target_y == 15 && self.current_y == 14 {
            return true;
        }

        false
    }

    pub fn travel_to(&mut self, target_x: i32, target_y: i32, has_flight: bool) -> Result<(), &'static str> {
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
        if self.current_y >= 0 && self.current_x >= 0 && (self.current_y as usize) < self.graph.len() {
            let row = &self.graph[self.current_y as usize];
            if (self.current_x as usize) < row.len() {
                return Some(&row[self.current_x as usize]);
            }
        }
        None
    }
}
