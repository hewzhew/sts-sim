use crate::state::core::{EngineState, ClientInput};
use crate::state::run::RunState;
use crate::state::reward::{RewardState, RewardItem};
use crate::content::relics::RelicId;

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
        run_state.pending_boss_reward = false; // consume
        let mut relics = Vec::new();
        // Generate 3 unique boss relics
        for _ in 0..3 {
            // we should make sure they are unique if possible, but for now just roll 3 times
            // duplicate handling is ideally done during generation. We'll roll 3 times and accept it.
            let mut next_relic = run_state.random_relic_by_tier(crate::content::relics::RelicTier::Boss);
            // simple deduplication (max 10 retries to prevent infinite loop)
            let mut retries = 0;
            while relics.contains(&next_relic) && retries < 10 {
                next_relic = run_state.random_relic_by_tier(crate::content::relics::RelicTier::Boss);
                retries += 1;
            }
            relics.push(next_relic);
        }
        return EngineState::BossRelicSelect(crate::state::reward::BossRelicChoiceState::new(relics));
    }
    EngineState::MapNavigation
}

pub fn handle(run_state: &mut crate::state::run::RunState, reward_state: &mut crate::state::reward::RewardState, input: Option<crate::state::core::ClientInput>) -> Option<crate::state::core::EngineState> {
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
                            // Java: applyGoldBonus(false) — GoldenIdol adds 25% in non-treasure rooms
                            let bonus = if run_state.relics.iter().any(|r| r.id == RelicId::GoldenIdol) {
                                (amount as f32 * 0.25).round() as i32
                            } else {
                                0
                            };
                            run_state.gold += amount + bonus;
                        },
                        RewardItem::StolenGold { amount } => {
                            // Java: applyGoldBonus(theft=true) — no GoldenIdol bonus for stolen gold
                            run_state.gold += amount;
                        },
                        RewardItem::Relic { relic_id: id } => {
                            if let Some(next_state) = run_state.obtain_relic(id, EngineState::RewardScreen(crate::state::reward::RewardState::new())) {
                                return Some(next_state);
                            }
                        },
                        RewardItem::Potion { potion_id } => {
                            // Check Sozu — blocks obtaining potions
                            if run_state.relics.iter().any(|r| r.id == RelicId::Sozu) {
                                // Sozu prevents obtaining — discard the potion
                            } else if let Some(slot) = run_state.potions.iter().position(|p| p.is_none()) {
                                run_state.potions[slot] = Some(crate::content::potions::Potion::new(potion_id, 50000 + slot as u32));
                            } else {
                                // All slots full — put item back
                                reward_state.items.insert(idx, RewardItem::Potion { potion_id });
                            }
                        },
                        RewardItem::Card { cards } => {
                            // Enter card choice mode — player must pick one (or skip)
                            // Stay in RewardScreen; handler branches on pending_card_choice
                            reward_state.pending_card_choice = Some(cards);
                        },
                        RewardItem::EmeraldKey => {
                            // Java: ObtainKeyEffect(GREEN) — sets green key
                            run_state.keys[2] = true; // keys[2] = Green/Emerald
                        },
                        RewardItem::SapphireKey => {
                            // Java: ObtainKeyEffect(BLUE) — sets blue key
                            // Also cancels the linked relic reward
                            run_state.keys[1] = true; // keys[1] = Blue/Sapphire
                        }
                    }
                }
                if reward_state.items.is_empty() && reward_state.pending_card_choice.is_none() {
                    return Some(post_reward_state(run_state));
                }
            },
            crate::state::core::ClientInput::Proceed | crate::state::core::ClientInput::Cancel => {
                return Some(post_reward_state(run_state));
            },
            _ => {}
        }
    }
    None
}

/// Handle card choice selection.
/// Player must pick one card from the offered set, or skip (Cancel/Proceed).
fn handle_card_choice(run_state: &mut RunState, reward_state: &mut RewardState, input: Option<ClientInput>) -> Option<EngineState> {
    if let Some(in_val) = input {
        match in_val {
            ClientInput::SelectCard(idx) => {
                if let Some(ref cards) = reward_state.pending_card_choice {
                    if idx < cards.len() {
                        let card_id = cards[idx];
                        run_state.add_card_to_deck(card_id);
                    } else if idx == cards.len() {
                        // SingingBowl: extra option at index == cards.len()
                        // Choosing this gives +2 Max HP instead of a card
                        if run_state.relics.iter().any(|r| r.id == RelicId::SingingBowl) {
                            run_state.max_hp += 2;
                            run_state.current_hp = (run_state.current_hp + 2).min(run_state.max_hp);
                        }
                    }
                }
                reward_state.pending_card_choice = None;
                // Stay in RewardScreen — if no more items, proceed
                if reward_state.items.is_empty() {
                    return Some(post_reward_state(run_state));
                }
            },
            ClientInput::Proceed | ClientInput::Cancel => {
                // Skip card reward
                reward_state.pending_card_choice = None;
                if reward_state.items.is_empty() {
                    return Some(post_reward_state(run_state));
                }
            },
            _ => {}
        }
    }
    None
}


