extern crate self as sts_simulator;

pub mod content;
mod core;
mod deck;
pub mod engine;
mod events;
pub mod map;
pub mod projection;
pub mod protocol;
pub mod runtime;
mod semantics;
mod shop;
pub mod state;
pub mod verification;

// Integration layers around the runtime path.
pub mod diff;
mod testing;
pub use testing::fixtures;
pub use testing::support as test_support;

// User-facing and experimental surfaces.
pub mod bot;
pub mod cli;

pub use core::EntityId;
mod rewards;
mod utils;
pub use utils::SimulationWatchdog;
