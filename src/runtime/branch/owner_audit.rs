#![allow(dead_code)]

use std::path::PathBuf;

#[path = "owner_audit/accepted_combat_attrition.rs"]
mod accepted_combat_attrition;
#[path = "owner_audit/accepted_high_loss_diagnostic.rs"]
mod accepted_high_loss_diagnostic;
#[path = "owner_audit/boss_relic_owner.rs"]
mod boss_relic_owner;
#[path = "owner_audit/boundary_router.rs"]
mod boundary_router;
#[path = "owner_audit/branch_frontier.rs"]
mod branch_frontier;
#[path = "owner_audit/branch_generation.rs"]
mod branch_generation;
#[path = "owner_audit/branch_generation_step.rs"]
mod branch_generation_step;
#[path = "owner_audit/branch_model.rs"]
mod branch_model;
#[path = "owner_audit/branch_observer.rs"]
mod branch_observer;
#[path = "owner_audit/branch_path.rs"]
mod branch_path;
#[path = "owner_audit/branch_policy_lane.rs"]
mod branch_policy_lane;
#[path = "owner_audit/branch_runtime.rs"]
mod branch_runtime;
#[path = "owner_audit/branch_scheduler.rs"]
mod branch_scheduler;
#[path = "owner_audit/branch_status_view.rs"]
mod branch_status_view;
#[path = "owner_audit/branch_trajectory.rs"]
mod branch_trajectory;
#[path = "owner_audit/campfire_owner.rs"]
mod campfire_owner;
#[path = "owner_audit/candidate_ir_adapter.rs"]
mod candidate_ir_adapter;
#[path = "owner_audit/capsule_artifact_store.rs"]
mod capsule_artifact_store;
#[path = "owner_audit/card_reward_owner.rs"]
mod card_reward_owner;
#[path = "owner_audit/cli_args.rs"]
mod cli_args;
#[path = "owner_audit/combat_gap_case.rs"]
mod combat_gap_case;
#[path = "owner_audit/combat_search_orchestrator.rs"]
mod combat_search_orchestrator;
#[path = "owner_audit/combat_search_report.rs"]
mod combat_search_report;
#[path = "owner_audit/combat_search_session_json.rs"]
mod combat_search_session_json;
#[path = "owner_audit/combat_search_session_output.rs"]
mod combat_search_session_output;
#[path = "owner_audit/combat_search_session_plan.rs"]
mod combat_search_session_plan;
#[path = "owner_audit/combat_search_session_result.rs"]
mod combat_search_session_result;
#[path = "owner_audit/combat_search_survival.rs"]
mod combat_search_survival;
#[path = "owner_audit/combat_search_trace_actions.rs"]
mod combat_search_trace_actions;
#[path = "owner_audit/decision_delta.rs"]
mod decision_delta;
#[cfg(test)]
#[path = "owner_audit/event_owner_boundaries.rs"]
mod event_owner_boundaries;
#[path = "owner_audit/event_owner_bridge.rs"]
mod event_owner_bridge;
#[path = "owner_audit/event_owner_probe.rs"]
mod event_owner_probe;
#[path = "owner_audit/expansion_policy.rs"]
mod expansion_policy;
#[path = "owner_audit/frontier_checkpoint.rs"]
mod frontier_checkpoint;
#[path = "owner_audit/neow_owner.rs"]
mod neow_owner;
#[path = "owner_audit/owner_choice_expander.rs"]
mod owner_choice_expander;
#[path = "owner_audit/owner_commands.rs"]
mod owner_commands;
#[path = "owner_audit/owner_model.rs"]
mod owner_model;
#[path = "owner_audit/owner_orchestrator.rs"]
mod owner_orchestrator;
#[path = "owner_audit/owner_routines.rs"]
mod owner_routines;
#[path = "owner_audit/owners.rs"]
mod owners;
#[path = "owner_audit/policy_expansion_plan.rs"]
mod policy_expansion_plan;
#[path = "owner_audit/primary_search_outcome.rs"]
mod primary_search_outcome;
#[path = "owner_audit/render.rs"]
mod render;
#[path = "owner_audit/render_choice.rs"]
mod render_choice;
#[path = "owner_audit/reward_tiny_owner.rs"]
mod reward_tiny_owner;
#[path = "owner_audit/run_capsule.rs"]
mod run_capsule;
#[path = "owner_audit/run_capsule_format.rs"]
mod run_capsule_format;
#[path = "owner_audit/run_capsule_io.rs"]
mod run_capsule_io;
#[path = "owner_audit/run_chain.rs"]
mod run_chain;
#[path = "owner_audit/run_chain_state.rs"]
mod run_chain_state;
#[path = "owner_audit/run_choice_owner.rs"]
mod run_choice_owner;
#[path = "owner_audit/run_contract.rs"]
mod run_contract;
#[path = "owner_audit/run_cutpoint.rs"]
mod run_cutpoint;
#[path = "owner_audit/run_cutpoint_recorder.rs"]
mod run_cutpoint_recorder;
#[path = "owner_audit/run_cutpoint_store.rs"]
mod run_cutpoint_store;
#[path = "owner_audit/run_deadline.rs"]
mod run_deadline;
#[path = "owner_audit/run_identity.rs"]
mod run_identity;
#[path = "owner_audit/run_loop.rs"]
mod run_loop;
#[path = "owner_audit/run_persistence.rs"]
mod run_persistence;
#[path = "owner_audit/run_slice_request.rs"]
mod run_slice_request;
#[path = "owner_audit/run_slice_result.rs"]
mod run_slice_result;
#[path = "owner_audit/run_startup.rs"]
mod run_startup;
#[path = "owner_audit/run_state_json.rs"]
mod run_state_json;
#[path = "owner_audit/run_stop_recorder.rs"]
mod run_stop_recorder;
#[path = "owner_audit/runner.rs"]
mod runner;
#[path = "owner_audit/search_comparability.rs"]
mod search_comparability;
#[path = "owner_audit/shop_route_evidence.rs"]
mod shop_route_evidence;
#[path = "owner_audit/shop_tiny_owner.rs"]
mod shop_tiny_owner;
#[path = "owner_audit/trace.rs"]
mod trace;
#[path = "owner_audit/trace_format.rs"]
mod trace_format;
#[path = "owner_audit/trajectory_artifact_store.rs"]
mod trajectory_artifact_store;
#[path = "owner_audit/trajectory_evidence_store.rs"]
mod trajectory_evidence_store;
#[path = "owner_audit/trajectory_projector.rs"]
mod trajectory_projector;
#[path = "owner_audit/trajectory_snapshot.rs"]
mod trajectory_snapshot;

