mod agent;
pub(crate) mod card_disposition;
pub mod combat;
mod combat_families;
mod decision_meta;
pub(crate) mod deck;
pub mod facts;
pub mod harness;
pub(crate) mod infra;
pub(crate) mod potions;
pub mod snapshots;

pub(crate) use deck::card_taxonomy;
pub(crate) use facts::{card_facts, card_structure};
pub(crate) use snapshots::{deck_archetype, deck_profile};

pub use agent::Agent;
pub use combat::{branch_family_for_card, legal_moves_for_audit, BranchFamily};
pub use combat::{
    SearchEquivalenceKind, SearchEquivalenceMode, SearchNodeCounters, SearchPhaseProfile,
    SearchProfileBreakdown, SearchProfilingLevel,
};
pub use decision_meta::DecisionMetadata;
pub use deck_archetype::{archetype_summary, archetype_tags};
pub use deck_profile::{combat_zone_profile, deck_profile, DeckProfile};
pub use infra::coverage::{
    archetype_tags_for_combat, curiosity_bonus, curiosity_target_matches, novelty_bonus,
    CoverageDb, CoverageMode, CuriosityTarget,
};
