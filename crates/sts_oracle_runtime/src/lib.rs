//! Branch execution and persistence over oracle evaluation and run-control.

extern crate self as sts_simulator;

pub use sts_oracle_eval::{
    ai, content, engine, eval, fixtures, sim, state, test_support, EntityId,
};

pub mod testing {
    pub use sts_oracle_eval::testing::*;
}

pub mod runtime;

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
