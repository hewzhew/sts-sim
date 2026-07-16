use crate::content::cards::CardId;
use crate::content::powers::PowerId;
use crate::content::relics::{RelicId, RelicState};
use crate::runtime::combat::{CombatCard, Power, PowerPayload};

use super::*;

#[test]
fn policy_observation_keeps_public_card_runtime_and_excludes_uuid() {
    let mut combat = crate::test_support::blank_test_combat();
    let mut card = CombatCard::new(CardId::SearingBlow, 987_654);
    card.upgrades = 3;
    card.misc_value = 9;
    card.cost_modifier = -1;
    card.cost_for_turn = Some(0);
    card.free_to_play_once = true;
    combat.zones.discard_pile = vec![card];

    let observation = combat_policy_observation_v1(&combat);
    let public_card = &observation.zones.discard.cards[0];

    assert_eq!(public_card.card_id, "Searing Blow");
    assert_eq!(public_card.upgrades, 3);
    assert_eq!(public_card.misc_value, 9);
    assert_eq!(public_card.cost_modifier, -1);
    assert_eq!(public_card.cost_for_turn, 0);
    assert!(public_card.free_to_play_once);

    let json = serde_json::to_string(&observation).expect("policy observation serialization");
    assert!(!json.contains("987654"));
    assert!(!json.contains("uuid"));
}

#[test]
fn policy_observation_keeps_relic_power_and_timing_state() {
    let mut combat = crate::test_support::blank_test_combat();
    let mut pen_nib = RelicState::new(RelicId::PenNib);
    pen_nib.counter = 9;
    combat.entities.player.add_relic(pen_nib);
    combat.entities.power_db.insert(
        combat.entities.player.id,
        vec![Power {
            power_type: PowerId::Strength,
            instance_id: Some(44_444),
            amount: 3,
            extra_data: 7,
            payload: PowerPayload::None,
            just_applied: true,
        }],
    );

    let observation = combat_policy_observation_v1(&combat);

    assert_eq!(observation.player_runtime.relics[0].relic_id, "PenNib");
    assert_eq!(observation.player_runtime.relics[0].counter, 9);
    assert_eq!(observation.player_runtime.powers[0].power_id, "Strength");
    assert_eq!(observation.player_runtime.powers[0].amount, 3);
    assert_eq!(observation.player_runtime.powers[0].extra_data, 7);
    assert!(observation.player_runtime.powers[0].fresh_this_round);

    let json = serde_json::to_string(&observation).expect("policy observation serialization");
    assert!(!json.contains("44444"));
    assert!(!json.contains("instance_id"));
}
