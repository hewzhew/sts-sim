use super::*;
use crate::content::monsters::EnemyId;
use crate::test_support::{blank_test_combat, test_monster};

#[derive(Clone, Copy)]
struct FirstActionWinsStepper;

impl CombatStepper for FirstActionWinsStepper {
    fn legal_actions(&self, _position: &CombatPosition) -> Vec<ClientInput> {
        vec![ClientInput::EndTurn]
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let engine = if matches!(input, ClientInput::EndTurn) {
            EngineState::GameOver(crate::state::core::RunResult::Victory)
        } else {
            position.engine.clone()
        };
        let position = CombatPosition::new(engine, position.combat.clone());
        crate::sim::combat::CombatStepResult {
            terminal: combat_terminal(&position.engine, &position.combat),
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
            position,
        }
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

#[derive(Clone, Copy)]
struct PendingChoiceWinsStepper;

impl CombatStepper for PendingChoiceWinsStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        if matches!(position.engine, EngineState::PendingChoice(_)) {
            vec![
                ClientInput::SubmitDiscoverChoice(0),
                ClientInput::SubmitDiscoverChoice(1),
            ]
        } else {
            Vec::new()
        }
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let engine = if matches!(input, ClientInput::SubmitDiscoverChoice(_)) {
            EngineState::GameOver(crate::state::core::RunResult::Victory)
        } else {
            position.engine.clone()
        };
        let position = CombatPosition::new(engine, position.combat.clone());
        crate::sim::combat::CombatStepResult {
            terminal: combat_terminal(&position.engine, &position.combat),
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
            position,
        }
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

#[test]
fn conservative_rollout_records_estimated_terminal_win() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let node = SearchNode {
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
    };
    let config = CombatSearchV2Config::default();

    let estimate = conservative_no_potion_rollout(&node, &FirstActionWinsStepper, &config, 4, None);

    assert!(estimate.evaluated);
    assert_eq!(estimate.terminal, SearchTerminalLabel::Win);
    assert!(!estimate.truncated);
}

#[test]
fn rollout_cache_reuses_exact_state_estimate() {
    let mut cache = RolloutCache::new(CombatSearchV2RolloutPolicy::ConservativeNoPotion, 4, 4);
    let node = SearchNode {
        engine: EngineState::CombatPlayerTurn,
        combat: blank_test_combat(),
        actions: Vec::new(),
        turn_prefix: TurnPrefixState::default(),
        initial_hp: 80,
        potions_used: 0,
        potions_discarded: 0,
        cards_played: 0,
        potion_tactical_priority: 0,
        last_turn_branch_priority: 0,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    };
    let config = CombatSearchV2Config::default();

    let first = cache.estimate(&node, &FirstActionWinsStepper, &config, None);
    let second = cache.estimate(&node, &FirstActionWinsStepper, &config, None);

    assert_eq!(first, second);
    assert_eq!(cache.evaluations, 1);
    assert_eq!(cache.cache_hits, 1);
}

#[test]
fn rollout_estimate_records_phase_adjusted_enemy_effort() {
    let mut combat = blank_test_combat();
    let mut guardian = test_monster(EnemyId::TheGuardian);
    guardian.id = 1;
    guardian.current_hp = 180;
    guardian.max_hp = 240;
    guardian.block = 20;
    guardian.guardian.is_open = false;
    combat.entities.monsters = vec![guardian];
    let node = SearchNode {
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
    };

    let estimate = RolloutNodeEstimate::from_node(
        &node,
        0,
        RolloutStopReason::MaxActions,
        None,
        RolloutPendingChoiceProgress::default(),
    );

    assert_eq!(estimate.total_enemy_hp, 180);
    assert_eq!(estimate.total_enemy_block, 20);
    assert_eq!(estimate.phase_adjusted_enemy_effort, 200);
}

#[test]
fn conservative_rollout_stops_before_large_pending_choice_branch() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let node = SearchNode {
        engine: EngineState::PendingChoice(crate::state::core::PendingChoice::ScrySelect {
            cards: vec![crate::content::cards::CardId::Strike; 7],
            card_uuids: (0..7).collect(),
        }),
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
    };
    let config = CombatSearchV2Config::default();

    let estimate = conservative_no_potion_rollout(&node, &FirstActionWinsStepper, &config, 4, None);

    assert!(estimate.evaluated);
    assert!(estimate.truncated);
    assert_eq!(
        estimate.stop_reason,
        RolloutStopReason::HighFanoutPendingChoice
    );
    assert!(estimate.high_fanout_pending_choice);
    assert_eq!(estimate.pending_choice_estimated_action_fanout, 128);
    assert_eq!(estimate.pending_choices_seen, 1);
    assert_eq!(estimate.pending_choice_actions_simulated, 0);
    assert_eq!(estimate.max_pending_choice_candidate_count, 7);
    assert_eq!(estimate.max_pending_choice_estimated_action_fanout, 128);
    assert_eq!(estimate.last_pending_choice_kind, Some("scry_select"));
    assert!(estimate.stopped_on_high_fanout_pending_choice);
    assert_eq!(estimate.actions_simulated, 0);
}

#[test]
fn conservative_rollout_tracks_small_pending_choice_resolution() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let node = SearchNode {
        engine: EngineState::PendingChoice(crate::state::core::PendingChoice::StanceChoice),
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
    };
    let config = CombatSearchV2Config::default();

    let estimate =
        conservative_no_potion_rollout(&node, &PendingChoiceWinsStepper, &config, 4, None);

    assert_eq!(estimate.terminal, SearchTerminalLabel::Win);
    assert!(!estimate.truncated);
    assert_eq!(estimate.pending_choices_seen, 1);
    assert_eq!(estimate.pending_choice_actions_simulated, 1);
    assert_eq!(estimate.max_pending_choice_candidate_count, 2);
    assert_eq!(estimate.max_pending_choice_estimated_action_fanout, 2);
    assert_eq!(estimate.last_pending_choice_kind, Some("stance_choice"));
    assert!(!estimate.stopped_on_high_fanout_pending_choice);
    assert_eq!(estimate.actions_simulated, 1);
}
