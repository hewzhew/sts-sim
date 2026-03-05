//! Map Generation Module
//! 
//! Pitch-perfect copy of map generation algorithm from Slay the Spire.
//! Ported from https://github.com/Ru5ty0ne/sts_map_oracle
//!
//! Key algorithm details:
//! - Map dimensions: 15 floors (height) x 7 columns (width)
//! - Path density: 6 paths from floor 0
//! - Fixed rooms: Floor 0 = Monster, Floor 8 = Treasure, Floor 14 = Rest
//! - No Elite/Rest before floor 5
//! - No Rest on floor 13+
//! - Room probabilities: Shop 5%, Rest 12%, Elite 8% (or 12.8% for A1+), Event 22%

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::fmt;

// ============================================================================
// RNG - Exact copy of the game's Xoshiro128** variant
// ============================================================================

/// Murmur hash 3 finalizer - used to initialize the RNG state
fn murmur_hash3(mut x: u64) -> u64 {
    x ^= x >> 33;
    x = x.wrapping_mul(0xff51afd7ed558ccd);
    x ^= x >> 33;
    x = x.wrapping_mul(0xc4ceb9fe1a85ec53);
    x ^= x >> 33;
    x
}

/// Random number generator matching the game's implementation
#[derive(Debug, Clone)]
struct MapRandom {
    seed0: u64,
    seed1: u64,
}

impl MapRandom {
    fn new(seed: i64) -> Self {
        let mut seed = seed as u64;
        if seed == 0 {
            seed = i64::MIN as u64;
        }
        let seed0 = murmur_hash3(seed);
        let seed1 = murmur_hash3(seed0);
        Self { seed0, seed1 }
    }

    fn next_u64(&mut self) -> u64 {
        let mut s1 = self.seed0;
        let s0 = self.seed1;
        self.seed0 = s0;
        s1 ^= s1 << 23;
        self.seed1 = s1 ^ s0 ^ (s1 >> 17) ^ (s0 >> 26);
        s0.wrapping_add(self.seed1)
    }

    fn next_u64_capped(&mut self, n: u64) -> u64 {
        loop {
            let bits = self.next_u64() >> 1;
            let value = bits % n;
            if bits + n >= value + 1 {
                return value;
            }
        }
    }

    fn next_i32(&mut self, n: u64) -> i32 {
        self.next_u64_capped(n + 1) as i32
    }

    fn rand_range(&mut self, min: i32, max: i32) -> i32 {
        min + self.next_i32((max - min) as u64)
    }

    fn next_shuffle(&mut self, n: u64) -> usize {
        self.next_u64_capped(n) as usize
    }
}

// ============================================================================
// Room Types
// ============================================================================

/// Room types in the map
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoomType {
    Monster,
    MonsterElite,
    Rest,
    Shop,
    Treasure,
    Event,
    Boss,
}

impl fmt::Display for RoomType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = match self {
            RoomType::Monster => 'M',
            RoomType::MonsterElite => 'E',
            RoomType::Rest => 'R',
            RoomType::Shop => '$',
            RoomType::Treasure => 'T',
            RoomType::Event => '?',
            RoomType::Boss => 'B',
        };
        write!(f, "{}", c)
    }
}

// ============================================================================
// Map Structures
// ============================================================================

/// A point in the map grid
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Point {
    pub x: usize,
    pub y: usize,
}

/// An edge connecting two nodes
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MapEdge {
    pub src_x: i32,
    pub src_y: i32,
    pub dst_x: i32,
    pub dst_y: i32,
}

impl MapEdge {
    fn new(src_x: i32, src_y: i32, dst_x: i32, dst_y: i32) -> Self {
        Self { src_x, src_y, dst_x, dst_y }
    }
}

impl PartialOrd for MapEdge {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MapEdge {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.dst_x.cmp(&other.dst_x) {
            Ordering::Equal => self.dst_y.cmp(&other.dst_y),
            res => res,
        }
    }
}

