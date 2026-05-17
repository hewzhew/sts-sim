use crate::content::cards::CardId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let curse_chance = if run_state.ascension_level >= 15 {
                100
            } else {
                50
            };
            vec![
                EventChoiceMeta::new(format!(
                    "[Open] {}% chance of Writhe. Obtain a random Relic.",
                    curse_chance
                )),
                EventChoiceMeta::new("[Leave]"),
            ]
        }
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Open: relic + possible Writhe curse
                    // Java: always calls miscRng.randomBoolean(), then overrides at A15
                    let mut gets_curse = run_state.rng_pool.misc_rng.random_boolean();
                    if run_state.ascension_level >= 15 {
                        gets_curse = true;
                    }
                    let relic_id = run_state.random_screenless_relic_reward();
                    if let Some(next_state) = run_state.obtain_relic_with_source(
                        relic_id,
                        EngineState::EventRoom,
                        DomainEventSource::Event(EventId::Mausoleum),
                    ) {
                        *engine_state = next_state;
                    }
                    if gets_curse {
                        super::obtain_event_card(run_state, EventId::Mausoleum, CardId::Writhe);
                    }
                    event_state.current_screen = 1;
                }
                _ => {
                    event_state.completed = true;
                }
            }
        }
        _ => {
            event_state.completed = true;
        }
    }

    run_state.event_state = Some(event_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::relics::{RelicId, RelicState};
    use crate::state::selection::DomainEvent;

    fn mausoleum_run() -> RunState {
        let mut run_state = RunState::new(1, 15, true, "Ironclad");
        run_state.current_hp = 50;
        run_state.max_hp = 80;
        run_state.common_relic_pool = vec![RelicId::DarkstonePeriapt];
        run_state.uncommon_relic_pool = vec![RelicId::DarkstonePeriapt];
        run_state.rare_relic_pool = vec![RelicId::DarkstonePeriapt];
        run_state.event_state = Some(EventState {
            id: EventId::Mausoleum,
            current_screen: 0,
            internal_state: 0,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        });
        run_state
    }

    #[test]
    fn cursed_open_obtains_relic_before_writhe_effect_resolves_like_java() {
        let mut run_state = mausoleum_run();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::DarkstonePeriapt));
        assert!(run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::Writhe));
        assert_eq!(run_state.max_hp, 86);
        assert_eq!(run_state.current_hp, 56);
        assert!(run_state.take_emitted_events().iter().any(|event| matches!(
            event,
            DomainEvent::MaxHpChanged {
                delta: 6,
                source: DomainEventSource::Event(EventId::Mausoleum),
                ..
            }
        )));
    }

    #[test]
    fn cursed_open_still_rolls_misc_rng_before_a15_forces_curse() {
        let mut run_state = mausoleum_run();
        let before_counter = run_state.rng_pool.misc_rng.counter;
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert_eq!(run_state.rng_pool.misc_rng.counter, before_counter + 1);
    }

    #[test]
    fn omamori_blocks_writhe_after_relic_obtain_so_darkstone_does_not_trigger() {
        let mut run_state = mausoleum_run();
        run_state.relics.push(RelicState::new(RelicId::Omamori));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::DarkstonePeriapt));
        assert!(!run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::Writhe));
        assert_eq!(run_state.max_hp, 80);
        assert_eq!(run_state.current_hp, 50);
        let omamori = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::Omamori)
            .expect("Omamori should remain after blocking Writhe");
        assert_eq!(omamori.counter, 1);
    }
}
