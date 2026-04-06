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
                            run_state.relics.push(crate::content::relics::RelicState::new(id));
                            if let Some(next_state) = apply_on_obtain_effect(run_state, id, EngineState::RewardScreen(crate::state::reward::RewardState::new())) {
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

/// Applies on-obtain (onEquip) effects for relics that have simulation-relevant immediate effects.
pub fn apply_on_obtain_effect(run_state: &mut RunState, relic_id: RelicId, return_state: EngineState) -> Option<EngineState> {
    use crate::content::cards::{get_card_definition, CardType, CardId};
    use crate::state::core::{RunPendingChoiceState, RunPendingChoiceReason};

    match relic_id {
        // === HP Gain ===
        RelicId::Strawberry => {
            // Java: increaseMaxHp(7, true) — +7 maxHP, heal proportionally
            run_state.max_hp += 7;
            run_state.current_hp = (run_state.current_hp + 7).min(run_state.max_hp);
        },
        RelicId::Pear => {
            run_state.max_hp += 10;
            run_state.current_hp = (run_state.current_hp + 10).min(run_state.max_hp);
        },
        RelicId::Mango => {
            run_state.max_hp += 14;
            run_state.current_hp = (run_state.current_hp + 14).min(run_state.max_hp);
        },
        RelicId::Waffle => {
            run_state.max_hp += 7;
            run_state.current_hp = run_state.max_hp;
        },

        // === Gold ===
        RelicId::OldCoin => {
            run_state.gold += 300;
        },

        // === Potion Slots ===
        RelicId::PotionBelt => {
            run_state.potions.push(None);
            run_state.potions.push(None);
        },

        // === Card Upgrades (shuffle-based, no UI needed) ===
        RelicId::Whetstone => {
            upgrade_random_cards_of_type(run_state, CardType::Attack, 2);
        },
        RelicId::WarPaint => {
            upgrade_random_cards_of_type(run_state, CardType::Skill, 2);
        },

        // === TinyHouse ===
        RelicId::TinyHouse => {
            run_state.max_hp += 5;
            run_state.current_hp = (run_state.current_hp + 5).min(run_state.max_hp);
            let mut upgradable: Vec<usize> = run_state.master_deck.iter().enumerate()
                .filter(|(_, c)| {
                    let def = get_card_definition(c.id);
                    def.card_type != CardType::Curse && (c.id == CardId::SearingBlow || c.upgrades == 0)
                })
                .map(|(i, _)| i)
                .collect();
            if !upgradable.is_empty() {
                crate::rng::shuffle_with_random_long(&mut upgradable, &mut run_state.rng_pool.misc_rng);
                run_state.master_deck[upgradable[0]].upgrades += 1;
            }
            run_state.gold += 50;
        },

        // === DollysMirror: duplicate a card from deck ===
        RelicId::DollysMirror => {
            if !run_state.master_deck.is_empty() {
                return Some(EngineState::RunPendingChoice(RunPendingChoiceState {
                    min_choices: 1,
                    max_choices: 1,
                    reason: RunPendingChoiceReason::Duplicate,
                    return_state: Box::new(return_state),
                }));
            }
        },

        // === Astrolabe: select 3 cards to Transform + Upgrade ===
        RelicId::Astrolabe => {
            let purgeable_count = run_state.master_deck.iter()
                .filter(|c| {
                    let def = get_card_definition(c.id);
                    def.card_type != CardType::Curse
                })
                .count();
            if purgeable_count > 0 {
                return Some(EngineState::RunPendingChoice(RunPendingChoiceState {
                    min_choices: purgeable_count.min(3),
                    max_choices: purgeable_count.min(3),
                    reason: RunPendingChoiceReason::Transform,
                    return_state: Box::new(return_state),
                }));
            }
        },

        // === EmptyCage: Purge 2 cards ===
        RelicId::EmptyCage => {
            let purgeable_count = run_state.master_deck.len();
            if purgeable_count > 0 {
                return Some(EngineState::RunPendingChoice(RunPendingChoiceState {
                    min_choices: purgeable_count.min(2),
                    max_choices: purgeable_count.min(2),
                    reason: RunPendingChoiceReason::Purge, // Purge reason natively deletes selected cards
                    return_state: Box::new(return_state),
                }));
            }
        },

        // === PandorasBox: Transform all Strikes and Defends ===
        RelicId::PandorasBox => {
            let results = crate::content::relics::pandoras_box::on_equip(run_state);
            if !results.is_empty() {
                println!("  [Pandora's Box] Transformed {} cards:", results.len());
                for (old, new) in &results {
                    println!("    {} → {}", old, new);
                }
            }
        },
        // === CallingBell: Curse of the Bell + 3 Relics ===
        RelicId::CallingBell => {
            // Add Curse of the Bell directly to deck
            run_state.add_card_to_deck(CardId::CurseOfTheBell);

            let mut rs = crate::state::reward::RewardState::new();
            rs.items.push(crate::state::reward::RewardItem::Relic { relic_id: run_state.random_relic_by_tier(crate::content::relics::RelicTier::Common) });
            rs.items.push(crate::state::reward::RewardItem::Relic { relic_id: run_state.random_relic_by_tier(crate::content::relics::RelicTier::Uncommon) });
            rs.items.push(crate::state::reward::RewardItem::Relic { relic_id: run_state.random_relic_by_tier(crate::content::relics::RelicTier::Rare) });
            
            return Some(EngineState::RewardScreen(rs));
        },

        // === Orrery: 5 Card Rewards ===
        RelicId::Orrery => {
            let mut rs = crate::state::reward::RewardState::new();
            for _ in 0..5 {
                let cards = run_state.generate_card_reward(3);
                rs.items.push(crate::state::reward::RewardItem::Card { cards });
            }
            return Some(EngineState::RewardScreen(rs));
        },

        _ => {}
    }
    None
}

/// Shuffle upgradable cards of a specific type and upgrade the first `count`.
/// Java: Whetstone/WarPaint pattern — miscRng.randomLong() shuffle + upgrade.
fn upgrade_random_cards_of_type(run_state: &mut RunState, card_type: crate::content::cards::CardType, count: usize) {
    use crate::content::cards::{get_card_definition, CardId};

    let mut upgradable_indices: Vec<usize> = run_state.master_deck.iter().enumerate()
        .filter(|(_, c)| {
            let def = get_card_definition(c.id);
            def.card_type == card_type
                && (c.id == CardId::SearingBlow || c.upgrades == 0)
        })
        .map(|(i, _)| i)
        .collect();

    if upgradable_indices.is_empty() {
        return;
    }

    crate::rng::shuffle_with_random_long(&mut upgradable_indices, &mut run_state.rng_pool.misc_rng);

    for i in 0..count.min(upgradable_indices.len()) {
        run_state.master_deck[upgradable_indices[i]].upgrades += 1;
    }
}
