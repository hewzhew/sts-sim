use crate::core::EntityId;
use crate::runtime::action::{Action, DamageType, NO_SOURCE};
use crate::runtime::combat::CombatState;

pub fn on_attacked(
    state: &CombatState,
    owner: EntityId,
    damage: i32,
    source: EntityId,
    damage_type: DamageType,
    _power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    let owner_hp = if owner == 0 {
        state.entities.player.current_hp
    } else {
        state
            .entities
            .monsters
            .iter()
            .find(|monster| monster.id == owner)
            .map_or(0, |monster| monster.current_hp)
    };

    if source != NO_SOURCE
        && !matches!(damage_type, DamageType::HpLoss | DamageType::Thorns)
        && damage > 0
        && damage < owner_hp
    {
        actions.push(Action::RollMonsterMove { monster_id: owner });
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;

    #[test]
    fn reactive_power_matches_java_on_attacked_filters() {
        let mut mass = crate::test_support::test_monster(EnemyId::WrithingMass);
        mass.id = 7;
        mass.current_hp = 10;
        let state = crate::test_support::combat_with_monsters(vec![mass]);

        assert!(on_attacked(&state, 7, 3, 0, DamageType::Thorns, 1).is_empty());
        assert!(on_attacked(&state, 7, 3, 0, DamageType::HpLoss, 1).is_empty());
        assert!(on_attacked(&state, 7, 3, NO_SOURCE, DamageType::Normal, 1).is_empty());
        assert!(on_attacked(&state, 7, 10, 0, DamageType::Normal, 1).is_empty());

        assert_eq!(
            on_attacked(&state, 7, 3, 0, DamageType::Normal, 1).as_slice(),
            &[Action::RollMonsterMove { monster_id: 7 }],
            "Java ReactivePower only rerolls for real non-lethal non-HP-loss/non-thorns attacks"
        );
    }
}
