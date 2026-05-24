use super::*;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::{Power, PowerPayload};
use crate::sim::combat::{CombatPosition, CombatStepLimits};
use crate::test_support::{blank_test_combat, test_monster};

#[derive(Clone, Copy)]
struct ProbeStepper {
    damage_on_card_index: Option<usize>,
    player_hp_loss_on_damage: i32,
}

impl CombatStepper for ProbeStepper {
    fn legal_actions(&self, _position: &CombatPosition) -> Vec<ClientInput> {
        Vec::new()
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> crate::sim::combat::CombatStepResult {
        let mut combat = position.combat.clone();
        if let ClientInput::PlayCard { card_index, .. } = input {
            if Some(card_index) == self.damage_on_card_index {
                if let Some(monster) = combat.entities.monsters.first_mut() {
                    monster.current_hp = monster.current_hp.saturating_sub(10);
                }
                combat.entities.player.current_hp = combat
                    .entities
                    .player
                    .current_hp
                    .saturating_sub(self.player_hp_loss_on_damage);
            }
        }
        let position = CombatPosition::new(position.engine.clone(), combat);
        crate::sim::combat::CombatStepResult {
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
fn one_step_probe_can_choose_nonterminal_special_phase_value_upgrade() {
    let combat = split_debt_combat();
    let node = test_node(combat.clone());
    let ordered = vec![
        indexed_choice(&combat, 0, ClientInput::EndTurn),
        indexed_choice(
            &combat,
            1,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ),
    ];

    let (selection, reason) = choose_by_one_step_probe(
        &node,
        &ProbeStepper {
            damage_on_card_index: Some(1),
            player_hp_loss_on_damage: 0,
        },
        &test_config(),
        None,
        &ordered,
        true,
    )
    .expect("phase progress without hp regression should be eligible");

    assert_eq!(selection.original_action_id, 1);
    assert_eq!(
        reason,
        super::super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_PHASE_VALUE
    );
}

#[test]
fn one_step_probe_rejects_phase_upgrade_with_hp_regression() {
    let combat = split_debt_combat();
    let node = test_node(combat.clone());
    let ordered = vec![
        indexed_choice(&combat, 0, ClientInput::EndTurn),
        indexed_choice(
            &combat,
            1,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ),
    ];

    let selection = choose_by_one_step_probe(
        &node,
        &ProbeStepper {
            damage_on_card_index: Some(1),
            player_hp_loss_on_damage: 1,
        },
        &test_config(),
        None,
        &ordered,
        true,
    );

    assert!(selection.is_none());
}

#[test]
fn one_step_probe_terminal_only_mode_rejects_nonterminal_phase_upgrade() {
    let combat = split_debt_combat();
    let node = test_node(combat.clone());
    let ordered = vec![
        indexed_choice(&combat, 0, ClientInput::EndTurn),
        indexed_choice(
            &combat,
            1,
            ClientInput::PlayCard {
                card_index: 1,
                target: Some(1),
            },
        ),
    ];

    let selection = choose_by_one_step_probe(
        &node,
        &ProbeStepper {
            damage_on_card_index: Some(1),
            player_hp_loss_on_damage: 0,
        },
        &test_config(),
        None,
        &ordered,
        false,
    );

    assert!(selection.is_none());
}

#[test]
fn probe_upgrade_reason_accepts_hp_gain_as_survival_value() {
    let fallback = score_with_survival(30, 5, 0);
    let candidate = score_with_survival(31, 5, 0);

    assert_eq!(
        probe_upgrade_reason(candidate, fallback, true),
        Some(
            super::super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_SURVIVAL_VALUE
        )
    );
}

#[test]
fn probe_upgrade_reason_rejects_block_only_survival_value_without_visible_hp_loss_reduction() {
    let fallback = score_with_survival(30, 30, 0);
    let candidate = score_with_survival(30, 35, 0);

    assert_eq!(probe_upgrade_reason(candidate, fallback, true), None);
}

#[test]
fn probe_upgrade_reason_rejects_nonterminal_end_turn_phase_upgrade() {
    let fallback = score_with_survival(30, 5, 0);
    let mut candidate = fallback;
    candidate.split_debt_stability = 10;
    candidate.nonterminal_upgrade_eligible = false;

    assert_eq!(probe_upgrade_reason(candidate, fallback, true), None);
}

#[test]
fn probe_upgrade_reason_accepts_reduced_visible_hp_loss() {
    let fallback = score_with_survival(30, 5, 6);
    let candidate = score_with_survival(30, 10, 1);

    assert_eq!(
        probe_upgrade_reason(candidate, fallback, true),
        Some(
            super::super::rollout_policy::ROLLOUT_ACTION_REASON_CONSERVATIVE_ONE_STEP_SURVIVAL_VALUE
        )
    );
}

fn indexed_choice(
    combat: &CombatState,
    original_action_id: usize,
    input: ClientInput,
) -> IndexedActionChoice {
    IndexedActionChoice {
        original_action_id,
        choice: CombatActionChoice::from_input(combat, input),
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

fn split_debt_combat() -> CombatState {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::AcidSlimeL);
    monster.id = 1;
    monster.current_hp = 30;
    monster.max_hp = 65;
    combat.entities.monsters = vec![monster];
    combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Split,
            instance_id: None,
            amount: -1,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );
    combat
}

fn test_config() -> CombatSearchV2Config {
    CombatSearchV2Config {
        max_nodes: 100,
        max_actions_per_line: 100,
        max_engine_steps_per_action: 10,
        wall_time: None,
        input_label: None,
        potion_policy: CombatSearchV2PotionPolicy::Never,
        max_potions_used: None,
        rollout_policy: CombatSearchV2RolloutPolicy::ConservativeNoPotion,
        rollout_max_evaluations: 10,
        rollout_max_actions: 10,
    }
}

fn score_with_survival(
    final_hp: i32,
    survival_margin: i32,
    visible_hp_loss: i32,
) -> RolloutActionProbeScore {
    RolloutActionProbeScore {
        terminal_rank: terminal_rank(SearchTerminalLabel::Unresolved),
        final_hp,
        survival_margin,
        visible_hp_loss,
        living_enemy_progress: -1,
        phase_adjusted_enemy_progress: -30,
        split_debt_stability: 0,
        mechanics_stability: 0,
        pending_choice_fanout: 0,
        ordered_preference: 0,
        nonterminal_upgrade_eligible: true,
    }
}
