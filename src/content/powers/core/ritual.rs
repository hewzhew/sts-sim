use crate::core::EntityId;
use crate::runtime::action::Action;
use crate::runtime::combat::PowerId;

const PLAYER_CONTROLLED_FLAG: i32 = 1 << 0;
const SKIP_FIRST_FLAG: i32 = 1 << 1;

pub fn extra_data(player_controlled: bool, skip_first: bool) -> i32 {
    let mut extra = 0;
    if player_controlled {
        extra |= PLAYER_CONTROLLED_FLAG;
    }
    if skip_first {
        extra |= SKIP_FIRST_FLAG;
    }
    extra
}

pub fn infer_extra_data(owner: EntityId, just_applied: bool) -> i32 {
    extra_data(owner == 0, owner != 0 && just_applied)
}

pub fn at_end_of_turn(
    owner: EntityId,
    amount: i32,
    extra_data: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    if extra_data & PLAYER_CONTROLLED_FLAG == 0 {
        return smallvec::smallvec![];
    }

    smallvec::smallvec![Action::ApplyPower {
        source: owner,
        target: owner,
        power_id: PowerId::Strength,
        amount,
    }]
}

pub fn at_end_of_round(
    state: &crate::runtime::combat::CombatState,
    owner: EntityId,
    amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];

    let extra = state
        .entities
        .power_db
        .get(&owner)
        .and_then(|ps| ps.iter().find(|p| p.power_type == PowerId::Ritual))
        .map(|p| p.extra_data)
        .unwrap_or(0);

    if extra & PLAYER_CONTROLLED_FLAG != 0 {
        return actions;
    }

    if extra & SKIP_FIRST_FLAG != 0 {
        actions.push(Action::UpdatePowerExtraData {
            target: owner,
            power_id: PowerId::Ritual,
            value: extra & !SKIP_FIRST_FLAG,
        });
    } else {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::Strength,
            amount,
        });
    }

    actions
}
