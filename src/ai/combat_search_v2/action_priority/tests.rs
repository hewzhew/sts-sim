use super::*;
use crate::ai::combat_search_v2::{
    CombatSearchActionOrderingPlugins, CombatSearchV2PhaseGuardPolicy,
    CombatSearchV2SetupBiasPolicy,
};
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatCard;
use crate::state::core::{ClientInput, EngineState};
use crate::state::selection::{SelectionResolution, SelectionScope};
use crate::test_support::{blank_test_combat, planned_monster, test_monster};

fn grid_select(uuids: impl IntoIterator<Item = u32>) -> ClientInput {
    ClientInput::SubmitSelection(SelectionResolution::card_uuids(SelectionScope::Grid, uuids))
}

fn add_explosive(combat: &mut crate::runtime::combat::CombatState, entity_id: usize, amount: i32) {
    combat
        .entities
        .power_db
        .entry(entity_id)
        .or_default()
        .push(crate::runtime::combat::Power {
            power_type: crate::content::powers::PowerId::Explosive,
            instance_id: None,
            amount,
            extra_data: 0,
            payload: crate::runtime::combat::PowerPayload::None,
            just_applied: false,
        });
}

fn retaliation_ordering_combat(
    energy: u8,
    retaliation: i32,
) -> crate::runtime::combat::CombatState {
    use crate::runtime::monster_move::{BuffSpec, MonsterMoveSpec};

    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 60;
    combat.turn.energy = energy;
    let mut flame_barrier = CombatCard::new(CardId::FlameBarrier, 11);
    flame_barrier.upgrades = 1;
    combat.zones.hand = vec![CombatCard::new(CardId::HeavyBlade, 10), flame_barrier];
    let mut target = test_monster(EnemyId::Cultist);
    target.id = 1;
    target.current_hp = 100;
    target.max_hp = 100;
    target.set_planned_steps(
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: crate::content::powers::PowerId::Thorns,
            amount: 2,
        })
        .to_steps(),
    );
    combat.entities.monsters = vec![target];
    if retaliation > 0 {
        combat.entities.power_db.insert(
            1,
            vec![crate::runtime::combat::Power {
                power_type: crate::content::powers::PowerId::Thorns,
                instance_id: None,
                amount: retaliation,
                extra_data: 0,
                payload: crate::runtime::combat::PowerPayload::None,
                just_applied: false,
            }],
        );
    }
    combat
}

fn flame_priority(combat: &crate::runtime::combat::CombatState) -> ActionOrderingPriority {
    priority_for_input(
        &EngineState::CombatPlayerTurn,
        combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    )
}

fn add_visible_attacker(combat: &mut crate::runtime::combat::CombatState) {
    let mut attacker = planned_monster(EnemyId::TimeEater, 2);
    attacker.id = 2;
    combat.entities.monsters.push(attacker);
}

fn priority_for_input(
    engine: &EngineState,
    combat: &crate::runtime::combat::CombatState,
    input: &ClientInput,
    phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
    setup_bias_policy: CombatSearchV2SetupBiasPolicy,
) -> ActionOrderingPriority {
    priority_for_input_with_plugins(
        engine,
        combat,
        input,
        CombatSearchActionOrderingPlugins {
            root_action_prior: None,
            phase_guard: phase_guard_policy.into(),
            action_prior: setup_bias_policy.into(),
        },
    )
}

fn awakened_transition_ordering_combat(
    hp: i32,
    strength: i32,
    form_one: bool,
) -> crate::runtime::combat::CombatState {
    let mut combat = blank_test_combat();
    let mut awakened = test_monster(EnemyId::AwakenedOne);
    awakened.id = 7;
    awakened.current_hp = hp;
    awakened.max_hp = 300;
    awakened.awakened_one.form1 = form_one;
    combat.entities.monsters = vec![awakened];
    combat.entities.power_db.insert(
        7,
        vec![crate::runtime::combat::Power {
            power_type: crate::content::powers::PowerId::Strength,
            instance_id: None,
            amount: strength,
            extra_data: 0,
            payload: crate::runtime::combat::PowerPayload::None,
            just_applied: false,
        }],
    );
    combat
}

