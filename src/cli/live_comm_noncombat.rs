use crate::diff::protocol::mapper::{
    card_id_from_java, java_potion_id_to_rust, relic_id_from_java,
};
use crate::diff::state_sync::snapshot_uuid;
use crate::state::core::{EngineState, RunPendingChoiceReason, RunPendingChoiceState};
use serde_json::Value;

#[derive(Clone, Debug)]
pub(crate) struct LiveEventPolicyTrace {
    pub command: String,
    pub summary: String,
    pub detail: String,
    pub audit: Value,
}

pub(crate) fn choose_live_event_command_with_trace(
    gs: &serde_json::Value,
    rs: &crate::state::run::RunState,
) -> Option<LiveEventPolicyTrace> {
    let context = crate::bot::event_policy::live_event_context(gs, rs)?;
    let decision = crate::bot::event_policy::choose_event_option(rs, &context)?;
    Some(LiveEventPolicyTrace {
        command: format!("CHOOSE {}", decision.command_index),
        summary: crate::bot::event_policy::compact_choice_summary(&context, &decision),
        detail: crate::bot::event_policy::describe_choice(&context, &decision),
        audit: crate::bot::event_policy::decision_trace_json(&context, &decision),
    })
}

pub(crate) fn choose_best_index(_choices: &[&str]) -> usize {
    0
}

fn has_available_command(gs: &serde_json::Value, command: &str) -> bool {
    gs.get("available_commands")
        .and_then(|v| v.as_array())
        .is_some_and(|commands| {
            commands
                .iter()
                .filter_map(|v| v.as_str())
                .any(|c| c.eq_ignore_ascii_case(command))
        })
}

