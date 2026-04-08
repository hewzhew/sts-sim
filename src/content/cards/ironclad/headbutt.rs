use crate::action::{Action, ActionInfo, AddTo, DamageInfo, DamageType};
use crate::combat::{CombatCard, CombatState};
use crate::core::EntityId;
use crate::state::{GridSelectReason, PileType};
use smallvec::SmallVec;

pub fn headbutt_play(
    state: &CombatState,
    card: &CombatCard,
    target: Option<EntityId>,
) -> SmallVec<[ActionInfo; 4]> {
    let target = target.expect("Headbutt requires a valid target!");
    let mut actions = SmallVec::new();

    actions.push(ActionInfo {
        action: Action::Damage(DamageInfo {
            source: 0,
            target,
            base: card.base_damage_mut,
            output: card.base_damage_mut,
            damage_type: DamageType::Normal,
            is_modified: false,
        }),
        insertion_mode: AddTo::Bottom,
    });

    let discard_size = state.discard_pile.len();
    if discard_size > 1 {
        actions.push(ActionInfo {
            action: Action::SuspendForGridSelect {
                source_pile: PileType::Discard,
                min: 1,
                max: 1,
                can_cancel: false,
                filter: crate::state::GridSelectFilter::Any,
                reason: GridSelectReason::MoveToDrawPile,
            },
            insertion_mode: AddTo::Bottom,
        });
    } else if discard_size == 1 {
        // Just directly move the 1 card
        actions.push(ActionInfo {
            action: Action::MoveCard {
                card_uuid: state.discard_pile[0].uuid,
                from: PileType::Discard,
                to: PileType::Draw,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
