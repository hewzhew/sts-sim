use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    CombatSearchActionPriorPluginId, CombatSearchProfile, CombatSearchV2EnemySummary,
};
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::eval::combat_case::CombatCase;

use super::focus::{review_focus, CombatReviewFocus};
use super::options::ReviewOptions;
use super::search_runner::{review_all_potions_profile, run_profile_search};
use super::search_types::SearchReview;

#[derive(Serialize)]
pub(super) struct CollectorTacticLaneReview {
    schema: &'static str,
    contract: &'static str,
    total_nodes: usize,
    total_wall_ms: u64,
    per_lane_nodes: usize,
    per_lane_wall_ms: u64,
    skipped_reason: Option<&'static str>,
    lanes: Vec<CollectorTacticLaneResult>,
}

#[derive(Serialize)]
struct CollectorTacticLaneResult {
    lane: &'static str,
    intent: &'static str,
    action_prior_policy: &'static str,
    search: SearchReview,
    focus: Option<CombatReviewFocus>,
    enemy_final_state: Vec<CombatSearchV2EnemySummary>,
}

#[derive(Clone, Copy, Debug)]
struct CollectorTacticLaneSpec {
    lane: &'static str,
    intent: &'static str,
    prior: CombatSearchActionPriorPluginId,
}

impl CollectorTacticLaneSpec {
    fn profile(
        self,
        options: &ReviewOptions,
        max_nodes: usize,
        wall_ms: u64,
    ) -> CombatSearchProfile {
        review_all_potions_profile(self.lane, max_nodes, wall_ms, options)
            .with_action_prior_plugin(self.prior)
    }
}

pub(super) fn run_collector_tactic_lanes(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<CollectorTacticLaneReview> {
    if !options.collector_tactic_lanes {
        return None;
    }

    let total_nodes = options
        .quality_lane_total_nodes
        .unwrap_or(options.slow_nodes)
        .max(1);
    let total_wall_ms = options
        .quality_lane_total_ms
        .unwrap_or(options.slow_ms)
        .max(1);
    let specs = collector_tactic_lane_specs();
    let (per_lane_nodes, per_lane_wall_ms) = split_budget(total_nodes, total_wall_ms, specs.len());

    if !is_collector_fight(case) {
        return Some(CollectorTacticLaneReview {
            schema: "collector_tactic_lane_review_v0",
            contract: "review_only_same_total_budget_split_across_two_collector_tactics_no_runner_policy_change",
            total_nodes,
            total_wall_ms,
            per_lane_nodes,
            per_lane_wall_ms,
            skipped_reason: Some("not_collector_fight"),
            lanes: Vec::new(),
        });
    }

    let lanes = specs
        .into_iter()
        .map(|spec| {
            let profile = spec.profile(options, per_lane_nodes, per_lane_wall_ms);
            let (search, report) = run_profile_search(case, profile, options.action_preview_limit);
            let focus = review_focus(std::slice::from_ref(&search));
            let enemy_final_state = report
                .best_complete_trajectory
                .as_ref()
                .map(|trajectory| trajectory.enemy_final_state.clone())
                .unwrap_or_default();
            CollectorTacticLaneResult {
                lane: spec.lane,
                intent: spec.intent,
                action_prior_policy: spec.prior.label(),
                search,
                focus,
                enemy_final_state,
            }
        })
        .collect();

    Some(CollectorTacticLaneReview {
        schema: "collector_tactic_lane_review_v0",
        contract:
            "review_only_same_total_budget_split_across_two_collector_tactics_no_runner_policy_change",
        total_nodes,
        total_wall_ms,
        per_lane_nodes,
        per_lane_wall_ms,
        skipped_reason: None,
        lanes,
    })
}

fn collector_tactic_lane_specs() -> [CollectorTacticLaneSpec; 2] {
    [
        CollectorTacticLaneSpec {
            lane: "collector_single_head_control",
            intent: "focus one Torch Head, preserve the last head, then damage The Collector",
            prior: CombatSearchActionPriorPluginId::CollectorSingleHeadControl,
        },
        CollectorTacticLaneSpec {
            lane: "collector_boss_race",
            intent: "prefer direct damage to The Collector over Torch Head cleanup",
            prior: CombatSearchActionPriorPluginId::CollectorBossRace,
        },
    ]
}

fn split_budget(total_nodes: usize, total_wall_ms: u64, lane_count: usize) -> (usize, u64) {
    let lane_count = lane_count.max(1);
    (
        (total_nodes / lane_count).max(1),
        (total_wall_ms / lane_count as u64).max(1),
    )
}

fn is_collector_fight(case: &CombatCase) -> bool {
    case.position
        .combat
        .entities
        .monsters
        .iter()
        .any(|monster| {
            monster.is_alive_for_action()
                && EnemyId::from_id(monster.monster_type) == Some(EnemyId::TheCollector)
        })
}

#[cfg(test)]
mod tests {
    use sts_simulator::ai::combat_search_v2::CombatSearchActionPriorPluginId;

    use super::{collector_tactic_lane_specs, split_budget};

    #[test]
    fn collector_tactic_lane_specs_contain_exactly_the_two_approved_priors() {
        let specs = collector_tactic_lane_specs();

        assert_eq!(specs.len(), 2);
        assert_eq!(
            specs[0].prior,
            CombatSearchActionPriorPluginId::CollectorSingleHeadControl
        );
        assert_eq!(
            specs[1].prior,
            CombatSearchActionPriorPluginId::CollectorBossRace
        );
    }

    #[test]
    fn collector_tactic_lanes_split_one_total_budget_evenly() {
        assert_eq!(split_budget(1_600_000, 20_000, 2), (800_000, 10_000));
    }
}
