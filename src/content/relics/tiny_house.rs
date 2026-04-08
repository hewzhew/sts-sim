use crate::state::core::EngineState;
use crate::state::run::RunState;

pub fn on_equip(run_state: &mut RunState) -> Option<EngineState> {
    use crate::content::cards::{get_card_definition, CardId, CardType};
    run_state.max_hp += 5;
    run_state.current_hp = (run_state.current_hp + 5).min(run_state.max_hp);
    let mut upgradable: Vec<usize> = run_state
        .master_deck
        .iter()
        .enumerate()
        .filter(|(_, c)| {
            let def = get_card_definition(c.id);
            def.card_type != CardType::Curse && (c.id == CardId::SearingBlow || c.upgrades == 0)
        })
        .map(|(i, _)| i)
        .collect();
    if !upgradable.is_empty() {
        crate::rng::shuffle_with_random_long(&mut upgradable, &mut run_state.rng_pool.misc_rng);
        run_state.master_deck[upgradable[0]].upgrades += 1;
    }
    run_state.gold += 50;
    None
}