use branch_model::{BoundarySite, Branch, BranchStatus, Owner, TerminalOutcome};
use cli_args::{Args, ArgsOverrides, ContinueCapsuleArgs, EventOwnerProbeArgs};
use run_slice_request::ContinueSliceRequest;

use super::RunSliceResult;

pub struct OwnerAuditRuntime;

pub struct OwnerAuditSliceRequest {
    pub args: super::Args,
    pub capsule_path: PathBuf,
    pub resume: bool,
    pub human_output: bool,
}

impl OwnerAuditRuntime {
    pub fn run_cli() -> Result<(), String> {
        let context = match run_startup::prepare()? {
            run_startup::RunStartup::Delegated => return Ok(()),
            run_startup::RunStartup::Ready(context) => context,
        };
        branch_runtime::BranchRuntime::run_slice(context).map(|_| ())
    }

    pub fn run_capsule_slice(request: OwnerAuditSliceRequest) -> Result<RunSliceResult, String> {
        let slice = ContinueSliceRequest {
            args: request.args,
            overrides: ArgsOverrides::default(),
            capsule_path: request.capsule_path,
            resume: request.resume,
            human_output: request.human_output,
        }
        .prepare()?;
        branch_runtime::BranchRuntime::run_slice(slice)
    }
}

/// Reconstructs the strategic facts captured when the production combat owner
/// opens an exact combat. Callers must first restore and validate the exact
/// production session; this helper never infers context from display text.
pub fn reconstruct_oracle_combat_context_trace_v1(
    session: &sts_simulator::eval::run_control::RunControlSession,
) -> Result<sts_simulator::eval::run_control::CombatSearchTraceSummary, String> {
    let active_combat = session
        .active_combat
        .as_ref()
        .ok_or_else(|| "oracle combat context reconstruction requires active combat".to_string())?;
    let potion_continuation_context =
        sts_simulator::ai::potion_continuation_context_v1::potion_run_continuation_context_v1(
            &session.run_state,
            &active_combat.combat_state,
        );
    let potion_continuation_pressure =
        sts_simulator::ai::potion_continuation_pressure_v1::potion_continuation_pressure_v1(
            &session.run_state,
            &potion_continuation_context,
        );
    let combat_victory_continuation = sts_simulator::eval::run_control::
        CombatVictoryContinuationFactsV1::from_guaranteed_room_boss_full_heal(
            sts_simulator::eval::run_control::strategic_combat_victory_reaches_full_heal_v1(
                session,
            ),
        );
    let (entry_current_hp, entry_max_hp) = session.visible_player_hp();
    let strategic_hp_quality =
        sts_simulator::eval::run_control::CombatSearchStrategicHpQualityFactsV1::from_owner_limits(
            entry_current_hp,
            entry_max_hp,
            combat_search_survival::owner_audit_hp_loss_limit(session),
            combat_search_survival::owner_audit_search_quality_loss_target(session),
        );

    Ok(sts_simulator::eval::run_control::CombatSearchTraceSummary {
        source: "reconstructed_exact_production_context".to_string(),
        potion_continuation_context: Some(potion_continuation_context),
        potion_continuation_pressure: Some(potion_continuation_pressure),
        combat_victory_continuation: Some(combat_victory_continuation),
        strategic_hp_quality: Some(strategic_hp_quality),
        ..sts_simulator::eval::run_control::CombatSearchTraceSummary::default()
    })
}

