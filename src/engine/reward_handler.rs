use crate::content::relics::RelicId;
use crate::state::core::{ClientInput, EngineState};
use crate::state::rewards::{RewardItem, RewardScreenContext, RewardState};
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
        return EngineState::BossRelicSelect(crate::state::rewards::BossRelicChoiceState::new(
            relics,
        ));
    }
    if run_state.complete_pending_boss_act_transition() {
        return EngineState::MapNavigation;
    }
    EngineState::MapNavigation
}

fn reward_map_overlay_or_post_reward_state(
    run_state: &mut RunState,
    reward_state: &RewardState,
) -> EngineState {
    if reward_state.items.is_empty() && reward_state.pending_card_choice.is_none() {
        post_reward_state(run_state)
    } else {
        EngineState::map_overlay(EngineState::RewardScreen(reward_state.clone()))
    }
}

fn reward_done_state(
    run_state: &mut RunState,
    overlay_return_state: Option<EngineState>,
) -> EngineState {
    overlay_return_state.unwrap_or_else(|| post_reward_state(run_state))
}

fn reward_state_has_unclaimed_content(reward_state: &RewardState) -> bool {
    !reward_state.items.is_empty() || reward_state.pending_card_choice.is_some()
}

fn append_reward_overlay(existing: &mut RewardState, incoming: &RewardState) {
    let index_offset = existing.items.len();
    existing.items.extend(incoming.items.clone());
    if existing.pending_card_choice.is_none() && incoming.pending_card_choice.is_some() {
        existing.pending_card_choice = incoming.pending_card_choice.clone();
        existing.pending_card_reward_index = incoming
            .pending_card_reward_index
            .map(|idx| idx.saturating_add(index_offset));
    }
}

fn return_state_with_pending_reward_overlay(
    return_state: EngineState,
    reward_state: &RewardState,
) -> EngineState {
    match return_state {
        EngineState::Shop(mut shop) if reward_state_has_unclaimed_content(reward_state) => {
            if let Some(existing) = &mut shop.pending_reward_overlay {
                append_reward_overlay(existing, reward_state);
            } else {
                shop.pending_reward_overlay = Some(reward_state.clone());
            }
            EngineState::Shop(shop)
        }
        other => other,
    }
}

fn reward_exit_state(
    run_state: &mut RunState,
    reward_state: &RewardState,
    overlay_return_state: Option<EngineState>,
) -> EngineState {
    overlay_return_state
        .map(|return_state| return_state_with_pending_reward_overlay(return_state, reward_state))
        .unwrap_or_else(|| reward_map_overlay_or_post_reward_state(run_state, reward_state))
}

fn current_reward_surface_state(
    reward_state: &RewardState,
    overlay_return_state: Option<&EngineState>,
) -> EngineState {
    match overlay_return_state {
        Some(return_state) => {
            EngineState::reward_overlay(reward_state.clone(), return_state.clone())
        }
        None => EngineState::RewardScreen(reward_state.clone()),
    }
}

pub fn handle(
    run_state: &mut crate::state::run::RunState,
    reward_state: &mut crate::state::rewards::RewardState,
    input: Option<crate::state::core::ClientInput>,
) -> Option<crate::state::core::EngineState> {
    handle_internal(run_state, reward_state, input, None)
}

pub fn handle_overlay(
    run_state: &mut crate::state::run::RunState,
    reward_state: &mut crate::state::rewards::RewardState,
    input: Option<crate::state::core::ClientInput>,
    return_state: EngineState,
) -> Option<crate::state::core::EngineState> {
    handle_internal(run_state, reward_state, input, Some(return_state))
}

pub(crate) fn skip_card_reward_item_for_branch_experiment(
    run_state: &mut RunState,
    reward_state: &mut RewardState,
    reward_index: usize,
) -> Result<Option<EngineState>, String> {
    if reward_state.pending_card_choice.is_some() {
        return Err("branch card reward skip requires an unopened reward item".to_string());
    }
    if !matches!(
        reward_state.items.get(reward_index),
        Some(RewardItem::Card { .. })
    ) {
        return Err(format!("reward item {reward_index} is not a card reward"));
    }
    reward_state.items.remove(reward_index);
    if reward_state.items.is_empty() {
        return Ok(Some(reward_done_state(run_state, None)));
    }
    Ok(None)
}

