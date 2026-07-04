use super::*;
use crate::ai::combat_search_v2::CombatSearchV2PhaseGuardPolicy;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{ClientInput, EngineState};
use crate::state::selection::{SelectionResolution, SelectionScope};
use crate::test_support::{blank_test_combat, test_monster};

fn grid_select(uuids: impl IntoIterator<Item = u32>) -> ClientInput {
    ClientInput::SubmitSelection(SelectionResolution::card_uuids(SelectionScope::Grid, uuids))
}

#[test]
fn non_player_turn_priority_is_neutral() {
    let combat = blank_test_combat();

    let priority = priority_for_input(
        &EngineState::CombatProcessing,
        &combat,
        &ClientInput::EndTurn,
        CombatSearchV2PhaseGuardPolicy::Default,
    );

    assert_eq!(priority.role, ActionOrderingRole::Neutral);
}

#[test]
fn lethal_play_card_gets_lethal_role() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::Cultist);
    monster.current_hp = 6;
    monster.max_hp = 6;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let priority = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
    );

    assert_eq!(priority.role, ActionOrderingRole::LethalCard);
}

#[test]
fn pending_choice_priority_uses_structured_selection_role() {
    let mut combat = blank_test_combat();
    combat.zones.discard_pile = vec![CombatCard::new(CardId::Carnage, 20)];
    let engine = EngineState::PendingChoice(crate::state::core::PendingChoice::GridSelect {
        source_pile: crate::state::core::PileType::Discard,
        candidate_uuids: vec![20],
        min_cards: 1,
        max_cards: 1,
        can_cancel: false,
        reason: crate::state::core::GridSelectReason::MoveToDrawPile,
    });

    let priority = priority_for_input(
        &engine,
        &combat,
        &grid_select([20]),
        CombatSearchV2PhaseGuardPolicy::Default,
    );

    assert_eq!(
        priority.role,
        ActionOrderingRole::PendingChoiceValueSelection
    );
    assert!(priority.pending_choice_primary > 0);
}

#[test]
fn sleeping_lagavulin_wake_damage_has_phase_penalty() {
    let mut combat = blank_test_combat();
    let mut lagavulin = test_monster(EnemyId::Lagavulin);
    lagavulin.id = 1;
    lagavulin.lagavulin.is_out = false;
    combat.entities.monsters = vec![lagavulin];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let priority = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
    );

    assert!(priority.phase_hint.has_signal());
    assert!(priority.phase_setup < 0);
    assert!(priority.phase_transition_safety < 0);
}
