use std::time::Duration;

use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    compile_combat_search_witness_prior_v0, CombatSearchV2ActionPreview, CombatSearchV2Config,
    CombatSearchV2PhaseGuardPolicy, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2TurnPlanPolicy, CombatSearchV2WitnessLine, CombatSearchV2WitnessReplay,
    SearchTerminalLabel,
};
use sts_simulator::eval::combat_case::CombatCase;
use sts_simulator::sim::combat::CombatTerminal;

use super::options::ReviewOptions;
use super::search_runner::run_configured_search;
use super::search_types::{SearchDiagnosticProgressFacts, SearchReview};

#[derive(Serialize)]
pub(super) struct CombatReviewFocus {
    pub(super) selected_review: &'static str,
    pub(super) reason: &'static str,
    pub(super) progress: SearchDiagnosticProgressFacts,
}

#[derive(Serialize)]
pub(super) struct CombatReviewFocusPriorRerun {
    selected_review: &'static str,
    witness_replayed_actions: usize,
    witness_action_count: Option<usize>,
    witness_terminal: CombatTerminal,
    prior_states: usize,
    duplicate_prior_hints: usize,
    rerun: SearchReview,
}

pub(super) fn review_focus(ladder: &[SearchReview]) -> Option<CombatReviewFocus> {
    let mut selected: Option<(&SearchReview, &SearchDiagnosticProgressFacts)> = None;
    for review in ladder {
        let Some(progress) = review.facts.diagnostic_progress.as_ref() else {
            continue;
        };
        if selected
            .map(|(_, current)| progress_is_better_focus(progress, current))
            .unwrap_or(true)
        {
            selected = Some((review, progress));
        }
    }
    selected.map(|(review, progress)| CombatReviewFocus {
        selected_review: review.label,
        reason: focus_reason(progress),
        progress: progress.clone(),
    })
}

pub(super) fn focus_witness_line(focus: &CombatReviewFocus) -> CombatSearchV2WitnessLine {
    let actions = if focus.progress.full_action_preview.is_empty() {
        focus
            .progress
            .action_key_preview
            .iter()
            .cloned()
            .zip(focus.progress.input_preview.iter().cloned())
            .map(|(action_key, input)| CombatSearchV2ActionPreview { action_key, input })
            .collect()
    } else {
        focus.progress.full_action_preview.clone()
    };
    CombatSearchV2WitnessLine {
        source: focus.progress.source,
        terminal: focus.progress.terminal,
        final_hp: focus.progress.final_hp,
        total_enemy_hp: focus.progress.total_enemy_hp,
        action_count: focus.progress.action_count,
        actions,
    }
}

pub(super) fn witness_prior_rerun(
    options: &ReviewOptions,
    case: &CombatCase,
    focus: &CombatReviewFocus,
    replay: &CombatSearchV2WitnessReplay,
) -> Option<CombatReviewFocusPriorRerun> {
    if focus.progress.source != "rollout_frontier"
        || !matches!(replay.terminal, CombatTerminal::Win)
    {
        return None;
    }
    let witness_prior =
        compile_combat_search_witness_prior_v0(&case.position, &focus_witness_line(focus));
    if witness_prior.prior.is_empty() {
        return None;
    }
    let prior_states = witness_prior.prior_states;
    let duplicate_prior_hints = witness_prior.duplicate_prior_hints;
    let rollout_policy = if options.disable_rollout {
        CombatSearchV2RolloutPolicy::Disabled
    } else {
        CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion
    };
    let (rerun, _) = run_configured_search(
        "focus_witness_prior_rerun",
        case,
        CombatSearchV2Config {
            max_nodes: options.fast_nodes,
            wall_time: Some(Duration::from_millis(options.fast_ms)),
            turn_plan_policy: CombatSearchV2TurnPlanPolicy::DiagnosticOnly,
            potion_policy: CombatSearchV2PotionPolicy::Never,
            max_potions_used: Some(0),
            phase_guard_policy: CombatSearchV2PhaseGuardPolicy::Default,
            rollout_policy,
            child_rollout_policy: options.child_rollout_policy(),
            root_action_prior: Some(witness_prior.prior),
            ..CombatSearchV2Config::default()
        },
        options.action_preview_limit,
    );
    Some(CombatReviewFocusPriorRerun {
        selected_review: focus.selected_review,
        witness_replayed_actions: replay.replayed_actions,
        witness_action_count: focus.progress.action_count,
        witness_terminal: replay.terminal,
        prior_states,
        duplicate_prior_hints,
        rerun,
    })
}

