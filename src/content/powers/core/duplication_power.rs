use crate::combat::{CombatCard, CombatState};
use crate::content::powers::PowerId;

/// DuplicationPower: from Duplication Potion.
/// Same as DoubleTap but copies ANY card, not just Attacks.
/// Java: DuplicationPower.onUseCard checks !card.purgeOnUse && amount > 0 (no type check).
pub fn on_use_card(state: &mut CombatState, card: &CombatCard, purge: bool, target: Option<usize>) {
    if !purge {
        let mut clone = card.clone();

        // Allocate a new UUID for the cloned card
        state.card_uuid_counter += 1;
        clone.uuid = state.card_uuid_counter;

        // Deduct power amount
        if let Some(powers) = state.power_db.get_mut(&0) {
            for p in powers.iter_mut() {
                if p.power_type == PowerId::DuplicationPower {
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
                target, // Propagate original target
                purge: true,
            });
    }
}
