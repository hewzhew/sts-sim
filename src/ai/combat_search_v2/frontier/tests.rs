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
    let mut queue = FrontierQueue::new();
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
