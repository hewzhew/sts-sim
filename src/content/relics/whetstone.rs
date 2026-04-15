use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState) -> Option<EngineState> {
    use crate::content::cards::{get_card_definition, CardId, CardType};
    let mut upgradable_indices: Vec<usize> = run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            let def = get_card_definition(c.id);
            def.card_type == CardType::Attack && (c.id == CardId::SearingBlow || c.upgrades == 0)
        })
        .map(|(i, _)| i)
        .collect();

    if !upgradable_indices.is_empty() {
        crate::runtime::rng::shuffle_with_random_long(
            &mut upgradable_indices,
            &mut run_state.rng_pool.misc_rng,
        );
        for i in 0..2.min(upgradable_indices.len()) {
            run_state.master_deck[upgradable_indices[i]].upgrades += 1;
        }
    }
    None
}
