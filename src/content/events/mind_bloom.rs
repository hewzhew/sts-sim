use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::core::EngineState;
use crate::state::events::{
    EventActionKind, EventCardKind, EventChoiceMeta, EventEffect, EventId, EventOption,
    EventOptionSemantics, EventOptionTransition, EventRelicKind, EventState,
};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => {
            // Java: 3rd option depends on floorNum % 50
            let desire_text = if run_state.floor_num % 50 <= 40 {
                "[Desire] Gain 999 Gold. Obtain 2 Normality."
            } else {
                "[Desire] Heal to full HP. Obtain Doubt."
            };
            vec![
                EventOption::new(
                    EventChoiceMeta::new("[Fight] Battle a boss for rewards."),
                    EventOptionSemantics {
                        action: EventActionKind::Fight,
                        effects: vec![],
                        constraints: vec![],
                        transition: EventOptionTransition::StartCombat,
                        repeatable: false,
                        terminal: false,
                    },
                ),
                EventOption::new(
                    EventChoiceMeta::new("[Remember] Upgrade all cards. Obtain Mark of the Bloom."),
                    EventOptionSemantics {
                        action: EventActionKind::Accept,
                        effects: vec![
                            EventEffect::UpgradeCard { count: usize::MAX },
                            EventEffect::ObtainRelic {
                                count: 1,
                                kind: EventRelicKind::Specific(RelicId::MarkOfTheBloom),
                            },
                        ],
                        constraints: vec![],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ),
                if run_state.floor_num % 50 <= 40 {
                    EventOption::new(
                        EventChoiceMeta::new(desire_text),
                        EventOptionSemantics {
                            action: EventActionKind::Accept,
                            effects: vec![
                                EventEffect::GainGold(999),
                                EventEffect::ObtainCurse {
                                    count: 2,
                                    kind: EventCardKind::Specific(CardId::Normality),
                                },
                            ],
                            constraints: vec![],
                            transition: EventOptionTransition::AdvanceScreen,
                            repeatable: false,
                            terminal: false,
                        },
                    )
                } else {
                    EventOption::new(
                        EventChoiceMeta::new(desire_text),
                        EventOptionSemantics {
                            action: EventActionKind::Accept,
                            effects: vec![
                                EventEffect::Heal((run_state.max_hp - run_state.current_hp).max(0)),
                                EventEffect::ObtainCurse {
                                    count: 1,
                                    kind: EventCardKind::Specific(CardId::Doubt),
                                },
                            ],
                            constraints: vec![],
                            transition: EventOptionTransition::AdvanceScreen,
                            repeatable: false,
                            terminal: false,
                        },
                    )
                },
            ]
        }
        _ => vec![EventOption::new(
            EventChoiceMeta::new("[Leave]"),
            EventOptionSemantics {
                action: EventActionKind::Leave,
                effects: vec![],
                constraints: vec![],
                transition: EventOptionTransition::Complete,
                repeatable: false,
                terminal: true,
            },
        )],
    }
}

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    get_options(run_state, event_state)
        .into_iter()
        .map(|option| option.ui)
        .collect()
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            match choice_idx {
                0 => {
                    // Fight: battle Act 1 boss
                    // Java: shuffle boss list with miscRng.randomLong()
                    let mut boss_indices = [0u8, 1, 2]; // Guardian, Hexaghost, SlimeBoss
                    crate::runtime::rng::shuffle_with_random_long(
                        &mut boss_indices,
                        &mut run_state.rng_pool.misc_rng,
                    );

                    // Java: addGoldToRewards(A13>=13 ? 25 : 50) + addRelicToRewards(RARE)
                    let mut rewards = crate::rewards::state::RewardState::new();
                    let gold = if run_state.ascension_level >= 13 {
                        25
                    } else {
                        50
                    };
                    rewards
                        .items
                        .push(crate::rewards::state::RewardItem::Gold { amount: gold });
                    let rare_relic =
                        run_state.random_screenless_relic(crate::content::relics::RelicTier::Rare);
                    rewards
                        .items
                        .push(crate::rewards::state::RewardItem::Relic {
                            relic_id: rare_relic,
                        });

                    event_state.current_screen = 1;
                    event_state.completed = true;
                    run_state.event_state = Some(event_state);
                    *engine_state =
                        EngineState::EventCombat(crate::state::core::EventCombatState {
                            rewards,
                            reward_allowed: true,
                            no_cards_in_rewards: false,
                            elite_trigger: false,
                            post_combat_return: crate::state::core::PostCombatReturn::MapNavigation,
                            encounter_key: "Mind Bloom Boss",
                        });
                    return;
                }
                1 => {
                    // Remember: upgrade all upgradable cards + MarkOfTheBloom
                    // Java checks canUpgrade() — most cards: upgrades == 0, SearingBlow: always
                    for card in run_state.master_deck.iter_mut() {
                        let def = crate::content::cards::get_card_definition(card.id);
                        let can_upgrade = match def.rarity {
                            crate::content::cards::CardRarity::Curse => false,
                            _ => {
                                // SearingBlow can upgrade infinitely; others only once
                                card.id == crate::content::cards::CardId::SearingBlow
                                    || card.upgrades == 0
                            }
                        };
                        if can_upgrade {
                            card.upgrades += 1;
                        }
                    }
                    if let Some(next_state) = run_state.obtain_relic_with_source(
                        RelicId::MarkOfTheBloom,
                        EngineState::EventRoom,
                        DomainEventSource::Event(EventId::MindBloom),
                    ) {
                        *engine_state = next_state;
                    }
                    event_state.current_screen = 1;
                }
                _ => {
                    // Desire: depends on floorNum % 50
                    if run_state.floor_num % 50 <= 40 {
                        // Normal path: 999 gold + 2 Normality
                        run_state.change_gold_with_source(
                            999,
                            DomainEventSource::Event(EventId::MindBloom),
                        );
                        super::obtain_event_card(run_state, EventId::MindBloom, CardId::Normality);
                        super::obtain_event_card(run_state, EventId::MindBloom, CardId::Normality);
                    } else {
                        // High floor path: heal to full + Doubt curse
                        run_state.heal_with_source(
                            run_state.max_hp,
                            DomainEventSource::Event(EventId::MindBloom),
                        );
                        super::obtain_event_card(run_state, EventId::MindBloom, CardId::Doubt);
                    }
                    event_state.current_screen = 1;
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
    use crate::content::relics::RelicState;
    use crate::state::events::{EventOptionTransition, EventRelicKind};
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn remember_option_exposes_mark_of_the_bloom_semantics() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.floor_num = 20;
        let state = EventState::new(crate::state::events::EventId::MindBloom);
        let options = get_options(&rs, &state);
        assert!(options[1]
            .semantics
            .effects
            .contains(&EventEffect::ObtainRelic {
                count: 1,
                kind: EventRelicKind::Specific(RelicId::MarkOfTheBloom),
            }));
        assert_eq!(
            options[0].semantics.transition,
            EventOptionTransition::StartCombat
        );
    }

    #[test]
    fn remember_obtains_mark_of_the_bloom_with_event_source() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.floor_num = 20;
        rs.event_state = Some(EventState::new(EventId::MindBloom));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 1);

        assert!(rs
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::MarkOfTheBloom));
        let events = rs.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::RelicObtained {
                relic_id: RelicId::MarkOfTheBloom,
                source: DomainEventSource::Event(EventId::MindBloom),
            }
        )));
    }

    #[test]
    fn high_floor_desire_heals_with_event_source_and_obtains_doubt() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.floor_num = 41;
        rs.current_hp = 10;
        rs.max_hp = 80;
        rs.event_state = Some(EventState::new(EventId::MindBloom));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 2);

        assert_eq!(rs.current_hp, 80);
        assert!(rs.master_deck.iter().any(|card| card.id == CardId::Doubt));
        let events = rs.take_emitted_events();
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::HpChanged {
                delta: 70,
                current_hp: 80,
                max_hp: 80,
                source: DomainEventSource::Event(EventId::MindBloom),
            }
        )));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                source: DomainEventSource::Event(EventId::MindBloom),
                card,
            } if card.id == CardId::Doubt
        )));
    }

    #[test]
    fn high_floor_desire_heal_respects_mark_of_the_bloom() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.floor_num = 41;
        rs.current_hp = 10;
        rs.max_hp = 80;
        rs.relics.push(RelicState::new(RelicId::MarkOfTheBloom));
        rs.event_state = Some(EventState::new(EventId::MindBloom));
        rs.emitted_events.clear();
        let mut engine_state = EngineState::EventRoom;

        handle_choice(&mut engine_state, &mut rs, 2);

        assert_eq!(rs.current_hp, 10);
        assert!(rs.master_deck.iter().any(|card| card.id == CardId::Doubt));
        let events = rs.take_emitted_events();
        assert!(!events
            .iter()
            .any(|event| matches!(event, DomainEvent::HpChanged { .. })));
        assert!(events.iter().any(|event| matches!(
            event,
            DomainEvent::CardObtained {
                source: DomainEventSource::Event(EventId::MindBloom),
                card,
            } if card.id == CardId::Doubt
        )));
    }
}
