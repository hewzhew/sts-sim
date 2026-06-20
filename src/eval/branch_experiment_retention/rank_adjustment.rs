use crate::eval::branch_experiment::{
    BranchExperimentChoiceDecisionSignalV1,
    BRANCH_EXPERIMENT_CARD_REWARD_STRATEGIC_TRACE_SIGNAL_SOURCE_V1,
    BRANCH_EXPERIMENT_SHOP_ALTERNATIVE_PLAN_SIGNAL_SOURCE_V1,
    BRANCH_EXPERIMENT_SHOP_BRANCH_PROJECTION_SIGNAL_SOURCE_V1,
    BRANCH_EXPERIMENT_SHOP_SELECTED_PLAN_SIGNAL_SOURCE_V1,
};

use super::context_packet::{
    branch_retention_context_packet_v2, BranchRetentionContextKeyV2, BranchRetentionContextPacketV2,
};
use super::{
    branch_retention_slot_evidence_scores_v1, current_startup_debt_rank_adjustment_v1,
    BranchRetentionCandidateInputV1, BranchRetentionRankAdjustmentV1,
};

const SHOP_COVERAGE_ALTERNATIVE_ACTIVE_BIAS: i32 = -900;

pub fn branch_retention_order_rank_key_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    branch_retention_adjusted_rank_key_v1(candidate)
}

pub fn branch_retention_adjusted_rank_key_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    branch_retention_rank_adjustment_v1(candidate).effective_rank_key
}

pub fn branch_retention_rank_adjustment_v1(
    candidate: &BranchRetentionCandidateInputV1,
) -> BranchRetentionRankAdjustmentV1 {
    let context = branch_retention_context_packet_v2(candidate);
    let current_startup_debt_adjustment = current_startup_debt_rank_adjustment_v1(candidate);
    let startup_adjustment = current_startup_debt_adjustment;
    let admission_pressure = 0;
    let mut reasons = Vec::new();

    if current_startup_debt_adjustment != 0 {
        reasons.push(format!(
            "current_startup_debt_rank_adjustment:{current_startup_debt_adjustment}"
        ));
    }
    if startup_adjustment != 0 {
        reasons.push(format!(
            "startup_rank_adjustment_total:{startup_adjustment}"
        ));
    }

    let strategic_debt_adjustment = branch_strategic_debt_rank_adjustment_v1(candidate);
    if strategic_debt_adjustment != 0 {
        if candidate.curse_count > 0 {
            reasons.push(format!("curse_debt_count:{}", candidate.curse_count));
        }
        for tag in &candidate.strategic_debt_tags {
            reasons.push(format!("strategic_debt_tag:{tag}"));
        }
        reasons.push(format!(
            "strategic_debt_rank_adjustment:{strategic_debt_adjustment}"
        ));
    }
    let formation_need_adjustment = branch_formation_need_rank_adjustment_v1(&context);
    if formation_need_adjustment != 0 {
        for key in &context.keys {
            reasons.push(format!(
                "formation_context_key:{}",
                branch_retention_context_key_label(*key)
            ));
        }
        reasons.push(format!(
            "formation_need_rank_adjustment:{formation_need_adjustment}"
        ));
    }
    let decision_signal_adjustment = branch_decision_signal_rank_adjustment_v1(candidate);
    if decision_signal_adjustment != 0 {
        reasons.push(format!(
            "decision_signal_component_rank_hint:{decision_signal_adjustment}"
        ));
    }
    let card_reward_plan_adjustment = branch_card_reward_plan_rank_adjustment_v1(candidate);
    if card_reward_plan_adjustment != 0 {
        reasons.push(format!(
            "card_reward_plan_rank_adjustment:{card_reward_plan_adjustment}"
        ));
        for signal in candidate.decision_signals.iter().filter(|signal| {
            signal.source == BRANCH_EXPERIMENT_CARD_REWARD_STRATEGIC_TRACE_SIGNAL_SOURCE_V1
        }) {
            for summary in &signal.acquisition_thesis_summary {
                reasons.push(format!("acquisition_thesis:{summary}"));
            }
        }
    }
    let shop_plan_adjustment = branch_shop_plan_rank_adjustment_v1(candidate);
    if shop_plan_adjustment != 0 {
        reasons.push(format!("shop_plan_rank_adjustment:{shop_plan_adjustment}"));
    }
    let campfire_plan_adjustment = branch_campfire_plan_rank_adjustment_v1(candidate);
    if campfire_plan_adjustment != 0 {
        reasons.push(format!(
            "campfire_plan_rank_adjustment:{campfire_plan_adjustment}"
        ));
    }

    let effective_rank_key = candidate
        .rank_key
        .saturating_add(startup_adjustment)
        .saturating_add(strategic_debt_adjustment)
        .saturating_add(formation_need_adjustment)
        .saturating_add(card_reward_plan_adjustment)
        .saturating_add(shop_plan_adjustment)
        .saturating_add(campfire_plan_adjustment);

    BranchRetentionRankAdjustmentV1 {
        base_rank_key: candidate.rank_key,
        startup_adjustment,
        strategic_debt_adjustment,
        formation_need_adjustment,
        shop_plan_adjustment,
        campfire_plan_adjustment,
        card_reward_plan_adjustment,
        decision_signal_adjustment,
        admission_pressure,
        effective_rank_key,
        context_keys: context
            .keys
            .iter()
            .map(|key| branch_retention_context_key_label(*key).to_string())
            .collect(),
        slot_scores: branch_retention_slot_evidence_scores_v1(candidate),
        reasons,
    }
}

