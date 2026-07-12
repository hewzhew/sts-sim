use super::*;
use crate::ai::combat_search_v2::{
    COMBAT_SEARCH_V2_REPORT_SCHEMA_NAME, COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION,
};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use crate::test_support::{blank_test_combat, test_monster};

#[derive(Clone, Copy)]
struct PotionWinStepper;

#[test]
fn node_ordering_carries_only_the_best_retaliation_protection_into_frontier() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 4;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Defend, 10),
        CombatCard::new(CardId::HeavyBlade, 11),
        {
            let mut flame = CombatCard::new(CardId::FlameBarrier, 12);
            flame.upgrades = 1;
            flame
        },
    ];
    let mut spiker = test_monster(EnemyId::Spiker);
    spiker.id = 1;
    spiker.current_hp = 100;
    spiker.max_hp = 100;
    combat.entities.monsters = vec![spiker];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Thorns,
            instance_id: None,
            amount: 13,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    let node = SearchNode::root(EngineState::CombatPlayerTurn, combat.clone());
    let position = CombatPosition::new(node.engine.clone(), combat);
    let legal = EngineCombatStepper.legal_action_choices(&position);
    let config = CombatSearchV2Config::default();
    let mut loop_state = SearchLoopState::new(&config);

    let ordered = super::node_action_ordering::order_node_actions(
        &mut loop_state,
        &node,
        legal,
        None,
        &config,
    );

    assert!(matches!(
        ordered.ordered_choices[0].choice.choice.input,
        ClientInput::PlayCard {
            card_index: 2,
            target: None
        }
    ));
    assert_eq!(ordered.ordered_choices[0].action_ordering_frontier_hint, 1);
    assert!(ordered
        .ordered_choices
        .iter()
        .skip(1)
        .all(|choice| choice.action_ordering_frontier_hint == 0));
}

