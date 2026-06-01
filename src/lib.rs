extern crate self as sts_simulator;

pub mod content;
mod core;
pub mod engine;
pub mod runtime;
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
pub mod eval;
mod testing;
pub use testing::fixtures;
pub use testing::support as test_support;

// Crate-private compatibility for Java-content modules that still use the old
// reward paths. The active ownership is `state::rewards` plus engine handlers;
// there is intentionally no `src/rewards` implementation tree.
mod rewards {
    pub mod generator {
        pub use crate::state::rewards::generator::*;
    }

    #[cfg(test)]
    pub mod handler {
        pub use crate::engine::reward_handler::*;
    }

    pub mod state {
        pub use crate::state::rewards::*;
    }
}

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
