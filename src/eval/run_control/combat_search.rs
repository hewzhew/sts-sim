use std::time::{Duration, Instant};

use crate::ai::combat_search_v2::{
    CombatSearchV2AdvanceStop, CombatSearchV2DecisionSnapshot, CombatSearchV2Session,
    CombatSearchV2WorkQuantum,
};

use super::accepted_combat_line_evidence::AcceptedCombatLineEvidenceV1;
use super::combat_line_adjudication::{CombatLineAcceptancePolicy, CombatLineAdjudicationV1};
use super::combat_line_executor::apply_selected_combat_candidate_line;
use super::combat_line_selector::{select_accepted_search_combat_line, CombatLineSelection};
use super::combat_line_trace::{
    attach_execution_adjudication, combat_candidate_line_summary, combat_search_line_summary,
};
use super::combat_no_win_fallback::{
    try_apply_no_win_fallback, try_apply_turn_segment_after_rejection,
};
use super::combat_search_rejection::{
    build_combat_search_rejection_outcome, CombatSearchRejectionOutcome,
};
use super::combat_search_setup::{
    effective_hp_loss_limit, prepare_search_combat, search_report_has_invalid_card_identity,
    PreparedCombatSearch,
};
use super::progress_options::{RunControlCombatSearchQuantum, RunControlSearchCombatOptions};
use super::session::{RunControlCombatSearchRejection, RunControlSession, RunProgressOutcome};
use super::trace_annotation::CombatAutomationTrajectorySource;

pub(super) fn apply_search_combat(
    session: &mut RunControlSession,
    options: RunControlSearchCombatOptions,
) -> Result<RunProgressOutcome, String> {
    let prepared = prepare_search_combat(session, options)?;
    let report = run_search_work_plan(
        &prepared.start,
        prepared.config.clone(),
        &prepared.options.work_quanta,
    );
    apply_prepared_search_report(session, prepared, report)
}

pub struct RunControlCombatWorkV1 {
    prepared: PreparedCombatSearch,
    search: CombatSearchV2Session,
    remaining_nodes: usize,
    remaining_wall_time: Option<Duration>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunControlCombatWorkAdvanceV1 {
    Pending,
    ReadyToFinish,
    AllowanceExhausted,
    GlobalDeadlineReached,
}

impl RunControlCombatWorkV1 {
    pub fn new(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
    ) -> Result<Self, String> {
        let prepared = prepare_search_combat(session, options)?;
        let search = CombatSearchV2Session::new(
            &prepared.start.engine,
            &prepared.start.combat,
            prepared.config.clone(),
        );
        Ok(Self {
            remaining_nodes: prepared.config.max_nodes,
            remaining_wall_time: prepared.config.wall_time,
            prepared,
            search,
        })
    }

    pub fn advance(
        &mut self,
        quantum: &RunControlCombatSearchQuantum,
        global_deadline: Option<Instant>,
    ) -> RunControlCombatWorkAdvanceV1 {
        let now = Instant::now();
        let global_remaining =
            global_deadline.map(|deadline| deadline.saturating_duration_since(now));
        if global_remaining == Some(Duration::ZERO) {
            return RunControlCombatWorkAdvanceV1::GlobalDeadlineReached;
        }

        let additional_nodes = quantum.additional_nodes.min(self.remaining_nodes);
        if additional_nodes == 0 {
            return RunControlCombatWorkAdvanceV1::AllowanceExhausted;
        }
        if self.remaining_wall_time == Some(Duration::ZERO) {
            return RunControlCombatWorkAdvanceV1::AllowanceExhausted;
        }

        let requested_wall = quantum.soft_wall_ms.map(Duration::from_millis);
        let soft_wall_time = [requested_wall, self.remaining_wall_time, global_remaining]
            .into_iter()
            .flatten()
            .min();
        if soft_wall_time == Some(Duration::ZERO) {
            return if global_remaining == Some(Duration::ZERO) {
                RunControlCombatWorkAdvanceV1::GlobalDeadlineReached
            } else {
                RunControlCombatWorkAdvanceV1::AllowanceExhausted
            };
        }

        let before_nodes = self.search.nodes_expanded();
        let started = Instant::now();
        let stop = self.search.advance(CombatSearchV2WorkQuantum {
            additional_nodes,
            soft_wall_time,
        });
        let elapsed = started.elapsed();
        let expanded = self.search.nodes_expanded().saturating_sub(before_nodes);
        self.remaining_nodes = self
            .remaining_nodes
            .saturating_sub(expanded.min(usize::MAX as u64) as usize);
        if let Some(remaining) = &mut self.remaining_wall_time {
            *remaining = remaining.saturating_sub(elapsed);
        }

        if matches!(
            stop,
            CombatSearchV2AdvanceStop::CandidateSatisfied
                | CombatSearchV2AdvanceStop::FrontierExhausted
                | CombatSearchV2AdvanceStop::AlreadyComplete
        ) {
            return RunControlCombatWorkAdvanceV1::ReadyToFinish;
        }
        if self.remaining_nodes == 0 || self.remaining_wall_time == Some(Duration::ZERO) {
            RunControlCombatWorkAdvanceV1::AllowanceExhausted
        } else {
            RunControlCombatWorkAdvanceV1::Pending
        }
    }