impl CombatStepper for PotionWinStepper {
    fn legal_actions(&self, _position: &CombatPosition) -> Vec<ClientInput> {
        vec![
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
            ClientInput::EndTurn,
        ]
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let engine = if matches!(input, ClientInput::UsePotion { .. }) {
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
struct ReversePotionWinStepper;

impl CombatStepper for ReversePotionWinStepper {
    fn legal_actions(&self, _position: &CombatPosition) -> Vec<ClientInput> {
        vec![
            ClientInput::EndTurn,
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
        ]
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let engine = if matches!(input, ClientInput::UsePotion { .. }) {
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
struct PendingChoiceResolveStepper;

impl CombatStepper for PendingChoiceResolveStepper {
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
            EngineState::CombatPlayerTurn
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
struct OneCardWinStepper;

impl CombatStepper for OneCardWinStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        if !matches!(position.engine, EngineState::CombatPlayerTurn)
            || position.combat.zones.hand.is_empty()
        {
            return Vec::new();
        }
        vec![ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        }]
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let mut combat = position.combat.clone();
        let mut engine = position.engine.clone();
        if matches!(input, ClientInput::PlayCard { .. }) {
            if let Some(monster) = combat.entities.monsters.first_mut() {
                monster.current_hp = 0;
            }
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
struct DuplicateUnresolvedChildStepper;

impl CombatStepper for DuplicateUnresolvedChildStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        if matches!(position.engine, EngineState::CombatPlayerTurn) {
            vec![ClientInput::EndTurn, ClientInput::Proceed]
        } else {
            Vec::new()
        }
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        _input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        crate::sim::combat::CombatStepResult {
            terminal: combat_terminal(&position.engine, &position.combat),
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
            position: position.clone(),
        }
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

#[derive(Clone, Copy)]
struct TwoTurnWinStepper;

impl CombatStepper for TwoTurnWinStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        if !matches!(position.engine, EngineState::CombatPlayerTurn) {
            return Vec::new();
        }
        if position.combat.turn.turn_count == 0 {
            vec![ClientInput::EndTurn]
        } else {
            vec![ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            }]
        }
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let mut combat = position.combat.clone();
        let mut engine = position.engine.clone();
        match input {
            ClientInput::EndTurn => {
                combat.turn.turn_count = combat.turn.turn_count.saturating_add(1);
                combat.entities.player.current_hp =
                    combat.entities.player.current_hp.saturating_sub(1);
            }
            ClientInput::PlayCard { .. } => {
                if let Some(monster) = combat.entities.monsters.first_mut() {
                    monster.current_hp = 0;
                }
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
struct WinOrDelayStepper;

impl CombatStepper for WinOrDelayStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        if !matches!(position.engine, EngineState::CombatPlayerTurn) {
            return Vec::new();
        }
        vec![
            ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
            ClientInput::EndTurn,
        ]
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let mut combat = position.combat.clone();
        let mut engine = position.engine.clone();
        match input {
            ClientInput::PlayCard { .. } => {
                if let Some(monster) = combat.entities.monsters.first_mut() {
                    monster.current_hp = 0;
                }
                engine = EngineState::GameOver(crate::state::core::RunResult::Victory);
            }
            ClientInput::EndTurn => {
                combat.turn.turn_count = combat.turn.turn_count.saturating_add(1);
                combat.entities.player.current_hp =
                    combat.entities.player.current_hp.saturating_sub(1);
            }
            _ => {}
        }
        let position = CombatPosition::new(engine, combat);
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
fn max_potions_used_cuts_potion_branches_without_disabling_policy_all() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let mut config = CombatSearchV2Config {
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_potions_used: Some(0),
        max_nodes: 8,
        ..CombatSearchV2Config::default()
    };

    let blocked = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        config.clone(),
        &PotionWinStepper,
    );

    assert!(!blocked.outcome.complete_trajectory_found);
    assert!(blocked.frontier.potion_budget_cut_count > 0);
    assert!(blocked
        .diagnostics
        .diagnosis
        .contains(&"potion_budget_cutoffs"));

    config.max_potions_used = Some(1);
    let allowed = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        config,
        &PotionWinStepper,
    );

    assert!(allowed.outcome.complete_trajectory_found);
    assert_eq!(
        allowed
            .best_complete_trajectory
            .as_ref()
            .map(|trajectory| trajectory.potions_used),
        Some(1)
    );
}

#[test]
fn search_report_declares_privileged_policy_evidence_boundary() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.draw_pile = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Defend, 12),
    ];
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::RunicDome));

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 1,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            ..CombatSearchV2Config::default()
        },
        &OneCardWinStepper,
    );

    assert_eq!(report.schema_name, COMBAT_SEARCH_V2_REPORT_SCHEMA_NAME);
    assert_eq!(
        report.schema_version,
        COMBAT_SEARCH_V2_REPORT_SCHEMA_VERSION
    );
    assert_eq!(
        report.policy_evidence.information_access,
        CombatSearchV2InformationAccess::PrivilegedSimulator
    );
    assert!(!report.policy_evidence.public_safe);
    assert!(report
        .policy_evidence
        .hidden_information_risks
        .contains(&CombatSearchV2HiddenInformationRisk::ExactDrawPileOrderWithoutFrozenEye));
    assert!(report
        .policy_evidence
        .hidden_information_risks
        .contains(&CombatSearchV2HiddenInformationRisk::ExactMonsterIntentUnderRunicDome));
}

#[test]
fn hp_loss_acceptance_threshold_stops_after_good_enough_complete_win() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
        crate::content::cards::CardId::Strike,
        100,
    )];
    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 20,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            stop_on_win_hp_loss_at_most: Some(0),
            ..CombatSearchV2Config::default()
        },
        &WinOrDelayStepper,
    );

    assert_eq!(
        report.outcome.coverage_status,
        SearchCoverageStatus::AcceptedCompleteCandidate
    );
    assert!(report.outcome.complete_trajectory_found);
    assert!(
        report.stats.nodes_expanded < 20,
        "search should stop before exhausting node budget after an accepted complete win"
    );
    assert_eq!(
        report
            .best_complete_trajectory
            .as_ref()
            .map(|trajectory| trajectory.hp_loss),
        Some(0)
    );
}

