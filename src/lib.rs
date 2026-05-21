extern crate self as sts_simulator;

pub mod content;
mod core;
pub mod engine;
mod events;
pub mod map;
pub mod runtime;
mod shop;
pub mod sim;
pub mod state;

// Backward-compatible crate-private path for existing monster content. The
// actual move-plan types live with runtime combat state now.
mod semantics {
    pub mod combat {
        pub use crate::runtime::monster_move::*;
    }
}

// Integration layers around the runtime path.
pub mod ai;
pub mod app;
pub mod eval;
mod testing;
pub use testing::fixtures;
pub use testing::support as test_support;

// Legacy compatibility paths. New code should use `ai`, `sim`, or `eval`
// directly; there is intentionally no `src/bot` implementation tree.
pub mod bot {
    pub mod combat {
        pub mod monster_belief {
            pub use crate::ai::combat_belief::*;
        }

        pub mod search_v2 {
            pub use crate::ai::combat_search_v2::*;
        }

        pub mod search_v2_eval {
            pub use crate::eval::combat_search_v2::*;
        }

        pub use crate::sim::combat_legal_actions::legal_moves_for_audit;
    }
}

pub use core::EntityId;
mod rewards;
mod utils;
pub use utils::SimulationWatchdog;
