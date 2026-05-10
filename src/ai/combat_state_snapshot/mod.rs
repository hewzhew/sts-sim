//! AI-facing combat state snapshot schema.
//!
//! These types are an audit boundary, not a legacy engine wrapper. They are
//! shaped from `docs/AI_COMBAT_STATE_SCHEMA.md`: Java source defines combat
//! semantics, Rust owns the deterministic simulator representation.

use serde::{Deserialize, Serialize};

mod actions;
mod cards;
mod choices;
mod combatants;
mod context;
mod coverage;
mod derived;
mod lifecycle;
mod manifest;
mod monsters;
mod orbs;
mod potions;
mod powers;
mod public_refs;
mod refs;
mod relics;
mod rng;
mod stances;

pub use actions::*;
pub use cards::*;
pub use choices::*;
pub use combatants::*;
pub use context::*;
pub use coverage::*;
pub use derived::*;
pub use lifecycle::*;
pub use manifest::*;
pub use monsters::*;
pub use orbs::*;
pub use potions::*;
pub use powers::*;
pub use public_refs::*;
pub use refs::*;
pub use relics::*;
pub use rng::*;
pub use stances::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatStateSnapshot {
    pub source_manifest: SourceManifest,
    pub snapshot_origin: CombatSnapshotOrigin,
    pub dungeon_context: DungeonCombatContext,
    pub room_state: RoomCombatState,
    pub content_pools: CombatContentPoolState,
    pub global_temp: GlobalCombatTempState,
    pub action_manager: ActionManagerState,
    pub player: PlayerCombatState,
    pub monster_group: MonsterGroupState,
    pub card_store: CardInstanceStore,
    pub card_zones: CardZoneState,
    pub powers: PowerState,
    pub relics: RelicState,
    pub blights: BlightState,
    pub potions: PotionBeltState,
    pub orbs: OrbState,
    pub stance: StanceState,
    pub choice_screens: ChoiceScreenState,
    pub rng: CombatRngState,
    pub lifecycle: CombatLifecycleState,
    pub public_refs: PublicRefState,
    pub derived_values: DerivedCombatValues,
    pub source_coverage: Vec<SourceCoverageEntry>,
    pub migration_ledger: Vec<MigrationLedgerEntry>,
}