#[test]
fn zero_hp_loss_complete_win_stops_without_explicit_hp_gate() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
        crate::content::cards::CardId::Strike,
        100,
    )];
    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 20,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            stop_on_win_hp_loss_at_most: None,
            ..CombatSearchV2Config::default()
        },
        &WinOrDelayStepper,
    );

    assert_eq!(
        report.outcome.coverage_status,
        SearchCoverageStatus::AcceptedCompleteCandidate
    );
    assert_eq!(
        report
            .best_complete_trajectory
            .as_ref()
            .map(|trajectory| trajectory.hp_loss),
        Some(0)
    );
    assert!(
        report.stats.nodes_expanded < 20,
        "zero-loss complete win cannot be improved on hp loss, so search should stop early"
    );
}

#[test]
fn zero_hp_loss_complete_win_waits_when_external_payoff_remains_possible() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
        crate::content::cards::CardId::Feed,
        100,
    )];
    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 20,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            stop_on_win_hp_loss_at_most: None,
            ..CombatSearchV2Config::default()
        },
        &WinOrDelayStepper,
    );

    assert_ne!(
        report.outcome.coverage_status,
        SearchCoverageStatus::AcceptedCompleteCandidate,
        "default zero-loss early accept should not close search when combat-external payoff cards remain relevant"
    );
    assert!(report.outcome.complete_trajectory_found);
}

#[test]
fn action_ordering_preserves_original_action_id_in_trace() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let config = CombatSearchV2Config {
        potion_policy: CombatSearchV2PotionPolicy::All,
        max_nodes: 8,
        ..CombatSearchV2Config::default()
    };

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        config,
        &ReversePotionWinStepper,
    );

    let first_action_id = report
        .best_complete_trajectory
        .as_ref()
        .and_then(|trajectory| trajectory.actions.first())
        .map(|action| action.action_id);

    assert_eq!(first_action_id, Some(1));
}

#[test]
fn pending_choice_contract_counts_exact_child_resolution() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let config = CombatSearchV2Config {
        max_nodes: 1,
        rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
        ..CombatSearchV2Config::default()
    };

    let report = run_combat_search_v2_with_stepper(
        &EngineState::PendingChoice(crate::state::core::PendingChoice::StanceChoice),
        &combat,
        config,
        &PendingChoiceResolveStepper,
    );

    assert_eq!(report.diagnostics.pending_choice.pending_choice_states, 1);
    assert_eq!(
        report
            .diagnostics
            .pending_choice
            .expanded_pending_choice_states,
        1
    );
    assert_eq!(
        report
            .diagnostics
            .pending_choice
            .legal_actions_from_pending_choice,
        2
    );
    assert_eq!(report.diagnostics.pending_choice.resolved_children, 2);
    assert_eq!(report.diagnostics.pending_choice.still_pending_children, 0);
    assert!(report
        .diagnostics
        .pending_choice
        .ordering_role_counts
        .iter()
        .any(|role| role.role == "pending_choice_neutral_selection"
            && role.actions == 2
            && role.first_actions == 1));
    assert!(report
        .diagnostics
        .diagnosis
        .contains(&"pending_choice_contract_observed"));
}

