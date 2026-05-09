use crate::protocol::java::{
    card_id_from_java, java_potion_id_to_rust, relic_id_from_java, snapshot_uuid,
};
use serde_json::Value;

#[derive(Clone, Debug)]
pub(crate) struct LiveEventTrace {
    pub command: String,
    pub summary: String,
    pub detail: String,
    pub audit: Value,
    pub deck_improvement_summary: Option<String>,
}

pub(crate) fn choose_live_event_command_with_trace(
    gs: &serde_json::Value,
    rs: &crate::state::run::RunState,
) -> Option<LiveEventTrace> {
    let screen_state = gs.get("screen_state")?;
    let event_label = screen_state
        .get("event_name")
        .or_else(|| screen_state.get("event_id"))
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .unwrap_or("Event");
    let (option_index, command_index) = screen_state
        .get("options")
        .and_then(Value::as_array)
        .and_then(|options| {
            options
                .iter()
                .enumerate()
                .find(|(_, option)| {
                    !option
                        .get("disabled")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .map(|(idx, option)| {
                    let command_index = option
                        .get("choice_index")
                        .and_then(Value::as_u64)
                        .map(|value| value as usize)
                        .unwrap_or(idx);
                    (idx, command_index)
                })
        })?;
    let protocol_audit = gs
        .get("screen_state")
        .map(|screen_state| {
            crate::engine::event_handler::live_event_protocol_audit(rs, screen_state)
        })
        .unwrap_or(Value::Null);
    let protocol_note = live_event_protocol_note(&protocol_audit);
    let audit = serde_json::json!({
        "family": "protocol_live_event_fallback",
        "option_index": option_index,
        "command_index": command_index,
        "live_event_protocol": protocol_audit,
    });
    Some(LiveEventTrace {
        command: format!("CHOOSE {}", command_index),
        summary: format!(
            "{} | option={}{}",
            event_label,
            option_index,
            protocol_note
                .as_deref()
                .map(|note| format!(" | {}", note))
                .unwrap_or_default()
        ),
        detail: format!(
            "{} | protocol option={} command={}{}",
            event_label,
            option_index,
            command_index,
            protocol_note
                .as_deref()
                .map(|note| format!(" [{}]", note))
                .unwrap_or_default()
        ),
        audit,
        deck_improvement_summary: None,
    })
}

fn live_event_protocol_note(protocol_audit: &Value) -> Option<String> {
    let status = protocol_audit
        .get("rebuild_status")
        .and_then(Value::as_str)
        .unwrap_or("");
    match status {
        "ready" => Some("protocol=structured_live_ready".to_string()),
        "missing_semantics_state" => {
            let missing = protocol_audit
                .get("event_semantics_missing_keys")
                .and_then(Value::as_array)
                .map(|keys| {
                    keys.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            Some(format!(
                "protocol=missing_event_semantics_state({})",
                missing
            ))
        }
        "unknown_event_name" => Some("protocol=unknown_event_name".to_string()),
        "unsupported_event" => Some("protocol=unsupported_event".to_string()),
        "state_decode_failed" => Some("protocol=state_decode_failed".to_string()),
        "option_count_mismatch" => Some("protocol=option_count_mismatch".to_string()),
        "disabled_mismatch" => Some("protocol=disabled_mismatch".to_string()),
        _ => None,
    }
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
    _agent: &mut crate::bot::Agent,
    root: &serde_json::Value,
    screen: &str,
    choice_list: &[&str],
) -> Option<String> {
    let gs = root.get("game_state").unwrap_or(root);
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
        "SHOP_SCREEN" => None,
        "CARD_REWARD" => {
            if has_available_command(root, "skip") {
                Some("SKIP".to_string())
            } else {
                None
            }
        }
        "COMBAT_REWARD" | "MAP" | "BOSS_REWARD" | "REST" => None,
        "GRID" => decide_live_grid_screen(root),
        "EVENT" => {
            let rs = build_live_run_state(gs)?;
            choose_live_event_command_with_trace(gs, &rs).map(|trace| trace.command)
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::choose_live_event_command_with_trace;
    use crate::content::cards::CardId;
    use crate::state::run::RunState;
    use serde_json::{json, Value};

    #[test]
    fn live_event_trace_uses_structured_event_choice_path() {
        let mut rs = RunState::new(2, 0, false, "Ironclad");
        rs.gold = 70;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            91_777,
        ));
        let gs = json!({
            "screen_state": {
                "event_name": "Designer",
                "event_id": "Designer",
                "current_screen": 1,
                "event_semantics_state": {
                    "adjust_upgrades_one": true,
                    "clean_up_removes_cards": true
                },
                "options": [
                    { "text": "x1", "disabled": false, "choice_index": 40 },
                    { "text": "x2", "disabled": false, "choice_index": 41 },
                    { "text": "x3", "disabled": true, "choice_index": 42 },
                    { "text": "x4", "disabled": false, "choice_index": 43 }
                ]
            }
        });

        let trace = choose_live_event_command_with_trace(&gs, &rs).unwrap();
        assert_eq!(trace.command, "CHOOSE 40");
        assert!(trace.summary.contains("Designer"));
        assert_eq!(
            trace.audit.get("family").and_then(Value::as_str),
            Some("protocol_live_event_fallback")
        );
    }

    #[test]
    fn live_event_trace_surfaces_missing_event_semantics_state_keys() {
        let mut rs = RunState::new(3, 0, false, "Ironclad");
        rs.gold = 70;
        rs.master_deck.push(crate::runtime::combat::CombatCard::new(
            CardId::Parasite,
            91_778,
        ));
        let gs = json!({
            "screen_state": {
                "event_name": "Designer",
                "event_id": "Designer",
                "current_screen": 1,
                "options": [
                    { "text": "[Adjust] 40 Gold. Upgrade 1 card.", "disabled": false, "choice_index": 50 },
                    { "text": "[Clean Up] 60 Gold. Remove 1 card.", "disabled": false, "choice_index": 51 },
                    { "text": "[Full Service] 90 Gold. Remove 1 card + upgrade 1 random.", "disabled": true, "choice_index": 52 },
                    { "text": "[Punch] Lose 3 HP.", "disabled": false, "choice_index": 53 }
                ]
            }
        });

        let trace = choose_live_event_command_with_trace(&gs, &rs).unwrap();
        assert!(trace
            .summary
            .contains("protocol=missing_event_semantics_state"));
        let protocol = trace
            .audit
            .get("live_event_protocol")
            .and_then(Value::as_object)
            .unwrap();
        assert_eq!(
            protocol
                .get("event_semantics_required")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert!(protocol
            .get("event_semantics_missing_keys")
            .and_then(Value::as_array)
            .is_some_and(|keys| keys.len() == 2));
    }
}

fn decide_live_grid_screen(root: &serde_json::Value) -> Option<String> {
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

    for (idx, card) in cards.iter().enumerate() {
        let uuid = snapshot_uuid(&card["uuid"], 60_000 + idx as u32);
        if !selected.contains(&uuid) {
            return Some(format!("CHOOSE {}", idx));
        }
    }

    can_confirm.then(|| "CONFIRM".to_string())
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
                    let mut combat_card = crate::runtime::combat::CombatCard::new(id, idx as u32);
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
                    let runtime_state = relic.get("runtime_state").unwrap_or_else(|| {
                        panic!("strict live_comm: relic.runtime_state missing for {:?}", id)
                    });
                    state.counter = runtime_state
                        .get("counter")
                        .and_then(|v| v.as_i64())
                        .unwrap_or_else(|| {
                            panic!(
                                "strict live_comm: relic.runtime_state.counter missing for {:?}",
                                id
                            )
                        }) as i32;
                    state.used_up = runtime_state
                        .get("used_up")
                        .and_then(|v| v.as_bool())
                        .unwrap_or_else(|| {
                            panic!(
                                "strict live_comm: relic.runtime_state.used_up missing for {:?}",
                                id
                            )
                        });
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