/// A single node in the map
#[derive(Debug, Clone)]
pub struct MapRoomNode {
    pub x: i32,
    pub y: i32,
    pub room_type: Option<RoomType>,
    pub edges: BTreeSet<MapEdge>,
    pub parents: Vec<Point>,
}

impl PartialEq for MapRoomNode {
    fn eq(&self, other: &Self) -> bool {
        self.x == other.x && self.y == other.y
    }
}

impl Eq for MapRoomNode {}

impl MapRoomNode {
    fn new(x: i32, y: i32) -> Self {
        Self {
            x,
            y,
            edges: BTreeSet::new(),
            parents: Vec::new(),
            room_type: None,
        }
    }
}

/// The map grid type
pub type MapGrid = Vec<Vec<MapRoomNode>>;

/// Complete map structure with metadata
#[derive(Debug, Clone)]
pub struct Map {
    /// The grid of nodes (indexed by [y][x])
    pub nodes: MapGrid,
    /// Map width (typically 7)
    pub width: usize,
    /// Map height (typically 15)
    pub height: usize,
    /// Act number (1, 2, or 3)
    pub act: u8,
}

impl Map {
    /// Get the node at position (x, y)
    pub fn get(&self, x: usize, y: usize) -> &MapRoomNode {
        &self.nodes[y][x]
    }

    /// Get a mutable reference to the node at position (x, y)
    pub fn get_mut(&mut self, x: usize, y: usize) -> &mut MapRoomNode {
        &mut self.nodes[y][x]
    }

    /// Check if a node has connections (is part of a path)
    pub fn is_connected(&self, x: usize, y: usize) -> bool {
        let node = &self.nodes[y][x];
        !node.edges.is_empty() || !node.parents.is_empty()
    }

    /// Get all starting positions (floor 0 nodes with edges)
    pub fn get_starting_positions(&self) -> Vec<usize> {
        self.nodes[0]
            .iter()
            .enumerate()
            .filter(|(_, node)| !node.edges.is_empty())
            .map(|(x, _)| x)
            .collect()
    }

    /// Get children positions from a node at (x, y)
    pub fn get_children(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        self.nodes[y][x]
            .edges
            .iter()
            .map(|e| (e.dst_x as usize, e.dst_y as usize))
            .collect()
    }
}

// ============================================================================
// Map Generation Algorithm
// ============================================================================

/// Create an empty grid of nodes
fn create_nodes(height: i32, width: i32) -> MapGrid {
    (0..height)
        .map(|y| (0..width).map(|x| MapRoomNode::new(x, y)).collect())
        .collect()
}

/// Get common ancestor between two nodes (game bug preserved for compatibility)
fn get_common_ancestor<'a>(
    map: &'a MapGrid,
    node1: &'a Point,
    node2: &'a Point,
    max_depth: i32,
) -> Option<&'a Point> {
    assert!(node1.y == node2.y);
    assert!(node1.x != node2.x);

    let (mut l_node, mut r_node): (&Point, &Point);
    
    // Bug from game's original codebase: should be "node1.x < node2.x"
    // Retained for backwards compatibility
    if node1.x < node2.y {
        l_node = node1;
        r_node = node2;
    } else {
        l_node = node2;
        r_node = node1;
    }

    let mut current_y = node1.y as i32;
    while current_y >= 0 && current_y >= node1.y as i32 - max_depth {
        let l_parents = &map[l_node.y][l_node.x].parents;
        let r_parents = &map[r_node.y][r_node.x].parents;

        if l_parents.is_empty() || r_parents.is_empty() {
            return None;
        }

        l_node = l_parents.iter().max_by_key(|p| p.x).unwrap();
        r_node = r_parents.iter().min_by_key(|p| p.x).unwrap();

        if l_node == r_node {
            return Some(l_node);
        }
        current_y -= 1;
    }
    None
}

