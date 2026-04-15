//! Protocol-facing parsing, mapping, and snapshot shaping.

#[path = "../mapper.rs"]
mod mapper;
#[path = "../parser.rs"]
mod parser;
mod snapshot;

pub use mapper::{
    card_id_from_java, intent_from_java, java_potion_id_to_rust, monster_id_from_java,
    power_id_from_java, power_instance_id_from_java, relic_id_from_java,
};
pub use parser::{CombatReplay, ReplayAction, ReplayData, parse_replay};
pub use snapshot::build_live_combat_snapshot;
