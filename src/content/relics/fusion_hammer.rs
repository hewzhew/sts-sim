//! Fusion Hammer has no combat hook.
//!
//! Java semantics:
//! - `onEquip`/`onUnequip` adjust `energyMaster`.
//! - `canUseCampfireOption` disables the normal `SmithOption` only.
//!
//! Rust handles those in `energy_master_delta` and
//! `campfire_handler::get_available_options`.
