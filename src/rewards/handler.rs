use crate::content::relics::RelicId;
use crate::rewards::state::{RewardItem, RewardScreenContext, RewardState};
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;
use crate::state::selection::DomainEventSource;

/// Determines the post-reward destination: EventRoom if an event-combat is pending, else MapNavigation.
/// If pending_boss_reward is set, advances to the next act before returning MapNavigation.
fn post_reward_state(run_state: &mut RunState) -> EngineState {
    if let Some(ref mut event_state) = run_state.event_state {
        if event_state.combat_pending {
            event_state.combat_pending = false;
            return EngineState::EventRoom;
        }
    }
    // After boss reward screen, trigger boss relic selection before advancing act
    if run_state.pending_boss_reward {
        run_state.pending_boss_reward = false;
        // Java BossChest constructor calls returnRandomRelic(BOSS) exactly
        // three times. The boss relic pool already removes candidates as they
        // are drawn, so there is no retry/dedup layer here.
        let relics = (0..3)
            .map(|_| run_state.random_relic_by_tier(crate::content::relics::RelicTier::Boss))
            .collect();
        return EngineState::BossRelicSelect(crate::rewards::state::BossRelicChoiceState::new(
            relics,
        ));
    }
    EngineState::MapNavigation
}

pub fn handle(
    run_state: &mut crate::state::run::RunState,
    reward_state: &mut crate::rewards::state::RewardState,
    input: Option<crate::state::core::ClientInput>,
) -> Option<crate::state::core::EngineState> {
    // If we're in card choice mode, handle that first
    if reward_state.pending_card_choice.is_some() {
        return handle_card_choice(run_state, reward_state, input);
    }

    if let Some(in_val) = input {
        match in_val {
            ClientInput::ClaimReward(idx) => {
                if idx < reward_state.items.len() {
                    let item = reward_state.items.remove(idx);
                    match item {
                        RewardItem::Gold { amount } => {
                            // Java RewardItem.applyGoldBonus(false): Golden Idol adds 25%
                            // except when the current room is a TreasureRoom.
                            let bonus = if reward_state.screen_context
                                != RewardScreenContext::TreasureRoom
                                && run_state.relics.iter().any(|r| r.id == RelicId::GoldenIdol)
                            {
                                crate::content::relics::golden_idol::reward_gold_bonus(amount)
                            } else {
                                0
                            };
                            run_state.change_gold_with_source(
                                amount + bonus,
                                DomainEventSource::RewardScreen,
                            );
                        }
                        RewardItem::StolenGold { amount } => {
                            // Java: applyGoldBonus(theft=true) — no GoldenIdol bonus for stolen gold
                            run_state
                                .change_gold_with_source(amount, DomainEventSource::RewardScreen);
                        }
                        RewardItem::Relic { relic_id: id } => {
                            remove_linked_sapphire_key_after_claiming_relic(reward_state, idx);
                            let return_state = EngineState::RewardScreen(reward_state.clone());
                            if let Some(next_state) = run_state.obtain_relic_with_source(
                                id,
                                return_state,
                                DomainEventSource::RewardScreen,
                            ) {
                                return Some(next_state);
                            }
                        }
                        RewardItem::Potion { potion_id } => {
                            // Check Sozu — blocks obtaining potions
                            if run_state.relics.iter().any(|r| r.id == RelicId::Sozu) {
                                // Sozu prevents obtaining — discard the potion
                            } else if let Some(slot) = run_state.find_empty_potion_slot() {
                                run_state.obtain_potion_with_source(
                                    crate::content::potions::Potion::new(
                                        potion_id,
                                        50000 + slot as u32,
                                    ),
                                    DomainEventSource::RewardScreen,
                                );
                            } else {
                                // All slots full — put item back
                                reward_state
                                    .items
                                    .insert(idx, RewardItem::Potion { potion_id });
                            }
                        }
                        RewardItem::Card { cards } => {
                            // Enter card choice mode — player must pick one (or skip)
                            // Stay in RewardScreen; handler branches on pending_card_choice
                            reward_state.pending_card_choice = Some(cards);
                        }
                        RewardItem::EmeraldKey => {
                            // Java: ObtainKeyEffect(GREEN) — sets green key
                            run_state.keys[2] = true; // keys[2] = Green/Emerald
                            run_state.map.has_emerald_key = true;
                        }
                        RewardItem::SapphireKey => {
                            // Java: ObtainKeyEffect(BLUE) — sets blue key
                            // Also cancels the linked relic reward.
                            run_state.keys[1] = true; // keys[1] = Blue/Sapphire
                            remove_linked_relic_after_claiming_sapphire_key(reward_state, idx);
                        }
                    }
                }
                if reward_state.items.is_empty() && reward_state.pending_card_choice.is_none() {
                    return Some(post_reward_state(run_state));
                }
            }
            crate::state::core::ClientInput::Proceed | crate::state::core::ClientInput::Cancel => {
                return Some(post_reward_state(run_state));
            }
            _ => {}
        }
    }
    None
}

