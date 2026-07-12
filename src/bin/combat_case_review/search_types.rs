use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{CombatSearchV2ActionPreview, SearchTerminalLabel};
use sts_simulator::eval::run_control::CombatCaseCandidateAdjudicationCensusV1;
use sts_simulator::state::core::ClientInput;

#[path = "search_types/performance.rs"]
mod performance;

pub(super) use performance::{SearchPerformanceReview, SearchRolloutPerformanceReview};

#[derive(Serialize)]
pub(super) struct SearchReview {
    pub(super) label: &'static str,
    pub(super) nodes: usize,
    pub(super) wall_ms: u64,
    pub(super) rollout_policy: &'static str,
    pub(super) turn_plan_policy: &'static str,
    pub(super) phase_guard_policy: &'static str,
    pub(super) setup_bias_policy: &'static str,
    pub(super) child_rollout_policy: &'static str,
    pub(super) potion_policy: &'static str,
    pub(super) max_potions_used: Option<u32>,
    pub(super) complete_win: bool,
    pub(super) hp_loss: Option<i32>,
    pub(super) final_hp: Option<i32>,
    pub(super) turns: Option<u32>,
    pub(super) potions_used: Option<u32>,
    pub(super) nodes_expanded: u64,
    pub(super) nodes_generated: u64,
    pub(super) nodes_to_first_win: Option<u64>,
    pub(super) terminal_wins: u64,
    pub(super) elapsed_ms: u128,
    pub(super) deadline_hit: bool,
    pub(super) node_budget_hit: bool,
    pub(super) performance: SearchPerformanceReview,
    pub(super) facts: SearchReviewFacts,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) candidate_adjudication_census: Option<CombatCaseCandidateAdjudicationCensusV1>,
}

impl SearchReview {
    pub(super) fn attach_candidate_adjudication_census(
        &mut self,
        census: CombatCaseCandidateAdjudicationCensusV1,
    ) -> bool {
        if self.label != census.source_review() {
            return false;
        }
        self.candidate_adjudication_census = Some(census);
        true
    }
}

#[derive(Serialize)]
pub(super) struct SearchReviewFacts {
    pub(super) diagnostic_progress: Option<SearchDiagnosticProgressFacts>,
}

#[derive(Clone, Serialize)]
pub(super) struct SearchDiagnosticProgressFacts {
    pub(super) source: &'static str,
    pub(super) terminal: SearchTerminalLabel,
    pub(super) estimated: bool,
    pub(super) final_hp: i32,
    pub(super) hp_loss: i32,
    pub(super) turns: u32,
    pub(super) potions_used: u32,
    pub(super) cards_played: u32,
    pub(super) living_enemy_count: usize,
    pub(super) total_enemy_hp: i32,
    pub(super) half_dead_enemy_count: usize,
    pub(super) visible_incoming_damage: Option<i32>,
    pub(super) action_count: Option<usize>,
    pub(super) exact_prefix_action_count: Option<usize>,
    pub(super) action_key_preview: Vec<String>,
    pub(super) input_preview: Vec<ClientInput>,
    #[serde(skip)]
    pub(super) full_action_preview: Vec<CombatSearchV2ActionPreview>,
}

#[cfg(test)]
mod tests {
    use serde_json::Value;
    use sts_simulator::eval::run_control::CombatCaseCandidateAdjudicationCensusV1;

    use super::*;

    fn zero_performance() -> SearchPerformanceReview {
        SearchPerformanceReview {
            total_us: 0,
            rollout_us: 0,
            rollout_calls: 0,
            root_rollout_calls: 0,
            child_rollout_calls: 0,
            deferred_child_rollout_calls: 0,
            turn_plan_seed_rollout_calls: 0,
            rollout_evaluations: 0,
            rollout_budget_skips: 0,
            rollout_max_evaluation_budget_skips: 0,
            rollout_deadline_budget_skips: 0,
            deferred_child_rollout_admitted_signal: 0,
            deferred_child_rollout_admitted_periodic: 0,
            deferred_child_rollout_skipped_low_signal: 0,
            deferred_child_rollout_skipped_budget_share: 0,
            turn_plan_seed_us: 0,
            engine_step_us: 0,
            frontier_pop_us: 0,
            expansion_us: 0,
            child_bookkeeping_us: 0,
            rollout_profile: SearchRolloutPerformanceReview {
                cache_queries: 0,
                cache_hits: 0,
                cache_misses: 0,
                cache_lookup_us: 0,
                policy_dispatch_us: 0,
                no_potion_iterations: 0,
                no_potion_phase_profile_us: 0,
                no_potion_legal_actions_us: 0,
                no_potion_choose_action_us: 0,
                no_potion_choose_ordering_us: 0,
                no_potion_probe_us: 0,
                no_potion_probe_score_calls: 0,
                no_potion_probe_actions_evaluated: 0,
                no_potion_probe_step_reuses: 0,
                no_potion_probe_engine_step_us: 0,
                no_potion_probe_phase_profile_us: 0,
                no_potion_probe_action_facts_us: 0,
                no_potion_engine_step_us: 0,
                no_potion_child_build_us: 0,
            },
        }
    }

    fn review(
        candidate_adjudication_census: Option<CombatCaseCandidateAdjudicationCensusV1>,
    ) -> SearchReview {
        SearchReview {
            label: "lane",
            nodes: 1,
            wall_ms: 1,
            rollout_policy: "disabled",
            turn_plan_policy: "disabled",
            phase_guard_policy: "default",
            setup_bias_policy: "default",
            child_rollout_policy: "disabled",
            potion_policy: "never",
            max_potions_used: Some(0),
            complete_win: false,
            hp_loss: None,
            final_hp: None,
            turns: None,
            potions_used: None,
            nodes_expanded: 0,
            nodes_generated: 0,
            nodes_to_first_win: None,
            terminal_wins: 0,
            elapsed_ms: 0,
            deadline_hit: false,
            node_budget_hit: false,
            performance: zero_performance(),
            facts: SearchReviewFacts {
                diagnostic_progress: None,
            },
            candidate_adjudication_census,
        }
    }

    #[test]
    fn candidate_adjudication_census_serialization() {
        let absent = serde_json::to_value(review(None)).expect("serialize absent census");
        assert!(absent.get("candidate_adjudication_census").is_none());

        let present = serde_json::to_value(review(Some(
            CombatCaseCandidateAdjudicationCensusV1::NoRetainedCandidates {
                source_review: "lane".to_string(),
                retained_candidate_count: 0,
            },
        )))
        .expect("serialize present census");
        assert_eq!(
            present["candidate_adjudication_census"]["status"],
            Value::String("no_retained_candidates".to_string())
        );
    }

    #[test]
    fn census_attaches_only_to_matching_review_label() {
        let mut matching = review(None);
        let mut other = review(None);
        other.label = "other";
        let census = CombatCaseCandidateAdjudicationCensusV1::NoRetainedCandidates {
            source_review: "lane".to_string(),
            retained_candidate_count: 0,
        };

        assert!(matching.attach_candidate_adjudication_census(census.clone()));
        assert!(!other.attach_candidate_adjudication_census(census));
        assert!(matching.candidate_adjudication_census.is_some());
        assert!(other.candidate_adjudication_census.is_none());
    }
}
