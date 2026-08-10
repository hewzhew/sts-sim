//! Combat evaluation and exact-search orchestration above the stable simulator and planner.

extern crate self as sts_simulator;

pub use sts_core::{content, engine, fixtures, sim, state, test_support, EntityId};

pub mod testing {
    pub use sts_core::fixtures;
    pub use sts_core::test_support as support;
}

pub mod ai {
    pub use sts_combat_legacy::ai::*;
}

pub mod runtime {
    pub use sts_core::runtime::{action, combat, monster_move, rng};
}

#[cfg(test)]
mod semantics {
    pub mod combat {
        pub use crate::runtime::monster_move::*;
    }
}

#[path = "../../../src/eval/mod.rs"]
pub mod eval;

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
    }
}
