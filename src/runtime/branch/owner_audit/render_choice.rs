use sts_simulator::ai::strategy::boss_relic_admission::render_boss_relic_admission_compact;
use sts_simulator::ai::strategy::decision_pipeline::{candidate_lane_label, DecisionCandidateKind};
use sts_simulator::ai::strategy::reward_admission::render_reward_admission_compact;
use sts_simulator::eval::run_control::{DecisionCandidateKey, RunDecisionAction};

use super::branch_path::BranchPathStep;
use super::owner_model::{
    cleanup_target_label, ChoiceAnnotation, OwnerCandidateDecision, OwnerChoice,
};

pub(super) fn render_timeline_choice(choice: &OwnerChoice) -> String {
    let base = match &choice.key {
        Some(key) => render_choice_key_timeline(key),
        None => format!("{}:{}", action_hint(&choice.action), choice.label),
    };
    match &choice.annotation {
        ChoiceAnnotation::Candidate(decision) => {
            format!(
                "{:<34} {:<8} score={:<4} {}",
                base,
                candidate_lane_label(decision.evaluation.lane),
                decision.evaluation.total_score(),
                render_candidate_decision_compact(decision)
            )
        }
        ChoiceAnnotation::BossRelic(admission) => {
            format!(
                "{:<34} {}",
                base,
                render_boss_relic_admission_compact(admission)
            )
        }
        ChoiceAnnotation::None => base,
    }
}

pub(super) fn render_timeline_step(step: &BranchPathStep) -> String {
    let base = match &step.key {
        Some(key) => render_choice_key_timeline(key),
        None => format!("{}:{}", step.action_debug, step.label),
    };
    match step.annotation.detail() {
        Some(detail) if !detail.is_empty() => format!("{base}  {detail}"),
        _ => base,
    }
}

pub(super) fn render_candidate_decision_compact(decision: &OwnerCandidateDecision) -> String {
    if let Some(admission) = decision.admission.as_ref() {
        return render_reward_admission_compact(admission);
    }
    match decision.evaluation.candidate.kind {
        DecisionCandidateKind::ShopPurge { target } => {
            format!("Purge {}", cleanup_target_label(target))
        }
        DecisionCandidateKind::ShopBuyRelic { relic, price } => {
            format!("BuyRelic {relic:?} {price}g")
        }
        DecisionCandidateKind::ShopBuyPotion { potion, price } => {
            format!("BuyPotion {potion:?} {price}g")
        }
        DecisionCandidateKind::ShopOpenRewards => "OpenRewards".to_string(),
        DecisionCandidateKind::ShopLeave => "Leave".to_string(),
        DecisionCandidateKind::Unsupported => "Unsupported typed-gap".to_string(),
        _ => String::new(),
    }
}

fn render_choice_key_timeline(key: &DecisionCandidateKey) -> String {
    match key {
        DecisionCandidateKey::EventOption {
            option_index,
            action,
            ..
        } => format!("option {option_index} {action:?}"),
        DecisionCandidateKey::CardRewardPick {
            option_index,
            card,
            upgrades,
            ..
        } => format!("slot {option_index} {card:?}+{upgrades}"),
        DecisionCandidateKey::CardRewardOpen { reward_item_index } => {
            format!("open reward {reward_item_index}")
        }
        DecisionCandidateKey::CardRewardSingingBowl { option_index, .. } => {
            format!("bowl slot {option_index}")
        }
        DecisionCandidateKey::CardRewardSkip { .. } => "skip".to_string(),
        DecisionCandidateKey::BossRelicPick {
            option_index,
            relic,
        } => format!("boss relic {option_index} {relic:?}"),
        DecisionCandidateKey::BossRelicSkip => "skip boss relic".to_string(),
        DecisionCandidateKey::ShopPurgeCard {
            deck_index,
            card,
            upgrades,
        } => format!("purge {deck_index} {card:?}+{upgrades}"),
        DecisionCandidateKey::ShopBuyCard {
            shop_slot,
            card,
            upgrades,
            price,
        } => format!("buy card {shop_slot} {card:?}+{upgrades} {price}g"),
        DecisionCandidateKey::ShopBuyRelic {
            shop_slot,
            relic,
            price,
        } => format!("buy relic {shop_slot} {relic:?} {price}g"),
        DecisionCandidateKey::ShopBuyPotion {
            shop_slot,
            potion,
            price,
        } => format!("buy potion {shop_slot} {potion:?} {price}g"),
        DecisionCandidateKey::ShopOpenRewards => "open shop rewards".to_string(),
        DecisionCandidateKey::SelectionSubmit { reason, .. } => format!("select {reason:?}"),
        DecisionCandidateKey::ShopLeave => "leave shop".to_string(),
    }
}

fn action_hint(action: &RunDecisionAction) -> String {
    match action {
        RunDecisionAction::Input(input) => format!("{input:?}"),
        RunDecisionAction::SkipCardReward { reward_item_index } => {
            format!("SkipCardReward({reward_item_index})")
        }
        RunDecisionAction::SingingBowlCardReward { reward_item_index } => {
            format!("SingingBowlCardReward({reward_item_index})")
        }
    }
}