fn transition_priority(
    combat: &crate::runtime::combat::CombatState,
    card_index: usize,
) -> ActionOrderingPriority {
    priority_for_input(
        &EngineState::CombatPlayerTurn,
        combat,
        &ClientInput::PlayCard {
            card_index,
            target: Some(7),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    )
}

#[test]
fn awakened_transition_setup_orders_before_direct_form_one_lethal() {
    let mut combat = awakened_transition_ordering_combat(6, 4, true);
    combat.turn.energy = 1;
    combat.zones.hand = vec![
        CombatCard::new(CardId::DarkShackles, 10),
        CombatCard::new(CardId::Strike, 11),
    ];

    let dark = transition_priority(&combat, 0);
    let lethal = transition_priority(&combat, 1);

    assert!(
        dark.role_rank > lethal.role_rank,
        "dark={dark:?} lethal={lethal:?}"
    );
    assert!(dark
        .phase_hint
        .awakened_one_strength_transition_setup
        .is_some());
}

#[test]
fn awakened_transition_setup_bonus_requires_all_hard_gates() {
    for (label, mut combat) in [
        ("form-two", awakened_transition_ordering_combat(6, 4, false)),
        (
            "zero-strength",
            awakened_transition_ordering_combat(6, 0, true),
        ),
        (
            "insufficient-damage",
            awakened_transition_ordering_combat(40, 4, true),
        ),
    ] {
        combat.turn.energy = 1;
        combat.zones.hand = vec![
            CombatCard::new(CardId::DarkShackles, 10),
            CombatCard::new(CardId::Strike, 11),
        ];
        let priority = transition_priority(&combat, 0);
        assert!(
            priority
                .phase_hint
                .awakened_one_strength_transition_setup
                .is_none(),
            "unexpected setup hint for {label}"
        );
        assert_eq!(priority.phase_hint.role_rank_adjustment, 0, "{label}");
    }
}

#[test]
fn non_player_turn_priority_is_neutral() {
    let combat = blank_test_combat();

    let priority = priority_for_input(
        &EngineState::CombatProcessing,
        &combat,
        &ClientInput::EndTurn,
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
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
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(priority.role, ActionOrderingRole::LethalCard);
}

#[test]
fn timed_threat_damage_progress_orders_before_neutral_target() {
    let mut combat = blank_test_combat();
    let mut neutral = test_monster(EnemyId::Repulsor);
    neutral.id = 1;
    neutral.current_hp = 40;
    neutral.max_hp = 40;
    let mut timed = test_monster(EnemyId::Exploder);
    timed.id = 2;
    timed.current_hp = 40;
    timed.max_hp = 40;
    combat.entities.monsters = vec![neutral, timed];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    add_explosive(&mut combat, 2, 3);

    let neutral = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let timed = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(neutral.targets_timed_threat, 0);
    assert_eq!(timed.targets_timed_threat, 1);
    assert_eq!(timed.timed_threat_urgency, -3);
    assert_eq!(timed.timed_threat_raw_damage, 30);
    assert!(timed > neutral);
}

#[test]
fn timed_threat_damage_progress_prefers_shorter_countdown() {
    let mut combat = blank_test_combat();
    let mut slower = test_monster(EnemyId::Exploder);
    slower.id = 1;
    slower.current_hp = 40;
    slower.max_hp = 40;
    let mut sooner = test_monster(EnemyId::Exploder);
    sooner.id = 2;
    sooner.current_hp = 40;
    sooner.max_hp = 40;
    combat.entities.monsters = vec![slower, sooner];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    add_explosive(&mut combat, 1, 3);
    add_explosive(&mut combat, 2, 1);

    let slower = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let sooner = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(slower.timed_threat_urgency, -3);
    assert_eq!(sooner.timed_threat_urgency, -1);
    assert!(sooner > slower);
}

#[test]
fn timed_threat_damage_progress_is_neutral_without_power() {
    let mut combat = blank_test_combat();
    let mut first = test_monster(EnemyId::Repulsor);
    first.id = 1;
    first.current_hp = 40;
    first.max_hp = 40;
    let mut second = test_monster(EnemyId::Exploder);
    second.id = 2;
    second.current_hp = 40;
    second.max_hp = 40;
    combat.entities.monsters = vec![first, second];
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let first = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let second = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(2),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(first.targets_timed_threat, 0);
    assert_eq!(second.targets_timed_threat, 0);
    assert_eq!(first.timed_threat_urgency, 0);
    assert_eq!(second.timed_threat_urgency, 0);
    assert_eq!(first.timed_threat_raw_damage, 0);
    assert_eq!(second.timed_threat_raw_damage, 0);
    assert_eq!(first, second);
}

#[test]
fn damage_progress_prefers_fewer_attack_retaliation_triggers() {
    let mut combat = blank_test_combat();
    combat.entities.player.block = 5;
    let mut spiker = test_monster(EnemyId::Spiker);
    spiker.id = 1;
    spiker.current_hp = 40;
    spiker.max_hp = 40;
    combat.entities.monsters = vec![spiker];
    combat.zones.hand = vec![
        CombatCard::new(CardId::TwinStrike, 10),
        CombatCard::new(CardId::Strike, 11),
    ];
    combat.entities.power_db.insert(
        1,
        vec![crate::runtime::combat::Power {
            power_type: crate::content::powers::PowerId::Thorns,
            instance_id: None,
            amount: 3,
            extra_data: 0,
            payload: crate::runtime::combat::PowerPayload::None,
            just_applied: false,
        }],
    );

    let twin_strike = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let strike = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(twin_strike.role, ActionOrderingRole::DamageProgress);
    assert_eq!(strike.role, ActionOrderingRole::DamageProgress);
    assert_eq!(
        twin_strike
            .effects
            .reactive
            .attack_retaliation_raw_player_damage_hint,
        6
    );
    assert_eq!(
        twin_strike
            .effects
            .reactive
            .attack_retaliation_player_block_loss_hint,
        5
    );
    assert_eq!(
        twin_strike
            .effects
            .reactive
            .attack_retaliation_player_hp_loss_hint,
        1
    );
    assert_eq!(
        strike
            .effects
            .reactive
            .attack_retaliation_raw_player_damage_hint,
        3
    );
    assert_eq!(
        strike
            .effects
            .reactive
            .attack_retaliation_player_block_loss_hint,
        3
    );
    assert_eq!(
        strike
            .effects
            .reactive
            .attack_retaliation_player_hp_loss_hint,
        0
    );
    assert!(strike > twin_strike);
}

#[test]
fn damage_progress_has_no_attack_retaliation_attribution_without_retaliation_power() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::Repulsor);
    monster.id = 1;
    monster.current_hp = 40;
    monster.max_hp = 40;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::TwinStrike, 10),
        CombatCard::new(CardId::Strike, 11),
    ];

    let twin_strike = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let strike = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(
        twin_strike
            .effects
            .reactive
            .attack_retaliation_trigger_count_hint,
        0
    );
    assert_eq!(
        twin_strike
            .effects
            .reactive
            .attack_retaliation_player_hp_loss_hint,
        0
    );
    assert_eq!(twin_strike.role, ActionOrderingRole::DamageProgress);
    assert_eq!(strike.role, ActionOrderingRole::DamageProgress);
}

