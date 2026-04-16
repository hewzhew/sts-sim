use crate::runtime::action::Action;
use crate::runtime::combat::{CombatCard, CombatState, QueuedCardPlay, QueuedCardSource};
use crate::content::powers::store;
use crate::content::powers::PowerId;

pub fn on_use_card(state: &mut CombatState, card: &CombatCard, purge: bool, target: Option<usize>) {
    if purge
        || crate::content::cards::get_card_definition(card.id).card_type
            != crate::content::cards::CardType::Skill
    {
        return;
    }

    let mut clone = card.clone();
    state.zones.card_uuid_counter += 1;
    clone.uuid = state.zones.card_uuid_counter;

    if let Some(amount) = store::with_power_mut(state, 0, PowerId::Burst, |p| {
        p.amount -= 1;
        p.amount
    }) {
        if !crate::content::powers::should_keep_power_instance(PowerId::Burst, amount) {
            store::remove_power_type(state, 0, PowerId::Burst);
        }
    }

    state.queue_action_back(Action::EnqueueCardPlay {
        item: Box::new(QueuedCardPlay {
            card: clone.clone(),
            target,
            energy_on_use: clone.energy_on_use,
            ignore_energy_total: true,
            autoplay: true,
            random_target: false,
            is_end_turn_autoplay: false,
            purge_on_use: true,
            source: QueuedCardSource::Burst,
        }),
        in_front: true,
    });
}
