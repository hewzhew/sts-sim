pub mod action;
pub mod combat;
pub mod content;
pub mod core;
pub mod deck;
pub mod engine;
pub mod events;
pub mod map;
pub mod rewards;
pub mod rng;
pub mod shop;
pub mod state;

// Integration layers around the runtime path.
pub mod diff;
pub mod testing;

// User-facing and experimental surfaces.
pub mod bot;
pub mod cli;
pub mod interaction_coverage;

pub mod utils;