#[test]
fn block_skill_precedes_payable_retaliation_attack_when_order_saves_hp() {
    let combat = retaliation_ordering_combat(4, 13);

    let flame = flame_priority(&combat);
    let heavy = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(
        flame.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
    assert!(flame > heavy);
}

#[test]
fn retaliation_protection_requires_both_cards_to_be_payable() {
    let combat = retaliation_ordering_combat(3, 13);

    let flame = flame_priority(&combat);

    assert_ne!(
        flame.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
}

#[test]
fn block_skill_is_not_retaliation_setup_without_retaliation() {
    let combat = retaliation_ordering_combat(4, 0);

    let flame = flame_priority(&combat);

    assert_ne!(
        flame.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
}

#[test]
fn retaliation_setup_is_not_promoted_when_visible_damage_makes_orders_equal() {
    let mut combat = retaliation_ordering_combat(4, 13);
    add_visible_attacker(&mut combat);

    let flame = flame_priority(&combat);

    assert_ne!(
        flame.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
}

fn apply_combat_input(
    position: &crate::sim::combat::CombatPosition,
    input: ClientInput,
) -> crate::sim::combat::CombatPosition {
    use crate::sim::combat::{CombatStepLimits, CombatStepper, EngineCombatStepper};

    let result = EngineCombatStepper.apply_to_stable(
        position,
        input,
        CombatStepLimits {
            max_engine_steps: 250,
            deadline: None,
        },
    );
    assert!(!result.truncated && !result.timed_out);
    result.position
}

#[test]
fn retaliation_protection_exact_order_preserves_thirteen_hp() {
    let combat = retaliation_ordering_combat(4, 13);
    let start =
        crate::sim::combat::CombatPosition::new(EngineState::CombatPlayerTurn, combat.clone());

    let after_attack = apply_combat_input(
        &start,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    );
    let attack_then_block = apply_combat_input(
        &after_attack,
        ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
    );
    let after_block = apply_combat_input(
        &start,
        ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
    );
    let block_then_attack = apply_combat_input(
        &after_block,
        ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
    );

    assert_eq!(attack_then_block.combat.entities.player.current_hp, 47);
    assert_eq!(attack_then_block.combat.entities.player.block, 16);
    assert_eq!(block_then_attack.combat.entities.player.current_hp, 60);
    assert_eq!(block_then_attack.combat.entities.player.block, 3);
}

#[test]
fn retaliation_protection_uses_sequential_multi_hit_projection() {
    let mut combat = retaliation_ordering_combat(2, 3);
    combat.zones.hand = vec![
        CombatCard::new(CardId::Defend, 20),
        CombatCard::new(CardId::TwinStrike, 21),
    ];

    let defend = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(
        defend.role,
        ActionOrderingRole::CurrentTurnRetaliationProtection
    );
    assert_eq!(defend.block, 5);
    assert_eq!(defend.phase_setup, 5);
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
        CombatSearchV2SetupBiasPolicy::Default,
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
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert!(priority.phase_hint.has_signal());
    assert!(priority.phase_setup < 0);
    assert!(priority.phase_transition_safety < 0);
}

#[test]
fn guardian_mode_shift_attack_interrupts_visible_damage_before_end_turn() {
    use crate::runtime::monster_move::{AttackSpec, DamageKind, MonsterMoveSpec};

    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 46;
    let mut guardian = test_monster(EnemyId::TheGuardian);
    guardian.id = 1;
    guardian.guardian.is_open = true;
    guardian.set_planned_move_id(2);
    let attack = MonsterMoveSpec::Attack(AttackSpec {
        base_damage: 32,
        hits: 1,
        damage_kind: DamageKind::Normal,
    });
    guardian.set_planned_steps(attack.to_steps());
    guardian.set_planned_visible_spec(Some(attack));
    combat.entities.monsters = vec![guardian];
    combat.entities.power_db.insert(
        1,
        vec![crate::runtime::combat::Power {
            power_type: crate::content::powers::PowerId::ModeShift,
            instance_id: None,
            amount: 5,
            extra_data: 0,
            payload: crate::runtime::combat::PowerPayload::None,
            just_applied: false,
        }],
    );
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];

    let trigger = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let end_turn = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::EndTurn,
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(trigger.role, ActionOrderingRole::PreventHpLoss);
    assert!(
        trigger > end_turn,
        "trigger={trigger:?} end_turn={end_turn:?}"
    );
}

#[test]
fn key_card_setup_bias_promotes_strength_scaling_power() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    monster.current_hp = 50;
    monster.max_hp = 50;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::DemonForm, 11),
    ];

    let default_setup = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    assert_eq!(default_setup.role, ActionOrderingRole::DeferredSetup);

    let biased_setup = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::KeyCardOnline,
    );
    let strike = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::KeyCardOnline,
    );

    assert_eq!(biased_setup.role, ActionOrderingRole::KeySetupCard);
    assert!(biased_setup > strike);
}