/// Handle card choice selection.
/// Player must pick one card from the offered set, or skip (Cancel/Proceed).
fn handle_card_choice(
    run_state: &mut RunState,
    reward_state: &mut RewardState,
    input: Option<ClientInput>,
) -> Option<EngineState> {
    if let Some(in_val) = input {
        match in_val {
            ClientInput::SelectCard(idx) => {
                if let Some(ref cards) = reward_state.pending_card_choice {
                    if idx < cards.len() {
                        let reward_card = &cards[idx];
                        run_state.add_card_to_deck_with_upgrades_from(
                            reward_card.id,
                            reward_card.upgrades,
                            DomainEventSource::RewardScreen,
                        );
                    } else if idx == cards.len() {
                        // SingingBowl: extra option at index == cards.len()
                        // Choosing this gives +2 Max HP instead of a card
                        if run_state
                            .relics
                            .iter()
                            .any(|r| r.id == RelicId::SingingBowl)
                        {
                            run_state.gain_max_hp_with_source(
                                2,
                                2,
                                DomainEventSource::RewardScreen,
                            );
                        }
                    }
                }
                reward_state.pending_card_choice = None;
                // Stay in RewardScreen — if no more items, proceed
                if reward_state.items.is_empty() {
                    return Some(post_reward_state(run_state));
                }
            }
            ClientInput::Proceed | ClientInput::Cancel => {
                // Skip card reward
                reward_state.pending_card_choice = None;
                if reward_state.items.is_empty() {
                    return Some(post_reward_state(run_state));
                }
            }
            _ => {}
        }
    }
    None
}

fn remove_linked_sapphire_key_after_claiming_relic(
    reward_state: &mut crate::rewards::state::RewardState,
    removed_relic_index: usize,
) {
    if matches!(
        reward_state.items.get(removed_relic_index),
        Some(RewardItem::SapphireKey)
    ) {
        reward_state.items.remove(removed_relic_index);
    }
}

fn remove_linked_relic_after_claiming_sapphire_key(
    reward_state: &mut crate::rewards::state::RewardState,
    removed_key_index: usize,
) {
    if removed_key_index == 0 {
        return;
    }
    let linked_relic_index = removed_key_index - 1;
    if matches!(
        reward_state.items.get(linked_relic_index),
        Some(RewardItem::Relic { .. })
    ) {
        reward_state.items.remove(linked_relic_index);
    }
}