fn branch_decision_signal_rank_adjustment_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    candidate
        .decision_signals
        .iter()
        .map(|signal| signal.component_net_rank)
        .sum()
}

fn branch_card_reward_plan_rank_adjustment_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    candidate
        .decision_signals
        .iter()
        .filter(|signal| {
            signal.source == BRANCH_EXPERIMENT_CARD_REWARD_STRATEGIC_TRACE_SIGNAL_SOURCE_V1
        })
        .map(card_reward_signal_rank_adjustment_v1)
        .sum::<i32>()
        .clamp(-2_500, 1_200)
}

fn card_reward_signal_rank_adjustment_v1(signal: &BranchExperimentChoiceDecisionSignalV1) -> i32 {
    let verdict_adjustment = match signal.verdict.as_str() {
        "Reject" => -900 + signal.component_net_rank.min(0) / 2,
        "SkipPreferred" => -500 + signal.component_net_rank.min(0) / 2,
        _ => 0,
    };
    let thesis_adjustment = if verdict_adjustment < 0 {
        signal.acquisition_thesis_rank_adjustment.min(0)
    } else {
        signal.acquisition_thesis_rank_adjustment
    };
    verdict_adjustment
        .saturating_add(thesis_adjustment)
        .clamp(-1_800, 900)
}

fn branch_shop_plan_rank_adjustment_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    candidate
        .decision_signals
        .iter()
        .filter(|signal| {
            matches!(
                signal.source.as_str(),
                BRANCH_EXPERIMENT_SHOP_SELECTED_PLAN_SIGNAL_SOURCE_V1
                    | BRANCH_EXPERIMENT_SHOP_BRANCH_PROJECTION_SIGNAL_SOURCE_V1
                    | BRANCH_EXPERIMENT_SHOP_ALTERNATIVE_PLAN_SIGNAL_SOURCE_V1
            ) && signal.verdict == "Allow"
        })
        .map(shop_plan_signal_rank_adjustment_v1)
        .sum::<i32>()
        .clamp(-25_000, 1_000)
}

fn shop_plan_signal_rank_adjustment_v1(signal: &BranchExperimentChoiceDecisionSignalV1) -> i32 {
    let tier_bonus = signal.tier.saturating_sub(250).max(0).saturating_mul(2);
    let score_bonus = (signal.score.max(0) / 10).min(250);
    let component_bonus = (signal.component_net_rank.max(0) / 4).min(100);
    let evaluation_bonus = tier_bonus
        .saturating_add(score_bonus)
        .saturating_add(component_bonus);
    match signal.source.as_str() {
        BRANCH_EXPERIMENT_SHOP_SELECTED_PLAN_SIGNAL_SOURCE_V1 => {
            evaluation_bonus.saturating_add(600).min(1_000)
        }
        BRANCH_EXPERIMENT_SHOP_BRANCH_PROJECTION_SIGNAL_SOURCE_V1 => evaluation_bonus.min(1_000),
        BRANCH_EXPERIMENT_SHOP_ALTERNATIVE_PLAN_SIGNAL_SOURCE_V1 => {
            // Alternative shop plans are coverage probes. Prefer the compiler-
            // selected plan at the same frontier, but keep structurally valuable
            // alternatives eligible for active exploration when the rest of the
            // branch rank supports them.
            SHOP_COVERAGE_ALTERNATIVE_ACTIVE_BIAS.saturating_add(evaluation_bonus)
        }
        _ => 0,
    }
}

