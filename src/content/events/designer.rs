use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventChoiceMeta, EventState};
use crate::state::run::RunState;

// Java Designer: randomizes upgrade-one vs upgrade-two-random, and remove-one vs transform-two
// internal_state encodes: bit0 = adjustmentUpgradesOne, bit1 = cleanUpRemovesCards
// Costs: A15: 50/75/110/5hp, else: 40/60/90/3hp

fn adjust_cost(asc: u8) -> i32 { if asc >= 15 { 50 } else { 40 } }
fn cleanup_cost(asc: u8) -> i32 { if asc >= 15 { 75 } else { 60 } }
fn full_service_cost(asc: u8) -> i32 { if asc >= 15 { 110 } else { 90 } }
fn hp_loss(asc: u8) -> i32 { if asc >= 15 { 5 } else { 3 } }

fn upgrades_one(state: i32) -> bool { state & 1 != 0 }
fn removes_cards(state: i32) -> bool { state & 2 != 0 }

pub fn get_choices(run_state: &RunState, event_state: &EventState) -> Vec<EventChoiceMeta> {
    match event_state.current_screen {
        0 => vec![EventChoiceMeta::new("[Proceed]")],
        1 => {
            let asc = run_state.ascension_level;
            let has_upgradable = run_state.master_deck.iter().any(|c| {
                let def = crate::content::cards::get_card_definition(c.id);
                def.card_type != crate::content::cards::CardType::Curse
                    && def.card_type != crate::content::cards::CardType::Status
            });

            let adj_label = if upgrades_one(event_state.internal_state) {
                format!("[Adjust] {} Gold. Upgrade 1 card.", adjust_cost(asc))
            } else {
                format!("[Adjust] {} Gold. Upgrade 2 random cards.", adjust_cost(asc))
            };
            let adj_disabled = run_state.gold < adjust_cost(asc) || !has_upgradable;

            let clean_label = if removes_cards(event_state.internal_state) {
                format!("[Clean Up] {} Gold. Remove 1 card.", cleanup_cost(asc))
            } else {
                format!("[Clean Up] {} Gold. Transform 2 cards.", cleanup_cost(asc))
            };
            let clean_disabled = run_state.gold < cleanup_cost(asc);

            let full_label = format!("[Full Service] {} Gold. Remove 1 card + upgrade 1 random.", full_service_cost(asc));
            let full_disabled = run_state.gold < full_service_cost(asc);

            let punch_label = format!("[Punch] Lose {} HP.", hp_loss(asc));

            vec![
                if adj_disabled { EventChoiceMeta::disabled(adj_label, "Not enough Gold/cards") } else { EventChoiceMeta::new(adj_label) },
                if clean_disabled { EventChoiceMeta::disabled(clean_label, "Not enough Gold") } else { EventChoiceMeta::new(clean_label) },
                if full_disabled { EventChoiceMeta::disabled(full_label, "Not enough Gold") } else { EventChoiceMeta::new(full_label) },
                EventChoiceMeta::new(punch_label),
            ]
        },
        _ => vec![EventChoiceMeta::new("[Leave]")],
    }
}

pub fn handle_choice(engine_state: &mut EngineState, run_state: &mut RunState, choice_idx: usize) {
    let mut event_state = run_state.event_state.take().unwrap();

    match event_state.current_screen {
        0 => {
            event_state.current_screen = 1;
            run_state.event_state = Some(event_state);
        },
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
                        let mut upgradable: Vec<usize> = run_state.master_deck.iter().enumerate()
                            .filter(|(_, c)| {
                                let def = crate::content::cards::get_card_definition(c.id);
                                def.card_type != crate::content::cards::CardType::Curse
                                    && def.card_type != crate::content::cards::CardType::Status
                            })
                            .map(|(i, _)| i)
                            .collect();
                        // Shuffle and upgrade up to 2
                        if !upgradable.is_empty() {
                            crate::rng::shuffle_with_random_long(&mut upgradable, &mut run_state.rng_pool.misc_rng);
                            run_state.master_deck[upgradable[0]].upgrades += 1;
                            if upgradable.len() > 1 {
                                run_state.master_deck[upgradable[1]].upgrades += 1;
                            }
                        }
                        event_state.current_screen = 2;
                    }
                },
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
                },
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
                },
                _ => {
                    // Punch: HP loss
                    run_state.current_hp = (run_state.current_hp - hp_loss(asc)).max(0);
                    event_state.current_screen = 2;
                },
            }
            run_state.event_state = Some(event_state);
        },
        _ => {
            // Returned from purge/upgrade/transform. For Full Service, upgrade 1 random card.
            // Java: REMOVE_AND_UPGRADE callback shuffles upgradable cards and upgrades [0].
            // We use extra_data[0] = 1 to mark Full Service so we can do the upgrade on return.
            if !event_state.extra_data.is_empty() && event_state.extra_data[0] == 1 {
                event_state.extra_data.clear();
                // Upgrade 1 random upgradable card
                let mut upgradable: Vec<usize> = run_state.master_deck.iter().enumerate()
                    .filter(|(_, c)| {
                        let def = crate::content::cards::get_card_definition(c.id);
                        // canUpgrade(): SearingBlow always, others only once; curses never
                        match def.rarity {
                            crate::content::cards::CardRarity::Curse => false,
                            _ => c.id == crate::content::cards::CardId::SearingBlow || c.upgrades == 0,
                        }
                    })
                    .map(|(i, _)| i)
                    .collect();
                if !upgradable.is_empty() {
                    crate::rng::shuffle_with_random_long(&mut upgradable, &mut run_state.rng_pool.misc_rng);
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
