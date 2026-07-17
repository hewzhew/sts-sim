use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::action::CardDestination;
use crate::runtime::combat::{CombatCard, Power, PowerPayload};
use crate::sim::combat::{CombatPosition, CombatStepLimits};
use crate::state::core::{
    ChooseOneCardChoice, ClientInput, DiscoveryChoiceState, EngineState, GridSelectReason,
    HandSelectReason, PendingChoice, PileType,
};

use super::super::{
    combat_turn_option_observable_effect_v1, compare_combat_turn_option_observable_effects_v1,
    CombatTurnOptionObservableEffectComparisonGapV1, CombatTurnOptionObservableEffectEvidenceGapV1,
    CombatTurnOptionObservableEffectEvidenceV1, CombatTurnOptionObservableEffectRelationV1,
    CombatTurnOptionWideningChoiceV1, CombatTurnOptionWideningContextV1,
    CombatTurnOptionWideningScheduleV1,
};
use super::*;

#[test]
fn hidden_draw_orders_share_one_information_set_without_frozen_eye() {
    let first = position_with_draw_order([CardId::Bash, CardId::Defend]);
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 1);

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden draw-order variants should group");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().scenario_count, 2);
}

#[test]
fn frozen_eye_draw_orders_form_distinct_information_sets() {
    let mut first = position_with_draw_order([CardId::Bash, CardId::Defend]);
    first
        .combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::FrozenEye));
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 1);

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("Frozen Eye variants should remain valid");

    assert_eq!(groups.len(), 2);
}

#[test]
fn public_history_separates_otherwise_identical_observations() {
    let position = position_with_draw_order([CardId::Bash, CardId::Defend]);

    let groups = group_combat_scenarios_v1(vec![
        CombatScenarioParticleV1::from_public_history("first", "history-a", position.clone()),
        CombatScenarioParticleV1::from_public_history("second", "history-b", position),
    ])
    .expect("public history should be part of the information set");

    assert_eq!(groups.len(), 2);
}

#[test]
fn one_public_target_action_binds_to_each_worlds_exact_entity_id() {
    let first = position_with_monster_id(700_001);
    let second = position_with_monster_id(990_001);
    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("entity identity must stay behind the public action boundary");

    assert_eq!(groups.len(), 1);
    let action = groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| {
            matches!(
                action,
                CombatPublicActionV1::PlayCard {
                    target: Some(CombatPublicTargetV1 {
                        monster_slot: 0,
                        ..
                    }),
                    ..
                }
            )
        })
        .expect("targeted Strike action")
        .clone();
    let binding = groups[0]
        .bind_action(&action)
        .expect("public action binding");

    assert_eq!(binding.scenario_count(), 2);
    assert_eq!(
        binding
            .exact_inputs()
            .iter()
            .map(|(_, input)| match input {
                ClientInput::PlayCard {
                    target: Some(target),
                    ..
                } => *target,
                other => panic!("unexpected exact input: {other:?}"),
            })
            .collect::<Vec<_>>(),
        vec![700_001, 990_001]
    );

    let public_json =
        serde_json::to_string(groups[0].view()).expect("public group view should serialize");
    assert!(!public_json.contains("first"));
    assert!(!public_json.contains("second"));
    assert!(!public_json.contains("700001"));
    assert!(!public_json.contains("990001"));
}

#[test]
fn public_discard_contents_form_distinct_information_sets() {
    let mut first = position_with_monster_id(7);
    first.combat.zones.discard_pile = vec![CombatCard::new(CardId::Bash, 31)];
    let mut second = first.clone();
    second.combat.zones.discard_pile = vec![CombatCard::new(CardId::Defend, 32)];

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("public discard contents should remain valid");

    assert_eq!(groups.len(), 2);
}

#[test]
fn public_relic_counters_form_distinct_information_sets() {
    let mut first = position_with_monster_id(7);
    let mut pen_nib = RelicState::new(RelicId::PenNib);
    pen_nib.counter = 8;
    first.combat.entities.player.add_relic(pen_nib);
    let mut second = first.clone();
    second.combat.entities.player.relics[0].counter = 9;

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("public relic counters should remain valid");

    assert_eq!(groups.len(), 2);
}

#[test]
fn public_power_amounts_form_distinct_information_sets() {
    let mut first = position_with_monster_id(7);
    first.combat.entities.power_db.insert(
        first.combat.entities.player.id,
        vec![power(PowerId::Strength, 2)],
    );
    let mut second = first.clone();
    second
        .combat
        .entities
        .power_db
        .get_mut(&second.combat.entities.player.id)
        .expect("player power")[0]
        .amount = 3;

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("public power amounts should remain valid");

    assert_eq!(groups.len(), 2);
}

#[test]
fn monster_power_state_uses_public_slot_not_exact_entity_id() {
    let mut first = position_with_monster_id(700_001);
    first
        .combat
        .entities
        .power_db
        .insert(700_001, vec![power(PowerId::Weak, 2)]);
    let mut second = position_with_monster_id(990_001);
    second
        .combat
        .entities
        .power_db
        .insert(990_001, vec![power(PowerId::Weak, 2)]);

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("exact monster identity must stay private");

    assert_eq!(groups.len(), 1);
    let json = serde_json::to_string(groups[0].view()).expect("public group view serialization");
    assert!(!json.contains("700001"));
    assert!(!json.contains("990001"));
}

#[test]
fn hidden_rng_state_does_not_split_an_information_set() {
    let first = position_with_monster_id(7);
    let mut second = first.clone();
    second.combat.rng.pool.shuffle_rng.counter += 1;
    second.combat.rng.pool.shuffle_rng.seed0 ^= 0x55aa;

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden RNG variants should group");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().scenario_count, 2);
}

#[test]
fn exact_card_uuid_does_not_split_an_information_set() {
    let first = position_with_monster_id(7);
    let mut second = first.clone();
    second.combat.zones.hand[0].uuid = 999_999;

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("exact card identity should stay private");

    assert_eq!(groups.len(), 1);
}

#[test]
fn non_quiescent_player_turn_is_rejected() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.limbo = vec![CombatCard::new(CardId::Bash, 40)];

    let error = match group_combat_scenarios_v1(vec![particle("pending", position)]) {
        Ok(_) => panic!("half-resolved player turn must fail closed"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        CombatScenarioPolicyErrorV1::NonQuiescentBoundary {
            scenario_id: "pending".to_string(),
            pending_work: vec!["limbo".to_string()],
        }
    );
}

#[test]
fn one_public_action_steps_all_worlds_and_regroups_hidden_rng_variants() {
    let first = position_with_monster_id(7);
    let mut second = first.clone();
    second.combat.rng.pool.shuffle_rng.counter += 1;
    second.combat.rng.pool.shuffle_rng.seed0 ^= 0x55aa;
    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden RNG variants should group");
    let strike = groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| matches!(action, CombatPublicActionV1::PlayCard { .. }))
        .expect("Strike action")
        .clone();

    let stepped = step_combat_scenario_group_v1(
        &groups[0],
        &strike,
        CombatStepLimits {
            max_engine_steps: 50,
            deadline: None,
        },
    )
    .expect("one public action should step both exact worlds");

    assert_eq!(stepped.view.scenario_count, 2);
    assert_eq!(stepped.view.continuing_scenario_count, 2);
    assert_eq!(stepped.view.next_information_set_count, 1);
    assert_eq!(stepped.next_groups[0].view().scenario_count, 2);
    assert_ne!(
        stepped.next_groups[0].view().key.public_history_id,
        COMBAT_POLICY_ROOT_HISTORY_ID
    );
}

