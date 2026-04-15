use crate::runtime::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// LetterOpener: Every time you play 3 Skills in a single turn, deal 5 damage to ALL enemies.
/// Java: onUseCard() → ++counter; if counter % 3 == 0: counter=0, addToBot(DamageAllEnemiesAction(5, THORNS))
pub fn on_use_card(
    state: &crate::runtime::combat::CombatState,
    card_id: crate::content::cards::CardId,
    counter: i32,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let def = crate::content::cards::get_card_definition(card_id);

    if def.card_type == crate::content::cards::CardType::Skill {
        let next_counter = if counter + 1 >= 3 { 0 } else { counter + 1 };

        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::LetterOpener,
                counter: next_counter,
            },
            insertion_mode: AddTo::Bottom, // Java: counter mutation is inline, not an action
        });

        if next_counter == 0 {
            // Java: addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(5, true), THORNS))
            let damages: smallvec::SmallVec<[i32; 5]> =
                state.entities.monsters.iter().map(|_| 5i32).collect();
            actions.push(ActionInfo {
                action: Action::DamageAllEnemies {
                    source: 0,
                    damages,
                    damage_type: crate::runtime::action::DamageType::Thorns,
                    is_modified: false,
                },
                insertion_mode: AddTo::Bottom, // Java: addToBot
            });
        }
    }

    actions
}
