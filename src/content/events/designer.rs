use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{
    EventActionKind, EventCardKind, EventChoiceMeta, EventEffect, EventOption,
    EventOptionConstraint, EventOptionSemantics, EventOptionTransition, EventSelectionKind,
    EventState,
};
use crate::state::run::RunState;

// Java Designer: randomizes upgrade-one vs upgrade-two-random, and remove-one vs transform-two
// internal_state encodes: bit0 = adjustmentUpgradesOne, bit1 = cleanUpRemovesCards
// Costs: A15: 50/75/110/5hp, else: 40/60/90/3hp

fn adjust_cost(asc: u8) -> i32 {
    if asc >= 15 {
        50
    } else {
        40
    }
}
fn cleanup_cost(asc: u8) -> i32 {
    if asc >= 15 {
        75
    } else {
        60
    }
}
fn full_service_cost(asc: u8) -> i32 {
    if asc >= 15 {
        110
    } else {
        90
    }
}
fn hp_loss(asc: u8) -> i32 {
    if asc >= 15 {
        5
    } else {
        3
    }
}

fn upgrades_one(state: i32) -> bool {
    state & 1 != 0
}
fn removes_cards(state: i32) -> bool {
    state & 2 != 0
}

pub fn get_options(run_state: &RunState, event_state: &EventState) -> Vec<EventOption> {
    match event_state.current_screen {
        0 => vec![EventOption::new(
            EventChoiceMeta::new("[Proceed]"),
            EventOptionSemantics {
                action: EventActionKind::Continue,
                effects: vec![],
                constraints: vec![],
                transition: EventOptionTransition::AdvanceScreen,
                repeatable: false,
                terminal: false,
            },
        )],
        1 => {
            let asc = run_state.ascension_level;
            let upgradable_count = run_state
                .master_deck
                .iter()
                .filter(|c| {
                    let def = crate::content::cards::get_card_definition(c.id);
                    def.card_type != crate::content::cards::CardType::Curse
                        && def.card_type != crate::content::cards::CardType::Status
                })
                .count();
            let has_upgradable = upgradable_count > 0;

            let adj_label = if upgrades_one(event_state.internal_state) {
                format!("[Adjust] {} Gold. Upgrade 1 card.", adjust_cost(asc))
            } else {
                format!(
                    "[Adjust] {} Gold. Upgrade 2 random cards.",
                    adjust_cost(asc)
                )
            };
            let adj_disabled = run_state.gold < adjust_cost(asc) || !has_upgradable;

            let clean_label = if removes_cards(event_state.internal_state) {
                format!("[Clean Up] {} Gold. Remove 1 card.", cleanup_cost(asc))
            } else {
                format!("[Clean Up] {} Gold. Transform 2 cards.", cleanup_cost(asc))
            };
            let clean_disabled = run_state.gold < cleanup_cost(asc);

            let full_label = format!(
                "[Full Service] {} Gold. Remove 1 card + upgrade 1 random.",
                full_service_cost(asc)
            );
            let full_disabled = run_state.gold < full_service_cost(asc);

            let punch_label = format!("[Punch] Lose {} HP.", hp_loss(asc));

            vec![
                if adj_disabled {
                    EventOption::new(
                        EventChoiceMeta::disabled(adj_label, "Not enough Gold/cards"),
                        EventOptionSemantics {
                            action: EventActionKind::DeckOperation,
                            effects: vec![EventEffect::LoseGold(adjust_cost(asc) as i32)],
                            constraints: vec![
                                EventOptionConstraint::RequiresGold(adjust_cost(asc)),
                                EventOptionConstraint::RequiresUpgradeableCard,
                            ],
                            transition: if upgrades_one(event_state.internal_state) {
                                EventOptionTransition::OpenSelection(
                                    EventSelectionKind::UpgradeCard,
                                )
                            } else {
                                EventOptionTransition::AdvanceScreen
                            },
                            repeatable: false,
                            terminal: false,
                        },
                    )
                } else {
                    EventOption::new(
                        EventChoiceMeta::new(adj_label),
                        EventOptionSemantics {
                            action: EventActionKind::DeckOperation,
                            effects: vec![
                                EventEffect::LoseGold(adjust_cost(asc) as i32),
                                EventEffect::UpgradeCard {
                                    count: if upgrades_one(event_state.internal_state) {
                                        1
                                    } else {
                                        2
                                    },
                                },
                            ],
                            constraints: vec![
                                EventOptionConstraint::RequiresGold(adjust_cost(asc)),
                                EventOptionConstraint::RequiresUpgradeableCard,
                            ],
                            transition: if upgrades_one(event_state.internal_state) {
                                EventOptionTransition::OpenSelection(
                                    EventSelectionKind::UpgradeCard,
                                )
                            } else {
                                EventOptionTransition::AdvanceScreen
                            },
                            repeatable: false,
                            terminal: false,
                        },
                    )
                },
                if clean_disabled {
                    EventOption::new(
                        EventChoiceMeta::disabled(clean_label, "Not enough Gold"),
                        EventOptionSemantics {
                            action: EventActionKind::DeckOperation,
                            effects: vec![
                                EventEffect::LoseGold(cleanup_cost(asc) as i32),
                                if removes_cards(event_state.internal_state) {
                                    EventEffect::RemoveCard {
                                        count: 1,
                                        target_uuid: None,
                                        kind: EventCardKind::Unknown,
                                    }
                                } else {
                                    EventEffect::TransformCard { count: 2 }
                                },
                            ],
                            constraints: vec![
                                EventOptionConstraint::RequiresGold(cleanup_cost(asc)),
                                if removes_cards(event_state.internal_state) {
                                    EventOptionConstraint::RequiresRemovableCard
                                } else {
                                    EventOptionConstraint::RequiresTransformableCard
                                },
                            ],
                            transition: if removes_cards(event_state.internal_state) {
                                EventOptionTransition::OpenSelection(EventSelectionKind::RemoveCard)
                            } else {
                                EventOptionTransition::OpenSelection(
                                    EventSelectionKind::TransformCard,
                                )
                            },
                            repeatable: false,
                            terminal: false,
                        },
                    )
                } else {
                    EventOption::new(
                        EventChoiceMeta::new(clean_label),
                        EventOptionSemantics {
                            action: EventActionKind::DeckOperation,
                            effects: vec![
                                EventEffect::LoseGold(cleanup_cost(asc) as i32),
                                if removes_cards(event_state.internal_state) {
                                    EventEffect::RemoveCard {
                                        count: 1,
                                        target_uuid: None,
                                        kind: EventCardKind::Unknown,
                                    }
                                } else {
                                    EventEffect::TransformCard { count: 2 }
                                },
                            ],
                            constraints: vec![
                                EventOptionConstraint::RequiresGold(cleanup_cost(asc)),
                                if removes_cards(event_state.internal_state) {
                                    EventOptionConstraint::RequiresRemovableCard
                                } else {
                                    EventOptionConstraint::RequiresTransformableCard
                                },
                            ],
                            transition: if removes_cards(event_state.internal_state) {
                                EventOptionTransition::OpenSelection(EventSelectionKind::RemoveCard)
                            } else {
                                EventOptionTransition::OpenSelection(
                                    EventSelectionKind::TransformCard,
                                )
                            },
                            repeatable: false,
                            terminal: false,
                        },
                    )
                },
                if full_disabled {
                    EventOption::new(
                        EventChoiceMeta::disabled(full_label, "Not enough Gold"),
                        EventOptionSemantics {
                            action: EventActionKind::DeckOperation,
                            effects: vec![
                                EventEffect::LoseGold(full_service_cost(asc) as i32),
                                EventEffect::RemoveCard {
                                    count: 1,
                                    target_uuid: None,
                                    kind: EventCardKind::Unknown,
                                },
                                EventEffect::UpgradeCard { count: 1 },
                            ],
                            constraints: vec![
                                EventOptionConstraint::RequiresGold(full_service_cost(asc)),
                                EventOptionConstraint::RequiresRemovableCard,
                            ],
                            transition: EventOptionTransition::OpenSelection(
                                EventSelectionKind::RemoveCard,
                            ),
                            repeatable: false,
                            terminal: false,
                        },
                    )
                } else {
                    EventOption::new(
                        EventChoiceMeta::new(full_label),
                        EventOptionSemantics {
                            action: EventActionKind::DeckOperation,
                            effects: vec![
                                EventEffect::LoseGold(full_service_cost(asc) as i32),
                                EventEffect::RemoveCard {
                                    count: 1,
                                    target_uuid: None,
                                    kind: EventCardKind::Unknown,
                                },
                                EventEffect::UpgradeCard { count: 1 },
                            ],
                            constraints: vec![
                                EventOptionConstraint::RequiresGold(full_service_cost(asc)),
                                EventOptionConstraint::RequiresRemovableCard,
                            ],
                            transition: EventOptionTransition::OpenSelection(
                                EventSelectionKind::RemoveCard,
                            ),
                            repeatable: false,
                            terminal: false,
                        },
                    )
                },
                EventOption::new(
                    EventChoiceMeta::new(punch_label),
                    EventOptionSemantics {
                        action: EventActionKind::Decline,
                        effects: vec![EventEffect::LoseHp(hp_loss(asc))],
                        constraints: vec![],
                        transition: EventOptionTransition::AdvanceScreen,
                        repeatable: false,
                        terminal: false,
                    },
                ),
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
            event_state.current_screen = 1;
            run_state.event_state = Some(event_state);
        }
        1 => {
            let asc = run_state.ascension_level;
            match choice_idx {
                0 => {
                    // Adjust
                    run_state.gold -= adjust_cost(asc);
                    if upgrades_one(event_state.internal_state) {
                        // Upgrade 1: go to RunPendingChoice::Upgrade
                        event_state.current_screen = 2;
                        run_state.event_state = Some(event_state);
                        *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                            min_choices: 1,
                            max_choices: 1,
                            reason: RunPendingChoiceReason::Upgrade,
                            return_state: Box::new(EngineState::EventRoom),
                        });
                        return;
                    } else {
                        // Upgrade 2 random cards (auto, no grid)
                        // Java: Collections.shuffle(upgradableCards, new Random(miscRng.randomLong()))
                        let mut upgradable: Vec<usize> = run_state
                            .master_deck
                            .iter()
                            .enumerate()
                            .filter(|(_, c)| {
                                let def = crate::content::cards::get_card_definition(c.id);
                                def.card_type != crate::content::cards::CardType::Curse
                                    && def.card_type != crate::content::cards::CardType::Status
                            })
                            .map(|(i, _)| i)
                            .collect();
                        // Shuffle and upgrade up to 2
                        if !upgradable.is_empty() {
                            crate::runtime::rng::shuffle_with_random_long(
                                &mut upgradable,
                                &mut run_state.rng_pool.misc_rng,
                            );
                            run_state.master_deck[upgradable[0]].upgrades += 1;
                            if upgradable.len() > 1 {
                                run_state.master_deck[upgradable[1]].upgrades += 1;
                            }
                        }
                        event_state.current_screen = 2;
                    }
                }
                1 => {
                    // Clean Up
                    run_state.gold -= cleanup_cost(asc);
                    if removes_cards(event_state.internal_state) {
                        // Remove 1 card
                        event_state.current_screen = 2;
                        run_state.event_state = Some(event_state);
                        *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                            min_choices: 1,
                            max_choices: 1,
                            reason: RunPendingChoiceReason::Purge,
                            return_state: Box::new(EngineState::EventRoom),
                        });
                        return;
                    } else {
                        // Transform 2 cards
                        event_state.current_screen = 2;
                        run_state.event_state = Some(event_state);
                        *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                            min_choices: 2,
                            max_choices: 2,
                            reason: RunPendingChoiceReason::Transform,
                            return_state: Box::new(EngineState::EventRoom),
                        });
                        return;
                    }
                }
                2 => {
                    // Full Service: remove 1 card + upgrade 1 random (Java: REMOVE_AND_UPGRADE)
                    run_state.gold -= full_service_cost(asc);
                    event_state.extra_data = vec![1]; // Mark as Full Service for post-purge upgrade
                    event_state.current_screen = 2;
                    run_state.event_state = Some(event_state);
                    *engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
                        min_choices: 1,
                        max_choices: 1,
                        reason: RunPendingChoiceReason::Purge,
                        return_state: Box::new(EngineState::EventRoom),
                    });
                    return;
                }
                _ => {
                    // Punch: HP loss
                    run_state.current_hp = (run_state.current_hp - hp_loss(asc)).max(0);
                    event_state.current_screen = 2;
                }
            }
            run_state.event_state = Some(event_state);
        }
        _ => {
            // Returned from purge/upgrade/transform. For Full Service, upgrade 1 random card.
            // Java: REMOVE_AND_UPGRADE callback shuffles upgradable cards and upgrades [0].
            // We use extra_data[0] = 1 to mark Full Service so we can do the upgrade on return.
            if !event_state.extra_data.is_empty() && event_state.extra_data[0] == 1 {
                event_state.extra_data.clear();
                // Upgrade 1 random upgradable card
                let mut upgradable: Vec<usize> = run_state
                    .master_deck
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| {
                        let def = crate::content::cards::get_card_definition(c.id);
                        // canUpgrade(): SearingBlow always, others only once; curses never
                        match def.rarity {
                            crate::content::cards::CardRarity::Curse => false,
                            _ => {
                                c.id == crate::content::cards::CardId::SearingBlow
                                    || c.upgrades == 0
                            }
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();
                if !upgradable.is_empty() {
                    crate::runtime::rng::shuffle_with_random_long(
                        &mut upgradable,
                        &mut run_state.rng_pool.misc_rng,
                    );
                    run_state.master_deck[upgradable[0]].upgrades += 1;
                }
            }
            event_state.completed = true;
            run_state.event_state = Some(event_state);
        }
    }
}

