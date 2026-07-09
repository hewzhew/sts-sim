use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use clap::Parser;
use serde::Serialize;
use serde_json::{json, Value};
use sts_simulator::content::cards::{
    get_card_definition, CardId, CardRarity, CardTag, CardTarget, CardType,
};

#[derive(Parser, Debug)]
#[command(
    about = "Export branch path decisions as an RLDS-style episode JSON dataset",
    version
)]
struct Args {
    /// Path to a branch_tiny result.json/frontier.json/path.json file or a capsule/panel directory.
    #[arg(long)]
    input: PathBuf,

    /// Optional capsule summary.json for file input. Directory input reads sibling summary.json files.
    #[arg(long)]
    summary: Option<PathBuf>,

    /// Output JSON path. Writes to stdout when omitted.
    #[arg(long)]
    out: Option<PathBuf>,
}

#[derive(Serialize)]
struct RldsDatasetV0 {
    schema: &'static str,
    format_basis: &'static str,
    input_path: String,
    metadata: DatasetMetadataV0,
    episodes: Vec<RldsEpisodeV0>,
}

#[derive(Serialize)]
struct DatasetMetadataV0 {
    step_fields: [&'static str; 8],
    source_file_count: usize,
    reward_contract: &'static str,
    action_contract: &'static str,
    observation_contract: &'static str,
    action_feature_contract: &'static str,
    observation_feature_contract: &'static str,
    truncation_contract: &'static str,
}

#[derive(Serialize)]
struct RldsEpisodeV0 {
    episode_id: String,
    seed: Option<u64>,
    branch_id: Option<i64>,
    episode_metadata: Value,
    steps: Vec<RldsStepV0>,
}

#[derive(Serialize)]
struct RldsStepV0 {
    is_first: bool,
    is_last: bool,
    is_terminal: bool,
    observation: Value,
    action: Value,
    reward: f64,
    discount: f64,
    step_metadata: Value,
}

#[derive(Serialize)]
struct ActionMetaV0 {
    index: usize,
    rank: Option<i64>,
    label: Option<String>,
    key: Option<Value>,
    features_v0: Value,
    auto_expand: bool,
    inspect_only: Option<String>,
    lane: Option<String>,
    score: Option<i64>,
}

struct InputBundle {
    path: PathBuf,
    input: Value,
    summary: Option<Value>,
}

fn main() -> Result<(), String> {
    let args = Args::parse();
    let bundles = read_input_bundles(&args.input, args.summary.as_ref())?;
    let mut episodes = Vec::new();
    for bundle in &bundles {
        let seed = bundle
            .summary
            .as_ref()
            .and_then(|summary| summary.get("seed"))
            .and_then(Value::as_u64)
            .or_else(|| bundle.input.get("seed").and_then(Value::as_u64));
        let final_outcome = final_outcome_value(bundle.summary.as_ref(), &bundle.input);
        let combat_summary = combat_history_summary(&bundle.input);

        for path in path_values(&bundle.input)? {
            if let Some(episode) = episode_from_path(
                &path,
                seed,
                &bundle.path,
                &bundle.input,
                &final_outcome,
                &combat_summary,
            ) {
                episodes.push(episode);
            }
        }
    }

    let source_file_count = bundles.len();
    let dataset = RldsDatasetV0 {
        schema: "rlds_episode_dataset_v0",
        format_basis: "RLDS-style episode with steps: observation, action, reward, discount, is_first, is_last, is_terminal, step_metadata",
        input_path: args.input.display().to_string(),
        metadata: DatasetMetadataV0 {
            step_fields: [
                "observation",
                "action",
                "reward",
                "discount",
                "is_first",
                "is_last",
                "is_terminal",
                "step_metadata",
            ],
            source_file_count,
            reward_contract: "sparse_terminal_victory_v0: +1 terminal victory, -1 terminal defeat, 0 otherwise",
            action_contract: "action.index is a discrete index into step_metadata.action_candidates_v0 for non-last steps; final step action is null",
            observation_contract: "raw_branch_path_state_json_v0; final step observation is result/frontier state when available",
            action_feature_contract: "action_features_v0: stable observed identity fields such as kind, card/relic/potion/event ids, price, slot, option indexes, and skip/buy/remove/pick flags",
            observation_feature_contract: "observation_features_v0: fixed scalar/list/count facts derived from visible run state; raw observation remains authoritative",
            truncation_contract: "RLDS-style: is_last=true and is_terminal=false means truncated/gap/timeout, not a terminal game result",
        },
        episodes,
    };
    write_json(args.out, &dataset)
}

fn episode_from_path(
    path: &Value,
    seed: Option<u64>,
    input_path: &PathBuf,
    input: &Value,
    final_outcome: &Value,
    combat_summary: &Value,
) -> Option<RldsEpisodeV0> {
    let branch_id = path.get("branch_id").and_then(Value::as_i64);
    let steps = path.get("steps").and_then(Value::as_array)?;
    if steps.is_empty() {
        return None;
    }
    let terminal = final_outcome_is_terminal(final_outcome);
    let final_observation = input
        .get("state")
        .cloned()
        .or_else(|| {
            steps
                .last()
                .and_then(|step| step.get("state_before").cloned())
        })
        .unwrap_or(Value::Null);

    let mut rlds_steps = Vec::with_capacity(steps.len() + 1);
    for (index, step) in steps.iter().enumerate() {
        let observation = step.get("state_before").cloned().unwrap_or(Value::Null);
        let candidates = action_meta(step);
        let action_index = selected_action_index(step, &candidates).unwrap_or(0);
        let group_features = candidate_group_features(&candidates, action_index);
        let selected_action_features = candidates
            .get(action_index)
            .map(|action| action.features_v0.clone())
            .unwrap_or(Value::Null);
        rlds_steps.push(RldsStepV0 {
            is_first: index == 0,
            is_last: false,
            is_terminal: false,
            observation: observation.clone(),
            action: action_value(action_index, step, &candidates),
            reward: 0.0,
            discount: 1.0,
            step_metadata: json!({
                "t": index,
                "source_step_label": string_field(step, "label"),
                "observation_features_v0": observation_features(&observation),
                "candidate_group_features_v0": group_features,
                "action_candidates_v0": candidates,
                "selected_action_features_v0": selected_action_features,
                "action_mask": action_mask(step),
                "branch_expand_mask": branch_expand_mask(step),
                "selected_action_label": selected_action_label(step),
                "immediate_delta": step.get("decision_delta").cloned().unwrap_or(Value::Null),
            }),
        });
    }

    rlds_steps.push(RldsStepV0 {
        is_first: false,
        is_last: true,
        is_terminal: terminal,
        observation: final_observation.clone(),
        action: Value::Null,
        reward: if terminal {
            terminal_reward(final_outcome)
        } else {
            0.0
        },
        discount: 0.0,
        step_metadata: json!({
            "t": steps.len(),
            "observation_features_v0": observation_features(&final_observation),
            "final_outcome": final_outcome,
            "episode_combat_summary": combat_summary,
        }),
    });

    Some(RldsEpisodeV0 {
        episode_id: episode_id(seed, branch_id, input_path),
        seed,
        branch_id,
        episode_metadata: json!({
            "source_input": input_path.display().to_string(),
            "final_outcome": final_outcome,
            "episode_combat_summary": combat_summary,
        }),
        steps: rlds_steps,
    })
}

fn read_json_path(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn read_input_bundles(
    input_path: &Path,
    explicit_summary: Option<&PathBuf>,
) -> Result<Vec<InputBundle>, String> {
    if input_path.is_file() {
        return Ok(vec![InputBundle {
            path: input_path.to_path_buf(),
            input: read_json_path(input_path)?,
            summary: explicit_summary
                .map(|path| read_json_path(path))
                .transpose()?,
        }]);
    }
    if !input_path.is_dir() {
        return Err(format!(
            "input path does not exist: {}",
            input_path.display()
        ));
    }
    if explicit_summary.is_some() {
        return Err("--summary is only supported with file input".to_string());
    }

    let mut source_paths = Vec::new();
    collect_episode_source_paths(input_path, &mut source_paths)?;
    source_paths.sort();
    if source_paths.is_empty() {
        return Err(format!(
            "directory contains no branch_tiny result.json/frontier.json/path.json files: {}",
            input_path.display()
        ));
    }

    source_paths
        .into_iter()
        .map(|path| {
            let summary_path = path.with_file_name("summary.json");
            let summary = summary_path
                .is_file()
                .then(|| read_json_path(&summary_path))
                .transpose()?;
            Ok(InputBundle {
                input: read_json_path(&path)?,
                path,
                summary,
            })
        })
        .collect()
}

fn collect_episode_source_paths(root: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let mut local_result = None;
    let mut local_frontier = None;
    let mut local_path = None;

    for entry in fs::read_dir(root)
        .map_err(|err| format!("failed to read directory {}: {err}", root.display()))?
    {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read directory entry under {}: {err}",
                root.display()
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_episode_source_paths(&path, out)?;
        } else {
            match path.file_name().and_then(|name| name.to_str()) {
                Some("result.json") => local_result = Some(path),
                Some("frontier.json") => local_frontier = Some(path),
                Some("path.json") => local_path = Some(path),
                _ => {}
            }
        }
    }
    if let Some(path) = local_result.or(local_frontier).or(local_path) {
        out.push(path);
    }
    Ok(())
}

fn write_json<T: Serialize>(out: Option<PathBuf>, value: &T) -> Result<(), String> {
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize dataset: {err}"))?;
    if let Some(path) = out {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|err| {
                format!(
                    "failed to create output directory {}: {err}",
                    parent.display()
                )
            })?;
        }
        fs::write(&path, format!("{text}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        eprintln!("wrote RLDS-style episode dataset to {}", path.display());
    } else {
        let mut stdout = std::io::stdout().lock();
        stdout
            .write_all(text.as_bytes())
            .map_err(|err| format!("failed to write dataset: {err}"))?;
        stdout
            .write_all(b"\n")
            .map_err(|err| format!("failed to write dataset newline: {err}"))?;
    }
    Ok(())
}

fn path_values(input: &Value) -> Result<Vec<Value>, String> {
    if let Some(path) = input.get("path") {
        return Ok(vec![path.clone()]);
    }
    if input.get("steps").is_some() {
        return Ok(vec![input.clone()]);
    }
    if let Some(frontier) = input.get("frontier").and_then(Value::as_array) {
        return Ok(frontier
            .iter()
            .filter_map(|branch| {
                let steps = branch.get("path")?.clone();
                Some(json!({
                    "branch_id": branch.get("id").cloned().unwrap_or(Value::Null),
                    "parent_id": branch.get("parent_id").cloned().unwrap_or(Value::Null),
                    "steps": steps,
                    "source": "frontier",
                }))
            })
            .collect());
    }
    if let Some(paths) = input.as_array() {
        return Ok(paths.iter().cloned().collect());
    }
    Err("input is not a result.json, frontier.json, path.json object, or path array".to_string())
}

fn episode_id(seed: Option<u64>, branch_id: Option<i64>, input_path: &PathBuf) -> String {
    match (seed, branch_id) {
        (Some(seed), Some(branch_id)) => format!("seed{seed}_branch{branch_id}"),
        (Some(seed), None) => format!("seed{seed}_branch_unknown"),
        _ => input_path.display().to_string(),
    }
}

fn action_meta(step: &Value) -> Vec<ActionMetaV0> {
    let mut actions = step
        .get("candidate_pool")
        .and_then(Value::as_array)
        .map(|pool| {
            pool.iter()
                .enumerate()
                .map(|(index, candidate)| action_from_candidate(index, candidate))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if actions.is_empty() {
        actions.push(ActionMetaV0 {
            index: 0,
            rank: Some(1),
            label: string_field(step, "label"),
            key: step.get("key").cloned().filter(|value| !value.is_null()),
            features_v0: action_features(
                step.get("key").filter(|value| !value.is_null()),
                string_field(step, "label").as_deref(),
            ),
            auto_expand: true,
            inspect_only: None,
            lane: None,
            score: None,
        });
    }
    actions
}

fn action_from_candidate(index: usize, candidate: &Value) -> ActionMetaV0 {
    let annotation = candidate.get("annotation");
    let label = string_field(candidate, "label");
    let key = candidate
        .get("key")
        .cloned()
        .filter(|value| !value.is_null());
    let features_v0 = action_features(key.as_ref(), label.as_deref());
    ActionMetaV0 {
        index,
        rank: candidate.get("rank").and_then(Value::as_i64),
        label,
        key,
        features_v0,
        auto_expand: candidate
            .get("auto_expand")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        inspect_only: string_field(candidate, "inspect_only"),
        lane: annotation.and_then(|annotation| string_field(annotation, "lane")),
        score: annotation
            .and_then(|annotation| annotation.get("score"))
            .and_then(Value::as_i64),
    }
}

fn selected_action_index(step: &Value, actions: &[ActionMetaV0]) -> Option<usize> {
    step.get("candidate_pool")
        .and_then(Value::as_array)
        .and_then(|pool| {
            pool.iter().position(|candidate| {
                candidate.get("selected").and_then(Value::as_bool) == Some(true)
            })
        })
        .or_else(|| (!actions.is_empty()).then_some(0))
}

fn action_value(action_index: usize, step: &Value, actions: &[ActionMetaV0]) -> Value {
    let selected = actions.get(action_index);
    json!({
        "index": action_index,
        "label": selected.and_then(|action| action.label.clone()).or_else(|| string_field(step, "label")),
        "key": selected.and_then(|action| action.key.clone()).or_else(|| step.get("key").cloned()),
        "features_v0": selected.map(|action| action.features_v0.clone()).unwrap_or_else(|| {
            action_features(step.get("key").filter(|value| !value.is_null()), string_field(step, "label").as_deref())
        }),
    })
}

fn observation_features(state: &Value) -> Value {
    let hp = i64_field(state, "hp");
    let max_hp = i64_field(state, "max_hp");
    let deck = state.get("deck").and_then(Value::as_array);
    let deck_counts = deck_card_counts(deck, false);
    let upgraded_deck_counts = deck_card_counts(deck, true);
    let deck_definition_counts = deck_definition_counts(deck);
    let boundary_label = string_field(state, "boundary");

    json!({
        "schema": "observation_features_v0",
        "act": i64_field(state, "act"),
        "floor": i64_field(state, "floor"),
        "hp": hp,
        "max_hp": max_hp,
        "hp_ratio_bp": ratio_basis_points(hp, max_hp),
        "gold": i64_field(state, "gold"),
        "deck_size": i64_field(state, "deck_size").or_else(|| deck.map(|deck| deck.len() as i64)),
        "deck_card_counts": deck_counts,
        "upgraded_deck_card_counts": upgraded_deck_counts,
        "deck_type_counts": deck_definition_counts.type_counts,
        "deck_rarity_counts": deck_definition_counts.rarity_counts,
        "deck_tag_counts": deck_definition_counts.tag_counts,
        "relic_ids": id_list(state.get("relics")),
        "relic_count": state.get("relics").and_then(Value::as_array).map(|items| items.len()),
        "potion_ids": id_list(state.get("potions")),
        "potion_count": state.get("potions").and_then(Value::as_array).map(|items| items.len()),
        "boss": state.get("boss").or_else(|| state.get("boss_id")).or_else(|| state.get("known_boss")).cloned().unwrap_or(Value::Null),
        "boundary_label": boundary_label,
        "boundary_kind": boundary_label.as_deref().map(normalize_boundary_kind),
    })
}

fn action_features(key: Option<&Value>, label: Option<&str>) -> Value {
    let (kind, body) = action_variant_and_body(key);
    let kind_text = kind.unwrap_or_else(|| "Unknown".to_string());
    let card_id = body.and_then(|body| string_field(body, "card"));
    let card_definition = card_id
        .as_deref()
        .and_then(card_definition_features)
        .unwrap_or(Value::Null);
    let key_price = body.and_then(|body| i64_field(body, "price"));
    let label_price = label.and_then(label_gold_price);
    let price = key_price.or(label_price);
    let price_source = match (key_price, label_price) {
        (Some(_), _) => Some("key"),
        (None, Some(_)) => Some("label"),
        (None, None) => None,
    };
    json!({
        "schema": "action_features_v0",
        "kind": kind_text,
        "label": label,
        "card_id": card_id,
        "card_definition": card_definition,
        "relic_id": body.and_then(|body| string_field(body, "relic")),
        "potion_id": body.and_then(|body| string_field(body, "potion")),
        "event_id": body.and_then(|body| string_field(body, "event_id")),
        "option_index": body.and_then(|body| i64_field(body, "option_index")),
        "reward_item_index": body.and_then(|body| i64_field(body, "reward_item_index")),
        "shop_slot": body.and_then(|body| i64_field(body, "shop_slot")),
        "price": price,
        "price_source": price_source,
        "deck_index": body.and_then(|body| i64_field(body, "deck_index")),
        "upgrades": body.and_then(|body| i64_field(body, "upgrades")),
        "is_skip": action_is_skip(&kind_text),
        "is_buy": kind_text.starts_with("ShopBuy"),
        "is_remove": kind_text.contains("Purge") || kind_text.contains("Remove"),
        "is_pick": kind_text.contains("Pick"),
        "is_leave": kind_text.contains("Leave"),
        "is_event_option": kind_text == "EventOption",
    })
}

fn label_gold_price(label: &str) -> Option<i64> {
    let (_, suffix) = label.rsplit_once('|')?;
    let mut parts = suffix.split_whitespace();
    let amount = parts.next()?.parse::<i64>().ok()?;
    let unit = parts.next()?.to_ascii_lowercase();
    unit.contains("gold").then_some(amount)
}

fn candidate_group_features(candidates: &[ActionMetaV0], selected_index: usize) -> Value {
    let mut kind_counts = BTreeMap::new();
    let mut affordable_count = 0usize;
    let mut auto_expand_count = 0usize;
    let mut inspect_only_count = 0usize;
    let mut price_min: Option<i64> = None;
    let mut price_max: Option<i64> = None;
    let mut price_sum = 0i64;
    let mut price_count = 0usize;

    for candidate in candidates {
        let kind = candidate
            .features_v0
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("Unknown");
        *kind_counts.entry(kind.to_string()).or_insert(0usize) += 1;
        if candidate.auto_expand {
            auto_expand_count += 1;
        }
        if candidate.inspect_only.is_some() {
            inspect_only_count += 1;
        }
        if let Some(price) = candidate.features_v0.get("price").and_then(Value::as_i64) {
            price_min = Some(price_min.map_or(price, |current| current.min(price)));
            price_max = Some(price_max.map_or(price, |current| current.max(price)));
            price_sum += price;
            price_count += 1;
        }
        if candidate.auto_expand
            && candidate
                .features_v0
                .get("is_buy")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            affordable_count += 1;
        }
    }

    json!({
        "schema": "candidate_group_features_v0",
        "candidate_count": candidates.len(),
        "auto_expand_count": auto_expand_count,
        "inspect_only_count": inspect_only_count,
        "kind_counts": kind_counts,
        "has_skip": candidates.iter().any(|candidate| candidate.features_v0.get("is_skip").and_then(Value::as_bool) == Some(true)),
        "has_leave": candidates.iter().any(|candidate| candidate.features_v0.get("is_leave").and_then(Value::as_bool) == Some(true)),
        "has_remove": candidates.iter().any(|candidate| candidate.features_v0.get("is_remove").and_then(Value::as_bool) == Some(true)),
        "has_buy": candidates.iter().any(|candidate| candidate.features_v0.get("is_buy").and_then(Value::as_bool) == Some(true)),
        "has_pick": candidates.iter().any(|candidate| candidate.features_v0.get("is_pick").and_then(Value::as_bool) == Some(true)),
        "affordable_buy_count": affordable_count,
        "price_count": price_count,
        "price_min": price_min,
        "price_max": price_max,
        "price_mean": (price_count > 0).then_some(price_sum as f64 / price_count as f64),
        "selected_index": selected_index,
        "selected_rank": candidates.get(selected_index).and_then(|candidate| candidate.rank),
        "selected_kind": candidates.get(selected_index).and_then(|candidate| candidate.features_v0.get("kind")).cloned().unwrap_or(Value::Null),
    })
}

fn action_variant_and_body(key: Option<&Value>) -> (Option<String>, Option<&Value>) {
    match key {
        Some(Value::String(kind)) => (Some(kind.clone()), None),
        Some(Value::Object(map)) if map.len() == 1 => {
            let (kind, body) = map.iter().next().expect("checked map length");
            (Some(kind.clone()), Some(body))
        }
        Some(other) => (Some(render_json_scalar(other)), None),
        None => (None, None),
    }
}

fn action_is_skip(kind: &str) -> bool {
    kind.contains("Skip") || kind.contains("Leave")
}

struct DeckDefinitionCounts {
    type_counts: BTreeMap<String, usize>,
    rarity_counts: BTreeMap<String, usize>,
    tag_counts: BTreeMap<String, usize>,
}

fn deck_definition_counts(deck: Option<&Vec<Value>>) -> DeckDefinitionCounts {
    let mut counts = DeckDefinitionCounts {
        type_counts: BTreeMap::new(),
        rarity_counts: BTreeMap::new(),
        tag_counts: BTreeMap::new(),
    };
    let Some(deck) = deck else {
        return counts;
    };
    for card in deck {
        let Some(id) = item_id(card).and_then(|id| parse_card_id(&id)) else {
            continue;
        };
        let def = get_card_definition(id);
        *counts
            .type_counts
            .entry(card_type_label(def.card_type).to_string())
            .or_insert(0) += 1;
        *counts
            .rarity_counts
            .entry(card_rarity_label(def.rarity).to_string())
            .or_insert(0) += 1;
        for tag in def.tags {
            *counts
                .tag_counts
                .entry(card_tag_label(*tag).to_string())
                .or_insert(0) += 1;
        }
    }
    counts
}

fn card_definition_features(card_id: &str) -> Option<Value> {
    let id = parse_card_id(card_id)?;
    let def = get_card_definition(id);
    Some(json!({
        "id": card_id,
        "name": def.name,
        "type": card_type_label(def.card_type),
        "rarity": card_rarity_label(def.rarity),
        "target": card_target_label(def.target),
        "cost": def.cost,
        "base_damage": def.base_damage,
        "base_block": def.base_block,
        "base_magic": def.base_magic,
        "is_multi_damage": def.is_multi_damage,
        "exhaust": def.exhaust,
        "ethereal": def.ethereal,
        "innate": def.innate,
        "tags": def.tags.iter().map(|tag| card_tag_label(*tag)).collect::<Vec<_>>(),
        "upgrade_damage": def.upgrade_damage,
        "upgrade_block": def.upgrade_block,
        "upgrade_magic": def.upgrade_magic,
    }))
}

fn parse_card_id(card_id: &str) -> Option<CardId> {
    serde_json::from_value::<CardId>(Value::String(card_id.to_string()))
        .ok()
        .or_else(|| {
            let normalized = match card_id {
                "Thunderclap" => "ThunderClap",
                "Jax" => "JAX",
                _ => return None,
            };
            serde_json::from_value::<CardId>(Value::String(normalized.to_string())).ok()
        })
}

fn card_type_label(card_type: CardType) -> &'static str {
    match card_type {
        CardType::Attack => "Attack",
        CardType::Skill => "Skill",
        CardType::Power => "Power",
        CardType::Status => "Status",
        CardType::Curse => "Curse",
    }
}

fn card_rarity_label(rarity: CardRarity) -> &'static str {
    match rarity {
        CardRarity::Basic => "Basic",
        CardRarity::Common => "Common",
        CardRarity::Uncommon => "Uncommon",
        CardRarity::Rare => "Rare",
        CardRarity::Special => "Special",
        CardRarity::Curse => "Curse",
    }
}