fn handle_internal(
    run_state: &mut crate::state::run::RunState,
    reward_state: &mut crate::state::rewards::RewardState,
    input: Option<crate::state::core::ClientInput>,
    overlay_return_state: Option<EngineState>,
) -> Option<crate::state::core::EngineState> {
    // If we're in card choice mode, handle that first
    if reward_state.pending_card_choice.is_some() {
        return handle_card_choice(run_state, reward_state, input, overlay_return_state);
    }

    if let Some(in_val) = input {
        match in_val {
            ClientInput::ClaimReward(idx) => {
                if idx < reward_state.items.len() {
                    if let Some(RewardItem::Card { cards }) = reward_state.items.get(idx) {
                        // Java opens CardRewardScreen with a pointer to the
                        // RewardItem and removes that item only after the
                        // player actually picks a card. Closing the card screen
                        // returns to the combat reward screen with the card
                        // reward still present.
                        reward_state.pending_card_choice = Some(cards.clone());
                        reward_state.pending_card_reward_index = Some(idx);
                        return None;
                    }

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
                            let return_state = current_reward_surface_state(
                                reward_state,
                                overlay_return_state.as_ref(),
                            );
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
                        RewardItem::Card { .. } => {
                            unreachable!("card rewards are handled before removal")
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
                    return Some(reward_done_state(run_state, overlay_return_state));
                }
            }
            crate::state::core::ClientInput::Proceed | crate::state::core::ClientInput::Cancel => {
                if reward_state.skippable {
                    return Some(reward_exit_state(
                        run_state,
                        reward_state,
                        overlay_return_state,
                    ));
                }
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
    overlay_return_state: Option<EngineState>,
) -> Option<EngineState> {
    if let Some(in_val) = input {
        match in_val {
            ClientInput::SelectCard(idx) => {
                let mut resolved_choice = false;
                if let Some(ref cards) = reward_state.pending_card_choice {
                    if idx < cards.len() {
                        let reward_card = &cards[idx];
                        run_state.add_card_to_deck_with_upgrades_from(
                            reward_card.id,
                            reward_card.upgrades,
                            DomainEventSource::RewardScreen,
                        );
                        resolved_choice = true;
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
                            resolved_choice = true;
                        }
                    }
                }
                if !resolved_choice {
                    return None;
                }
                reward_state.pending_card_choice = None;
                remove_pending_card_reward(reward_state);
                // Stay in RewardScreen — if no more items, proceed
                if reward_state.items.is_empty() {
                    return Some(reward_done_state(run_state, overlay_return_state));
                }
            }
            ClientInput::Proceed | ClientInput::Cancel => {
                // Java closes CardRewardScreen back to CombatRewardScreen
                // without consuming the underlying RewardItem. The player can
                // inspect map/relic/deck context and return later; the card
                // reward is only abandoned when the next room is committed.
                reward_state.pending_card_choice = None;
                reward_state.pending_card_reward_index = None;
            }
            _ => {}
        }
    }
    None
}

fn remove_pending_card_reward(reward_state: &mut RewardState) {
    let Some(idx) = reward_state.pending_card_reward_index.take() else {
        return;
    };
    if matches!(reward_state.items.get(idx), Some(RewardItem::Card { .. })) {
        reward_state.items.remove(idx);
    }
}

fn remove_linked_sapphire_key_after_claiming_relic(
    reward_state: &mut crate::state::rewards::RewardState,
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
    reward_state: &mut crate::state::rewards::RewardState,
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
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{ClientInput, EngineState};
    use crate::state::rewards::{RewardItem, RewardState};
    use crate::state::run::RunState;
    use crate::state::selection::{DomainEvent, DomainEventSource};
    use crate::state::shop::ShopState;

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
    fn unclaimed_reward_proceed_opens_map_overlay_without_dropping_rewards() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::Gold { amount: 25 }];

        let next = handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::Proceed),
        )
        .expect("proceed with unclaimed rewards should open map overlay");

        let EngineState::MapOverlay { return_state } = next else {
            panic!("expected Java-style dismissible map overlay");
        };
        let EngineState::RewardScreen(returned_rewards) = *return_state else {
            panic!("map overlay should return to the reward screen");
        };
        assert_eq!(
            returned_rewards.items,
            vec![RewardItem::Gold { amount: 25 }]
        );
        assert_eq!(reward_state.items, vec![RewardItem::Gold { amount: 25 }]);
        assert_eq!(run_state.gold, 99, "opening map must not claim gold");
    }

    #[test]
    fn card_reward_item_is_removed_only_after_selecting_card() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::Card {
            cards: vec![crate::state::rewards::RewardCard::new(
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
        assert_eq!(reward_state.pending_card_reward_index, Some(0));
        assert_eq!(
            reward_state.items.len(),
            1,
            "opening card choices is read-only"
        );

        handle(&mut run_state, &mut reward_state, Some(ClientInput::Cancel));
        assert!(reward_state.pending_card_choice.is_none());
        assert_eq!(
            reward_state.items.len(),
            1,
            "closing card choices returns to reward screen with the card reward intact"
        );

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
        assert!(reward_state.items.is_empty());
        assert!(run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::PommelStrike));
    }

    #[test]
    fn singing_bowl_card_reward_option_consumes_reward_and_grants_max_hp() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.relics.push(RelicState::new(RelicId::SingingBowl));
        let max_hp_before = run_state.max_hp;
        let deck_len_before = run_state.master_deck.len();
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::Card {
            cards: vec![crate::state::rewards::RewardCard::new(
                CardId::PommelStrike,
                0,
            )],
        }];

        handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(0)),
        );
        let next = handle(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::SelectCard(1)),
        )
        .expect("last reward should finish reward screen");

        assert_eq!(run_state.max_hp, max_hp_before + 2);
        assert_eq!(
            run_state.master_deck.len(),
            deck_len_before,
            "Singing Bowl should not add the offered card"
        );
        assert!(reward_state.items.is_empty());
        assert!(matches!(next, EngineState::MapNavigation));
    }

    #[test]
    fn overlay_card_reward_returns_to_shop_after_last_card_choice() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::Card {
            cards: vec![crate::state::rewards::RewardCard::new(
                CardId::PommelStrike,
                0,
            )],
        }];
        let shop_return = EngineState::Shop(ShopState::new());

        assert!(super::handle_overlay(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(0)),
            shop_return.clone(),
        )
        .is_none());
        assert!(reward_state.pending_card_choice.is_some());

        let next = super::handle_overlay(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::SelectCard(0)),
            shop_return,
        )
        .expect("last overlay reward should return to parent screen");

        assert!(matches!(next, EngineState::Shop(_)));
        assert!(reward_state.items.is_empty());
        assert!(run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::PommelStrike));
    }

    #[test]
    fn overlay_cancel_returns_to_shop_without_opening_map_preview() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::Gold { amount: 25 }];

        let next = super::handle_overlay(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::Cancel),
            EngineState::Shop(ShopState::new()),
        )
        .expect("overlay cancel should return to the parent screen");

        let EngineState::Shop(returned_shop) = next else {
            panic!("overlay cancel should return to shop");
        };
        assert_eq!(
            returned_shop
                .pending_reward_overlay
                .as_ref()
                .map(|reward| &reward.items),
            Some(&vec![RewardItem::Gold { amount: 25 }]),
            "Java combat reward overlay can be closed back to the shop without consuming or abandoning its reward items"
        );
        assert_eq!(run_state.gold, 99);
        assert_eq!(reward_state.items, vec![RewardItem::Gold { amount: 25 }]);
    }

    #[test]
    fn overlay_card_choice_cancel_persists_card_reward_on_shop() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut reward_state = RewardState::new();
        reward_state.items = vec![RewardItem::Card {
            cards: vec![crate::state::rewards::RewardCard::new(
                CardId::PommelStrike,
                0,
            )],
        }];
        let shop_return = EngineState::Shop(ShopState::new());

        assert!(super::handle_overlay(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::ClaimReward(0)),
            shop_return.clone(),
        )
        .is_none());
        assert!(reward_state.pending_card_choice.is_some());

        let next = super::handle_overlay(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::Cancel),
            shop_return,
        );
        assert!(
            next.is_none(),
            "closing the card reward sub-screen mutates the current reward overlay in place"
        );
        assert!(reward_state.pending_card_choice.is_none());
        assert_eq!(reward_state.items.len(), 1);

        let next = super::handle_overlay(
            &mut run_state,
            &mut reward_state,
            Some(ClientInput::Cancel),
            EngineState::Shop(ShopState::new()),
        )
        .expect("closing reward overlay should return to shop");
        let EngineState::Shop(returned_shop) = next else {
            panic!("overlay cancel should return to shop");
        };
        let pending = returned_shop
            .pending_reward_overlay
            .expect("shop should remember unclaimed overlay card reward");
        assert_eq!(pending.items.len(), 1);
        assert!(matches!(pending.items[0], RewardItem::Card { .. }));
    }

    #[test]
    fn overlay_cancel_merges_with_existing_shop_pending_rewards() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        let mut existing = RewardState::new();
        existing.items = vec![RewardItem::Gold { amount: 25 }];
        let mut shop = ShopState::new();
        shop.pending_reward_overlay = Some(existing);

        let mut active = RewardState::new();
        active.items = vec![RewardItem::Potion {
            potion_id: PotionId::FirePotion,
        }];

        let next = super::handle_overlay(
            &mut run_state,
            &mut active,
            Some(ClientInput::Cancel),
            EngineState::Shop(shop),
        )
        .expect("overlay cancel should return to shop");
        let EngineState::Shop(returned_shop) = next else {
            panic!("overlay cancel should return to shop");
        };
        let pending = returned_shop
            .pending_reward_overlay
            .expect("shop should keep merged pending rewards");
        assert_eq!(
            pending.items,
            vec![
                RewardItem::Gold { amount: 25 },
                RewardItem::Potion {
                    potion_id: PotionId::FirePotion
                }
            ]
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
            cards: vec![crate::state::rewards::RewardCard::new(
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
            cards: vec![crate::state::rewards::RewardCard::new(CardId::Regret, 0)],
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
