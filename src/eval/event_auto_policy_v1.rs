use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::state::run::RunState;

pub(crate) fn note_for_yourself_default_note_is_ignorable(run_state: &RunState) -> bool {
    let note_card = run_state.note_for_yourself_card;
    let def = get_card_definition(note_card);
    if matches!(def.card_type, CardType::Curse | CardType::Status) {
        return true;
    }

    note_card == CardId::IronWave && run_state.note_for_yourself_upgrades == 0
}
