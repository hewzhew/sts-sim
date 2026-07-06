use sts_simulator::ai::combat_search_v2::{CombatSearchV2ActionPreview, CombatSearchV2Report};

use super::super::search_types::SearchDiagnosticProgressFacts;

pub(super) fn diagnostic_progress_facts(
    report: &CombatSearchV2Report,
    action_preview_limit: usize,
) -> Option<SearchDiagnosticProgressFacts> {
    if let Some(trajectory) = report.best_complete_trajectory.as_ref() {
        return Some(SearchDiagnosticProgressFacts {
            source: "best_complete",
            terminal: trajectory.terminal,
            estimated: trajectory.estimated,
            final_hp: trajectory.final_hp,
            hp_loss: trajectory.hp_loss,
            turns: trajectory.turns,
            potions_used: trajectory.potions_used,
            cards_played: trajectory.cards_played,
            living_enemy_count: trajectory.final_state.living_enemy_count,
            total_enemy_hp: trajectory.final_state.total_enemy_hp,
            half_dead_enemy_count: trajectory
                .final_state
                .enemy_slots
                .iter()
                .filter(|enemy| enemy.half_dead)
                .count(),
            visible_incoming_damage: Some(trajectory.final_state.visible_incoming_damage),
            action_count: Some(trajectory.actions.len()),
            exact_prefix_action_count: Some(trajectory.actions.len()),
            action_key_preview: trajectory
                .actions
                .iter()
                .take(action_preview_limit)
                .map(|action| action.action_key.clone())
                .collect(),
            input_preview: trajectory
                .actions
                .iter()
                .take(action_preview_limit)
                .map(|action| action.input.clone())
                .collect(),
            full_action_preview: trajectory
                .actions
                .iter()
                .map(|action| CombatSearchV2ActionPreview {
                    action_key: action.action_key.clone(),
                    input: action.input.clone(),
                })
                .collect(),
        });
    }
    report
        .rollout
        .best_frontier_estimate
        .as_ref()
        .map(|rollout| {
            let frontier = report.best_frontier_trajectory.as_ref();
            let exact_prefix_actions = frontier
                .map(|trajectory| trajectory.actions.as_slice())
                .unwrap_or(&[]);
            let exact_prefix_action_count = Some(exact_prefix_actions.len());
            SearchDiagnosticProgressFacts {
                source: "rollout_frontier",
                terminal: rollout.terminal,
                estimated: rollout.estimated,
                final_hp: rollout.final_hp,
                hp_loss: rollout.hp_loss,
                turns: rollout.turns,
                potions_used: rollout.potions_used,
                cards_played: rollout.cards_played,
                living_enemy_count: rollout.living_enemy_count,
                total_enemy_hp: rollout.total_enemy_hp,
                half_dead_enemy_count: frontier
                    .map(|trajectory| {
                        trajectory
                            .final_state
                            .enemy_slots
                            .iter()
                            .filter(|enemy| enemy.half_dead)
                            .count()
                    })
                    .unwrap_or_default(),
                visible_incoming_damage: frontier
                    .map(|trajectory| trajectory.final_state.visible_incoming_damage),
                action_count: Some(
                    rollout
                        .actions_simulated
                        .saturating_add(exact_prefix_actions.len()),
                ),
                exact_prefix_action_count,
                action_key_preview: rollout
                    .action_preview
                    .iter()
                    .take(action_preview_limit)
                    .map(|action| action.action_key.clone())
                    .collect(),
                input_preview: rollout
                    .action_preview
                    .iter()
                    .take(action_preview_limit)
                    .map(|action| action.input.clone())
                    .collect(),
                full_action_preview: rollout.action_preview.clone(),
            }
        })
}
