mod agent;
pub mod boss_relic;
pub mod campfire;
pub(crate) mod card_disposition;
pub mod combat;
mod combat_families;
pub(crate) mod deck;
pub mod deck_ops;
pub mod event;
pub mod facts;
pub mod harness;
pub(crate) mod infra;
pub mod map;
mod policy_spine;
pub(crate) mod potions;
pub mod reward;
pub mod shared;
pub mod shop;
pub mod snapshots;

pub(crate) use deck::card_taxonomy;
pub(crate) use deck::noncombat_signals as noncombat_card_signals;
pub(crate) use deck::scoring as deck_scoring;
pub(crate) use facts::{card_facts, card_structure, upgrade_facts};
pub(crate) use snapshots::{deck_archetype, deck_profile};

pub use agent::Agent;
pub use boss_relic::{
    BossRelicCandidate, BossRelicDecisionDiagnostics, RelicCompatibility, RelicJudgement,
};
pub use campfire::{CampfireDecisionDiagnostics, CampfireOptionScore};
pub use combat::{branch_family_for_card, legal_moves_for_audit, BranchFamily};
pub use combat::{
    SearchEquivalenceKind, SearchEquivalenceMode, SearchNodeCounters, SearchPhaseProfile,
    SearchProfileBreakdown, SearchProfilingLevel,
};
pub use deck_archetype::{archetype_summary, archetype_tags};
pub use deck_ops::{DeckOperationKind, DeckOpsAssessment, DeckOpsCandidate};
pub use deck_profile::{combat_zone_profile, deck_profile, DeckProfile};
pub(crate) use deck_scoring::curse_remove_severity;
pub use deck_scoring::{score_card_offer, score_owned_card};
pub use event::{EventDecision, EventDecisionDiagnostics, EventOptionAssessment};
pub use infra::coverage::{
    archetype_tags_for_combat, curiosity_bonus, curiosity_target_matches, novelty_bonus,
    CoverageDb, CoverageMode, CuriosityTarget,
};
pub use map::{MapDecisionDiagnostics, MapOptionScore};
pub use policy_spine::{
    BlockedPotionOffer, BossRelicDecision, BotPolicyDecision, CampfireDecision, CombatDecision,
    CombatDecisionContext, DecisionDomain, DecisionMetadata, DeckImprovementDecision,
    DeckImprovementDecisionContext, EventDecisionPolicy, MapDecision, RewardCardDecision,
    RewardCardDecisionContext, RewardClaimDecision, RewardClaimDecisionContext, ShopDecision,
    ShopDecisionContext,
};
pub use reward::{
    RewardCardAction, RewardCardCandidate, RewardClaimAction, RewardClaimDiagnostics,
    RewardDecisionDiagnostics,
};
pub use reward::{
    RewardCardAction as RewardCardDecisionAction, RewardClaimAction as RewardClaimDecisionAction,
};
pub use shop::ShopAction as ShopDecisionAction;
pub use shop::{ShopAction, ShopDecisionDiagnostics, ShopOptionKind, ShopOptionScore};
