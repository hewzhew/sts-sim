//! Analysis layer.
//!
//! This layer converts domain facts and a concrete run state into profiles:
//! deck shape, startup, block plans, boss pressure, and similar observations.
//! It may diagnose debt or support, but it must not own final scene decisions.

pub mod block_profile;
pub mod boss_profile;
pub mod card_semantics;
pub mod deck_shape;
pub mod startup_profile;