/// Initialize Designer state.
/// Java constructor: miscRng.randomBoolean() × 2 for adjustmentUpgradesOne and cleanUpRemovesCards.
/// internal_state: bit0 = upgrades_one, bit1 = removes_cards
pub fn init_designer_state(run_state: &mut RunState) -> i32 {
    let upgrades_one = run_state.rng_pool.misc_rng.random_boolean();
    let removes_cards = run_state.rng_pool.misc_rng.random_boolean();
    (upgrades_one as i32) | ((removes_cards as i32) << 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::events::{EventOptionConstraint, EventOptionTransition, EventSelectionKind};

    #[test]
    fn designer_cleanup_remove_exposes_selection_semantics() {
        let mut rs = RunState::new(1, 0, true, "Ironclad");
        rs.gold = 99;
        let state = EventState {
            id: crate::state::events::EventId::Designer,
            current_screen: 1,
            internal_state: 0b11,
            completed: false,
            combat_pending: false,
            extra_data: Vec::new(),
        };
        let options = get_options(&rs, &state);
        assert_eq!(
            options[1].semantics.transition,
            EventOptionTransition::OpenSelection(EventSelectionKind::RemoveCard)
        );
        assert!(options[1]
            .semantics
            .constraints
            .contains(&EventOptionConstraint::RequiresRemovableCard));
    }
}
