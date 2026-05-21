use super::*;

pub(super) fn terminal_rank(label: SearchTerminalLabel) -> i32 {
    match label {
        SearchTerminalLabel::Win => 2,
        SearchTerminalLabel::Unresolved => 1,
        SearchTerminalLabel::Loss => 0,
    }
}

pub(super) fn total_living_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0))
        .sum()
}

pub(super) fn living_enemy_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .count()
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

pub(super) fn survival_margin(combat: &CombatState) -> i32 {
    combat.entities.player.current_hp + combat.entities.player.block
        - visible_incoming_damage(combat)
}
