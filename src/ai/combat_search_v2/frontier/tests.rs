use super::*;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::runtime::combat::CombatCard;
use crate::runtime::combat::{Power, PowerPayload};
use crate::test_support::{blank_test_combat, test_monster};

#[test]
fn frontier_priority_prefers_stronger_visible_next_draw_when_state_ties() {
    let mut strike = test_node();
    strike.combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 11)];

    let mut carnage = test_node();
    carnage.combat.zones.draw_pile = vec![CombatCard::new(CardId::Carnage, 12)];

    assert!(priority_for_node(&carnage) > priority_for_node(&strike));
}

#[test]
fn frontier_priority_prefers_higher_potion_tactical_role_when_state_ties() {
    let mut sustain = test_node();
    sustain.potion_tactical_priority = 10;

    let mut lethal = test_node();
    lethal.potion_tactical_priority = 50;

    assert!(priority_for_node(&lethal) > priority_for_node(&sustain));
}

#[test]
fn frontier_priority_uses_turn_branch_hint_as_late_tie_break() {
    let neutral = test_node();
    let mut same_turn = test_node();
    same_turn.last_turn_branch_priority = 12;

    assert!(priority_for_node(&same_turn) > priority_for_node(&neutral));
}

#[test]
fn frontier_priority_uses_sustained_mitigation_after_raw_enemy_progress() {
    let mut better_progress = test_node();
    let mut monster = test_monster(EnemyId::TheGuardian);
    monster.id = 1;
    monster.current_hp = 220;
    monster.max_hp = 240;
    better_progress.combat.entities.monsters = vec![monster.clone()];

    let mut disarmed = test_node();
    monster.current_hp = 240;
    disarmed.combat.entities.monsters = vec![monster];
    disarmed.combat.entities.power_db.insert(
        1,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: None,
            amount: -3,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }],
    );

    assert!(priority_for_node(&better_progress) > priority_for_node(&disarmed));

    better_progress.combat.entities.monsters[0].current_hp = 240;
    assert!(priority_for_node(&disarmed) > priority_for_node(&better_progress));
}

#[test]
fn frontier_queue_preserves_single_queue_priority_order() {
    let mut queue = FrontierQueue::new(CombatSearchV2FrontierPolicy::SingleQueue);
    let mut next_sequence_id = 0;

    let low_priority = test_node();
    let mut high_priority = test_node();
    high_priority.potion_tactical_priority = 50;

    push_frontier(&mut queue, low_priority, &mut next_sequence_id);
    push_frontier(&mut queue, high_priority, &mut next_sequence_id);

    assert_eq!(queue.len(), 2);
    assert_eq!(queue.pop().unwrap().node.potion_tactical_priority, 50);
    assert_eq!(queue.pop().unwrap().node.potion_tactical_priority, 0);
    assert!(queue.is_empty());
}

#[test]
fn round_robin_frontier_queue_interleaves_survival_and_progress_lanes() {
    let mut queue = FrontierQueue::new(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets);
    let mut next_sequence_id = 0;

    for index in 0..8 {
        let mut progress = evaluated_node(20, 40);
        progress
            .actions
            .push(test_action_trace(format!("progress-{index}")));
        push_frontier(&mut queue, progress, &mut next_sequence_id);
    }
    let mut survival = evaluated_node(0, 300);
    survival
        .actions
        .push(test_action_trace("survival".to_string()));
    push_frontier(&mut queue, survival, &mut next_sequence_id);

    for index in 0..7 {
        assert_eq!(
            queue.pop().unwrap().node.actions[0].action_key,
            format!("progress-{index}")
        );
    }
    assert_eq!(queue.pop().unwrap().node.actions[0].action_key, "survival");
}

#[test]
fn round_robin_frontier_gives_dangerous_race_progress_its_own_budget() {
    let mut queue = FrontierQueue::new(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets);
    let mut next_sequence_id = 0;

    for index in 0..32 {
        let mut survival = evaluated_node(5, 300);
        survival
            .actions
            .push(test_action_trace(format!("survival-{index}")));
        push_frontier(&mut queue, survival, &mut next_sequence_id);
    }
    let mut dangerous_race = evaluated_node(-1, 40);
    dangerous_race
        .actions
        .push(test_action_trace("dangerous-race".to_string()));
    push_frontier(&mut queue, dangerous_race, &mut next_sequence_id);

    let popped = (0..16)
        .map(|_| queue.pop().unwrap().node.actions[0].action_key.clone())
        .collect::<Vec<_>>();

    assert!(
        popped.iter().any(|key| key == "dangerous-race"),
        "dangerous race progress should receive progress-lane budget; popped={popped:?}"
    );
}

fn evaluated_node(survival_margin: i32, phase_adjusted_enemy_effort: i32) -> SearchNode {
    let mut node = test_node();
    node.combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    node.rollout_estimate = RolloutNodeEstimate {
        evaluated: true,
        final_hp: 30,
        survival_margin,
        phase_adjusted_enemy_effort,
        ..RolloutNodeEstimate::unevaluated()
    };
    node
}

fn test_action_trace(action_key: String) -> CombatSearchV2ActionTrace {
    CombatSearchV2ActionTrace {
        step_index: 0,
        action_id: 0,
        action_debug: action_key.clone(),
        action_key,
        input: ClientInput::EndTurn,
    }
}

fn test_node() -> SearchNode {
    SearchNode {
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
    }
}