#[test]
fn scenario_group_engine_step_limit_is_shared_across_exact_worlds() {
    let first = position_with_monster_id(7);
    let single_groups = group_combat_scenarios_v1(vec![particle("single", first.clone())])
        .expect("single exact world");
    let strike = single_groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| matches!(action, CombatPublicActionV1::PlayCard { .. }))
        .expect("Strike action")
        .clone();
    let single = step_combat_scenario_group_v1(
        &single_groups[0],
        &strike,
        CombatStepLimits {
            max_engine_steps: 50,
            deadline: None,
        },
    )
    .expect("single exact world reaches a stable boundary");
    let paired_limit = single.view.engine_steps.saturating_mul(2).saturating_sub(1);

    let mut second = first.clone();
    second.combat.rng.pool.shuffle_rng.counter += 1;
    let paired_groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden exact worlds share one information set");
    let error = match step_combat_scenario_group_v1(
        &paired_groups[0],
        &strike,
        CombatStepLimits {
            max_engine_steps: paired_limit,
            deadline: None,
        },
    ) {
        Ok(_) => panic!("the information-set action must share one total engine-step limit"),
        Err(error) => error,
    };

    assert_eq!(
        error.exact_error(),
        &CombatScenarioPolicyErrorV1::StepTruncated {
            scenario_id: "second".to_string(),
            engine_steps: paired_limit,
            timed_out: false,
        }
    );
}

#[test]
fn newly_observed_draws_split_successor_information_sets() {
    let first = position_with_battle_trance_draw_order([
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
    ]);
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 1);
    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden draw-order variants should group before drawing");
    assert_eq!(groups.len(), 1);
    let battle_trance = groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| {
            matches!(
                action,
                CombatPublicActionV1::PlayCard {
                    card_id,
                    target: None,
                    ..
                } if card_id == "Battle Trance"
            )
        })
        .expect("Battle Trance action")
        .clone();

    let stepped = step_combat_scenario_group_v1(
        &groups[0],
        &battle_trance,
        CombatStepLimits {
            max_engine_steps: 50,
            deadline: None,
        },
    )
    .expect("draw action should reach a new public boundary");

    assert_eq!(stepped.view.scenario_count, 2);
    assert_eq!(stepped.view.next_information_set_count, 2);
    assert_eq!(
        stepped
            .next_groups
            .iter()
            .map(|group| group.view().scenario_count)
            .sum::<usize>(),
        2
    );
}

#[test]
fn turn_option_prefix_widening_resumes_without_replaying_opened_actions() {
    let mut position = position_with_monster_id(7);
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    position.combat.entities.monsters[0].set_planned_move_id(1);
    position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
    position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack));
    let groups = group_combat_scenarios_v1(vec![particle("single", position)])
        .expect("single public scenario");
    let mut session = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut budget = turn_option_budget(2, 100, None);

    let first = session
        .widen(&mut budget, 1)
        .expect("open the first action");
    assert_eq!(first.previous_opened_action_count, 0);
    assert_eq!(first.newly_opened.len(), 1);
    assert_eq!(first.total_opened_action_count, 1);
    assert_eq!(first.remaining_action_count, 1);
    assert_eq!(
        first.status,
        CombatTurnOptionPrefixExpansionStatusV1::PartiallyExpanded {
            cause: CombatTurnOptionPrefixExpansionStopV1::RequestedWidth,
        }
    );
    let first_action = first.newly_opened[0].action.clone();

    let second = session
        .widen(&mut budget, 1)
        .expect("resume with the second action");
    assert_eq!(second.previous_opened_action_count, 1);
    assert_eq!(second.newly_opened.len(), 1);
    assert_ne!(second.newly_opened[0].action, first_action);
    assert_eq!(second.total_opened_action_count, 2);
    assert_eq!(second.remaining_action_count, 0);
    assert_eq!(
        second.status,
        CombatTurnOptionPrefixExpansionStatusV1::Exhausted
    );
    assert_eq!(
        second.cumulative_engine_steps,
        first
            .new_engine_steps
            .saturating_add(second.new_engine_steps)
    );

    let completed_steps = session.cumulative_engine_steps();
    let completed = session
        .widen(&mut budget, 1)
        .expect("an exhausted prefix remains resumable");
    assert!(completed.newly_opened.is_empty());
    assert_eq!(completed.previous_opened_action_count, 2);
    assert_eq!(completed.new_engine_steps, 0);
    assert_eq!(completed.cumulative_engine_steps, completed_steps);
    assert_eq!(session.opened_action_count(), 2);
}

#[test]
fn turn_option_prefix_report_does_not_expose_exact_world_identity() {
    let first = position_with_battle_trance_draw_order([
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
    ]);
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 3);
    second.combat.rng.pool.shuffle_rng.seed0 ^= 0x55aa;
    let groups = group_combat_scenarios_v1(vec![
        particle("private-first-scenario", first),
        particle("private-second-scenario", second),
    ])
    .expect("hidden worlds share one public information set");
    let mut session = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut budget = turn_option_budget(1, 100, None);

    let report = session
        .widen(&mut budget, 1)
        .expect("open one public prefix action");
    let json = serde_json::to_string(&report).expect("serialize public prefix report");

    assert!(!json.contains("private-first-scenario"));
    assert!(!json.contains("private-second-scenario"));
    assert!(!json.contains("uuid"));
    assert!(!json.contains("rng"));
    assert!(!json.contains("seed0"));
}

#[test]
fn turn_option_prefix_can_branch_after_hidden_draw_order_is_revealed() {
    let first = position_with_battle_trance_draw_order([
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
    ]);
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 3);
    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden draw-order variants should initially share one information set");
    let mut root = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut budget = turn_option_budget(3, 300, None);

    let root_report = root
        .widen(&mut budget, 1)
        .expect("open Battle Trance as the first public prefix action");
    let root_action = root_report.newly_opened[0].action.clone();
    assert!(matches!(
        &root_action,
        CombatPublicActionV1::PlayCard { card_id, .. } if card_id == "Battle Trance"
    ));
    assert_eq!(root_report.newly_opened[0].successors.len(), 2);

    let successor_groups = root
        .successor_groups(&root_action)
        .expect("the opened action retains exact successor groups")
        .to_vec();
    let mut child_actions = std::collections::BTreeSet::new();
    for group in successor_groups {
        let child_key = group.view().key.clone();
        let mut child = CombatTurnOptionPrefixExpansionSessionV1::new(group);
        let child_report = child
            .widen(&mut budget, 1)
            .expect("each revealed information set can continue independently");
        assert_eq!(child_report.information_set, child_key);
        child_actions.insert(child_report.newly_opened[0].action.clone());
    }

    assert_eq!(
        child_actions.len(),
        2,
        "revealed public branches may choose different continuation actions"
    );
    assert_eq!(budget.snapshot().candidate_evaluations, 3);
}