    pub fn snapshot(&self) -> CombatSearchV2DecisionSnapshot {
        self.search.snapshot()
    }

    pub fn quantum_count(&self) -> usize {
        self.search.quantum_count()
    }

    pub fn nodes_expanded(&self) -> u64 {
        self.search.nodes_expanded()
    }

    pub fn remaining_nodes(&self) -> usize {
        self.remaining_nodes
    }

    pub fn remaining_wall_ms(&self) -> Option<u64> {
        self.remaining_wall_time
            .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
    }

    pub fn finish_and_apply(
        self,
        session: &mut RunControlSession,
        finalization_deadline: Option<Instant>,
    ) -> Result<RunProgressOutcome, String> {
        let current = session.current_active_combat_position()?;
        if current != self.prepared.start {
            return Err(
                "combat work parent changed before its search result was committed".to_string(),
            );
        }
        let timed_production_deadline = finalization_deadline
            .or_else(|| self.prepared.config.wall_time.map(|_| Instant::now()));
        let report = self.search.finish_with_deadline_and_wall_time(
            timed_production_deadline,
            self.prepared.config.wall_time,
        );
        apply_prepared_search_report(session, self.prepared, report)
    }
}

fn apply_prepared_search_report(
    session: &mut RunControlSession,
    prepared: PreparedCombatSearch,
    report: crate::ai::combat_search_v2::CombatSearchV2Report,
) -> Result<RunProgressOutcome, String> {
    let effective_profile = prepared.effective_profile;
    let options = prepared.options;
    let start = prepared.start;
    let config = prepared.config;
    if search_report_has_invalid_card_identity(&report) {
        return Ok(build_combat_search_rejection_outcome(
            session,
            &start,
            &report,
            CombatSearchRejectionOutcome {
                result: "invalid_card_identity",
                detail: None,
                rejection: RunControlCombatSearchRejection::InvalidCardIdentity,
                trace_source: "search_combat_rejected",
                execution_adjudication: None,
            },
        ));
    }
    let Some(trajectory) = report.best_win_trajectory.as_ref() else {
        if options.enable_legacy_no_win_rescue {
            if let Some(outcome) = try_apply_no_win_fallback(
                session,
                &start,
                &config,
                &options,
                &report,
                effective_hp_loss_limit(session, &options),
            )? {
                return Ok(outcome);
            }
        } else if options.allow_smoke_bomb_survival_fallback {
            if let Some(outcome) =
                super::combat_no_win_fallback::try_apply_smoke_bomb_survival_fallback_after_rejection(
                    session,
                    "no_complete_winning_candidate",
                )?
            {
                return Ok(outcome);
            }
        }
        return Ok(build_combat_search_rejection_outcome(
            session,
            &start,
            &report,
            CombatSearchRejectionOutcome {
                result: "no_complete_winning_candidate",
                detail: None,
                rejection: RunControlCombatSearchRejection::NoCompleteWinningCandidate,
                trace_source: "search_combat_rejected",
                execution_adjudication: None,
            },
        ));
    };
    let acceptance_policy = CombatLineAcceptancePolicy::from_plugin(effective_profile.acceptance);
    let selected = match select_accepted_search_combat_line(
        session,
        &start,
        &config,
        &report,
        trajectory,
        acceptance_policy,
    ) {
        CombatLineSelection::Selected(selected) => selected,
        CombatLineSelection::Rejected {
            adjudication,
            detail,
        } => {
            return Ok(build_combat_search_rejection_outcome(
                session,
                &start,
                &report,
                CombatSearchRejectionOutcome {
                    result: "dirty_winning_candidate_rejected",
                    detail: Some(detail),
                    rejection: RunControlCombatSearchRejection::DirtyWinningCandidateRejected,
                    trace_source: "search_combat_rejected_dirty_win",
                    execution_adjudication: Some(adjudication),
                },
            ));
        }
        CombatLineSelection::ReplayFailed { adjudication } => {
            let CombatLineAdjudicationV1::ReplayFailed { error, .. } = adjudication else {
                unreachable!("replay-failed selection must carry replay-failed adjudication")
            };
            return Err(format!("combat line replay failed: {error}"));
        }
    };

    if let Some(max_hp_loss) = effective_hp_loss_limit(session, &options) {
        if selected.line.hp_loss > max_hp_loss as i32 {
            if let Some(outcome) = try_apply_turn_segment_after_rejection(
                session,
                &start,
                &config,
                &options,
                &report,
                "complete_winning_candidate_exceeds_hp_loss_limit",
            )? {
                return Ok(outcome);
            }
            return Ok(build_combat_search_rejection_outcome(
                session,
                &start,
                &report,
                CombatSearchRejectionOutcome {
                    result: "complete_winning_candidate_exceeds_hp_loss_limit",
                    detail: Some(format!(
                        "candidate_hp_loss={} max_hp_loss={max_hp_loss}",
                        selected.line.hp_loss
                    )),
                    rejection: RunControlCombatSearchRejection::HpLossLimitExceeded,
                    trace_source: "search_combat_rejected",
                    execution_adjudication: None,
                },
            ));
        }
    }

    let mut summary = format!(
        "search-combat applied {} actions profile={}",
        selected.line.actions.len(),
        effective_profile.profile_id
    );
    if let Some(repair_summary) = selected.summary.as_ref() {
        summary.push_str(&format!(" {repair_summary}"));
    }
    let accepted_line_evidence = AcceptedCombatLineEvidenceV1::new(
        combat_search_line_summary(trajectory),
        combat_candidate_line_summary(&selected.line),
        selected.summary.clone(),
    );
    let selected_adjudication = selected.adjudication;
    let mut outcome = apply_selected_combat_candidate_line(
        session,
        &start,
        &config,
        &report,
        selected.line,
        CombatAutomationTrajectorySource::SearchCombat,
        summary,
        None,
    )?
    .with_execution_adjudication(selected_adjudication.clone());
    outcome
        .trace_annotations
        .push(accepted_line_evidence.into_annotation());
    attach_execution_adjudication(&mut outcome.trace_annotations, &selected_adjudication);
    Ok(outcome)
}

fn run_search_work_plan(
    start: &crate::sim::combat::CombatPosition,
    config: crate::ai::combat_search_v2::CombatSearchV2Config,
    work_quanta: &[super::progress_options::RunControlCombatSearchQuantum],
) -> crate::ai::combat_search_v2::CombatSearchV2Report {
    let default_quantum = super::progress_options::RunControlCombatSearchQuantum {
        label: "single_run",
        additional_nodes: config.max_nodes,
        soft_wall_ms: config
            .wall_time
            .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64),
    };
    let quanta = if work_quanta.is_empty() {
        std::slice::from_ref(&default_quantum)
    } else {
        work_quanta
    };
    let wall_time = config
        .wall_time
        .or_else(|| summed_quantum_wall_time(quanta));
    let mut budget = CombatSearchWorkBudget::new(config.max_nodes, wall_time, Instant::now());
    let deadline = budget.deadline;
    let mut search = CombatSearchV2Session::new(&start.engine, &start.combat, config);
    for quantum in quanta {
        let Some(authorized) = budget.authorize(quantum, Instant::now()) else {
            break;
        };
        let stop = search.advance(authorized);
        if matches!(
            stop,
            crate::ai::combat_search_v2::CombatSearchV2AdvanceStop::CandidateSatisfied
                | crate::ai::combat_search_v2::CombatSearchV2AdvanceStop::FrontierExhausted
                | crate::ai::combat_search_v2::CombatSearchV2AdvanceStop::AlreadyComplete
        ) {
            break;
        }
    }
    search.finish_with_deadline(deadline)
}

