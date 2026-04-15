use crate::action::Action;
use crate::combat::{CombatCard, CombatState, QueuedCardPlay, QueuedCardSource};
use crate::content::powers::store;
use crate::content::powers::PowerId;

pub fn on_use_card(state: &mut CombatState, card: &CombatCard, purge: bool, target: Option<usize>) {
    if !purge
        && crate::content::cards::get_card_definition(card.id).card_type
            == crate::content::cards::CardType::Attack
    {
        let clone = card.clone();

        // Deduct power
        if let Some(amount) = store::with_power_mut(state, 0, PowerId::DoubleTap, |p| {
            p.amount -= 1;
            p.amount
        }) {
            if !crate::content::powers::should_keep_power_instance(PowerId::DoubleTap, amount) {
                store::remove_power_type(state, 0, PowerId::DoubleTap);
            }
        }

        state
            .engine
            .action_queue
            .push_back(Action::EnqueueCardPlay {
                item: Box::new(QueuedCardPlay {
                    card: clone.clone(),
                    target,
                    energy_on_use: clone.energy_on_use,
                    ignore_energy_total: true,
                    autoplay: true,
                    random_target: false,
                    is_end_turn_autoplay: false,
                    purge_on_use: true,
                    source: QueuedCardSource::DoubleTap,
                }),
                in_front: true,
            });
    }
}
