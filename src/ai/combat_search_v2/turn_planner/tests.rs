use super::super::*;
use super::{diagnostics::TurnPlanDiagnosticsCollector, enumerate::enumerate_turn_plans, types::*};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::runtime::monster_move::{AttackSpec, DamageKind, MonsterMoveSpec};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepResult, CombatStepper,
    EngineCombatStepper,
};
use crate::test_support::{blank_test_combat, test_monster};

#[derive(Clone, Copy, Debug)]
enum TestTurnMode {
    PlayThenEnd,
    WideDepthWin,
    DirectNextTurnOutcomes,
    EqualNextTurnOutcomes,
    DangerRepairOutcomes,
}

#[derive(Clone, Copy, Debug)]
struct TestTurnStepper {
    mode: TestTurnMode,
}

impl CombatStepper for TestTurnStepper {
    fn atomic_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        match self.mode {
            TestTurnMode::PlayThenEnd => {
                if !matches!(position.engine, EngineState::CombatPlayerTurn)
                    || position.combat.turn.turn_count > 0
                {
                    return Vec::new();
                }
                let mut actions = Vec::new();
                if position.combat.turn.energy > 0 && !position.combat.zones.hand.is_empty() {
                    actions.push(ClientInput::PlayCard {
                        card_index: 0,
                        target: Some(1),
                    });
                }
                actions.push(ClientInput::EndTurn);
                actions
            }
            TestTurnMode::WideDepthWin => {
                if !matches!(position.engine, EngineState::CombatPlayerTurn)
                    || position.combat.turn.turn_count > 0
                {
                    return Vec::new();
                }
                let mut actions = if position.combat.turn.energy > 0 {
                    position
                        .combat
                        .zones
                        .hand
                        .iter()
                        .enumerate()
                        .map(|(card_index, _)| ClientInput::PlayCard {
                            card_index,
                            target: Some(1),
                        })
                        .collect::<Vec<_>>()
                } else {
                    Vec::new()
                };
                actions.push(ClientInput::EndTurn);
                actions
            }
            TestTurnMode::DirectNextTurnOutcomes => position
                .combat
                .zones
                .hand
                .iter()
                .enumerate()
                .map(|(card_index, _)| ClientInput::PlayCard {
                    card_index,
                    target: Some(1),
                })
                .collect(),
            TestTurnMode::EqualNextTurnOutcomes => position
                .combat
                .zones
                .hand
                .iter()
                .enumerate()
                .map(|(card_index, _)| ClientInput::PlayCard {
                    card_index,
                    target: Some(1),
                })
                .collect(),
            TestTurnMode::DangerRepairOutcomes => position
                .combat
                .zones
                .hand
                .iter()
                .enumerate()
                .map(|(card_index, _)| ClientInput::PlayCard {
                    card_index,
                    target: Some(1),
                })
                .collect(),
        }
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> CombatStepResult {
        let mut combat = position.combat.clone();
        let mut engine = position.engine.clone();
        match (self.mode, input) {
            (TestTurnMode::PlayThenEnd, ClientInput::PlayCard { card_index, .. }) => {
                move_hand_card_to_discard(card_index, &mut combat);
                combat.turn.energy = combat.turn.energy.saturating_sub(1);
                if let Some(monster) = combat.entities.monsters.first_mut() {
                    monster.current_hp = monster.current_hp.saturating_sub(10);
                }
            }
            (TestTurnMode::PlayThenEnd, ClientInput::EndTurn) => {
                combat.turn.turn_count = combat.turn.turn_count.saturating_add(1);
            }
            (TestTurnMode::WideDepthWin, ClientInput::PlayCard { card_index, .. }) => {
                let card_marker = combat
                    .zones
                    .hand
                    .get(card_index)
                    .map(|card| (card.uuid % 10) as i32)
                    .unwrap_or_default();
                move_hand_card_to_discard(card_index, &mut combat);
                combat.turn.energy = combat.turn.energy.saturating_sub(1);
                combat.entities.player.block = combat
                    .entities
                    .player
                    .block
                    .saturating_mul(10)
                    .saturating_add(card_marker);
                if let Some(monster) = combat.entities.monsters.first_mut() {
                    monster.current_hp = monster.current_hp.saturating_sub(10);
                }
            }
            (TestTurnMode::WideDepthWin, ClientInput::EndTurn) => {
                combat.turn.turn_count = combat.turn.turn_count.saturating_add(1);
            }
            (TestTurnMode::DirectNextTurnOutcomes, ClientInput::PlayCard { card_index, .. }) => {
                apply_direct_outcome(card_index, &mut combat);
            }
            (TestTurnMode::EqualNextTurnOutcomes, ClientInput::PlayCard { card_index, .. }) => {
                apply_equal_outcome(card_index, &mut combat);
            }
            (TestTurnMode::DangerRepairOutcomes, ClientInput::PlayCard { card_index, .. }) => {
                apply_danger_repair_outcome(card_index, &mut combat);
            }
            _ => {}
        }
        if combat
            .entities
            .monsters
            .iter()
            .all(|monster| !monster.is_alive_for_action())
        {
            engine = EngineState::GameOver(crate::state::core::RunResult::Victory);
        }
        let position = CombatPosition::new(engine, combat);
        CombatStepResult {
            terminal: combat_terminal(&position.engine, &position.combat),
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
            position,
        }
    }

