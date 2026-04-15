//! App-layer bot harnesses and evaluation workbenches.

mod boss_validation;
pub mod combat_env;
pub mod combat_lab;
mod combat_policy;

pub use boss_validation::{build_ledger_record, validate_case};
pub use combat_policy::PolicyKind;
