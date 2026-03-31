use crate::combat::CombatState;
use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Meat on the Bone: If your HP is at or below 50% at the end of combat, heal 12 HP.
pub fn on_victory(state: &CombatState, used: bool) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    
    // In Spire, it checks at the end of combat.
    let threshold = state.player.max_hp / 2;
    if state.player.current_hp <= threshold && state.player.current_hp > 0 && !used {
        // Technically Meat on the bone triggers out of combat.
        // Assuming heal acts on current combat state right before it's purged out
        actions.push(ActionInfo {
            action: Action::Heal { target: 0, amount: 12 },
            insertion_mode: AddTo::Top,
        });

        // Trigger flash
    }
    
    actions
}
