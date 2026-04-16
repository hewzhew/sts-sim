mod agent;
mod agent_curiosity;
mod agent_deck_ops;
pub(crate) mod card_knowledge;
pub(crate) mod card_disposition;
pub(crate) mod card_taxonomy;
mod combat_heuristic;
pub(crate) mod combat_posture;
pub(crate) mod comm_mod;
mod coverage;
pub(crate) mod coverage_signatures;
mod deck_delta_eval;
pub(crate) mod encounter_suite;
mod evaluator;
mod event_policy;
pub mod harness;
pub(crate) mod monster_belief;
pub(crate) mod noncombat_families;
pub(crate) mod potions;
mod policy_spine;
mod reward_heuristics;
pub(crate) mod run_deck_improvement;
pub(crate) mod run_rule_context;
pub mod search;
pub(crate) mod sidecar;
mod strategy_families;

pub use agent::Agent;
pub use combat_heuristic::{
    describe_end_turn_options, diagnose_decision, evaluate_combat_state, HeuristicDiagnostics,
};
pub use coverage::{
    archetype_tags_for_combat, curiosity_bonus, curiosity_target_matches, novelty_bonus,
    CoverageDb, CoverageMode, CuriosityTarget,
};
pub use deck_delta_eval::{compare_pick_vs_skip, DeltaScore};
pub use evaluator::{evaluate_state, CardEvaluator, DeckProfile};
pub use event_policy::{
    choose_event_option, choose_live_event_choice, choose_local_event_choice,
    compact_choice_summary, decision_trace_json, describe_choice, live_event_context,
    local_event_context, EventChoiceDecision, EventDecisionContext, EventDecisionFeatures,
    EventOptionPayload, EventOptionScore, EventOptionTag, EventOptionView, EventPolicyFamily,
};
pub use policy_spine::{
    BlockedPotionOffer, BotPolicyDecision, CombatDecision, CombatDecisionContext, DecisionDomain,
    DecisionMetadata, EventDecision, RewardCardDecision, RewardCardDecisionAction,
    RewardCardDecisionContext, RewardClaimDecision, RewardClaimDecisionAction,
    RewardClaimDecisionContext, ShopDecision, ShopDecisionAction, ShopDecisionContext,
};
pub use reward_heuristics::{
    evaluate_reward_screen, evaluate_reward_screen_for_run, evaluate_reward_screen_for_run_detailed,
    pick_probability, RewardCardScore, RewardScreenEvaluation,
};
pub use strategy_families::{branch_family_for_card, BranchFamily};
