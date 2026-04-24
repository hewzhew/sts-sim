use sts_simulator::bot::combat::exact_turn_solver::{
    solve_exact_turn_with_config, ExactTurnConfig,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::content::potions::{Potion, PotionId};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::core::{ClientInput, EngineState};
use sts_simulator::test_support::{blank_test_combat, planned_monster};

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

fn exact_config() -> ExactTurnConfig {
    ExactTurnConfig {
        max_nodes: 20_000,
        max_engine_steps: 200,
        ..ExactTurnConfig::default()
    }
}

#[test]
fn exact_turn_solver_does_not_end_turn_in_obvious_defend_spot() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    combat.entities.player.current_hp = 24;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat.zones.hand.push(card(CardId::Defend, 1));

    let solution =
        solve_exact_turn_with_config(&EngineState::CombatPlayerTurn, &combat, exact_config());

    assert_eq!(
        solution.best_first_input,
        Some(ClientInput::PlayCard {
            card_index: 0,
            target: None,
        })
    );
    assert!(
        !matches!(solution.best_line.first(), Some(ClientInput::EndTurn)),
        "exact same-turn solver should not immediately end turn here"
    );
}

#[test]
fn exact_turn_solver_does_not_choose_flex_potion_into_empty_end_turn() {
    let mut combat = blank_test_combat();
    combat.meta.is_elite_fight = true;
    combat.turn.energy = 0;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat.zones.hand.push(card(CardId::Defend, 1));
    combat.entities.potions = vec![Some(Potion::new(PotionId::SteroidPotion, 1)), None, None];

    let solution =
        solve_exact_turn_with_config(&EngineState::CombatPlayerTurn, &combat, exact_config());

    assert_eq!(solution.best_line, vec![ClientInput::EndTurn]);
    assert_eq!(
        solution.best_first_input,
        Some(ClientInput::EndTurn),
        "even without hard pruning, a temporary-strength potion line that cashes out into nothing should not outrank direct EndTurn"
    );
}

#[test]
fn exact_turn_solver_finds_flex_limit_break_attack_line() {
    let mut combat = blank_test_combat();
    combat.meta.is_elite_fight = true;
    combat.turn.energy = 3;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat.entities.monsters[0].current_hp = 30;
    combat.entities.monsters[0].max_hp = 30;
    combat
        .zones
        .hand
        .extend([card(CardId::LimitBreak, 1), card(CardId::TwinStrike, 2)]);
    combat.entities.potions = vec![Some(Potion::new(PotionId::SteroidPotion, 1)), None, None];

    let solution =
        solve_exact_turn_with_config(&EngineState::CombatPlayerTurn, &combat, exact_config());

    assert_eq!(
        solution.best_first_input,
        Some(ClientInput::UsePotion {
            potion_index: 0,
            target: None,
        })
    );
    assert!(
        matches!(
            solution.best_line.as_slice(),
            [
                ClientInput::UsePotion {
                    potion_index: 0,
                    target: None,
                },
                ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                },
                ClientInput::PlayCard {
                    target: Some(1),
                    ..
                },
                ..
            ]
        ),
        "exact same-turn solver should surface the temporary-strength combo line"
    );
}

#[test]
fn exact_turn_solver_reports_truncation_when_node_budget_is_tiny() {
    let mut combat = blank_test_combat();
    combat.meta.is_elite_fight = true;
    combat.turn.energy = 3;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat
        .zones
        .hand
        .extend([card(CardId::LimitBreak, 1), card(CardId::TwinStrike, 2)]);
    combat.entities.potions = vec![Some(Potion::new(PotionId::SteroidPotion, 1)), None, None];

    let solution = solve_exact_turn_with_config(
        &EngineState::CombatPlayerTurn,
        &combat,
        ExactTurnConfig {
            max_nodes: 1,
            max_engine_steps: 200,
            ..ExactTurnConfig::default()
        },
    );

    assert!(
        solution.truncated,
        "tiny node budgets should be surfaced explicitly"
    );
}

#[test]
fn exact_turn_solver_does_not_treat_same_turn_pending_choice_as_leaf() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));

    let solution = solve_exact_turn_with_config(
        &EngineState::PendingChoice(sts_simulator::state::core::PendingChoice::DiscoverySelect(
            vec![CardId::Strike, CardId::Defend, CardId::Bash],
        )),
        &combat,
        exact_config(),
    );

    assert!(
        solution
            .nondominated_end_states
            .iter()
            .all(|state| !state.line.is_empty()),
        "same-turn pending choices should not be admitted as empty leaf candidates"
    );
    assert!(
        matches!(
            solution.best_first_input,
            Some(ClientInput::SubmitDiscoverChoice(0 | 1 | 2))
        ),
        "solver should keep expanding through same-turn choice submissions"
    );
}

#[test]
fn exact_turn_solver_respects_root_inputs_without_synthetic_end_turn_leaf() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    combat.entities.player.current_hp = 24;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat.zones.hand.push(card(CardId::Defend, 1));

    let solution = solve_exact_turn_with_config(
        &EngineState::CombatPlayerTurn,
        &combat,
        ExactTurnConfig {
            root_inputs: Some(vec![ClientInput::PlayCard {
                card_index: 0,
                target: None,
            }]),
            ..exact_config()
        },
    );

    assert_eq!(
        solution.best_first_input,
        Some(ClientInput::PlayCard {
            card_index: 0,
            target: None,
        })
    );
    assert!(
        solution
            .nondominated_end_states
            .iter()
            .all(|state| !state.line.is_empty()),
        "when root_inputs removes EndTurn, solver should not synthesize an empty or free turn-close candidate"
    );
}