fn progress_is_better_focus(
    candidate: &SearchDiagnosticProgressFacts,
    current: &SearchDiagnosticProgressFacts,
) -> bool {
    match (
        candidate.terminal == SearchTerminalLabel::Win,
        current.terminal == SearchTerminalLabel::Win,
    ) {
        (true, false) => return true,
        (false, true) => return false,
        (true, true) => {
            return (candidate.final_hp, -(candidate.potions_used as i32))
                > (current.final_hp, -(current.potions_used as i32));
        }
        (false, false) => {}
    }

    (
        -(candidate.half_dead_enemy_count as i32),
        -candidate.total_enemy_hp,
        -(candidate.living_enemy_count as i32),
        candidate.turns as i32,
        candidate.final_hp,
        -(candidate.potions_used as i32),
    ) > (
        -(current.half_dead_enemy_count as i32),
        -current.total_enemy_hp,
        -(current.living_enemy_count as i32),
        current.turns as i32,
        current.final_hp,
        -(current.potions_used as i32),
    )
}

fn focus_reason(progress: &SearchDiagnosticProgressFacts) -> &'static str {
    if progress.terminal == SearchTerminalLabel::Win {
        "complete_win_available"
    } else if progress.final_hp <= 0 && progress.half_dead_enemy_count > 0 {
        "phase_pending_enemy_player_died"
    } else {
        "closest_failure_progress_by_enemy_hp"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::ai::combat_search_v2::CombatSearchV2ActionPreview;
    use sts_simulator::state::core::ClientInput;

    fn progress_with_hidden_full_actions() -> SearchDiagnosticProgressFacts {
        SearchDiagnosticProgressFacts {
            source: "best_complete",
            terminal: SearchTerminalLabel::Loss,
            estimated: false,
            final_hp: 0,
            hp_loss: 10,
            turns: 1,
            potions_used: 0,
            cards_played: 0,
            living_enemy_count: 1,
            total_enemy_hp: 20,
            half_dead_enemy_count: 0,
            visible_incoming_damage: None,
            action_count: Some(2),
            exact_prefix_action_count: Some(2),
            action_key_preview: vec!["preview_only".to_string()],
            input_preview: vec![ClientInput::EndTurn],
            full_action_preview: vec![
                CombatSearchV2ActionPreview {
                    action_key: "full_1".to_string(),
                    input: ClientInput::EndTurn,
                },
                CombatSearchV2ActionPreview {
                    action_key: "full_2".to_string(),
                    input: ClientInput::Cancel,
                },
            ],
        }
    }

    #[test]
    fn focus_witness_line_prefers_hidden_full_actions_over_json_preview() {
        let focus = CombatReviewFocus {
            selected_review: "test",
            reason: "test",
            progress: progress_with_hidden_full_actions(),
        };

        let witness = focus_witness_line(&focus);

        assert_eq!(
            witness
                .actions
                .iter()
                .map(|action| action.action_key.as_str())
                .collect::<Vec<_>>(),
            vec!["full_1", "full_2"]
        );
    }

    #[test]
    fn focus_reason_marks_phase_pending_enemy_death_separately() {
        let mut progress = progress_with_hidden_full_actions();
        progress.final_hp = 0;
        progress.total_enemy_hp = 0;
        progress.living_enemy_count = 0;
        progress.half_dead_enemy_count = 1;

        assert_eq!(focus_reason(&progress), "phase_pending_enemy_player_died");
    }

    #[test]
    fn focus_ranking_does_not_treat_half_dead_zero_hp_as_plain_enemy_clear() {
        let mut phase_pending = progress_with_hidden_full_actions();
        phase_pending.final_hp = 0;
        phase_pending.total_enemy_hp = 0;
        phase_pending.living_enemy_count = 0;
        phase_pending.half_dead_enemy_count = 1;

        let mut ordinary_failure = progress_with_hidden_full_actions();
        ordinary_failure.final_hp = 0;
        ordinary_failure.total_enemy_hp = 10;
        ordinary_failure.living_enemy_count = 1;
        ordinary_failure.half_dead_enemy_count = 0;

        assert!(progress_is_better_focus(&ordinary_failure, &phase_pending));
        assert!(!progress_is_better_focus(&phase_pending, &ordinary_failure));
    }
}