/// Adapts the current production owners into a complete positive-support
/// policy prior. The exact decision model still owns legality and successor
/// construction; this legacy adapter only guides exploration order.
pub(super) fn legacy_oracle_policy_prior_v1(
    session: &sts_simulator::eval::run_control::RunControlSession,
    legal: &[sts_simulator::eval::run_control::RunPolicyCandidateV1<'_>],
) -> Result<sts_simulator::eval::run_control::RunPolicyPriorV1, String> {
    let immediate_run_potion_ids = legal
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.action,
                sts_simulator::eval::run_control::RunDecisionAction::Input(
                    sts_simulator::state::core::ClientInput::UsePotion { target: None, .. }
                )
            )
        })
        .map(|candidate| candidate.candidate_id.to_string())
        .collect::<Vec<_>>();
    if !immediate_run_potion_ids.is_empty() {
        return sts_simulator::eval::run_control::positive_ranked_run_policy_prior_v1(
            legal,
            immediate_run_potion_ids,
        );
    }
    if matches!(
        session.engine_state,
        sts_simulator::state::core::EngineState::Shop(_)
    ) {
        return sts_simulator::eval::run_control::exact_shop_policy_prior_v1(session, legal);
    }
    if matches!(
        session.engine_state,
        sts_simulator::state::core::EngineState::Campfire
    ) {
        return sts_simulator::eval::run_control::exact_campfire_policy_prior_v1(session, legal);
    }
    if matches!(
        session.engine_state,
        sts_simulator::state::core::EngineState::BossRelicSelect(_)
    ) {
        return sts_simulator::eval::run_control::exact_boss_relic_policy_prior_v1(session, legal);
    }
    if session.engine_state.is_map_surface() {
        return sts_simulator::eval::run_control::exact_route_policy_prior_v1(session, legal);
    }
    let card_reward_ids = sts_simulator::eval::run_control::build_decision_surface(session)
        .view
        .candidates
        .into_iter()
        .filter(|candidate| {
            candidate.action.executable_action_ref().is_some()
                && matches!(
                    candidate.key,
                    Some(
                        sts_simulator::eval::run_control::DecisionCandidateKey::CardRewardPick {
                            ..
                        } | sts_simulator::eval::run_control::DecisionCandidateKey::CardRewardOpen {
                            ..
                        } | sts_simulator::eval::run_control::DecisionCandidateKey::CardRewardSingingBowl {
                            ..
                        } | sts_simulator::eval::run_control::DecisionCandidateKey::CardRewardSkip {
                            ..
                        }
                    )
                )
        })
        .map(|candidate| candidate.id)
        .collect::<std::collections::BTreeSet<_>>();
    if !card_reward_ids.is_empty()
        && card_reward_ids.iter().all(|candidate_id| {
            legal
                .iter()
                .any(|candidate| candidate.candidate_id == candidate_id.as_str())
        })
    {
        return sts_simulator::eval::run_control::exact_card_reward_policy_prior_v1(session, legal);
    }
    sts_simulator::eval::run_control::positive_ranked_run_policy_prior_v1(
        legal,
        legacy_oracle_preferred_candidate_ids_v1(session),
    )
}