#[test]
fn root_turn_plan_frontier_seed_remains_explicit_opt_in() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
        crate::content::cards::CardId::Strike,
        100,
    )];
    let base_config = CombatSearchV2Config {
        max_nodes: 1,
        rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
        ..CombatSearchV2Config::default()
    };

    let baseline = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        base_config.clone(),
        &OneCardWinStepper,
    );
    assert!(!baseline.outcome.complete_trajectory_found);
    assert_eq!(baseline.search_policy.turn_plan_policy, "disabled");
    assert_eq!(baseline.diagnostics.turn_plan.frontier_seeded_nodes, 0);
    assert_eq!(baseline.diagnostics.turn_plan.root_states_observed, 0);

    let seeded = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::RootFrontierSeed,
            ..base_config
        },
        &OneCardWinStepper,
    );

    assert!(seeded.outcome.complete_trajectory_found);
    assert_eq!(seeded.stats.nodes_to_first_win, Some(1));
    assert_eq!(seeded.diagnostics.turn_plan.frontier_seeded_nodes, 1);
    assert_eq!(
        seeded.diagnostics.turn_plan.behavioral_effect,
        "turn_plan_frontier_seed_exact_end_states_no_prune_no_terminal_claim"
    );
    assert!(seeded
        .diagnostics
        .diagnosis
        .contains(&"turn_plan_frontier_seeded"));
    assert_eq!(seeded.search_policy.turn_plan_policy, "root_frontier_seed");
}

#[test]
fn report_includes_search_performance_attribution_counts() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
        crate::content::cards::CardId::Strike,
        100,
    )];

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 2,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::RootFrontierSeed,
            ..CombatSearchV2Config::default()
        },
        &OneCardWinStepper,
    );

    assert!(report.performance.rollout_estimate_calls >= 1);
    assert_eq!(report.performance.root_rollout_estimate_calls, 1);
    assert_eq!(
        report.performance.rollout_estimate_calls,
        report
            .performance
            .root_rollout_estimate_calls
            .saturating_add(report.performance.child_rollout_estimate_calls)
            .saturating_add(report.performance.deferred_child_rollout_estimate_calls)
            .saturating_add(report.performance.turn_plan_seed_rollout_estimate_calls)
    );
    assert!(report.performance.frontier_pop_calls >= 1);
    assert_eq!(report.performance.turn_plan_frontier_seed_calls, 1);
    assert!(report.performance.total_elapsed_us >= report.performance.engine_step_elapsed_us);
    assert!(report.performance.total_elapsed_us >= report.performance.rollout_estimate_elapsed_us);
    assert!(report.performance.total_elapsed_us >= report.performance.shadow_audit_elapsed_us);
    assert!(
        report.performance.total_elapsed_us
            >= report.performance.root_turn_plan_diagnostics_elapsed_us
    );

    let exact_report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 2,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            ..CombatSearchV2Config::default()
        },
        &OneCardWinStepper,
    );

    assert!(exact_report.performance.engine_step_calls >= 1);
}

#[test]
fn terminal_children_skip_rollout_estimate() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 100)];

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 2,
            rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            ..CombatSearchV2Config::default()
        },
        &OneCardWinStepper,
    );

    assert_eq!(
        report.performance.rollout_estimate_calls, 1,
        "only the root needs a rollout estimate; the generated terminal child should use terminal value"
    );
    assert_eq!(report.performance.root_rollout_estimate_calls, 1);
    assert_eq!(report.performance.child_rollout_estimate_calls, 0);
    assert_eq!(report.performance.terminal_child_rollout_skips, 1);
    assert_eq!(report.stats.terminal_wins, 1);
}

#[test]
fn terminal_rollout_is_promoted_only_after_exact_replay() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 100)];

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 0,
            rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            ..CombatSearchV2Config::default()
        },
        &OneCardWinStepper,
    );

    assert_eq!(report.stats.nodes_expanded, 0);
    assert_eq!(report.rollout.terminal_wins, 1);
    assert!(
        report.outcome.complete_trajectory_found,
        "a complete terminal rollout should become evidence only after exact replay"
    );
    let trajectory = report
        .best_complete_trajectory
        .expect("exactly replayed rollout win");
    assert_eq!(trajectory.terminal, SearchTerminalLabel::Win);
    assert_eq!(trajectory.actions.len(), 1);
    assert_eq!(trajectory.actions[0].action_id, 0);
    assert_eq!(
        trajectory.actions[0].input,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        }
    );
}

