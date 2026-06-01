use super::super::*;
use super::{enumerate::enumerate_turn_plans, types::*};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::sim::combat::{
    combat_terminal, CombatPosition, CombatStepLimits, CombatStepResult, CombatStepper,
};
use crate::test_support::{blank_test_combat, test_monster};

#[derive(Clone, Copy, Debug)]
enum TestTurnMode {
    PlayThenEnd,
    DirectNextTurnOutcomes,
}

#[derive(Clone, Copy, Debug)]
struct TestTurnStepper {
    mode: TestTurnMode,
}

impl CombatStepper for TestTurnStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
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
            (TestTurnMode::DirectNextTurnOutcomes, ClientInput::PlayCard { card_index, .. }) => {
                apply_direct_outcome(card_index, &mut combat);
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
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    }
}

fn test_combat_with_hand(hand_size: usize) -> CombatState {
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
    let ids = [
        CardId::Strike,
        CardId::Bash,
        CardId::Carnage,
        CardId::TwinStrike,
    ];
    combat.zones.hand = ids
        .iter()
        .copied()
        .take(hand_size)
        .enumerate()
        .map(|(idx, card_id)| CombatCard::new(card_id, 10 + idx as u32))
        .collect();
    combat
}