fn card_target_label(target: CardTarget) -> &'static str {
    match target {
        CardTarget::Enemy => "Enemy",
        CardTarget::AllEnemy => "AllEnemy",
        CardTarget::All => "All",
        CardTarget::SelfAndEnemy => "SelfAndEnemy",
        CardTarget::SelfTarget => "SelfTarget",
        CardTarget::None => "None",
    }
}

fn card_tag_label(tag: CardTag) -> &'static str {
    match tag {
        CardTag::Strike => "Strike",
        CardTag::StarterStrike => "StarterStrike",
        CardTag::StarterDefend => "StarterDefend",
        CardTag::Healing => "Healing",
        CardTag::Empty => "Empty",
    }
}

fn action_mask(step: &Value) -> Vec<bool> {
    step.get("candidate_pool")
        .and_then(Value::as_array)
        .map(|pool| {
            pool.iter()
                .map(|candidate| candidate.get("key").is_some_and(|key| !key.is_null()))
                .collect()
        })
        .unwrap_or_else(|| vec![true])
}

fn branch_expand_mask(step: &Value) -> Vec<bool> {
    step.get("candidate_pool")
        .and_then(Value::as_array)
        .map(|pool| {
            pool.iter()
                .map(|candidate| {
                    candidate
                        .get("auto_expand")
                        .and_then(Value::as_bool)
                        .unwrap_or(false)
                })
                .collect()
        })
        .unwrap_or_else(|| vec![true])
}

