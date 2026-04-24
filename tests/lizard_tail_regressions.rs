use sts_simulator::content::relics::{RelicId, RelicState};
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::runtime::action::Action;
use sts_simulator::test_support::blank_test_combat;

#[test]
fn lizard_tail_revives_inline_without_double_healing_from_queued_hooks() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 13;
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::LizardTail));

    execute_action(
        Action::LoseHp {
            target: 0,
            amount: 20,
            triggers_rupture: false,
        },
        &mut combat,
    );

    assert_eq!(
        combat.entities.player.current_hp, 40,
        "Lizard Tail should revive to 50% max HP immediately when lethal damage lands"
    );
    assert_eq!(
        combat.action_queue_len(),
        0,
        "Lizard Tail should not also queue a later heal through on_lose_hp hooks"
    );

    while let Some(action) = combat.pop_next_action() {
        execute_action(action, &mut combat);
    }

    assert_eq!(
        combat.entities.player.current_hp, 40,
        "draining follow-up actions must not raise the revive to full HP"
    );

    let lizard_tail = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == RelicId::LizardTail)
        .expect("player should still have Lizard Tail after revive");
    assert!(lizard_tail.used_up);
    assert_eq!(lizard_tail.counter, -2);
}
