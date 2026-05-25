use super::super::phase_profile::combat_search_phase_profile;
use super::*;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{ClientInput, EngineState};
use crate::test_support::{blank_test_combat, test_monster};

#[test]
fn lagavulin_sleep_phase_penalizes_wake_damage_and_rewards_power_setup() {
    let mut combat = blank_test_combat();
    let mut lagavulin = test_monster(EnemyId::Lagavulin);
    lagavulin.id = 1;
    lagavulin.lagavulin.is_out = false;
    combat.entities.monsters = vec![lagavulin];
    combat.zones.hand = vec![
        CombatCard::new(crate::content::cards::CardId::Strike, 10),
        CombatCard::new(crate::content::cards::CardId::Inflame, 11),
    ];
    let profile = combat_search_phase_profile(&EngineState::CombatPlayerTurn, &combat);

    let wake_hint = phase_action_ordering_hint(
        profile,
        PhaseActionOrderingFacts {
            card_type: CardType::Attack,
            block: 0,
            mitigation: 0,
            phase_transition:
                super::super::enemy_phase_transition::enemy_phase_transition_hint_for_input(
                    &combat,
                    &ClientInput::PlayCard {
                        card_index: 0,
                        target: Some(1),
                    },
                ),
        },
    );
    let setup_hint = phase_action_ordering_hint(
        profile,
        PhaseActionOrderingFacts {
            card_type: CardType::Power,
            block: 0,
            mitigation: 0,
            phase_transition:
                super::super::enemy_phase_transition::enemy_phase_transition_hint_for_input(
                    &combat,
                    &ClientInput::PlayCard {
                        card_index: 1,
                        target: None,
                    },
                ),
        },
    );

    assert!(wake_hint.role_rank_adjustment < 0);
    assert!(wake_hint.phase_transition_safety < 0);
    assert!(setup_hint.role_rank_adjustment > 0);
    assert!(setup_hint.phase_setup > wake_hint.phase_setup);
}

#[test]
fn guardian_defensive_phase_records_survival_tiebreak() {
    let mut combat = blank_test_combat();
    let mut guardian = test_monster(EnemyId::TheGuardian);
    guardian.guardian.is_open = false;
    combat.entities.monsters = vec![guardian];
    let profile = combat_search_phase_profile(&EngineState::CombatPlayerTurn, &combat);

    let hint = phase_action_ordering_hint(
        profile,
        PhaseActionOrderingFacts {
            card_type: CardType::Skill,
            block: 8,
            mitigation: 3,
            phase_transition: EnemyPhaseTransitionHint::default(),
        },
    );

    assert_eq!(hint.phase_survival, 11);
    assert!(hint.has_signal());
}
