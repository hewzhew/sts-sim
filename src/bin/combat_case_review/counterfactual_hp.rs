use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    replay_combat_search_witness_line_v0, CombatSearchV2WitnessLine,
};
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::sim::combat::CombatTerminal;

use super::quality_lanes::{
    combat_line_quality, compare_quality, quality_lane_specs, witness_line_from_trajectory,
    CombatLineQuality,
};
use super::search_runner::run_configured_search;
use super::search_types::SearchReview;
use super::Args;

#[derive(Serialize)]
pub(super) struct CounterfactualHpProbe {
    schema: &'static str,
    contract: &'static str,
    original_hp: i32,
    max_hp: i32,
    levels: Vec<CounterfactualHpLevel>,
    classification: CounterfactualHpClassification,
}

#[derive(Serialize)]
struct CounterfactualHpLevel {
    label: String,
    hp: i32,
    selected_lane: Option<&'static str>,
    complete_win: bool,
    quality: Option<CombatLineQuality>,
    nodes_to_first_win: Option<u64>,
    total_terminal_wins: u64,
    replay_on_original_hp: Option<CounterfactualHpReplay>,
}

#[derive(Serialize)]
struct CounterfactualHpReplay {
    terminal: CombatTerminal,
    final_hp: i32,
    total_enemy_hp: i32,
    living_enemy_count: usize,
    replayed_actions: usize,
    action_count: Option<usize>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum CounterfactualHpClassification {
    OriginalHpWin,
    CounterfactualLineStillWinsOriginalHp,
    CounterfactualOnlyWin,
    NoWinFound,
}

struct CounterfactualHpCandidate {
    lane: &'static str,
    review: SearchReview,
    quality: CombatLineQuality,
    witness: CombatSearchV2WitnessLine,
}

pub(super) fn run_counterfactual_hp_probe(args: &Args, case: &CombatCase) -> CounterfactualHpProbe {
    let original_hp = case.position.combat.entities.player.current_hp;
    let max_hp = case.position.combat.entities.player.max_hp.max(1);
    let levels = counterfactual_hp_targets(&args.counterfactual_hp_levels, original_hp, max_hp)
        .into_iter()
        .map(|(label, hp)| run_counterfactual_hp_level(args, case, label, hp))
        .collect::<Vec<_>>();
    let classification = classify_counterfactual_hp_probe(&levels, original_hp);
    CounterfactualHpProbe {
        schema: "counterfactual_hp_probe_v0",
        contract: "diagnostic_only_mutate_root_player_hp_then_replay_found_win_line_on_original_hp_no_runner_policy_change",
        original_hp,
        max_hp,
        levels,
        classification,
    }
}

fn run_counterfactual_hp_level(
    args: &Args,
    original_case: &CombatCase,
    label: String,
    hp: i32,
) -> CounterfactualHpLevel {
    let case = combat_case_with_player_hp(original_case, hp);
    let specs = quality_lane_specs();
    let lane_count = specs.len().max(1);
    let total_nodes = args
        .quality_lane_total_nodes
        .unwrap_or(args.slow_nodes)
        .max(1);
    let total_wall_ms = args.quality_lane_total_ms.unwrap_or(args.slow_ms).max(1);
    let per_lane_nodes = (total_nodes / lane_count).max(1);
    let per_lane_wall_ms = (total_wall_ms / lane_count as u64).max(1);
    let mut best: Option<CounterfactualHpCandidate> = None;
    let mut total_terminal_wins = 0;
    for spec in specs {
        let (review, report) = run_configured_search(
            spec.label,
            &case,
            spec.config(per_lane_nodes, per_lane_wall_ms),
            args.action_preview_limit,
        );
        total_terminal_wins += review.terminal_wins;
        let quality = combat_line_quality(&report);
        let witness = report
            .best_win_trajectory
            .as_ref()
            .map(|trajectory| witness_line_from_trajectory(spec.label, trajectory));
        if let (Some(quality), Some(witness)) = (quality, witness) {
            let candidate = CounterfactualHpCandidate {
                lane: spec.label,
                review,
                quality,
                witness,
            };
            if best
                .as_ref()
                .is_none_or(|current| compare_quality(&candidate.quality, &current.quality).is_gt())
            {
                best = Some(candidate);
            }
        }
    }
    let replay_on_original_hp = best.as_ref().map(|candidate| {
        let replay =
            replay_combat_search_witness_line_v0(&original_case.position, &candidate.witness);
        CounterfactualHpReplay {
            terminal: replay.terminal,
            final_hp: replay.final_hp,
            total_enemy_hp: replay.total_enemy_hp,
            living_enemy_count: replay.living_enemy_count,
            replayed_actions: replay.replayed_actions,
            action_count: replay.action_count,
        }
    });
    CounterfactualHpLevel {
        label,
        hp,
        selected_lane: best.as_ref().map(|candidate| candidate.lane),
        complete_win: best.is_some(),
        quality: best.as_ref().map(|candidate| candidate.quality.clone()),
        nodes_to_first_win: best
            .as_ref()
            .and_then(|candidate| candidate.review.nodes_to_first_win),
        total_terminal_wins,
        replay_on_original_hp,
    }
}

fn combat_case_with_player_hp(case: &CombatCase, hp: i32) -> CombatCase {
    let mut case = case.clone();
    let max_hp = case.position.combat.entities.player.max_hp.max(1);
    let hp = hp.clamp(1, max_hp);
    case.position.combat.entities.player.current_hp = hp;
    case.run.hp = hp;
    case.combat.hp = hp;
    case
}

fn counterfactual_hp_targets(levels: &str, original_hp: i32, max_hp: i32) -> Vec<(String, i32)> {
    let mut targets = Vec::new();
    for token in levels
        .split(',')
        .map(str::trim)
        .filter(|token| !token.is_empty())
    {
        let normalized = token.to_ascii_lowercase();
        let hp = match normalized.as_str() {
            "real" | "original" => Some(original_hp),
            "half" => Some((max_hp + 1) / 2),
            "full" | "max" => Some(max_hp),
            _ => normalized.parse::<i32>().ok(),
        };
        if let Some(hp) = hp {
            let hp = hp.clamp(1, max_hp);
            if !targets.iter().any(|(_, existing_hp)| *existing_hp == hp) {
                targets.push((token.to_string(), hp));
            }
        }
    }
    if targets.is_empty() {
        targets.push(("real".to_string(), original_hp.clamp(1, max_hp)));
    }
    targets
}

fn classify_counterfactual_hp_probe(
    levels: &[CounterfactualHpLevel],
    original_hp: i32,
) -> CounterfactualHpClassification {
    if levels
        .iter()
        .any(|level| level.hp == original_hp && level.complete_win)
    {
        return CounterfactualHpClassification::OriginalHpWin;
    }
    if levels.iter().any(|level| {
        level.hp != original_hp
            && level
                .replay_on_original_hp
                .as_ref()
                .is_some_and(|replay| matches!(replay.terminal, CombatTerminal::Win))
    }) {
        return CounterfactualHpClassification::CounterfactualLineStillWinsOriginalHp;
    }
    if levels
        .iter()
        .any(|level| level.hp != original_hp && level.complete_win)
    {
        return CounterfactualHpClassification::CounterfactualOnlyWin;
    }
    CounterfactualHpClassification::NoWinFound
}
