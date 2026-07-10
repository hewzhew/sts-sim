use super::priority::{priority_for_node, priority_for_node_with_action_prior};
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
fn frontier_priority_uses_action_prior_score_when_rollout_ties() {
    let mut low_prior = test_node();
    low_prior.action_prior_score = Some(0.1);
    let mut high_prior = test_node();
    high_prior.action_prior_score = Some(0.9);

    assert!(priority_for_node(&high_prior) > priority_for_node(&low_prior));
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
fn collector_control_frontier_prefers_one_living_head_to_zero() {
    let mut one_head = collector_node(282, &[20]);
    let zero_heads = collector_node(282, &[]);
    one_head.rollout_estimate = zero_heads.rollout_estimate.clone();

    assert!(
        priority_for_node_with_action_prior(
            &one_head,
            CombatSearchActionPriorPluginId::CollectorSingleHeadControl,
        ) > priority_for_node_with_action_prior(
            &zero_heads,
            CombatSearchActionPriorPluginId::CollectorSingleHeadControl,
        )
    );
    assert!(priority_for_node(&zero_heads) > priority_for_node(&one_head));
}

#[test]
fn collector_control_frontier_prefers_concentrated_head_damage() {
    let concentrated = collector_node(282, &[10, 40]);
    let spread = collector_node(282, &[25, 25]);

    assert!(
        priority_for_node_with_action_prior(
            &concentrated,
            CombatSearchActionPriorPluginId::CollectorSingleHeadControl,
        ) > priority_for_node_with_action_prior(
            &spread,
            CombatSearchActionPriorPluginId::CollectorSingleHeadControl,
        )
    );
}

#[test]
fn collector_boss_race_frontier_prefers_damage_on_collector() {
    let collector_damage = collector_node(252, &[40]);
    let torch_damage = collector_node(282, &[10]);

    assert!(
        priority_for_node_with_action_prior(
            &collector_damage,
            CombatSearchActionPriorPluginId::CollectorBossRace,
        ) > priority_for_node_with_action_prior(
            &torch_damage,
            CombatSearchActionPriorPluginId::CollectorBossRace,
        )
    );
}

#[test]
fn collector_tactic_frontier_queue_uses_configured_prior() {
    let mut queue = FrontierQueue::new_with_action_prior(
        CombatSearchV2FrontierPolicy::SingleQueue,
        CombatSearchActionPriorPluginId::CollectorSingleHeadControl,
    );
    queue.push_node(collector_node(282, &[]));
    queue.push_node(collector_node(282, &[20]));

    assert_eq!(queue.pop().unwrap().node.combat.entities.monsters.len(), 2);
}

#[test]
fn frontier_queue_preserves_single_queue_priority_order() {
    let mut queue = FrontierQueue::new(CombatSearchV2FrontierPolicy::SingleQueue);

    let low_priority = test_node();
    let mut high_priority = test_node();
    high_priority.potion_tactical_priority = 50;

    queue.push_node(low_priority);
    queue.push_node(high_priority);

    assert_eq!(queue.len(), 2);
    assert_eq!(queue.pop().unwrap().node.potion_tactical_priority, 50);
    assert_eq!(queue.pop().unwrap().node.potion_tactical_priority, 0);
    assert!(queue.is_empty());
}

#[test]
fn round_robin_frontier_queue_interleaves_survival_and_progress_lanes() {
    let mut queue = FrontierQueue::new(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets);

    for index in 0..8 {
        let mut progress = evaluated_node(20, 40);
        progress
            .actions
            .push(test_action_trace(format!("progress-{index}")));
        queue.push_node(progress);
    }
    let mut survival = evaluated_node(0, 300);
    survival
        .actions
        .push(test_action_trace("survival".to_string()));
    queue.push_node(survival);

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

    for index in 0..32 {
        let mut survival = evaluated_node(5, 300);
        survival
            .actions
            .push(test_action_trace(format!("survival-{index}")));
        queue.push_node(survival);
    }
    let mut dangerous_race = evaluated_node(-1, 40);
    dangerous_race
        .actions
        .push(test_action_trace("dangerous-race".to_string()));
    queue.push_node(dangerous_race);

    let popped = (0..16)
        .map(|_| queue.pop().unwrap().node.actions[0].action_key.clone())
        .collect::<Vec<_>>();

    assert!(
        popped.iter().any(|key| key == "dangerous-race"),
        "dangerous race progress should receive progress-lane budget; popped={popped:?}"
    );
}

#[test]
fn round_robin_frontier_does_not_treat_unevaluated_rollout_as_survival_crisis() {
    let mut queue = FrontierQueue::new(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets);

    for index in 0..8 {
        let mut progress = evaluated_node(20, 40);
        progress
            .actions
            .push(test_action_trace(format!("progress-{index}")));
        queue.push_node(progress);
    }
    let mut unevaluated = test_node();
    unevaluated.combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
    unevaluated
        .actions
        .push(test_action_trace("unevaluated".to_string()));
    queue.push_node(unevaluated);

    let popped = (0..8)
        .map(|_| queue.pop().unwrap().node.actions[0].action_key.clone())
        .collect::<Vec<_>>();

    assert!(
        popped.iter().all(|key| key.starts_with("progress-")),
        "unevaluated rollout should stay in balanced lane instead of taking survival-lane budget; popped={popped:?}"
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
        action_prior_score: None,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    }
}

fn collector_node(collector_hp: i32, torch_hps: &[i32]) -> SearchNode {
    let mut node = test_node();
    let mut collector = test_monster(EnemyId::TheCollector);
    collector.id = 1;
    collector.current_hp = collector_hp;
    collector.max_hp = 282;
    let mut monsters = vec![collector];
    for (index, hp) in torch_hps.iter().copied().enumerate() {
        let mut torch = test_monster(EnemyId::TorchHead);
        torch.id = index + 2;
        torch.current_hp = hp;
        torch.max_hp = 40;
        monsters.push(torch);
    }
    node.combat.entities.monsters = monsters;
    node
}