#[test]
fn turn_option_candidate_budget_is_shared_across_revealed_branches() {
    let first = position_with_battle_trance_draw_order([
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
    ]);
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 3);
    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("hidden draw-order variants initially group");
    let mut root = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut budget = turn_option_budget(2, 300, None);
    let root_report = root
        .widen(&mut budget, 1)
        .expect("root action uses the first shared candidate evaluation");
    let root_action = root_report.newly_opened[0].action.clone();
    let successor_groups = root
        .successor_groups(&root_action)
        .expect("revealing action keeps both public successors")
        .to_vec();

    let mut first_child =
        CombatTurnOptionPrefixExpansionSessionV1::new(successor_groups[0].clone());
    let first_child_report = first_child
        .widen(&mut budget, 1)
        .expect("first child uses the final shared candidate evaluation");
    assert_eq!(first_child_report.newly_opened.len(), 1);

    let mut second_child =
        CombatTurnOptionPrefixExpansionSessionV1::new(successor_groups[1].clone());
    let second_child_report = second_child
        .widen(&mut budget, 1)
        .expect("candidate-budget exhaustion is a typed partial result");
    assert!(second_child_report.newly_opened.is_empty());
    assert_eq!(
        second_child_report.status,
        CombatTurnOptionPrefixExpansionStatusV1::PartiallyExpanded {
            cause: CombatTurnOptionPrefixExpansionStopV1::CandidateEvaluationBudget,
        }
    );
    assert_eq!(second_child_report.budget.candidate_evaluations, 2);
}

#[test]
fn turn_option_engine_and_deadline_stops_remain_inconclusive() {
    let position = position_with_battle_trance_draw_order([
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
    ]);
    let groups = group_combat_scenarios_v1(vec![particle("single", position)])
        .expect("single Battle Trance world");

    let mut baseline = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut baseline_budget = turn_option_budget(1, 100, None);
    let baseline_report = baseline
        .widen(&mut baseline_budget, 1)
        .expect("measure the complete public transition");
    let required_steps = baseline_report.new_engine_steps;
    assert!(required_steps > 1);

    let mut engine_limited = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut engine_budget = turn_option_budget(1, required_steps - 1, None);
    let engine_report = engine_limited
        .widen(&mut engine_budget, 1)
        .expect("engine exhaustion is a typed partial result");
    assert!(engine_report.newly_opened.is_empty());
    assert_eq!(engine_report.new_candidate_evaluations, 1);
    assert_eq!(
        engine_report.status,
        CombatTurnOptionPrefixExpansionStatusV1::PartiallyExpanded {
            cause: CombatTurnOptionPrefixExpansionStopV1::EngineStepBudget,
        }
    );
    assert_eq!(engine_limited.opened_action_count(), 0);
    let remaining_after_stop = engine_limited.remaining_action_count();
    engine_budget
        .grant(CombatTurnOptionExpansionBudgetGrantV1 {
            additional_candidate_evaluations: 1,
            additional_engine_steps: required_steps,
            wall_time_ms: None,
        })
        .expect("a new explicit budget slice preserves prior accounting");
    let retry_report = engine_limited
        .widen(&mut engine_budget, 1)
        .expect("the unknown candidate is retried under the new budget slice");
    assert_eq!(retry_report.previous_opened_action_count, 0);
    assert_eq!(retry_report.newly_opened.len(), 1);
    assert_eq!(
        retry_report.newly_opened[0].action,
        baseline_report.newly_opened[0].action
    );
    assert_eq!(engine_limited.opened_action_count(), 1);
    assert_eq!(
        engine_limited.remaining_action_count(),
        remaining_after_stop.saturating_sub(1)
    );

    let mut deadline_limited = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut deadline_budget = turn_option_budget(1, 100, Some(0));
    let deadline_report = deadline_limited
        .widen(&mut deadline_budget, 1)
        .expect("deadline exhaustion is a typed partial result");
    assert!(deadline_report.newly_opened.is_empty());
    assert_eq!(deadline_report.new_candidate_evaluations, 0);
    assert_eq!(
        deadline_report.status,
        CombatTurnOptionPrefixExpansionStatusV1::PartiallyExpanded {
            cause: CombatTurnOptionPrefixExpansionStopV1::Deadline,
        }
    );
    assert!(deadline_report.budget.deadline_reached);
}

#[test]
fn turn_option_prefix_status_has_one_canonical_json_contract() {
    let mut position = position_with_monster_id(7);
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    position.combat.entities.monsters[0].set_planned_move_id(1);
    position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
    position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack));
    let groups = group_combat_scenarios_v1(vec![particle("status", position)])
        .expect("status scenario groups");
    let mut session = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut budget = turn_option_budget(2, 100, None);

    let partial = session.widen(&mut budget, 1).expect("partial widening");
    assert!(partial.remaining_action_count > 0);
    assert_eq!(
        partial.status,
        CombatTurnOptionPrefixExpansionStatusV1::PartiallyExpanded {
            cause: CombatTurnOptionPrefixExpansionStopV1::RequestedWidth,
        }
    );
    let value = serde_json::to_value(&partial).expect("serialize typed status");
    assert_eq!(value["status"]["kind"], "partially_expanded");
    assert_eq!(value["status"]["cause"]["kind"], "requested_width");
    assert!(value.get("completion").is_none());
    assert!(value.get("stop").is_none());
    let round_trip: CombatTurnOptionPrefixExpansionV1 =
        serde_json::from_value(value.clone()).expect("round-trip typed status");
    assert_eq!(round_trip, partial);

    let mut legacy = value.clone();
    let legacy_object = legacy.as_object_mut().expect("expansion object");
    legacy_object.remove("status");
    legacy_object.insert("completion".to_string(), serde_json::json!("exhausted"));
    legacy_object.insert("stop".to_string(), serde_json::json!("deadline"));
    assert!(serde_json::from_value::<CombatTurnOptionPrefixExpansionV1>(legacy).is_err());

    let mut unknown = value;
    unknown["status"]
        .as_object_mut()
        .expect("status object")
        .insert("legacy_stop".to_string(), serde_json::json!("deadline"));
    assert!(serde_json::from_value::<CombatTurnOptionPrefixExpansionV1>(unknown).is_err());

    let exhausted = session.widen(&mut budget, 8).expect("exhaust widening");
    assert_eq!(exhausted.remaining_action_count, 0);
    assert_eq!(
        exhausted.status,
        CombatTurnOptionPrefixExpansionStatusV1::Exhausted
    );
}

#[test]
fn turn_option_budget_grant_is_transactional_on_overflow() {
    let mut budget = turn_option_budget(1, 1, None);
    let before = budget.snapshot();

    let error = budget
        .grant(CombatTurnOptionExpansionBudgetGrantV1 {
            additional_candidate_evaluations: 1,
            additional_engine_steps: usize::MAX,
            wall_time_ms: Some(10),
        })
        .expect_err("engine limit overflow must reject the whole grant");

    assert_eq!(
        error,
        CombatTurnOptionExpansionErrorV1::InvalidLimit {
            field: "additional_engine_steps",
        }
    );
    assert_eq!(budget.snapshot(), before);
}

