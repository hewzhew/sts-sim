use super::*;
use std::sync::atomic::{AtomicUsize, Ordering as AtomicOrdering};
use sts_core::sim::combat::EngineCombatStepper;

struct CountingLookahead {
    calls: Arc<AtomicUsize>,
}

struct CountingGenerationGuides {
    calls: Arc<AtomicUsize>,
}

impl super::super::policy::CombatActionPolicy for CountingGenerationGuides {
    fn weights(
        &self,
        _position: &CombatPosition,
        choices: &[super::super::policy::CombatPolicyChoice<'_>],
    ) -> Vec<f64> {
        vec![1.0; choices.len()]
    }

    fn turn_generation_guides(&self, _position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.calls.fetch_add(1, AtomicOrdering::Relaxed);
        vec![CombatStateGuide::new(CombatGuideLaneId::new(98), vec![0])]
    }
}

impl super::super::policy::CombatLookaheadEvaluator for CountingLookahead {
    fn pending_guide(&self, _position: &CombatPosition) -> Option<CombatStateGuide> {
        Some(CombatStateGuide::new(CombatGuideLaneId::new(99), vec![0]))
    }

    fn admit_atomic_state(
        &self,
        _position: &CombatPosition,
        _atomic_expansions_before: usize,
    ) -> bool {
        true
    }

    fn evaluate(
        &self,
        _position: &CombatPosition,
        max_work: usize,
        _deadline: Option<Instant>,
    ) -> Option<super::super::policy::CombatLookaheadEvaluation> {
        self.calls.fetch_add(1, AtomicOrdering::Relaxed);
        Some(super::super::policy::CombatLookaheadEvaluation {
            guide: CombatStateGuide::new(CombatGuideLaneId::new(99), vec![1]),
            work: 3.min(max_work),
        })
    }
}

fn test_root() -> CombatDecisionRoot {
    let mut combat = sts_core::test_support::blank_test_combat();
    combat.entities.monsters = vec![sts_core::test_support::test_monster(
        sts_core::content::monsters::EnemyId::JawWorm,
    )];
    CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat))
        .expect("test combat is a player-turn root")
}

fn guided_entry(
    guide: i32,
    cumulative_negative_log_policy: f64,
    atomic_depth: usize,
    sequence_id: u64,
) -> GuidedGeneratorQueueEntry {
    GuidedGeneratorQueueEntry {
        guide_lane: CombatGuideLaneId::new(0),
        work_id: sequence_id as usize,
        sequence_id,
        guide_rank: CombatStateGuideRank::new(vec![guide]),
        anchor_priority: GeneratorWorkPriority::for_path(
            atomic_depth,
            cumulative_negative_log_policy,
        ),
    }
}

#[test]
fn guided_prefix_priority_uses_exact_state_before_anchor_policy() {
    let improved_after_setup = guided_entry(10, 8.0, 3, 0);
    let locally_greedy = guided_entry(9, 0.01, 1, 1);

    assert!(improved_after_setup > locally_greedy);
}

#[test]
fn one_partial_state_computes_base_generation_guides_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let policy = Arc::new(CountingGenerationGuides {
        calls: calls.clone(),
    });
    let mut session = TurnOptionGeneratorSession::with_policy(
        test_root(),
        TurnOptionGeneratorConfig::default(),
        policy,
    );

    assert_eq!(calls.load(AtomicOrdering::Relaxed), 1);
    let report = session.advance(
        &EngineCombatStepper,
        CombatPlanningQuantum::deterministic(1, 250_000),
    );

    assert_eq!(report.after.generation_work, 1);
    assert_eq!(
        calls.load(AtomicOrdering::Relaxed),
        1,
        "expanding a queued partial must reuse the guide bundle computed at publication"
    );
}

