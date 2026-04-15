use crate::action::Action;
use crate::content::powers::PowerId;
use crate::core::EntityId;

pub fn at_turn_start(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    if amount <= 0 {
        return smallvec::smallvec![Action::RemovePower {
            target: owner,
            power_id: PowerId::Poison,
        }];
    }

    smallvec::smallvec![
        Action::LoseHp {
            target: owner,
            amount,
            triggers_rupture: false,
        },
        Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Poison,
            amount: -1,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::at_turn_start;
    use crate::action::Action;
    use crate::content::powers::PowerId;

    #[test]
    fn poison_hp_loss_does_not_opt_into_rupture() {
        let actions = at_turn_start(0, 5);
        assert_eq!(actions.len(), 2);
        assert!(matches!(
            actions[0],
            Action::LoseHp {
                target: 0,
                amount: 5,
                triggers_rupture: false,
            }
        ));
        assert!(matches!(
            actions[1],
            Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::Poison,
                amount: -1,
            }
        ));
    }
}