#[test]
fn identical_strike_actions_have_same_observable_effect_without_sharing_history() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Strike, 11),
    ];
    lock_test_monster_attack(&mut position);
    let groups = group_combat_scenarios_v1(vec![particle("identical-strikes", position)])
        .expect("identical Strike scenario groups");
    let mut session = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut budget = turn_option_budget(16, 5_000, None);

    let report = session
        .widen(&mut budget, 16)
        .expect("expand both public Strike actions");
    let strikes = report
        .newly_opened
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.action,
                CombatPublicActionV1::PlayCard { ref card_id, .. } if card_id == "Strike_R"
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(strikes.len(), 2);
    assert_ne!(strikes[0].action, strikes[1].action);
    assert_eq!(strikes[0].successors.len(), 1);
    assert_eq!(strikes[1].successors.len(), 1);
    assert_ne!(
        strikes[0].successors[0].information_set.public_history_id,
        strikes[1].successors[0].information_set.public_history_id,
        "the two public actions remain distinct history edges"
    );

    let first = combat_turn_option_observable_effect_v1(strikes[0]);
    let second = combat_turn_option_observable_effect_v1(strikes[1]);
    assert_eq!(
        compare_combat_turn_option_observable_effects_v1(&first, &second),
        CombatTurnOptionObservableEffectRelationV1::ObservablySame
    );

    let json = serde_json::to_string(&first).expect("serialize public observable effect");
    for forbidden in [
        "identical-strikes",
        "public_history_id",
        "hand_index",
        "action",
        "uuid",
        "engine_steps",
    ] {
        assert!(
            !json.contains(forbidden),
            "observable-effect evidence must not expose `{forbidden}`"
        );
    }
}

#[test]
fn strike_and_defend_have_observably_different_effects() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Defend, 11),
    ];
    lock_test_monster_attack(&mut position);
    let groups = group_combat_scenarios_v1(vec![particle("attack-or-block", position)])
        .expect("Strike and Defend scenario groups");
    let mut session = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut budget = turn_option_budget(16, 5_000, None);
    let report = session
        .widen(&mut budget, 16)
        .expect("expand Strike and Defend");
    let strike = report
        .newly_opened
        .iter()
        .find(|candidate| {
            matches!(
                candidate.action,
                CombatPublicActionV1::PlayCard { ref card_id, .. } if card_id == "Strike_R"
            )
        })
        .expect("expanded Strike");
    let defend = report
        .newly_opened
        .iter()
        .find(|candidate| {
            matches!(
                candidate.action,
                CombatPublicActionV1::PlayCard { ref card_id, .. } if card_id == "Defend_R"
            )
        })
        .expect("expanded Defend");

    assert_eq!(
        compare_combat_turn_option_observable_effects_v1(
            &combat_turn_option_observable_effect_v1(strike),
            &combat_turn_option_observable_effect_v1(defend),
        ),
        CombatTurnOptionObservableEffectRelationV1::ObservablyDifferent
    );
}

#[test]
fn terminal_and_malformed_effect_comparisons_remain_inconclusive() {
    let terminal = CombatTurnOptionPrefixCandidateV1 {
        action: CombatPublicActionV1::EndTurn,
        scenario_count: 1,
        wins: 1,
        losses: 0,
        escapes: 0,
        continuing: 0,
        successors: Vec::new(),
        engine_steps: 1,
    };
    let terminal_evidence = combat_turn_option_observable_effect_v1(&terminal);
    assert!(matches!(
        terminal_evidence,
        CombatTurnOptionObservableEffectEvidenceV1::Available { .. }
    ));
    assert_eq!(
        compare_combat_turn_option_observable_effects_v1(&terminal_evidence, &terminal_evidence,),
        CombatTurnOptionObservableEffectRelationV1::Inconclusive {
            reason: CombatTurnOptionObservableEffectComparisonGapV1::TerminalPublicStateUnavailable,
        }
    );

    let mut malformed = terminal;
    malformed.scenario_count = 2;
    let malformed_evidence = combat_turn_option_observable_effect_v1(&malformed);
    assert!(matches!(
        malformed_evidence,
        CombatTurnOptionObservableEffectEvidenceV1::Inconclusive {
            gap: CombatTurnOptionObservableEffectEvidenceGapV1::OutcomeCountMismatch {
                input_scenario_count: 2,
                accounted_scenario_count: 1,
            },
        }
    ));
    assert_eq!(
        compare_combat_turn_option_observable_effects_v1(&malformed_evidence, &malformed_evidence,),
        CombatTurnOptionObservableEffectRelationV1::Inconclusive {
            reason:
                CombatTurnOptionObservableEffectComparisonGapV1::PublicEffectEvidenceUnavailable,
        }
    );
}