#[test]
fn expensive_lookahead_is_lazy_budgeted_and_does_not_expand_the_state() {
    let calls = Arc::new(AtomicUsize::new(0));
    let evaluator = Arc::new(CountingLookahead {
        calls: calls.clone(),
    });
    let mut session = TurnOptionGeneratorSession::with_policy_and_lookahead(
        test_root(),
        TurnOptionGeneratorConfig::default(),
        uniform_policy(),
        evaluator,
    );
    let report = session.advance_with_lookahead(
        &EngineCombatStepper,
        CombatPlanningQuantum::deterministic(1, 250_000),
        1,
        3,
        3,
    );

    assert_eq!(calls.load(AtomicOrdering::Relaxed), 1);
    assert_eq!(session.lookahead_evaluations(), 1);
    assert_eq!(session.lookahead_work(), 3);
    assert_eq!(session.atomic_state_expansions(), 0);
    assert_eq!(session.retained_lookahead_guides(), 1);
    assert_eq!(report.after.generation_work, 1);
    assert!(session.retained_work_items() > 0);
}

#[test]
fn atomic_cursor_conserves_residual_probability_mass() {
    let mut session =
        TurnOptionGeneratorSession::new(test_root(), TurnOptionGeneratorConfig::default());
    let GeneratorWork::Expand(parent) = session.pop_scheduled_work().expect("root expansion work")
    else {
        panic!("root work must be an expansion");
    };
    let mut cursor = AtomicActionCursorWork::new(
        Arc::new(parent),
        vec![
            ClientInput::EndTurn,
            ClientInput::Cancel,
            ClientInput::Proceed,
        ],
        vec![0.2, 0.5, 0.3],
        Vec::new(),
    )
    .expect("non-empty action surface");

    let initial = cursor.priority().unwrap();
    assert!(initial.negative_log_policy.abs() < 1.0e-12);
    assert_eq!(
        cursor.current_transition().unwrap().input,
        ClientInput::Cancel,
        "the most probable concrete edge is emitted first"
    );

    cursor.consume_current();
    let residual = cursor.priority().unwrap();
    assert!((residual.negative_log_policy - (-0.5_f64.ln())).abs() < 1.0e-12);
    let next_concrete = cursor.current_transition().unwrap();
    assert!(residual.negative_log_policy <= next_concrete.negative_log_policy);

    cursor.consume_current();
    let final_residual = cursor.priority().unwrap();
    let final_concrete = cursor.current_transition().unwrap();
    assert_eq!(
        final_residual.negative_log_policy.to_bits(),
        final_concrete.negative_log_policy.to_bits(),
        "one remaining edge has exactly the cursor's residual bound"
    );
    cursor.consume_current();
    assert!(cursor.priority().is_none());
}

#[test]
fn action_transition_does_not_bypass_explicit_anchor_priority() {
    let mut session =
        TurnOptionGeneratorSession::new(test_root(), TurnOptionGeneratorConfig::default());
    let GeneratorWork::Expand(parent) = session.pop_scheduled_work().expect("root expansion work")
    else {
        panic!("root work must be an expansion");
    };

    for _ in 0..32 {
        session.push_work(
            GeneratorWork::Expand(parent.clone()),
            GeneratorWorkPriority::for_path(1, 0.0),
        );
    }
    session.push_work(
        GeneratorWork::ApplyAction(ActionTransitionWork {
            parent: Arc::new(parent),
            input: ClientInput::EndTurn,
            atomic_depth: 1,
            negative_log_policy: 100.0,
        }),
        GeneratorWorkPriority::for_path(1, 100.0),
    );
    session.prefer_lane(TurnOptionGeneratorPreferredLane::Anchor);

    assert!(matches!(
        session.pop_scheduled_work(),
        Some(GeneratorWork::Expand(_))
    ));
}

