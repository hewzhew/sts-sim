use sts_simulator::ai::strategy::boss_relic_admission::render_boss_relic_admission_compact;
use sts_simulator::ai::strategy::reward_admission::render_reward_admission_compact;
use sts_simulator::eval::run_control::{
    build_decision_surface, render_auto_applied_step_compact_v1, DecisionCandidateKey,
    RunControlAutoAppliedStepV1, RunControlCommand, RunControlSession,
};

use super::owners::{
    render_shop_tiny_annotation_compact, reward_plan_lane_label, ChoiceAnnotation, OwnerChoice,
};
use super::{
    BossRetryReport, BossRetryStatus, BoundarySite, Branch, BranchPathStep, BranchStatus, Owner,
};

pub(super) fn print_branch_timeline(
    generation: usize,
    branch: &Branch,
    choices: &[OwnerChoice],
    expanded: &[bool],
) {
    println!(
        "\n[{generation:02}] b{:04} A{}F{} {} owner={} hp={}/{} deck={} status={}",
        branch.id,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        status_boundary(&branch.status),
        status_owner(&branch.status),
        branch.session.run_state.current_hp,
        branch.session.run_state.max_hp,
        branch.session.run_state.master_deck.len(),
        status_label(&branch.status),
    );
    if let Some(previous) = branch.path.last() {
        println!("  arrived: {}", render_timeline_step(previous));
    }
    print_auto_steps(&branch.auto_steps);
    if let Some(retry) = branch.boss_retry.as_ref() {
        print_boss_retry(retry);
    }
    print_reward_gap_detail(&branch.session, &branch.status);
    if choices.is_empty() {
        return;
    }
    println!("  choices:");
    for (rank, choice) in choices.iter().enumerate() {
        let marker = if expanded.get(rank).copied().unwrap_or(false) {
            ">"
        } else {
            " "
        };
        println!(
            "  {marker} {:>2}. {}",
            rank + 1,
            render_timeline_choice(choice)
        );
    }
    let expanded_count = expanded.iter().filter(|expanded| **expanded).count();
    if expanded_count == 0 && choices.iter().all(|choice| !choice.auto_expand_allowed()) {
        let reason = choices
            .iter()
            .find_map(|choice| choice.inspect_only_reason())
            .unwrap_or("inspect-only owner");
        println!("  expansion: inspect-only ({reason})");
    } else if expanded_count < choices.len() {
        println!(
            "  expansion: expanded {} hidden {}",
            expanded_count,
            choices.len() - expanded_count
        );
    }
}

pub(super) fn status_boundary(status: &BranchStatus) -> &str {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => boundary,
        BranchStatus::Terminal(_)
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => "-",
    }
}

pub(super) fn render_timeline_choice(choice: &OwnerChoice) -> String {
    let base = match &choice.key {
        Some(key) => render_choice_key_timeline(key),
        None => format!("{}:{}", command_hint(&choice.action), choice.label),
    };
    match &choice.annotation {
        ChoiceAnnotation::Reward { admission, lane } => {
            format!(
                "{:<34} {:<8} {}",
                base,
                reward_plan_lane_label(*lane),
                render_reward_admission_compact(admission)
            )
        }
        ChoiceAnnotation::BossRelic(admission) => {
            format!(
                "{:<34} {}",
                base,
                render_boss_relic_admission_compact(admission)
            )
        }
        ChoiceAnnotation::ShopTiny(annotation) => {
            format!(
                "{:<34} {}",
                base,
                render_shop_tiny_annotation_compact(annotation)
            )
        }
        ChoiceAnnotation::None => base,
    }
}

pub(super) fn one_line(text: &str) -> String {
    text.lines()
        .next()
        .unwrap_or("")
        .trim()
        .chars()
        .take(160)
        .collect()
}

fn print_auto_steps(steps: &[RunControlAutoAppliedStepV1]) {
    if steps.is_empty() {
        return;
    }
    let shown = steps.iter().take(12).collect::<Vec<_>>();
    println!("  auto:");
    for step in shown {
        println!("    - {}", render_auto_applied_step_compact_v1(step));
    }
    if steps.len() > 12 {
        println!("    ... {} more auto steps", steps.len() - 12);
    }
}

fn print_boss_retry(retry: &BossRetryReport) {
    println!(
        "  boss_retry: {} budget={}nodes/{}ms",
        boss_retry_status_label(&retry.status),
        retry.max_nodes,
        retry.wall_ms
    );
    for attempt in &retry.attempts {
        println!(
            "    attempt {}: {} potion={} max_potions={:?} budget={}nodes/{}ms",
            attempt.label,
            boss_retry_status_label(&attempt.status),
            attempt.potion_policy,
            attempt.max_potions_used,
            attempt.max_nodes,
            attempt.wall_ms
        );
        print_action_path("      applied_path", &attempt.action_keys);
    }
}