#[test]
fn turn_option_scheduler_expands_arbitrary_action_keys_without_replay() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 10),
        CombatCard::new(CardId::Defend, 11),
        CombatCard::new(CardId::Bash, 12),
    ];
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    position.combat.entities.monsters[0].set_planned_move_id(1);
    position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
    position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack));
    let groups = group_combat_scenarios_v1(vec![particle("scheduled", position)])
        .expect("scheduled root group");
    let mut session = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut budget = turn_option_budget(16, 5_000, None);
    let initial_context = session.widening_context(&budget);
    assert!(initial_context
        .candidates
        .iter()
        .all(|candidate| candidate.result.is_none() && candidate.observable_effect.is_none()));
    let canonical = initial_context
        .candidates
        .iter()
        .map(|candidate| candidate.action.clone())
        .collect::<Vec<_>>();
    assert!(canonical.len() >= 4);

    let first = canonical.last().expect("last canonical action").clone();
    let first_report = session
        .widen_next_with_schedule(&mut budget, &SelectActionSchedule(first.clone()))
        .expect("scheduler may expand a non-prefix action first");
    assert_eq!(first_report.newly_opened[0].action, first);
    assert_eq!(first_report.expansion_order, vec![first.clone()]);
    let opened_context = session.widening_context(&budget);
    for candidate in &opened_context.candidates {
        if candidate.action == &first {
            assert!(candidate.result.is_some());
            assert!(candidate.observable_effect.is_some());
        } else {
            assert!(candidate.result.is_none());
            assert!(candidate.observable_effect.is_none());
        }
    }

    let state_before_rejection = session.widening_context(&budget).expansion_order.to_vec();
    let budget_before_rejection = budget.snapshot();
    assert_eq!(
        session
            .widen_next_with_schedule(&mut budget, &SelectActionSchedule(first.clone()))
            .expect_err("expanded action cannot be selected twice"),
        CombatTurnOptionExpansionErrorV1::SelectedAlreadyExpandedCandidate {
            action: first.clone(),
        }
    );
    let unknown = CombatPublicActionV1::UsePotion {
        potion_slot: 99,
        potion_id: "unknown-schedule-action".to_string(),
        target: None,
    };
    assert_eq!(
        session
            .widen_next_with_schedule(&mut budget, &SelectActionSchedule(unknown.clone()))
            .expect_err("unknown action cannot be selected"),
        CombatTurnOptionExpansionErrorV1::SelectedUnknownCandidate { action: unknown }
    );
    assert_eq!(
        session
            .widen_next_with_schedule(&mut budget, &ExhaustedSchedule)
            .expect_err("schedule cannot hide unopened candidates"),
        CombatTurnOptionExpansionErrorV1::ReportedExhaustedWithUnopenedCandidates {
            remaining_action_count: canonical.len() - 1,
        }
    );
    assert_eq!(budget.snapshot(), budget_before_rejection);
    assert_eq!(
        session.widening_context(&budget).expansion_order,
        state_before_rejection
    );

    let mut selected = vec![first];
    for action in canonical.iter().rev().skip(1) {
        let report = session
            .widen_next_with_schedule(&mut budget, &SelectActionSchedule(action.clone()))
            .expect("each remaining action expands exactly once");
        selected.push(action.clone());
        assert_eq!(report.newly_opened.len(), 1);
        assert_eq!(report.newly_opened[0].action, *action);
        assert_eq!(report.expansion_order, selected);
    }
    assert_eq!(session.opened_action_count(), canonical.len());
    assert_eq!(session.remaining_action_count(), 0);
    assert_eq!(
        session
            .widen_next_with_schedule(&mut budget, &ExhaustedSchedule)
            .expect("an actually exhausted scheduler is a zero-work success")
            .status,
        CombatTurnOptionPrefixExpansionStatusV1::Exhausted
    );
}
#[test]
fn public_turn_option_commits_a_noncanonical_scheduled_action() {
    let mut position = position_with_monster_id(7);
    position
        .combat
        .zones
        .hand
        .push(CombatCard::new(CardId::Defend, 11));
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    position.combat.entities.monsters[0].set_planned_move_id(1);
    position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
    position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack));
    let groups = group_combat_scenarios_v1(vec![particle("scheduled-option", position)])
        .expect("scheduled option root");
    assert_ne!(
        groups[0].view().candidates[0],
        CombatPublicActionV1::EndTurn,
        "EndTurn should not be the canonical first candidate in this fixture"
    );
    let mut option = CombatPublicTurnOptionCompositionSessionV1::new(
        groups[0].clone(),
        turn_option_budget(1, 500, None),
    )
    .expect("scheduled public option");
    let root_key = option.snapshot().open_leaves[0].information_set.clone();

    let report = option
        .widen_open_leaf_with_schedule(
            &root_key,
            &SelectActionSchedule(CombatPublicActionV1::EndTurn),
        )
        .expect("schedule EndTurn before canonical card actions");
    assert_eq!(report.newly_opened.len(), 1);
    assert_eq!(report.newly_opened[0].action, CombatPublicActionV1::EndTurn);
    assert_eq!(report.expansion_order, vec![CombatPublicActionV1::EndTurn]);
    let complete = option
        .commit_opened_action(&root_key, &CombatPublicActionV1::EndTurn)
        .expect("commit the retained scheduled EndTurn transition");
    assert_eq!(
        complete.completion,
        CombatPublicTurnOptionCompletionV1::Complete
    );
    assert_eq!(complete.decisions.len(), 1);
    assert_eq!(complete.decisions[0].action, CombatPublicActionV1::EndTurn);
}
#[test]
fn public_turn_option_duplicate_successor_rejection_is_transactional() {
    let mut position = position_with_monster_id(7);
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    position.combat.entities.monsters[0].set_planned_move_id(1);
    position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
    position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack));
    let groups = group_combat_scenarios_v1(vec![particle("transactional", position)])
        .expect("transactional root group");

    let mut probe = CombatTurnOptionPrefixExpansionSessionV1::new(groups[0].clone());
    let mut probe_budget = turn_option_budget(1, 200, None);
    let probe_report = probe
        .widen(&mut probe_budget, 1)
        .expect("probe the exact successor key");
    let action = probe_report.newly_opened[0].action.clone();
    let successor_group = probe
        .successor_groups(&action)
        .expect("opened action keeps its successor")[0]
        .clone();
    let successor_key = successor_group.view().key.clone();

    let mut option = CombatPublicTurnOptionCompositionSessionV1::new(
        groups[0].clone(),
        turn_option_budget(1, 200, None),
    )
    .expect("transactional option root");
    let root_key = option.snapshot().open_leaves[0].information_set.clone();
    let option_report = option
        .widen_open_leaf(&root_key, 1)
        .expect("open the same root action");
    assert_eq!(option_report.newly_opened[0].action, action);
    option.insert_open_leaf_for_test(successor_group);
    let snapshot_before = option.snapshot();
    let budget_before = option.budget_snapshot();

    assert_eq!(
        option
            .commit_opened_action(&root_key, &action)
            .expect_err("duplicate successor must reject before consuming the root"),
        CombatPublicTurnOptionCompositionErrorV1::DuplicateOpenInformationSet {
            information_set: successor_key.clone(),
        }
    );
    assert_eq!(option.snapshot(), snapshot_before);
    assert_eq!(option.budget_snapshot(), budget_before);

    option.remove_open_leaf_for_test(&successor_key);
    let committed = option
        .commit_opened_action(&root_key, &action)
        .expect("the retained action remains committable after collision removal");
    assert_eq!(committed.decisions.len(), 1);
    assert_eq!(committed.decisions[0].action, action);
}
#[test]
fn public_turn_option_composes_opened_prefixes_without_replaying_them() {
    let mut position = position_with_monster_id(7);
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    position.combat.entities.monsters[0].set_planned_move_id(1);
    position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
    position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack));
    let groups = group_combat_scenarios_v1(vec![particle("single", position)])
        .expect("single public scenario");
    let mut option = CombatPublicTurnOptionCompositionSessionV1::new(
        groups[0].clone(),
        turn_option_budget(1, 200, None),
    )
    .expect("player-turn root starts a public turn option");

    let root_key = option.snapshot().open_leaves[0].information_set.clone();
    let root_report = option
        .widen_open_leaf(&root_key, 1)
        .expect("open Strike at the root");
    let strike = root_report.newly_opened[0].action.clone();
    assert!(matches!(&strike, CombatPublicActionV1::PlayCard { .. }));
    let budget_after_root_expansion = option.budget_snapshot();
    let after_strike = option
        .commit_opened_action(&root_key, &strike)
        .expect("compose the already-expanded Strike transition");
    assert_eq!(option.budget_snapshot(), budget_after_root_expansion);
    assert_eq!(
        after_strike.completion,
        CombatPublicTurnOptionCompletionV1::Open
    );
    assert_eq!(after_strike.decisions.len(), 1);
    assert_eq!(after_strike.open_leaves.len(), 1);

    let child_key = after_strike.open_leaves[0].information_set.clone();
    let stopped_child = option
        .widen_open_leaf(&child_key, 1)
        .expect("the exhausted first slice keeps the child unknown");
    assert!(stopped_child.newly_opened.is_empty());
    assert_eq!(
        stopped_child.status,
        CombatTurnOptionPrefixExpansionStatusV1::PartiallyExpanded {
            cause: CombatTurnOptionPrefixExpansionStopV1::CandidateEvaluationBudget,
        }
    );
    option
        .grant_budget(CombatTurnOptionExpansionBudgetGrantV1 {
            additional_candidate_evaluations: 1,
            additional_engine_steps: 100,
            wall_time_ms: None,
        })
        .expect("grant a second explicit composition budget slice");
    let child_report = option
        .widen_open_leaf(&child_key, 1)
        .expect("resume the same exact child leaf without replaying Strike");
    assert_eq!(child_report.previous_opened_action_count, 0);
    let end_turn = child_report.newly_opened[0].action.clone();
    assert_eq!(end_turn, CombatPublicActionV1::EndTurn);
    let budget_after_child_expansion = option.budget_snapshot();
    let complete = option
        .commit_opened_action(&child_key, &end_turn)
        .expect("compose the already-expanded turn boundary");

    assert_eq!(option.budget_snapshot(), budget_after_child_expansion);
    assert_eq!(
        complete.completion,
        CombatPublicTurnOptionCompletionV1::Complete
    );
    assert!(complete.open_leaves.is_empty());
    assert_eq!(complete.decisions.len(), 2);
    assert!(complete
        .decisions
        .iter()
        .flat_map(|decision| decision.successors.iter())
        .any(|successor| {
            matches!(
                successor,
                CombatPublicTurnOptionSuccessorV1::NextPlayerTurn { turn_count, .. }
                    if *turn_count > complete.root_turn_count
            )
        }));
    assert_eq!(option.budget_snapshot().candidate_evaluations, 2);
    let json = serde_json::to_string(&complete).expect("serialize public turn option");
    assert!(!json.contains("single"));
    assert!(!json.contains("uuid"));
    assert!(!json.contains("rng"));
}

