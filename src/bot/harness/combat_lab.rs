use std::collections::BTreeMap;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::bot::evaluator::evaluate_state;
use crate::combat::{CombatCard, CombatState, Intent, Power};
use crate::content::cards::get_card_definition;
use crate::diff::replay::replay_support::tick_until_stable;
use crate::rng::{shuffle_with_random_long, StsRng};
use crate::state::core::{ClientInput, EngineState, RunResult};
use crate::state::run::RunState;
use crate::testing::fixtures::scenario::{initialize_fixture_state, ScenarioFixture};
use crate::bot::harness::combat_policy::{
    decide_policy_action, extract_state_features, flag_bad_action_tags, EvalMetrics,
    PolicyDecision, PolicyKind,
};

const DEFAULT_ACTION_CAP: usize = 256;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LabVariantMode {
    Exact,
    ReshuffleDraw,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeOutcome {
    Victory,
    Defeat,
    StepCap,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatLabMonsterSnapshot {
    pub slot: usize,
    pub id: String,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub intent: String,
    pub intent_damage: i32,
    pub key_powers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatLabStepTrace {
    pub step_index: usize,
    pub turn_index: u32,
    pub player_hp_before: i32,
    pub player_block_before: i32,
    pub energy_before: u8,
    pub hand_before: Vec<String>,
    pub hand_size_before: usize,
    pub draw_size_before: usize,
    pub discard_size_before: usize,
    pub monsters_before: Vec<CombatLabMonsterSnapshot>,
    pub evaluate_state_before: f32,
    pub state_features_preview: BTreeMap<String, f32>,
    pub state_features_full: BTreeMap<String, f32>,
    pub policy_decision: PolicyDecision,
    pub bad_action_tags: Vec<String>,
    pub chosen_action: String,
    pub action_kind: String,
    pub action_payload: serde_json::Value,
    pub player_hp_after: i32,
    pub player_block_after: i32,
    pub energy_after: u8,
    pub hand_after: Vec<String>,
    pub hand_size_after: usize,
    pub draw_size_after: usize,
    pub discard_size_after: usize,
    pub monsters_after: Vec<CombatLabMonsterSnapshot>,
    pub evaluate_state_after: f32,
    pub remaining_monster_hp_after_step: i32,
    pub remaining_player_hp_after_step: i32,
    pub episode_outcome: Option<EpisodeOutcome>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatLabEpisodeTrace {
    pub fixture_name: String,
    pub episode_id: usize,
    pub variant_mode: LabVariantMode,
    pub seed: u64,
    pub policy: PolicyKind,
    pub depth: u32,
    pub outcome: EpisodeOutcome,
    pub final_player_hp: i32,
    pub final_monster_hp: i32,
    pub turns: u32,
    pub path_score: f32,
    pub steps: Vec<CombatLabStepTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatLabEpisodeRecord {
    pub episode_id: usize,
    pub variant_mode: LabVariantMode,
    pub seed: u64,
    pub won: bool,
    pub final_player_hp: i32,
    pub final_monster_hp: i32,
    pub turns: u32,
    pub path_score: f32,
    pub trace_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatLabSummary {
    pub fixture_name: String,
    pub policy: PolicyKind,
    pub total_episodes: usize,
    pub wins: usize,
    pub win_rate: f32,
    pub average_final_hp: f32,
    pub metrics: EvalMetrics,
    pub best_win_episode_id: Option<usize>,
    pub best_attempt_episode_id: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct CombatLabConfig {
    pub fixture: ScenarioFixture,
    pub episodes: usize,
    pub policy: PolicyKind,
    pub depth: u32,
    pub variant_mode: LabVariantMode,
    pub base_seed: u64,
    pub out_dir: PathBuf,
}

pub fn sanitize_fixture_for_local_lab(fixture: &ScenarioFixture) -> ScenarioFixture {
    let mut sanitized = fixture.clone();
    if !sanitized.name.ends_with("_start") {
        sanitized.name = format!("{}_start", sanitized.name);
    }
    sanitized.steps.clear();
    sanitized.assertions.clear();
    sanitized.tags.push("local_lab_start".to_string());
    sanitized.tags.sort();
    sanitized.tags.dedup();

    let mut provenance = sanitized.provenance.take().unwrap_or_default();
    provenance.notes.push("sanitized_for_local_lab".to_string());
    provenance.notes.sort();
    provenance.notes.dedup();
    sanitized.provenance = Some(provenance);
    sanitized
}

pub fn write_sanitized_fixture_for_local_lab(
    fixture: &ScenarioFixture,
    out_path: &Path,
) -> Result<ScenarioFixture, String> {
    let sanitized = sanitize_fixture_for_local_lab(fixture);
    let payload = serde_json::to_string_pretty(&sanitized).map_err(|err| err.to_string())?;
    std::fs::write(out_path, payload).map_err(|err| err.to_string())?;
    Ok(sanitized)
}

pub fn run_combat_lab(config: &CombatLabConfig) -> Result<CombatLabSummary, String> {
    std::fs::create_dir_all(&config.out_dir).map_err(|err| err.to_string())?;
    let fixture = sanitize_fixture_for_local_lab(&config.fixture);
    let mut records = Vec::new();
    let mut traces = Vec::new();

    for episode_id in 0..config.episodes {
        let episode_seed = config.base_seed.wrapping_add(episode_id as u64);
        let episode = run_episode(
            &fixture,
            config.policy,
            config.depth,
            config.variant_mode,
            episode_id,
            episode_seed,
        )?;
        let trace = episode.trace;
        let trace_name = format!("trace_{episode_id:04}.json");
        let trace_path = config.out_dir.join(&trace_name);
        let trace_payload = serde_json::to_string_pretty(&trace).map_err(|err| err.to_string())?;
        std::fs::write(&trace_path, trace_payload).map_err(|err| err.to_string())?;

        let record = CombatLabEpisodeRecord {
            episode_id,
            variant_mode: config.variant_mode,
            seed: episode_seed,
            won: trace.outcome == EpisodeOutcome::Victory,
            final_player_hp: trace.final_player_hp,
            final_monster_hp: trace.final_monster_hp,
            turns: trace.turns,
            path_score: trace.path_score,
            trace_path: trace_name,
        };
        records.push(record);
        traces.push(trace);
    }

    let episodes_path = config.out_dir.join("episodes.jsonl");
    let mut episodes_file = File::create(&episodes_path).map_err(|err| err.to_string())?;
    for record in &records {
        writeln!(
            episodes_file,
            "{}",
            serde_json::to_string(record).map_err(|err| err.to_string())?
        )
        .map_err(|err| err.to_string())?;
    }

    let best_win_idx = records
        .iter()
        .enumerate()
        .filter(|(_, record)| record.won)
        .max_by(|(_, left), (_, right)| compare_episode_records(left, right))
        .map(|(idx, _)| idx);
    let best_attempt_idx = records
        .iter()
        .enumerate()
        .max_by(|(_, left), (_, right)| compare_episode_records(left, right))
        .map(|(idx, _)| idx);

    if let Some(best_idx) = best_win_idx {
        let markdown = render_trace_markdown(&traces[best_idx]);
        std::fs::write(config.out_dir.join("best_win_trace.md"), markdown)
            .map_err(|err| err.to_string())?;
    } else if let Some(best_idx) = best_attempt_idx {
        let markdown = render_trace_markdown(&traces[best_idx]);
        std::fs::write(config.out_dir.join("best_attempt_trace.md"), markdown)
            .map_err(|err| err.to_string())?;
    }

    let wins = records.iter().filter(|record| record.won).count();
    let average_final_hp = if records.is_empty() {
        0.0
    } else {
        records
            .iter()
            .map(|record| record.final_player_hp as f32)
            .sum::<f32>()
            / records.len() as f32
    };
    let summary = CombatLabSummary {
        fixture_name: fixture.name.clone(),
        policy: config.policy,
        total_episodes: records.len(),
        wins,
        win_rate: if records.is_empty() {
            0.0
        } else {
            wins as f32 / records.len() as f32
        },
        average_final_hp,
        metrics: aggregate_metrics(&traces),
        best_win_episode_id: best_win_idx.map(|idx| records[idx].episode_id),
        best_attempt_episode_id: best_attempt_idx.map(|idx| records[idx].episode_id),
    };
    let summary_payload = serde_json::to_string_pretty(&summary).map_err(|err| err.to_string())?;
    std::fs::write(config.out_dir.join("summary.json"), summary_payload)
        .map_err(|err| err.to_string())?;

    Ok(summary)
}

struct EpisodeArtifacts {
    trace: CombatLabEpisodeTrace,
}

fn run_episode(
    fixture: &ScenarioFixture,
    policy: PolicyKind,
    depth: u32,
    variant_mode: LabVariantMode,
    episode_id: usize,
    episode_seed: u64,
) -> Result<EpisodeArtifacts, String> {
    let initial = initialize_fixture_state(fixture);
    let mut combat = initial.combat;
    let mut engine_state = initial.engine_state;
    apply_variant(&mut combat, variant_mode, episode_seed);
    let mut run_state = build_lab_run_state(fixture, &combat);
    let mut agent = crate::bot::agent::Agent::new();

    let mut steps = Vec::new();
    let mut safety_counter = 0usize;
    loop {
        let outcome = current_outcome(&engine_state, &combat);
        if let Some(outcome) = outcome {
            let mut trace = build_episode_trace(
                fixture,
                episode_id,
                variant_mode,
                episode_seed,
                policy,
                depth,
                outcome,
                &combat,
                &steps,
                &engine_state,
            );
            for step in &mut trace.steps {
                step.episode_outcome = Some(trace.outcome.clone());
            }
            return Ok(EpisodeArtifacts { trace });
        }

        if safety_counter >= DEFAULT_ACTION_CAP {
            let mut trace = build_episode_trace(
                fixture,
                episode_id,
                variant_mode,
                episode_seed,
                policy,
                depth,
                EpisodeOutcome::StepCap,
                &combat,
                &steps,
                &engine_state,
            );
            for step in &mut trace.steps {
                step.episode_outcome = Some(trace.outcome.clone());
            }
            return Ok(EpisodeArtifacts { trace });
        }

        let before_score = evaluate_state(&engine_state, &combat);
        let policy_decision = decide_policy_action(
            policy,
            &engine_state,
            &combat,
            &run_state,
            &mut agent,
            depth,
        );
        let step_trace = execute_step(
            steps.len(),
            &mut engine_state,
            &mut combat,
            &policy_decision,
            before_score,
        )?;
        steps.push(step_trace);
        safety_counter += 1;

        // Future-proofing: if Agent ever starts reading RunState in combat, keep core HP/gold synced.
        run_state.current_hp = combat.entities.player.current_hp;
        run_state.max_hp = combat.entities.player.max_hp;
        run_state.gold = combat.entities.player.gold;
    }
}

fn build_episode_trace(
    fixture: &ScenarioFixture,
    episode_id: usize,
    variant_mode: LabVariantMode,
    episode_seed: u64,
    policy: PolicyKind,
    depth: u32,
    outcome: EpisodeOutcome,
    combat: &CombatState,
    steps: &[CombatLabStepTrace],
    engine_state: &EngineState,
) -> CombatLabEpisodeTrace {
    CombatLabEpisodeTrace {
        fixture_name: fixture.name.clone(),
        episode_id,
        variant_mode,
        seed: episode_seed,
        policy,
        depth,
        outcome,
        final_player_hp: combat.entities.player.current_hp,
        final_monster_hp: total_remaining_monster_hp(combat),
        turns: combat.turn.turn_count,
        path_score: evaluate_state(engine_state, combat),
        steps: steps.to_vec(),
    }
}

fn execute_step(
    step_index: usize,
    engine_state: &mut EngineState,
    combat: &mut CombatState,
    policy_decision: &PolicyDecision,
    before_score: f32,
) -> Result<CombatLabStepTrace, String> {
    let turn_index = combat.turn.turn_count;
    let player_hp_before = combat.entities.player.current_hp;
    let player_block_before = combat.entities.player.block;
    let energy_before = combat.turn.energy;
    let hand_before = card_names(&combat.zones.hand);
    let hand_size_before = combat.zones.hand.len();
    let draw_size_before = combat.zones.draw_pile.len();
    let discard_size_before = combat.zones.discard_pile.len();
    let monsters_before = monster_snapshots(combat);
    let state_features_preview = preview_features(combat);
    let state_features_full = extract_state_features(combat);
    let bad_action_tags = flag_bad_action_tags(combat, policy_decision);
    let input = decode_policy_input(policy_decision)?;
    let chosen_action = policy_decision.final_input_debug.clone();
    let (action_kind, action_payload) = action_descriptor(&input, combat);

    let _alive = tick_until_stable(engine_state, combat, input);
    let after_score = evaluate_state(engine_state, combat);

    Ok(CombatLabStepTrace {
        step_index,
        turn_index,
        player_hp_before,
        player_block_before,
        energy_before,
        hand_before,
        hand_size_before,
        draw_size_before,
        discard_size_before,
        monsters_before,
        evaluate_state_before: before_score,
        state_features_preview,
        state_features_full,
        policy_decision: policy_decision.clone(),
        bad_action_tags,
        chosen_action,
        action_kind,
        action_payload,
        player_hp_after: combat.entities.player.current_hp,
        player_block_after: combat.entities.player.block,
        energy_after: combat.turn.energy,
        hand_after: card_names(&combat.zones.hand),
        hand_size_after: combat.zones.hand.len(),
        draw_size_after: combat.zones.draw_pile.len(),
        discard_size_after: combat.zones.discard_pile.len(),
        monsters_after: monster_snapshots(combat),
        evaluate_state_after: after_score,
        remaining_monster_hp_after_step: total_remaining_monster_hp(combat),
        remaining_player_hp_after_step: combat.entities.player.current_hp,
        episode_outcome: None,
    })
}

fn decode_policy_input(policy_decision: &PolicyDecision) -> Result<ClientInput, String> {
    match policy_decision.final_action.kind.as_str() {
        "play_card" => Ok(ClientInput::PlayCard {
            card_index: policy_decision
                .final_action
                .card_index
                .ok_or_else(|| "missing card_index for play_card".to_string())?,
            target: policy_decision.final_action.target,
        }),
        "use_potion" => Ok(ClientInput::UsePotion {
            potion_index: policy_decision
                .final_action
                .potion_index
                .ok_or_else(|| "missing potion_index for use_potion".to_string())?,
            target: policy_decision.final_action.target,
        }),
        "end_turn" => Ok(ClientInput::EndTurn),
        "submit_hand_select" => Ok(ClientInput::SubmitHandSelect(
            policy_decision
                .final_action
                .selected_uuids
                .clone()
                .ok_or_else(|| "missing selected_uuids for submit_hand_select".to_string())?,
        )),
        "submit_grid_select" => Ok(ClientInput::SubmitGridSelect(
            policy_decision
                .final_action
                .selected_uuids
                .clone()
                .ok_or_else(|| "missing selected_uuids for submit_grid_select".to_string())?,
        )),
        "submit_discover_choice" => Ok(ClientInput::SubmitDiscoverChoice(
            policy_decision
                .final_action
                .card_index
                .ok_or_else(|| "missing discover index".to_string())?,
        )),
        "submit_card_choice" => Ok(ClientInput::SubmitCardChoice(
            policy_decision
                .final_action
                .selected_indices
                .clone()
                .ok_or_else(|| "missing selected_indices for submit_card_choice".to_string())?,
        )),
        "proceed" => Ok(ClientInput::Proceed),
        "cancel" => Ok(ClientInput::Cancel),
        other => Err(format!("unsupported policy action kind: {other}")),
    }
}

fn current_outcome(engine_state: &EngineState, combat: &CombatState) -> Option<EpisodeOutcome> {
    match engine_state {
        EngineState::GameOver(RunResult::Victory) => Some(EpisodeOutcome::Victory),
        EngineState::GameOver(RunResult::Defeat) => Some(EpisodeOutcome::Defeat),
        _ => {
            if combat.entities.player.current_hp <= 0 {
                Some(EpisodeOutcome::Defeat)
            } else if combat
                .entities
                .monsters
                .iter()
                .all(|monster| monster.is_dying || monster.is_escaped || monster.current_hp <= 0)
            {
                Some(EpisodeOutcome::Victory)
            } else {
                None
            }
        }
    }
}

fn compare_episode_records(
    left: &CombatLabEpisodeRecord,
    right: &CombatLabEpisodeRecord,
) -> std::cmp::Ordering {
    match (left.won, right.won) {
        (true, false) => std::cmp::Ordering::Greater,
        (false, true) => std::cmp::Ordering::Less,
        (true, true) => left
            .final_player_hp
            .cmp(&right.final_player_hp)
            .then_with(|| right.turns.cmp(&left.turns)),
        (false, false) => right
            .final_monster_hp
            .cmp(&left.final_monster_hp)
            .then_with(|| right.turns.cmp(&left.turns)),
    }
}

fn apply_variant(combat: &mut CombatState, variant_mode: LabVariantMode, seed: u64) {
    match variant_mode {
        LabVariantMode::Exact => {}
        LabVariantMode::ReshuffleDraw => {
            let mut shuffle_rng = StsRng::new(seed);
            shuffle_with_random_long(&mut combat.zones.draw_pile, &mut shuffle_rng);
        }
    }
}

fn build_lab_run_state(fixture: &ScenarioFixture, combat: &CombatState) -> RunState {
    let game_state = &fixture.initial_game_state;
    let seed = game_state
        .get("seed")
        .and_then(|value| value.as_i64().map(|seed| seed as u64))
        .or_else(|| game_state.get("seed").and_then(|value| value.as_u64()))
        .unwrap_or(1);
    let ascension_level = game_state
        .get("ascension_level")
        .and_then(|value| value.as_u64())
        .and_then(|value| u8::try_from(value).ok())
        .unwrap_or(0);
    let player_class = game_state
        .get("class")
        .and_then(|value| value.as_str())
        .map(static_player_class)
        .unwrap_or("Ironclad");
    let mut run_state = RunState::new(seed, ascension_level, false, player_class);
    run_state.current_hp = combat.entities.player.current_hp;
    run_state.max_hp = combat.entities.player.max_hp;
    run_state.gold = combat.entities.player.gold;
    run_state.relics = combat.entities.player.relics.clone();
    run_state.potions = combat.entities.potions.clone();
    run_state.master_deck = all_cards_in_combat(combat);
    run_state
}

fn static_player_class(player_class: &str) -> &'static str {
    match player_class {
        "Silent" => "Silent",
        "Defect" => "Defect",
        "Watcher" => "Watcher",
        _ => "Ironclad",
    }
}

fn all_cards_in_combat(combat: &CombatState) -> Vec<CombatCard> {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .chain(combat.zones.exhaust_pile.iter())
        .chain(combat.zones.limbo.iter())
        .cloned()
        .collect()
}

fn total_remaining_monster_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp)
        .sum()
}

fn card_names(cards: &[CombatCard]) -> Vec<String> {
    cards.iter().map(card_label).collect()
}

fn card_label(card: &CombatCard) -> String {
    let def = get_card_definition(card.id);
    let upgrades = if card.upgrades > 0 {
        format!("+{}", card.upgrades)
    } else {
        String::new()
    };
    format!("{}{}({})", def.name, upgrades, card.get_cost())
}

fn power_labels(powers: &[Power]) -> Vec<String> {
    let mut labels = powers
        .iter()
        .filter(|power| power.amount != 0)
        .map(|power| format!("{:?}={}", power.power_type, power.amount))
        .collect::<Vec<_>>();
    labels.sort();
    labels
}

fn monster_snapshots(combat: &CombatState) -> Vec<CombatLabMonsterSnapshot> {
    combat
        .entities
        .monsters
        .iter()
        .enumerate()
        .filter(|(_, monster)| !monster.is_escaped)
        .map(|(slot, monster)| CombatLabMonsterSnapshot {
            slot,
            id: format!("{:?}", monster.monster_type),
            hp: monster.current_hp,
            max_hp: monster.max_hp,
            block: monster.block,
            intent: intent_label(&monster.current_intent),
            intent_damage: monster.intent_dmg,
            key_powers: power_labels(
                combat
                    .entities
                    .power_db
                    .get(&monster.id)
                    .map(Vec::as_slice)
                    .unwrap_or(&[]),
            ),
        })
        .collect()
}

fn intent_label(intent: &Intent) -> String {
    match intent {
        Intent::Attack { damage, hits } => format!("attack {}x{}", damage, hits),
        Intent::AttackBuff { damage, hits } => format!("attack_buff {}x{}", damage, hits),
        Intent::AttackDebuff { damage, hits } => format!("attack_debuff {}x{}", damage, hits),
        Intent::AttackDefend { damage, hits } => format!("attack_defend {}x{}", damage, hits),
        Intent::Buff => "buff".to_string(),
        Intent::Debuff => "debuff".to_string(),
        Intent::StrongDebuff => "strong_debuff".to_string(),
        Intent::Defend => "defend".to_string(),
        Intent::DefendDebuff => "defend_debuff".to_string(),
        Intent::DefendBuff => "defend_buff".to_string(),
        Intent::Escape => "escape".to_string(),
        Intent::Magic => "magic".to_string(),
        Intent::None => "none".to_string(),
        Intent::Sleep => "sleep".to_string(),
        Intent::Stun => "stun".to_string(),
        Intent::Unknown => "unknown".to_string(),
        Intent::Debug => "debug".to_string(),
    }
}

fn preview_features(combat: &CombatState) -> BTreeMap<String, f32> {
    let mut preview = extract_state_features(combat);
    preview.retain(|key, _| {
        matches!(
            key.as_str(),
            "turn"
                | "player_hp"
                | "player_block"
                | "energy"
                | "hand_size"
                | "draw_size"
                | "discard_size"
                | "remaining_monster_hp"
                | "living_monsters"
                | "incoming_damage"
        )
    });
    preview
}

fn aggregate_metrics(traces: &[CombatLabEpisodeTrace]) -> EvalMetrics {
    let mut action_kind_counts = BTreeMap::new();
    let mut policy_source_counts = BTreeMap::new();
    let mut total_damage_taken = 0i32;
    let mut total_potion_uses = 0usize;
    let mut bad_action_count = 0u32;

    for trace in traces {
        if let Some(first_step) = trace.steps.first() {
            total_damage_taken += (first_step.player_hp_before - trace.final_player_hp).max(0);
        }
        for step in &trace.steps {
            *action_kind_counts
                .entry(step.action_kind.clone())
                .or_insert(0) += 1;
            *policy_source_counts
                .entry(step.policy_decision.source.clone())
                .or_insert(0) += 1;
            if step.action_kind == "use_potion" {
                total_potion_uses += 1;
            }
            bad_action_count += step.bad_action_tags.len() as u32;
        }
    }

    let episodes = traces.len().max(1) as f32;
    EvalMetrics {
        average_damage_taken_per_episode: total_damage_taken as f32 / episodes,
        average_potion_uses_per_episode: total_potion_uses as f32 / episodes,
        bad_action_count,
        action_kind_counts,
        policy_source_counts,
    }
}

fn action_descriptor(input: &ClientInput, combat: &CombatState) -> (String, serde_json::Value) {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat.zones.hand.get(*card_index);
            (
                "play_card".to_string(),
                json!({
                    "card_index": card_index,
                    "target": target,
                    "card": card.map(card_label),
                }),
            )
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => (
            "use_potion".to_string(),
            json!({
                "potion_index": potion_index,
                "target": target,
            }),
        ),
        ClientInput::EndTurn => ("end_turn".to_string(), json!({})),
        ClientInput::Cancel => ("cancel".to_string(), json!({})),
        ClientInput::SubmitDiscoverChoice(index) => (
            "submit_discover_choice".to_string(),
            json!({ "index": index }),
        ),
        ClientInput::SubmitHandSelect(uuids) => {
            ("submit_hand_select".to_string(), json!({ "uuids": uuids }))
        }
        ClientInput::SubmitGridSelect(uuids) => {
            ("submit_grid_select".to_string(), json!({ "uuids": uuids }))
        }
        other => (
            "other".to_string(),
            json!({ "debug": format!("{other:?}") }),
        ),
    }
}

pub fn render_trace_markdown(trace: &CombatLabEpisodeTrace) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Combat Lab Trace\n\n- Fixture: `{}`\n- Episode: `{}`\n- Variant: `{:?}`\n- Seed: `{}`\n- Outcome: `{:?}`\n- Final player HP: `{}`\n- Final monster HP: `{}`\n- Turns: `{}`\n- Path score: `{:.2}`\n\n",
        trace.fixture_name,
        trace.episode_id,
        trace.variant_mode,
        trace.seed,
        trace.outcome,
        trace.final_player_hp,
        trace.final_monster_hp,
        trace.turns,
        trace.path_score
    ));

    for step in &trace.steps {
        out.push_str(&format!(
            "## Turn {} Step {}\n\n",
            step.turn_index, step.step_index
        ));
        out.push_str(&format!(
            "- Before: hp=`{}` block=`{}` energy=`{}` hand_size=`{}` draw=`{}` discard=`{}` eval=`{:.2}`\n",
            step.player_hp_before,
            step.player_block_before,
            step.energy_before,
            step.hand_size_before,
            step.draw_size_before,
            step.discard_size_before,
            step.evaluate_state_before
        ));
        out.push_str(&format!("- Hand before: {}\n", step.hand_before.join(", ")));
        let monster_before = step
            .monsters_before
            .iter()
            .map(|monster| {
                format!(
                    "{} hp={}/{} block={} intent={} powers=[{}]",
                    monster.id,
                    monster.hp,
                    monster.max_hp,
                    monster.block,
                    monster.intent,
                    monster.key_powers.join(", ")
                )
            })
            .collect::<Vec<_>>();
        out.push_str(&format!(
            "- Monsters before: {}\n",
            monster_before.join(" | ")
        ));
        out.push_str(&format!(
            "- Action: `{}` kind=`{}` payload=`{}`\n",
            step.chosen_action, step.action_kind, step.action_payload
        ));
        out.push_str(&format!(
            "- After: hp=`{}` block=`{}` energy=`{}` hand_size=`{}` draw=`{}` discard=`{}` eval=`{:.2}` remaining_monster_hp=`{}`\n",
            step.player_hp_after,
            step.player_block_after,
            step.energy_after,
            step.hand_size_after,
            step.draw_size_after,
            step.discard_size_after,
            step.evaluate_state_after,
            step.remaining_monster_hp_after_step
        ));
        out.push_str(&format!("- Hand after: {}\n\n", step.hand_after.join(", ")));
    }

    out
}