/// Recursively create paths through the map
fn create_paths_recursive(mut nodes: MapGrid, edge: &MapEdge, rng: &mut MapRandom) -> MapGrid {
    if edge.dst_y + 1 >= nodes.len() as i32 {
        return nodes;
    }

    let row_width = nodes[edge.dst_y as usize].len();
    let row_end_node = row_width - 1;

    // Determine min/max offset based on position
    let (min, max) = if edge.dst_x == 0 {
        (0, 1)
    } else if edge.dst_x == row_end_node as i32 {
        (-1, 0)
    } else {
        (-1, 1)
    };

    let mut new_edge_x = edge.dst_x + rng.rand_range(min, max);
    let new_edge_y = edge.dst_y + 1;

    let target_node_candidate = &nodes[new_edge_y as usize][new_edge_x as usize];
    let mut target_coord_candidate = Point {
        x: new_edge_x as usize,
        y: new_edge_y as usize,
    };

    let min_ancestor_gap = 3i32;
    let max_ancestor_gap = 5i32;
    let current_node_coord = Point {
        y: edge.dst_y as usize,
        x: edge.dst_x as usize,
    };

    // Check for path crossing with common ancestor
    for parent in target_node_candidate.parents.iter() {
        if &current_node_coord != parent {
            let ancestor = get_common_ancestor(&nodes, parent, &current_node_coord, max_ancestor_gap);
            if let Some(ancestor) = ancestor {
                let ancestor_gap = new_edge_y - ancestor.y as i32;
                if ancestor_gap < min_ancestor_gap {
                    match target_coord_candidate.x.cmp(&(edge.dst_x as usize)) {
                        Ordering::Greater => {
                            new_edge_x = edge.dst_x + rng.rand_range(-1, 0);
                            if new_edge_x < 0 {
                                new_edge_x = edge.dst_x;
                            }
                        }
                        Ordering::Equal => {
                            new_edge_x = edge.dst_x + rng.rand_range(-1, 1);
                            if new_edge_x > row_end_node as i32 {
                                new_edge_x = edge.dst_x - 1;
                            } else if new_edge_x < 0 {
                                new_edge_x = edge.dst_x + 1;
                            }
                        }
                        Ordering::Less => {
                            new_edge_x = edge.dst_x + rng.rand_range(0, 1);
                            if new_edge_x > row_end_node as i32 {
                                new_edge_x = edge.dst_x;
                            }
                        }
                    }
                    target_coord_candidate = Point {
                        x: new_edge_x as usize,
                        y: new_edge_y as usize,
                    };
                    continue;
                }
            }
        }
    }

    // Eliminate edge crosses with neighbors
    if edge.dst_x != 0 {
        let left_node = &nodes[edge.dst_y as usize][(edge.dst_x - 1) as usize];
        if let Some(right_edge_of_left_node) = left_node.edges.iter().last() {
            if right_edge_of_left_node.dst_x > new_edge_x {
                new_edge_x = right_edge_of_left_node.dst_x;
            }
        }
    }
    if edge.dst_x < row_end_node as i32 {
        let right_node = &nodes[edge.dst_y as usize][(edge.dst_x + 1) as usize];
        if let Some(left_edge_of_right_node) = right_node.edges.iter().next() {
            if left_edge_of_right_node.dst_x < new_edge_x {
                new_edge_x = left_edge_of_right_node.dst_x;
            }
        }
    }

    // Add the edge
    let new_edge = MapEdge::new(edge.dst_x, edge.dst_y, new_edge_x, new_edge_y);
    let copy_edge = new_edge.clone();
    nodes[edge.dst_y as usize][edge.dst_x as usize].edges.insert(new_edge);

    // Add parent reference
    nodes[new_edge_y as usize][new_edge_x as usize].parents.push(Point {
        x: edge.dst_x as usize,
        y: edge.dst_y as usize,
    });

    create_paths_recursive(nodes, &copy_edge, rng)
}

