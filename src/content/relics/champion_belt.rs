use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub struct ChampionBelt;

impl ChampionBelt {
    pub fn on_apply_power(
        state: &CombatState,
        source: EntityId,
        target: EntityId,
        power_id: PowerId,
    ) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        let target_has_artifact = crate::content::powers::store::powers_for(state, target)
            .is_some_and(|powers| powers.iter().any(|p| p.power_type == PowerId::Artifact));

        if source == 0
            && target != source
            && power_id == PowerId::Vulnerable
            && !target_has_artifact
        {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: 0,
                    target,
                    power_id: PowerId::Weak,
                    amount: 1,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
        actions
    }
}
