pub mod action;
pub mod combat;
pub mod content;
mod core;
mod deck;
pub mod engine;
mod events;
pub mod map;
pub mod rng;
mod shop;
pub mod state;

// Integration layers around the runtime path.
pub mod diff;
mod testing;
pub use testing::fixtures;

// User-facing and experimental surfaces.
pub mod bot;
pub mod cli;
pub mod interaction_coverage;
pub mod interaction_signatures;

pub use core::EntityId;
mod rewards;
mod utils;
pub use utils::SimulationWatchdog;