    fn terminal(&self, position: &CombatPosition) -> crate::sim::combat::CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

#[test]
fn turn_planner_enumerates_same_turn_prefix_until_next_turn_boundary() {
    let root = test_node(test_combat_with_hand(1));
    let plans = enumerate_turn_plans(
        &root,
        &TestTurnStepper {
            mode: TestTurnMode::PlayThenEnd,
        },
        &TurnPlannerConfigV1::default(),
        None,
    );

    assert!(plans
        .plans
        .iter()
        .any(|plan| plan.stop_reason == TurnPlanStopReason::NextTurn && plan.actions.len() == 2));
    assert!(plans.nodes_expanded >= 2);
    assert!(plans.nodes_generated >= 3);
}

#[test]
fn turn_planner_step_trace_capture_does_not_change_selected_plans() {
    let root = test_node(test_combat_with_hand(1));
    let stepper = TestTurnStepper {
        mode: TestTurnMode::PlayThenEnd,
    };
    let traced = enumerate_turn_plans(&root, &stepper, &TurnPlannerConfigV1::default(), None);
    let untraced = enumerate_turn_plans(
        &root,
        &stepper,
        &TurnPlannerConfigV1 {
            capture_step_trace: false,
            ..TurnPlannerConfigV1::default()
        },
        None,
    );
    let plan_keys = |plans: &TurnPlanEnumeration| {
        plans
            .plans
            .iter()
            .map(|plan| {
                (
                    plan.bucket,
                    plan.stop_reason,
                    plan.actions
                        .iter()
                        .map(|action| action.action_key.clone())
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };

    assert_eq!(plan_keys(&traced), plan_keys(&untraced));
    assert!(traced
        .plans
        .iter()
        .all(|plan| plan.step_states.len() == plan.actions.len()));
    assert!(untraced
        .plans
        .iter()
        .all(|plan| plan.step_states.is_empty()));
}

#[test]
fn turn_planner_reaches_deep_plan_within_wide_branching_budget() {
    let mut combat = test_combat_with_cards(&[
        CardId::Strike,
        CardId::Bash,
        CardId::Carnage,
        CardId::TwinStrike,
        CardId::Defend,
        CardId::ShrugItOff,
        CardId::Inflame,
        CardId::BattleTrance,
    ]);
    combat.turn.energy = 4;
    combat.entities.monsters[0].current_hp = 40;
    combat.entities.monsters[0].max_hp = 40;
    let root = test_node(combat);
    let plans = enumerate_turn_plans(
        &root,
        &TestTurnStepper {
            mode: TestTurnMode::WideDepthWin,
        },
        &TurnPlannerConfigV1 {
            max_inner_nodes: 64,
            max_end_states: 8,
            per_bucket_limit: 8,
            ..TurnPlannerConfigV1::default()
        },
        None,
    );

    assert!(
        plans.plans.iter().any(|plan| {
            plan.stop_reason == TurnPlanStopReason::Terminal && plan.actions.len() == 4
        }),
        "bounded planner should reach a four-action terminal instead of exhausting the budget on shallow permutations"
    );
}

#[test]
fn turn_planner_uses_combat_eval_to_rank_stable_progress_over_hp_stall() {
    let root = test_node(test_combat_with_hand(2));
    let plans = enumerate_turn_plans(
        &root,
        &TestTurnStepper {
            mode: TestTurnMode::DirectNextTurnOutcomes,
        },
        &TurnPlannerConfigV1::default(),
        None,
    );

    let best = plans.plans.first().expect("plans should be generated");

    assert_eq!(best.bucket, TurnPlanBucket::Progress);
    assert!(matches!(
        best.actions[0].input,
        ClientInput::PlayCard { card_index: 1, .. }
    ));
}

#[test]
fn turn_plan_prior_reorders_equal_eval_plans_without_pruning() {
    let root = test_node(test_combat_with_hand(2));
    let root_hash = combat_exact_state_hash_v1(&root.engine, &root.combat);
    let preferred_action_key = "combat/play_card/hand:1/card:Bash+0#11/target:monster_slot:0";
    let other_action_key = "combat/play_card/hand:0/card:Strike_R+0#10/target:monster_slot:0";
    let plans = enumerate_turn_plans(
        &root,
        &TestTurnStepper {
            mode: TestTurnMode::EqualNextTurnOutcomes,
        },
        &TurnPlannerConfigV1 {
            turn_plan_prior: Some(CombatSearchV2TurnPlanPrior::from_plan_scores([(
                root_hash,
                [
                    (vec![preferred_action_key.to_string()], 0.9),
                    (vec![other_action_key.to_string()], 0.1),
                ],
            )])),
            ..TurnPlannerConfigV1::default()
        },
        None,
    );

    assert_eq!(plans.preselection_plan_count, 2);
    assert_eq!(
        plans
            .plans
            .iter()
            .map(|plan| plan.actions[0].action_key.as_str())
            .collect::<Vec<_>>(),
        vec![preferred_action_key, other_action_key]
    );
    assert_eq!(plans.turn_plan_prior_scored_plans, 2);
    let best = plans.plans.first().expect("plans should be generated");
    assert_eq!(
        best.actions
            .first()
            .map(|action| action.action_key.as_str()),
        Some(preferred_action_key)
    );
}

#[test]
fn turn_planner_retains_different_objective_buckets_before_filling_overall() {
    let root = test_node(test_combat_with_hand(3));
    let plans = enumerate_turn_plans(
        &root,
        &TestTurnStepper {
            mode: TestTurnMode::DirectNextTurnOutcomes,
        },
        &TurnPlannerConfigV1 {
            max_end_states: 2,
            per_bucket_limit: 1,
            ..TurnPlannerConfigV1::default()
        },
        None,
    );

    assert_eq!(plans.plans.len(), 2);
    assert!(plans
        .plans
        .iter()
        .any(|plan| plan.bucket == TurnPlanBucket::Progress));
    assert!(plans
        .plans
        .iter()
        .any(|plan| plan.bucket == TurnPlanBucket::Survival));
}

#[test]
fn turn_planner_classifies_repaired_danger_as_survival_plan() {
    let mut combat = test_combat_with_hand(2);
    combat.entities.player.current_hp = 10;
    combat.entities.player.block = 0;
    combat.entities.monsters[0].set_planned_visible_spec(Some(MonsterMoveSpec::Attack(
        AttackSpec {
            base_damage: 20,
            hits: 1,
            damage_kind: DamageKind::Normal,
        },
    )));
    let root = test_node(combat);
    let plans = enumerate_turn_plans(
        &root,
        &TestTurnStepper {
            mode: TestTurnMode::DangerRepairOutcomes,
        },
        &TurnPlannerConfigV1 {
            max_end_states: 2,
            per_bucket_limit: 1,
            ..TurnPlannerConfigV1::default()
        },
        None,
    );

    assert!(
        plans
            .plans
            .iter()
            .any(|plan| plan.bucket == TurnPlanBucket::Survival),
        "danger-repair bucket should be retained"
    );
}

#[test]
fn turn_planner_selection_audit_reports_bucket_cap_drops() {
    let root = test_node(test_combat_with_hand(3));
    let plans = enumerate_turn_plans(
        &root,
        &TestTurnStepper {
            mode: TestTurnMode::EqualNextTurnOutcomes,
        },
        &TurnPlannerConfigV1 {
            max_end_states: 3,
            per_bucket_limit: 1,
            ..TurnPlannerConfigV1::default()
        },
        None,
    );

    assert_eq!(plans.preselection_plan_count, 3);
    assert_eq!(plans.plans.len(), 1);
    assert_eq!(plans.selection_audit.candidates.len(), 3);
    assert_eq!(
        plans
            .selection_audit
            .candidates
            .iter()
            .filter(|candidate| candidate.outcome == TurnPlanCandidateSelectionOutcomeV1::Dropped)
            .filter(|candidate| {
                candidate.drop_reason == Some(TurnPlanCandidateDropReasonV1::BucketCap)
            })
            .count(),
        2
    );
    assert_eq!(
        plans
            .selection_audit
            .coverage_groups
            .iter()
            .map(|group| group.preselection_count)
            .sum::<usize>(),
        3
    );
    assert_eq!(
        plans
            .selection_audit
            .coverage_groups
            .iter()
            .map(|group| group.selected_count)
            .sum::<usize>(),
        1
    );
    assert_eq!(
        plans
            .selection_audit
            .coverage_groups
            .iter()
            .map(|group| group.bucket_cap_dropped_count)
            .sum::<usize>(),
        2
    );
}

#[test]
fn turn_planner_coverage_signature_reports_vulnerable_setup_from_bash() {
    let mut combat = test_combat_with_cards(&[CardId::Bash]);
    combat.turn.energy = 2;
    let root = test_node(combat);
    let plans = enumerate_turn_plans(
        &root,
        &EngineCombatStepper,
        &TurnPlannerConfigV1 {
            max_end_states: 4,
            per_bucket_limit: 4,
            ..TurnPlannerConfigV1::default()
        },
        None,
    );

    let bash_candidate = plans
        .selection_audit
        .candidates
        .iter()
        .find(|candidate| {
            candidate
                .action_keys
                .iter()
                .any(|action| action.contains("card:Bash"))
        })
        .expect("Bash turn plan should be audited");

    assert!(
        bash_candidate.coverage_signature.enemy_vulnerable_added > 0,
        "coverage signature must expose vulnerable setup from action facts"
    );
    assert!(bash_candidate.coverage_signature.damage_done > 0);
}

#[test]
fn turn_planner_coverage_signature_reports_power_setup() {
    let mut combat = test_combat_with_cards(&[CardId::Berserk]);
    combat.turn.energy = 1;
    let root = test_node(combat);
    let plans = enumerate_turn_plans(
        &root,
        &EngineCombatStepper,
        &TurnPlannerConfigV1 {
            max_end_states: 4,
            per_bucket_limit: 4,
            ..TurnPlannerConfigV1::default()
        },
        None,
    );

    let power_candidate = plans
        .selection_audit
        .candidates
        .iter()
        .find(|candidate| {
            candidate
                .action_keys
                .iter()
                .any(|action| action.contains("card:Berserk"))
        })
        .expect("Berserk turn plan should be audited");

    assert_eq!(power_candidate.coverage_key.setup.label(), "power");
}

#[test]
fn turn_plan_diagnostics_reports_root_plan_preview_without_behavior_claim() {
    let root = test_node(test_combat_with_hand(3));
    let mut collector = TurnPlanDiagnosticsCollector::default();

    collector.observe_root(
        &root,
        &TestTurnStepper {
            mode: TestTurnMode::DirectNextTurnOutcomes,
        },
    );
    let report = collector.finish();

    assert_eq!(
        report.behavioral_effect,
        "diagnostic_only_no_frontier_steering_no_prune_no_terminal_claim"
    );
    assert_eq!(report.root_states_observed, 1);
    assert!(report.total_plans >= 2);
    assert!(report
        .bucket_counts
        .iter()
        .any(|count| count.label == "progress"));
    assert!(report
        .samples
        .first()
        .and_then(|sample| sample.top_plans.first())
        .and_then(|plan| plan.first_action_key.as_ref())
        .is_some());
}

fn apply_direct_outcome(card_index: usize, combat: &mut CombatState) {
    move_hand_card_to_discard(card_index, combat);
    match card_index {
        0 => {
            combat.entities.player.current_hp = 55;
            combat.entities.player.block = 25;
            combat.entities.monsters[0].current_hp = 180;
        }
        1 => {
            combat.entities.player.current_hp = 40;
            combat.entities.player.block = 20;
            combat.entities.monsters[0].current_hp = 10;
        }
        2 => {
            combat.entities.player.current_hp = 4;
            combat.entities.player.block = 0;
            combat.entities.monsters[0].current_hp = 180;
        }
        _ => {}
    }
    combat.turn.turn_count = combat.turn.turn_count.saturating_add(1);
}

fn apply_equal_outcome(card_index: usize, combat: &mut CombatState) {
    move_hand_card_to_discard(card_index, combat);
    combat.entities.player.current_hp = 60;
    combat.entities.player.block = 10;
    combat.entities.monsters[0].current_hp = 100;
    combat.turn.turn_count = combat.turn.turn_count.saturating_add(1);
}

fn apply_danger_repair_outcome(card_index: usize, combat: &mut CombatState) {
    move_hand_card_to_discard(card_index, combat);
    match card_index {
        0 => {
            combat.entities.player.current_hp = 10;
            combat.entities.player.block = 25;
            combat.entities.monsters[0].current_hp = 190;
        }
        1 => {
            combat.entities.player.current_hp = 10;
            combat.entities.player.block = 0;
            combat.entities.monsters[0].current_hp = 30;
        }
        _ => {}
    }
    combat.turn.turn_count = combat.turn.turn_count.saturating_add(1);
}

fn move_hand_card_to_discard(card_index: usize, combat: &mut CombatState) {
    if card_index < combat.zones.hand.len() {
        let card = combat.zones.hand.remove(card_index);
        combat.zones.add_to_discard_pile_top(card);
    }
}

fn test_node(combat: CombatState) -> SearchNode {
    SearchNode {
        engine: EngineState::CombatPlayerTurn,
        combat,
        actions: Vec::new(),
        turn_prefix: TurnPrefixState::default(),
        initial_hp: 80,
        potions_used: 0,
        potions_discarded: 0,
        cards_played: 0,
        potion_tactical_priority: 0,
        last_turn_branch_priority: 0,
        action_prior_score: None,
        action_ordering_frontier_hint: 0,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
        root_lineage: Default::default(),
    }
}

fn test_combat_with_hand(hand_size: usize) -> CombatState {
    let ids = [
        CardId::Strike,
        CardId::Bash,
        CardId::Carnage,
        CardId::TwinStrike,
    ];
    test_combat_with_cards(&ids[..hand_size.min(ids.len())])
}

fn test_combat_with_cards(ids: &[CardId]) -> CombatState {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 80;
    combat.turn.turn_count = 0;
    combat.turn.energy = 1;
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    monster.set_planned_move_id(1);
    monster.current_hp = 200;
    monster.max_hp = 200;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = ids
        .iter()
        .copied()
        .enumerate()
        .map(|(idx, card_id)| CombatCard::new(card_id, 10 + idx as u32))
        .collect();
    combat
}