#[test]
fn terminal_turn_plan_seeds_skip_rollout_estimate() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 100)];

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 1,
            rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::RootFrontierSeed,
            ..CombatSearchV2Config::default()
        },
        &OneCardWinStepper,
    );

    assert_eq!(report.performance.root_rollout_estimate_calls, 1);
    assert_eq!(report.performance.turn_plan_seed_rollout_estimate_calls, 0);
    assert!(
        report.performance.terminal_turn_plan_seed_rollout_skips >= 1,
        "terminal turn-plan seed nodes should use terminal value instead of rollout"
    );
}

#[test]
fn turn_local_dominance_pruned_children_skip_rollout_estimate() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 1,
            rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::Immediate,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            ..CombatSearchV2Config::default()
        },
        &DuplicateUnresolvedChildStepper,
    );

    assert_eq!(report.stats.turn_local_dominance_prunes, 1);
    assert_eq!(report.performance.child_rollout_estimate_calls, 1);
    assert_eq!(report.performance.turn_local_dominance_rollout_skips, 1);
}

#[test]
fn lazy_child_rollout_is_completed_from_frontier_pop() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 2,
            rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
            child_rollout_policy: CombatSearchV2ChildRolloutPolicy::LazyOnPop,
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            ..CombatSearchV2Config::default()
        },
        &DuplicateUnresolvedChildStepper,
    );

    assert_eq!(report.performance.child_rollout_estimate_calls, 0);
    assert!(report.performance.deferred_child_rollout_nodes >= 1);
    assert!(report.performance.deferred_child_rollout_estimate_calls >= 1);
    assert!(report.performance.deferred_child_rollout_requeues >= 1);
}

#[test]
fn config_and_turn_plan_policy_defaults_match() {
    assert_eq!(
        CombatSearchV2Config::default().turn_plan_policy,
        CombatSearchV2TurnPlanPolicy::default()
    );
    assert_eq!(
        CombatSearchV2Config::default().frontier_policy,
        CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets
    );
    assert_eq!(CombatSearchV2TurnPlanPolicy::default().label(), "disabled");
}

#[test]
fn default_turn_plan_policy_does_not_run_root_diagnostics() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 100)];

    let report = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            max_nodes: 2,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            ..CombatSearchV2Config::default()
        },
        &OneCardWinStepper,
    );

    assert_eq!(report.search_policy.turn_plan_policy, "disabled");
    assert_eq!(report.diagnostics.turn_plan.root_states_observed, 0);
    assert_eq!(report.performance.root_turn_plan_diagnostics_elapsed_us, 0);
}

#[test]
fn tactical_enemy_turn_plan_seed_gate_requires_healer_pair() {
    let mut combat = blank_test_combat();
    let mut healer = test_monster(EnemyId::Healer);
    healer.id = 2;
    combat.entities.monsters = vec![healer];

    assert!(!tactical_enemy_turn_plan_seed_gate(&test_search_node(
        combat.clone()
    )));

    let mut centurion = test_monster(EnemyId::Centurion);
    centurion.id = 1;
    combat.entities.monsters.push(centurion);

    assert!(tactical_enemy_turn_plan_seed_gate(&test_search_node(
        combat
    )));
}

#[test]
fn tactical_enemy_turn_plan_seed_gate_allows_fungi_swarm() {
    let mut combat = blank_test_combat();
    let mut first = test_monster(EnemyId::FungiBeast);
    first.id = 1;
    let mut second = test_monster(EnemyId::FungiBeast);
    second.id = 2;
    combat.entities.monsters = vec![first, second];

    assert!(!tactical_enemy_turn_plan_seed_gate(&test_search_node(
        combat.clone()
    )));

    let mut third = test_monster(EnemyId::FungiBeast);
    third.id = 3;
    combat.entities.monsters.push(third);

    assert!(tactical_enemy_turn_plan_seed_gate(&test_search_node(
        combat
    )));
}

