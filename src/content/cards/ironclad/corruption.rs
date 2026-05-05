use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn corruption_play(state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // In Java, the card checks `if (!powerExists)` directly in `use()`.
    if state.get_power(0, crate::content::powers::PowerId::Corruption) == 0 {
        actions.push(ActionInfo {
            action: Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: crate::content::powers::PowerId::Corruption,
                amount: -1,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}

/// Mimics `ApplyPowerAction.java` line 43: Immediately reduces all skill costs in all piles
pub fn corruption_on_apply(state: &mut CombatState) {
    let is_skill = |id| {
        crate::content::cards::get_card_definition(id).card_type
            == crate::content::cards::CardType::Skill
    };
    for c in state
        .zones
        .hand
        .iter_mut()
        .chain(state.zones.draw_pile.iter_mut())
        .chain(state.zones.discard_pile.iter_mut())
        .chain(state.zones.exhaust_pile.iter_mut())
        .chain(state.zones.limbo.iter_mut())
    {
        if is_skill(c.id) {
            c.cost_modifier -= 9;
            c.cost_for_turn = Some(0);
        }
    }
}

/// Mimics `CorruptionPower.onCardDraw(AbstractCard)`: Reduces cost for turn for newly drawn skills
/// (E.g. effectively overriding Snecko Eye's previous draw cost adjustments).
pub fn corruption_on_card_draw(_state: &CombatState, card: &mut CombatCard) {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == crate::content::cards::CardType::Skill {
        card.cost_for_turn = Some(0);
    }
}

/// Mimics `CorruptionPower.onUseCard(AbstractCard, UseCardAction)`: Forces skills to exhaust.
pub fn corruption_on_use_card(state: &CombatState, card: &CombatCard, exhaust_override: &mut bool) {
    // Only active when the player actually has the Corruption power
    let has_corruption = state.entities.power_db.get(&0).map_or(false, |powers| {
        powers
            .iter()
            .any(|p| p.power_type == crate::content::powers::PowerId::Corruption)
    });
    if !has_corruption {
        return;
    }
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == crate::content::cards::CardType::Skill {
        *exhaust_override = true;
    }
}