/// Create paths from the bottom of the map
fn create_paths(mut nodes: MapGrid, path_density: i32, rng: &mut MapRandom) -> MapGrid {
    assert!(!nodes.is_empty());
    assert!(!nodes[0].is_empty());

    let row_size = (nodes[0].len() - 1) as i32;
    let mut first_starting_node = -1i32;

    for i in 0..path_density {
        let mut starting_node = rng.rand_range(0, row_size);
        if i == 0 {
            first_starting_node = starting_node;
        }
        // Ensure second path doesn't start at same position as first
        while starting_node == first_starting_node && i == 1 {
            starting_node = rng.rand_range(0, row_size);
        }
        let tmp_edge = MapEdge::new(starting_node, -1, starting_node, 0);
        nodes = create_paths_recursive(nodes, &tmp_edge, rng);
    }
    nodes
}

/// Filter redundant edges with common destination on first floor
fn filter_redundant_edges(mut map: MapGrid) -> MapGrid {
    let mut existing_edges: HashSet<Point> = HashSet::new();
    let mut delete_list: Vec<(usize, MapEdge)> = Vec::new();

    for (i, node) in map[0].iter().enumerate() {
        for edge in node.edges.iter() {
            for prev in existing_edges.iter() {
                if prev.x == edge.dst_x as usize && prev.y == edge.dst_y as usize {
                    delete_list.push((i, edge.clone()));
                }
            }
            existing_edges.insert(Point {
                x: edge.dst_x as usize,
                y: edge.dst_y as usize,
            });
        }
    }

    for (idx, edge) in delete_list {
        map[0][idx].edges.remove(&edge);
    }

    map
}

/// Generate the dungeon structure (paths only, no room types)
fn generate_dungeon(height: i32, width: i32, path_density: i32, rng: &mut MapRandom) -> MapGrid {
    let map = create_nodes(height, width);
    let map = create_paths(map, path_density, rng);
    filter_redundant_edges(map)
}

// ============================================================================
// Room Assignment
// ============================================================================

/// Shuffle a vector using the RNG
fn shuffle<T>(list: &mut Vec<T>, rng: &mut MapRandom) {
    for i in (2..=list.len()).rev() {
        let tmp = rng.next_shuffle(i as u64);
        list.swap(tmp, i - 1);
    }
}

/// Generate room type list based on probabilities
fn generate_room_type(
    room_chances: &HashMap<RoomType, f64>,
    available_room_count: usize,
) -> Vec<RoomType> {
    let mut acc = Vec::new();
    
    // Order matters for determinism: Shop, Rest, Elite, Event
    let rooms_type_order = [RoomType::Shop, RoomType::Rest, RoomType::MonsterElite, RoomType::Event];
    
    for room_type in rooms_type_order.iter() {
        let chance = room_chances.get(room_type).unwrap_or(&0.0);
        let rooms = (chance * available_room_count as f64).round() as usize;
        for _ in 0..rooms {
            acc.push(*room_type);
        }
    }
    
    // Remaining rooms are Monster rooms
    acc
}

/// Check if a room type can be assigned to a given row
fn rule_assignable_to_row(node: &MapRoomNode, room: &RoomType) -> bool {
    let applicable_rooms = [RoomType::Rest, RoomType::MonsterElite];
    
    // No Elite or Rest before floor 5 (y <= 4)
    if node.y <= 4 && applicable_rooms.contains(room) {
        return false;
    }
    
    // No Rest on floor 13+
    if node.y >= 13 && *room == RoomType::Rest {
        return false;
    }
    
    true
}

/// Check if parent has same room type (forbidden for certain types)
fn rule_parent_matches(map: &MapGrid, parents: &[Point], room: &RoomType) -> bool {
    let applicable_rooms = [RoomType::Rest, RoomType::Treasure, RoomType::Shop, RoomType::MonsterElite];
    
    applicable_rooms.contains(room)
        && parents.iter()
            .filter_map(|parent| map[parent.y][parent.x].room_type)
            .any(|class| class == *room)
}

