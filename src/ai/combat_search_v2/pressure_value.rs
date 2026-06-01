use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CombatPressureValueV1 {
    pub(super) visible_incoming_damage: i32,
    pub(super) survival_margin: i32,
}

pub(super) fn combat_pressure_value(combat: &CombatState) -> CombatPressureValueV1 {
    let incoming = visible_incoming_damage(combat);
    CombatPressureValueV1 {
        visible_incoming_damage: incoming,
        survival_margin: combat.entities.player.current_hp + combat.entities.player.block
            - incoming,
    }
}

pub(super) fn visible_incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster_preview_total_damage_in_combat(combat, monster))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::blank_test_combat;

    #[test]
    fn pressure_value_uses_hp_plus_block_when_no_visible_incoming_damage() {
        let mut combat = blank_test_combat();
        combat.entities.player.current_hp = 13;
        combat.entities.player.block = 7;
        combat.entities.monsters.clear();

        let value = combat_pressure_value(&combat);

        assert_eq!(value.visible_incoming_damage, 0);
        assert_eq!(value.survival_margin, 20);
    }
}
