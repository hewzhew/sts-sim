//! Deprecated compatibility shims for legacy `bot::combat` paths.

pub mod monster_belief;

pub mod search_v2 {
    pub use crate::ai::combat_search_v2::*;
}

pub mod search_v2_eval {
    pub use crate::eval::combat_search_v2::*;
}

pub use crate::sim::combat_legal_actions::legal_moves_for_audit;