/// Get sibling nodes (nodes that share a parent)
fn get_siblings<'a>(map: &'a MapGrid, node: &'a MapRoomNode) -> Vec<&'a MapRoomNode> {
    node.parents
        .iter()
        .flat_map(|parent| map[parent.y][parent.x].edges.iter())
        .map(|edge| &map[edge.dst_y as usize][edge.dst_x as usize])
        .filter(|sib_node| *sib_node != node)
        .collect()
}

/// Check if sibling has same room type (forbidden for certain types)
fn rule_sibling_matches(sibs: &[&MapRoomNode], room: &RoomType) -> bool {
    let applicable_rooms = [
        RoomType::Rest,
        RoomType::Treasure,
        RoomType::Shop,
        RoomType::MonsterElite,
        RoomType::Monster,
        RoomType::Event,
    ];
    
    applicable_rooms.contains(room)
        && sibs.iter()
            .filter_map(|sib| sib.room_type)
            .any(|class| class == *room)
}

/// Get the next valid room type for a node
fn get_next_room_type(map: &MapGrid, node: &MapRoomNode, room_list: &[RoomType]) -> Option<RoomType> {
    let parents = &node.parents;
    let siblings = get_siblings(map, node);
    
    for room in room_list.iter() {
        if rule_assignable_to_row(node, room) {
            if !rule_parent_matches(map, parents, room) && !rule_sibling_matches(&siblings, room) {
                return Some(*room);
            }
            // Floor 0 always succeeds
            if node.y == 0 {
                return Some(*room);
            }
        }
    }
    None
}

/// Assign rooms to connected nodes
fn assign_rooms_to_nodes(mut map: MapGrid, room_list: &mut Vec<RoomType>) -> MapGrid {
    let height = map.len();
    let width = map[0].len();
    
    for y in 0..height {
        for x in 0..width {
            let node = &map[y][x];
            if !node.edges.is_empty() && node.room_type.is_none() {
                if let Some(room_to_be_set) = get_next_room_type(&map, node, room_list) {
                    if let Some(pos) = room_list.iter().position(|&r| r == room_to_be_set) {
                        room_list.remove(pos);
                        map[y][x].room_type = Some(room_to_be_set);
                    }
                }
            }
        }
    }
    map
}

/// Fill any remaining empty nodes with Monster rooms
fn last_minute_node_checker(mut map: MapGrid) -> MapGrid {
    for row in map.iter_mut() {
        for node in row.iter_mut() {
            if !node.edges.is_empty() && node.room_type.is_none() {
                node.room_type = Some(RoomType::Monster);
            }
        }
    }
    map
}

/// Count nodes that need room assignment
fn count_connected_nodes(map: &MapGrid) -> usize {
    let map_size = map.len();
    map.iter()
        .flat_map(|row| row.iter())
        .filter(|node| {
            (!node.edges.is_empty() || (node.y as usize == map_size - 1 && !node.parents.is_empty()))
                && node.y as usize != map_size - 2  // Skip treasure floor
        })
        .count()
}

/// Distribute rooms across the map
fn distribute_rooms_across_map(
    mut map: MapGrid,
    mut room_list: Vec<RoomType>,
    rng: &mut MapRandom,
) -> MapGrid {
    let node_count = count_connected_nodes(&map);
    room_list.resize(std::cmp::max(room_list.len(), node_count), RoomType::Monster);
    shuffle(&mut room_list, rng);
    map = assign_rooms_to_nodes(map, &mut room_list);
    map = last_minute_node_checker(map);
    map
}

// ============================================================================
// Public API
// ============================================================================

