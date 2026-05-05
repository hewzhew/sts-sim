use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatState, PlayerEntity};
use smallvec::SmallVec;

/// Gold-Plated Cables: Your rightmost Orb's passive triggers an additional time.
/// The trigger logic itself occurs inside `engine::resolve_action` -> `Action::TriggerPassiveOrbs`.
/// We can either hardcode the double loop inside `TriggerPassiveOrbs` by checking `has_relic`,
/// or use an `at_end_of_turn` hook to trigger the rightmost orb manually.
/// Let's use the explicit `at_end_of_turn` hook to append an extra trigger.

pub fn at_end_of_turn(_state: &CombatState, player: &PlayerEntity) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Rightmost orb is index 0 in our logic (it's the next to be evoked).
    if !player.orbs.is_empty() && player.orbs[0].id != crate::runtime::combat::OrbId::Empty {
        let orb = &player.orbs[0];
        match orb.id {
            crate::runtime::combat::OrbId::Lightning => {
                actions.push(ActionInfo {
                    action: Action::AttackDamageRandomEnemy {
                        base_damage: orb.passive_amount,
                        damage_type: crate::runtime::action::DamageType::Thorns,
                        applies_target_modifiers: false,
                    },
                    insertion_mode: AddTo::Bottom,
                });
            }
            crate::runtime::combat::OrbId::Dark => {
                // Dark Orb doesn't "trigger" actively, it just grows.
                // We'll queue a custom power modification to increase it again.
                // However, `TriggerPassiveOrbs` handles Dark manually.
                // To keep it simple, we use a specialized Action or inline mutation...
                // But passive hook runs AFTER triggers, so modifying it dynamically is tricky.
                // Let's rely on `Action::TriggerPassiveOrbs` directly checking `GoldenCables`
                // in the engine to double its growth for safety.
            }
            crate::runtime::combat::OrbId::Frost => {
                actions.push(ActionInfo {
                    action: Action::GainBlock {
                        target: player.id,
                        amount: orb.passive_amount,
                    },
                    insertion_mode: AddTo::Bottom,
                });
            }
            crate::runtime::combat::OrbId::Plasma => {
                actions.push(ActionInfo {
                    action: Action::GainEnergy {
                        amount: orb.passive_amount,
                    },
                    insertion_mode: AddTo::Bottom,
                });
            }
            crate::runtime::combat::OrbId::Empty => {}
        }
    }
    actions
}