#[test]
fn scheduling_round_heads_cannot_be_overtaken_by_new_arrivals() {
    let mut session =
        TurnOptionGeneratorSession::new(test_root(), TurnOptionGeneratorConfig::default());
    let GeneratorWork::Expand(parent) = session.pop_scheduled_work().expect("root expansion work")
    else {
        panic!("root work must be an expansion");
    };
    let anchor_head = session.push_work(
        GeneratorWork::Expand(parent.clone()),
        GeneratorWorkPriority::for_path(1, 0.0),
    );
    let guide_head = session.push_work(
        GeneratorWork::Expand(parent.clone()),
        GeneratorWorkPriority::for_path(1, 10.0),
    );
    let lane = CombatGuideLaneId::new(99);
    let guide_index = session.ensure_guide_frontier(lane);
    session.guided_frontiers[guide_index]
        .entries
        .push(GuidedGeneratorQueueEntry {
            guide_lane: lane,
            work_id: anchor_head,
            sequence_id: 10_000,
            guide_rank: CombatStateGuideRank::new(vec![0]),
            anchor_priority: GeneratorWorkPriority::for_path(1, 0.0),
        });
    session.guided_frontiers[guide_index]
        .entries
        .push(GuidedGeneratorQueueEntry {
            guide_lane: lane,
            work_id: guide_head,
            sequence_id: 10_001,
            guide_rank: CombatStateGuideRank::new(vec![10]),
            anchor_priority: GeneratorWorkPriority::for_path(1, 10.0),
        });

    session.next_scheduler_lane = 0;
    let round = session.snapshot_scheduling_round();
    assert_eq!(
        round.iter().copied().collect::<Vec<_>>(),
        vec![(0, anchor_head), (1, guide_head)]
    );

    let newcomer = session.push_work(
        GeneratorWork::Expand(parent),
        GeneratorWorkPriority::for_path(1, 0.0),
    );
    session.guided_frontiers[guide_index]
        .entries
        .push(GuidedGeneratorQueueEntry {
            guide_lane: lane,
            work_id: newcomer,
            sequence_id: 10_002,
            guide_rank: CombatStateGuideRank::new(vec![20]),
            anchor_priority: GeneratorWorkPriority::for_path(1, 0.0),
        });

    assert_eq!(
        round.iter().copied().collect::<Vec<_>>(),
        vec![(0, anchor_head), (1, guide_head)],
        "a later arrival belongs to the next round even when it becomes the new guide head"
    );
}

#[test]
fn interrupted_scheduling_round_resumes_before_new_arrivals() {
    let mut session =
        TurnOptionGeneratorSession::new(test_root(), TurnOptionGeneratorConfig::default());
    let GeneratorWork::Expand(parent) = session.pop_scheduled_work().expect("root expansion work")
    else {
        panic!("root work must be an expansion");
    };
    let anchor_head = session.push_work(
        GeneratorWork::Expand(parent.clone()),
        GeneratorWorkPriority::for_path(1, 0.0),
    );
    let guide_head = session.push_work(
        GeneratorWork::Expand(parent.clone()),
        GeneratorWorkPriority::for_path(1, 10.0),
    );
    let lane = CombatGuideLaneId::new(99);
    let guide_index = session.ensure_guide_frontier(lane);
    session.guided_frontiers[guide_index]
        .entries
        .push(GuidedGeneratorQueueEntry {
            guide_lane: lane,
            work_id: anchor_head,
            sequence_id: 10_000,
            guide_rank: CombatStateGuideRank::new(vec![0]),
            anchor_priority: GeneratorWorkPriority::for_path(1, 0.0),
        });
    session.guided_frontiers[guide_index]
        .entries
        .push(GuidedGeneratorQueueEntry {
            guide_lane: lane,
            work_id: guide_head,
            sequence_id: 10_001,
            guide_rank: CombatStateGuideRank::new(vec![10]),
            anchor_priority: GeneratorWorkPriority::for_path(1, 10.0),
        });

    session.next_scheduler_lane = 0;
    let first = session.advance(
        &EngineCombatStepper,
        CombatPlanningQuantum::deterministic(1, 250),
    );
    assert_eq!(
        first.status,
        TurnOptionGenerationStatus::Partial(GenerationInterruption::GenerationWorkBudget)
    );
    assert_eq!(
        session.scheduled_round.front().copied(),
        Some((1, guide_head)),
        "the unserved guide head remains frozen across the quantum boundary"
    );

    let newcomer = session.push_work(
        GeneratorWork::Expand(parent),
        GeneratorWorkPriority::for_path(1, 0.0),
    );
    session.guided_frontiers[guide_index]
        .entries
        .push(GuidedGeneratorQueueEntry {
            guide_lane: lane,
            work_id: newcomer,
            sequence_id: 10_002,
            guide_rank: CombatStateGuideRank::new(vec![20]),
            anchor_priority: GeneratorWorkPriority::for_path(1, 0.0),
        });

    session.advance(
        &EngineCombatStepper,
        CombatPlanningQuantum::deterministic(1, 250),
    );
    assert!(session.work[guide_head].is_none());
    assert!(session.work[newcomer].is_some());
}

