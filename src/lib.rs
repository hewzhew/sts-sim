pub mod action;
pub mod combat;
pub mod content;
mod core;
mod deck;
pub mod engine;
mod events;
pub mod map;
pub mod rewards;
pub mod rng;
mod shop;
pub mod state;

// Integration layers around the runtime path.
pub mod diff;
pub mod testing;

// User-facing and experimental surfaces.
pub mod bot;
pub mod cli;
pub mod interaction_coverage;

pub mod utils;

pub use core::EntityId;
