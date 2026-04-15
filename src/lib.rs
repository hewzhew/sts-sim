pub mod content;
mod core;
mod deck;
pub mod engine;
mod events;
pub mod map;
mod shop;
pub mod state;
pub mod runtime;

// Integration layers around the runtime path.
pub mod diff;
mod testing;
pub use testing::fixtures;

// User-facing and experimental surfaces.
pub mod bot;
pub mod cli;

pub use core::EntityId;
pub use runtime::{action, combat, rng};
mod rewards;
mod utils;
pub use utils::SimulationWatchdog;