#[test]
fn public_turn_option_preserves_uncommitted_revealed_sibling() {
    let mut first = position_with_battle_trance_draw_order([
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
    ]);
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 3);
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    for position in [&mut first, &mut second] {
        position.combat.entities.monsters[0].set_planned_move_id(1);
        position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
        position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack.clone()));
    }
    let groups = group_combat_scenarios_v1(vec![
        particle("private-first-branch", first),
        particle("private-second-branch", second),
    ])
    .expect("hidden draw orders initially share one public information set");
    let mut option = CombatPublicTurnOptionCompositionSessionV1::new(
        groups[0].clone(),
        turn_option_budget(16, 1_000, None),
    )
    .expect("paired draw orders start one public option");

    let root_key = option.snapshot().open_leaves[0].information_set.clone();
    let root_report = option
        .widen_open_leaf(&root_key, 1)
        .expect("open Battle Trance");
    let battle_trance = root_report.newly_opened[0].action.clone();
    let revealed = option
        .commit_opened_action(&root_key, &battle_trance)
        .expect("Battle Trance reveals two public continuations");
    assert_eq!(revealed.open_leaves.len(), 2);
    assert_eq!(
        revealed
            .open_leaves
            .iter()
            .map(|leaf| leaf.scenario_count)
            .sum::<usize>(),
        2
    );
    let first_key = revealed.open_leaves[0].information_set.clone();
    let second_key = revealed.open_leaves[1].information_set.clone();

    let first_report = option
        .widen_open_leaf(&first_key, 16)
        .expect("open the first revealed branch");
    let first_end_turn = first_report
        .newly_opened
        .iter()
        .find(|candidate| candidate.action == CombatPublicActionV1::EndTurn)
        .expect("first branch EndTurn")
        .action
        .clone();
    let one_closed = option
        .commit_opened_action(&first_key, &first_end_turn)
        .expect("close only the first revealed branch");
    assert_eq!(
        one_closed.completion,
        CombatPublicTurnOptionCompletionV1::Open
    );
    assert_eq!(one_closed.open_leaves.len(), 1);
    assert_eq!(one_closed.open_leaves[0].information_set, second_key);
    assert_eq!(one_closed.open_leaves[0].scenario_count, 1);

    let second_report = option
        .widen_open_leaf(&second_key, 16)
        .expect("open the preserved sibling branch");
    let second_end_turn = second_report
        .newly_opened
        .iter()
        .find(|candidate| candidate.action == CombatPublicActionV1::EndTurn)
        .expect("second branch EndTurn")
        .action
        .clone();
    let complete = option
        .commit_opened_action(&second_key, &second_end_turn)
        .expect("the option completes only after both siblings close");
    assert_eq!(
        complete.completion,
        CombatPublicTurnOptionCompletionV1::Complete
    );
    assert!(complete.open_leaves.is_empty());
    let mut reverse = CombatPublicTurnOptionCompositionSessionV1::new(
        groups[0].clone(),
        turn_option_budget(16, 1_000, None),
    )
    .expect("reverse-order paired draw option");
    let reverse_root_key = reverse.snapshot().open_leaves[0].information_set.clone();
    let reverse_root_report = reverse
        .widen_open_leaf(&reverse_root_key, 1)
        .expect("open Battle Trance in reverse-order option");
    let reverse_battle_trance = reverse_root_report.newly_opened[0].action.clone();
    let reverse_revealed = reverse
        .commit_opened_action(&reverse_root_key, &reverse_battle_trance)
        .expect("reveal reverse-order siblings");
    let reverse_first_key = reverse_revealed.open_leaves[0].information_set.clone();
    let reverse_second_key = reverse_revealed.open_leaves[1].information_set.clone();

    let reverse_second_report = reverse
        .widen_open_leaf(&reverse_second_key, 16)
        .expect("open second sibling first");
    let reverse_second_end_turn = reverse_second_report
        .newly_opened
        .iter()
        .find(|candidate| candidate.action == CombatPublicActionV1::EndTurn)
        .expect("reverse second branch EndTurn")
        .action
        .clone();
    reverse
        .commit_opened_action(&reverse_second_key, &reverse_second_end_turn)
        .expect("close second sibling first");
    let reverse_first_report = reverse
        .widen_open_leaf(&reverse_first_key, 16)
        .expect("open first sibling second");
    let reverse_first_end_turn = reverse_first_report
        .newly_opened
        .iter()
        .find(|candidate| candidate.action == CombatPublicActionV1::EndTurn)
        .expect("reverse first branch EndTurn")
        .action
        .clone();
    let reverse_complete = reverse
        .commit_opened_action(&reverse_first_key, &reverse_first_end_turn)
        .expect("close first sibling second");
    assert_eq!(
        reverse_complete, complete,
        "sibling commit order must not change the canonical public option"
    );
    let json = serde_json::to_string(&complete).expect("serialize branched public option");
    assert!(!json.contains("private-first-branch"));
    assert!(!json.contains("private-second-branch"));
    assert!(!json.contains("uuid"));
    assert!(!json.contains("rng"));
}
#[test]
fn smoke_bomb_turn_option_closes_with_typed_escape() {
    let mut position = position_with_monster_id(7);
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    position.combat.entities.monsters[0].set_planned_move_id(1);
    position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
    position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack));
    position.combat.entities.potions = vec![Some(crate::content::potions::Potion::new(
        crate::content::potions::PotionId::SmokeBomb,
        1,
    ))];
    let groups = group_combat_scenarios_v1(vec![particle("smoke", position)])
        .expect("Smoke Bomb scenario exposes public actions");
    let mut option = CombatPublicTurnOptionCompositionSessionV1::new(
        groups[0].clone(),
        turn_option_budget(16, 1_000, None),
    )
    .expect("Smoke Bomb starts from a player-turn boundary");

    let root_key = option.snapshot().open_leaves[0].information_set.clone();
    let root_report = option
        .widen_open_leaf(&root_key, 16)
        .expect("every root action, including Smoke Bomb, reaches a typed boundary");
    let smoke_bomb = root_report
        .newly_opened
        .iter()
        .find_map(|candidate| match &candidate.action {
            CombatPublicActionV1::UsePotion { potion_id, .. } if potion_id == "SmokeBomb" => {
                Some(candidate.action.clone())
            }
            _ => None,
        })
        .expect("Smoke Bomb public action");
    let after_smoke = option
        .commit_opened_action(&root_key, &smoke_bomb)
        .expect("Smoke Bomb remains an open same-turn option leaf until EndTurn");
    assert_eq!(
        after_smoke.completion,
        CombatPublicTurnOptionCompletionV1::Open
    );
    assert_eq!(after_smoke.open_leaves.len(), 1);

    let escape_key = after_smoke.open_leaves[0].information_set.clone();
    let escape_report = option
        .widen_open_leaf(&escape_key, 16)
        .expect("post-Smoke-Bomb EndTurn reaches the escape boundary");
    let end_turn = escape_report
        .newly_opened
        .iter()
        .find(|candidate| candidate.action == CombatPublicActionV1::EndTurn)
        .expect("EndTurn after Smoke Bomb")
        .action
        .clone();
    let complete = option
        .commit_opened_action(&escape_key, &end_turn)
        .expect("typed Escape closes the option");

    assert_eq!(
        complete.completion,
        CombatPublicTurnOptionCompletionV1::Complete
    );
    assert!(complete
        .decisions
        .iter()
        .flat_map(|decision| decision.successors.iter())
        .any(|successor| {
            matches!(
                successor,
                CombatPublicTurnOptionSuccessorV1::Terminal {
                    terminal: CombatPublicTurnOptionTerminalV1::Escape,
                    scenario_count: 1,
                }
            )
        }));
}
#[test]
fn pending_hand_choice_groups_different_uuids_and_collapses_identical_cards() {
    let first = pending_hand_select_position(
        vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Strike, 20),
        ],
        1,
        1,
    );
    let second = pending_hand_select_position(
        vec![
            CombatCard::new(CardId::Strike, 70_001),
            CombatCard::new(CardId::Strike, 99_001),
        ],
        1,
        1,
    );

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("publicly identical hand choices should group");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().scenario_count, 2);
    assert_eq!(groups[0].view().candidates.len(), 1);
    let pending = groups[0]
        .view()
        .observation
        .pending_choice
        .as_ref()
        .expect("pending choice observation");
    let CombatPublicPendingChoiceV1::HandSelect { candidates, .. } = pending else {
        panic!("expected hand selection, got {pending:?}");
    };
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].count, 2);

    let binding = groups[0]
        .bind_action(&groups[0].view().candidates[0])
        .expect("public multiset selection should bind in both worlds");
    assert_eq!(binding.scenario_count(), 2);
    let public_json =
        serde_json::to_string(groups[0].view()).expect("public pending choice serialization");
    assert!(!public_json.contains("70001"));
    assert!(!public_json.contains("99001"));
    assert!(!public_json.contains("uuid"));
}

