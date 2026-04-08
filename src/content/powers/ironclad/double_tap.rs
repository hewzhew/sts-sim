use crate::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;

pub fn on_use_card(state: &mut CombatState, card: &CombatCard, purge: bool, target: Option<usize>) {
    if !purge
        && crate::content::cards::get_card_definition(card.id).card_type
            == crate::content::cards::CardType::Attack
    {
        let mut clone = card.clone();

        // Correctly allocate a new UUID for the Temp card clone!
        state.card_uuid_counter += 1;
        clone.uuid = state.card_uuid_counter;

        // Deduct power
        if let Some(powers) = state.power_db.get_mut(&0) {
            // 0 is player EntityId
            for p in powers.iter_mut() {
                if p.power_type == PowerId::DoubleTap {
                    p.amount -= 1;
                    break;
                }
            }
            powers.retain(|p| p.amount > 0);
        }

        state
            .action_queue
            .push_back(crate::action::Action::PlayCardDirect {
                card: Box::new(clone),
                target,      // Propagate original target
                purge: true, // The copied card naturally purges after playing.
            });
    }
}
