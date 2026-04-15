use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::bot::search::{
    audit_state, DecisionAuditConfig, DecisionAuditReport, ScoreBreakdown, StatePressureFeatures,
    TrajectoryOutcomeKind, TrajectoryReport,
};
use crate::content::cards::{get_card_definition, CardType};
use crate::engine::core::tick_until_stable_turn;
use crate::state::core::EngineState;

use crate::testing::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};
use crate::testing::fixtures::scenario::{input_for_step, ScenarioStep, StructuredScenarioStep};

const CLOSE_ENOUGH_SCORE_GAP: i32 = 120;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BossPreferenceVerdict {
    PreferA,
    PreferB,
    CloseEnough,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BossRationaleTag {
    CurrentWindowRelief,
    NextWindowRisk,
    IrreversibleResourceSpend,
    SetupTiming,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamedLine {
    pub name: String,
    pub steps: Vec<StructuredScenarioStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BossPreferenceCaseSpec {
    pub name: String,
    pub start_spec_path: PathBuf,
    #[serde(default)]
    pub seed: Option<u64>,
    #[serde(default)]
    pub prefix_actions: Vec<StructuredScenarioStep>,
    pub candidates: Vec<NamedLine>,
    pub expected_verdict: BossPreferenceVerdict,
    #[serde(default)]
    pub expected_rationale_tags: Vec<BossRationaleTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NaturalStartTraceRecord {
    pub start_spec_name: String,
    pub seed: u64,
    pub episode_id: String,
    pub turn_count: u32,
    pub encounter_name: String,
    pub observation: serde_json::Value,
    pub legal_actions: Vec<String>,
    pub action_labels: Vec<String>,
    pub belief_summary: serde_json::Value,
    pub pressure_summary: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossPreferenceCaseRecord {
    pub case_name: String,
    pub start_spec_name: String,
    pub seed: u64,
    pub prefix_actions: Vec<StructuredScenarioStep>,
    pub candidate_a: Vec<StructuredScenarioStep>,
    pub candidate_b: Vec<StructuredScenarioStep>,
    #[serde(default)]
    pub candidate_c: Option<Vec<StructuredScenarioStep>>,
    pub expected_verdict: BossPreferenceVerdict,
    pub expected_rationale_tags: Vec<BossRationaleTag>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossValidationCandidateResult {
    pub name: String,
    pub outcome: TrajectoryOutcomeKind,
    pub outcome_rank: i32,
    pub score: i32,
    pub top_actions: Vec<String>,
    pub tags: Vec<String>,
    pub rationale_tags: Vec<BossRationaleTag>,
    pub score_breakdown: ScoreBreakdown,
    pub line_summary: CandidateLineSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossValidationResult {
    pub case_name: String,
    pub expected_verdict: BossPreferenceVerdict,
    pub actual_verdict: BossPreferenceVerdict,
    pub rationale_tags: Vec<BossRationaleTag>,
    pub candidates: Vec<BossValidationCandidateResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossValidationStateSummary {
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: u8,
    pub enemy_total_hp: i32,
    pub visible_incoming: i32,
    pub hand_size: usize,
    pub playable_cards: usize,
    pub attack_cards_in_hand: usize,
    pub skill_cards_in_hand: usize,
    pub power_cards_in_hand: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CandidateLineSummary {
    pub step_count: usize,
    pub play_count: usize,
    pub total_energy_spend: i32,
    pub attack_plays: usize,
    pub skill_plays: usize,
    pub power_plays: usize,
    pub enemy_targeted_plays: usize,
    pub untargeted_plays: usize,
    pub end_turn_steps: usize,
    pub played_cards: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BossValidationLedgerRecord {
    pub validation_pack: String,
    pub boss: String,
    pub deck_summary: String,
    pub case_id: String,
    pub expected_label: BossPreferenceVerdict,
    pub observed_label: BossPreferenceVerdict,
    pub pass: bool,
    pub rationale_tags: Vec<BossRationaleTag>,
    pub state_summary: BossValidationStateSummary,
    pub candidates: Vec<BossValidationCandidateResult>,
    pub case_path: String,
    pub start_spec_path: String,
    pub seed: u64,
    pub timestamp_epoch_s: u64,
}

pub fn load_case_spec(path: &Path) -> Result<(BossPreferenceCaseSpec, CombatStartSpec), String> {
    let payload = fs::read_to_string(path)
        .map_err(|err| format!("failed to read case spec {}: {err}", path.display()))?;
    let case: BossPreferenceCaseSpec = serde_json::from_str(&payload)
        .map_err(|err| format!("failed to parse case spec {}: {err}", path.display()))?;
    let start_path = resolve_relative(path, &case.start_spec_path);
    let start_payload = fs::read_to_string(&start_path)
        .map_err(|err| format!("failed to read start spec {}: {err}", start_path.display()))?;
    let mut start_spec: CombatStartSpec = serde_json::from_str(&start_payload)
        .map_err(|err| format!("failed to parse start spec {}: {err}", start_path.display()))?;
    if let Some(seed) = case.seed {
        start_spec.seed = seed;
    }
    Ok((case, start_spec))
}

pub fn validate_case(
    case_path: &Path,
    config: DecisionAuditConfig,
) -> Result<BossValidationResult, String> {
    let (case, start_spec) = load_case_spec(case_path)?;
    if case.candidates.len() < 2 || case.candidates.len() > 3 {
        return Err(format!(
            "boss preference case '{}' must contain 2 or 3 candidates, got {}",
            case.name,
            case.candidates.len()
        ));
    }

    let (mut engine_state, mut combat) = compile_combat_start_spec(&start_spec)?;
    execute_steps(&mut engine_state, &mut combat, &case.prefix_actions)?;

    let mut candidate_results = Vec::new();
    for candidate in &case.candidates {
        let mut branch_engine = engine_state.clone();
        let mut branch_combat = combat.clone();
        let line_summary = execute_steps(&mut branch_engine, &mut branch_combat, &candidate.steps)?;
        if !matches!(branch_engine, EngineState::CombatPlayerTurn) {
            return Err(format!(
                "candidate '{}' for case '{}' ended in unsupported engine state {branch_engine:?}; end candidate lines on a stable player turn",
                candidate.name, case.name
            ));
        }
        let report = audit_state(
            format!("{}::{}", case.name, candidate.name),
            Some(case_path.display().to_string()),
            None,
            None,
            branch_engine,
            branch_combat,
            None,
            config,
        )?;
        let best = best_trajectory(&report).ok_or_else(|| {
            format!(
                "audit report for case '{}' candidate '{}' had no trajectories",
                case.name, candidate.name
            )
        })?;
        candidate_results.push(BossValidationCandidateResult {
            name: candidate.name.clone(),
            outcome: best.outcome,
            outcome_rank: outcome_rank(best.outcome),
            score: best.score,
            top_actions: best.actions.clone(),
            tags: best.tags.clone(),
            rationale_tags: infer_rationale_tags(&best.score_breakdown),
            score_breakdown: best.score_breakdown.clone(),
            line_summary,
        });
    }

    let actual_verdict = compare_primary_pair(&candidate_results)?;
    let rationale_tags = dominant_rationale_tags(&candidate_results, &actual_verdict);
    Ok(BossValidationResult {
        case_name: case.name,
        expected_verdict: case.expected_verdict,
        actual_verdict,
        rationale_tags,
        candidates: candidate_results,
    })
}

pub fn build_ledger_record(
    case_path: &Path,
    result: &BossValidationResult,
) -> Result<BossValidationLedgerRecord, String> {
    let (case, start_spec) = load_case_spec(case_path)?;
    let (mut engine_state, mut combat) = compile_combat_start_spec(&start_spec)?;
    execute_steps(&mut engine_state, &mut combat, &case.prefix_actions)?;
    let validation_pack = case_path
        .parent()
        .and_then(|parent| parent.file_name())
        .and_then(|name| name.to_str())
        .unwrap_or("unknown_pack")
        .to_string();
    let start_spec_path = resolve_relative(case_path, &case.start_spec_path);
    let timestamp_epoch_s = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|err| format!("system time error: {err}"))?
        .as_secs();
    Ok(BossValidationLedgerRecord {
        validation_pack,
        boss: start_spec.encounter_id.clone(),
        deck_summary: summarize_deck(&start_spec),
        case_id: result.case_name.clone(),
        expected_label: result.expected_verdict.clone(),
        observed_label: result.actual_verdict.clone(),
        pass: result.expected_verdict == result.actual_verdict,
        rationale_tags: result.rationale_tags.clone(),
        state_summary: summarize_state(&combat),
        candidates: result.candidates.clone(),
        case_path: case_path.display().to_string(),
        start_spec_path: start_spec_path.display().to_string(),
        seed: start_spec.seed,
        timestamp_epoch_s,
    })
}

fn execute_steps(
    engine_state: &mut EngineState,
    combat: &mut crate::combat::CombatState,
    steps: &[StructuredScenarioStep],
) -> Result<CandidateLineSummary, String> {
    let mut summary = CandidateLineSummary::default();
    for structured in steps {
        let step = ScenarioStep {
            command: String::new(),
            label: None,
            response_id: None,
            frame_id: None,
            command_kind: None,
            structured: Some(structured.clone()),
        };
        let input = input_for_step(&step, engine_state, combat).ok_or_else(|| {
            format!(
                "structured step {:?} is not legal in engine state {engine_state:?}",
                structured
            )
        })?;
        summary.step_count += 1;
        match &input {
            crate::state::core::ClientInput::PlayCard { card_index, target } => {
                let card = combat
                    .zones
                    .hand
                    .get(*card_index)
                    .ok_or_else(|| format!("card index {} out of range", card_index))?;
                let def = get_card_definition(card.id);
                summary.play_count += 1;
                summary.total_energy_spend += i32::from(card.get_cost().max(0) as u8);
                summary.played_cards.push(def.name.to_string());
                match def.card_type {
                    CardType::Attack => summary.attack_plays += 1,
                    CardType::Skill => summary.skill_plays += 1,
                    CardType::Power => summary.power_plays += 1,
                    _ => {}
                }
                if target.is_some() {
                    summary.enemy_targeted_plays += 1;
                } else {
                    summary.untargeted_plays += 1;
                }
            }
            crate::state::core::ClientInput::EndTurn => {
                summary.end_turn_steps += 1;
            }
            _ => {}
        }
        let alive = tick_until_stable_turn(engine_state, combat, input);
        if !alive {
            break;
        }
    }
    Ok(summary)
}

fn best_trajectory<'a>(report: &'a DecisionAuditReport) -> Option<&'a TrajectoryReport> {
    report
        .first_move_reports
        .iter()
        .flat_map(|first| first.top_trajectories.iter())
        .max_by_key(|trajectory| (outcome_rank(trajectory.outcome), trajectory.score))
}

fn compare_primary_pair(
    candidates: &[BossValidationCandidateResult],
) -> Result<BossPreferenceVerdict, String> {
    let candidate_a = candidates
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case("A"))
        .or_else(|| candidates.first())
        .ok_or_else(|| "missing candidate A".to_string())?;
    let candidate_b = candidates
        .iter()
        .find(|candidate| candidate.name.eq_ignore_ascii_case("B"))
        .or_else(|| candidates.get(1))
        .ok_or_else(|| "missing candidate B".to_string())?;

    if candidate_a.outcome_rank != candidate_b.outcome_rank {
        return Ok(if candidate_a.outcome_rank > candidate_b.outcome_rank {
            BossPreferenceVerdict::PreferA
        } else {
            BossPreferenceVerdict::PreferB
        });
    }

    let score_gap = candidate_a.score - candidate_b.score;
    if score_gap.abs() < CLOSE_ENOUGH_SCORE_GAP {
        Ok(BossPreferenceVerdict::CloseEnough)
    } else if score_gap > 0 {
        Ok(BossPreferenceVerdict::PreferA)
    } else {
        Ok(BossPreferenceVerdict::PreferB)
    }
}

fn dominant_rationale_tags(
    candidates: &[BossValidationCandidateResult],
    verdict: &BossPreferenceVerdict,
) -> Vec<BossRationaleTag> {
    let winner = match verdict {
        BossPreferenceVerdict::PreferA => candidates
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case("A"))
            .or_else(|| candidates.first()),
        BossPreferenceVerdict::PreferB => candidates
            .iter()
            .find(|candidate| candidate.name.eq_ignore_ascii_case("B"))
            .or_else(|| candidates.get(1)),
        BossPreferenceVerdict::CloseEnough => candidates.first(),
    };
    winner
        .map(|candidate| candidate.rationale_tags.iter().take(2).cloned().collect())
        .unwrap_or_default()
}

fn infer_rationale_tags(breakdown: &ScoreBreakdown) -> Vec<BossRationaleTag> {
    let mut tags = Vec::new();
    if breakdown.threat_relief_before_first_enemy_window > 0 {
        tags.push(BossRationaleTag::CurrentWindowRelief);
    }
    if breakdown.defense_gap_at_first_enemy_window > 0 || breakdown.hp_loss_penalty > 0 {
        tags.push(BossRationaleTag::NextWindowRisk);
    }
    if breakdown.collateral_exhaust_cost_of_immediate_conversion > 0 || breakdown.potions_used > 0 {
        tags.push(BossRationaleTag::IrreversibleResourceSpend);
    }
    if breakdown.steps > 0 && breakdown.threat_relief_before_first_enemy_window == 0 {
        tags.push(BossRationaleTag::SetupTiming);
    }
    tags
}

fn outcome_rank(outcome: TrajectoryOutcomeKind) -> i32 {
    match outcome {
        TrajectoryOutcomeKind::LethalWin => 3,
        TrajectoryOutcomeKind::Survives => 2,
        TrajectoryOutcomeKind::Timeout => 1,
        TrajectoryOutcomeKind::Dies => 0,
    }
}

fn resolve_relative(base: &Path, relative: &Path) -> PathBuf {
    if relative.is_absolute() {
        relative.to_path_buf()
    } else {
        base.parent()
            .unwrap_or_else(|| Path::new("."))
            .join(relative)
    }
}

fn summarize_state(combat: &crate::combat::CombatState) -> BossValidationStateSummary {
    let pressure = StatePressureFeatures::from_combat(combat);
    let mut attack_cards = 0usize;
    let mut skill_cards = 0usize;
    let mut power_cards = 0usize;
    let mut playable_cards = 0usize;
    for card in &combat.zones.hand {
        let def = get_card_definition(card.id);
        match def.card_type {
            CardType::Attack => attack_cards += 1,
            CardType::Skill => skill_cards += 1,
            CardType::Power => power_cards += 1,
            _ => {}
        }
        if crate::content::cards::can_play_card(card, combat).is_ok() {
            playable_cards += 1;
        }
    }
    BossValidationStateSummary {
        player_hp: combat.entities.player.current_hp,
        player_block: combat.entities.player.block,
        energy: combat.turn.energy,
        enemy_total_hp: combat
            .entities
            .monsters
            .iter()
            .map(|monster| monster.current_hp.max(0))
            .sum(),
        visible_incoming: pressure.visible_incoming,
        hand_size: combat.zones.hand.len(),
        playable_cards,
        attack_cards_in_hand: attack_cards,
        skill_cards_in_hand: skill_cards,
        power_cards_in_hand: power_cards,
    }
}

fn summarize_deck(start_spec: &CombatStartSpec) -> String {
    start_spec
        .master_deck
        .iter()
        .map(|card| match card {
            crate::testing::fixtures::author_spec::AuthorCardSpec::Simple(id) => id.clone(),
            crate::testing::fixtures::author_spec::AuthorCardSpec::Detailed(entry) => {
                if entry.upgrades > 0 {
                    format!("{}+{}", entry.id, entry.upgrades)
                } else {
                    entry.id.clone()
                }
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::bot::search::DecisionAuditConfig;

    use super::{validate_case, BossPreferenceVerdict};

    fn repo_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)
    }

    #[test]
    fn hexaghost_v1_h1_prefers_disarm_now() {
        let result = validate_case(
            &repo_path("data/boss_validation/hexaghost_v1/state_case_h1_disarm_now.json"),
            DecisionAuditConfig::default(),
        )
        .expect("validate H1");
        assert_eq!(result.actual_verdict, BossPreferenceVerdict::PreferA);
    }

    #[test]
    fn hexaghost_v1_h2_prefers_pressure_relief() {
        let result = validate_case(
            &repo_path(
                "data/boss_validation/hexaghost_v1/state_case_h2_reduce_pressure_vs_race.json",
            ),
            DecisionAuditConfig::default(),
        )
        .expect("validate H2");
        assert_eq!(result.actual_verdict, BossPreferenceVerdict::PreferA);
    }

    #[test]
    fn hexaghost_v1_h3_stays_close_enough() {
        let result = validate_case(
            &repo_path("data/boss_validation/hexaghost_v1/state_case_h3_close_enough.json"),
            DecisionAuditConfig::default(),
        )
        .expect("validate H3");
        assert_eq!(result.actual_verdict, BossPreferenceVerdict::CloseEnough);
    }

    #[test]
    fn guardian_v1_g1_prefers_proactive_line() {
        let result = validate_case(
            &repo_path(
                "data/boss_validation/guardian_v1/state_case_g1_proactive_vs_overblock.json",
            ),
            DecisionAuditConfig::default(),
        )
        .expect("validate G1");
        assert_eq!(result.actual_verdict, BossPreferenceVerdict::PreferA);
    }

    #[test]
    fn guardian_v1_g2_prefers_proactive_line_even_when_it_is_b() {
        let result = validate_case(
            &repo_path(
                "data/boss_validation/guardian_v1/state_case_g2_overblock_vs_proactive.json",
            ),
            DecisionAuditConfig::default(),
        )
        .expect("validate G2");
        assert_eq!(result.actual_verdict, BossPreferenceVerdict::PreferB);
    }

    #[test]
    fn guardian_v1_g3_stays_close_enough() {
        let result = validate_case(
            &repo_path("data/boss_validation/guardian_v1/state_case_g3_close_enough_opening.json"),
            DecisionAuditConfig::default(),
        )
        .expect("validate G3");
        assert_eq!(result.actual_verdict, BossPreferenceVerdict::CloseEnough);
    }
}