#[test]
fn pending_selection_enumerates_all_combinations_beyond_legacy_sixteen_cap() {
    let cards = [
        CardId::Bash,
        CardId::Defend,
        CardId::Strike,
        CardId::Anger,
        CardId::Cleave,
        CardId::ShrugItOff,
        CardId::PommelStrike,
    ]
    .into_iter()
    .enumerate()
    .map(|(index, card_id)| CombatCard::new(card_id, 100 + index as u32))
    .collect();
    let position = pending_hand_select_position(cards, 2, 2);

    let groups = group_combat_scenarios_v1(vec![particle("all-pairs", position)])
        .expect("seven choose two should be fully enumerated");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().candidates.len(), 21);
}

#[test]
fn hidden_draw_order_does_not_leak_through_grid_selection_candidates() {
    let mut first = position_with_monster_id(7);
    first.combat.zones.draw_pile = vec![
        CombatCard::new(CardId::Bash, 10),
        CombatCard::new(CardId::Defend, 20),
    ];
    first.engine = EngineState::PendingChoice(PendingChoice::GridSelect {
        source_pile: PileType::Draw,
        candidate_uuids: vec![10, 20],
        min_cards: 1,
        max_cards: 1,
        can_cancel: false,
        reason: GridSelectReason::DrawPileToHand,
    });
    let mut second = first.clone();
    second.combat.zones.draw_pile.swap(0, 1);
    if let EngineState::PendingChoice(PendingChoice::GridSelect {
        candidate_uuids, ..
    }) = &mut second.engine
    {
        candidate_uuids.swap(0, 1);
    }

    let groups =
        group_combat_scenarios_v1(vec![particle("first", first), particle("second", second)])
            .expect("grid presentation must not reveal hidden draw order");

    assert_eq!(groups.len(), 1);
    assert_eq!(groups[0].view().scenario_count, 2);
}

#[test]
fn step_loop_crosses_hand_choice_without_exposing_uuid() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.hand = vec![
        CombatCard::new(CardId::Armaments, 10),
        CombatCard::new(CardId::Strike, 20),
        CombatCard::new(CardId::Defend, 30),
    ];
    let groups = group_combat_scenarios_v1(vec![particle("armaments", position)])
        .expect("Armaments root information set");
    let armaments = groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| {
            matches!(
                action,
                CombatPublicActionV1::PlayCard { card_id, .. } if card_id == "Armaments"
            )
        })
        .expect("Armaments play")
        .clone();

    let pending = step_combat_scenario_group_v1(
        &groups[0],
        &armaments,
        CombatStepLimits {
            max_engine_steps: 100,
            deadline: None,
        },
    )
    .expect("Armaments should step to a public hand choice");

    assert_eq!(pending.next_groups.len(), 1);
    assert_eq!(
        pending.next_groups[0].view().observation.engine_state,
        "combat_pending_choice"
    );
    let choose_strike = pending.next_groups[0]
        .view()
        .candidates
        .iter()
        .find(|action| {
            matches!(
                action,
                CombatPublicActionV1::SelectCards { selected, .. }
                    if selected.len() == 1 && selected[0].card.card_id == "Strike_R"
            )
        })
        .expect("public Strike selection")
        .clone();

    let resumed = step_combat_scenario_group_v1(
        &pending.next_groups[0],
        &choose_strike,
        CombatStepLimits {
            max_engine_steps: 100,
            deadline: None,
        },
    )
    .expect("public selection should resume card resolution");

    assert_eq!(resumed.next_groups.len(), 1);
    assert_eq!(
        resumed.next_groups[0].view().observation.engine_state,
        "combat_player_turn"
    );
    assert!(resumed.next_groups[0]
        .view()
        .observation
        .pending_choice
        .is_none());
}