fn selected_action_label(step: &Value) -> Option<String> {
    step.get("candidate_pool")
        .and_then(Value::as_array)
        .and_then(|pool| {
            pool.iter()
                .find(|candidate| candidate.get("selected").and_then(Value::as_bool) == Some(true))
        })
        .and_then(|candidate| string_field(candidate, "label"))
        .or_else(|| string_field(step, "label"))
}

fn final_outcome_value(summary: Option<&Value>, input: &Value) -> Value {
    let source = summary.unwrap_or(input);
    let status = source.get("status").cloned().unwrap_or(Value::Null);
    json!({
        "status": status,
        "blocker_kind": source.get("blocker_kind").cloned().unwrap_or(Value::Null),
        "reason": source.get("reason").cloned().unwrap_or(Value::Null),
        "subject": source.get("subject").cloned().unwrap_or(Value::Null),
        "combat_case": source.get("combat_case").cloned().unwrap_or(Value::Null),
        "act": source.get("act").cloned().or_else(|| input.get("state").and_then(|state| state.get("act")).cloned()).unwrap_or(Value::Null),
        "floor": source.get("floor").cloned().or_else(|| input.get("state").and_then(|state| state.get("floor")).cloned()).unwrap_or(Value::Null),
        "hp": source.get("hp").cloned().or_else(|| input.get("state").and_then(|state| state.get("hp")).cloned()).unwrap_or(Value::Null),
        "max_hp": source.get("max_hp").cloned().or_else(|| input.get("state").and_then(|state| state.get("max_hp")).cloned()).unwrap_or(Value::Null),
        "gold": source.get("gold").cloned().or_else(|| input.get("state").and_then(|state| state.get("gold")).cloned()).unwrap_or(Value::Null),
        "deck_size": source.get("deck_size").cloned().or_else(|| input.get("state").and_then(|state| state.get("deck_size")).cloned()).unwrap_or(Value::Null),
    })
}

