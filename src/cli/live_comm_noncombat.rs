use crate::diff::mapper::{card_id_from_java, java_potion_id_to_rust, relic_id_from_java};
use crate::diff::state_sync::snapshot_uuid;
use crate::state::core::EngineState;

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
    gs: &serde_json::Value,
    screen: &str,
    choice_list: &[&str],
) -> Option<String> {
    let rs = build_live_run_state(gs)?;
    match screen {
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
            let rewards = build_live_reward_state(gs)?;
            let input = agent.decide(&EngineState::RewardScreen(rewards), &rs, &None, false);
            match input {
                crate::state::core::ClientInput::ClaimReward(idx) => {
                    Some(format!("CHOOSE {}", idx))
                }
                crate::state::core::ClientInput::Proceed
                | crate::state::core::ClientInput::Cancel => Some("PROCEED".to_string()),
                _ => None,
            }
        }
        "GRID" => decide_live_grid_screen(gs, &rs),
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
        _ => None,
    }
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

fn build_live_reward_state(gs: &serde_json::Value) -> Option<crate::rewards::state::RewardState> {
    use crate::rewards::state::{RewardItem, RewardState};

    let rewards = gs
        .get("screen_state")
        .and_then(|v| v.get("rewards"))
        .and_then(|v| v.as_array())?;

    let mut state = RewardState::new();
    for reward in rewards {
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

fn decide_live_grid_screen(
    gs: &serde_json::Value,
    rs: &crate::state::run::RunState,
) -> Option<String> {
    let screen_state = gs.get("screen_state")?;
    let can_choose = has_available_command(gs, "choose");
    let can_confirm = has_available_command(gs, "confirm")
        || has_available_command(gs, "proceed");
    let can_cancel = has_available_command(gs, "cancel")
        || has_available_command(gs, "return")
        || has_available_command(gs, "leave");

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
        CardId::Strike => -18,
        CardId::Defend => -22,
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

fn build_live_run_state(gs: &serde_json::Value) -> Option<crate::state::run::RunState> {
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