#[test]
fn tactical_enemy_turn_plan_seed_gate_allows_boss_and_elite_boundaries() {
    let mut combat = blank_test_combat();
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];

    assert!(!tactical_enemy_turn_plan_seed_gate(&test_search_node(
        combat.clone()
    )));

    combat.meta.is_boss_fight = true;
    assert!(tactical_enemy_turn_plan_seed_gate(&test_search_node(
        combat.clone()
    )));

    combat.meta.is_boss_fight = false;
    combat.meta.is_elite_fight = true;
    assert!(tactical_enemy_turn_plan_seed_gate(&test_search_node(
        combat
    )));
}

#[test]
fn tactical_enemy_turn_plan_seed_gate_allows_visible_high_pressure() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 10;
    combat.entities.player.block = 0;
    let mut jaw_worm = test_monster(EnemyId::JawWorm);
    jaw_worm.set_planned_move_id(1);
    combat.entities.monsters = vec![jaw_worm];

    assert!(tactical_enemy_turn_plan_seed_gate(&test_search_node(
        combat
    )));
}

#[test]
fn tactical_turn_plan_policy_seeds_exact_states_with_turn_plan_prior_hints() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 80;
    combat.entities.player.block = 0;
    combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    let node = test_search_node(combat);
    assert!(
        !tactical_enemy_turn_plan_seed_gate(&node),
        "fixture should be an ordinary low-pressure fight"
    );
    let state_hash = combat_exact_state_hash_v1(&node.engine, &node.combat);
    let config = CombatSearchV2Config {
        turn_plan_policy: CombatSearchV2TurnPlanPolicy::TacticalEnemyTurnBoundaryFrontierSeed,
        turn_plan_prior: Some(CombatSearchV2TurnPlanPrior::from_plan_scores([(
            state_hash,
            [(vec!["combat/end_turn".to_string()], 1.0)],
        )])),
        ..CombatSearchV2Config::default()
    };

    assert!(
        should_seed_turn_plan_at_node(
            &node,
            &config,
            &CombatSearchPluginStack::from_config(&config)
        ),
        "exact-state turn-plan hints should make tactical policy enumerate known prior states"
    );
}

fn test_search_node(combat: CombatState) -> SearchNode {
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
    }
}

#[test]
fn turn_boundary_frontier_seed_extends_beyond_root_when_explicitly_enabled() {
    let mut combat = blank_test_combat();
    combat.turn.turn_count = 0;
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    monster.current_hp = 200;
    monster.max_hp = 200;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![crate::runtime::combat::CombatCard::new(
        crate::content::cards::CardId::Strike,
        100,
    )];
    let base_config = CombatSearchV2Config {
        max_nodes: 4,
        rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
        ..CombatSearchV2Config::default()
    };

    let root_only = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::RootFrontierSeed,
            ..base_config.clone()
        },
        &TwoTurnWinStepper,
    );

    let turn_boundary = run_combat_search_v2_with_stepper(
        &EngineState::CombatPlayerTurn,
        &combat,
        CombatSearchV2Config {
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::TurnBoundaryFrontierSeed,
            ..base_config
        },
        &TwoTurnWinStepper,
    );

    assert_eq!(root_only.diagnostics.turn_plan.frontier_seeded_nodes, 0);
    assert!(
        turn_boundary.diagnostics.turn_plan.frontier_seeded_nodes
            > root_only.diagnostics.turn_plan.frontier_seeded_nodes
    );
    assert_eq!(
        turn_boundary.search_policy.turn_plan_policy,
        "turn_boundary_frontier_seed"
    );
}
