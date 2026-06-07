use std::collections::BTreeMap;

use crate::content::relics::RelicId;
use crate::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentFrontierGroupV1, BranchExperimentFrontierV1,
    BranchExperimentLineageV1,
};
use crate::eval::branch_experiment_boundary::{active_or_visible_reward_cards, card_offer_labels};
use crate::eval::run_control::RunControlSession;
use crate::state::core::EngineState;
use crate::state::rewards::RewardScreenContext;

pub(super) fn branch_frontier(session: &RunControlSession) -> BranchExperimentFrontierV1 {
    let next_card_reward_offer = active_or_visible_reward_cards(session).map(card_offer_labels);
    let boundary_title = super::current_boundary_title(session);
    let lineage = branch_lineage(session, &boundary_title, next_card_reward_offer.as_ref());
    let key = format!(
        "act{}:floor{}:{}:{}",
        session.run_state.act_num,
        session.run_state.floor_num,
        boundary_title,
        lineage.same_reward_offer_lineage_key
    );
    BranchExperimentFrontierV1 {
        key,
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        boundary_title,
        card_rng_counter: session.run_state.rng_pool.card_rng.counter,
        card_blizz_randomizer: session.run_state.card_blizz_randomizer,
        next_card_reward_offer,
        lineage,
    }
}

fn branch_lineage(
    session: &RunControlSession,
    boundary_title: &str,
    next_card_reward_offer: Option<&Vec<String>>,
) -> BranchExperimentLineageV1 {
    let reward_screen_context = reward_screen_context_label(session)
        .map(str::to_string)
        .unwrap_or_else(|| "none".to_string());
    let reward_count_modifiers = reward_count_modifiers(session);
    let card_pool_modifiers = card_pool_modifiers(session);
    let rarity_modifiers = rarity_modifiers(session);
    let preview_modifiers = preview_modifiers(session);
    let sequence_breakers_present = sequence_breakers_present(
        &reward_count_modifiers,
        &card_pool_modifiers,
        &rarity_modifiers,
        &preview_modifiers,
    );
    let same_reward_offer_lineage_key = format!(
        "card_rng{}:blizz{}:context{}:count{}:pool{}:rarity{}:preview{}:offer{}",
        session.run_state.rng_pool.card_rng.counter,
        session.run_state.card_blizz_randomizer,
        reward_screen_context,
        join_key_parts(&reward_count_modifiers),
        join_key_parts(&card_pool_modifiers),
        join_key_parts(&rarity_modifiers),
        join_key_parts(&preview_modifiers),
        next_card_reward_offer
            .map(|offer| offer.join("|"))
            .unwrap_or_else(|| "-".to_string())
    );

    BranchExperimentLineageV1 {
        visibility: "privileged_simulator_diagnostic".to_string(),
        public_policy_input: false,
        direct_pick_consumes_card_rng: false,
        same_reward_offer_lineage_key,
        reward_screen_context: format!("{reward_screen_context}@{boundary_title}"),
        reward_count_modifiers,
        card_pool_modifiers,
        rarity_modifiers,
        preview_modifiers,
        sequence_breakers_present,
    }
}

fn reward_screen_context_label(session: &RunControlSession) -> Option<&'static str> {
    let context = match &session.engine_state {
        EngineState::RewardScreen(reward) => reward.screen_context,
        EngineState::RewardOverlay { reward_state, .. } => reward_state.screen_context,
        _ => return None,
    };
    Some(match context {
        RewardScreenContext::Standard => "standard",
        RewardScreenContext::TreasureRoom => "treasure_room",
        RewardScreenContext::MuggedCombat => "mugged_combat",
        RewardScreenContext::SmokedCombat => "smoked_combat",
    })
}

fn reward_count_modifiers(session: &RunControlSession) -> Vec<String> {
    relic_flags(
        session,
        &[
            (RelicId::BustedCrown, "busted_crown_reward_count_minus_2"),
            (RelicId::QuestionCard, "question_card_reward_count_plus_1"),
            (
                RelicId::PrayerWheel,
                "prayer_wheel_extra_normal_combat_card_reward",
            ),
        ],
    )
}

fn card_pool_modifiers(session: &RunControlSession) -> Vec<String> {
    relic_flags(
        session,
        &[(RelicId::PrismaticShard, "prismatic_shard_any_color_pool")],
    )
}

fn rarity_modifiers(session: &RunControlSession) -> Vec<String> {
    relic_flags(
        session,
        &[(RelicId::NlothsGift, "nloths_gift_triple_rare_chance")],
    )
}

fn preview_modifiers(session: &RunControlSession) -> Vec<String> {
    let mut modifiers = relic_flags(
        session,
        &[
            (RelicId::MoltenEgg, "molten_egg_upgrade_attack_previews"),
            (RelicId::ToxicEgg, "toxic_egg_upgrade_skill_previews"),
            (RelicId::FrozenEgg, "frozen_egg_upgrade_power_previews"),
        ],
    );
    if session.run_state.card_upgraded_chance > 0.0 {
        modifiers.push(format!(
            "card_upgrade_chance_rng_{:.3}",
            session.run_state.card_upgraded_chance
        ));
    }
    modifiers
}

fn relic_flags(session: &RunControlSession, flags: &[(RelicId, &str)]) -> Vec<String> {
    flags
        .iter()
        .filter_map(|(relic_id, label)| {
            session
                .run_state
                .relics
                .iter()
                .any(|relic| relic.id == *relic_id)
                .then_some((*label).to_string())
        })
        .collect()
}

fn sequence_breakers_present(
    reward_count_modifiers: &[String],
    card_pool_modifiers: &[String],
    rarity_modifiers: &[String],
    preview_modifiers: &[String],
) -> Vec<String> {
    reward_count_modifiers
        .iter()
        .chain(card_pool_modifiers.iter())
        .chain(rarity_modifiers.iter())
        .chain(preview_modifiers.iter())
        .cloned()
        .collect()
}

fn join_key_parts(parts: &[String]) -> String {
    if parts.is_empty() {
        "-".to_string()
    } else {
        parts.join("+")
    }
}

pub(super) fn frontier_groups(
    branches: &[BranchExperimentBranchReportV1],
) -> Vec<BranchExperimentFrontierGroupV1> {
    let mut groups = BTreeMap::<String, BranchExperimentFrontierGroupV1>::new();
    for branch in branches {
        groups
            .entry(branch.frontier.key.clone())
            .and_modify(|group| group.branch_count += 1)
            .or_insert_with(|| BranchExperimentFrontierGroupV1 {
                key: branch.frontier.key.clone(),
                branch_count: 1,
                representative_branch_id: branch.branch_id.clone(),
                boundary_title: branch.frontier.boundary_title.clone(),
                next_card_reward_offer: branch.frontier.next_card_reward_offer.clone(),
                lineage_flags: branch.frontier.lineage.sequence_breakers_present.clone(),
            });
    }
    let mut groups = groups.into_values().collect::<Vec<_>>();
    groups.sort_by(|left, right| {
        right
            .branch_count
            .cmp(&left.branch_count)
            .then_with(|| left.key.cmp(&right.key))
    });
    groups
}
