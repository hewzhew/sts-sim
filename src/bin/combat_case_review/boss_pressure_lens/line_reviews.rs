use sts_simulator::ai::combat_search_v2::{CombatLineLabReport, SearchTerminalLabel};

use super::super::search_types::SearchReview;
use super::types::BossLineReview;

pub(super) fn collect_line_reviews(
    ladder: &[SearchReview],
    line_lab: Option<&CombatLineLabReport>,
) -> Vec<BossLineReview> {
    let mut reviews: Vec<_> = ladder.iter().map(line_review_from_search).collect();
    if let Some(turn_pool) = line_lab.and_then(|report| report.turn_pool.as_ref()) {
        reviews.extend(turn_pool.lanes.iter().map(|line| BossLineReview {
            source: format!("turn_pool:{}", line.lane),
            terminal: line.terminal,
            final_hp: Some(line.final_hp),
            hp_loss: None,
            turns: Some(line.turns),
            potions_used: Some(line.potions_used),
            tags: line_quality_tags(
                line.terminal,
                Some(line.final_hp),
                Some(line.turns),
                Some(line.potions_used),
                line.living_enemy_count,
                line.total_enemy_hp,
            ),
        }));
    }
    reviews
}

pub(super) fn aggregate_line_tags(line_reviews: &[BossLineReview]) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if line_reviews.iter().any(|line| {
        line.tags
            .iter()
            .any(|tag| *tag == "no_win_left_multi_target_pressure")
    }) {
        tags.push("collector_lines_leave_multi_target_pressure");
    }
    if line_reviews.iter().any(|line| {
        line.tags
            .iter()
            .any(|tag| *tag == "no_win_boss_hp_still_high")
    }) {
        tags.push("collector_lines_leave_boss_hp_high");
    }
    if line_reviews.iter().any(|line| {
        line.tags
            .iter()
            .any(|tag| *tag == "failed_after_debuff_window")
    }) {
        tags.push("collector_lines_fail_after_debuff_window");
    }
    if line_reviews
        .iter()
        .any(|line| line.tags.iter().any(|tag| *tag == "dirty_win_low_hp"))
    {
        tags.push("collector_dirty_win_only");
    }
    tags
}

fn line_review_from_search(review: &SearchReview) -> BossLineReview {
    let progress = review.facts.diagnostic_progress.as_ref();
    BossLineReview {
        source: review.label.to_string(),
        terminal: progress
            .map(|facts| facts.terminal)
            .unwrap_or(SearchTerminalLabel::Unresolved),
        final_hp: review
            .final_hp
            .or_else(|| progress.map(|facts| facts.final_hp)),
        hp_loss: review
            .hp_loss
            .or_else(|| progress.map(|facts| facts.hp_loss)),
        turns: review.turns.or_else(|| progress.map(|facts| facts.turns)),
        potions_used: review
            .potions_used
            .or_else(|| progress.map(|facts| facts.potions_used)),
        tags: line_quality_tags(
            progress
                .map(|facts| facts.terminal)
                .unwrap_or(SearchTerminalLabel::Unresolved),
            review
                .final_hp
                .or_else(|| progress.map(|facts| facts.final_hp)),
            review.turns.or_else(|| progress.map(|facts| facts.turns)),
            review
                .potions_used
                .or_else(|| progress.map(|facts| facts.potions_used)),
            progress
                .map(|facts| facts.living_enemy_count)
                .unwrap_or_default(),
            progress
                .map(|facts| facts.total_enemy_hp)
                .unwrap_or_default(),
        ),
    }
}

fn line_quality_tags(
    terminal: SearchTerminalLabel,
    final_hp: Option<i32>,
    turns: Option<u32>,
    potions_used: Option<u32>,
    living_enemy_count: usize,
    total_enemy_hp: i32,
) -> Vec<&'static str> {
    let mut tags = Vec::new();
    if terminal == SearchTerminalLabel::Win && final_hp.is_some_and(|hp| hp <= 10) {
        tags.push("dirty_win_low_hp");
    }
    if terminal == SearchTerminalLabel::Win
        && final_hp.is_some_and(|hp| hp <= 20)
        && potions_used.unwrap_or(0) > 0
    {
        tags.push("potion_rescue_desperate");
    }
    if terminal != SearchTerminalLabel::Win && living_enemy_count >= 2 {
        tags.push("no_win_left_multi_target_pressure");
    }
    if terminal != SearchTerminalLabel::Win && total_enemy_hp >= 80 {
        tags.push("no_win_boss_hp_still_high");
    }
    if terminal != SearchTerminalLabel::Win && turns.unwrap_or(0) >= 4 {
        tags.push("failed_after_debuff_window");
    }
    tags
}
