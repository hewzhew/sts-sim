use crate::bot::evaluator::{CardEvaluator, DeckProfile};
use crate::cli::live_comm_noncombat::build_live_run_state;
use crate::content::cards::CardId;
use crate::map::node::RoomType;
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardRunProgress {
    pub class_name: String,
    pub act: i32,
    pub floor: i32,
    pub floor_in_act: i32,
    pub ascension_level: i32,
    pub current_hp: i32,
    pub max_hp: i32,
    pub hp_ratio: f32,
    pub gold: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardRoutePressure {
    pub current_room_type: Option<String>,
    pub act_boss: Option<String>,
    pub reachable_shop_nodes_within_3: u32,
    pub reachable_rest_nodes_within_3: u32,
    pub reachable_elite_nodes_within_3: u32,
    pub reachable_event_nodes_within_3: u32,
    pub next_shop_distance: Option<u32>,
    pub next_rest_distance: Option<u32>,
    pub next_elite_distance: Option<u32>,
    pub next_event_distance: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardDeckFeatures {
    pub deck_size: usize,
    pub curse_count: usize,
    pub attack_count: usize,
    pub skill_count: usize,
    pub power_count: usize,
    pub status_count: usize,
    pub upgraded_count: usize,
    pub exhaust_count: usize,
    pub ethereal_count: usize,
    pub card_counts: BTreeMap<String, u32>,
    pub upgraded_card_counts: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardArchetypeFeatures {
    pub archetype_tags: Vec<String>,
    pub strength_enablers: i32,
    pub strength_payoffs: i32,
    pub exhaust_engines: i32,
    pub exhaust_outlets: i32,
    pub exhaust_fodder: i32,
    pub block_core: i32,
    pub block_payoffs: i32,
    pub draw_sources: i32,
    pub power_scalers: i32,
    pub status_generators: i32,
    pub status_payoffs: i32,
    pub searing_blow_count: i32,
    pub searing_blow_upgrades: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardDeficitFeatures {
    pub frontload_damage_gap: i32,
    pub reliable_block_gap: i32,
    pub damage_control_gap: i32,
    pub draw_consistency_gap: i32,
    pub aoe_gap: i32,
    pub scaling_gap: i32,
    pub curse_pressure: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardThreatTags {
    pub boss_is_slime_boss: bool,
    pub boss_is_guardian: bool,
    pub boss_is_hexaghost: bool,
    pub boss_is_champ: bool,
    pub boss_is_collector: bool,
    pub boss_is_bronze_automaton: bool,
    pub boss_is_time_eater: bool,
    pub boss_is_awakened_one: bool,
    pub boss_is_donu_and_deca: bool,
    pub boss_needs_frontload: bool,
    pub boss_needs_block_scaling: bool,
    pub boss_needs_aoe: bool,
    pub boss_punishes_long_fight: bool,
    pub elite_pool_requires_aoe: bool,
    pub elite_pool_punishes_skills: bool,
    pub elite_pool_rewards_setup: bool,
    pub elite_pool_demands_damage_control: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardCandidateCard {
    pub choice_index: usize,
    pub card_id: String,
    pub upgrades: u8,
    pub rarity: Option<String>,
    pub cost: Option<i32>,
    pub card_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardAuditFeatures {
    pub bot_recommended_choice: Option<usize>,
    pub bot_human_agree: Option<bool>,
    pub best_pick_rate: Option<f32>,
    pub best_local_score: Option<i32>,
    pub best_combined_score: Option<f32>,
    pub skip_probability: Option<f32>,
    pub force_pick_in_act1: Option<bool>,
    pub force_pick_for_shell: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardContextFeatures {
    pub relic_ids: Vec<String>,
    pub potion_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RewardSampleQuality {
    pub response_id: i64,
    pub choice_source: String,
    pub supervision_source: String,
    pub quality_weight: f32,
    pub disagreement_weight: f32,
    pub bot_human_agree: Option<bool>,
    pub best_pick_rate: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardChoiceLabel {
    pub choice_kind: String,
    pub choice_index: Option<usize>,
    pub chosen_card_id: Option<String>,
    pub source: String,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardSample {
    pub sample_id: String,
    pub quality: RewardSampleQuality,
    pub run_progress: RewardRunProgress,
    pub route_pressure: RewardRoutePressure,
    pub deck: RewardDeckFeatures,
    pub archetypes: RewardArchetypeFeatures,
    pub deficits: RewardDeficitFeatures,
    pub threats: RewardThreatTags,
    pub context: RewardContextFeatures,
    pub offered_cards: Vec<RewardCandidateCard>,
    pub audit: RewardAuditFeatures,
    pub label: RewardChoiceLabel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RewardChoiceTrainingRow {
    pub sample_id: String,
    pub class_name: String,
    pub choice_index: usize,
    pub card_id: String,
    pub label: i32,
    pub source: String,
    pub quality_weight: f32,
    pub disagreement_weight: f32,
    pub bot_human_agree: Option<bool>,
    pub features: BTreeMap<String, f32>,
}

pub fn load_raw_response_lookup(
    raw_jsonl: &str,
) -> Result<HashMap<i64, Value>, Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(raw_jsonl)?;
    let mut lookup = HashMap::new();
    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let root: Value = serde_json::from_str(line)?;
        let response_id = root
            .get("protocol_meta")
            .and_then(|v| v.get("response_id"))
            .and_then(|v| v.as_i64());
        if let Some(response_id) = response_id {
            lookup.insert(response_id, root);
        }
    }
    Ok(lookup)
}

pub fn reward_sample_from_audit_line(
    audit_line: &str,
    raw_lookup: &HashMap<i64, Value>,
) -> Result<Option<RewardSample>, Box<dyn std::error::Error>> {
    let audit_root: Value = serde_json::from_str(audit_line)?;
    let response_id = match audit_root.get("response_id").and_then(|v| v.as_i64()) {
        Some(id) => id,
        None => return Ok(None),
    };
    let raw_root = match raw_lookup.get(&response_id) {
        Some(root) => root,
        None => return Ok(None),
    };
    let gs = raw_root.get("game_state").unwrap_or(raw_root);

    let run_progress = build_run_progress(&audit_root, gs);
    let live_run_state = build_live_run_state(gs);
    let route_pressure = build_route_pressure(gs, live_run_state.as_ref());
    let deck = build_deck_features(gs.get("deck").and_then(|v| v.as_array()));
    let archetypes = build_archetype_features(live_run_state.as_ref());
    let deficits = build_deficit_features(live_run_state.as_ref(), &run_progress);
    let threats = build_threat_tags(&route_pressure, &run_progress, &archetypes);
    let context = build_context_features(gs);
    let offered_cards = build_offered_cards(&audit_root, gs);
    let audit = build_audit_features(&audit_root);
    let label = build_choice_label(&audit_root);
    let quality = build_quality_features(response_id, &label, &audit);
    let sample_id = label
        .session_id
        .clone()
        .unwrap_or_else(|| format!("reward-{}", response_id));

    Ok(Some(RewardSample {
        sample_id,
        quality,
        run_progress,
        route_pressure,
        deck,
        archetypes,
        deficits,
        threats,
        context,
        offered_cards,
        audit,
        label,
    }))
}

pub fn expand_reward_sample_to_choice_rows(sample: &RewardSample) -> Vec<RewardChoiceTrainingRow> {
    let mut rows = Vec::new();
    for card in &sample.offered_cards {
        rows.push(RewardChoiceTrainingRow {
            sample_id: sample.sample_id.clone(),
            class_name: sample.run_progress.class_name.clone(),
            choice_index: card.choice_index,
            card_id: card.card_id.clone(),
            label: if sample.label.choice_kind == "card"
                && sample.label.choice_index == Some(card.choice_index)
            {
                1
            } else {
                0
            },
            source: sample.quality.choice_source.clone(),
            quality_weight: sample.quality.quality_weight,
            disagreement_weight: sample.quality.disagreement_weight,
            bot_human_agree: sample.quality.bot_human_agree,
            features: build_choice_feature_map(sample, card),
        });
    }

    rows.push(RewardChoiceTrainingRow {
        sample_id: sample.sample_id.clone(),
        class_name: sample.run_progress.class_name.clone(),
        choice_index: sample.offered_cards.len(),
        card_id: "SKIP".to_string(),
        label: if sample.label.choice_kind == "skip" {
            1
        } else {
            0
        },
        source: sample.quality.choice_source.clone(),
        quality_weight: sample.quality.quality_weight,
        disagreement_weight: sample.quality.disagreement_weight,
        bot_human_agree: sample.quality.bot_human_agree,
        features: build_skip_feature_map(sample),
    });

    rows
}

pub fn reward_sample_is_disagreement(sample: &RewardSample) -> bool {
    matches!(sample.quality.bot_human_agree, Some(false))
}

pub fn reward_choice_row_is_disagreement(row: &RewardChoiceTrainingRow) -> bool {
    matches!(row.bot_human_agree, Some(false))
}

fn build_run_progress(audit_root: &Value, gs: &Value) -> RewardRunProgress {
    let act = audit_root
        .get("act")
        .and_then(|v| v.as_i64())
        .or_else(|| gs.get("act").and_then(|v| v.as_i64()))
        .unwrap_or(0) as i32;
    let floor = audit_root
        .get("floor")
        .and_then(|v| v.as_i64())
        .or_else(|| gs.get("floor").and_then(|v| v.as_i64()))
        .unwrap_or(0) as i32;
    let max_hp = audit_root
        .get("max_hp")
        .and_then(|v| v.as_i64())
        .or_else(|| gs.get("max_hp").and_then(|v| v.as_i64()))
        .unwrap_or(0) as i32;
    let current_hp = audit_root
        .get("current_hp")
        .and_then(|v| v.as_i64())
        .or_else(|| gs.get("current_hp").and_then(|v| v.as_i64()))
        .unwrap_or(0) as i32;

    RewardRunProgress {
        class_name: audit_root
            .get("class")
            .and_then(|v| v.as_str())
            .or_else(|| gs.get("class").and_then(|v| v.as_str()))
            .unwrap_or("UNKNOWN")
            .to_string(),
        act,
        floor,
        floor_in_act: floor - ((act - 1).max(0) * 17),
        ascension_level: gs
            .get("ascension_level")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32,
        current_hp,
        max_hp,
        hp_ratio: if max_hp > 0 {
            current_hp as f32 / max_hp as f32
        } else {
            0.0
        },
        gold: audit_root
            .get("gold")
            .and_then(|v| v.as_i64())
            .or_else(|| gs.get("gold").and_then(|v| v.as_i64()))
            .unwrap_or(0) as i32,
    }
}

fn build_route_pressure(gs: &Value, rs: Option<&RunState>) -> RewardRoutePressure {
    let mut pressure = RewardRoutePressure {
        current_room_type: gs
            .get("room_type")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        act_boss: gs
            .get("act_boss")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        ..Default::default()
    };
    let Some(rs) = rs else {
        return pressure;
    };
    let map = &rs.map.graph;
    if map.is_empty() {
        return pressure;
    }

    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();

    if rs.map.current_y < 0 {
        for x in 0..map[0].len() {
            if !map[0][x].edges.is_empty() {
                queue.push_back((x as i32, 0i32, 1u32));
                seen.insert((x as i32, 0i32));
            }
        }
    } else {
        queue.push_back((rs.map.current_x, rs.map.current_y, 0u32));
        seen.insert((rs.map.current_x, rs.map.current_y));
    }

    while let Some((x, y, depth)) = queue.pop_front() {
        if y < 0 || (y as usize) >= map.len() || x < 0 || (x as usize) >= map[y as usize].len() {
            continue;
        }
        let node = &map[y as usize][x as usize];
        let room = node.class;
        if let Some(room) = room {
            if depth <= 3 {
                match room {
                    RoomType::ShopRoom => pressure.reachable_shop_nodes_within_3 += 1,
                    RoomType::RestRoom => pressure.reachable_rest_nodes_within_3 += 1,
                    RoomType::MonsterRoomElite => pressure.reachable_elite_nodes_within_3 += 1,
                    RoomType::EventRoom => pressure.reachable_event_nodes_within_3 += 1,
                    _ => {}
                }
            }
            update_min_distance(&mut pressure, room, depth);
        }

        for edge in &node.edges {
            let next = (edge.dst_x, edge.dst_y);
            if seen.insert(next) {
                queue.push_back((edge.dst_x, edge.dst_y, depth + 1));
            }
        }
    }

    pressure
}

fn update_min_distance(pressure: &mut RewardRoutePressure, room: RoomType, depth: u32) {
    let slot = match room {
        RoomType::ShopRoom => &mut pressure.next_shop_distance,
        RoomType::RestRoom => &mut pressure.next_rest_distance,
        RoomType::MonsterRoomElite => &mut pressure.next_elite_distance,
        RoomType::EventRoom => &mut pressure.next_event_distance,
        _ => return,
    };
    match slot {
        Some(current) if *current <= depth => {}
        _ => *slot = Some(depth),
    }
}

fn build_deck_features(deck_cards: Option<&Vec<Value>>) -> RewardDeckFeatures {
    let mut features = RewardDeckFeatures::default();
    let Some(deck_cards) = deck_cards else {
        return features;
    };

    features.deck_size = deck_cards.len();
    for card in deck_cards {
        let card_id = card
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("UNKNOWN")
            .to_string();
        *features.card_counts.entry(card_id.clone()).or_insert(0) += 1;

        let upgrades = card.get("upgrades").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
        if upgrades > 0 {
            features.upgraded_count += 1;
            *features.upgraded_card_counts.entry(card_id).or_insert(0) += 1;
        }

        match card.get("type").and_then(|v| v.as_str()).unwrap_or("") {
            "ATTACK" => features.attack_count += 1,
            "SKILL" => features.skill_count += 1,
            "POWER" => features.power_count += 1,
            "STATUS" | "CURSE" => features.status_count += 1,
            _ => {}
        }
        if card.get("type").and_then(|v| v.as_str()) == Some("CURSE") {
            features.curse_count += 1;
        }
        if card
            .get("exhausts")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            features.exhaust_count += 1;
        }
        if card
            .get("ethereal")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
        {
            features.ethereal_count += 1;
        }
    }

    features
}

fn build_context_features(gs: &Value) -> RewardContextFeatures {
    let relic_ids = gs
        .get("relics")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|relic| relic.get("id").and_then(|v| v.as_str()))
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    let potion_ids = gs
        .get("potions")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|potion| potion.get("id").and_then(|v| v.as_str()))
                .filter(|id| *id != "Potion Slot")
                .map(|s| s.to_string())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    RewardContextFeatures {
        relic_ids,
        potion_ids,
    }
}

fn build_archetype_features(rs: Option<&RunState>) -> RewardArchetypeFeatures {
    let Some(rs) = rs else {
        return RewardArchetypeFeatures::default();
    };
    let profile = CardEvaluator::deck_profile(rs);
    RewardArchetypeFeatures {
        archetype_tags: CardEvaluator::archetype_tags(&profile),
        strength_enablers: profile.strength_enablers,
        strength_payoffs: profile.strength_payoffs,
        exhaust_engines: profile.exhaust_engines,
        exhaust_outlets: profile.exhaust_outlets,
        exhaust_fodder: profile.exhaust_fodder,
        block_core: profile.block_core,
        block_payoffs: profile.block_payoffs,
        draw_sources: profile.draw_sources,
        power_scalers: profile.power_scalers,
        status_generators: profile.status_generators,
        status_payoffs: profile.status_payoffs,
        searing_blow_count: profile.searing_blow_count,
        searing_blow_upgrades: profile.searing_blow_upgrades,
    }
}

fn build_deficit_features(
    rs: Option<&RunState>,
    progress: &RewardRunProgress,
) -> RewardDeficitFeatures {
    let Some(rs) = rs else {
        return RewardDeficitFeatures::default();
    };
    let profile = CardEvaluator::deck_profile(rs);
    RewardDeficitFeatures {
        frontload_damage_gap: frontload_damage_gap(&profile, progress),
        reliable_block_gap: reliable_block_gap(&profile, progress),
        damage_control_gap: damage_control_gap(&profile, progress),
        draw_consistency_gap: draw_consistency_gap(&profile, rs),
        aoe_gap: aoe_gap(rs, progress),
        scaling_gap: scaling_gap(&profile, progress),
        curse_pressure: curse_pressure(rs),
    }
}

fn build_threat_tags(
    route_pressure: &RewardRoutePressure,
    progress: &RewardRunProgress,
    archetypes: &RewardArchetypeFeatures,
) -> RewardThreatTags {
    let boss = route_pressure.act_boss.as_deref().unwrap_or("");
    let boss_is_slime_boss = boss.eq_ignore_ascii_case("Slime Boss");
    let boss_is_guardian = boss.eq_ignore_ascii_case("The Guardian");
    let boss_is_hexaghost = boss.eq_ignore_ascii_case("Hexaghost");
    let boss_is_champ =
        boss.eq_ignore_ascii_case("Champ") || boss.eq_ignore_ascii_case("The Champ");
    let boss_is_collector = boss.eq_ignore_ascii_case("The Collector");
    let boss_is_bronze_automaton = boss.eq_ignore_ascii_case("Bronze Automaton");
    let boss_is_time_eater = boss.eq_ignore_ascii_case("Time Eater");
    let boss_is_awakened_one = boss.eq_ignore_ascii_case("Awakened One");
    let boss_is_donu_and_deca = boss.eq_ignore_ascii_case("Donu and Deca");

    RewardThreatTags {
        boss_is_slime_boss,
        boss_is_guardian,
        boss_is_hexaghost,
        boss_is_champ,
        boss_is_collector,
        boss_is_bronze_automaton,
        boss_is_time_eater,
        boss_is_awakened_one,
        boss_is_donu_and_deca,
        boss_needs_frontload: boss_is_hexaghost || boss_is_champ,
        boss_needs_block_scaling: boss_is_guardian || boss_is_hexaghost || boss_is_champ,
        boss_needs_aoe: boss_is_slime_boss || boss_is_collector || boss_is_donu_and_deca,
        boss_punishes_long_fight: boss_is_champ || boss_is_awakened_one || boss_is_time_eater,
        elite_pool_requires_aoe: progress.act == 1 || progress.act == 2,
        elite_pool_punishes_skills: progress.act == 1,
        elite_pool_rewards_setup: progress.act == 1
            && !archetypes
                .archetype_tags
                .iter()
                .any(|t| t == "power_scaling"),
        elite_pool_demands_damage_control: progress.act >= 2,
    }
}

fn frontload_damage_gap(profile: &DeckProfile, progress: &RewardRunProgress) -> i32 {
    let frontload =
        profile.attack_count + profile.strength_payoffs + profile.searing_blow_count * 2;
    let target = match progress.act {
        1 => 9,
        2 => 12,
        _ => 14,
    };
    (target - frontload).max(0)
}

fn reliable_block_gap(profile: &DeckProfile, progress: &RewardRunProgress) -> i32 {
    let block = profile.block_core + profile.block_payoffs;
    let hp_pressure = if progress.hp_ratio < 0.45 { 2 } else { 0 };
    (6 + hp_pressure - block).max(0)
}

fn damage_control_gap(profile: &DeckProfile, progress: &RewardRunProgress) -> i32 {
    let control = profile.block_payoffs + profile.power_scalers;
    let target = if progress.act >= 2 { 4 } else { 2 };
    (target - control).max(0)
}

fn draw_consistency_gap(profile: &DeckProfile, rs: &RunState) -> i32 {
    let deck_size_pressure = (rs.master_deck.len() as i32 - 14).max(0) / 3;
    (deck_size_pressure + 2 - profile.draw_sources).max(0)
}

fn aoe_gap(rs: &RunState, progress: &RewardRunProgress) -> i32 {
    let aoe_sources = count_cards(
        rs,
        &[
            CardId::Whirlwind,
            CardId::Cleave,
            CardId::ThunderClap,
            CardId::Immolate,
            CardId::FireBreathing,
            CardId::DaggerSpray,
        ],
    );
    let target = if progress.act == 1 { 1 } else { 2 };
    (target - aoe_sources).max(0)
}

fn scaling_gap(profile: &DeckProfile, progress: &RewardRunProgress) -> i32 {
    let scaling = profile.strength_enablers + profile.power_scalers + profile.searing_blow_count;
    let target = if progress.act >= 2 { 3 } else { 1 };
    (target - scaling).max(0)
}

fn curse_pressure(rs: &RunState) -> i32 {
    rs.master_deck
        .iter()
        .map(|card| crate::bot::evaluator::curse_remove_severity(card.id))
        .sum()
}

fn count_cards(rs: &RunState, ids: &[CardId]) -> i32 {
    rs.master_deck
        .iter()
        .filter(|card| ids.contains(&card.id))
        .count() as i32
}

fn build_offered_cards(audit_root: &Value, gs: &Value) -> Vec<RewardCandidateCard> {
    if let Some(cards) = audit_root.get("offered_cards").and_then(|v| v.as_array()) {
        return cards
            .iter()
            .enumerate()
            .map(|(idx, card)| RewardCandidateCard {
                choice_index: idx,
                card_id: card
                    .get("rust_card_id")
                    .and_then(|v| v.as_str())
                    .or_else(|| card.get("java_id").and_then(|v| v.as_str()))
                    .unwrap_or("UNKNOWN")
                    .to_string(),
                upgrades: card.get("upgrades").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                rarity: None,
                cost: None,
                card_type: None,
            })
            .collect();
    }

    gs.get("screen_state")
        .and_then(|v| v.get("cards"))
        .and_then(|v| v.as_array())
        .map(|cards| {
            cards
                .iter()
                .enumerate()
                .map(|(idx, card)| RewardCandidateCard {
                    choice_index: idx,
                    card_id: card
                        .get("id")
                        .and_then(|v| v.as_str())
                        .unwrap_or("UNKNOWN")
                        .to_string(),
                    upgrades: card.get("upgrades").and_then(|v| v.as_u64()).unwrap_or(0) as u8,
                    rarity: card
                        .get("rarity")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    cost: card.get("cost").and_then(|v| v.as_i64()).map(|v| v as i32),
                    card_type: card
                        .get("type")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn build_audit_features(audit_root: &Value) -> RewardAuditFeatures {
    let eval = audit_root.get("bot_evaluation");
    RewardAuditFeatures {
        bot_recommended_choice: audit_root
            .get("bot_recommended_choice")
            .and_then(|v| v.get("choice_index"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
        bot_human_agree: audit_root.get("bot_human_agree").and_then(|v| v.as_bool()),
        best_pick_rate: eval
            .and_then(|v| v.get("best_pick_rate"))
            .and_then(|v| v.as_f64())
            .map(|v| v as f32),
        best_local_score: eval
            .and_then(|v| v.get("best_local_score"))
            .and_then(|v| v.as_i64())
            .map(|v| v as i32),
        best_combined_score: eval
            .and_then(|v| v.get("best_combined_score"))
            .and_then(|v| v.as_f64())
            .map(|v| v as f32),
        skip_probability: eval
            .and_then(|v| v.get("skip_probability"))
            .and_then(|v| v.as_f64())
            .map(|v| v as f32),
        force_pick_in_act1: eval
            .and_then(|v| v.get("force_pick_in_act1"))
            .and_then(|v| v.as_bool()),
        force_pick_for_shell: eval
            .and_then(|v| v.get("force_pick_for_shell"))
            .and_then(|v| v.as_bool()),
    }
}

fn build_choice_label(audit_root: &Value) -> RewardChoiceLabel {
    let human_choice = audit_root.get("human_choice");
    RewardChoiceLabel {
        choice_kind: human_choice
            .and_then(|v| v.get("choice_kind"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        choice_index: human_choice
            .and_then(|v| v.get("choice_index"))
            .and_then(|v| v.as_u64())
            .map(|v| v as usize),
        chosen_card_id: human_choice
            .and_then(|v| v.get("card_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        source: human_choice
            .and_then(|v| v.get("choice_source"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string(),
        session_id: human_choice
            .and_then(|v| v.get("session_id"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
    }
}

fn build_quality_features(
    response_id: i64,
    label: &RewardChoiceLabel,
    audit: &RewardAuditFeatures,
) -> RewardSampleQuality {
    let supervision_source = match label.source.as_str() {
        "combat_reward" | "boss_reward" | "shop_reward" | "event_reward" => {
            "human_live_choice".to_string()
        }
        "" | "unknown" => "unknown".to_string(),
        other => format!("human_{}", sanitize_feature_key(other)),
    };
    let quality_weight = match label.source.as_str() {
        "combat_reward" | "boss_reward" => 1.0,
        "shop_reward" | "event_reward" => 0.95,
        "" | "unknown" => 0.75,
        _ => 0.9,
    };
    let disagreement_weight = match audit.bot_human_agree {
        Some(false) => 2.0,
        Some(true) => 1.0,
        None => 1.1,
    };
    RewardSampleQuality {
        response_id,
        choice_source: label.source.clone(),
        supervision_source,
        quality_weight,
        disagreement_weight,
        bot_human_agree: audit.bot_human_agree,
        best_pick_rate: audit.best_pick_rate,
    }
}

fn build_choice_feature_map(
    sample: &RewardSample,
    card: &RewardCandidateCard,
) -> BTreeMap<String, f32> {
    let mut features = build_shared_feature_map(sample);
    features.insert("choice.is_skip".to_string(), 0.0);
    features.insert(
        format!("choice.card.{}", sanitize_feature_key(&card.card_id)),
        1.0,
    );
    features.insert("choice.upgrades".to_string(), card.upgrades as f32);
    if let Some(cost) = card.cost {
        features.insert("choice.cost".to_string(), cost as f32);
    }
    if let Some(rarity) = &card.rarity {
        features.insert(
            format!("choice.rarity.{}", sanitize_feature_key(rarity)),
            1.0,
        );
    }
    if let Some(card_type) = &card.card_type {
        features.insert(
            format!("choice.type.{}", sanitize_feature_key(card_type)),
            1.0,
        );
    }
    features
}

fn build_skip_feature_map(sample: &RewardSample) -> BTreeMap<String, f32> {
    let mut features = build_shared_feature_map(sample);
    features.insert("choice.is_skip".to_string(), 1.0);
    features.insert(format!("choice.card.{}", sanitize_feature_key("SKIP")), 1.0);
    features
}

fn build_shared_feature_map(sample: &RewardSample) -> BTreeMap<String, f32> {
    let mut features = BTreeMap::new();

    features.insert("run.act".to_string(), sample.run_progress.act as f32);
    features.insert("run.floor".to_string(), sample.run_progress.floor as f32);
    features.insert(
        "run.floor_in_act".to_string(),
        sample.run_progress.floor_in_act as f32,
    );
    features.insert(
        "run.ascension_level".to_string(),
        sample.run_progress.ascension_level as f32,
    );
    features.insert(
        "run.current_hp".to_string(),
        sample.run_progress.current_hp as f32,
    );
    features.insert("run.max_hp".to_string(), sample.run_progress.max_hp as f32);
    features.insert("run.hp_ratio".to_string(), sample.run_progress.hp_ratio);
    features.insert("run.gold".to_string(), sample.run_progress.gold as f32);

    if let Some(room_type) = &sample.route_pressure.current_room_type {
        features.insert(
            format!("route.current_room.{}", sanitize_feature_key(room_type)),
            1.0,
        );
    }
    if let Some(act_boss) = &sample.route_pressure.act_boss {
        features.insert(
            format!("route.act_boss.{}", sanitize_feature_key(act_boss)),
            1.0,
        );
    }
    features.insert(
        "route.reachable_shop_nodes_within_3".to_string(),
        sample.route_pressure.reachable_shop_nodes_within_3 as f32,
    );
    features.insert(
        "route.reachable_rest_nodes_within_3".to_string(),
        sample.route_pressure.reachable_rest_nodes_within_3 as f32,
    );
    features.insert(
        "route.reachable_elite_nodes_within_3".to_string(),
        sample.route_pressure.reachable_elite_nodes_within_3 as f32,
    );
    features.insert(
        "route.reachable_event_nodes_within_3".to_string(),
        sample.route_pressure.reachable_event_nodes_within_3 as f32,
    );
    features.insert(
        "route.next_shop_distance".to_string(),
        sample.route_pressure.next_shop_distance.unwrap_or(99) as f32,
    );
    features.insert(
        "route.next_rest_distance".to_string(),
        sample.route_pressure.next_rest_distance.unwrap_or(99) as f32,
    );
    features.insert(
        "route.next_elite_distance".to_string(),
        sample.route_pressure.next_elite_distance.unwrap_or(99) as f32,
    );
    features.insert(
        "route.next_event_distance".to_string(),
        sample.route_pressure.next_event_distance.unwrap_or(99) as f32,
    );

    features.insert("deck.size".to_string(), sample.deck.deck_size as f32);
    features.insert(
        "deck.curse_count".to_string(),
        sample.deck.curse_count as f32,
    );
    features.insert(
        "deck.attack_count".to_string(),
        sample.deck.attack_count as f32,
    );
    features.insert(
        "deck.skill_count".to_string(),
        sample.deck.skill_count as f32,
    );
    features.insert(
        "deck.power_count".to_string(),
        sample.deck.power_count as f32,
    );
    features.insert(
        "deck.status_count".to_string(),
        sample.deck.status_count as f32,
    );
    features.insert(
        "deck.upgraded_count".to_string(),
        sample.deck.upgraded_count as f32,
    );
    features.insert(
        "deck.exhaust_count".to_string(),
        sample.deck.exhaust_count as f32,
    );
    features.insert(
        "deck.ethereal_count".to_string(),
        sample.deck.ethereal_count as f32,
    );

    for (card_id, count) in &sample.deck.card_counts {
        features.insert(
            format!("deck.card.{}", sanitize_feature_key(card_id)),
            *count as f32,
        );
    }
    for (card_id, count) in &sample.deck.upgraded_card_counts {
        features.insert(
            format!("deck.card_upgraded.{}", sanitize_feature_key(card_id)),
            *count as f32,
        );
    }

    for tag in &sample.archetypes.archetype_tags {
        features.insert(format!("arch.tag.{}", sanitize_feature_key(tag)), 1.0);
    }
    features.insert(
        "arch.strength_enablers".to_string(),
        sample.archetypes.strength_enablers as f32,
    );
    features.insert(
        "arch.strength_payoffs".to_string(),
        sample.archetypes.strength_payoffs as f32,
    );
    features.insert(
        "arch.exhaust_engines".to_string(),
        sample.archetypes.exhaust_engines as f32,
    );
    features.insert(
        "arch.exhaust_outlets".to_string(),
        sample.archetypes.exhaust_outlets as f32,
    );
    features.insert(
        "arch.exhaust_fodder".to_string(),
        sample.archetypes.exhaust_fodder as f32,
    );
    features.insert(
        "arch.block_core".to_string(),
        sample.archetypes.block_core as f32,
    );
    features.insert(
        "arch.block_payoffs".to_string(),
        sample.archetypes.block_payoffs as f32,
    );
    features.insert(
        "arch.draw_sources".to_string(),
        sample.archetypes.draw_sources as f32,
    );
    features.insert(
        "arch.power_scalers".to_string(),
        sample.archetypes.power_scalers as f32,
    );
    features.insert(
        "arch.status_generators".to_string(),
        sample.archetypes.status_generators as f32,
    );
    features.insert(
        "arch.status_payoffs".to_string(),
        sample.archetypes.status_payoffs as f32,
    );
    features.insert(
        "arch.searing_blow_count".to_string(),
        sample.archetypes.searing_blow_count as f32,
    );
    features.insert(
        "arch.searing_blow_upgrades".to_string(),
        sample.archetypes.searing_blow_upgrades as f32,
    );

    features.insert(
        "deficit.frontload_damage_gap".to_string(),
        sample.deficits.frontload_damage_gap as f32,
    );
    features.insert(
        "deficit.reliable_block_gap".to_string(),
        sample.deficits.reliable_block_gap as f32,
    );
    features.insert(
        "deficit.damage_control_gap".to_string(),
        sample.deficits.damage_control_gap as f32,
    );
    features.insert(
        "deficit.draw_consistency_gap".to_string(),
        sample.deficits.draw_consistency_gap as f32,
    );
    features.insert(
        "deficit.aoe_gap".to_string(),
        sample.deficits.aoe_gap as f32,
    );
    features.insert(
        "deficit.scaling_gap".to_string(),
        sample.deficits.scaling_gap as f32,
    );
    features.insert(
        "deficit.curse_pressure".to_string(),
        sample.deficits.curse_pressure as f32,
    );

    insert_bool_feature(
        &mut features,
        "threat.boss_is_slime_boss",
        sample.threats.boss_is_slime_boss,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_is_guardian",
        sample.threats.boss_is_guardian,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_is_hexaghost",
        sample.threats.boss_is_hexaghost,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_is_champ",
        sample.threats.boss_is_champ,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_is_collector",
        sample.threats.boss_is_collector,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_is_bronze_automaton",
        sample.threats.boss_is_bronze_automaton,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_is_time_eater",
        sample.threats.boss_is_time_eater,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_is_awakened_one",
        sample.threats.boss_is_awakened_one,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_is_donu_and_deca",
        sample.threats.boss_is_donu_and_deca,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_needs_frontload",
        sample.threats.boss_needs_frontload,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_needs_block_scaling",
        sample.threats.boss_needs_block_scaling,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_needs_aoe",
        sample.threats.boss_needs_aoe,
    );
    insert_bool_feature(
        &mut features,
        "threat.boss_punishes_long_fight",
        sample.threats.boss_punishes_long_fight,
    );
    insert_bool_feature(
        &mut features,
        "threat.elite_pool_requires_aoe",
        sample.threats.elite_pool_requires_aoe,
    );
    insert_bool_feature(
        &mut features,
        "threat.elite_pool_punishes_skills",
        sample.threats.elite_pool_punishes_skills,
    );
    insert_bool_feature(
        &mut features,
        "threat.elite_pool_rewards_setup",
        sample.threats.elite_pool_rewards_setup,
    );
    insert_bool_feature(
        &mut features,
        "threat.elite_pool_demands_damage_control",
        sample.threats.elite_pool_demands_damage_control,
    );

    for relic_id in &sample.context.relic_ids {
        features.insert(format!("ctx.relic.{}", sanitize_feature_key(relic_id)), 1.0);
    }
    for potion_id in &sample.context.potion_ids {
        features.insert(
            format!("ctx.potion.{}", sanitize_feature_key(potion_id)),
            1.0,
        );
    }
    features.insert(
        "ctx.potion_count".to_string(),
        sample.context.potion_ids.len() as f32,
    );
    features.insert(
        "sample.quality_weight".to_string(),
        sample.quality.quality_weight,
    );
    features.insert(
        "sample.disagreement_weight".to_string(),
        sample.quality.disagreement_weight,
    );
    if let Some(best_pick_rate) = sample.quality.best_pick_rate {
        features.insert("sample.best_pick_rate".to_string(), best_pick_rate);
    }
    if let Some(agree) = sample.quality.bot_human_agree {
        insert_bool_feature(&mut features, "sample.bot_human_agree", agree);
        insert_bool_feature(&mut features, "sample.bot_human_disagree", !agree);
    }
    features.insert(
        format!(
            "sample.supervision_source.{}",
            sanitize_feature_key(&sample.quality.supervision_source)
        ),
        1.0,
    );

    features
}

fn insert_bool_feature(features: &mut BTreeMap<String, f32>, key: &str, value: bool) {
    if value {
        features.insert(key.to_string(), 1.0);
    }
}

fn sanitize_feature_key(raw: &str) -> String {
    raw.chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' => c.to_ascii_lowercase(),
            _ => '_',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        expand_reward_sample_to_choice_rows, load_raw_response_lookup,
        reward_sample_from_audit_line,
    };

    #[test]
    fn reward_sample_extracts_core_progress_and_boss_context() {
        let raw = r#"{"protocol_meta":{"response_id":19},"game_state":{"class":"IRONCLAD","act":1,"floor":4,"current_hp":73,"max_hp":80,"gold":133,"ascension_level":0,"act_boss":"Slime Boss","room_type":"MonsterRoom","deck":[{"id":"Strike_R","type":"ATTACK","upgrades":0,"exhausts":false,"ethereal":false},{"id":"Inflame","type":"POWER","upgrades":1,"exhausts":false,"ethereal":false},{"id":"Injury","type":"CURSE","upgrades":0,"exhausts":false,"ethereal":false}],"relics":[{"id":"Burning Blood"},{"id":"Vajra"}],"potions":[{"id":"Strength Potion"},{"id":"Potion Slot"}],"map":[{"x":0,"y":5,"symbol":"$"},{"x":0,"y":6,"symbol":"R"},{"x":0,"y":7,"symbol":"E"},{"x":0,"y":8,"symbol":"?"}]}}"#;
        let audit = r#"{"response_id":19,"act":1,"floor":4,"current_hp":73,"max_hp":80,"gold":133,"class":"IRONCLAD","bot_human_agree":false,"offered_cards":[{"rust_card_id":"TwinStrike","upgrades":0},{"rust_card_id":"SwordBoomerang","upgrades":0},{"rust_card_id":"Headbutt","upgrades":0}],"bot_recommended_choice":{"choice_index":0,"kind":"card"},"bot_evaluation":{"best_pick_rate":0.1,"best_local_score":42,"best_combined_score":44.5,"skip_probability":0.3,"force_pick_in_act1":true,"force_pick_for_shell":false},"human_choice":{"choice_kind":"card","choice_index":1,"card_id":"Sword Boomerang","choice_source":"combat_reward","session_id":"reward-1"}}"#;

        let temp = std::env::temp_dir().join("reward_sample_raw_test.jsonl");
        std::fs::write(&temp, format!("{raw}\n")).unwrap();
        let lookup = load_raw_response_lookup(temp.to_str().unwrap()).unwrap();
        let sample = reward_sample_from_audit_line(audit, &lookup)
            .unwrap()
            .expect("sample");

        assert_eq!(sample.run_progress.floor, 4);
        assert_eq!(
            sample.route_pressure.act_boss.as_deref(),
            Some("Slime Boss")
        );
        assert_eq!(sample.deck.deck_size, 3);
        assert_eq!(sample.deck.curse_count, 1);
        assert_eq!(sample.context.relic_ids, vec!["Burning Blood", "Vajra"]);
        assert_eq!(sample.context.potion_ids, vec!["Strength Potion"]);
        assert_eq!(sample.offered_cards.len(), 3);
        assert_eq!(sample.quality.choice_source, "combat_reward");
        assert_eq!(sample.quality.quality_weight, 1.0);
        assert_eq!(sample.quality.disagreement_weight, 2.0);
    }

    #[test]
    fn reward_choice_rows_expand_to_trainable_choice_examples() {
        let raw = r#"{"protocol_meta":{"response_id":19},"game_state":{"class":"IRONCLAD","act":1,"floor":4,"current_hp":73,"max_hp":80,"gold":133,"ascension_level":0,"act_boss":"Slime Boss","room_type":"MonsterRoom","deck":[{"id":"Strike_R","type":"ATTACK","upgrades":0,"exhausts":false,"ethereal":false},{"id":"Inflame","type":"POWER","upgrades":1,"exhausts":false,"ethereal":false},{"id":"Injury","type":"CURSE","upgrades":0,"exhausts":false,"ethereal":false}],"relics":[{"id":"Burning Blood"},{"id":"Vajra"}],"potions":[{"id":"Strength Potion"},{"id":"Potion Slot"}],"map":[{"x":0,"y":5,"symbol":"$"},{"x":0,"y":6,"symbol":"R"},{"x":0,"y":7,"symbol":"E"},{"x":0,"y":8,"symbol":"?"}]}}"#;
        let audit = r#"{"response_id":19,"act":1,"floor":4,"current_hp":73,"max_hp":80,"gold":133,"class":"IRONCLAD","bot_human_agree":false,"offered_cards":[{"rust_card_id":"TwinStrike","upgrades":0},{"rust_card_id":"SwordBoomerang","upgrades":0},{"rust_card_id":"Headbutt","upgrades":0}],"bot_recommended_choice":{"choice_index":0,"kind":"card"},"bot_evaluation":{"best_pick_rate":0.1,"best_local_score":42,"best_combined_score":44.5,"skip_probability":0.3,"force_pick_in_act1":true,"force_pick_for_shell":false},"human_choice":{"choice_kind":"card","choice_index":1,"card_id":"Sword Boomerang","choice_source":"combat_reward","session_id":"reward-1"}}"#;

        let temp = std::env::temp_dir().join("reward_choice_rows_raw_test.jsonl");
        std::fs::write(&temp, format!("{raw}\n")).unwrap();
        let lookup = load_raw_response_lookup(temp.to_str().unwrap()).unwrap();
        let sample = reward_sample_from_audit_line(audit, &lookup)
            .unwrap()
            .expect("sample");
        let rows = expand_reward_sample_to_choice_rows(&sample);

        assert_eq!(rows.len(), 4);
        assert_eq!(rows.iter().filter(|row| row.label == 1).count(), 1);
        let chosen = rows
            .iter()
            .find(|row| row.card_id == "SwordBoomerang")
            .expect("SwordBoomerang row");
        assert_eq!(chosen.label, 1);
        assert_eq!(chosen.features.get("run.act"), Some(&1.0));
        assert_eq!(chosen.features.get("threat.boss_is_slime_boss"), Some(&1.0));
        assert_eq!(
            chosen.features.get("choice.card.swordboomerang"),
            Some(&1.0)
        );
        assert_eq!(chosen.features.get("choice.index"), None);
        assert_eq!(chosen.source, "combat_reward");
        assert_eq!(chosen.quality_weight, 1.0);
        assert_eq!(chosen.disagreement_weight, 2.0);
        let skip = rows
            .iter()
            .find(|row| row.card_id == "SKIP")
            .expect("skip row");
        assert_eq!(skip.features.get("choice.is_skip"), Some(&1.0));
    }
}
