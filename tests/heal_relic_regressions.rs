use sts_simulator::content::potions::{Potion, PotionId};
use sts_simulator::content::relics::{RelicId, RelicState};
use sts_simulator::engine::action_handlers::execute_action;
use sts_simulator::runtime::action::Action;
use sts_simulator::test_support::blank_test_combat;

#[test]
fn heal_actions_respect_magic_flower_in_combat() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 40;
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::MagicFlower));

    execute_action(
        Action::Heal {
            target: 0,
            amount: 10,
        },
        &mut combat,
    );

    assert_eq!(combat.entities.player.current_hp, 55);
}

#[test]
fn heal_actions_respect_mark_of_the_bloom_in_combat() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 40;
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::MarkOfTheBloom));

    execute_action(
        Action::Heal {
            target: 0,
            amount: 10,
        },
        &mut combat,
    );

    assert_eq!(combat.entities.player.current_hp, 40);
}

#[test]
fn lizard_tail_revive_respects_magic_flower_modifier() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 13;
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::LizardTail));
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::MagicFlower));

    execute_action(
        Action::LoseHp {
            target: 0,
            amount: 20,
            triggers_rupture: false,
        },
        &mut combat,
    );

    assert_eq!(combat.entities.player.current_hp, 60);
}

#[test]
fn fairy_potion_revive_respects_magic_flower_modifier() {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = 13;
    combat
        .entities
        .player
        .add_relic(RelicState::new(RelicId::MagicFlower));
    combat.entities.potions = vec![Some(Potion::new(PotionId::FairyPotion, 1))];

    execute_action(
        Action::LoseHp {
            target: 0,
            amount: 20,
            triggers_rupture: false,
        },
        &mut combat,
    );

    assert_eq!(combat.entities.player.current_hp, 36);
    assert!(combat.entities.potions[0].is_none());
}