fn final_outcome_is_terminal(final_outcome: &Value) -> bool {
    final_outcome
        .get("status")
        .and_then(|status| {
            status
                .get("kind")
                .or_else(|| status.as_str().map(|_| status))
        })
        .and_then(Value::as_str)
        == Some("terminal")
}

fn terminal_reward(final_outcome: &Value) -> f64 {
    let result = final_outcome
        .get("status")
        .and_then(|status| status.get("result"))
        .and_then(Value::as_str);
    match result {
        Some("victory") => 1.0,
        Some("defeat") => -1.0,
        _ => 0.0,
    }
}

fn combat_history_summary(input: &Value) -> Value {
    let attempts = input
        .get("combat_search_history")
        .and_then(Value::as_array)
        .filter(|history| !history.is_empty())
        .or_else(|| {
            input
                .get("combat_search_attempts")
                .and_then(Value::as_array)
        });
    let Some(attempts) = attempts else {
        return json!({
            "attempt_count": 0,
            "high_hp_loss_threshold": 25,
            "high_hp_loss_attempts": [],
        });
    };
    let high_hp_loss_threshold = 25;
    let high = attempts
        .iter()
        .filter_map(|attempt| {
            let hp_loss = attempt
                .get("best_complete")
                .and_then(|best| best.get("hp_loss"))
                .and_then(Value::as_i64)?;
            (hp_loss >= high_hp_loss_threshold).then(|| {
                json!({
                    "act": attempt.get("act"),
                    "floor": attempt.get("floor"),
                    "combat_kind": attempt.get("combat_kind"),
                    "enemies": attempt.get("enemies"),
                    "lane": attempt.get("lane"),
                    "source": attempt.get("source"),
                    "complete_win_found": attempt.get("complete_win_found"),
                    "hp_loss": hp_loss,
                    "turns": attempt.get("best_complete").and_then(|best| best.get("turns")),
                    "potions_used": attempt.get("best_complete").and_then(|best| best.get("potions_used")),
                })
            })
        })
        .collect::<Vec<_>>();
    json!({
        "attempt_count": attempts.len(),
        "high_hp_loss_threshold": high_hp_loss_threshold,
        "high_hp_loss_attempts": high,
    })
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    match value.get(field)? {
        Value::String(text) => Some(text.clone()),
        Value::Null => None,
        other => Some(render_json_scalar(other)),
    }
}

