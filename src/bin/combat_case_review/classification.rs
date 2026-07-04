use serde::Serialize;
use sts_simulator::ai::combat_search_v2::SearchTerminalLabel;

use super::focus::CombatReviewFocus;
use super::search_types::{SearchDiagnosticProgressFacts, SearchReview};

#[derive(Serialize)]
pub(super) struct CombatGapReviewClassification {
    pub(super) kind: &'static str,
    reason: &'static str,
    basis_review: Option<&'static str>,
}

pub(super) fn classify_gap_review(
    ladder: &[SearchReview],
    focus: Option<&CombatReviewFocus>,
) -> CombatGapReviewClassification {
    if ladder.is_empty() {
        return classification("NotReviewed", "ladder_not_requested", None);
    }
    if let Some(review) = ladder.iter().find(|review| review.complete_win) {
        return if review.potions_used.unwrap_or(0) > 0 {
            classification(
                "PotionRescueWon",
                "win_found_using_potions",
                Some(review.label),
            )
        } else {
            classification(
                "SearchMissWonWithReview",
                "win_found_with_review_budget",
                Some(review.label),
            )
        };
    }
    let review = ladder
        .last()
        .expect("non-empty ladder was checked before classification");
    if search_starved_by_rollout(review) {
        return classification(
            "SearchStarvedByRollout",
            "rollout_pct_high_and_nodes_low",
            Some(review.label),
        );
    }
    if review.deadline_hit && review.nodes_expanded < 1_000 {
        return classification(
            "TimeoutNoConclusion",
            "deadline_hit_with_too_few_exact_nodes",
            Some(review.label),
        );
    }
    if let Some(focus) = focus {
        if is_exact_near_miss_loss(&focus.progress) {
            return classification(
                "NearMissNoWinAfterReview",
                "exact_loss_reached_single_enemy_with_low_remaining_hp",
                Some(focus.selected_review),
            );
        }
    }
    classification(
        "StillNoWinAfterReview",
        "no_win_after_review_budget",
        Some(review.label),
    )
}

fn is_exact_near_miss_loss(progress: &SearchDiagnosticProgressFacts) -> bool {
    progress.source == "best_complete"
        && progress.terminal == SearchTerminalLabel::Loss
        && !progress.estimated
        && progress.living_enemy_count == 1
        && progress.total_enemy_hp <= 10
}

fn classification(
    kind: &'static str,
    reason: &'static str,
    basis_review: Option<&'static str>,
) -> CombatGapReviewClassification {
    CombatGapReviewClassification {
        kind,
        reason,
        basis_review,
    }
}

fn search_starved_by_rollout(review: &SearchReview) -> bool {
    review.nodes_expanded < 500 && rollout_pct(review) >= 75.0
}

fn rollout_pct(review: &SearchReview) -> f64 {
    if review.performance.total_us == 0 {
        return 0.0;
    }
    100.0 * review.performance.rollout_us as f64 / review.performance.total_us as f64
}