#[test]
fn premature_fiend_fire_waits_behind_access_when_it_would_burn_key_hand_resources() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    monster.current_hp = 80;
    monster.max_hp = 80;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::FiendFire, 10),
        CombatCard::new(CardId::Offering, 11),
        CombatCard::new(CardId::SpotWeakness, 12),
        CombatCard::new(CardId::Strike, 13),
    ];

    let fiend_fire = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let offering = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert!(fiend_fire.resource_timing < 0);
    assert!(fiend_fire.role_rank < constants::ROLE_DAMAGE_PROGRESS);
    assert!(
        offering > fiend_fire,
        "premature Fiend Fire should not outrank access while burning Offering/Spot Weakness"
    );
}

#[test]
fn lethal_fiend_fire_keeps_finisher_priority_even_when_it_consumes_hand_resources() {
    let mut combat = blank_test_combat();
    let mut monster = test_monster(EnemyId::JawWorm);
    monster.id = 1;
    monster.current_hp = 20;
    monster.max_hp = 20;
    combat.entities.monsters = vec![monster];
    combat.zones.hand = vec![
        CombatCard::new(CardId::FiendFire, 10),
        CombatCard::new(CardId::Offering, 11),
        CombatCard::new(CardId::SpotWeakness, 12),
        CombatCard::new(CardId::Strike, 13),
    ];

    let fiend_fire = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 0,
            target: Some(1),
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );
    let offering = priority_for_input(
        &EngineState::CombatPlayerTurn,
        &combat,
        &ClientInput::PlayCard {
            card_index: 1,
            target: None,
        },
        CombatSearchV2PhaseGuardPolicy::Default,
        CombatSearchV2SetupBiasPolicy::Default,
    );

    assert_eq!(fiend_fire.role, ActionOrderingRole::LethalCard);
    assert!(fiend_fire > offering);
}