fn render_json_scalar(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Null => String::new(),
        other => json!(other).to_string(),
    }
}

fn i64_field(value: &Value, field: &str) -> Option<i64> {
    value.get(field).and_then(|field| {
        field
            .as_i64()
            .or_else(|| field.as_u64().and_then(|value| i64::try_from(value).ok()))
    })
}

fn ratio_basis_points(numerator: Option<i64>, denominator: Option<i64>) -> Option<i64> {
    let numerator = numerator?;
    let denominator = denominator?;
    (denominator > 0).then_some((numerator * 10_000) / denominator)
}

fn deck_card_counts(deck: Option<&Vec<Value>>, upgraded_only: bool) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    let Some(deck) = deck else {
        return counts;
    };
    for card in deck {
        if upgraded_only && !card_is_upgraded(card) {
            continue;
        }
        if let Some(id) = item_id(card) {
            *counts.entry(id).or_insert(0) += 1;
        }
    }
    counts
}

fn card_is_upgraded(card: &Value) -> bool {
    card.get("upgraded").and_then(Value::as_bool) == Some(true)
        || card
            .get("upgrades")
            .and_then(|value| {
                value
                    .as_i64()
                    .or_else(|| value.as_u64().and_then(|value| i64::try_from(value).ok()))
            })
            .is_some_and(|upgrades| upgrades > 0)
}

fn id_list(items: Option<&Value>) -> Vec<String> {
    items
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(item_id).collect())
        .unwrap_or_default()
}

fn item_id(item: &Value) -> Option<String> {
    match item {
        Value::String(text) => Some(text.clone()),
        Value::Object(_) => string_field(item, "id"),
        _ => None,
    }
}

fn normalize_boundary_kind(label: &str) -> String {
    let lower = label.to_ascii_lowercase();
    if lower.contains("neow") {
        "neow".to_string()
    } else if lower.contains("shop") {
        "shop".to_string()
    } else if lower.contains("card reward") || lower.contains("reward") {
        "reward".to_string()
    } else if lower.contains("event") {
        "event".to_string()
    } else if lower.contains("boss") {
        "boss".to_string()
    } else if lower.contains("elite") {
        "elite".to_string()
    } else if lower.contains("monster") || lower.contains("combat") {
        "combat".to_string()
    } else if lower.contains("treasure") || lower.contains("chest") {
        "treasure".to_string()
    } else if lower.contains("rest") || lower.contains("campfire") {
        "rest".to_string()
    } else {
        "unknown".to_string()
    }
}
