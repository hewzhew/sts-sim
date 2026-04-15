use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{QueuedCardPlay, QueuedCardSource};
use smallvec::SmallVec;

/// Necronomicon: The first Attack you play each turn that costs 2 or more is played twice.
/// Java uses an internal `activated` boolean rather than the visible relic counter.
/// We model this with `used_up = false` meaning available this turn.
pub fn at_turn_start() -> SmallVec<[crate::runtime::action::ActionInfo; 4]> {
    smallvec::smallvec![ActionInfo {
        action: Action::UpdateRelicUsedUp {
            relic_id: crate::content::relics::RelicId::Necronomicon,
            used_up: false,
        },
        insertion_mode: AddTo::Bottom,
    }]
}

pub fn on_use_card(
    card_id: crate::content::cards::CardId,
    card_cost_for_turn: i32,
    used_up: bool,
    card: &crate::runtime::combat::CombatCard,
    target: Option<crate::core::EntityId>,
) -> SmallVec<[crate::runtime::action::ActionInfo; 4]> {
    let def = crate::content::cards::get_card_definition(card_id);
    let mut actions = SmallVec::new();

    let meets_cost_threshold = (card_cost_for_turn >= 2 && !card.free_to_play_once)
        || (def.cost == -1 && card.energy_on_use >= 2);

    if !used_up && def.card_type == crate::content::cards::CardType::Attack && meets_cost_threshold
    {
        let mut clone = card.clone();
        clone.energy_on_use = card.energy_on_use;
        actions.push(ActionInfo {
            action: Action::UpdateRelicUsedUp {
                relic_id: crate::content::relics::RelicId::Necronomicon,
                used_up: true,
            },
            insertion_mode: AddTo::Bottom,
        });
        actions.push(ActionInfo {
            action: Action::EnqueueCardPlay {
                item: Box::new(QueuedCardPlay {
                    card: clone.clone(),
                    target,
                    energy_on_use: clone.energy_on_use,
                    ignore_energy_total: true,
                    autoplay: true,
                    random_target: false,
                    is_end_turn_autoplay: false,
                    purge_on_use: true,
                    source: QueuedCardSource::Necronomicon,
                }),
                in_front: true,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
