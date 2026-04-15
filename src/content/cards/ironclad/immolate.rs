use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn immolate_play(_state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![
        ActionInfo {
            action: Action::DamageAllEnemies {
                source: 0,
                damages: card.multi_damage.clone(),
                damage_type: crate::action::DamageType::Normal,
                is_modified: false,
            },
            insertion_mode: AddTo::Bottom,
        },
        ActionInfo {
            action: Action::MakeTempCardInDiscard {
                card_id: crate::content::cards::CardId::Burn,
                amount: 1,
                upgraded: false
            },
            insertion_mode: AddTo::Bottom,
        }
    ]
}

#[cfg(test)]
mod tests {
    use super::immolate_play;
    use crate::action::Action;
    use crate::combat::CombatCard;
    use crate::content::cards::{evaluate_card, CardId};
    use crate::content::monsters::EnemyId;
    use crate::content::powers::PowerId;
    use crate::content::test_support::{basic_combat, CombatTestExt};

    #[test]
    fn immolate_uses_per_target_multi_damage_matrix() {
        let mut combat = basic_combat();
        let first_monster = combat.entities.monsters[0].clone();
        let mut second_monster = first_monster.clone();
        second_monster.id = 2;
        combat = combat.with_monsters(vec![first_monster, second_monster]);
        combat = combat
            .with_monster_type(1, EnemyId::Looter)
            .with_monster_max_hp(1, 47)
            .with_monster_hp(1, 47)
            .with_monster_type(2, EnemyId::Mugger)
            .with_monster_max_hp(2, 50)
            .with_monster_hp(2, 50)
            .with_monster_power(2, PowerId::Vulnerable, 1);

        let mut immolate = CombatCard::new(CardId::Immolate, 101);
        immolate.upgrades = 1;
        evaluate_card(&mut immolate, &combat, None);

        let actions = immolate_play(&combat, &immolate);
        match &actions[0].action {
            Action::DamageAllEnemies { damages, .. } => {
                assert_eq!(damages.as_slice(), &[28, 42]);
            }
            other => panic!("expected DamageAllEnemies, got {:?}", other),
        }
    }
}