/// Generate a map for a specific act
/// 
/// # Arguments
/// * `seed` - The game seed
/// * `act` - The act number (1, 2, or 3)
/// * `is_ascension_zero` - If true, use A0 elite chances (8%), otherwise A1+ (12.8%)
pub fn generate_map(seed: i64, act: u8, is_ascension_zero: bool) -> Map {
    const ACT_SEEDS: [i64; 3] = [1, 200, 600];
    const MAP_HEIGHT: i32 = 15;
    const MAP_WIDTH: i32 = 7;
    const PATH_DENSITY: i32 = 6;

    let act_idx = (act.saturating_sub(1) as usize).min(2);
    let act_seed = ACT_SEEDS[act_idx];
    
    let mut rng = MapRandom::new(seed + act_seed);
    let mut map = generate_dungeon(MAP_HEIGHT, MAP_WIDTH, PATH_DENSITY, &mut rng);
    
    // Count nodes for room distribution
    let count = map.iter()
        .flat_map(|row| row.iter())
        .filter(|n| {
            (!n.edges.is_empty() || (n.y as usize == map.len() - 1 && !n.parents.is_empty()))
                && n.y as usize != map.len() - 2
        })
        .count();
    
    // Fixed room assignments
    // Floor 0: Monster rooms
    map[0].iter_mut().for_each(|node| node.room_type = Some(RoomType::Monster));
    
    // Floor 8: Treasure rooms
    map[8].iter_mut().for_each(|node| node.room_type = Some(RoomType::Treasure));
    
    // Floor 14: Rest rooms (before boss)
    let map_size = map.len();
    map[map_size - 1].iter_mut().for_each(|node| node.room_type = Some(RoomType::Rest));
    
    // Room probabilities
    let elite_chance = if is_ascension_zero { 0.08 } else { 0.08 * 1.6 };
    let room_chances: HashMap<RoomType, f64> = [
        (RoomType::Shop, 0.05),
        (RoomType::Rest, 0.12),
        (RoomType::Event, 0.22),
        (RoomType::MonsterElite, elite_chance),
    ]
    .iter()
    .cloned()
    .collect();
    
    let room_list = generate_room_type(&room_chances, count);
    let nodes = distribute_rooms_across_map(map, room_list, &mut rng);
    
    Map {
        nodes,
        width: MAP_WIDTH as usize,
        height: MAP_HEIGHT as usize,
        act,
    }
}

/// Generate all three act maps
pub fn generate_all_maps(seed: i64, is_ascension_zero: bool) -> Vec<Map> {
    (1..=3).map(|act| generate_map(seed, act, is_ascension_zero)).collect()
}

// ============================================================================
// Display Implementation
// ============================================================================

fn get_room_symbol(t: &Option<RoomType>) -> &'static str {
    match t {
        None => " ",
        Some(RoomType::Monster) => "M",
        Some(RoomType::MonsterElite) => "E",
        Some(RoomType::Rest) => "R",
        Some(RoomType::Shop) => "$",
        Some(RoomType::Treasure) => "T",
        Some(RoomType::Event) => "?",
        Some(RoomType::Boss) => "B",
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\nAct {}", self.act)?;
        
        for row_num in (0..self.nodes.len()).rev() {
            // Print edges (connectors)
            write!(f, "{: <6}", "")?;
            for node in self.nodes[row_num].iter() {
                let (mut left, mut mid, mut right) = (" ", " ", " ");
                for edge in node.edges.iter() {
                    match edge.dst_x.cmp(&node.x) {
                        Ordering::Equal => mid = "|",
                        Ordering::Less => left = "\\",
                        Ordering::Greater => right = "/",
                    }
                }
                write!(f, "{}{}{}", left, mid, right)?;
            }
            writeln!(f)?;
            
            // Print nodes
            write!(f, "{: <6}", row_num)?;
            for node in self.nodes[row_num].iter() {
                let node_symbol = if row_num == self.nodes.len() - 1 {
                    // Top row: check if any lower node points to this
                    let mut symbol = " ";
                    for lower_node in self.nodes[row_num - 1].iter() {
                        for edge in lower_node.edges.iter() {
                            if edge.dst_x == node.x {
                                symbol = get_room_symbol(&node.room_type);
                            }
                        }
                    }
                    symbol
                } else if !node.edges.is_empty() {
                    get_room_symbol(&node.room_type)
                } else {
                    " "
                };
                write!(f, " {} ", node_symbol)?;
            }
            writeln!(f)?;
        }
        
        Ok(())
    }
}

