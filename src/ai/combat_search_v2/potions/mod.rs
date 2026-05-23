use super::*;

mod proposals;
mod semantics;

pub(super) use proposals::semantic_potion_action_allowed;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::{Potion, PotionId};
    use crate::runtime::combat::CombatCard;
    use crate::sim::combat_action::CombatActionChoice;
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn semantic_policy_keeps_attack_potion_when_hand_lacks_lethal() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.current_hp = 65;
        monster.max_hp = 65;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ];
        combat.entities.potions = vec![Some(Potion::new(PotionId::AttackPotion, 3))];
        let legal = vec![
            CombatActionChoice::from_input(
                &combat,
                ClientInput::UsePotion {
                    potion_index: 0,
                    target: None,
                },
            ),
            CombatActionChoice::from_input(&combat, ClientInput::DiscardPotion(0)),
            CombatActionChoice::from_input(&combat, ClientInput::EndTurn),
        ];

        let filtered =
            filtered_legal_actions(legal, CombatSearchV2PotionPolicy::SemanticBudgeted, &combat);

        assert!(filtered.iter().any(|choice| matches!(
            choice.input,
            ClientInput::UsePotion {
                potion_index: 0,
                ..
            }
        )));
        assert!(filtered
            .iter()
            .all(|choice| !matches!(choice.input, ClientInput::DiscardPotion(0))));
    }

    #[test]
    fn semantic_policy_does_not_admit_passive_fairy_use() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        combat.entities.potions = vec![Some(Potion::new(PotionId::FairyPotion, 3))];
        let legal = vec![CombatActionChoice::from_input(
            &combat,
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
        )];

        let filtered =
            filtered_legal_actions(legal, CombatSearchV2PotionPolicy::SemanticBudgeted, &combat);

        assert!(filtered.is_empty());
    }

    #[test]
    fn semantic_policy_keeps_lethal_fire_potion_without_incoming_damage() {
        let mut combat = blank_test_combat();
        let mut monster = test_monster(EnemyId::JawWorm);
        monster.current_hp = 20;
        monster.max_hp = 20;
        combat.entities.monsters = vec![monster];
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 3))];
        let legal = vec![CombatActionChoice::from_input(
            &combat,
            ClientInput::UsePotion {
                potion_index: 0,
                target: Some(1),
            },
        )];

        let filtered =
            filtered_legal_actions(legal, CombatSearchV2PotionPolicy::SemanticBudgeted, &combat);

        assert_eq!(filtered.len(), 1);
    }

    #[test]
    fn semantic_policy_rejects_block_potion_without_visible_incoming_loss() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(EnemyId::JawWorm)];
        combat.entities.potions = vec![Some(Potion::new(PotionId::BlockPotion, 3))];
        let legal = vec![CombatActionChoice::from_input(
            &combat,
            ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            },
        )];

        let filtered =
            filtered_legal_actions(legal, CombatSearchV2PotionPolicy::SemanticBudgeted, &combat);

        assert!(filtered.is_empty());
    }
}
