use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::bot::combat_heuristic;
use crate::combat::{CombatCard, CombatState};
use crate::content::cards::{self, CardType};
use crate::content::powers::{store, PowerId};
use crate::diff::replay::live_comm_replay::{
    mapped_command_to_input, CombatMappedCommand, CombatReconstructedStep,
};
use crate::diff::state_sync::build_combat_state;
use crate::engine::core::tick_until_stable_turn;
use crate::state::core::{ClientInput, EngineState, RunResult};

const DEFAULT_DECISION_DEPTH: usize = 4;
const DEFAULT_TOP_K: usize = 3;
const DEFAULT_BRANCH_CAP: usize = 6;
const FIRST_WINDOW_THREAT_RELIEF_WEIGHT: i32 = 2;
const FIRST_WINDOW_DEFENSE_GAP_WEIGHT: i32 = 2;
const NEXT_PLAYER_WINDOW_DEAD_DRAW_BURDEN_WEIGHT: i32 = 8;
const IMMEDIATE_CONVERSION_COLLATERAL_WEIGHT: i32 = 16;
const LIVE_REPLAY_SOURCE: &str = "live_replay";
const OFFLINE_AUDIT_SEARCH_SOURCE: &str = "offline_audit_search";
const RECONSTRUCTED_LIVE_REPLAY_STATE_SOURCE: &str = "reconstructed_live_replay_state";
const OFFLINE_COUNTERFACTUAL_BRANCH_SEARCH_KIND: &str = "offline_counterfactual_branch_search";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecisionAuditEngineState {
    CombatPlayerTurn,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionAuditFixture {
    pub name: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub target_step_index: Option<usize>,
    #[serde(default)]
    pub before_response_id: Option<u64>,
    #[serde(default)]
    pub before_frame_id: Option<u64>,
    pub engine_state: DecisionAuditEngineState,
    pub chosen_command_text: String,
    pub chosen_command: CombatMappedCommand,
    pub combat_snapshot: Value,
    pub relics: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecisionAuditReport {
    pub name: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub before_frame_id: Option<u64>,
    #[serde(default)]
    pub before_response_id: Option<u64>,
    #[serde(default)]
    pub chosen_first_move: Option<String>,
    pub legal_first_moves: Vec<String>,
    #[serde(default)]
    pub chosen_trajectory: Option<TrajectoryReport>,
    pub first_move_reports: Vec<FirstMoveReport>,
    pub survival_tag_counts: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatPreferenceState {
    pub encounter_names: Vec<String>,
    pub player_hp: i32,
    pub player_max_hp: i32,
    pub player_block: i32,
    pub energy: u8,
    pub incoming: i32,
    pub hand: Vec<String>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
    pub player_powers: Vec<String>,
    pub relic_ids: Vec<String>,
    pub monsters: Vec<CombatPreferenceMonsterState>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatPreferenceMonsterState {
    pub name: String,
    pub hp: i32,
    pub block: i32,
    pub intent: String,
    pub powers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatPreferenceSample {
    pub sample_id: String,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub before_frame_id: Option<u64>,
    #[serde(default)]
    pub before_response_id: Option<u64>,
    pub state_source: String,
    pub chosen_source: String,
    pub preferred_source: String,
    pub preferred_search_kind: String,
    pub chosen_action_observed: bool,
    pub preferred_action_observed: bool,
    pub decision_depth: usize,
    pub top_k: usize,
    pub branch_cap: usize,
    pub chosen_action: String,
    pub preferred_action: String,
    pub chosen_outcome: TrajectoryOutcomeKind,
    pub preferred_outcome: TrajectoryOutcomeKind,
    pub chosen_score: i32,
    pub preferred_score: i32,
    pub score_gap: i32,
    pub chosen_tags: Vec<String>,
    pub preferred_tags: Vec<String>,
    pub preference_kind: String,
    pub state: CombatPreferenceState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FirstMoveReport {
    pub first_move: String,
    pub top_trajectories: Vec<TrajectoryReport>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrajectoryOutcomeKind {
    Survives,
    Dies,
    LethalWin,
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryReport {
    pub first_move: String,
    pub actions: Vec<String>,
    pub steps: Vec<TrajectoryStepReport>,
    pub outcome: TrajectoryOutcomeKind,
    pub score: i32,
    pub score_breakdown: ScoreBreakdown,
    pub final_player_hp: i32,
    pub final_player_block: i32,
    pub final_incoming: i32,
    pub final_monster_hp: Vec<i32>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrajectoryStepReport {
    pub action: String,
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: u8,
    pub incoming: i32,
    pub monster_hp: Vec<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScoreBreakdown {
    pub bucket: String,
    pub combat_cleared: bool,
    pub victory: bool,
    pub timeout: bool,
    pub first_enemy_window_observed: bool,
    pub steps: i32,
    pub baseline_hp: i32,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub baseline_potions: i32,
    pub final_filled_potions: i32,
    pub potions_used: i32,
    pub final_total_monster_hp: i32,
    pub threat_relief_before_first_enemy_window: i32,
    pub defense_gap_at_first_enemy_window: i32,
    pub dead_draw_burden_at_next_player_window: i32,
    pub collateral_exhaust_cost_of_immediate_conversion: i32,
    pub bucket_bonus: i32,
    pub hp_bonus: i32,
    pub threat_relief_bonus: i32,
    pub monster_hp_penalty: i32,
    pub hp_loss_penalty: i32,
    pub potions_used_penalty: i32,
    pub steps_penalty: i32,
    pub defense_gap_penalty: i32,
    pub dead_draw_burden_penalty: i32,
    pub collateral_exhaust_penalty: i32,
    pub total: i32,
}

#[derive(Debug, Clone, Copy)]
pub struct DecisionAuditConfig {
    pub decision_depth: usize,
    pub top_k: usize,
    pub branch_cap: usize,
}

impl Default for DecisionAuditConfig {
    fn default() -> Self {
        Self {
            decision_depth: DEFAULT_DECISION_DEPTH,
            top_k: DEFAULT_TOP_K,
            branch_cap: DEFAULT_BRANCH_CAP,
        }
    }
}

#[derive(Debug, Clone)]
struct TrajectoryStepState {
    action: String,
    player_hp: i32,
    player_block: i32,
    energy: u8,
    incoming: i32,
    monster_hp: Vec<i32>,
    incoming_reduced: bool,
    weak_applied: bool,
    block_gained: bool,
    used_potion: bool,
    setup_greed: bool,
    direct_damage_push: bool,
    ended_turn: bool,
    threat_relief_before_enemy_window: Option<i32>,
    defense_gap_before_enemy_window: Option<i32>,
    dead_draw_burden_at_next_player_window: Option<i32>,
    collateral_exhaust_cost_of_immediate_conversion: Option<i32>,
}

#[derive(Debug, Clone)]
struct TrajectoryCandidate {
    first_move: String,
    actions: Vec<String>,
    steps: Vec<TrajectoryStepState>,
    outcome: TrajectoryOutcomeKind,
    score: i32,
    score_breakdown: ScoreBreakdown,
    final_player_hp: i32,
    final_player_block: i32,
    final_incoming: i32,
    final_monster_hp: Vec<i32>,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct StateSnapshot {
    player_hp: i32,
    player_block: i32,
    energy: u8,
    incoming: i32,
    total_monster_hp: i32,
    monster_hp: [i32; 5],
    weak_total: i32,
    filled_potions: i32,
    cycling_dead_draw_count: i32,
}

#[derive(Debug, Clone, Copy, Default)]
struct LocalWindowTrace {
    first_enemy_window_observed: bool,
    threat_relief_before_first_enemy_window: i32,
    defense_gap_at_first_enemy_window: i32,
    dead_draw_burden_at_next_player_window: i32,
    collateral_exhaust_cost_of_immediate_conversion: i32,
}

pub fn build_fixture_from_reconstructed_step(
    reconstructed: &CombatReconstructedStep,
    source_path: Option<String>,
    name: impl Into<String>,
) -> Result<DecisionAuditFixture, String> {
    let before_root = reconstructed.before_root.clone();
    let combat_snapshot =
        crate::diff::replay::live_comm_replay::build_live_combat_snapshot_from_root(&before_root)?;
    let relics = before_root
        .get("game_state")
        .and_then(|gs| gs.get("relics"))
        .cloned()
        .unwrap_or(Value::Null);
    Ok(DecisionAuditFixture {
        name: name.into(),
        source_path,
        target_step_index: Some(reconstructed.step_index),
        before_response_id: reconstructed.before_response_id,
        before_frame_id: reconstructed.before_state_frame_id,
        engine_state: DecisionAuditEngineState::CombatPlayerTurn,
        chosen_command_text: reconstructed.command_text.clone(),
        chosen_command: reconstructed.mapped_command.clone(),
        combat_snapshot,
        relics,
    })
}

pub fn load_fixture_path(path: &Path) -> Result<DecisionAuditFixture, String> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read decision audit fixture '{}': {err}",
            path.display()
        )
    })?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse decision audit fixture '{}': {err}",
            path.display()
        )
    })
}

pub fn write_fixture_path(fixture: &DecisionAuditFixture, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create decision audit fixture directory '{}': {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(fixture)
        .map_err(|err| format!("failed to serialize decision audit fixture: {err}"))?;
    std::fs::write(path, text).map_err(|err| {
        format!(
            "failed to write decision audit fixture '{}': {err}",
            path.display()
        )
    })
}

pub fn audit_fixture(
    fixture: &DecisionAuditFixture,
    config: DecisionAuditConfig,
) -> Result<DecisionAuditReport, String> {
    let combat = build_combat_state(&fixture.combat_snapshot, &fixture.relics);
    let engine = match fixture.engine_state {
        DecisionAuditEngineState::CombatPlayerTurn => EngineState::CombatPlayerTurn,
    };
    let chosen_input = mapped_command_to_input(&fixture.chosen_command, &combat)?;
    audit_state(
        fixture.name.clone(),
        fixture.source_path.clone(),
        fixture.before_frame_id,
        fixture.before_response_id,
        engine,
        combat,
        Some(chosen_input),
        config,
    )
}

pub fn audit_state(
    name: String,
    source_path: Option<String>,
    before_frame_id: Option<u64>,
    before_response_id: Option<u64>,
    engine: EngineState,
    combat: CombatState,
    chosen_input: Option<ClientInput>,
    config: DecisionAuditConfig,
) -> Result<DecisionAuditReport, String> {
    let legal_moves = crate::bot::search::get_legal_moves(&engine, &combat);
    if legal_moves.is_empty() {
        return Err("decision audit state had no legal moves".into());
    }

    let legal_first_moves = legal_moves
        .iter()
        .map(|input| describe_input(&engine, &combat, input))
        .collect::<Vec<_>>();
    let chosen_first_move = chosen_input
        .as_ref()
        .map(|input| describe_input(&engine, &combat, input));
    let baseline_hp = combat.entities.player.current_hp;
    let baseline_potions = filled_potion_slots(&combat);

    let mut trajectories = Vec::new();
    explore_paths(
        &engine,
        &combat,
        config.decision_depth,
        true,
        config.branch_cap,
        capture_state(&combat),
        baseline_hp,
        baseline_potions,
        &mut Vec::new(),
        &mut trajectories,
    );

    annotate_survival_lines(&mut trajectories, chosen_first_move.as_deref());

    let mut groups = BTreeMap::<String, Vec<TrajectoryCandidate>>::new();
    for trajectory in trajectories {
        groups
            .entry(trajectory.first_move.clone())
            .or_default()
            .push(trajectory);
    }

    let mut first_move_reports = Vec::new();
    let mut chosen_trajectory = None;
    let mut survival_tag_counts = BTreeMap::new();

    for first_move in &legal_first_moves {
        let mut candidates = groups.remove(first_move.as_str()).unwrap_or_default();
        candidates.sort_by(trajectory_sort_cmp);
        if chosen_first_move.as_deref() == Some(first_move.as_str()) {
            chosen_trajectory = candidates.first().cloned().map(candidate_to_report);
        }
        for candidate in candidates
            .iter()
            .filter(|candidate| candidate.tags.iter().any(|tag| tag == "survival_line"))
        {
            for tag in &candidate.tags {
                *survival_tag_counts.entry(tag.clone()).or_insert(0) += 1;
            }
        }
        first_move_reports.push(FirstMoveReport {
            first_move: first_move.clone(),
            top_trajectories: candidates
                .into_iter()
                .take(config.top_k)
                .map(candidate_to_report)
                .collect(),
        });
    }

    Ok(DecisionAuditReport {
        name,
        source_path,
        before_frame_id,
        before_response_id,
        chosen_first_move,
        legal_first_moves,
        chosen_trajectory,
        first_move_reports,
        survival_tag_counts,
    })
}

pub fn render_text_report(report: &DecisionAuditReport) -> String {
    let mut lines = Vec::new();
    lines.push(format!("decision audit: {}", report.name));
    if let Some(source_path) = &report.source_path {
        lines.push(format!("  source={source_path}"));
    }
    lines.push(format!(
        "  before_frame_id={:?} before_response_id={:?}",
        report.before_frame_id, report.before_response_id
    ));
    lines.push(format!(
        "  chosen_first_move={}",
        report.chosen_first_move.as_deref().unwrap_or("<none>")
    ));
    lines.push(format!(
        "  legal_first_moves={}",
        report.legal_first_moves.join(" | ")
    ));
    if !report.survival_tag_counts.is_empty() {
        let summary = report
            .survival_tag_counts
            .iter()
            .map(|(tag, count)| format!("{tag}={count}"))
            .collect::<Vec<_>>()
            .join(", ");
        lines.push(format!("  survival_tag_counts={summary}"));
    }
    for group in &report.first_move_reports {
        lines.push(format!("first_move: {}", group.first_move));
        for (idx, trajectory) in group.top_trajectories.iter().enumerate() {
            lines.push(format!(
                "  [{}] outcome={:?} score={} bucket={} hp={} block={} incoming={} monster_hp={:?} tags={}",
                idx,
                trajectory.outcome,
                trajectory.score,
                trajectory.score_breakdown.bucket,
                trajectory.final_player_hp,
                trajectory.final_player_block,
                trajectory.final_incoming,
                trajectory.final_monster_hp,
                trajectory.tags.join(",")
            ));
            lines.push(format!(
                "      score_breakdown: bucket_bonus={} hp_bonus={} threat_relief_bonus={} monster_hp_penalty={} hp_loss_penalty={} potions_penalty={} steps_penalty={} defense_gap_penalty={} dead_draw_burden_penalty={} collateral_exhaust_penalty={} first_window_observed={} threat_relief_before_window={} defense_gap_at_window={} dead_draw_burden_at_next_player_window={} collateral_exhaust_cost_of_immediate_conversion={} combat_cleared={} victory={} timeout={}",
                trajectory.score_breakdown.bucket_bonus,
                trajectory.score_breakdown.hp_bonus,
                trajectory.score_breakdown.threat_relief_bonus,
                trajectory.score_breakdown.monster_hp_penalty,
                trajectory.score_breakdown.hp_loss_penalty,
                trajectory.score_breakdown.potions_used_penalty,
                trajectory.score_breakdown.steps_penalty,
                trajectory.score_breakdown.defense_gap_penalty,
                trajectory.score_breakdown.dead_draw_burden_penalty,
                trajectory.score_breakdown.collateral_exhaust_penalty,
                trajectory.score_breakdown.first_enemy_window_observed,
                trajectory
                    .score_breakdown
                    .threat_relief_before_first_enemy_window,
                trajectory.score_breakdown.defense_gap_at_first_enemy_window,
                trajectory
                    .score_breakdown
                    .dead_draw_burden_at_next_player_window,
                trajectory
                    .score_breakdown
                    .collateral_exhaust_cost_of_immediate_conversion,
                trajectory.score_breakdown.combat_cleared,
                trajectory.score_breakdown.victory,
                trajectory.score_breakdown.timeout,
            ));
            lines.push(format!("      actions={}", trajectory.actions.join(" -> ")));
        }
    }
    lines.join("\n")
}

pub fn extract_preference_samples(
    fixture: &DecisionAuditFixture,
    report: &DecisionAuditReport,
    config: DecisionAuditConfig,
) -> Result<Vec<CombatPreferenceSample>, String> {
    let combat = build_combat_state(&fixture.combat_snapshot, &fixture.relics);
    let state = build_preference_state(fixture, &combat);
    let Some(chosen_action) = report.chosen_first_move.as_ref() else {
        return Ok(Vec::new());
    };
    let Some(chosen_group) = report
        .first_move_reports
        .iter()
        .find(|group| &group.first_move == chosen_action)
    else {
        return Ok(Vec::new());
    };
    let Some(chosen_best) = chosen_group.top_trajectories.first() else {
        return Ok(Vec::new());
    };

    let mut samples = Vec::new();
    for group in &report.first_move_reports {
        if &group.first_move == chosen_action {
            continue;
        }
        let Some(preferred) = group.top_trajectories.first() else {
            continue;
        };
        let Some(preference_kind) = preference_kind(chosen_best, preferred) else {
            continue;
        };
        samples.push(CombatPreferenceSample {
            sample_id: format!(
                "combat-pref-{}-{}-{}",
                fixture
                    .before_frame_id
                    .map(|id| id.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                sanitize_sample_label(chosen_action),
                sanitize_sample_label(&group.first_move),
            ),
            source_path: fixture.source_path.clone(),
            before_frame_id: fixture.before_frame_id,
            before_response_id: fixture.before_response_id,
            state_source: RECONSTRUCTED_LIVE_REPLAY_STATE_SOURCE.to_string(),
            chosen_source: LIVE_REPLAY_SOURCE.to_string(),
            preferred_source: OFFLINE_AUDIT_SEARCH_SOURCE.to_string(),
            preferred_search_kind: OFFLINE_COUNTERFACTUAL_BRANCH_SEARCH_KIND.to_string(),
            chosen_action_observed: true,
            preferred_action_observed: false,
            decision_depth: config.decision_depth,
            top_k: config.top_k,
            branch_cap: config.branch_cap,
            chosen_action: chosen_action.clone(),
            preferred_action: group.first_move.clone(),
            chosen_outcome: chosen_best.outcome,
            preferred_outcome: preferred.outcome,
            chosen_score: chosen_best.score,
            preferred_score: preferred.score,
            score_gap: preferred.score - chosen_best.score,
            chosen_tags: chosen_best.tags.clone(),
            preferred_tags: preferred.tags.clone(),
            preference_kind,
            state: state.clone(),
        });
    }

    Ok(samples)
}

fn build_preference_state(
    fixture: &DecisionAuditFixture,
    combat: &CombatState,
) -> CombatPreferenceState {
    let player_max_hp = fixture
        .combat_snapshot
        .get("player")
        .and_then(|player| player.get("max_hp"))
        .and_then(Value::as_i64)
        .unwrap_or(combat.entities.player.max_hp as i64) as i32;
    let relic_ids = fixture
        .relics
        .as_array()
        .map(|relics| {
            relics
                .iter()
                .filter_map(|relic| relic.get("id").and_then(Value::as_str))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();

    CombatPreferenceState {
        encounter_names: combat
            .entities
            .monsters
            .iter()
            .filter(|monster| !monster.is_escaped)
            .map(|monster| {
                crate::content::monsters::EnemyId::from_id(monster.monster_type)
                    .map(|enemy| enemy.get_name().to_string())
                    .unwrap_or_else(|| format!("Monster#{}", monster.monster_type))
            })
            .collect(),
        player_hp: combat.entities.player.current_hp,
        player_max_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        incoming: incoming_damage(combat),
        hand: combat.zones.hand.iter().map(format_card).collect(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
        player_powers: store::powers_for(combat, 0)
            .map(|powers| {
                powers
                    .iter()
                    .map(|power| {
                        format!(
                            "{}={}",
                            crate::content::powers::get_power_definition(power.power_type).name,
                            power.amount
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        relic_ids,
        monsters: combat
            .entities
            .monsters
            .iter()
            .filter(|monster| !monster.is_escaped)
            .map(|monster| CombatPreferenceMonsterState {
                name: crate::content::monsters::EnemyId::from_id(monster.monster_type)
                    .map(|enemy| enemy.get_name().to_string())
                    .unwrap_or_else(|| format!("Monster#{}", monster.monster_type)),
                hp: monster.current_hp,
                block: monster.block,
                intent: format!("{:?}", monster.current_intent),
                powers: store::powers_for(combat, monster.id)
                    .map(|powers| {
                        powers
                            .iter()
                            .map(|power| {
                                format!(
                                    "{}={}",
                                    crate::content::powers::get_power_definition(power.power_type)
                                        .name,
                                    power.amount
                                )
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            })
            .collect(),
    }
}

fn preference_kind(chosen: &TrajectoryReport, preferred: &TrajectoryReport) -> Option<String> {
    let chosen_rank = outcome_rank(chosen.outcome);
    let preferred_rank = outcome_rank(preferred.outcome);
    if preferred_rank > chosen_rank {
        return Some("better_outcome".to_string());
    }
    if preferred.tags.iter().any(|tag| tag == "survival_line") && preferred.score > chosen.score {
        return Some("survival_family".to_string());
    }
    if preferred.score >= chosen.score + 150 {
        return Some("higher_score".to_string());
    }
    None
}

fn outcome_rank(outcome: TrajectoryOutcomeKind) -> i32 {
    match outcome {
        TrajectoryOutcomeKind::LethalWin => 4,
        TrajectoryOutcomeKind::Survives => 3,
        TrajectoryOutcomeKind::Timeout => 2,
        TrajectoryOutcomeKind::Dies => 1,
    }
}

fn sanitize_sample_label(label: &str) -> String {
    label
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '_' })
        .collect()
}

fn explore_paths(
    engine: &EngineState,
    combat: &CombatState,
    depth_remaining: usize,
    is_root: bool,
    branch_cap: usize,
    root_snapshot: StateSnapshot,
    baseline_hp: i32,
    baseline_potions: i32,
    prefix: &mut Vec<TrajectoryStepState>,
    out: &mut Vec<TrajectoryCandidate>,
) {
    if matches!(engine, EngineState::GameOver(_)) {
        out.push(finalize_candidate(
            engine,
            combat,
            root_snapshot,
            baseline_hp,
            baseline_potions,
            prefix,
        ));
        return;
    }

    let legal_moves = crate::bot::search::get_legal_moves(engine, combat);
    if legal_moves.is_empty() || depth_remaining == 0 {
        out.push(finalize_candidate(
            engine,
            combat,
            root_snapshot,
            baseline_hp,
            baseline_potions,
            prefix,
        ));
        return;
    }

    let ordered_moves = rank_branch_moves(engine, combat, &legal_moves, is_root, branch_cap);
    for input in ordered_moves {
        let before = capture_state(combat);
        let ended_turn = matches!(input, ClientInput::EndTurn);
        let action = describe_input(engine, combat, &input);
        let step_flags = classify_action_flags(combat, &input);
        let mut next_engine = engine.clone();
        let mut next_combat = combat.clone();
        let _ = tick_until_stable_turn(&mut next_engine, &mut next_combat, input.clone());
        let after = capture_state(&next_combat);

        let mut step = TrajectoryStepState {
            action,
            player_hp: after.player_hp,
            player_block: after.player_block,
            energy: after.energy,
            incoming: after.incoming,
            monster_hp: compact_monster_hp(after.monster_hp),
            incoming_reduced: after.incoming < before.incoming,
            weak_applied: after.weak_total > before.weak_total,
            block_gained: after.player_block > before.player_block,
            used_potion: after.filled_potions < before.filled_potions,
            setup_greed: false,
            direct_damage_push: after.total_monster_hp < before.total_monster_hp,
            ended_turn,
            threat_relief_before_enemy_window: ended_turn
                .then_some((root_snapshot.total_monster_hp - before.total_monster_hp).max(0)),
            defense_gap_before_enemy_window: ended_turn
                .then_some((before.incoming - before.player_block).max(0)),
            dead_draw_burden_at_next_player_window: ended_turn.then_some(
                (after.cycling_dead_draw_count - root_snapshot.cycling_dead_draw_count).max(0),
            ),
            collateral_exhaust_cost_of_immediate_conversion: Some(
                immediate_conversion_collateral_cost(combat, &next_combat, &input),
            ),
        };
        step.setup_greed = step_flags.looks_like_setup
            && !step.block_gained
            && !step.incoming_reduced
            && before.incoming >= before.player_hp.saturating_sub(before.player_block).max(1);

        prefix.push(step);
        explore_paths(
            &next_engine,
            &next_combat,
            depth_remaining.saturating_sub(1),
            false,
            branch_cap,
            root_snapshot,
            baseline_hp,
            baseline_potions,
            prefix,
            out,
        );
        prefix.pop();
    }
}

fn finalize_candidate(
    engine: &EngineState,
    combat: &CombatState,
    root_snapshot: StateSnapshot,
    baseline_hp: i32,
    baseline_potions: i32,
    prefix: &[TrajectoryStepState],
) -> TrajectoryCandidate {
    let first_move = prefix
        .first()
        .map(|step| step.action.clone())
        .unwrap_or_else(|| "<no_action>".to_string());
    let mut tail_engine = engine.clone();
    let mut tail_combat = combat.clone();
    let prefix_local_trace = local_window_trace_from_prefix(prefix);
    let tail = run_conservative_tail(
        &mut tail_engine,
        &mut tail_combat,
        root_snapshot,
        prefix_local_trace,
        baseline_hp,
        baseline_potions,
        80,
    );
    let mut tags = summarize_tags(prefix, &tail);
    tags.sort();
    tags.dedup();

    TrajectoryCandidate {
        first_move,
        actions: prefix.iter().map(|step| step.action.clone()).collect(),
        steps: prefix.to_vec(),
        outcome: tail.outcome,
        score: tail.score,
        score_breakdown: tail.score_breakdown,
        final_player_hp: tail.snapshot.player_hp,
        final_player_block: tail.snapshot.player_block,
        final_incoming: tail.snapshot.incoming,
        final_monster_hp: compact_monster_hp(tail.snapshot.monster_hp),
        tags,
    }
}

fn annotate_survival_lines(
    trajectories: &mut [TrajectoryCandidate],
    chosen_first_move: Option<&str>,
) {
    let chosen_best_score = chosen_first_move.and_then(|chosen| {
        trajectories
            .iter()
            .filter(|trajectory| trajectory.first_move == chosen)
            .map(|trajectory| trajectory.score)
            .max()
    });

    for trajectory in trajectories {
        let survives = matches!(
            trajectory.outcome,
            TrajectoryOutcomeKind::Survives | TrajectoryOutcomeKind::LethalWin
        );
        if survives
            && chosen_first_move.is_some()
            && Some(trajectory.first_move.as_str()) != chosen_first_move
            && chosen_best_score.is_some_and(|score| trajectory.score > score)
        {
            trajectory.tags.push("survival_line".to_string());
        }
    }
}

fn summarize_tags(prefix: &[TrajectoryStepState], tail: &TailOutcome) -> Vec<String> {
    let mut tags = Vec::new();
    if prefix.iter().any(|step| step.incoming_reduced) {
        tags.push("incoming_reduced".to_string());
    }
    if prefix.iter().any(|step| step.weak_applied) {
        tags.push("weak_applied".to_string());
    }
    if prefix.iter().any(|step| step.block_gained) {
        tags.push("block_gained".to_string());
    }
    if prefix.iter().any(|step| step.used_potion) {
        tags.push("used_potion".to_string());
    }
    if prefix.iter().any(|step| step.setup_greed) {
        tags.push("setup_greed".to_string());
    }
    if prefix.iter().any(|step| step.direct_damage_push)
        && !prefix
            .iter()
            .any(|step| step.incoming_reduced || step.block_gained)
    {
        tags.push("damage_race".to_string());
    }
    if matches!(
        tail.outcome,
        TrajectoryOutcomeKind::Survives | TrajectoryOutcomeKind::LethalWin
    ) {
        tags.push("survives".to_string());
    }
    tags
}

fn local_window_trace_from_prefix(prefix: &[TrajectoryStepState]) -> LocalWindowTrace {
    let mut trace = LocalWindowTrace::default();
    for step in prefix {
        trace.collateral_exhaust_cost_of_immediate_conversion += step
            .collateral_exhaust_cost_of_immediate_conversion
            .unwrap_or(0);
        if !trace.first_enemy_window_observed && step.ended_turn {
            trace.first_enemy_window_observed = true;
            trace.threat_relief_before_first_enemy_window =
                step.threat_relief_before_enemy_window.unwrap_or(0);
            trace.defense_gap_at_first_enemy_window =
                step.defense_gap_before_enemy_window.unwrap_or(0);
            trace.dead_draw_burden_at_next_player_window =
                step.dead_draw_burden_at_next_player_window.unwrap_or(0);
            break;
        }
    }
    trace
}

fn observe_enemy_window(
    mut trace: LocalWindowTrace,
    root_snapshot: StateSnapshot,
    before: StateSnapshot,
    after: StateSnapshot,
) -> LocalWindowTrace {
    if trace.first_enemy_window_observed {
        return trace;
    }
    trace.first_enemy_window_observed = true;
    trace.threat_relief_before_first_enemy_window =
        (root_snapshot.total_monster_hp - before.total_monster_hp).max(0);
    trace.defense_gap_at_first_enemy_window = (before.incoming - before.player_block).max(0);
    trace.dead_draw_burden_at_next_player_window =
        (after.cycling_dead_draw_count - root_snapshot.cycling_dead_draw_count).max(0);
    trace
}

fn observe_immediate_conversion_cost(
    mut trace: LocalWindowTrace,
    before_combat: &CombatState,
    after_combat: &CombatState,
    input: &ClientInput,
) -> LocalWindowTrace {
    if trace.first_enemy_window_observed {
        return trace;
    }
    trace.collateral_exhaust_cost_of_immediate_conversion +=
        immediate_conversion_collateral_cost(before_combat, after_combat, input);
    trace
}

fn finalize_local_window_trace(
    mut trace: LocalWindowTrace,
    root_snapshot: StateSnapshot,
    final_snapshot: StateSnapshot,
    victory: bool,
) -> LocalWindowTrace {
    if !trace.first_enemy_window_observed && victory {
        trace.threat_relief_before_first_enemy_window =
            (root_snapshot.total_monster_hp - final_snapshot.total_monster_hp).max(0);
    }
    trace
}

fn rank_branch_moves(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: &[ClientInput],
    is_root: bool,
    branch_cap: usize,
) -> Vec<ClientInput> {
    let mut scored = legal_moves
        .iter()
        .cloned()
        .map(|input| {
            let mut sim_engine = engine.clone();
            let mut sim_combat = combat.clone();
            let _ = tick_until_stable_turn(&mut sim_engine, &mut sim_combat, input.clone());
            (
                conservative_choice_score(&sim_engine, &sim_combat, &input, legal_moves.len() > 1),
                input,
            )
        })
        .collect::<Vec<_>>();
    scored.sort_by(|(left_score, _), (right_score, _)| right_score.cmp(left_score));
    if !is_root {
        scored.truncate(branch_cap.max(1));
    }
    scored.into_iter().map(|(_, input)| input).collect()
}

fn describe_input(engine: &EngineState, combat: &CombatState, input: &ClientInput) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat
                .zones
                .hand
                .get(*card_index)
                .map(format_card)
                .unwrap_or_else(|| format!("hand[{card_index}]"));
            match target {
                Some(target) => format!("Play #{} {card} @{target}", card_index + 1),
                None => format!("Play #{} {card}", card_index + 1),
            }
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => match target {
            Some(target) => format!("UsePotion#{potion_index} @{target}"),
            None => format!("UsePotion#{potion_index}"),
        },
        ClientInput::EndTurn => "EndTurn".to_string(),
        ClientInput::Cancel => "Cancel".to_string(),
        ClientInput::SubmitHandSelect(uuids) => {
            let selected = uuids
                .iter()
                .filter_map(|uuid| combat.zones.hand.iter().find(|card| card.uuid == *uuid))
                .map(format_card)
                .collect::<Vec<_>>();
            format!("HandSelect [{}]", selected.join(", "))
        }
        ClientInput::SubmitGridSelect(uuids) => {
            format!("GridSelect {:?}", uuids)
        }
        ClientInput::SubmitDiscoverChoice(index) => {
            format!("DiscoverChoice#{index}")
        }
        ClientInput::Proceed => format!("Proceed({engine:?})"),
        other => format!("{other:?}"),
    }
}

fn format_card(card: &CombatCard) -> String {
    let mut label = cards::get_card_definition(card.id).name.to_string();
    for _ in 0..card.upgrades {
        label.push('+');
    }
    label
}

fn compact_monster_hp(monster_hp: [i32; 5]) -> Vec<i32> {
    monster_hp.into_iter().filter(|hp| *hp >= 0).collect()
}

fn capture_state(combat: &CombatState) -> StateSnapshot {
    let mut monster_hp = [-1; 5];
    for (idx, monster) in combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_escaped)
        .enumerate()
    {
        if idx >= monster_hp.len() {
            break;
        }
        monster_hp[idx] = monster.current_hp + monster.block;
    }

    StateSnapshot {
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        incoming: incoming_damage(combat),
        total_monster_hp: total_monster_hp(combat),
        monster_hp,
        weak_total: combat
            .entities
            .monsters
            .iter()
            .map(|monster| store::power_amount(combat, monster.id, PowerId::Weak))
            .sum(),
        filled_potions: filled_potion_slots(combat),
        cycling_dead_draw_count: cycling_dead_draw_count(combat),
    }
}

struct ActionFlags {
    looks_like_setup: bool,
}

fn classify_action_flags(combat: &CombatState, input: &ClientInput) -> ActionFlags {
    let looks_like_setup = match input {
        ClientInput::PlayCard { card_index, .. } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| {
                let def = cards::get_card_definition(card.id);
                if def.card_type == CardType::Power {
                    return true;
                }
                def.card_type == CardType::Skill
                    && def.base_block == 0
                    && def.base_damage == 0
                    && !matches!(
                        card.id,
                        cards::CardId::ShrugItOff
                            | cards::CardId::PowerThrough
                            | cards::CardId::FlameBarrier
                            | cards::CardId::Clothesline
                    )
            })
            .unwrap_or(false),
        _ => false,
    };
    ActionFlags { looks_like_setup }
}

#[derive(Debug, Clone)]
struct TailOutcome {
    outcome: TrajectoryOutcomeKind,
    score: i32,
    score_breakdown: ScoreBreakdown,
    snapshot: StateSnapshot,
}

fn run_conservative_tail(
    engine: &mut EngineState,
    combat: &mut CombatState,
    root_snapshot: StateSnapshot,
    initial_local_trace: LocalWindowTrace,
    baseline_hp: i32,
    baseline_potions: i32,
    step_limit: usize,
) -> TailOutcome {
    let mut local_trace = initial_local_trace;
    for steps in 0..=step_limit {
        let cleared = combat_cleared(combat);
        match engine {
            EngineState::GameOver(RunResult::Victory) => {
                let snapshot = capture_state(combat);
                let local_trace =
                    finalize_local_window_trace(local_trace, root_snapshot, snapshot, true);
                let score_breakdown = terminal_score_breakdown(
                    snapshot,
                    baseline_hp,
                    baseline_potions,
                    true,
                    false,
                    steps as i32,
                    cleared,
                    local_trace,
                );
                return TailOutcome {
                    outcome: TrajectoryOutcomeKind::LethalWin,
                    score: score_breakdown.total,
                    score_breakdown,
                    snapshot,
                };
            }
            EngineState::GameOver(_) => {
                let snapshot = capture_state(combat);
                let local_trace =
                    finalize_local_window_trace(local_trace, root_snapshot, snapshot, false);
                let score_breakdown = terminal_score_breakdown(
                    snapshot,
                    baseline_hp,
                    baseline_potions,
                    false,
                    false,
                    steps as i32,
                    cleared,
                    local_trace,
                );
                return TailOutcome {
                    outcome: TrajectoryOutcomeKind::Dies,
                    score: score_breakdown.total,
                    score_breakdown,
                    snapshot,
                };
            }
            _ => {}
        }

        if cleared {
            let snapshot = capture_state(combat);
            let local_trace =
                finalize_local_window_trace(local_trace, root_snapshot, snapshot, true);
            let score_breakdown = terminal_score_breakdown(
                snapshot,
                baseline_hp,
                baseline_potions,
                true,
                false,
                steps as i32,
                true,
                local_trace,
            );
            return TailOutcome {
                outcome: TrajectoryOutcomeKind::LethalWin,
                score: score_breakdown.total,
                score_breakdown,
                snapshot,
            };
        }

        let legal_moves = crate::bot::search::get_legal_moves(engine, combat);
        if legal_moves.is_empty() {
            let snapshot = capture_state(combat);
            let local_trace =
                finalize_local_window_trace(local_trace, root_snapshot, snapshot, false);
            let score_breakdown = terminal_score_breakdown(
                snapshot,
                baseline_hp,
                baseline_potions,
                false,
                true,
                steps as i32,
                cleared,
                local_trace,
            );
            return TailOutcome {
                outcome: TrajectoryOutcomeKind::Timeout,
                score: score_breakdown.total,
                score_breakdown,
                snapshot,
            };
        }

        let chosen = choose_conservative_action(engine, combat, &legal_moves);
        let chosen_is_end_turn = matches!(chosen, ClientInput::EndTurn);
        let before_combat = (!local_trace.first_enemy_window_observed).then(|| combat.clone());
        let before = chosen_is_end_turn.then(|| capture_state(combat));
        let _ = tick_until_stable_turn(engine, combat, chosen.clone());
        if let Some(before_combat) = before_combat.as_ref() {
            local_trace =
                observe_immediate_conversion_cost(local_trace, before_combat, combat, &chosen);
        }
        if !local_trace.first_enemy_window_observed && chosen_is_end_turn {
            let after = capture_state(combat);
            let before = before.expect("captured before end turn");
            local_trace = observe_enemy_window(local_trace, root_snapshot, before, after);
        }
    }

    let snapshot = capture_state(combat);
    let survives = snapshot.player_hp > 0;
    let cleared = combat_cleared(combat);
    let victory = survives && cleared;
    let timeout = !victory;
    let local_trace = finalize_local_window_trace(local_trace, root_snapshot, snapshot, victory);
    let score_breakdown = terminal_score_breakdown(
        snapshot,
        baseline_hp,
        baseline_potions,
        victory,
        timeout,
        step_limit as i32,
        cleared,
        local_trace,
    );
    TailOutcome {
        outcome: if victory {
            TrajectoryOutcomeKind::LethalWin
        } else if survives {
            TrajectoryOutcomeKind::Survives
        } else {
            TrajectoryOutcomeKind::Timeout
        },
        score: score_breakdown.total,
        score_breakdown,
        snapshot,
    }
}

fn choose_conservative_action(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: &[ClientInput],
) -> ClientInput {
    let mut best_move = legal_moves[0].clone();
    let mut best_score = i32::MIN;

    for input in legal_moves {
        let mut sim_engine = engine.clone();
        let mut sim_combat = combat.clone();
        let _ = tick_until_stable_turn(&mut sim_engine, &mut sim_combat, input.clone());
        let score =
            conservative_choice_score(&sim_engine, &sim_combat, input, legal_moves.len() > 1);
        if score > best_score {
            best_score = score;
            best_move = input.clone();
        }
    }

    best_move
}

fn conservative_choice_score(
    engine: &EngineState,
    combat: &CombatState,
    candidate: &ClientInput,
    has_alternative: bool,
) -> i32 {
    if matches!(engine, EngineState::GameOver(RunResult::Defeat)) {
        return -1_000_000;
    }
    if matches!(engine, EngineState::GameOver(RunResult::Victory)) || combat_cleared(combat) {
        return 1_000_000 + combat.entities.player.current_hp * 200;
    }

    let incoming = incoming_damage(combat);
    let unblocked = (incoming - combat.entities.player.block).max(0);
    let mut score = combat.entities.player.current_hp * 550 + combat.entities.player.block * 24
        - unblocked * 170
        - total_monster_hp(combat) * 8
        + (combat_heuristic::evaluate_combat_state(combat) as i32 / 4);

    if matches!(candidate, ClientInput::EndTurn) && has_alternative {
        score -= 320;
    }
    if matches!(candidate, ClientInput::UsePotion { .. }) {
        score -= 110;
    }

    score
}

fn terminal_score_breakdown(
    snapshot: StateSnapshot,
    baseline_hp: i32,
    baseline_potions: i32,
    victory: bool,
    timeout: bool,
    steps: i32,
    combat_cleared: bool,
    local_trace: LocalWindowTrace,
) -> ScoreBreakdown {
    let hp_loss = (baseline_hp - snapshot.player_hp).max(0);
    let potions_used = (baseline_potions - snapshot.filled_potions).max(0);
    let threat_relief_bonus =
        local_trace.threat_relief_before_first_enemy_window * FIRST_WINDOW_THREAT_RELIEF_WEIGHT;
    let defense_gap_penalty =
        local_trace.defense_gap_at_first_enemy_window * FIRST_WINDOW_DEFENSE_GAP_WEIGHT;
    let dead_draw_burden_penalty = local_trace.dead_draw_burden_at_next_player_window
        * NEXT_PLAYER_WINDOW_DEAD_DRAW_BURDEN_WEIGHT;
    let collateral_exhaust_penalty = local_trace.collateral_exhaust_cost_of_immediate_conversion
        * IMMEDIATE_CONVERSION_COLLATERAL_WEIGHT;
    let (
        bucket,
        bucket_bonus,
        hp_bonus,
        monster_hp_penalty,
        hp_loss_penalty,
        potions_used_penalty,
        steps_penalty,
    ) = if victory {
        (
            "victory".to_string(),
            5_000,
            snapshot.player_hp * 50,
            0,
            hp_loss * 18,
            potions_used * 90,
            steps * 6,
        )
    } else if timeout && snapshot.player_hp > 0 {
        (
            "timeout_survives".to_string(),
            1_000,
            snapshot.player_hp * 28,
            snapshot.total_monster_hp * 9,
            hp_loss * 15,
            potions_used * 70,
            steps * 4,
        )
    } else {
        (
            if timeout {
                "timeout_dead".to_string()
            } else {
                "defeat".to_string()
            }
            .to_string(),
            if timeout { -3_000 } else { -5_000 },
            0,
            snapshot.total_monster_hp * if timeout { 14 } else { 16 },
            hp_loss * if timeout { 20 } else { 22 },
            potions_used * if timeout { 80 } else { 90 },
            steps * if timeout { 4 } else { 5 },
        )
    };
    let total = bucket_bonus + hp_bonus + threat_relief_bonus
        - monster_hp_penalty
        - hp_loss_penalty
        - potions_used_penalty
        - steps_penalty
        - defense_gap_penalty
        - dead_draw_burden_penalty
        - collateral_exhaust_penalty;
    ScoreBreakdown {
        bucket,
        combat_cleared,
        victory,
        timeout,
        first_enemy_window_observed: local_trace.first_enemy_window_observed,
        steps,
        baseline_hp,
        final_hp: snapshot.player_hp,
        hp_loss,
        baseline_potions,
        final_filled_potions: snapshot.filled_potions,
        potions_used,
        final_total_monster_hp: snapshot.total_monster_hp,
        threat_relief_before_first_enemy_window: local_trace
            .threat_relief_before_first_enemy_window,
        defense_gap_at_first_enemy_window: local_trace.defense_gap_at_first_enemy_window,
        dead_draw_burden_at_next_player_window: local_trace.dead_draw_burden_at_next_player_window,
        collateral_exhaust_cost_of_immediate_conversion: local_trace
            .collateral_exhaust_cost_of_immediate_conversion,
        bucket_bonus,
        hp_bonus,
        threat_relief_bonus,
        monster_hp_penalty,
        hp_loss_penalty,
        potions_used_penalty,
        steps_penalty,
        defense_gap_penalty,
        dead_draw_burden_penalty,
        collateral_exhaust_penalty,
        total,
    }
}

fn total_monster_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp + monster.block)
        .sum()
}

fn incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| match monster.current_intent {
            crate::combat::Intent::Attack { hits, .. }
            | crate::combat::Intent::AttackBuff { hits, .. }
            | crate::combat::Intent::AttackDebuff { hits, .. }
            | crate::combat::Intent::AttackDefend { hits, .. } => monster.intent_dmg * hits as i32,
            _ => 0,
        })
        .sum()
}

fn filled_potion_slots(combat: &CombatState) -> i32 {
    combat
        .entities
        .potions
        .iter()
        .filter(|slot| slot.is_some())
        .count() as i32
}

fn cycling_dead_draw_count(combat: &CombatState) -> i32 {
    combat
        .zones
        .hand
        .iter()
        .chain(combat.zones.draw_pile.iter())
        .chain(combat.zones.discard_pile.iter())
        .filter(|card| {
            matches!(
                cards::get_card_definition(card.id).card_type,
                CardType::Status | CardType::Curse
            )
        })
        .count() as i32
}

fn immediate_conversion_collateral_cost(
    before_combat: &CombatState,
    after_combat: &CombatState,
    input: &ClientInput,
) -> i32 {
    if !is_immediate_conversion_action(before_combat, input) {
        return 0;
    }

    newly_exhausted_cards(before_combat, after_combat)
        .into_iter()
        .filter(|card| !is_obvious_conversion_payload(card))
        .map(collateral_exhaust_card_cost)
        .sum()
}

fn is_immediate_conversion_action(combat: &CombatState, input: &ClientInput) -> bool {
    match input {
        ClientInput::PlayCard { card_index, .. } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| matches!(card.id, cards::CardId::SecondWind))
            .unwrap_or(false),
        _ => false,
    }
}

fn newly_exhausted_cards<'a>(
    before_combat: &CombatState,
    after_combat: &'a CombatState,
) -> Vec<&'a CombatCard> {
    after_combat
        .zones
        .exhaust_pile
        .iter()
        .filter(|card| {
            !before_combat
                .zones
                .exhaust_pile
                .iter()
                .any(|before| before.uuid == card.uuid)
        })
        .collect()
}

fn is_obvious_conversion_payload(card: &CombatCard) -> bool {
    matches!(
        cards::get_card_definition(card.id).card_type,
        CardType::Status | CardType::Curse
    )
}

fn collateral_exhaust_card_cost(card: &CombatCard) -> i32 {
    match card.id {
        cards::CardId::Barricade => 2,
        cards::CardId::Entrench => 1,
        _ => 0,
    }
}

fn combat_cleared(combat: &CombatState) -> bool {
    combat
        .entities
        .monsters
        .iter()
        .all(|monster| monster.is_dying || monster.is_escaped || monster.current_hp <= 0)
}

fn candidate_to_report(candidate: TrajectoryCandidate) -> TrajectoryReport {
    TrajectoryReport {
        first_move: candidate.first_move,
        actions: candidate.actions,
        steps: candidate
            .steps
            .into_iter()
            .map(|step| TrajectoryStepReport {
                action: step.action,
                player_hp: step.player_hp,
                player_block: step.player_block,
                energy: step.energy,
                incoming: step.incoming,
                monster_hp: step.monster_hp,
            })
            .collect(),
        outcome: candidate.outcome,
        score: candidate.score,
        score_breakdown: candidate.score_breakdown,
        final_player_hp: candidate.final_player_hp,
        final_player_block: candidate.final_player_block,
        final_incoming: candidate.final_incoming,
        final_monster_hp: candidate.final_monster_hp,
        tags: candidate.tags,
    }
}

fn trajectory_sort_cmp(
    left: &TrajectoryCandidate,
    right: &TrajectoryCandidate,
) -> std::cmp::Ordering {
    trajectory_rank_key(right).cmp(&trajectory_rank_key(left))
}

fn trajectory_rank_key(candidate: &TrajectoryCandidate) -> (i32, i32, i32, i32, i32) {
    let outcome_rank = match candidate.outcome {
        TrajectoryOutcomeKind::LethalWin => 4,
        TrajectoryOutcomeKind::Survives => 3,
        TrajectoryOutcomeKind::Timeout => 2,
        TrajectoryOutcomeKind::Dies => 1,
    };
    (
        outcome_rank,
        candidate.final_player_hp,
        -candidate.final_incoming,
        -candidate.final_monster_hp.iter().sum::<i32>(),
        candidate.score,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::replay::live_comm_replay::{
        derive_combat_replay_view, find_combat_step_index_by_before_frame_id,
        load_live_session_replay_path, reconstruct_combat_replay_step,
    };
    use crate::state::EngineState;
    use crate::testing::support::test_support::{basic_combat, CombatTestExt};
    use std::path::Path;

    fn target_raw() -> &'static Path {
        Path::new(r"d:\rust\sts_simulator\logs\raw\live_comm_raw_20260412_214122.jsonl")
    }

    fn fixture_178() -> &'static Path {
        Path::new(r"d:\rust\sts_simulator\tests\decision_audit\hexaghost_frame_178.json")
    }

    #[test]
    fn can_reconstruct_hexaghost_decision_frame() {
        let replay = load_live_session_replay_path(target_raw()).expect("replay");
        let view = derive_combat_replay_view(&replay);
        let step_index = find_combat_step_index_by_before_frame_id(&view, 203).expect("frame 203");
        let reconstructed = reconstruct_combat_replay_step(&view, step_index).expect("step");
        assert_eq!(reconstructed.before_state_frame_id, Some(203));
        assert_eq!(reconstructed.command_text, "PLAY 3 0");
        assert_eq!(reconstructed.before_combat.entities.player.current_hp, 16);
        assert_eq!(reconstructed.before_combat.turn.energy, 2);
    }

    #[test]
    fn fixture_audit_finds_non_chosen_survival_family() {
        let fixture = load_fixture_path(fixture_178()).expect("fixture");
        let report = audit_fixture(&fixture, DecisionAuditConfig::default()).expect("report");
        assert_eq!(
            report.chosen_first_move.as_deref(),
            Some("Play #1 Strike+ @1")
        );
        let survival_trajectories = report
            .first_move_reports
            .iter()
            .flat_map(|group| group.top_trajectories.iter())
            .filter(|trajectory| trajectory.tags.iter().any(|tag| tag == "survival_line"))
            .collect::<Vec<_>>();
        assert!(!survival_trajectories.is_empty());
        assert!(survival_trajectories.iter().any(|trajectory| {
            trajectory.first_move != report.chosen_first_move.clone().unwrap()
                && trajectory.tags.iter().any(|tag| {
                    tag == "block_gained" || tag == "incoming_reduced" || tag == "used_potion"
                })
        }));
    }

    #[test]
    fn fixture_audit_is_stable_across_repeated_runs() {
        let fixture = load_fixture_path(fixture_178()).expect("fixture");
        let left = audit_fixture(&fixture, DecisionAuditConfig::default()).expect("left");
        let right = audit_fixture(&fixture, DecisionAuditConfig::default()).expect("right");
        let left_json = serde_json::to_string(&left).expect("left json");
        let right_json = serde_json::to_string(&right).expect("right json");
        assert_eq!(left_json, right_json);
    }

    #[test]
    fn raw_frame_and_fixture_agree_on_hexaghost_178_chosen_move() {
        let replay = load_live_session_replay_path(target_raw()).expect("replay");
        let view = derive_combat_replay_view(&replay);
        let step_index = find_combat_step_index_by_before_frame_id(&view, 178).expect("frame 178");
        let reconstructed = reconstruct_combat_replay_step(&view, step_index).expect("step");
        let rebuilt_fixture = build_fixture_from_reconstructed_step(
            &reconstructed,
            replay.source_path.clone(),
            "hexaghost_frame_178_rebuilt",
        )
        .expect("rebuilt fixture");
        let fixture_report = audit_fixture(
            &load_fixture_path(fixture_178()).expect("fixture"),
            DecisionAuditConfig::default(),
        )
        .expect("fixture report");
        let rebuilt_report = audit_fixture(&rebuilt_fixture, DecisionAuditConfig::default())
            .expect("rebuilt report");
        assert_eq!(
            fixture_report.chosen_first_move,
            rebuilt_report.chosen_first_move
        );
        assert_eq!(
            serde_json::to_string(&fixture_report.first_move_reports).expect("fixture groups"),
            serde_json::to_string(&rebuilt_report.first_move_reports).expect("rebuilt groups")
        );
    }

    #[test]
    fn fixture_audit_exports_state_conditioned_preference_samples() {
        let fixture = load_fixture_path(fixture_178()).expect("fixture");
        let config = DecisionAuditConfig::default();
        let report = audit_fixture(&fixture, config).expect("report");
        let samples = extract_preference_samples(&fixture, &report, config).expect("samples");
        assert!(!samples.is_empty());
        assert!(samples
            .iter()
            .all(|sample| sample.chosen_action != sample.preferred_action));
        assert!(samples
            .iter()
            .all(|sample| sample.state_source == RECONSTRUCTED_LIVE_REPLAY_STATE_SOURCE));
        assert!(samples
            .iter()
            .all(|sample| sample.chosen_source == LIVE_REPLAY_SOURCE));
        assert!(samples
            .iter()
            .all(|sample| sample.preferred_source == OFFLINE_AUDIT_SEARCH_SOURCE));
        assert!(samples.iter().all(|sample| sample.preferred_search_kind
            == OFFLINE_COUNTERFACTUAL_BRANCH_SEARCH_KIND
            && sample.chosen_action_observed
            && !sample.preferred_action_observed));
        assert!(samples
            .iter()
            .all(|sample| sample.decision_depth == config.decision_depth
                && sample.top_k == config.top_k
                && sample.branch_cap == config.branch_cap));
        assert!(samples.iter().any(|sample| sample
            .state
            .encounter_names
            .iter()
            .any(|name| name == "Hexaghost")));
        assert!(samples.iter().any(|sample| {
            sample
                .preferred_tags
                .iter()
                .any(|tag| tag == "survival_line" || tag == "block_gained")
        }));
    }

    #[test]
    fn conservative_tail_treats_cleared_combat_as_lethal_win_without_game_over() {
        let combat = basic_combat()
            .with_monster_hp(1, 0)
            .with_player_hp(42)
            .with_player_block(7);
        let mut engine = EngineState::CombatPlayerTurn;
        let mut combat = combat;
        let root_snapshot = capture_state(&combat);

        let tail = run_conservative_tail(
            &mut engine,
            &mut combat,
            root_snapshot,
            LocalWindowTrace::default(),
            42,
            0,
            8,
        );

        assert_eq!(tail.outcome, TrajectoryOutcomeKind::LethalWin);
        assert!(tail.score_breakdown.victory);
        assert!(tail.score_breakdown.combat_cleared);
        assert_eq!(tail.score_breakdown.bucket, "victory");
        assert_eq!(tail.snapshot.total_monster_hp, 0);
    }
}
