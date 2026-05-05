use sts_simulator::bot::combat::diagnose_root_search_with_depth;
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::content::powers::PowerId;
use sts_simulator::runtime::combat::{CombatCard, Power};
use sts_simulator::state::core::{ClientInput, EngineState};
use sts_simulator::test_support::{blank_test_combat, planned_monster};

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

#[test]
fn root_search_prefers_demon_form_in_clean_setup_window() {
    let mut combat = blank_test_combat();
    combat.meta.is_boss_fight = true;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    combat.entities.monsters[0].current_hp = 150;
    combat.entities.monsters[0].max_hp = 150;
    combat.zones.hand.push(card(CardId::DemonForm, 1));
    combat.zones.draw_pile.extend([
        card(CardId::Strike, 10),
        card(CardId::Strike, 11),
        card(CardId::Strike, 12),
        card(CardId::Defend, 13),
        card(CardId::Defend, 14),
    ]);
    combat.turn.energy = 3;

    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    assert_eq!(
        diagnostics.chosen_move,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        }
    );
}

#[test]
fn root_search_prefers_evolve_when_next_cycle_contains_status_draws() {
    let mut combat = blank_test_combat();
    combat.meta.is_elite_fight = true;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    combat.entities.monsters[0].current_hp = 120;
    combat.entities.monsters[0].max_hp = 120;
    combat.zones.hand.push(card(CardId::Evolve, 1));
    combat.zones.draw_pile.extend([
        card(CardId::Wound, 10),
        card(CardId::Wound, 15),
        card(CardId::Strike, 11),
        card(CardId::Wound, 12),
        card(CardId::Defend, 13),
        card(CardId::Strike, 14),
    ]);
    combat.turn.energy = 1;

    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    assert_eq!(
        diagnostics.chosen_move,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        }
    );
}

#[test]
fn root_search_prefers_combust_in_multi_target_setup_window() {
    let mut combat = blank_test_combat();
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    combat.entities.monsters[1].id = 2;
    combat.zones.hand.push(card(CardId::Combust, 1));
    combat.zones.draw_pile.extend([
        card(CardId::Strike, 10),
        card(CardId::Strike, 11),
        card(CardId::Defend, 12),
        card(CardId::Defend, 13),
        card(CardId::Strike, 14),
    ]);
    combat.turn.energy = 1;

    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    assert_eq!(
        diagnostics.chosen_move,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        }
    );
}

#[test]
fn root_search_does_not_gain_extra_horizon_from_end_turn() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 18;
    combat.turn.energy = 3;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::AcidSlimeM, 4));
    combat.entities.monsters[0].id = 1;
    combat.entities.monsters[0].current_hp = 27;
    combat.entities.monsters[0].max_hp = 33;

    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::AcidSlimeL, 3));
    combat.entities.monsters[1].id = 2;
    combat.entities.monsters[1].current_hp = 0;
    combat.entities.monsters[1].max_hp = 67;
    combat.entities.monsters[1].is_dying = true;

    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::AcidSlimeM, 4));
    combat.entities.monsters[2].id = 3;
    combat.entities.monsters[2].current_hp = 27;
    combat.entities.monsters[2].max_hp = 33;

    combat.entities.power_db.insert(
        0,
        vec![Power {
            power_type: PowerId::Evolve,
            instance_id: None,
            amount: 1,
            extra_data: 0,
            just_applied: false,
        }],
    );

    combat.zones.hand.extend([
        card(CardId::DarkEmbrace, 1),
        card(CardId::ShrugItOff, 2),
        card(CardId::Defend, 3),
        card(CardId::Strike, 4),
        card(CardId::SwordBoomerang, 5),
    ]);
    combat.zones.discard_pile.extend([
        card(CardId::Dropkick, 10),
        card(CardId::PommelStrike, 11),
        card(CardId::Strike, 12),
        card(CardId::Cleave, 13),
        card(CardId::Defend, 14),
        card(CardId::Dropkick, 15),
        card(CardId::Slimed, 16),
        card(CardId::Slimed, 17),
        card(CardId::Defend, 18),
        card(CardId::Strike, 19),
        card(CardId::Bash, 20),
        card(CardId::Shockwave, 21),
        card(CardId::Defend, 22),
        card(CardId::Strike, 23),
        card(CardId::Strike, 24),
        card(CardId::TrueGrit, 25),
        card(CardId::Slimed, 26),
        card(CardId::Slimed, 27),
    ]);

    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    assert!(
        !matches!(diagnostics.chosen_move, ClientInput::EndTurn),
        "root search should not prefer EndTurn here just because it sees an extra turn cycle"
    );
}

#[test]
fn root_search_decision_audit_includes_exact_turn_shadow() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 3));
    combat.zones.hand.push(card(CardId::Defend, 1));

    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    let exact_turn = diagnostics
        .decision_audit
        .get("exact_turn_shadow")
        .expect("decision audit should expose exact_turn_shadow");
    assert!(
        exact_turn.is_object(),
        "exact_turn shadow should be populated"
    );
    assert!(
        exact_turn
            .get("best_first_input")
            .and_then(|value| value.as_str())
            .is_some(),
        "exact_turn shadow should include its own first-action choice"
    );
    assert!(
        exact_turn
            .get("frontier_chosen_move")
            .and_then(|value| value.as_str())
            .is_some(),
        "exact_turn shadow should expose the baseline frontier choice it is shadowing"
    );
    assert!(
        exact_turn
            .get("takeover_eligible")
            .and_then(|value| value.as_bool())
            .is_some(),
        "exact_turn shadow should expose takeover eligibility"
    );
    assert!(
        exact_turn
            .get("takeover_applied")
            .and_then(|value| value.as_bool())
            .is_some(),
        "exact_turn shadow should expose whether a takeover was applied"
    );
    assert!(
        exact_turn
            .get("takeover_reason")
            .and_then(|value| value.as_str())
            .is_some(),
        "exact_turn shadow should expose why a takeover was or was not applied"
    );
}

#[test]
fn root_search_exact_turn_takeover_metadata_is_consistent_in_obvious_defend_spot() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    combat.entities.player.current_hp = 24;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat.zones.hand.push(card(CardId::Defend, 1));

    let diagnostics =
        diagnose_root_search_with_depth(&EngineState::CombatPlayerTurn, &combat, 2, 0);

    let exact_turn = diagnostics
        .decision_audit
        .get("exact_turn_shadow")
        .expect("decision audit should expose exact_turn_shadow");
    let frontier_choice = exact_turn
        .get("frontier_chosen_move")
        .and_then(|value| value.as_str())
        .expect("frontier chosen move should be present");
    let takeover_applied = exact_turn
        .get("takeover_applied")
        .and_then(|value| value.as_bool())
        .expect("takeover_applied should be present");

    assert_eq!(
        diagnostics.chosen_move,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        }
    );

    if frontier_choice == "EndTurn" {
        assert!(
            takeover_applied,
            "when the baseline frontier still wants EndTurn in this obvious same-turn spot, exact-turn should take over"
        );
        assert_eq!(
            exact_turn
                .get("takeover_move")
                .and_then(|value| value.as_str()),
            Some("PlayCard { card_index: 0, target: None }")
        );
    } else {
        assert!(
            !takeover_applied,
            "if the baseline frontier already agrees with the obvious defend play, takeover should stay inactive"
        );
    }
}
