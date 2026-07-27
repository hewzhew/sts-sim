use super::{
    backed_widen_due, backed_widen_quantum, boundary_service_views_from_guides,
    generator_needs_initial_grounding, guide_choice_order, guide_uses_progressive_service,
    guide_widen_service_due, local_path_service_cost, lookahead_acquisition_views_from_guides,
    progressive_candidate_index, progressive_guide_width, progressive_rollout_width,
    round_robin_available_index, select_path_service_view, update_max_guide, update_max_rank,
    GraphEdge, LocalServiceView,
};
use crate::policy::{
    CombatGuideLaneId, CombatStateGuide, CombatStateGuideRank, COMBAT_PLAN_STATE_GUIDE_LANE_V1,
};

fn edge(negative_log_policy: f64, visits: usize) -> GraphEdge {
    GraphEdge {
        successor: 0,
        actions: Vec::new(),
        negative_log_policy,
        plan_transition_annotation: None,
        visits,
        anchor_visits: visits,
        guide_visits: Default::default(),
        backed_guides: Default::default(),
        backed_lookahead_rank: None,
        backed_visits: 0,
    }
}

#[test]
fn virtual_widen_and_materialized_child_share_one_local_service_currency() {
    let widen_before = local_path_service_cost(2, 0.5, 0);
    let child_before = local_path_service_cost(3, 0.7, 0);
    assert!(widen_before < child_before);

    let widen_after_service = local_path_service_cost(2, 0.5, 2);
    assert!(child_before < widen_after_service);
}

#[test]
fn local_policy_service_cannot_permanently_starve_lower_prior_child() {
    let preferred = edge(0.0, 0);
    let alternate = edge(1.0, 0);
    let preferred_cost =
        preferred.negative_log_policy + (preferred.anchor_visits.saturating_add(1) as f64).ln();
    let alternate_cost =
        alternate.negative_log_policy + (alternate.anchor_visits.saturating_add(1) as f64).ln();
    assert!(preferred_cost < alternate_cost);

    let preferred_after_service = edge(0.0, 3);
    let preferred_after_cost = preferred_after_service.negative_log_policy
        + (preferred_after_service.anchor_visits.saturating_add(1) as f64).ln();
    assert!(alternate_cost < preferred_after_cost);
}

#[test]
fn guide_exploits_its_best_child_while_anchor_owns_fairness() {
    let best = CombatStateGuideRank::new(vec![1, 0]);
    let alternate = CombatStateGuideRank::new(vec![0, 10_000]);

    assert!(
        guide_choice_order(&best, 100.0, usize::MAX, 9, &alternate, 0.0, 0, 1).is_lt(),
        "guide service debt must not overturn the guide's semantic ordering"
    );
}

#[test]
fn guide_can_continue_a_stronger_unfinished_turn_before_deepening_a_child() {
    let retained_partial = CombatStateGuideRank::new(vec![2, 0]);
    let completed_child = CombatStateGuideRank::new(vec![1, 10_000]);

    assert!(
        guide_choice_order(
            &retained_partial,
            10.0,
            usize::MAX,
            usize::MAX,
            &completed_child,
            0.0,
            0,
            1,
        )
        .is_lt(),
        "a guide must compare its retained partial promise with completed boundary children"
    );
}

#[test]
fn guide_interleaves_widen_and_deepen_service_when_both_are_live() {
    assert!(guide_widen_service_due(0, 0));
    assert!(!guide_widen_service_due(1, 0));
    assert!(guide_widen_service_due(1, 1));
    assert!(!guide_widen_service_due(2, 1));
}

#[test]
fn expensive_guide_opens_competitors_logarithmically() {
    assert_eq!(progressive_guide_width(0), 1);
    assert_eq!(progressive_guide_width(1), 2);
    assert_eq!(progressive_guide_width(3), 3);
    assert_eq!(progressive_guide_width(7), 4);
    assert_eq!(progressive_guide_width(15), 5);
}

#[test]
fn expensive_guide_services_widen_as_a_ranked_peer() {
    // Widen is first in guide order. The first services go to it; once the
    // square-root progressive window opens, the unserved materialized child
    // gets service instead of either side permanently monopolizing the lane.
    assert_eq!(progressive_candidate_index(0, [0, 0]), Some(0));
    assert_eq!(progressive_candidate_index(1, [1, 0]), Some(0));
    assert_eq!(progressive_candidate_index(3, [1, 0]), Some(1));
}

#[test]
fn only_the_configured_expensive_guide_is_progressive() {
    let lookahead = CombatGuideLaneId::new(91);
    let ordinary = CombatGuideLaneId::new(92);

    assert!(!guide_uses_progressive_service(
        COMBAT_PLAN_STATE_GUIDE_LANE_V1,
        Some(lookahead)
    ));
    assert!(guide_uses_progressive_service(lookahead, Some(lookahead)));
    assert!(!guide_uses_progressive_service(ordinary, Some(lookahead)));
}

