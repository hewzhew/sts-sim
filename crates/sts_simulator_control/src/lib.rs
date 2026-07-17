extern crate self as sts_simulator;

// Search, evaluation, run-control, and their binaries intentionally share one
// upper-layer compilation unit; the simulator domain remains a cached input.
pub use sts_core::{content, engine, fixtures, sim, state, test_support, EntityId};

// Compatibility for the control sources while their physical files move into
// this package. Test fixture construction remains owned by the core package.
pub mod testing {
    pub use sts_core::fixtures;
    pub use sts_core::test_support as support;
}

// Compatibility for scenario tests that still use the pre-runtime move path.
#[cfg(test)]
mod semantics {
    pub mod combat {
        pub use crate::runtime::monster_move::*;
    }
}

pub mod ai;
#[path = "../../../src/eval/mod.rs"]
pub mod eval;
pub mod runtime;

// Compatibility paths retained inside the control package while callers move
// to the explicit workspace boundary.
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