fn branch_campfire_plan_rank_adjustment_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    candidate
        .decision_signals
        .iter()
        .filter(|signal| signal.source == "campfire_plan_v1")
        .map(campfire_plan_signal_rank_adjustment_v1)
        .sum::<i32>()
        .clamp(0, 1_000)
}

fn campfire_plan_signal_rank_adjustment_v1(signal: &BranchExperimentChoiceDecisionSignalV1) -> i32 {
    let role_bonus = match signal.verdict.as_str() {
        "PolicyPreferred" => 300,
        "InspectOnly" => 0,
        "StopFallback" => 0,
        _ => 0,
    };
    role_bonus + signal.score.max(0).min(1_000)
}

fn branch_formation_need_rank_adjustment_v1(context: &BranchRetentionContextPacketV2) -> i32 {
    let mut adjustment = 0i32;
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::MatchesFormationFrontloadNeed)
    {
        adjustment = adjustment.saturating_add(250);
    }
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::MatchesFormationBlockNeed)
    {
        adjustment = adjustment.saturating_add(350);
    }
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::MatchesFormationDrawEnergyNeed)
    {
        adjustment = adjustment.saturating_add(350);
    }
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::MatchesFormationScalingNeed)
    {
        adjustment = adjustment.saturating_add(200);
    }
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::ImmediateSafetyPatch)
    {
        adjustment = adjustment.saturating_add(400);
    }
    if context
        .keys
        .contains(&BranchRetentionContextKeyV2::ClosesPackage)
        || context
            .keys
            .contains(&BranchRetentionContextKeyV2::SupportsCommittedPackage)
    {
        adjustment = adjustment.saturating_add(250);
    }
    // Consistency is deliberately not a positive rank input yet: the current
    // context key also matches ordinary skips, so consuming it here would
    // reintroduce a hidden skip preference through a different path.
    adjustment.min(1_200)
}

fn branch_strategic_debt_rank_adjustment_v1(candidate: &BranchRetentionCandidateInputV1) -> i32 {
    const BOTTLE_HIGH_OPENING_HAND_DEBT: i32 = -1_200;
    const BOTTLE_SITUATIONAL_OPENING_HAND_DEBT: i32 = -800;
    const BOTTLE_POWER_VS_AWAKENED_ONE_DEBT: i32 = -1_000;
    const BOTTLE_TEMPORARY_STRENGTH_BURST_DEBT: i32 = -600;

    let curse_adjustment = -(candidate.curse_count as i32).saturating_mul(1_200);
    curse_adjustment
        + candidate
            .strategic_debt_tags
            .iter()
            .map(|tag| match tag.as_str() {
                "bottle_debt:high_opening_hand" => BOTTLE_HIGH_OPENING_HAND_DEBT,
                "bottle_debt:situational_opening_hand" => BOTTLE_SITUATIONAL_OPENING_HAND_DEBT,
                "bottle_debt:power_vs_awakened_one" => BOTTLE_POWER_VS_AWAKENED_ONE_DEBT,
                "bottle_debt:temporary_strength_burst" => BOTTLE_TEMPORARY_STRENGTH_BURST_DEBT,
                _ => crate::ai::strategic::run_debt_tag_rank_adjustment_v1(tag),
            })
            .sum::<i32>()
}

fn branch_retention_context_key_label(key: BranchRetentionContextKeyV2) -> &'static str {
    match key {
        BranchRetentionContextKeyV2::MatchesFormationFrontloadNeed => {
            "matches_formation_frontload_need"
        }
        BranchRetentionContextKeyV2::MatchesFormationBlockNeed => "matches_formation_block_need",
        BranchRetentionContextKeyV2::MatchesFormationScalingNeed => {
            "matches_formation_scaling_need"
        }
        BranchRetentionContextKeyV2::MatchesFormationDrawEnergyNeed => {
            "matches_formation_draw_energy_need"
        }
        BranchRetentionContextKeyV2::MatchesFormationConsistencyNeed => {
            "matches_formation_consistency_need"
        }
        BranchRetentionContextKeyV2::OpensPackageSetup => "opens_package_setup",
        BranchRetentionContextKeyV2::ClosesPackage => "closes_package",
        BranchRetentionContextKeyV2::SupportsCommittedPackage => "supports_committed_package",
        BranchRetentionContextKeyV2::ImmediateSafetyPatch => "immediate_safety_patch",
    }
}