#[test]
fn one_tree_service_preserves_its_semantic_view_across_depth() {
    let available = [
        LocalServiceView::Anchor,
        LocalServiceView::Guide(crate::policy::CombatGuideLaneId::new(6)),
    ];
    let mut next = 0;
    let root = select_path_service_view(None, &available, &mut next);
    assert_eq!(root, LocalServiceView::Anchor);
    assert_eq!(next, 1);

    let inherited = LocalServiceView::Guide(crate::policy::CombatGuideLaneId::new(6));
    assert_eq!(
        select_path_service_view(Some(inherited), &available, &mut next),
        inherited
    );
    assert_eq!(
        next, 1,
        "a descendant must not consume a fresh local lane rotation"
    );
}

#[test]
fn backed_value_is_monotone_and_keeps_the_best_descendant() {
    let weak = CombatStateGuideRank::new(vec![1, 2]);
    let strong = CombatStateGuideRank::new(vec![1, 3]);
    let weaker_later = CombatStateGuideRank::new(vec![1, 1]);
    let mut backed = None;

    assert!(update_max_rank(&mut backed, &weak));
    assert!(update_max_rank(&mut backed, &strong));
    assert!(!update_max_rank(&mut backed, &weaker_later));
    assert_eq!(backed, Some(strong));
}

#[test]
fn semantic_guide_backup_is_monotone_per_lane() {
    let lane = crate::policy::CombatGuideLaneId::new(4);
    let weak = CombatStateGuideRank::new(vec![1, 2]);
    let strong = CombatStateGuideRank::new(vec![1, 3]);
    let weaker_later = CombatStateGuideRank::new(vec![1, 1]);
    let mut backed = std::collections::BTreeMap::new();

    assert!(update_max_guide(&mut backed, lane, &weak));
    assert!(update_max_guide(&mut backed, lane, &strong));
    assert!(!update_max_guide(&mut backed, lane, &weaker_later));
    assert_eq!(backed.get(&lane), Some(&strong));
}

#[test]
fn backed_search_balances_widen_and_deepen_service() {
    assert!(backed_widen_due(0, 0, true));
    assert!(!backed_widen_due(1, 0, true));
    assert!(backed_widen_due(1, 1, true));
    assert!(!backed_widen_due(2, 1, true));
    assert!(!backed_widen_due(2, usize::MAX, false));
}

#[test]
fn backed_burst_deepens_selected_subtrees_without_widening_the_root() {
    assert_eq!(backed_widen_quantum(0, 4, 256), 4);
    assert_eq!(backed_widen_quantum(1, 4, 256), 256);
    assert_eq!(backed_widen_quantum(17, 4, 256), 256);
}

#[test]
fn live_generator_receives_initial_grounding_even_if_an_external_edge_exists() {
    assert!(generator_needs_initial_grounding(0, false));
    assert!(!generator_needs_initial_grounding(1, false));
    assert!(!generator_needs_initial_grounding(0, true));
}

#[test]
fn each_acquisition_view_gets_a_progressively_widened_rollout_window() {
    assert_eq!(progressive_rollout_width(0), 1);
    assert_eq!(progressive_rollout_width(3), 2);
    assert_eq!(progressive_rollout_width(8), 3);
    assert_eq!(progressive_rollout_width(783), 28);
}

#[test]
fn lookahead_acquisition_rotates_across_available_semantic_views() {
    let available = [true, true, false];
    assert_eq!(round_robin_available_index(0, &available), Some(0));
    assert_eq!(round_robin_available_index(1, &available), Some(1));
    assert_eq!(round_robin_available_index(2, &available), Some(0));
    assert_eq!(round_robin_available_index(7, &available), Some(1));
    assert_eq!(round_robin_available_index(0, &[false, false, false]), None);
}

#[test]
fn cheap_guides_acquire_lookahead_and_keep_their_proven_traversal() {
    let cheap_lane = CombatGuideLaneId::new(4);
    let lookahead_lane = CombatGuideLaneId::new(6);
    let guides = vec![
        CombatStateGuide::new(cheap_lane, vec![1]),
        CombatStateGuide::new(lookahead_lane, vec![1]),
    ];

    let traversal = boundary_service_views_from_guides(&guides, Some(lookahead_lane));
    assert_eq!(
        traversal,
        vec![
            LocalServiceView::Anchor,
            LocalServiceView::LookaheadEvaluation,
            LocalServiceView::Guide(cheap_lane),
        ]
    );

    assert_eq!(
        lookahead_acquisition_views_from_guides(&guides, Some(lookahead_lane)),
        vec![
            LocalServiceView::Anchor,
            LocalServiceView::Guide(cheap_lane)
        ]
    );
}