fn legacy_oracle_preferred_candidate_ids_v1(
    session: &sts_simulator::eval::run_control::RunControlSession,
) -> Vec<String> {
    use owner_model::{OwnerDecision, OwnerRoutine};
    use sts_simulator::eval::run_control::build_decision_surface;
    use sts_simulator::state::core::EngineState;

    let mut trial = session.clone();
    if matches!(
        &trial.engine_state,
        EngineState::MapNavigation | EngineState::MapOverlay { .. }
    ) {
        return trial
            .apply_route_plan()
            .ok()
            .and_then(|outcome| {
                outcome
                    .single_decision_transaction()
                    .map(|transaction| transaction.selection.candidate_id.clone())
            })
            .into_iter()
            .collect();
    }

    let Some(owner) = boundary_router::owner_for_current_boundary(&trial) else {
        return Vec::new();
    };
    let surface = build_decision_surface(&trial);
    match owners::owner_decision(&trial, owner, &surface) {
        OwnerDecision::Candidates(choices) => choices
            .into_iter()
            .map(|choice| choice.candidate_id)
            .collect(),
        OwnerDecision::Routine(OwnerRoutine::Candidate { candidate_id, .. }) => {
            vec![candidate_id]
        }
        OwnerDecision::Routine(OwnerRoutine::RewardPolicyStep) => {
            owner_routines::apply_owner_routine(&mut trial, OwnerRoutine::RewardPolicyStep)
                .ok()
                .and_then(|outcome| {
                    outcome
                        .single_decision_transaction()
                        .map(|transaction| transaction.selection.candidate_id.clone())
                })
                .into_iter()
                .collect()
        }
        OwnerDecision::Routine(OwnerRoutine::ForcedTransition(_)) => Vec::new(),
        OwnerDecision::Gap(_) => Vec::new(),
    }
}

