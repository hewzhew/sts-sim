use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// TheSpecimen: Whenever an enemy dies, transfer its Poison to a random enemy.
/// Java: onMonsterDeath(m) → if m has Poison, find alive enemy, apply that Poison.
pub fn on_monster_death(
    state: &crate::combat::CombatState,
    dead_monster_id: crate::core::EntityId,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Find dead monster's Poison amount from power_db
    let poison_amount = state
        .power_db
        .get(&dead_monster_id)
        .and_then(|powers| {
            powers
                .iter()
                .find(|p| p.power_type == crate::content::powers::PowerId::Poison)
        })
        .map(|p| p.amount)
        .unwrap_or(0);

    if poison_amount > 0 {
        // Find a random alive enemy to transfer Poison to
        let alive: Vec<_> = state
            .monsters
            .iter()
            .filter(|m| m.id != dead_monster_id && m.current_hp > 0 && !m.is_dying && !m.is_escaped)
            .collect();
        if let Some(target) = alive.first() {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: state.player.id,
                    target: target.id,
                    power_id: crate::content::powers::PowerId::Poison,
                    amount: poison_amount,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    actions
}