fn print_action_path(prefix: &str, action_keys: &[String]) {
    if action_keys.is_empty() {
        return;
    }
    let shown = action_keys.iter().take(12).cloned().collect::<Vec<_>>();
    println!("{prefix}: {}", shown.join(" -> "));
    if action_keys.len() > shown.len() {
        println!("      ... {} more actions", action_keys.len() - shown.len());
    }
}

fn boss_retry_status_label(status: &BossRetryStatus) -> String {
    match status {
        BossRetryStatus::Failed(reason) => format!("failed ({})", one_line(reason)),
        BossRetryStatus::Advanced(boundary) => format!("combat-win -> {boundary}"),
        BossRetryStatus::Terminal(result) => format!("terminal:{result}"),
    }
}

fn print_reward_gap_detail(session: &RunControlSession, status: &BranchStatus) {
    if !matches!(
        status,
        BranchStatus::AutomationGap {
            site: BoundarySite::Reward,
            ..
        }
    ) {
        return;
    }
    let surface = build_decision_surface(session);
    let candidates = super::owners::executable_choices(&surface)
        .into_iter()
        .map(|choice| render_timeline_choice(&choice))
        .collect::<Vec<_>>();
    if !candidates.is_empty() {
        println!("    reward_gap_candidates: {}", candidates.join(" | "));
    }
}

fn status_label(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { .. } => "running".to_string(),
        BranchStatus::Terminal(result) => format!("terminal:{result}"),
        BranchStatus::AutomationGap { .. } => "automation_gap".to_string(),
        BranchStatus::CombatGap { reason, .. } => format!("combat_gap:{}", one_line(reason)),
        BranchStatus::BudgetGap { reason, .. } => format!("budget_gap:{}", one_line(reason)),
        BranchStatus::ApplyFailed(err) => format!("apply_failed:{}", one_line(err)),
        BranchStatus::AdvanceFailed(err) => format!("advance_failed:{}", one_line(err)),
    }
}

fn status_owner(status: &BranchStatus) -> String {
    match status {
        BranchStatus::Running { owner, .. } => owner_label(*owner),
        BranchStatus::AutomationGap { site, .. } => site_label(*site),
        BranchStatus::CombatGap { .. } => "combat_search".to_string(),
        BranchStatus::BudgetGap { .. } => "automation_budget".to_string(),
        BranchStatus::Terminal(_) => "terminal".to_string(),
        BranchStatus::ApplyFailed(_) => "candidate_apply".to_string(),
        BranchStatus::AdvanceFailed(_) => "automation".to_string(),
    }
}

fn owner_label(owner: Owner) -> String {
    match owner {
        Owner::NeowStart => "NeowStart".to_string(),
        Owner::CardReward => "CardReward".to_string(),
        Owner::BossRelic => "BossRelic".to_string(),
        Owner::Event(event_id) => format!("Event({event_id:?})"),
        Owner::RewardTiny => "RewardTiny".to_string(),
        Owner::ShopTiny => "ShopTiny".to_string(),
        Owner::Campfire => "Campfire".to_string(),
    }
}

fn site_label(site: BoundarySite) -> String {
    match site {
        BoundarySite::Event(event_id) => format!("Event({event_id:?})"),
        BoundarySite::Reward => "Reward".to_string(),
        BoundarySite::Shop => "Shop".to_string(),
        BoundarySite::Route => "Route".to_string(),
        BoundarySite::Campfire => "Campfire".to_string(),
        BoundarySite::BossRelic => "BossRelic".to_string(),
        BoundarySite::RunChoice => "RunChoice".to_string(),
        BoundarySite::Treasure => "Treasure".to_string(),
        BoundarySite::Terminal => "Terminal".to_string(),
        BoundarySite::Unknown => "Unknown".to_string(),
    }
}

fn render_timeline_step(step: &BranchPathStep) -> String {
    let base = match &step.key {
        Some(key) => render_choice_key_timeline(key),
        None => format!("{}:{}", step.action_debug, step.label),
    };
    match &step.annotation {
        ChoiceAnnotation::Reward { admission, lane } => {
            format!(
                "{base}  {} {}",
                reward_plan_lane_label(*lane),
                render_reward_admission_compact(admission)
            )
        }
        ChoiceAnnotation::BossRelic(admission) => {
            format!("{base}  {}", render_boss_relic_admission_compact(admission))
        }
        ChoiceAnnotation::ShopTiny(annotation) => {
            format!(
                "{base}  {}",
                render_shop_tiny_annotation_compact(annotation)
            )
        }
        ChoiceAnnotation::None => base,
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

fn command_hint(command: &RunControlCommand) -> String {
    match command {
        RunControlCommand::Input(input) => format!("{input:?}"),
        RunControlCommand::BranchSkipCardReward(index) => {
            format!("BranchSkipCardReward({index})")
        }
        RunControlCommand::SingingBowlVisibleCardReward(index) => {
            format!("SingingBowlVisibleCardReward({index})")
        }
        _ => format!("{command:?}"),
    }
}