#[test]
fn oversized_pending_choice_fails_with_typed_gap() {
    let mut position = position_with_monster_id(7);
    position.combat.zones.draw_pile = (0..13)
        .map(|index| CombatCard::new(CardId::Strike, 500 + index))
        .collect();
    position.engine = EngineState::PendingChoice(PendingChoice::ScrySelect {
        cards: vec![CardId::Strike; 13],
        card_uuids: (0..13).map(|index| 500 + index).collect(),
    });

    let error = match group_combat_scenarios_v1(vec![particle("wide-scry", position)]) {
        Ok(_) => panic!("wide Scry should not silently truncate its action set"),
        Err(error) => error,
    };

    assert_eq!(
        error,
        CombatScenarioPolicyErrorV1::CandidateSpaceTooLarge {
            scenario_id: "wide-scry".to_string(),
            choice_kind: CombatPublicPendingChoiceKindV1::ScrySelect,
            candidate_count: 13,
            action_count: 8_192,
            cap: 4_096,
        }
    );
}

#[test]
fn remaining_pending_choice_kinds_expose_typed_public_actions() {
    let cases = vec![
        (
            "discovery",
            PendingChoice::DiscoverySelect(DiscoveryChoiceState {
                cards: vec![CardId::Anger, CardId::Cleave],
                colorless: false,
                card_type: None,
                amount: 1,
                can_skip: true,
            }),
            3,
        ),
        (
            "card-reward",
            PendingChoice::CardRewardSelect {
                cards: vec![CardId::Bash],
                destination: CardDestination::Hand,
                can_skip: true,
            },
            2,
        ),
        (
            "foreign-influence",
            PendingChoice::ForeignInfluenceSelect {
                cards: vec![CardId::Strike],
                upgraded: true,
            },
            1,
        ),
        (
            "choose-one",
            PendingChoice::ChooseOneSelect {
                choices: vec![ChooseOneCardChoice {
                    card_id: CardId::Anger,
                    upgrades: 1,
                }],
            },
            1,
        ),
        ("stance", PendingChoice::StanceChoice, 2),
    ];

    for (scenario_id, choice, expected_actions) in cases {
        let mut position = position_with_monster_id(7);
        position.engine = EngineState::PendingChoice(choice);
        let groups = group_combat_scenarios_v1(vec![particle(scenario_id, position)])
            .expect("typed pending choice should project");
        assert_eq!(
            groups[0].view().candidates.len(),
            expected_actions,
            "{scenario_id}"
        );
        assert!(
            groups[0].view().observation.pending_choice.is_some(),
            "{scenario_id}"
        );
    }

    let mut scry = position_with_monster_id(7);
    scry.combat.zones.draw_pile = vec![
        CombatCard::new(CardId::Strike, 80_001),
        CombatCard::new(CardId::Defend, 80_002),
    ];
    scry.engine = EngineState::PendingChoice(PendingChoice::ScrySelect {
        cards: vec![CardId::Strike, CardId::Defend],
        card_uuids: vec![80_001, 80_002],
    });
    let groups = group_combat_scenarios_v1(vec![particle("scry", scry)])
        .expect("Scry should expose every discard subset");
    assert_eq!(groups[0].view().candidates.len(), 4);
    assert!(groups[0]
        .view()
        .candidates
        .iter()
        .all(|action| matches!(action, CombatPublicActionV1::ScryDiscard { .. })));
    let json = serde_json::to_string(groups[0].view()).expect("public Scry serialization");
    assert!(!json.contains("80001"));
    assert!(!json.contains("80002"));
    assert!(!json.contains("uuid"));
}

struct SelectActionSchedule(CombatPublicActionV1);

impl CombatTurnOptionWideningScheduleV1 for SelectActionSchedule {
    fn select_next(
        &self,
        _context: &CombatTurnOptionWideningContextV1<'_>,
    ) -> CombatTurnOptionWideningChoiceV1 {
        CombatTurnOptionWideningChoiceV1::Expand {
            action: self.0.clone(),
        }
    }
}

struct ExhaustedSchedule;

impl CombatTurnOptionWideningScheduleV1 for ExhaustedSchedule {
    fn select_next(
        &self,
        _context: &CombatTurnOptionWideningContextV1<'_>,
    ) -> CombatTurnOptionWideningChoiceV1 {
        CombatTurnOptionWideningChoiceV1::Exhausted
    }
}
fn particle(scenario_id: &str, position: CombatPosition) -> CombatScenarioParticleV1 {
    CombatScenarioParticleV1::root(scenario_id, position)
}

fn turn_option_budget(
    max_candidate_evaluations: usize,
    max_engine_steps: usize,
    wall_time_ms: Option<u64>,
) -> CombatTurnOptionExpansionBudgetV1 {
    CombatTurnOptionExpansionBudgetV1::new(CombatTurnOptionExpansionBudgetLimitsV1 {
        max_candidate_evaluations,
        max_engine_steps,
        wall_time_ms,
    })
    .expect("valid turn-option expansion budget")
}

fn position_with_draw_order(cards: [CardId; 2]) -> CombatPosition {
    let mut position = position_with_monster_id(7);
    position.combat.zones.draw_pile =
        vec![CombatCard::new(cards[0], 20), CombatCard::new(cards[1], 21)];
    position
}

fn position_with_battle_trance_draw_order(cards: [CardId; 4]) -> CombatPosition {
    let mut position = position_with_monster_id(7);
    position.combat.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 10)];
    position.combat.zones.draw_pile = cards
        .into_iter()
        .enumerate()
        .map(|(index, card_id)| CombatCard::new(card_id, 20 + index as u32))
        .collect();
    position
}

fn position_with_monster_id(monster_id: usize) -> CombatPosition {
    let mut combat = crate::test_support::blank_test_combat();
    combat.turn.energy = 3;
    combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
    let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
    monster.id = monster_id;
    monster.slot = 0;
    combat.entities.monsters = vec![monster];
    CombatPosition::new(EngineState::CombatPlayerTurn, combat)
}

fn lock_test_monster_attack(position: &mut CombatPosition) {
    let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
        crate::runtime::monster_move::AttackSpec {
            base_damage: 1,
            hits: 1,
            damage_kind: crate::runtime::monster_move::DamageKind::Normal,
        },
    );
    position.combat.entities.monsters[0].set_planned_move_id(1);
    position.combat.entities.monsters[0].set_planned_steps(attack.to_steps());
    position.combat.entities.monsters[0].set_planned_visible_spec(Some(attack));
}

fn power(power_id: PowerId, amount: i32) -> Power {
    Power {
        power_type: power_id,
        instance_id: None,
        amount,
        extra_data: 0,
        payload: PowerPayload::None,
        just_applied: false,
    }
}

fn pending_hand_select_position(
    hand: Vec<CombatCard>,
    min_cards: u8,
    max_cards: u8,
) -> CombatPosition {
    let mut position = position_with_monster_id(7);
    let candidate_uuids = hand.iter().map(|card| card.uuid).collect();
    position.combat.zones.hand = hand;
    position.engine = EngineState::PendingChoice(PendingChoice::HandSelect {
        candidate_uuids,
        min_cards,
        max_cards,
        can_cancel: false,
        reason: HandSelectReason::Discard,
    });
    position
}