// ============================================================================
// Simplified Map for GameState
// ============================================================================

/// A simplified node for use in GameState
#[derive(Debug, Clone)]
pub struct SimpleMapNode {
    pub x: usize,
    pub y: usize,
    pub room_type: RoomType,
    /// Indices of child nodes (can travel to from this node)
    pub children: Vec<usize>,
    /// Indices of parent nodes (came from)
    pub parents: Vec<usize>,
}

/// A simplified map structure for use in GameState
#[derive(Debug, Clone)]
pub struct SimpleMap {
    pub nodes: Vec<SimpleMapNode>,
    pub width: usize,
    pub height: usize,
    pub act: u8,
    /// Boss node index (if any)
    pub boss_node: Option<usize>,
}

impl SimpleMap {
    /// Convert a full Map to a SimpleMap
    pub fn from_map(map: &Map) -> Self {
        let mut nodes = Vec::new();
        let mut coord_to_idx: HashMap<(usize, usize), usize> = HashMap::new();
        
        // First pass: collect all connected nodes
        for y in 0..map.height {
            for x in 0..map.width {
                let node = &map.nodes[y][x];
                // Include nodes with edges OR top-floor nodes with parents
                let is_connected = !node.edges.is_empty() 
                    || (y == map.height - 1 && !node.parents.is_empty());
                
                if is_connected {
                    if let Some(room_type) = node.room_type {
                        let idx = nodes.len();
                        coord_to_idx.insert((x, y), idx);
                        nodes.push(SimpleMapNode {
                            x,
                            y,
                            room_type,
                            children: Vec::new(),
                            parents: Vec::new(),
                        });
                    }
                }
            }
        }
        
        // Second pass: link children and parents
        for y in 0..map.height {
            for x in 0..map.width {
                let node = &map.nodes[y][x];
                if let Some(&src_idx) = coord_to_idx.get(&(x, y)) {
                    // Add children
                    for edge in node.edges.iter() {
                        if let Some(&dst_idx) = coord_to_idx.get(&(edge.dst_x as usize, edge.dst_y as usize)) {
                            nodes[src_idx].children.push(dst_idx);
                            nodes[dst_idx].parents.push(src_idx);
                        }
                    }
                }
            }
        }
        
        // Third pass: Add Boss node at floor 15 (y = map.height)
        // All floor 14 (Rest) nodes connect to the Boss
        let boss_idx = nodes.len();
        let top_floor = map.height - 1; // floor 14
        
        // Collect all top-floor node indices
        let top_floor_indices: Vec<usize> = nodes.iter()
            .enumerate()
            .filter(|(_, n)| n.y == top_floor)
            .map(|(i, _)| i)
            .collect();
        
        // Create Boss node at floor 15, center column
        let boss_x = map.width / 2;
        nodes.push(SimpleMapNode {
            x: boss_x,
            y: map.height, // floor 15
            room_type: RoomType::Boss,
            children: Vec::new(),
            parents: top_floor_indices.clone(),
        });
        
        // Link all top-floor nodes to boss
        for &idx in &top_floor_indices {
            nodes[idx].children.push(boss_idx);
        }
        
        SimpleMap {
            nodes,
            width: map.width,
            height: map.height + 1, // Include boss floor
            act: map.act,
            boss_node: Some(boss_idx),
        }
    }

    /// Get starting node indices (floor 0)
    pub fn get_starting_positions(&self) -> Vec<usize> {
        self.nodes.iter()
            .enumerate()
            .filter(|(_, n)| n.y == 0)
            .map(|(i, _)| i)
            .collect()
    }

    /// Get node by index
    pub fn get(&self, idx: usize) -> Option<&SimpleMapNode> {
        self.nodes.get(idx)
    }
}

