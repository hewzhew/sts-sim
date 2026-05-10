use crate::state::core::EngineState;
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn on_equip(run_state: &mut RunState) -> Option<EngineState> {
    use crate::content::cards::{get_card_definition, CardId, CardType};
    use crate::content::relics::RelicId;
    use crate::rewards::state::{RewardItem, RewardState};

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
        crate::runtime::rng::shuffle_with_random_long(
            &mut upgradable,
            &mut run_state.rng_pool.misc_rng,
        );
        let uuid = run_state.master_deck[upgradable[0]].uuid;
        run_state.upgrade_card_with_source(uuid, DomainEventSource::Relic(RelicId::TinyHouse));
    }

    run_state.gain_max_hp_with_source(5, 5, DomainEventSource::Relic(RelicId::TinyHouse));

    let potion_class = run_state.potion_class();
    let potion_id = crate::content::potions::random_potion(
        &mut run_state.rng_pool.misc_rng,
        potion_class,
        false,
    );
    let num_cards = crate::rewards::generator::adjusted_card_reward_choice_count(run_state, 3);
    let cards = crate::rewards::generator::generate_card_reward(run_state, num_cards, false);

    let mut reward_state = RewardState::new();
    reward_state.items.push(RewardItem::Gold { amount: 50 });
    reward_state.items.push(RewardItem::Potion { potion_id });
    reward_state.items.push(RewardItem::Card { cards });
    Some(EngineState::RewardScreen(reward_state))
}