#[test]
fn scheduling_rebuild_removes_only_buried_stale_entries() {
    let calls = Arc::new(AtomicUsize::new(0));
    let policy = Arc::new(CountingGenerationGuides { calls });
    let mut session = TurnOptionGeneratorSession::with_policy(
        test_root(),
        TurnOptionGeneratorConfig::default(),
        policy,
    );
    let GeneratorWork::Expand(parent) = session.pop_scheduled_work().expect("root expansion work")
    else {
        panic!("root work must be an expansion");
    };

    for negative_log_policy in [100.0, 110.0, 120.0] {
        let stale = session.push_work(
            GeneratorWork::Expand(parent.clone()),
            GeneratorWorkPriority::for_path(1, negative_log_policy),
        );
        session.take_live_work(stale);
    }
    let first_live = session.push_work(
        GeneratorWork::Expand(parent.clone()),
        GeneratorWorkPriority::for_path(1, 0.0),
    );
    let second_live = session.push_work(
        GeneratorWork::Expand(parent),
        GeneratorWorkPriority::for_path(1, 1.0),
    );

    assert_eq!(session.live_work_items, 2);
    assert_eq!(session.live_guide_entries, 2);
    assert_eq!(session.guide_entries_per_work.len(), session.work.len());
    let round_before = session.snapshot_scheduling_round();
    let anchor_entries_before = session.anchor_frontier.len();
    let guide_entries_before = session.guided_frontiers[0].entries.len();

    session.reclaim_stale_scheduling_entries();

    assert_eq!(session.snapshot_scheduling_round(), round_before);
    assert_eq!(
        round_before.front().map(|(_, work_id)| *work_id),
        Some(first_live)
    );
    assert!(session.work[first_live].is_some());
    assert!(session.work[second_live].is_some());
    assert_eq!(session.anchor_frontier.len(), 2);
    assert_eq!(session.guided_frontiers[0].entries.len(), 2);
    assert_eq!(session.scheduling_rebuilds, 1);
    assert_eq!(
        session.reclaimed_anchor_entries,
        anchor_entries_before.saturating_sub(2)
    );
    assert_eq!(
        session.reclaimed_guide_entries,
        guide_entries_before.saturating_sub(2)
    );
}

#[test]
fn finite_potion_allowance_is_part_of_generator_transposition_identity() {
    let root = test_root();
    let exact = combat_exact_state_key(&root.position().engine, &root.position().combat);
    let without_spend = IndexedExactStateKey::new(exact.clone(), Some(0));
    let after_one_spend = IndexedExactStateKey::new(exact, Some(1));

    assert_ne!(without_spend, after_one_spend);
    assert_eq!(
        HashSet::from([without_spend, after_one_spend]).len(),
        2,
        "equal simulator states with different remaining finite resources cannot transpose"
    );
}

#[test]
fn structural_hash_collision_still_compares_the_complete_typed_state() {
    let root = test_root();
    let position = root.position();
    let player_turn = combat_exact_state_key(&position.engine, &position.combat);
    let processing = combat_exact_state_key(&EngineState::CombatProcessing, &position.combat);
    let first = IndexedExactStateKey::new(player_turn, None);
    let mut collided = IndexedExactStateKey::new(processing, None);

    collided.structural_hash = first.structural_hash;

    assert_ne!(first, collided);
    assert_eq!(
        HashSet::from([first, collided]).len(),
        2,
        "a private structural-hash collision must not merge exact simulator states"
    );
}

#[test]
fn detail_timing_sampler_is_sparse_without_periodic_action_order_aliasing() {
    let sampled = (1..=16_384)
        .filter(|ordinal| detail_timing_scale(*ordinal).is_some())
        .collect::<Vec<_>>();

    assert!((900..=1_150).contains(&sampled.len()));
    assert!(
        sampled.windows(2).any(|pair| pair[1] - pair[0] != 16),
        "samples must not always select the same member of 16-wide action families"
    );
}