/// Public read-only view of the exact owner ordering used by oracle run
/// exploration. Diagnostic tools use this to compare a saved witness with the
/// current production policy without advancing or mutating that witness.
pub fn current_oracle_candidate_order_v1(
    session: &sts_simulator::eval::run_control::RunControlSession,
) -> Vec<String> {
    use sts_simulator::eval::run_control::{build_decision_surface, RunPolicyCandidateV1};

    let surface = build_decision_surface(session);
    let legal = surface
        .view
        .candidates
        .iter()
        .filter_map(|candidate| {
            candidate
                .action
                .executable_action_ref()
                .map(|action| RunPolicyCandidateV1 {
                    candidate_id: &candidate.id,
                    label: &candidate.label,
                    action,
                })
        })
        .collect::<Vec<_>>();
    if legal.is_empty() {
        return Vec::new();
    }
    legacy_oracle_policy_prior_v1(session, &legal)
        .map(|prior| {
            prior
                .entries
                .into_iter()
                .map(|entry| entry.candidate_id)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::{default_branch_args, ArtifactKind, RunSliceRequestKind};

    #[test]
    fn owner_audit_runtime_exposes_cli_entrypoint() {
        let _entrypoint: fn() -> Result<(), String> = OwnerAuditRuntime::run_cli;
    }

    #[test]
    fn legacy_oracle_adapter_covers_the_complete_exact_surface() {
        use sts_simulator::eval::run_control::{
            build_decision_surface, RunControlConfig, RunControlSession, RunPolicyCandidateV1,
        };
        use sts_simulator::state::core::EngineState;
        use sts_simulator::state::shop::ShopState;

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::Shop(ShopState::new());
        let surface = build_decision_surface(&session);
        let legal = surface
            .view
            .candidates
            .iter()
            .filter_map(|candidate| {
                Some(RunPolicyCandidateV1 {
                    candidate_id: &candidate.id,
                    label: &candidate.label,
                    action: candidate.action.executable_action_ref()?,
                })
            })
            .collect::<Vec<_>>();

        let prior =
            legacy_oracle_policy_prior_v1(&session, &legal).expect("complete legacy policy prior");

        prior.validate_for(&legal).expect("valid policy prior");
        assert_eq!(prior.entries.len(), legal.len());
        assert!(prior.entries.iter().all(|entry| entry.probability > 0.0));
    }

    #[test]
    fn policy_audit_uses_the_same_complete_route_prior_as_production() {
        use sts_simulator::eval::run_control::{
            build_decision_surface, RunControlConfig, RunControlSession,
        };
        use sts_simulator::state::core::EngineState;
        use sts_simulator::state::map::node::{MapEdge, MapRoomNode, RoomType};
        use sts_simulator::state::map::state::MapState;

        let mut left = MapRoomNode::new(0, 0);
        left.class = Some(RoomType::MonsterRoom);
        left.edges.insert(MapEdge::new(0, 0, 0, 1));
        let mut right = MapRoomNode::new(1, 0);
        right.class = Some(RoomType::MonsterRoomElite);
        right.edges.insert(MapEdge::new(1, 0, 1, 1));
        let mut left_next = MapRoomNode::new(0, 1);
        left_next.class = Some(RoomType::RestRoom);
        let mut right_next = MapRoomNode::new(1, 1);
        right_next.class = Some(RoomType::RestRoom);

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.run_state.map = MapState::new(vec![vec![left, right], vec![left_next, right_next]]);
        session.engine_state = EngineState::MapNavigation;

        let surface = build_decision_surface(&session);
        let legal_ids = surface
            .view
            .candidates
            .iter()
            .filter(|candidate| candidate.action.executable_action_ref().is_some())
            .map(|candidate| candidate.id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        let audited = current_oracle_candidate_order_v1(&session)
            .into_iter()
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(audited, legal_ids);
        assert_eq!(audited.len(), 2);
    }

    #[test]
    fn current_owner_realizes_fruit_juice_before_map_travel() {
        use sts_simulator::content::potions::{Potion, PotionId};
        use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};
        use sts_simulator::state::core::EngineState;

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::MapNavigation;
        session.run_state.potions = vec![Some(Potion::new(PotionId::FruitJuice, 10))];

        let ordered = current_oracle_candidate_order_v1(&session);

        assert_eq!(
            ordered.first().map(String::as_str),
            Some("use-run-potion-0")
        );
    }

    #[test]
    fn current_owner_replaces_covered_fear_with_strength_reward() {
        use sts_simulator::content::cards::CardId;
        use sts_simulator::content::potions::{Potion, PotionId};
        use sts_simulator::eval::run_control::{RunControlConfig, RunControlSession};
        use sts_simulator::runtime::combat::CombatCard;
        use sts_simulator::state::core::EngineState;
        use sts_simulator::state::rewards::{RewardItem, RewardState};

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.potions = vec![
            Some(Potion::new(PotionId::AncientPotion, 1)),
            Some(Potion::new(PotionId::FearPotion, 2)),
            Some(Potion::new(PotionId::AttackPotion, 3)),
        ];
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::Bash, 101),
            CombatCard::new(CardId::SwordBoomerang, 102),
        ];
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Potion {
            potion_id: PotionId::StrengthPotion,
        }];
        session.engine_state = EngineState::RewardScreen(reward);

        let ordered = current_oracle_candidate_order_v1(&session);

        assert_eq!(
            ordered.first().map(String::as_str),
            Some("discard-potion-1")
        );
    }

    #[test]
    fn oracle_adapter_composes_card_prior_with_the_full_reward_surface() {
        use sts_simulator::content::cards::CardId;
        use sts_simulator::eval::run_control::{
            build_decision_surface, DecisionCandidateKey, RunControlConfig, RunControlSession,
            RunPolicyCandidateV1,
        };
        use sts_simulator::state::core::EngineState;
        use sts_simulator::state::rewards::{RewardCard, RewardItem, RewardState};

        let mut session = RunControlSession::new(RunControlConfig::default());
        let cards = vec![
            RewardCard::new(CardId::BattleTrance, 1),
            RewardCard::new(CardId::WildStrike, 0),
        ];
        let mut reward = RewardState::new();
        reward.items = vec![RewardItem::Card {
            cards: cards.clone(),
        }];
        reward.pending_card_choice = Some(cards);
        reward.pending_card_reward_index = Some(0);
        session.engine_state = EngineState::RewardScreen(reward);

        let surface = build_decision_surface(&session);
        let legal = surface
            .view
            .candidates
            .iter()
            .filter_map(|candidate| {
                Some(RunPolicyCandidateV1 {
                    candidate_id: &candidate.id,
                    label: &candidate.label,
                    action: candidate.action.executable_action_ref()?,
                })
            })
            .collect::<Vec<_>>();
        let prior =
            legacy_oracle_policy_prior_v1(&session, &legal).expect("composed card reward prior");
        prior.validate_for(&legal).expect("valid composed prior");

        let rank = |predicate: fn(&DecisionCandidateKey) -> bool| {
            let candidate_id = surface
                .view
                .candidates
                .iter()
                .find(|candidate| candidate.key.as_ref().is_some_and(predicate))
                .map(|candidate| candidate.id.as_str())
                .expect("typed reward candidate");
            prior
                .entries
                .iter()
                .position(|entry| entry.candidate_id == candidate_id)
                .expect("prior entry")
        };
        assert!(
            rank(|key| matches!(
                key,
                DecisionCandidateKey::CardRewardPick {
                    card: CardId::BattleTrance,
                    ..
                }
            )) < rank(|key| matches!(key, DecisionCandidateKey::CardRewardSkip { .. }))
        );
        assert_eq!(prior.entries.len(), legal.len());
        assert!(prior.entries.iter().all(|entry| entry.probability > 0.0));
    }

    #[test]
    fn owner_audit_runtime_runs_one_capsule_slice_in_process() {
        let root = std::env::temp_dir().join("owner_audit_runtime_start_slice");
        let _ = std::fs::remove_dir_all(&root);
        let mut args = default_branch_args(123);
        args.generations = 0;
        args.max_branches = 1;
        args.search_nodes = 1;
        args.search_ms = 1;
        args.rescue_search_nodes = 1;
        args.rescue_search_ms = 1;
        args.boss_search_nodes = 1;
        args.boss_search_ms = 1;
        // This contract verifies capsule persistence, not deadline handling.
        // A wall deadline makes the assertion depend on unrelated parallel
        // test load and may stop before the frontier is committed.
        args.wall_ms = None;

        let result = OwnerAuditRuntime::run_capsule_slice(OwnerAuditSliceRequest {
            args,
            capsule_path: root.clone(),
            resume: false,
            human_output: false,
        })
        .unwrap();

        assert_eq!(result.request_kind, RunSliceRequestKind::Start);
        assert!(result.artifacts.manifest_written);
        assert!(result.artifacts.frontier_written);
        assert!(result.artifacts.summary_written);
        assert!(result.artifacts.trajectory_projection_written);
        assert!(!result.artifacts.result_written);
        assert_eq!(
            result
                .artifacts
                .manifest_ref
                .as_ref()
                .map(|artifact| artifact.kind),
            Some(ArtifactKind::Manifest)
        );
        assert_eq!(
            result
                .artifacts
                .frontier_ref
                .as_ref()
                .map(|artifact| artifact.kind),
            Some(ArtifactKind::Frontier)
        );
        assert_eq!(
            result
                .artifacts
                .summary_ref
                .as_ref()
                .map(|artifact| artifact.kind),
            Some(ArtifactKind::Summary)
        );
        assert!(result
            .artifacts
            .frontier_ref
            .as_ref()
            .unwrap()
            .path
            .ends_with("frontier.json"));
        assert!(root.join("manifest.json").exists());
        assert!(root.join("frontier.json").exists());
        assert!(root.join("summary.json").exists());
        let projection_index: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("trajectory/projection_index.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(
            projection_index["schema_name"],
            "RunTrajectoryProjectionIndex"
        );
        assert_eq!(projection_index["entries"].as_array().unwrap().len(), 1);
        let behavior_path = projection_index["entries"][0]["behavior_path"]
            .as_str()
            .unwrap();
        let behavior: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("trajectory").join(behavior_path)).unwrap(),
        )
        .unwrap();
        assert_eq!(behavior["schema_name"], "RunTrajectoryBehaviorProjection");
        assert_eq!(behavior["events"].as_array().unwrap().len(), 1);
        let deployment_path = projection_index["entries"][0]["deployment_path"]
            .as_str()
            .unwrap();
        let deployment: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(root.join("trajectory").join(deployment_path)).unwrap(),
        )
        .unwrap();
        assert_eq!(
            deployment["schema_name"],
            "RunTrajectoryDeploymentProjection"
        );
        assert!(deployment["records"].is_array());

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn owner_audit_runtime_records_capsule_ledger_slice_event() {
        let root = std::env::temp_dir().join("owner_audit_runtime_capsule_ledger");
        let _ = std::fs::remove_dir_all(&root);
        let mut args = default_branch_args(123);
        args.generations = 0;
        args.max_branches = 1;
        args.search_nodes = 1;
        args.search_ms = 1;
        args.rescue_search_nodes = 1;
        args.rescue_search_ms = 1;
        args.boss_search_nodes = 1;
        args.boss_search_ms = 1;
        // Ledger persistence is deterministic work; keep wall-clock pressure
        // out of this correctness contract.
        args.wall_ms = None;

        OwnerAuditRuntime::run_capsule_slice(OwnerAuditSliceRequest {
            args,
            capsule_path: root.clone(),
            resume: false,
            human_output: false,
        })
        .unwrap();

        let ledger = std::fs::read_to_string(root.join("capsule_ledger.jsonl")).unwrap();
        let rows = ledger
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();
        let started = rows
            .iter()
            .find(|row| row["event"] == "slice_started")
            .unwrap();
        let committed = rows
            .iter()
            .find(|row| row["event"] == "trajectory_segment_committed")
            .unwrap();
        let finished = rows
            .iter()
            .find(|row| row["event"] == "slice_finished")
            .unwrap();
        assert_eq!(started["schema"], "branch_tiny_capsule_ledger_event_v0");
        assert_eq!(started["request_kind"], "start");
        assert_eq!(started["seed"], 123);
        assert_eq!(started["generation_start"], 0);
        assert!(started["generation_end"].is_null());
        assert!(started["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact["kind"] == "manifest"));
        assert_eq!(
            committed["schema"],
            "trajectory_segment_committed_ledger_event_v1"
        );
        assert_eq!(committed["depth"], 0);
        assert!(committed["segment_id"]
            .as_str()
            .unwrap()
            .starts_with("trajectory_segment:"));
        assert_eq!(finished["seed"], 123);
        assert_eq!(finished["generation_start"], 0);
        assert_eq!(finished["generation_end"], 0);
        assert!(finished["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| {
                artifact["kind"] == "frontier"
                    && artifact["schema"] == "branch_tiny_frontier_checkpoint"
            }));
        assert!(finished["artifact_refs"]
            .as_array()
            .unwrap()
            .iter()
            .any(|artifact| artifact["kind"] == "trajectory_segment"));

        let _ = std::fs::remove_dir_all(root);
    }
}
