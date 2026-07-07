use serde::Serialize;
use sts_simulator::eval::combat_case::CombatCase;

use super::focus::{review_focus, CombatReviewFocus};
use super::key_card_lifecycle::{key_card_lifecycle, KeyCardLifecycleReport};
use super::options::ReviewOptions;
use super::search_runner::{review_key_setup_profile, run_profile_search};
use super::search_types::SearchReview;

#[derive(Serialize)]
pub(super) struct BossSetupLaneReview {
    schema: &'static str,
    contract: &'static str,
    lane: &'static str,
    skipped_reason: Option<&'static str>,
    search: Option<SearchReview>,
    focus: Option<CombatReviewFocus>,
    key_card_lifecycle: Option<KeyCardLifecycleReport>,
}

pub(super) fn run_boss_setup_lane(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<BossSetupLaneReview> {
    if !options.boss_setup_lane {
        return None;
    }
    if !case.position.combat.meta.is_boss_fight {
        return Some(skipped("not_boss_fight"));
    }

    let profile = review_key_setup_profile(
        "boss_setup_key_card_online",
        options.slow_nodes,
        options.slow_ms,
        options,
    );
    let (search, _) = run_profile_search(case, profile, options.action_preview_limit);
    let focus = review_focus(std::slice::from_ref(&search));
    let key_card_lifecycle = key_card_lifecycle(&case.position, focus.as_ref());

    Some(BossSetupLaneReview {
        schema: "boss_setup_lane_v0",
        contract: "review_only_key_setup_bias_search_not_runner_execution",
        lane: "boss_setup_key_card_online",
        skipped_reason: None,
        search: Some(search),
        focus,
        key_card_lifecycle,
    })
}

fn skipped(reason: &'static str) -> BossSetupLaneReview {
    BossSetupLaneReview {
        schema: "boss_setup_lane_v0",
        contract: "review_only_key_setup_bias_search_not_runner_execution",
        lane: "boss_setup_key_card_online",
        skipped_reason: Some(reason),
        search: None,
        focus: None,
        key_card_lifecycle: None,
    }
}