impl fmt::Display for SimpleMap {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "\nAct {} (SimpleMap: {} nodes)", self.act, self.nodes.len())?;
        
        // Include boss floor (height includes the boss floor now)
        for y in (0..self.height).rev() {
            write!(f, "{:>2}  ", y)?;
            for x in 0..self.width {
                let node = self.nodes.iter().find(|n| n.x == x && n.y == y);
                match node {
                    Some(n) => write!(f, " {} ", n.room_type)?,
                    None => write!(f, " . ")?,
                }
            }
            writeln!(f)?;
        }
        
        Ok(())
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_generation_deterministic() {
        let map1 = generate_map(12345, 1, true);
        let map2 = generate_map(12345, 1, true);
        
        // Same seed should produce same map
        for y in 0..map1.height {
            for x in 0..map1.width {
                assert_eq!(
                    map1.nodes[y][x].room_type,
                    map2.nodes[y][x].room_type,
                    "Mismatch at ({}, {})",
                    x, y
                );
            }
        }
    }

    #[test]
    fn test_floor_constraints() {
        let map = generate_map(42, 1, true);
        
        // Floor 0 should be all Monster
        for node in &map.nodes[0] {
            if !node.edges.is_empty() {
                assert_eq!(node.room_type, Some(RoomType::Monster));
            }
        }
        
        // Floor 8 should be all Treasure
        for node in &map.nodes[8] {
            if !node.edges.is_empty() || !node.parents.is_empty() {
                assert_eq!(node.room_type, Some(RoomType::Treasure));
            }
        }
        
        // Floor 14 should be all Rest
        for node in &map.nodes[14] {
            if !node.parents.is_empty() {
                assert_eq!(node.room_type, Some(RoomType::Rest));
            }
        }
        
        // No Elite/Rest before floor 5
        for y in 0..5 {
            for node in &map.nodes[y] {
                if let Some(room) = node.room_type {
                    assert_ne!(room, RoomType::MonsterElite, "Elite found on floor {}", y);
                    assert_ne!(room, RoomType::Rest, "Rest found on floor {}", y);
                }
            }
        }
    }

    #[test]
    fn test_simple_map_conversion() {
        let map = generate_map(12345, 1, true);
        let simple = SimpleMap::from_map(&map);
        
        // Should have starting positions
        let starts = simple.get_starting_positions();
        assert!(!starts.is_empty(), "Map should have starting positions");
        
        // All starting positions should be floor 0
        for &idx in &starts {
            let node = simple.get(idx).unwrap();
            assert_eq!(node.y, 0);
            assert_eq!(node.room_type, RoomType::Monster);
        }
    }

    #[test]
    fn test_boss_node_added() {
        let map = generate_map(42, 1, true);
        let simple = SimpleMap::from_map(&map);
        
        // Should have a boss node
        assert!(simple.boss_node.is_some(), "SimpleMap should have a boss node");
        
        let boss_idx = simple.boss_node.unwrap();
        let boss_node = simple.get(boss_idx).unwrap();
        
        // Boss should be at floor 15
        assert_eq!(boss_node.y, 15, "Boss should be at floor 15");
        assert_eq!(boss_node.room_type, RoomType::Boss, "Boss node should have Boss room type");
        
        // Boss should have parents (the rest rooms on floor 14)
        assert!(!boss_node.parents.is_empty(), "Boss should have parent nodes");
        
        // All rest nodes (floor 14) should have boss as their only child
        for &parent_idx in &boss_node.parents {
            let parent = simple.get(parent_idx).unwrap();
            assert_eq!(parent.y, 14, "Boss parents should be on floor 14");
            assert_eq!(parent.room_type, RoomType::Rest, "Boss parents should be Rest rooms");
            assert!(parent.children.contains(&boss_idx), "Rest rooms should connect to boss");
        }
    }

    #[test]
    fn test_all_acts() {
        let maps = generate_all_maps(12345, true);
        assert_eq!(maps.len(), 3);
        assert_eq!(maps[0].act, 1);
        assert_eq!(maps[1].act, 2);
        assert_eq!(maps[2].act, 3);
    }
}