pub(crate) fn decide_noncombat_with_agent(
    agent: &mut crate::bot::agent::Agent,
    root: &serde_json::Value,
    screen: &str,
    choice_list: &[&str],
) -> Option<String> {
    let gs = root.get("game_state").unwrap_or(root);
    let rs = build_live_run_state(gs)?;
    match screen {
        "SHOP_ROOM" => {
            let last_kind = root
                .get("protocol_meta")
                .and_then(|v| v.get("last_command_kind"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if matches!(last_kind, "return" | "leave" | "cancel") {
                if has_available_command(root, "proceed") {
                    Some("PROCEED".to_string())
                } else {
                    None
                }
            } else if has_available_command(root, "choose") && !choice_list.is_empty() {
                Some("CHOOSE 0".to_string())
            } else if has_available_command(root, "proceed") {
                Some("PROCEED".to_string())
            } else {
                None
            }
        }
        "SHOP_SCREEN" => {
            let shop = build_live_shop_state(gs)?;
            let input = agent.decide(&EngineState::Shop(shop.clone()), &rs, &None, false);
            shop_input_command(root, &rs, &shop, input)
        }
        "CARD_REWARD" => {
            let cards = gs
                .get("screen_state")
                .and_then(|v| v.get("cards"))
                .and_then(|v| v.as_array())?;
            let reward_cards: Vec<crate::rewards::state::RewardCard> = cards
                .iter()
                .filter_map(|card| {
                    let card_id = card
                        .get("id")
                        .and_then(|v| v.as_str())
                        .and_then(card_id_from_java)?;
                    let upgrades = card.get("upgrades").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                    Some(crate::rewards::state::RewardCard::new(card_id, upgrades))
                })
                .collect();
            if reward_cards.is_empty() {
                return None;
            }
            let reward = crate::rewards::state::RewardState {
                items: Vec::new(),
                skippable: gs
                    .get("screen_state")
                    .and_then(|v| v.get("skip_available"))
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true),
                pending_card_choice: Some(reward_cards),
            };
            let input = agent.decide(&EngineState::RewardScreen(reward), &rs, &None, false);
            match input {
                crate::state::core::ClientInput::SelectCard(idx)
                | crate::state::core::ClientInput::SubmitDiscoverChoice(idx) => {
                    Some(format!("CHOOSE {}", idx))
                }
                crate::state::core::ClientInput::Proceed
                | crate::state::core::ClientInput::Cancel => Some("SKIP".to_string()),
                _ => None,
            }
        }
        "COMBAT_REWARD" => {
            let rewards = build_live_reward_state_with_protocol(root, gs)?;
            if rewards.items.is_empty() {
                return blocked_potion_replacement_command(agent, root, &rs)
                    .or_else(|| Some("PROCEED".to_string()));
            }
            let input = agent.decide(&EngineState::RewardScreen(rewards), &rs, &None, false);
            match input {
                crate::state::core::ClientInput::ClaimReward(idx) => {
                    reward_choice_command_with_protocol(root, gs, idx)
                }
                crate::state::core::ClientInput::Proceed
                | crate::state::core::ClientInput::Cancel => {
                    blocked_potion_replacement_command(agent, root, &rs)
                        .or_else(|| Some("PROCEED".to_string()))
                }
                _ => None,
            }
        }
        "GRID" => decide_live_grid_screen(agent, root, &rs),
        "MAP" => {
            let input = agent.decide(&EngineState::MapNavigation, &rs, &None, false);
            match input {
                crate::state::core::ClientInput::SelectMapNode(target_x) => {
                    let target_x = target_x as i32;
                    choice_list
                        .iter()
                        .position(|choice| map_choice_x(choice) == Some(target_x))
                        .map(|idx| format!("CHOOSE {}", idx))
                }
                _ => None,
            }
        }
        "REST" => {
            let input = agent.decide(&EngineState::Campfire, &rs, &None, false);
            match input {
                crate::state::core::ClientInput::CampfireOption(choice) => {
                    campfire_choice_command(choice, choice_list)
                }
                crate::state::core::ClientInput::Proceed => Some("PROCEED".to_string()),
                crate::state::core::ClientInput::Cancel => Some("RETURN".to_string()),
                _ => None,
            }
        }
        "EVENT" => choose_live_event_command_with_trace(gs, &rs).map(|trace| trace.command),
        _ => None,
    }
}

fn shop_input_command(
    root: &serde_json::Value,
    rs: &crate::state::run::RunState,
    shop: &crate::shop::ShopState,
    input: crate::state::core::ClientInput,
) -> Option<String> {
    let choices = build_live_shop_choices(shop, rs);
    match input {
        crate::state::core::ClientInput::BuyCard(idx) => choices
            .iter()
            .position(|choice| matches!(choice, LiveShopChoice::Card(card_idx) if *card_idx == idx))
            .map(|idx| format!("CHOOSE {}", idx)),
        crate::state::core::ClientInput::BuyRelic(idx) => choices
            .iter()
            .position(
                |choice| matches!(choice, LiveShopChoice::Relic(relic_idx) if *relic_idx == idx),
            )
            .map(|idx| format!("CHOOSE {}", idx)),
        crate::state::core::ClientInput::BuyPotion(idx) => choices
            .iter()
            .position(
                |choice| matches!(choice, LiveShopChoice::Potion(potion_idx) if *potion_idx == idx),
            )
            .map(|idx| format!("CHOOSE {}", idx)),
        crate::state::core::ClientInput::PurgeCard(_) => choices
            .iter()
            .position(|choice| matches!(choice, LiveShopChoice::Purge))
            .map(|idx| format!("CHOOSE {}", idx)),
        crate::state::core::ClientInput::Proceed => {
            if has_available_command(root, "leave") {
                Some("LEAVE".to_string())
            } else if has_available_command(root, "return") || has_available_command(root, "cancel")
            {
                Some("RETURN".to_string())
            } else {
                None
            }
        }
        crate::state::core::ClientInput::Cancel => {
            if has_available_command(root, "leave") {
                Some("LEAVE".to_string())
            } else if has_available_command(root, "return") || has_available_command(root, "cancel")
            {
                Some("RETURN".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum LiveShopChoice {
    Purge,
    Card(usize),
    Relic(usize),
    Potion(usize),
}

fn build_live_shop_choices(
    shop: &crate::shop::ShopState,
    rs: &crate::state::run::RunState,
) -> Vec<LiveShopChoice> {
    let mut choices = Vec::new();
    if shop.purge_available && rs.gold >= shop.purge_cost {
        choices.push(LiveShopChoice::Purge);
    }
    for (idx, card) in shop.cards.iter().enumerate() {
        if card.can_buy && rs.gold >= card.price {
            choices.push(LiveShopChoice::Card(idx));
        }
    }
    for (idx, relic) in shop.relics.iter().enumerate() {
        if relic.can_buy && rs.gold >= relic.price {
            choices.push(LiveShopChoice::Relic(idx));
        }
    }
    for (idx, potion) in shop.potions.iter().enumerate() {
        if potion.can_buy && rs.gold >= potion.price {
            choices.push(LiveShopChoice::Potion(idx));
        }
    }
    choices
}

fn build_live_shop_state(gs: &serde_json::Value) -> Option<crate::shop::ShopState> {
    let screen_state = gs.get("screen_state")?;
    let mut shop = crate::shop::ShopState::new();

    shop.cards = screen_state
        .get("cards")
        .and_then(|v| v.as_array())
        .map(|cards| {
            cards
                .iter()
                .filter_map(|card| {
                    let card_id = card
                        .get("id")
                        .and_then(|v| v.as_str())
                        .and_then(card_id_from_java)?;
                    let price = card.get("price").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let can_buy = card
                        .get("can_buy")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let blocked_reason = card
                        .get("blocked_reason")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(crate::shop::ShopCard {
                        card_id,
                        price,
                        can_buy,
                        blocked_reason,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    shop.relics = screen_state
        .get("relics")
        .and_then(|v| v.as_array())
        .map(|relics| {
            relics
                .iter()
                .filter_map(|relic| {
                    let relic_id = relic
                        .get("id")
                        .and_then(|v| v.as_str())
                        .and_then(relic_id_from_java)?;
                    let price = relic.get("price").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let can_buy = relic
                        .get("can_buy")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let blocked_reason = relic
                        .get("blocked_reason")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(crate::shop::ShopRelic {
                        relic_id,
                        price,
                        can_buy,
                        blocked_reason,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    shop.potions = screen_state
        .get("potions")
        .and_then(|v| v.as_array())
        .map(|potions| {
            potions
                .iter()
                .filter_map(|potion| {
                    let potion_id = potion
                        .get("id")
                        .and_then(|v| v.as_str())
                        .and_then(java_potion_id_to_rust)?;
                    let price = potion.get("price").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                    let can_buy = potion
                        .get("can_buy")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true);
                    let blocked_reason = potion
                        .get("blocked_reason")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(crate::shop::ShopPotion {
                        potion_id,
                        price,
                        can_buy,
                        blocked_reason,
                    })
                })
                .collect()
        })
        .unwrap_or_default();

    shop.purge_available = screen_state
        .get("purge_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    shop.purge_cost = screen_state
        .get("purge_cost")
        .and_then(|v| v.as_i64())
        .unwrap_or(shop.purge_cost as i64) as i32;

    Some(shop)
}

fn campfire_choice_command(
    choice: crate::state::core::CampfireChoice,
    choice_list: &[&str],
) -> Option<String> {
    use crate::state::core::CampfireChoice;

    let target = match choice {
        CampfireChoice::Rest => "rest",
        CampfireChoice::Smith(_) => "smith",
        CampfireChoice::Dig => "dig",
        CampfireChoice::Lift => "lift",
        CampfireChoice::Toke(_) => "toke",
        CampfireChoice::Recall => "recall",
    };

    choice_list
        .iter()
        .position(|choice| choice.eq_ignore_ascii_case(target))
        .map(|idx| format!("CHOOSE {}", idx))
        .or_else(|| {
            if matches!(choice, CampfireChoice::Rest) {
                choice_list
                    .iter()
                    .position(|choice| choice.eq_ignore_ascii_case("sleep"))
                    .map(|idx| format!("CHOOSE {}", idx))
            } else {
                None
            }
        })
}

fn extract_recently_closed_card_reward_ids(root: &serde_json::Value) -> Option<Vec<String>> {
    let reward_session = root
        .get("protocol_meta")
        .and_then(|v| v.get("reward_session"))?;
    if reward_session.get("state").and_then(|v| v.as_str()) != Some("closed_without_choice") {
        return None;
    }
    let offered = reward_session
        .get("offered_card_ids")
        .and_then(|v| v.as_array())?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect::<Vec<_>>();
    if offered.is_empty() {
        None
    } else {
        Some(offered)
    }
}

fn reward_matches_recently_closed_card_session(
    reward: &serde_json::Value,
    recently_closed_card_ids: &[String],
    claimable_reward_count: usize,
) -> bool {
    if reward.get("reward_type").and_then(|v| v.as_str()) != Some("CARD") {
        return false;
    }

    let preview_ids = reward
        .get("preview_card_ids")
        .and_then(|v| v.as_array())
        .map(|cards| {
            cards
                .iter()
                .filter_map(|v| v.as_str().map(str::to_string))
                .collect::<Vec<_>>()
        });

    if let Some(preview_ids) = preview_ids {
        if !preview_ids.is_empty() {
            return preview_ids == recently_closed_card_ids;
        }
    }

    claimable_reward_count == 1
}

fn build_live_reward_state_with_protocol(
    root: &serde_json::Value,
    gs: &serde_json::Value,
) -> Option<crate::rewards::state::RewardState> {
    use crate::rewards::state::{RewardItem, RewardState};

    let rewards = gs
        .get("screen_state")
        .and_then(|v| v.get("rewards"))
        .and_then(|v| v.as_array())?;
    let claimable_reward_count = rewards
        .iter()
        .filter(|reward| {
            !reward
                .get("claimable")
                .and_then(|v| v.as_bool())
                .is_some_and(|claimable| !claimable)
        })
        .count();
    let recently_closed_card_ids = extract_recently_closed_card_reward_ids(root);

    let mut state = RewardState::new();
    for reward in rewards {
        if reward
            .get("claimable")
            .and_then(|v| v.as_bool())
            .is_some_and(|claimable| !claimable)
        {
            continue;
        }
        if let Some(recently_closed_card_ids) = recently_closed_card_ids.as_ref() {
            if reward_matches_recently_closed_card_session(
                reward,
                recently_closed_card_ids,
                claimable_reward_count,
            ) {
                continue;
            }
        }
        let reward_type = reward
            .get("reward_type")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        match reward_type {
            "GOLD" => {
                let amount = reward.get("gold").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                state.items.push(RewardItem::Gold { amount });
            }
            "POTION" => {
                if let Some(potion_id) = reward
                    .get("potion")
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    .and_then(java_potion_id_to_rust)
                {
                    state.items.push(RewardItem::Potion { potion_id });
                }
            }
            "RELIC" => {
                if let Some(relic_id) = reward
                    .get("relic")
                    .and_then(|v| v.get("id"))
                    .and_then(|v| v.as_str())
                    .and_then(relic_id_from_java)
                {
                    state.items.push(RewardItem::Relic { relic_id });
                }
            }
            "CARD" => state.items.push(RewardItem::Card { cards: Vec::new() }),
            "EMERALD_KEY" => state.items.push(RewardItem::EmeraldKey),
            "SAPPHIRE_KEY" => state.items.push(RewardItem::SapphireKey),
            _ => {}
        }
    }
    Some(state)
}

#[cfg(test)]
fn build_live_reward_state(gs: &serde_json::Value) -> Option<crate::rewards::state::RewardState> {
    build_live_reward_state_with_protocol(&serde_json::Value::Null, gs)
}

#[cfg(test)]
fn reward_choice_command(gs: &serde_json::Value, reward_idx: usize) -> Option<String> {
    let rewards = gs
        .get("screen_state")
        .and_then(|v| v.get("rewards"))
        .and_then(|v| v.as_array())?;

    let command_idx = rewards
        .iter()
        .nth(reward_idx)?
        .get("choice_index")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)
        .or_else(|| {
            rewards
                .iter()
                .enumerate()
                .filter(|(_, reward)| {
                    !reward
                        .get("claimable")
                        .and_then(|v| v.as_bool())
                        .is_some_and(|claimable| !claimable)
                })
                .nth(reward_idx)
                .map(|(idx, _)| idx)
        })?;

    Some(format!("CHOOSE {}", command_idx))
}

fn reward_choice_command_with_protocol(
    root: &serde_json::Value,
    gs: &serde_json::Value,
    reward_idx: usize,
) -> Option<String> {
    let rewards = gs
        .get("screen_state")
        .and_then(|v| v.get("rewards"))
        .and_then(|v| v.as_array())?;
    let claimable_reward_count = rewards
        .iter()
        .filter(|reward| {
            !reward
                .get("claimable")
                .and_then(|v| v.as_bool())
                .is_some_and(|claimable| !claimable)
        })
        .count();
    let recently_closed_card_ids = extract_recently_closed_card_reward_ids(root);

    let command_idx = rewards
        .iter()
        .filter(|reward| {
            if reward
                .get("claimable")
                .and_then(|v| v.as_bool())
                .is_some_and(|claimable| !claimable)
            {
                return false;
            }
            if let Some(recently_closed_card_ids) = recently_closed_card_ids.as_ref() {
                if reward_matches_recently_closed_card_session(
                    reward,
                    recently_closed_card_ids,
                    claimable_reward_count,
                ) {
                    return false;
                }
            }
            true
        })
        .nth(reward_idx)?
        .get("choice_index")
        .and_then(|v| v.as_u64())
        .map(|v| v as usize)?;

    Some(format!("CHOOSE {}", command_idx))
}

fn blocked_potion_replacement_command(
    agent: &crate::bot::agent::Agent,
    root: &serde_json::Value,
    rs: &crate::state::run::RunState,
) -> Option<String> {
    if !has_available_command(root, "potion") {
        return None;
    }

    let gs = root.get("game_state").unwrap_or(root);
    let rewards = gs
        .get("screen_state")
        .and_then(|v| v.get("rewards"))
        .and_then(|v| v.as_array())?;

    let offered_potion = rewards
        .iter()
        .filter_map(blocked_replaceable_reward_potion_id)
        .max_by_key(|potion_id| reward_potion_score(agent, rs, *potion_id))?;

    let offered_score = reward_potion_score(agent, rs, offered_potion);
    let (discard_idx, kept_score) = rs
        .potions
        .iter()
        .enumerate()
        .filter_map(|(idx, potion)| {
            potion
                .as_ref()
                .map(|potion| (idx, reward_potion_score(agent, rs, potion.id)))
        })
        .min_by_key(|(_, score)| *score)?;

    if offered_score > kept_score {
        Some(format!("POTION DISCARD {}", discard_idx))
    } else {
        None
    }
}

fn blocked_replaceable_reward_potion_id(
    reward: &serde_json::Value,
) -> Option<crate::content::potions::PotionId> {
    if reward.get("reward_type").and_then(|v| v.as_str()) != Some("POTION") {
        return None;
    }

    let blocked_reason = reward.get("blocked_reason").and_then(|v| v.as_str());
    let can_discard = reward
        .get("can_discard")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let claimable = reward
        .get("claimable")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    if claimable || blocked_reason != Some("potion_slots_full") || !can_discard {
        return None;
    }

    reward
        .get("potion")
        .and_then(|v| v.get("id"))
        .and_then(|v| v.as_str())
        .and_then(java_potion_id_to_rust)
}

fn reward_potion_score(
    agent: &crate::bot::agent::Agent,
    rs: &crate::state::run::RunState,
    potion_id: crate::content::potions::PotionId,
) -> i32 {
    agent
        .shop_potion_score(rs, potion_id)
        .max(base_reward_potion_score(potion_id))
}

fn base_reward_potion_score(potion_id: crate::content::potions::PotionId) -> i32 {
    use crate::content::potions::PotionId;

    match potion_id {
        PotionId::AncientPotion => 100,
        PotionId::PowerPotion | PotionId::ColorlessPotion => 94,
        PotionId::DuplicationPotion | PotionId::GhostInAJar => 90,
        PotionId::FruitJuice | PotionId::BloodPotion | PotionId::FairyPotion => 88,
        PotionId::BlessingOfTheForge => 84,
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze
        | PotionId::RegenPotion => 85,
        PotionId::EnergyPotion | PotionId::SwiftPotion => 82,
        _ => 55,
    }
}

fn decide_live_grid_screen(
    agent: &mut crate::bot::agent::Agent,
    root: &serde_json::Value,
    rs: &crate::state::run::RunState,
) -> Option<String> {
    let gs = root.get("game_state").unwrap_or(root);
    let screen_state = gs.get("screen_state")?;
    let can_choose = has_available_command(root, "choose");
    let can_confirm =
        has_available_command(root, "confirm") || has_available_command(root, "proceed");
    let can_cancel = has_available_command(root, "cancel")
        || has_available_command(root, "return")
        || has_available_command(root, "leave");

    if !can_choose {
        if can_confirm {
            return Some("CONFIRM".to_string());
        }
        if can_cancel {
            return Some("RETURN".to_string());
        }
        return None;
    }

    let cards = screen_state.get("cards")?.as_array()?;
    if cards.is_empty() {
        if can_confirm {
            return Some("CONFIRM".to_string());
        }
        return None;
    }

    let selected_cards = screen_state
        .get("selected_cards")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let selected: std::collections::HashSet<u32> = selected_cards
        .iter()
        .enumerate()
        .map(|(idx, card)| snapshot_uuid(&card["uuid"], 70_000 + idx as u32))
        .collect();

    let current_action = gs
        .get("current_action")
        .and_then(|v| v.as_str())
        .or_else(|| {
            gs.get("combat_state")
                .and_then(|v| v.get("current_action"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or("");

    let for_upgrade = screen_state
        .get("for_upgrade")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let for_purge = screen_state
        .get("for_purge")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let for_transform = screen_state
        .get("for_transform")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if let Some(choice_state) = build_live_run_pending_choice_state(
        current_action,
        for_upgrade,
        for_purge,
        for_transform,
        cards.len(),
        screen_state,
    ) {
        match agent.decide(
            &EngineState::RunPendingChoice(choice_state),
            rs,
            &None,
            false,
        ) {
            crate::state::core::ClientInput::SubmitDeckSelect(indices) => {
                if let Some(idx) = indices.into_iter().find(|idx| {
                    *idx < cards.len()
                        && !selected
                            .contains(&snapshot_uuid(&cards[*idx]["uuid"], 60_000 + *idx as u32))
                }) {
                    return Some(format!("CHOOSE {}", idx));
                }
            }
            crate::state::core::ClientInput::SubmitSelection(selection) => {
                if selection.scope == crate::state::selection::SelectionScope::Deck {
                    let desired = selection
                        .selected
                        .into_iter()
                        .filter_map(|target| match target {
                            crate::state::selection::SelectionTargetRef::CardUuid(uuid) => {
                                Some(uuid)
                            }
                        })
                        .collect::<Vec<_>>();
                    if let Some(idx) = cards.iter().enumerate().find_map(|(idx, card)| {
                        let uuid = snapshot_uuid(&card["uuid"], 60_000 + idx as u32);
                        if desired.contains(&uuid) && !selected.contains(&uuid) {
                            Some(idx)
                        } else {
                            None
                        }
                    }) {
                        return Some(format!("CHOOSE {}", idx));
                    }
                }
            }
            _ => {}
        }
    }

    let mut best_idx = None;
    let mut best_score = i32::MIN;

    for (idx, card) in cards.iter().enumerate() {
        let uuid = snapshot_uuid(&card["uuid"], 60_000 + idx as u32);
        if selected.contains(&uuid) {
            continue;
        }
        let Some(card_id) = card
            .get("id")
            .and_then(|v| v.as_str())
            .and_then(card_id_from_java)
        else {
            continue;
        };

        let mut score = crate::bot::evaluator::CardEvaluator::evaluate_owned_card(card_id, rs);
        if current_action == "DiscardPileToTopOfDeckAction" {
            score += 15;
        } else if current_action.contains("DiscardPileToHandAction")
            || current_action.contains("BetterDiscardPileToHandAction")
            || current_action.contains("ExhumeAction")
            || current_action.contains("SecretTechnique")
            || current_action.contains("SecretWeapon")
        {
            score += 10;
        }

        if for_purge || for_transform {
            score = -score;
        } else if for_upgrade {
            let upgrades = card.get("upgrades").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
            score += 20 - upgrades * 10;
            score += live_upgrade_priority(card_id, rs);
        }

        if score > best_score {
            best_score = score;
            best_idx = Some(idx);
        }
    }

    best_idx.map(|idx| format!("CHOOSE {}", idx))
}

fn build_live_run_pending_choice_state(
    current_action: &str,
    for_upgrade: bool,
    for_purge: bool,
    for_transform: bool,
    card_count: usize,
    screen_state: &serde_json::Value,
) -> Option<RunPendingChoiceState> {
    let max_choices = screen_state
        .get("num_cards")
        .and_then(|v| v.as_u64())
        .map(|value| value as usize)
        .unwrap_or(1)
        .min(card_count.max(1));
    let min_choices = if screen_state
        .get("any_number")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        0
    } else {
        max_choices
    };

    let reason = if for_purge {
        Some(RunPendingChoiceReason::Purge)
    } else if for_upgrade {
        Some(RunPendingChoiceReason::Upgrade)
    } else if for_transform {
        Some(RunPendingChoiceReason::Transform)
    } else if current_action.contains("Duplicate") || current_action.contains("Mirror") {
        Some(RunPendingChoiceReason::Duplicate)
    } else {
        None
    }?;

    Some(RunPendingChoiceState {
        min_choices,
        max_choices,
        reason,
        return_state: Box::new(EngineState::EventRoom),
    })
}

fn live_upgrade_priority(
    card_id: crate::content::cards::CardId,
    rs: &crate::state::run::RunState,
) -> i32 {
    use crate::content::cards::CardId;

    let profile = crate::bot::evaluator::CardEvaluator::deck_profile(rs);
    let mut score = match card_id {
        CardId::Whirlwind => 42,
        CardId::DemonForm => 38,
        CardId::Armaments => 34,
        CardId::Bash => 30,
        CardId::BattleTrance | CardId::ShrugItOff | CardId::TrueGrit | CardId::FlameBarrier => 24,
        CardId::Corruption | CardId::FeelNoPain | CardId::DarkEmbrace => 34,
        CardId::Barricade | CardId::Entrench | CardId::BodySlam => 30,
        CardId::LimitBreak | CardId::HeavyBlade | CardId::SwordBoomerang => 24,
        CardId::Strike | CardId::StrikeG => -18,
        CardId::Defend | CardId::DefendG => -22,
        CardId::Neutralize => 26,
        CardId::Survivor => 28,
        CardId::DeadlyPoison => 26,
        CardId::Prepared => 18,
        CardId::DaggerThrow => 24,
        CardId::PoisonedStab => 26,
        CardId::DaggerSpray => 24,
        CardId::BladeDance => 24,
        CardId::Backflip => 24,
        CardId::Acrobatics => 24,
        CardId::CloakAndDagger => 22,
        CardId::Catalyst => 20,
        CardId::BouncingFlask => 24,
        CardId::Footwork => 28,
        CardId::NoxiousFumes => 28,
        CardId::Adrenaline => 34,
        CardId::AfterImage => 30,
        CardId::Burst => 28,
        _ => 0,
    };

    if profile.strength_enablers >= 1 {
        score += match card_id {
            CardId::Whirlwind | CardId::HeavyBlade | CardId::LimitBreak => 10,
            _ => 0,
        };
    }
    if profile.exhaust_engines >= 1 || profile.exhaust_fodder >= 1 {
        score += match card_id {
            CardId::FeelNoPain | CardId::DarkEmbrace | CardId::TrueGrit => 10,
            _ => 0,
        };
    }
    if profile.block_core >= 2 {
        score += match card_id {
            CardId::FlameBarrier | CardId::Barricade | CardId::BodySlam => 8,
            _ => 0,
        };
    }

    score
}

pub(crate) fn build_live_run_state(gs: &serde_json::Value) -> Option<crate::state::run::RunState> {
    let seed = gs.get("seed").and_then(|v| v.as_u64()).unwrap_or(0);
    let ascension = gs
        .get("ascension_level")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u8;
    let player_class = match gs
        .get("class")
        .and_then(|v| v.as_str())
        .unwrap_or("IRONCLAD")
    {
        "IRONCLAD" => "Ironclad",
        "SILENT" => "Silent",
        "DEFECT" => "Defect",
        "WATCHER" => "Watcher",
        _ => "Ironclad",
    };
    let mut rs = crate::state::run::RunState::new(seed, ascension, false, player_class);
    rs.act_num = gs.get("act").and_then(|v| v.as_u64()).unwrap_or(1) as u8;
    rs.floor_num = gs.get("floor").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
    rs.current_hp = gs.get("current_hp").and_then(|v| v.as_i64()).unwrap_or(80) as i32;
    rs.max_hp = gs
        .get("max_hp")
        .and_then(|v| v.as_i64())
        .unwrap_or(rs.max_hp as i64) as i32;
    rs.gold = gs
        .get("gold")
        .and_then(|v| v.as_i64())
        .unwrap_or(rs.gold as i64) as i32;
    rs.keys = [
        gs.get("keys")
            .and_then(|v| v.get("ruby"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        gs.get("keys")
            .and_then(|v| v.get("emerald"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        gs.get("keys")
            .and_then(|v| v.get("sapphire"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    ];
    rs.master_deck = gs
        .get("deck")
        .and_then(|v| v.as_array())
        .map(|deck| {
            deck.iter()
                .enumerate()
                .filter_map(|(idx, card)| {
                    let id = card
                        .get("id")
                        .and_then(|v| v.as_str())
                        .and_then(card_id_from_java)?;
                    let upgrades = card.get("upgrades").and_then(|v| v.as_u64()).unwrap_or(0) as u8;
                    let mut combat_card = crate::combat::CombatCard::new(id, idx as u32);
                    combat_card.upgrades = upgrades;
                    Some(combat_card)
                })
                .collect()
        })
        .unwrap_or_default();
    rs.relics = gs
        .get("relics")
        .and_then(|v| v.as_array())
        .map(|relics| {
            relics
                .iter()
                .filter_map(|relic| {
                    let id = relic
                        .get("id")
                        .and_then(|v| v.as_str())
                        .and_then(relic_id_from_java)?;
                    let mut state = crate::content::relics::RelicState::new(id);
                    state.counter =
                        relic.get("counter").and_then(|v| v.as_i64()).unwrap_or(-1) as i32;
                    state.used_up = relic
                        .get("used_up")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    Some(state)
                })
                .collect()
        })
        .unwrap_or_default();
    rs.potions = gs
        .get("potions")
        .and_then(|v| v.as_array())
        .map(|potions| {
            potions
                .iter()
                .enumerate()
                .map(|(idx, potion)| {
                    potion
                        .get("id")
                        .and_then(|v| v.as_str())
                        .and_then(java_potion_id_to_rust)
                        .map(|id| crate::content::potions::Potion::new(id, 10_000 + idx as u32))
                })
                .collect()
        })
        .unwrap_or_else(|| vec![None, None, None]);
    if let Some(map_state) = build_live_map_state(gs) {
        rs.map = map_state;
    }
    Some(rs)
}

#[cfg(test)]
mod tests {
    use super::{
        blocked_potion_replacement_command, build_live_reward_state,
        build_live_reward_state_with_protocol, build_live_shop_state, decide_noncombat_with_agent,
        reward_choice_command, reward_choice_command_with_protocol,
    };

    #[test]
    fn build_live_shop_state_reads_can_buy_and_blocked_reason() {
        let gs = serde_json::json!({
            "screen_state": {
                "cards": [],
                "relics": [],
                "potions": [
                    {
                        "id": "PowerPotion",
                        "price": 52,
                        "can_buy": false,
                        "blocked_reason": "sozu"
                    }
                ],
                "purge_available": true,
                "purge_cost": 75
            }
        });

        let shop = build_live_shop_state(&gs).unwrap();

        assert_eq!(shop.potions.len(), 1);
        assert!(!shop.potions[0].can_buy);
        assert_eq!(shop.potions[0].blocked_reason.as_deref(), Some("sozu"));
    }

    #[test]
    fn build_live_reward_state_skips_unclaimable_rewards() {
        let gs = serde_json::json!({
            "screen_state": {
                "rewards": [
                    {
                        "reward_type": "POTION",
                        "claimable": false,
                        "blocked_reason": "sozu",
                        "potion": { "id": "PowerPotion" }
                    },
                    {
                        "reward_type": "GOLD",
                        "claimable": true,
                        "gold": 25
                    }
                ]
            }
        });

        let rewards = build_live_reward_state(&gs).unwrap();

        assert_eq!(rewards.items.len(), 1);
        assert!(matches!(
            rewards.items.first(),
            Some(crate::rewards::state::RewardItem::Gold { amount: 25 })
        ));
    }

    #[test]
    fn reward_choice_command_skips_unclaimable_reward_slots() {
        let gs = serde_json::json!({
            "screen_state": {
                "rewards": [
                    {
                        "reward_type": "POTION",
                        "claimable": false,
                        "blocked_reason": "potion_slots_full",
                        "can_discard": true,
                        "potion": { "id": "PowerPotion" }
                    },
                    {
                        "reward_type": "CARD",
                        "claimable": true
                    }
                ]
            }
        });

        let command = reward_choice_command(&gs, 0);

        assert_eq!(command.as_deref(), Some("CHOOSE 1"));
    }

    #[test]
    fn build_live_reward_state_filters_recently_skipped_card_reward() {
        let root = serde_json::json!({
            "protocol_meta": {
                "reward_session": {
                    "state": "closed_without_choice",
                    "offered_card_ids": ["flex", "sword_boomerang", "perfected_strike"]
                }
            },
            "game_state": {
                "screen_state": {
                    "rewards": [
                        {
                            "reward_type": "CARD",
                            "claimable": true,
                            "choice_index": 0,
                            "preview_card_ids": ["flex", "sword_boomerang", "perfected_strike"]
                        },
                        {
                            "reward_type": "GOLD",
                            "claimable": true,
                            "choice_index": 1,
                            "gold": 25
                        }
                    ]
                }
            }
        });

        let rewards = build_live_reward_state_with_protocol(&root, &root["game_state"]).unwrap();

        assert_eq!(rewards.items.len(), 1);
        assert!(matches!(
            rewards.items.first(),
            Some(crate::rewards::state::RewardItem::Gold { amount: 25 })
        ));
    }

    #[test]
    fn reward_choice_command_with_protocol_skips_recently_closed_card_reward() {
        let root = serde_json::json!({
            "protocol_meta": {
                "reward_session": {
                    "state": "closed_without_choice",
                    "offered_card_ids": ["flex", "sword_boomerang", "perfected_strike"]
                }
            },
            "game_state": {
                "screen_state": {
                    "rewards": [
                        {
                            "reward_type": "CARD",
                            "claimable": true,
                            "choice_index": 0,
                            "preview_card_ids": ["flex", "sword_boomerang", "perfected_strike"]
                        },
                        {
                            "reward_type": "GOLD",
                            "claimable": true,
                            "choice_index": 1,
                            "gold": 25
                        }
                    ]
                }
            }
        });

        let command = reward_choice_command_with_protocol(&root, &root["game_state"], 0);

        assert_eq!(command.as_deref(), Some("CHOOSE 1"));
    }

    #[test]
    fn decide_noncombat_with_agent_proceeds_after_skipping_only_card_reward() {
        let root = serde_json::json!({
            "available_commands": ["choose", "proceed", "state"],
            "game_state": {
                "screen_state": {
                    "rewards": [
                        {
                            "reward_type": "CARD",
                            "claimable": true,
                            "choice_index": 0,
                            "preview_card_ids": ["flex", "sword_boomerang", "perfected_strike"]
                        }
                    ]
                },
                "class": "IRONCLAD",
                "current_hp": 70,
                "max_hp": 80,
                "gold": 99,
                "floor": 10,
                "act": 1,
                "deck": [],
                "relics": [],
                "potions": [],
                "map": [],
                "screen_type": "COMBAT_REWARD",
                "room_phase": "COMPLETE",
                "choice_list": ["card"]
            },
            "protocol_meta": {
                "reward_session": {
                    "state": "closed_without_choice",
                    "offered_card_ids": ["flex", "sword_boomerang", "perfected_strike"]
                }
            }
        });
        let mut agent = crate::bot::agent::Agent::new();

        let cmd = decide_noncombat_with_agent(&mut agent, &root, "COMBAT_REWARD", &["card"]);

        assert_eq!(cmd.as_deref(), Some("PROCEED"));
    }

    #[test]
    fn blocked_potion_replacement_command_discards_worst_slot_for_better_reward() {
        use crate::content::potions::{Potion, PotionId};

        let root = serde_json::json!({
            "available_commands": ["potion", "proceed"],
            "game_state": {
                "screen_state": {
                    "rewards": [
                        {
                            "reward_type": "POTION",
                            "claimable": false,
                            "blocked_reason": "potion_slots_full",
                            "can_discard": true,
                            "potion": { "id": "PowerPotion" }
                        }
                    ]
                }
            }
        });
        let agent = crate::bot::agent::Agent::new();
        let mut rs = crate::state::run::RunState::new(17, 0, false, "Ironclad");
        rs.potions = vec![
            Some(Potion::new(PotionId::FirePotion, 1)),
            Some(Potion::new(PotionId::DexterityPotion, 2)),
            Some(Potion::new(PotionId::GhostInAJar, 3)),
        ];

        let command = blocked_potion_replacement_command(&agent, &root, &rs);

        assert_eq!(command.as_deref(), Some("POTION DISCARD 0"));
    }

    #[test]
    fn blocked_potion_replacement_command_keeps_inventory_when_reward_is_worse() {
        use crate::content::potions::{Potion, PotionId};

        let root = serde_json::json!({
            "available_commands": ["potion", "proceed"],
            "game_state": {
                "screen_state": {
                    "rewards": [
                        {
                            "reward_type": "POTION",
                            "claimable": false,
                            "blocked_reason": "potion_slots_full",
                            "can_discard": true,
                            "potion": { "id": "Fire Potion" }
                        }
                    ]
                }
            }
        });
        let agent = crate::bot::agent::Agent::new();
        let mut rs = crate::state::run::RunState::new(17, 0, false, "Ironclad");
        rs.potions = vec![
            Some(Potion::new(PotionId::PowerPotion, 1)),
            Some(Potion::new(PotionId::DexterityPotion, 2)),
            Some(Potion::new(PotionId::GhostInAJar, 3)),
        ];

        let command = blocked_potion_replacement_command(&agent, &root, &rs);

        assert_eq!(command, None);
    }

    #[test]
    fn live_grid_purge_uses_agent_deck_cut_and_keeps_bash() {
        let root = serde_json::json!({
            "available_commands": ["choose", "cancel"],
            "game_state": {
                "screen_type": "GRID",
                "screen_state": {
                    "cards": [
                        {"id": "Strike_R", "uuid": "c0", "upgrades": 0},
                        {"id": "Strike_R", "uuid": "c1", "upgrades": 0},
                        {"id": "Strike_R", "uuid": "c2", "upgrades": 0},
                        {"id": "Strike_R", "uuid": "c3", "upgrades": 0},
                        {"id": "Strike_R", "uuid": "c4", "upgrades": 0},
                        {"id": "Defend_R", "uuid": "c5", "upgrades": 0},
                        {"id": "Defend_R", "uuid": "c6", "upgrades": 0},
                        {"id": "Defend_R", "uuid": "c7", "upgrades": 0},
                        {"id": "Defend_R", "uuid": "c8", "upgrades": 0},
                        {"id": "Bash", "uuid": "c9", "upgrades": 0}
                    ],
                    "selected_cards": [],
                    "for_transform": false,
                    "for_upgrade": false,
                    "for_purge": true,
                    "num_cards": 1
                },
                "class": "IRONCLAD",
                "act": 1,
                "floor": 0,
                "current_hp": 80,
                "max_hp": 80,
                "gold": 99,
                "deck": [
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Defend_R", "upgrades": 0},
                    {"id": "Defend_R", "upgrades": 0},
                    {"id": "Defend_R", "upgrades": 0},
                    {"id": "Defend_R", "upgrades": 0},
                    {"id": "Bash", "upgrades": 0}
                ]
            }
        });

        let mut agent = crate::bot::agent::Agent::new();
        let cmd = decide_noncombat_with_agent(
            &mut agent,
            &root,
            "GRID",
            &[
                "strike", "strike", "strike", "strike", "strike", "defend", "defend", "defend",
                "defend", "bash",
            ],
        );

        assert_eq!(cmd.as_deref(), Some("CHOOSE 0"));
    }

    #[test]
    fn live_grid_upgrade_uses_agent_upgrade_targeting() {
        let root = serde_json::json!({
            "available_commands": ["choose", "cancel"],
            "game_state": {
                "screen_type": "GRID",
                "screen_state": {
                    "cards": [
                        {"id": "Strike_R", "uuid": "c0", "upgrades": 0},
                        {"id": "Strike_R", "uuid": "c1", "upgrades": 0},
                        {"id": "Strike_R", "uuid": "c2", "upgrades": 0},
                        {"id": "Strike_R", "uuid": "c3", "upgrades": 0},
                        {"id": "Strike_R", "uuid": "c4", "upgrades": 0},
                        {"id": "Defend_R", "uuid": "c5", "upgrades": 0},
                        {"id": "Defend_R", "uuid": "c6", "upgrades": 0},
                        {"id": "Defend_R", "uuid": "c7", "upgrades": 0},
                        {"id": "Defend_R", "uuid": "c8", "upgrades": 0},
                        {"id": "Bash", "uuid": "c9", "upgrades": 0}
                    ],
                    "selected_cards": [],
                    "for_transform": false,
                    "for_upgrade": true,
                    "for_purge": false,
                    "num_cards": 1
                },
                "class": "IRONCLAD",
                "act": 1,
                "floor": 6,
                "current_hp": 60,
                "max_hp": 80,
                "gold": 99,
                "deck": [
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Strike_R", "upgrades": 0},
                    {"id": "Defend_R", "upgrades": 0},
                    {"id": "Defend_R", "upgrades": 0},
                    {"id": "Defend_R", "upgrades": 0},
                    {"id": "Defend_R", "upgrades": 0},
                    {"id": "Bash", "upgrades": 0}
                ]
            }
        });

        let mut agent = crate::bot::agent::Agent::new();
        let cmd = decide_noncombat_with_agent(
            &mut agent,
            &root,
            "GRID",
            &[
                "strike", "strike", "strike", "strike", "strike", "defend", "defend", "defend",
                "defend", "bash",
            ],
        );

        assert_eq!(cmd.as_deref(), Some("CHOOSE 9"));
    }
}

fn build_live_map_state(gs: &serde_json::Value) -> Option<crate::map::state::MapState> {
    let map_nodes = gs.get("map")?.as_array()?;
    let mut max_y = 0i32;
    for node in map_nodes {
        max_y = max_y.max(node.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32);
    }
    let height = (max_y.max(14) + 1) as usize;
    let mut graph: crate::map::node::Map = (0..height)
        .map(|y| {
            (0..7)
                .map(|x| crate::map::node::MapRoomNode::new(x, y as i32))
                .collect()
        })
        .collect();

    for node in map_nodes {
        let x = node.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        let y = node.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as usize;
        if y >= graph.len() || x >= graph[y].len() {
            continue;
        }
        graph[y][x].class = symbol_to_room_type(node.get("symbol").and_then(|v| v.as_str()));
        if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
            for child in children {
                let dst_x = child.get("x").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                let dst_y = child.get("y").and_then(|v| v.as_i64()).unwrap_or(0) as i32;
                graph[y][x].edges.insert(crate::map::node::MapEdge::new(
                    x as i32, y as i32, dst_x, dst_y,
                ));
                if dst_y >= 0
                    && (dst_y as usize) < graph.len()
                    && dst_x >= 0
                    && (dst_x as usize) < graph[dst_y as usize].len()
                {
                    graph[dst_y as usize][dst_x as usize]
                        .parents
                        .push(crate::map::node::Point::new(x, y));
                }
            }
        }
    }

    let screen_state = gs.get("screen_state");
    let current_node = screen_state.and_then(|v| v.get("current_node"));
    let current_y = current_node
        .and_then(|v| v.get("y"))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(-1);
    let current_x = current_node
        .and_then(|v| v.get("x"))
        .and_then(|v| v.as_i64())
        .map(|v| v as i32)
        .unwrap_or(-1);

    Some(crate::map::state::MapState {
        graph,
        current_y,
        current_x,
        boss_node_available: screen_state
            .and_then(|v| v.get("boss_available"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        has_emerald_key: gs
            .get("keys")
            .and_then(|v| v.get("emerald"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
    })
}

fn symbol_to_room_type(symbol: Option<&str>) -> Option<crate::map::node::RoomType> {
    match symbol.unwrap_or("") {
        "M" => Some(crate::map::node::RoomType::MonsterRoom),
        "E" => Some(crate::map::node::RoomType::MonsterRoomElite),
        "$" => Some(crate::map::node::RoomType::ShopRoom),
        "R" => Some(crate::map::node::RoomType::RestRoom),
        "?" => Some(crate::map::node::RoomType::EventRoom),
        "T" => Some(crate::map::node::RoomType::TreasureRoom),
        _ => None,
    }
}

fn map_choice_x(choice: &str) -> Option<i32> {
    choice
        .strip_prefix("x=")
        .and_then(|value| value.parse::<i32>().ok())
}
