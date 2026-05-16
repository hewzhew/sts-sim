use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{EventChoiceMeta, EventId, EventState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => {
            let heal_amt = (run_state.max_hp as f32 * 0.25) as i32;
            vec![
                EventChoiceMeta::new("[Stomp] Fight the mushrooms!"),
                EventChoiceMeta::new(format!(
                    "[Eat] Heal {} HP. Become Cursed - Parasite.",
                    heal_amt
                )),
            ]
        }
        1 => {
            // Post-heal leave screen.
            vec![EventChoiceMeta::new("[Leave]")]
        }
        2 => vec![EventChoiceMeta::new("[Fight]")],
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Java first click only reveals the fight text and confirms the
                    // encounter. Rewards are generated on the next click.
                    event_state.current_screen = 2;
                }
                _ => {
                    // Eat: Heal 25% maxHP + Parasite curse.
                    // Java uses AbstractPlayer.heal and ShowCardAndObtainEffect.
                    let heal_amt = (run_state.max_hp as f32 * 0.25) as i32;
                    run_state
                        .heal_with_source(heal_amt, DomainEventSource::Event(EventId::Mushrooms));
                    run_state.add_card_to_deck_with_upgrades_from(
                        CardId::Parasite,
                        0,
                        DomainEventSource::Event(EventId::Mushrooms),
                    );
                    event_state.current_screen = 1;
                }
            }
        }
        2 => {
            // Fight the mushrooms. Java generates rewards immediately before
            // enterCombat().
            let gold = run_state.rng_pool.misc_rng.random_range(20, 30);
            let mut rewards = crate::rewards::state::RewardState::new();
            rewards
                .items
                .push(crate::rewards::state::RewardItem::Gold { amount: gold });

            if run_state
                .relics
                .iter()
                .any(|r| r.id == RelicId::OddMushroom)
            {
                rewards
                    .items
                    .push(crate::rewards::state::RewardItem::Relic {
                        relic_id: RelicId::Circlet,
                    });
            } else {
                rewards
                    .items
                    .push(crate::rewards::state::RewardItem::Relic {
                        relic_id: RelicId::OddMushroom,
                    });
            }

            event_state.completed = true;
            run_state.event_state = Some(event_state);

            *engine_state = EngineState::EventCombat(crate::state::core::EventCombatState {
                rewards,
                reward_allowed: true,
                no_cards_in_rewards: false,
                post_combat_return: crate::state::core::PostCombatReturn::MapNavigation,
                encounter_key: "The Mushroom Lair",
            });
            return;
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
    use crate::content::relics::RelicState;
    use crate::state::selection::DomainEvent;

    #[test]
    fn fight_path_requires_java_confirm_screen_before_combat() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.event_state = Some(EventState::new(EventId::Mushrooms));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 0);

        assert!(matches!(engine_state, EngineState::EventRoom));
        let event_state = run_state.event_state.as_ref().unwrap();
        assert_eq!(event_state.current_screen, 2);
        assert!(!event_state.completed);

        handle_choice(&mut engine_state, &mut run_state, 0);

        let EngineState::EventCombat(combat) = engine_state else {
            panic!("confirmed Mushrooms fight should enter EventCombat");
        };
        assert_eq!(combat.encounter_key, "The Mushroom Lair");
        assert!(combat.reward_allowed);
        assert!(combat
            .rewards
            .items
            .iter()
            .any(|item| matches!(item, crate::rewards::state::RewardItem::Gold { amount } if (20..=30).contains(amount))));
        assert!(combat.rewards.items.iter().any(|item| matches!(
            item,
            crate::rewards::state::RewardItem::Relic {
                relic_id: RelicId::OddMushroom
            }
        )));
    }

    #[test]
    fn eat_uses_player_heal_and_show_card_obtain_semantics() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = 20;
        run_state.max_hp = 80;
        run_state.event_state = Some(EventState::new(EventId::Mushrooms));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.current_hp, 40);
        assert_eq!(run_state.master_deck.last().unwrap().id, CardId::Parasite);
        let events = run_state.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: 20,
                source: DomainEventSource::Event(EventId::Mushrooms),
                ..
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                card,
                source: DomainEventSource::Event(EventId::Mushrooms),
            } if card.id == CardId::Parasite
        )));
    }

    #[test]
    fn eat_heal_is_blocked_by_mark_of_the_bloom_but_curse_obtain_still_runs() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.current_hp = 20;
        run_state.max_hp = 80;
        run_state
            .relics
            .push(RelicState::new(RelicId::MarkOfTheBloom));
        run_state.event_state = Some(EventState::new(EventId::Mushrooms));
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut run_state, 1);

        assert_eq!(run_state.current_hp, 20);
        assert_eq!(run_state.master_deck.last().unwrap().id, CardId::Parasite);
    }
}
