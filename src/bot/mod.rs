mod agent;
pub mod combat;
pub(crate) mod deck;
pub mod facts;
pub mod harness;
pub mod snapshots;

pub(crate) use facts::{card_facts, card_structure};
pub(crate) use snapshots::{deck_archetype, deck_profile};

pub use agent::Agent;
pub use combat::{branch_family_for_card, legal_moves_for_audit, BranchFamily};
pub use combat::{
    SearchEquivalenceKind, SearchEquivalenceMode, SearchNodeCounters, SearchPhaseProfile,
    SearchProfileBreakdown, SearchProfilingLevel,
};
pub use deck_archetype::{archetype_summary, archetype_tags};
pub use deck_profile::{combat_zone_profile, deck_profile, DeckProfile};
