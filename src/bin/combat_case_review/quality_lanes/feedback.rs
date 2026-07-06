use sts_simulator::ai::combat_search_v2::{
    compile_combat_search_witness_prior_v0, CombatSearchV2ActionPreview, CombatSearchV2WitnessLine,
    SearchTerminalLabel,
};
use sts_simulator::eval::combat_case::CombatCase;

use super::super::search_runner::run_configured_search;
use super::super::search_types::{SearchDiagnosticProgressFacts, SearchReview};
use super::types::{
    CombatSuccessFeedbackComparison, CombatSuccessFeedbackMetrics, CombatSuccessFeedbackRerun,
    CombatSuccessFeedbackSource,
};

pub(super) fn run_success_feedback_rerun(
    case: &CombatCase,
    source: CombatSuccessFeedbackSource,
    max_nodes: usize,
    wall_ms: u64,
    action_preview_limit: usize,
) -> Option<CombatSuccessFeedbackRerun> {
    let witness_prior = compile_combat_search_witness_prior_v0(&case.position, &source.witness);
    if witness_prior.prior.is_empty() {
        return None;
    }
    let prior_states = witness_prior.prior_states;
    let duplicate_prior_hints = witness_prior.duplicate_prior_hints;
    let mut config = source.spec.config(max_nodes, wall_ms);
    config.input_label = Some(format!("success_feedback_rerun:{}", source.spec.label));
    config.root_action_prior = Some(witness_prior.prior);
    let (rerun, _report) = run_configured_search(
        "quality_success_feedback_rerun",
        case,
        config,
        action_preview_limit,
    );
    let comparison = compare_success_feedback(&source.baseline, &rerun);
    Some(CombatSuccessFeedbackRerun {
        schema: "combat_success_feedback_rerun_v0",
        contract: "best_complete_or_estimated_rollout_witness_compiled_to_exact_state_action_prior_then_rerun_with_same_lane_budget",
        source_kind: source.source_kind,
        source_lane: source.spec.label,
        witness_action_count: source.witness.actions.len(),
        prior_states,
        duplicate_prior_hints,
        baseline: source.baseline,
        rerun,
        comparison,
    })
}

pub(super) fn estimated_rollout_feedback_witness(
    source: &'static str,
    progress: &SearchDiagnosticProgressFacts,
) -> Option<CombatSearchV2WitnessLine> {
    if progress.source != "rollout_frontier"
        || progress.terminal != SearchTerminalLabel::Win
        || !progress.estimated
    {
        return None;
    }
    let exact_prefix_action_count = progress.exact_prefix_action_count?;
    if exact_prefix_action_count == 0 {
        return None;
    }
    let actions = progress
        .action_key_preview
        .iter()
        .cloned()
        .zip(progress.input_preview.iter().cloned())
        .take(exact_prefix_action_count)
        .map(|(action_key, input)| CombatSearchV2ActionPreview { action_key, input })
        .collect::<Vec<_>>();
    if actions.is_empty() {
        return None;
    }
    Some(CombatSearchV2WitnessLine {
        source,
        terminal: progress.terminal,
        final_hp: progress.final_hp,
        total_enemy_hp: progress.total_enemy_hp,
        action_count: progress.action_count,
        actions,
    })
}

pub(super) fn estimated_rollout_feedback_rank(
    progress: &SearchDiagnosticProgressFacts,
) -> (i32, i32, i32, i32) {
    (
        progress.final_hp,
        -(progress.potions_used as i32),
        -(progress.total_enemy_hp),
        -(progress.action_count.unwrap_or(usize::MAX) as i32),
    )
}

fn compare_success_feedback(
    baseline: &CombatSuccessFeedbackMetrics,
    rerun: &SearchReview,
) -> CombatSuccessFeedbackComparison {
    let first_win_nodes_delta = match (baseline.nodes_to_first_win, rerun.nodes_to_first_win) {
        (Some(base), Some(next)) => Some(next as i64 - base as i64),
        _ => None,
    };
    CombatSuccessFeedbackComparison {
        rerun_found_win: rerun.complete_win,
        first_win_nodes_delta,
        terminal_wins_delta: rerun.terminal_wins as i64 - baseline.terminal_wins as i64,
        final_hp_delta: baseline
            .final_hp
            .zip(rerun.final_hp)
            .map(|(base, next)| next - base),
        hp_loss_delta: baseline
            .hp_loss
            .zip(rerun.hp_loss)
            .map(|(base, next)| next - base),
        potions_used_delta: baseline
            .potions_used
            .zip(rerun.potions_used)
            .map(|(base, next)| next as i32 - base as i32),
        easier_first_win: first_win_nodes_delta.map(|delta| delta < 0),
    }
}

#[cfg(test)]
mod tests {
    use sts_simulator::ai::combat_search_v2::{CombatSearchV2ActionPreview, SearchTerminalLabel};
    use sts_simulator::state::core::ClientInput;

    use super::*;

    #[test]
    fn estimated_rollout_win_progress_can_become_feedback_witness() {
        let progress = SearchDiagnosticProgressFacts {
            source: "rollout_frontier",
            terminal: SearchTerminalLabel::Win,
            estimated: true,
            final_hp: 12,
            hp_loss: 8,
            turns: 3,
            potions_used: 1,
            cards_played: 4,
            living_enemy_count: 0,
            total_enemy_hp: 0,
            half_dead_enemy_count: 0,
            visible_incoming_damage: Some(0),
            action_count: Some(6),
            exact_prefix_action_count: Some(2),
            action_key_preview: vec!["a".to_string(), "b".to_string()],
            input_preview: vec![ClientInput::EndTurn, ClientInput::EndTurn],
            full_action_preview: vec![
                CombatSearchV2ActionPreview {
                    action_key: "a".to_string(),
                    input: ClientInput::EndTurn,
                },
                CombatSearchV2ActionPreview {
                    action_key: "b".to_string(),
                    input: ClientInput::EndTurn,
                },
            ],
        };

        let witness = estimated_rollout_feedback_witness("lane", &progress)
            .expect("rollout win progress should be reusable as witness");

        assert_eq!(witness.source, "lane");
        assert_eq!(witness.terminal, SearchTerminalLabel::Win);
        assert_eq!(witness.action_count, Some(6));
        assert_eq!(witness.actions.len(), 2);
    }

    #[test]
    fn non_winning_rollout_progress_is_not_feedback_witness() {
        let progress = SearchDiagnosticProgressFacts {
            source: "rollout_frontier",
            terminal: SearchTerminalLabel::Loss,
            estimated: true,
            final_hp: 0,
            hp_loss: 30,
            turns: 2,
            potions_used: 0,
            cards_played: 3,
            living_enemy_count: 2,
            total_enemy_hp: 100,
            half_dead_enemy_count: 0,
            visible_incoming_damage: Some(20),
            action_count: Some(4),
            exact_prefix_action_count: Some(2),
            action_key_preview: vec!["a".to_string()],
            input_preview: vec![ClientInput::EndTurn],
            full_action_preview: vec![CombatSearchV2ActionPreview {
                action_key: "a".to_string(),
                input: ClientInput::EndTurn,
            }],
        };

        assert!(estimated_rollout_feedback_witness("lane", &progress).is_none());
    }
}
