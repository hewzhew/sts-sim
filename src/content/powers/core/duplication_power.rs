use crate::action::Action;
use crate::combat::{CombatCard, CombatState, QueuedCardPlay, QueuedCardSource};
use crate::content::powers::store;
use crate::content::powers::PowerId;

/// DuplicationPower: from Duplication Potion.
/// Same as DoubleTap but copies ANY card, not just Attacks.
/// Java: DuplicationPower.onUseCard checks !card.purgeOnUse && amount > 0 (no type check).
pub fn on_use_card(state: &mut CombatState, card: &CombatCard, purge: bool, target: Option<usize>) {
    if !purge {
        let mut clone = card.clone();

        // Allocate a new UUID for the cloned card
        state.zones.card_uuid_counter += 1;
        clone.uuid = state.zones.card_uuid_counter;

        // Deduct power amount
        if let Some(amount) = store::with_power_mut(state, 0, PowerId::DuplicationPower, |p| {
            p.amount -= 1;
            p.amount
        }) {
            if !crate::content::powers::should_keep_power_instance(
                PowerId::DuplicationPower,
                amount,
            ) {
                store::remove_power_type(state, 0, PowerId::DuplicationPower);
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
                    source: QueuedCardSource::Duplication,
                }),
                in_front: true,
            });
    }
}