fn summed_quantum_wall_time(
    quanta: &[super::progress_options::RunControlCombatSearchQuantum],
) -> Option<Duration> {
    quanta.iter().try_fold(Duration::ZERO, |total, quantum| {
        quantum
            .soft_wall_ms
            .map(Duration::from_millis)
            .map(|duration| total.saturating_add(duration))
    })
}

struct CombatSearchWorkBudget {
    deadline: Option<Instant>,
    remaining_nodes: usize,
}

impl CombatSearchWorkBudget {
    fn new(max_nodes: usize, wall_time: Option<Duration>, started: Instant) -> Self {
        Self {
            deadline: wall_time.and_then(|duration| started.checked_add(duration)),
            remaining_nodes: max_nodes,
        }
    }

    fn authorize(
        &mut self,
        quantum: &super::progress_options::RunControlCombatSearchQuantum,
        now: Instant,
    ) -> Option<CombatSearchV2WorkQuantum> {
        let additional_nodes = quantum.additional_nodes.min(self.remaining_nodes);
        if quantum.additional_nodes > 0 && additional_nodes == 0 {
            return None;
        }
        let remaining_wall_time = self
            .deadline
            .map(|deadline| deadline.saturating_duration_since(now));
        if remaining_wall_time == Some(Duration::ZERO) {
            return None;
        }
        let requested_wall_time = quantum.soft_wall_ms.map(Duration::from_millis);
        let soft_wall_time = match (requested_wall_time, remaining_wall_time) {
            (Some(requested), Some(remaining)) => Some(requested.min(remaining)),
            (Some(requested), None) => Some(requested),
            (None, Some(remaining)) => Some(remaining),
            (None, None) => None,
        };
        self.remaining_nodes = self.remaining_nodes.saturating_sub(additional_nodes);
        Some(CombatSearchV2WorkQuantum {
            additional_nodes,
            soft_wall_time,
        })
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::super::combat_line_trace::{
        combat_automation_answer_claims_v1, combat_automation_opportunity_state_v1,
        combat_automation_step_state_v1,
    };
    use super::super::combat_no_win_fallback::segment_mode_allows_turn_segment;
    use super::super::combat_search_setup::{
        effective_hp_loss_limit, high_stakes_search_options, search_config,
    };
    use super::{
        run_search_work_plan, CombatSearchWorkBudget, RunControlCombatWorkAdvanceV1,
        RunControlCombatWorkV1,
    };
    use crate::ai::combat_search_v2::{
        CombatSearchAcceptancePluginId, CombatSearchActionPriorPluginId,
        CombatSearchArtifactPluginId, CombatSearchAttemptPolicy, CombatSearchBudgetSpec,
        CombatSearchEngineProfile, CombatSearchPluginStack, CombatSearchPotionPlugin,
        CombatSearchProfile, CombatSearchRolloutPluginId, CombatSearchV2PotionPolicy,
        CombatSearchV2RolloutPolicy, CombatSearchV2Satisfaction, CombatSearchV2SetupBiasPolicy,
    };

    #[test]
    fn work_budget_caps_every_quantum_to_one_node_and_wall_owner() {
        let started = Instant::now();
        let mut budget = CombatSearchWorkBudget::new(10, Some(Duration::from_millis(10)), started);
        let requested = super::super::progress_options::RunControlCombatSearchQuantum {
            label: "test",
            additional_nodes: 6,
            soft_wall_ms: Some(8),
        };

        let first = budget
            .authorize(&requested, started)
            .expect("initial quantum should be authorized");
        assert_eq!(first.additional_nodes, 6);
        assert_eq!(first.soft_wall_time, Some(Duration::from_millis(8)));

        let second = budget
            .authorize(&requested, started + Duration::from_millis(8))
            .expect("refinement should consume only the remaining budget");
        assert_eq!(second.additional_nodes, 4);
        assert_eq!(second.soft_wall_time, Some(Duration::from_millis(2)));

        assert!(budget
            .authorize(&requested, started + Duration::from_millis(10))
            .is_none());
    }

    #[test]
    fn legacy_no_win_solver_chain_is_opt_in() {
        assert!(!RunControlSearchCombatOptions::default().enable_legacy_no_win_rescue);
    }

    #[test]
    fn search_work_plan_reports_only_resources_authorized_by_its_single_budget() {
        let mut combat = crate::test_support::blank_test_combat();
        let mut jaw_worm =
            crate::test_support::test_monster(crate::content::monsters::EnemyId::JawWorm);
        let plan = crate::content::monsters::roll_monster_turn_plan(
            &mut combat.rng.ai_rng,
            &jaw_worm,
            combat.meta.ascension_level,
            99,
            std::slice::from_ref(&jaw_worm),
            &[],
        );
        jaw_worm.set_planned_move_id(plan.move_id);
        jaw_worm.set_planned_steps(plan.steps);
        jaw_worm.set_planned_visible_spec(plan.visible_spec);
        combat.entities.monsters = vec![jaw_worm];
        combat.zones.hand = (0..5)
            .map(|index| {
                crate::runtime::combat::CombatCard::new(
                    crate::content::cards::CardId::Strike,
                    100 + index,
                )
            })
            .collect();
        combat.update_hand_cards();
        let start = crate::sim::combat::CombatPosition::new(
            crate::state::core::EngineState::CombatPlayerTurn,
            combat,
        );
        let report = run_search_work_plan(
            &start,
            crate::ai::combat_search_v2::CombatSearchV2Config {
                max_nodes: 10,
                wall_time: Some(Duration::from_millis(150)),
                rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
                satisfaction: CombatSearchV2Satisfaction::BudgetOrExhaustion,
                ..crate::ai::combat_search_v2::CombatSearchV2Config::default()
            },
            &[
                super::super::progress_options::RunControlCombatSearchQuantum {
                    label: "initial",
                    additional_nodes: 6,
                    soft_wall_ms: Some(100),
                },
                super::super::progress_options::RunControlCombatSearchQuantum {
                    label: "refine",
                    additional_nodes: 6,
                    soft_wall_ms: Some(100),
                },
            ],
        );

        assert_eq!(report.quantum_history.len(), 2);
        assert_eq!(report.quantum_history[0].requested_additional_nodes, 6);
        assert_eq!(report.quantum_history[1].requested_additional_nodes, 4);
        assert_eq!(report.budget.max_nodes, 10);
        assert!(report.budget.wall_time_ms.is_some_and(|wall| wall <= 150));
        assert!(report.quantum_history.iter().all(|quantum| quantum
            .requested_soft_wall_time_ms
            .is_some_and(|wall| wall <= 100)));
    }
    use crate::content::potions::{Potion, PotionId};
    use crate::content::powers::{store, PowerId};
    use crate::eval::run_control::trace_annotation::{
        CombatAutomationActionV1, CombatAutomationAnswerSourceV1, CombatAutomationCardOriginV1,
        CombatAutomationOpportunityStateV1, CombatAutomationPotionStateV1,
        CombatAutomationTrajectoryRecordV1, CombatAutomationTrajectorySource,
        RunControlTraceAnnotationV1,
    };
    use crate::eval::run_control::{
        RunControlConfig, RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
    };
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{
        ActiveCombat, ClientInput, CombatContext, EngineState, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;
    use crate::state::rewards::RewardScreenContext;

    fn session_with_active_combat(
        mut combat: crate::runtime::combat::CombatState,
    ) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            {
                combat.entities.monsters = vec![crate::test_support::test_monster(
                    crate::content::monsters::EnemyId::JawWorm,
                )];
                combat
            },
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    #[test]
    fn combat_automation_step_state_records_time_warp_counter_and_forced_end_state() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(
            crate::content::monsters::EnemyId::TimeEater,
        )];
        let monster_id = combat.entities.monsters[0].id;
        store::set_powers_for(
            &mut combat,
            monster_id,
            vec![
                crate::runtime::combat::Power {
                    power_type: PowerId::TimeWarp,
                    instance_id: None,
                    amount: 11,
                    extra_data: 0,
                    payload: crate::runtime::combat::PowerPayload::None,
                    just_applied: false,
                },
                crate::runtime::combat::Power {
                    power_type: PowerId::Strength,
                    instance_id: None,
                    amount: 2,
                    extra_data: 0,
                    payload: crate::runtime::combat::PowerPayload::None,
                    just_applied: false,
                },
            ],
        );
        combat.turn.counters.cards_played_this_turn = 11;
        combat.turn.mark_early_end_turn_pending();
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));

        let snapshot = combat_automation_step_state_v1(&session)
            .expect("active combat should produce automation step state");

        assert_eq!(snapshot.cards_played_this_turn, 11);
        assert!(snapshot.early_end_turn_pending);
        assert_eq!(snapshot.monsters.len(), 1);
        assert_eq!(snapshot.monsters[0].label, "Time Eater");
        assert_eq!(snapshot.monsters[0].time_warp, 11);
        assert_eq!(snapshot.monsters[0].strength, 2);
    }

    #[test]
    fn combat_automation_opportunity_uses_exact_legal_card_and_potion_masks() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.turn.energy = 0;
        combat.zones.hand = vec![
            CombatCard::new(crate::content::cards::CardId::Strike, 10),
            CombatCard::new(crate::content::cards::CardId::Anger, 20),
        ];
        combat.entities.potions = vec![
            Some(Potion::new(PotionId::BlockPotion, 30)),
            Some(Potion::new(PotionId::FairyPotion, 40)),
        ];
        let session = session_with_active_combat(combat);

        let snapshot = combat_automation_opportunity_state_v1(&session)
            .expect("active combat should expose an opportunity snapshot");

        assert_eq!(
            snapshot
                .hand
                .iter()
                .map(|card| card.uuid)
                .collect::<Vec<_>>(),
            vec![10, 20]
        );
        assert_eq!(snapshot.playable_card_uuids, vec![20]);
        assert_eq!(snapshot.usable_potion_uuids, vec![30]);
        assert_eq!(snapshot.potions[0].as_ref().unwrap().uuid, 30);
        assert_eq!(snapshot.potions[1].as_ref().unwrap().uuid, 40);
    }

    #[test]
    fn pending_choice_opportunity_does_not_enumerate_combinatorial_inputs() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.draw_pile = (0..13)
            .map(|index| CombatCard::new(crate::content::cards::CardId::Strike, 1_000 + index))
            .collect();
        let mut session = session_with_active_combat(combat);
        session.active_combat.as_mut().unwrap().engine_state =
            EngineState::PendingChoice(crate::state::core::PendingChoice::ScrySelect {
                cards: vec![crate::content::cards::CardId::Strike; 13],
                card_uuids: (1_000..1_013).collect(),
            });

        let snapshot = combat_automation_opportunity_state_v1(&session)
            .expect("pending combat choice should still expose the state snapshot");

        assert!(snapshot.playable_card_uuids.is_empty());
        assert!(snapshot.usable_potion_uuids.is_empty());
    }

    #[test]
    fn combat_automation_claims_cover_master_generated_and_active_potion_answers() {
        let master_deck = vec![CombatCard::new(
            crate::content::cards::CardId::Shockwave,
            10,
        )];
        let actions = vec![CombatAutomationActionV1 {
            step_index: 0,
            action_key: "combat/test".to_string(),
            input: ClientInput::EndTurn,
            opportunity_before: Some(CombatAutomationOpportunityStateV1 {
                turn: 1,
                energy: 3,
                hand: vec![
                    crate::eval::run_control::RunActionCardSnapshotV1 {
                        id: crate::content::cards::CardId::Shockwave,
                        uuid: 10,
                        upgrades: 0,
                    },
                    crate::eval::run_control::RunActionCardSnapshotV1 {
                        id: crate::content::cards::CardId::Corruption,
                        uuid: 20,
                        upgrades: 0,
                    },
                ],
                potions: vec![
                    Some(CombatAutomationPotionStateV1 {
                        id: PotionId::BlockPotion,
                        uuid: 30,
                    }),
                    Some(CombatAutomationPotionStateV1 {
                        id: PotionId::SmokeBomb,
                        uuid: 40,
                    }),
                ],
                playable_card_uuids: vec![10, 20],
                usable_potion_uuids: vec![30, 40],
            }),
            drawn_cards: Vec::new(),
            combat_after: None,
        }];

        let claims = combat_automation_answer_claims_v1(&master_deck, &actions);

        assert!(claims.iter().any(|claim| matches!(
            claim.source,
            CombatAutomationAnswerSourceV1::Card {
                uuid: 10,
                origin: CombatAutomationCardOriginV1::MasterDeck,
                ..
            }
        )));
        assert!(claims.iter().any(|claim| matches!(
            claim.source,
            CombatAutomationAnswerSourceV1::Card {
                uuid: 20,
                origin: CombatAutomationCardOriginV1::CombatGenerated,
                ..
            }
        )));
        assert!(claims.iter().any(|claim| matches!(
            claim.source,
            CombatAutomationAnswerSourceV1::Potion { uuid: 30, .. }
        )));
        assert!(!claims.iter().any(|claim| matches!(
            claim.source,
            CombatAutomationAnswerSourceV1::Potion { uuid: 40, .. }
        )));
    }

    fn session_with_combat_flags(is_boss_fight: bool, is_elite_fight: bool) -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = is_boss_fight;
        combat.meta.is_elite_fight = is_elite_fight;
        session_with_active_combat(combat)
    }

    #[test]
    fn resumable_run_control_work_keeps_one_search_session_across_quanta() {
        let mut session = session_with_combat_flags(false, false);
        session
            .active_combat
            .as_mut()
            .expect("active combat")
            .combat_state
            .entities
            .monsters[0] =
            crate::test_support::planned_monster(crate::content::monsters::EnemyId::JawWorm, 1);
        let mut work = RunControlCombatWorkV1::new(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(8),
                wall_ms: None,
                rollout_policy: Some(CombatSearchV2RolloutPolicy::Disabled),
                satisfaction: Some(CombatSearchV2Satisfaction::BudgetOrExhaustion),
                ..RunControlSearchCombatOptions::default()
            },
        )
        .expect("combat work should initialize");
        let quantum = super::super::progress_options::RunControlCombatSearchQuantum {
            label: "resume_contract",
            additional_nodes: 1,
            soft_wall_ms: None,
        };

        let first = work.advance(&quantum, None);
        assert_eq!(first, RunControlCombatWorkAdvanceV1::Pending);
        let first_nodes = work.snapshot().nodes_expanded;
        assert_eq!(work.quantum_count(), 1);

        let second = work.advance(&quantum, None);
        assert_eq!(second, RunControlCombatWorkAdvanceV1::Pending);
        assert_eq!(work.quantum_count(), 2);
        assert!(work.snapshot().nodes_expanded >= first_nodes);
        assert!(work.remaining_nodes() <= 7);
    }

    fn options_with_hp_loss(max_hp_loss: RunControlHpLossLimit) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            max_hp_loss: Some(max_hp_loss),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn options_with_potion_budget(
        potion_policy: CombatSearchV2PotionPolicy,
        max_potions_used: u32,
    ) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            potion_policy: Some(potion_policy),
            max_potions_used: Some(max_potions_used),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn assert_potion_budget(
        options: RunControlSearchCombatOptions,
        expected_policy: Option<CombatSearchV2PotionPolicy>,
        expected_max_used: Option<u32>,
    ) {
        assert_eq!(options.potion_policy, expected_policy);
        assert_eq!(options.max_potions_used, expected_max_used);
    }

    #[test]
    fn search_combat_can_use_only_smoke_bomb_fallback_when_full_rescue_is_disabled() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.player.current_hp = 1;
        combat.entities.player.max_hp = 80;
        combat.turn.energy = 0;
        combat.meta.is_boss_fight = false;
        let mut jaw_worm =
            crate::test_support::test_monster(crate::content::monsters::EnemyId::JawWorm);
        jaw_worm.current_hp = 40;
        jaw_worm.max_hp = 40;
        let plan = crate::content::monsters::roll_monster_turn_plan(
            &mut combat.rng.ai_rng,
            &jaw_worm,
            combat.meta.ascension_level,
            99,
            std::slice::from_ref(&jaw_worm),
            &[],
        );
        jaw_worm.set_planned_move_id(plan.move_id);
        jaw_worm.set_planned_steps(plan.steps);
        jaw_worm.set_planned_visible_spec(plan.visible_spec);
        combat.entities.monsters = vec![jaw_worm];
        combat.zones.hand = vec![CombatCard::new(crate::content::cards::CardId::Defend, 1)];
        combat.update_hand_cards();
        combat.entities.potions = vec![Some(Potion::new(PotionId::SmokeBomb, 1))];
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let outcome = super::apply_search_combat(
            &mut session,
            RunControlSearchCombatOptions {
                max_nodes: Some(1),
                wall_ms: Some(1),
                enable_legacy_no_win_rescue: false,
                allow_smoke_bomb_survival_fallback: true,
                ..RunControlSearchCombatOptions::default()
            },
        )
        .expect("search fallback should not error");

        let EngineState::RewardScreen(rewards) = &session.engine_state else {
            panic!(
                "smoke bomb fallback should leave combat at reward screen, got {:?}",
                session.engine_state
            );
        };
        assert_eq!(rewards.screen_context, RewardScreenContext::SmokedCombat);
        assert!(
            outcome.message.contains("Smoke Bomb"),
            "fallback outcome should be explicit, got: {}",
            outcome.message
        );
        let Some(resolution) = outcome.single_combat_resolution() else {
            panic!("Smoke Bomb fallback should preserve one combat resolution");
        };
        assert_eq!(
            resolution.kind,
            crate::eval::run_control::RunCombatResolutionKindV1::SmokeBombEscape
        );
        assert_eq!(resolution.before.decision_step, 0);
        assert_eq!(resolution.after.decision_step, 0);
        assert_eq!(session.decision_step, 0);
    }

    #[test]
    fn combat_automation_trace_annotation_preserves_action_inputs() {
        let annotation = CombatAutomationTrajectoryRecordV1::new(
            CombatAutomationTrajectorySource::SearchCombat,
            vec![CombatAutomationActionV1 {
                step_index: 7,
                action_key: "combat/end_turn".to_string(),
                input: ClientInput::EndTurn,
                opportunity_before: None,
                drawn_cards: Vec::new(),
                combat_after: None,
            }],
        )
        .into_annotation();

        let RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source,
            action_count,
            actions,
            answer_claims,
            label_role,
        } = annotation
        else {
            panic!("expected combat automation trajectory annotation")
        };
        assert_eq!(source, CombatAutomationTrajectorySource::SearchCombat);
        assert_eq!(action_count, 1);
        assert_eq!(actions[0].step_index, 7);
        assert_eq!(actions[0].action_key, "combat/end_turn");
        assert_eq!(actions[0].input, ClientInput::EndTurn);
        assert!(answer_claims.is_empty());
        assert_eq!(label_role, "simulator_generated_not_teacher_label");
    }

    #[test]
    fn hp_loss_limit_uses_session_default_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_max_hp_loss: Some(12),
            ..RunControlConfig::default()
        });

        assert_eq!(
            effective_hp_loss_limit(&session, &RunControlSearchCombatOptions::default()),
            Some(12)
        );
        assert_eq!(
            search_config(&session, RunControlSearchCombatOptions::default()).satisfaction,
            CombatSearchV2Satisfaction::HpLossAtMost(12)
        );
        assert_eq!(
            effective_hp_loss_limit(
                &session,
                &options_with_hp_loss(RunControlHpLossLimit::Limit(4))
            ),
            Some(4)
        );
        assert_eq!(
            search_config(
                &session,
                options_with_hp_loss(RunControlHpLossLimit::Limit(4))
            )
            .satisfaction,
            CombatSearchV2Satisfaction::HpLossAtMost(4)
        );
        assert_eq!(
            effective_hp_loss_limit(
                &session,
                &options_with_hp_loss(RunControlHpLossLimit::Unlimited)
            ),
            None
        );
        assert_eq!(
            search_config(
                &session,
                options_with_hp_loss(RunControlHpLossLimit::Unlimited)
            )
            .satisfaction,
            CombatSearchV2Satisfaction::FirstCompleteWin
        );
    }

    #[test]
    fn search_config_uses_session_budget_defaults_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_max_nodes: Some(1234),
            search_wall_ms: Some(5678),
            ..RunControlConfig::default()
        });

        let config = search_config(&session, RunControlSearchCombatOptions::default());
        assert_eq!(config.max_nodes, 1234);
        assert_eq!(config.wall_time, Some(Duration::from_millis(5678)));

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(90),
                wall_ms: Some(12),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.max_nodes, 90);
        assert_eq!(config.wall_time, Some(Duration::from_millis(12)));
    }

    #[test]
    fn search_config_uses_profile_as_default_config_source() {
        let session = RunControlSession::new(RunControlConfig::default());
        let profile = CombatSearchProfile {
            label: "profile_default",
            engine: CombatSearchEngineProfile {
                budget: CombatSearchBudgetSpec {
                    max_nodes: 222,
                    wall_ms: 333,
                },
                plugins: CombatSearchPluginStack {
                    action_prior: CombatSearchActionPriorPluginId::KeyCardOnline,
                    rollout: CombatSearchRolloutPluginId::Disabled,
                    ..CombatSearchPluginStack::default()
                },
            },
            policy: CombatSearchAttemptPolicy {
                acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
                artifacts: CombatSearchArtifactPluginId::None,
            },
        };

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                profile: Some(profile),
                ..RunControlSearchCombatOptions::default()
            },
        );

        assert_eq!(config.max_nodes, 222);
        assert_eq!(config.wall_time, Some(Duration::from_millis(333)));
        assert_eq!(config.rollout_policy, CombatSearchV2RolloutPolicy::Disabled);
        assert_eq!(
            config.setup_bias_policy,
            CombatSearchV2SetupBiasPolicy::KeyCardOnline
        );

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                profile: Some(profile),
                max_nodes: Some(444),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.max_nodes, 444);
    }

    #[test]
    fn search_config_uses_session_potion_defaults_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_potion_policy: Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            search_max_potions_used: Some(2),
            ..RunControlConfig::default()
        });

        let config = search_config(&session, RunControlSearchCombatOptions::default());
        assert_eq!(
            config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(2));

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                potion_policy: Some(CombatSearchV2PotionPolicy::Never),
                max_potions_used: Some(0),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(config.max_potions_used, Some(0));
    }

    #[test]
    fn high_stakes_search_options_enables_semantic_potions_for_boss_manual_search() {
        let session = session_with_combat_flags(true, false);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(
            options,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            Some(2),
        );
    }

    #[test]
    fn high_stakes_search_options_enables_single_semantic_potion_for_elite_manual_search() {
        let session = session_with_combat_flags(false, true);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(
            options,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            Some(1),
        );
    }

    #[test]
    fn high_stakes_search_options_does_not_override_profile_potion_plugin() {
        let session = session_with_combat_flags(true, false);
        let profile = CombatSearchProfile {
            label: "no_potion_profile",
            engine: CombatSearchEngineProfile {
                budget: CombatSearchBudgetSpec {
                    max_nodes: 10,
                    wall_ms: 20,
                },
                plugins: CombatSearchPluginStack {
                    potion: CombatSearchPotionPlugin {
                        policy: CombatSearchV2PotionPolicy::Never,
                        max_potions_used: Some(0),
                    },
                    ..CombatSearchPluginStack::default()
                },
            },
            policy: CombatSearchAttemptPolicy {
                acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
                artifacts: CombatSearchArtifactPluginId::None,
            },
        };

        let options = high_stakes_search_options(
            &session,
            RunControlSearchCombatOptions {
                profile: Some(profile),
                ..RunControlSearchCombatOptions::default()
            },
        );

        assert_eq!(options.potion_policy, None);
        assert_eq!(options.max_potions_used, None);
        let config = search_config(&session, options);
        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(config.max_potions_used, Some(0));
    }

    #[test]
    fn non_boss_segment_mode_allows_hallway_partial_turns_but_blocks_boss_partial_turns() {
        let hallway = session_with_combat_flags(false, false);
        let hallway_start = hallway
            .current_active_combat_position()
            .expect("hallway combat position");
        assert!(segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::NonBossTurnBoundary),
            &hallway_start
        ));

        let boss = session_with_combat_flags(true, false);
        let boss_start = boss
            .current_active_combat_position()
            .expect("boss combat position");
        assert!(!segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::NonBossTurnBoundary),
            &boss_start
        ));
        assert!(segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::TurnBoundary),
            &boss_start
        ));
    }

    #[test]
    fn high_stakes_search_options_keeps_ordinary_manual_search_no_potion_default() {
        let session = session_with_combat_flags(false, false);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(options, None, None);
    }

    #[test]
    fn high_stakes_search_options_respects_user_potion_override() {
        let session = session_with_combat_flags(true, false);

        let options = high_stakes_search_options(
            &session,
            options_with_potion_budget(CombatSearchV2PotionPolicy::Never, 0),
        );

        assert_potion_budget(options, Some(CombatSearchV2PotionPolicy::Never), Some(0));
    }
}
