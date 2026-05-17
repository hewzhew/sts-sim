use crate::rewards::state::{RewardItem, RewardState};
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            if run_state.ascension_level >= 15 {
                vec![EventChoiceMeta::new("[Focus 1] Obtain 1 colorless card.")]
            } else {
                vec![
                    EventChoiceMeta::new("[Focus 1] Obtain 1 colorless card."),
                    EventChoiceMeta::new("[Focus 2] Obtain 2 colorless cards. Take 5 damage."),
                    EventChoiceMeta::new("[Focus 3] Obtain 3 colorless cards. Take 10 damage."),
                ]
            }
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            let count = if run_state.ascension_level >= 15 {
                1
            } else {
                (choice_idx + 1).min(3)
            };

            // Java: Collections.shuffle(memories, new Random(miscRng.randomLong()))
            // Consume randomLong for seed parity
            let _seed = run_state.rng_pool.misc_rng.random_long();

            // HP_LOSS damage for Focus 2/3. Java bypasses block, but
            // AbstractPlayer.damage still applies relic onLoseHpLast.
            match choice_idx {
                1 => {
                    super::apply_player_hp_loss_damage(
                        run_state,
                        5,
                        DomainEventSource::Event(EventId::SensoryStone),
                    );
                }
                2 => {
                    super::apply_player_hp_loss_damage(
                        run_state,
                        10,
                        DomainEventSource::Event(EventId::SensoryStone),
                    );
                }
                _ => {}
            }

            // Java: addCardReward(RewardItem(COLORLESS)) × count
            // Each card reward row offers 3 colorless cards (pick 1, skippable)
            let mut rewards = RewardState::new();
            for _ in 0..count {
                let cards = generate_colorless_card_row(run_state);
                rewards.items.push(RewardItem::Card { cards });
            }

            event_state.current_screen = 1;
            run_state.event_state = Some(event_state);
            *engine_state = EngineState::RewardScreen(rewards);
            return;
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

/// Generate a row of 3 colorless uncommon cards for the card reward screen.
/// Mirrors Java: RewardItem(CardColor.COLORLESS) generates 3 colorless cards.
/// Uses card_rng for consistency with card reward generation.
fn generate_colorless_card_row(run_state: &mut RunState) -> Vec<crate::rewards::state::RewardCard> {
    use crate::content::cards::*;
    let pool = COLORLESS_UNCOMMON_POOL;
    let mut cards = Vec::with_capacity(3);
    for _ in 0..3 {
        let idx = run_state
            .rng_pool
            .card_rng
            .random_range(0, pool.len() as i32 - 1) as usize;
        let card_id = pool[idx];
        // Avoid duplicates within the same row
        if !cards
            .iter()
            .any(|c: &crate::rewards::state::RewardCard| c.id == card_id)
        {
            cards.push(crate::rewards::state::RewardCard::new(
                card_id,
                run_state.preview_obtain_card_upgrades(card_id, 0),
            ));
        } else {
            // If duplicate, try again with next random
            let idx2 = run_state
                .rng_pool
                .card_rng
                .random_range(0, pool.len() as i32 - 1) as usize;
            let fallback_id = pool[idx2];
            cards.push(crate::rewards::state::RewardCard::new(
                fallback_id,
                run_state.preview_obtain_card_upgrades(fallback_id, 0),
            ));
        }
    }
    cards
}

#[cfg(test)]
mod tests {
    use super::handle_choice;
    use crate::content::relics::{RelicId, RelicState};
    use crate::rewards::state::RewardItem;
    use crate::state::core::EngineState;
    use crate::state::events::{EventId, EventState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    fn sensory_run(current_hp: i32, max_hp: i32) -> RunState {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = current_hp;
        run_state.max_hp = max_hp;
        run_state.event_state = Some(EventState::new(EventId::SensoryStone));
        run_state.emitted_events.clear();
        run_state
    }

    #[test]
    fn focus_two_hp_loss_uses_event_source_and_opens_two_rewards() {
        let mut run_state = sensory_run(20, 80);
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.current_hp, 15);
        match engine_state {
            EngineState::RewardScreen(rewards) => {
                assert_eq!(rewards.items.len(), 2);
                assert!(rewards
                    .items
                    .iter()
                    .all(|item| matches!(item, RewardItem::Card { .. })));
            }
            other => panic!("expected reward screen, got {other:?}"),
        }
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -5,
                current_hp: 15,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::SensoryStone),
            }
        )));
    }

    #[test]
    fn focus_three_hp_loss_applies_tungsten_rod_on_lose_hp_last() {
        let mut run_state = sensory_run(20, 80);
        run_state.relics.push(RelicState::new(RelicId::TungstenRod));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 2);

        assert_eq!(run_state.current_hp, 11);
        match engine_state {
            EngineState::RewardScreen(rewards) => {
                assert_eq!(rewards.items.len(), 3);
            }
            other => panic!("expected reward screen, got {other:?}"),
        }
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: -9,
                current_hp: 11,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::SensoryStone),
            }
        )));
    }
}