#[cfg(test)]
mod tests {
    use super::handle;
    use crate::content::cards::CardId;
    use crate::content::potions::PotionId;
    use crate::content::relics::{RelicId, RelicState};
    use crate::rewards::state::{RewardItem, RewardState};
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ClientInput, EngineState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};

    #[test]
    fn interrupting_relic_claim_preserves_remaining_reward_screen_items() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state
            .master_deck
            .push(CombatCard::new(CardId::PommelStrike, 1001));
        let mut reward_state = RewardState::new();
        reward_state.items = vec![
            RewardItem::Relic {
                relic_id: RelicId::BottledFlame,
            },
            RewardItem::Gold { amount: 25 },
        ];

        let next = handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(0)),
        )
        .expect("Bottled Flame should interrupt into deck selection");

        let EngineState::RunPendingChoice(choice) = next else {
            panic!("expected Bottled Flame selection");
        };
        let EngineState::RewardScreen(returned_rewards) = *choice.return_state else {
            panic!("Bottled Flame should return to the current reward screen");
        };
        assert_eq!(
            returned_rewards.items,
            vec![RewardItem::Gold { amount: 25 }]
        );
    }

    #[test]
    fn potion_reward_claim_matches_java_sozu_and_full_slot_behavior() {
        let mut sozu_run = RunState::new(1, 0, false, "Ironclad");
        sozu_run.relics.clear();
        sozu_run.relics.push(RelicState::new(RelicId::Sozu));
        sozu_run.potions = vec![None, None, None];
        let mut sozu_rewards = RewardState::new();
        sozu_rewards.items = vec![RewardItem::Potion {
            potion_id: PotionId::FirePotion,
        }];

        handle(
            &mut sozu_run,
            &mut sozu_rewards,
            Some(ClientInput::ClaimReward(0)),
        );

        assert!(sozu_rewards.items.is_empty());
        assert!(sozu_run.potions.iter().all(|slot| slot.is_none()));

        let mut full_run = RunState::new(1, 0, false, "Ironclad");
        full_run.potions = vec![
            Some(crate::content::potions::Potion::new(
                PotionId::BlockPotion,
                1,
            )),
            Some(crate::content::potions::Potion::new(
                PotionId::EnergyPotion,
                2,
            )),
            Some(crate::content::potions::Potion::new(
                PotionId::DexterityPotion,
                3,
            )),
        ];
        let mut full_rewards = RewardState::new();
        full_rewards.items = vec![RewardItem::Potion {
            potion_id: PotionId::FirePotion,
        }];

        handle(
            &mut full_run,
            &mut full_rewards,
            Some(ClientInput::ClaimReward(0)),
        );

        assert_eq!(
            full_rewards.items,
            vec![RewardItem::Potion {
                potion_id: PotionId::FirePotion
            }],
            "Java RewardItem.claimReward returns false on full potion slots, leaving the reward"
        );
    }

    #[test]
    fn boss_reward_generates_three_boss_relics_by_pool_order_without_retry_layer() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.pending_boss_reward = true;
        run_state.boss_relic_pool = vec![
            RelicId::CoffeeDripper,
            RelicId::BlackStar,
            RelicId::Astrolabe,
        ];

        let next = super::post_reward_state(&mut run_state);

        let EngineState::BossRelicSelect(state) = next else {
            panic!("boss reward should open boss relic select");
        };
        assert_eq!(
            state.relics,
            vec![
                RelicId::CoffeeDripper,
                RelicId::BlackStar,
                RelicId::Astrolabe
            ],
            "Java BossChest calls returnRandomRelic(BOSS) exactly three times"
        );
        assert!(run_state.boss_relic_pool.is_empty());
    }

    #[test]
    fn boss_chest_relic_choice_does_not_apply_non_boss_chest_hooks() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.push(RelicState::new(RelicId::CursedKey));
        run_state.relics.push(RelicState::new(RelicId::Matryoshka));
        run_state.relics.push(RelicState::new(RelicId::NlothsMask));
        run_state.pending_boss_reward = true;
        run_state.boss_relic_pool = vec![
            RelicId::CoffeeDripper,
            RelicId::BlackStar,
            RelicId::Astrolabe,
        ];

        let deck_len_before = run_state.master_deck.len();

        let next = super::post_reward_state(&mut run_state);

        let EngineState::BossRelicSelect(state) = next else {
            panic!("boss reward should open boss relic select");
        };
        assert_eq!(
            state.relics,
            vec![
                RelicId::CoffeeDripper,
                RelicId::BlackStar,
                RelicId::Astrolabe
            ],
            "Java BossChest.open(true) opens the three prebuilt boss relics without Matryoshka insertion or N'loth removal"
        );
        assert_eq!(
            run_state.master_deck.len(),
            deck_len_before,
            "Java CursedKey.onChestOpen(true) does not add a curse"
        );
        assert_eq!(
            run_state
                .relics
                .iter()
                .find(|relic| relic.id == RelicId::Matryoshka)
                .map(|relic| relic.counter),
            Some(2),
            "Java BossChest.open(true) explicitly skips Matryoshka"
        );
        assert_eq!(
            run_state
                .relics
                .iter()
                .find(|relic| relic.id == RelicId::NlothsMask)
                .map(|relic| relic.counter),
            Some(1),
            "Java boss chests do not run onChestOpenAfter"
        );
    }

    #[test]
    fn emerald_key_reward_claim_updates_owned_key_visibility_state() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.keys[2] = false;
        run_state.map.has_emerald_key = false;
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::EmeraldKey];

        handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(0)),
        );

        assert!(run_state.keys[2]);
        assert!(
            run_state.map.has_emerald_key,
            "legacy map-level key visibility mirrors the owned Emerald key after claiming it"
        );
    }

    #[test]
    fn sapphire_key_claim_cancels_linked_relic_reward_like_java() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.keys[1] = false;
        let mut reward_state = RewardState::new();
        reward_state.items = vec![
            RewardItem::Gold { amount: 25 },
            RewardItem::Relic {
                relic_id: RelicId::Mango,
            },
            RewardItem::SapphireKey,
        ];

        handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(2)),
        );

        assert!(run_state.keys[1]);
        assert_eq!(
            reward_state.items,
            vec![RewardItem::Gold { amount: 25 }],
            "Java RewardItem.claimReward(SAPPHIRE_KEY) marks its relicLink ignored/done"
        );
        assert!(!run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::Mango));
    }

    #[test]
    fn linked_relic_claim_cancels_sapphire_key_reward_like_java() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.keys[1] = false;
        let mut reward_state = RewardState::new();
        reward_state.items = vec![
            RewardItem::Gold { amount: 25 },
            RewardItem::Relic {
                relic_id: RelicId::Mango,
            },
            RewardItem::SapphireKey,
        ];

        handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(1)),
        );

        assert!(!run_state.keys[1]);
        assert_eq!(
            reward_state.items,
            vec![RewardItem::Gold { amount: 25 }],
            "Java RewardItem.claimReward(RELIC) marks its sapphire key relicLink ignored/done"
        );
        assert!(run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::Mango));
    }

    #[test]
    fn card_reward_selection_runs_obtain_hooks_before_card_obtained_event() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::CeramicFish));
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::Card {
            cards: vec![crate::rewards::state::RewardCard::new(
                CardId::PommelStrike,
                0,
            )],
        }];

        handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(0)),
        );
        assert!(reward_state.pending_card_choice.is_some());

        handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::SelectCard(0)),
        );

        let events = run_state.take_emitted_events();
        let fish_gold_pos = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    DomainEvent::GoldChanged {
                        delta: 9,
                        source: DomainEventSource::RewardScreen,
                        ..
                    }
                )
            })
            .expect("Reward card selection should run Ceramic Fish obtain hook");
        let obtained_pos = events
            .iter()
            .position(|event| {
                matches!(
                    event,
                    DomainEvent::CardObtained {
                        card,
                        source: DomainEventSource::RewardScreen,
                    } if card.id == CardId::PommelStrike
                )
            })
            .expect("Reward card selection should obtain the selected card");

        assert!(
            fish_gold_pos < obtained_pos,
            "Java CardRewardScreen queues FastCardObtainEffect; that effect runs onObtainCard before Soul.obtain"
        );
    }

    #[test]
    fn card_reward_selection_omamori_intercepts_curse_like_fast_obtain_effect() {
        let mut run_state = RunState::new(1, 0, true, "Ironclad");
        run_state.relics.clear();
        run_state.relics.push(RelicState::new(RelicId::Omamori));
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::Card {
            cards: vec![crate::rewards::state::RewardCard::new(CardId::Regret, 0)],
        }];

        handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(0)),
        );
        handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::SelectCard(0)),
        );

        assert!(!run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::Regret));
        let omamori = run_state
            .relics
            .iter()
            .find(|relic| relic.id == RelicId::Omamori)
            .expect("Omamori should remain after blocking reward curse");
        assert_eq!(omamori.counter, 1);
        assert!(
            !run_state.take_emitted_events().iter().any(|event| matches!(
                event,
                DomainEvent::CardObtained {
                    card,
                    source: DomainEventSource::RewardScreen,
                } if card.id == CardId::Regret
            ))
        );
    }
}
