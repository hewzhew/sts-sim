use std::collections::BTreeMap;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::eval::branch_experiment_retention::{
    default_branch_retention_decision_v1, select_branch_retention_portfolio_v1,
    BranchRetentionCandidateInputV1, BranchRetentionConfigV1, BranchRetentionDecisionV1,
    BranchRetentionSlotV1,
};
use crate::eval::run_control::{
    build_decision_surface, parse_run_control_command, RunControlAutoStepOptions,
    RunControlCommand, RunControlConfig, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession,
};
use crate::state::core::{EngineState, RunResult};
use crate::state::rewards::{RewardCard, RewardItem, RewardScreenContext};

pub const BRANCH_EXPERIMENT_SCHEMA_NAME: &str = "BranchExperimentV1";
pub const BRANCH_EXPERIMENT_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq)]
pub struct BranchExperimentConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub player_class: &'static str,
    pub final_act: bool,
    pub max_branches: usize,
    pub max_branches_per_frontier_group: Option<usize>,
    pub max_depth: usize,
    pub auto_max_operations: usize,
    pub experiment_wall_ms: Option<u64>,
    pub search_max_nodes: Option<usize>,
    pub search_wall_ms: Option<u64>,
    pub search_max_hp_loss: Option<RunControlHpLossLimit>,
    pub include_skip: bool,
    pub prefix_commands: Vec<String>,
}

impl Default for BranchExperimentConfigV1 {
    fn default() -> Self {
        Self {
            seed: 1,
            ascension_level: 0,
            player_class: "Ironclad",
            final_act: false,
            max_branches: 12,
            max_branches_per_frontier_group: None,
            max_depth: 4,
            auto_max_operations: 128,
            experiment_wall_ms: None,
            search_max_nodes: None,
            search_wall_ms: Some(100),
            search_max_hp_loss: None,
            include_skip: false,
            prefix_commands: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub policy_quality_claim: bool,
    pub seed: u64,
    pub max_branches: usize,
    pub max_depth: usize,
    pub explored_branch_points: usize,
    pub branch_limit_hit: bool,
    pub frontier_group_limit_hit: bool,
    pub wall_limit_hit: bool,
    pub elapsed_wall_ms: u64,
    pub pruned_branch_count: usize,
    pub frontier_groups: Vec<BranchExperimentFrontierGroupV1>,
    pub branches: Vec<BranchExperimentBranchReportV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentBranchReportV1 {
    pub branch_id: String,
    pub status: BranchExperimentBranchStatusV1,
    pub rank_key: i32,
    pub retention: BranchRetentionDecisionV1,
    pub choices: Vec<BranchExperimentChoiceV1>,
    pub stop_reason: String,
    pub summary: BranchExperimentRunSummaryV1,
    pub frontier: BranchExperimentFrontierV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchExperimentBranchStatusV1 {
    Active,
    TerminalVictory,
    TerminalDefeat,
    NeedsHumanBoundary,
    Failed,
    Pruned,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentChoiceV1 {
    pub depth: usize,
    pub kind: String,
    pub card: CardId,
    pub upgrades: u8,
    pub label: String,
    pub command: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentRunSummaryV1 {
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_count: usize,
    pub relic_count: usize,
    pub potion_count: usize,
    pub boundary_title: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentFrontierV1 {
    pub key: String,
    pub act: u8,
    pub floor: i32,
    pub boundary_title: String,
    pub card_rng_counter: u32,
    pub card_blizz_randomizer: i32,
    pub next_card_reward_offer: Option<Vec<String>>,
    pub lineage: BranchExperimentLineageV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentLineageV1 {
    pub visibility: String,
    pub public_policy_input: bool,
    pub direct_pick_consumes_card_rng: bool,
    pub same_reward_offer_lineage_key: String,
    pub reward_screen_context: String,
    pub reward_count_modifiers: Vec<String>,
    pub card_pool_modifiers: Vec<String>,
    pub rarity_modifiers: Vec<String>,
    pub preview_modifiers: Vec<String>,
    pub sequence_breakers_present: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentFrontierGroupV1 {
    pub key: String,
    pub branch_count: usize,
    pub representative_branch_id: String,
    pub boundary_title: String,
    pub next_card_reward_offer: Option<Vec<String>>,
    pub lineage_flags: Vec<String>,
}

#[derive(Clone, Debug)]
struct BranchWork {
    id: String,
    session: RunControlSession,
    choices: Vec<BranchExperimentChoiceV1>,
    status: BranchExperimentBranchStatusV1,
    stop_reason: String,
    retention: BranchRetentionDecisionV1,
}

pub fn run_branch_experiment_v1(
    config: &BranchExperimentConfigV1,
) -> Result<BranchExperimentReportV1, String> {
    let mut session = RunControlSession::new(RunControlConfig {
        seed: config.seed,
        ascension_level: config.ascension_level,
        final_act: config.final_act,
        player_class: config.player_class,
        search_max_nodes: config.search_max_nodes,
        search_wall_ms: config.search_wall_ms,
        ..RunControlConfig::default()
    });

    for command_line in &config.prefix_commands {
        let command = parse_run_control_command(command_line)?;
        session.apply_command(command)?;
    }

    Ok(run_branch_experiment_from_session(session, config))
}

fn run_branch_experiment_from_session(
    session: RunControlSession,
    config: &BranchExperimentConfigV1,
) -> BranchExperimentReportV1 {
    let started_at = Instant::now();
    let mut branches = vec![BranchWork {
        id: "root".to_string(),
        session,
        choices: Vec::new(),
        status: BranchExperimentBranchStatusV1::Active,
        stop_reason: "initial".to_string(),
        retention: default_branch_retention_decision_v1(),
    }];
    let mut explored_branch_points = 0usize;
    let mut branch_limit_hit = false;
    let mut frontier_group_limit_hit = false;
    let mut wall_limit_hit = false;
    let mut pruned_branch_count = 0usize;

    for depth in 0..config.max_depth {
        if experiment_wall_limit_hit(started_at, config) {
            wall_limit_hit = true;
            break;
        }
        let mut next = Vec::new();
        let mut expanded_any = false;

        for mut branch in branches {
            if experiment_wall_limit_hit(started_at, config) {
                wall_limit_hit = true;
                next.push(branch);
                continue;
            }
            if branch.status != BranchExperimentBranchStatusV1::Active {
                next.push(branch);
                continue;
            }

            advance_to_experiment_boundary(&mut branch, config);
            if branch.status != BranchExperimentBranchStatusV1::Active {
                next.push(branch);
                continue;
            }

            let Some(options) = card_reward_branch_options(&branch.session) else {
                branch.status = BranchExperimentBranchStatusV1::NeedsHumanBoundary;
                branch.stop_reason = current_boundary_title(&branch.session);
                next.push(branch);
                continue;
            };

            explored_branch_points = explored_branch_points.saturating_add(1);
            expanded_any = true;
            for option in options {
                let mut child = branch.clone();
                child.id = format!("{}.{}", child.id, option.command);
                child.choices.push(BranchExperimentChoiceV1 {
                    depth,
                    kind: "card_reward".to_string(),
                    card: option.card,
                    upgrades: option.upgrades,
                    label: option.label,
                    command: option.command.clone(),
                });
                match apply_card_reward_branch_choice(&mut child.session, &option.command) {
                    Ok(()) => {
                        child.stop_reason = "card reward branch applied".to_string();
                        settle_branch_to_frontier(&mut child, config);
                    }
                    Err(err) => {
                        child.status = BranchExperimentBranchStatusV1::Failed;
                        child.stop_reason = err;
                    }
                }
                next.push(child);
            }
        }

        let retention = apply_branch_retention(next, config);
        next = retention.branches;
        branch_limit_hit |= retention.branch_limit_hit;
        frontier_group_limit_hit |= retention.frontier_group_limit_hit;
        pruned_branch_count = pruned_branch_count.saturating_add(retention.pruned_count);

        branches = next;
        if !expanded_any {
            break;
        }
    }
    for branch in &mut branches {
        if experiment_wall_limit_hit(started_at, config) {
            wall_limit_hit = true;
            break;
        }
        settle_branch_to_frontier(branch, config);
    }

    let mut branch_reports = branches
        .into_iter()
        .map(|branch| {
            let summary = run_summary(&branch.session);
            let frontier = branch_frontier(&branch.session);
            BranchExperimentBranchReportV1 {
                rank_key: branch_rank_key(&branch),
                retention: branch.retention,
                branch_id: branch.id,
                status: branch.status,
                choices: branch.choices,
                stop_reason: branch.stop_reason,
                summary,
                frontier,
            }
        })
        .collect::<Vec<_>>();
    branch_reports.sort_by(|left, right| {
        retention_report_slot_priority(left.retention.primary_slot)
            .cmp(&retention_report_slot_priority(
                right.retention.primary_slot,
            ))
            .then_with(|| right.rank_key.cmp(&left.rank_key))
            .then_with(|| left.branch_id.cmp(&right.branch_id))
    });

    BranchExperimentReportV1 {
        schema_name: BRANCH_EXPERIMENT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_EXPERIMENT_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        policy_quality_claim: false,
        seed: config.seed,
        max_branches: config.max_branches,
        max_depth: config.max_depth,
        explored_branch_points,
        branch_limit_hit,
        frontier_group_limit_hit,
        wall_limit_hit,
        elapsed_wall_ms: started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
        pruned_branch_count,
        frontier_groups: frontier_groups(&branch_reports),
        branches: branch_reports,
    }
}

fn experiment_wall_limit_hit(started_at: Instant, config: &BranchExperimentConfigV1) -> bool {
    let Some(limit_ms) = config.experiment_wall_ms else {
        return false;
    };
    started_at.elapsed().as_millis() >= u128::from(limit_ms)
}

fn advance_to_experiment_boundary(branch: &mut BranchWork, config: &BranchExperimentConfigV1) {
    if is_terminal(&branch.session) || card_reward_branch_options(&branch.session).is_some() {
        update_terminal_status(branch);
        return;
    }

    let outcome =
        branch
            .session
            .apply_command(RunControlCommand::AutoRun(RunControlAutoStepOptions {
                search: RunControlSearchCombatOptions {
                    max_nodes: config.search_max_nodes,
                    wall_ms: config.search_wall_ms,
                    max_hp_loss: config.search_max_hp_loss,
                    ..RunControlSearchCombatOptions::default()
                },
                max_operations: Some(config.auto_max_operations),
                route: RunControlRouteAutomationMode::Planner,
            }));

    match outcome {
        Ok(outcome) => {
            branch.stop_reason = first_reason_line(&outcome.message)
                .unwrap_or_else(|| current_boundary_title(&branch.session));
            update_terminal_status(branch);
        }
        Err(err) => {
            branch.status = BranchExperimentBranchStatusV1::Failed;
            branch.stop_reason = err;
        }
    }
}

fn update_terminal_status(branch: &mut BranchWork) {
    match &branch.session.engine_state {
        EngineState::GameOver(RunResult::Victory) => {
            branch.status = BranchExperimentBranchStatusV1::TerminalVictory;
            branch.stop_reason = "victory".to_string();
        }
        EngineState::GameOver(RunResult::Defeat) => {
            branch.status = BranchExperimentBranchStatusV1::TerminalDefeat;
            branch.stop_reason = "defeat".to_string();
        }
        _ => {}
    }
}

fn settle_branch_to_frontier(branch: &mut BranchWork, config: &BranchExperimentConfigV1) {
    if branch.status != BranchExperimentBranchStatusV1::Active {
        return;
    }
    advance_to_experiment_boundary(branch, config);
    if branch.status != BranchExperimentBranchStatusV1::Active || is_terminal(&branch.session) {
        return;
    }
    if card_reward_branch_options(&branch.session).is_none() {
        branch.status = BranchExperimentBranchStatusV1::NeedsHumanBoundary;
        branch.stop_reason = current_boundary_title(&branch.session);
    }
}

#[derive(Clone, Debug)]
struct BranchRetentionApplyResult {
    branches: Vec<BranchWork>,
    branch_limit_hit: bool,
    frontier_group_limit_hit: bool,
    pruned_count: usize,
}

fn apply_branch_retention(
    mut branches: Vec<BranchWork>,
    config: &BranchExperimentConfigV1,
) -> BranchRetentionApplyResult {
    let before_len = branches.len();
    let candidates = branches
        .iter()
        .enumerate()
        .map(|(index, branch)| BranchRetentionCandidateInputV1 {
            index,
            frontier_key: branch_frontier(&branch.session).key,
            rank_key: branch_rank_key(branch),
            hp: branch.session.run_state.current_hp,
            max_hp: branch.session.run_state.max_hp,
            gold: branch.session.run_state.gold,
            deck_count: branch.session.run_state.master_deck.len(),
            choice_profiles: branch
                .choices
                .iter()
                .map(|choice| {
                    card_reward_semantic_profile_v1(&RewardCard::new(choice.card, choice.upgrades))
                })
                .collect(),
        })
        .collect::<Vec<_>>();
    let selection = select_branch_retention_portfolio_v1(
        &candidates,
        BranchRetentionConfigV1 {
            max_total: config.max_branches,
            max_per_frontier: config.max_branches_per_frontier_group,
        },
    );

    for (index, branch) in branches.iter_mut().enumerate() {
        branch.retention = selection
            .decisions_by_index
            .get(&index)
            .cloned()
            .unwrap_or_else(default_branch_retention_decision_v1);
    }

    let mut branches = branches
        .into_iter()
        .enumerate()
        .filter_map(|(index, branch)| selection.keep_indices.contains(&index).then_some(branch))
        .collect::<Vec<_>>();
    branches.sort_by(|left, right| {
        retention_report_slot_priority(left.retention.primary_slot)
            .cmp(&retention_report_slot_priority(
                right.retention.primary_slot,
            ))
            .then_with(|| branch_rank_key(right).cmp(&branch_rank_key(left)))
            .then_with(|| left.id.cmp(&right.id))
    });

    BranchRetentionApplyResult {
        branches,
        branch_limit_hit: selection.total_limit_hit,
        frontier_group_limit_hit: selection.frontier_limit_hit,
        pruned_count: before_len.saturating_sub(selection.keep_indices.len()),
    }
}

fn retention_report_slot_priority(slot: BranchRetentionSlotV1) -> usize {
    match slot {
        BranchRetentionSlotV1::Package => 0,
        BranchRetentionSlotV1::Scaling => 1,
        BranchRetentionSlotV1::DefenseEngine => 2,
        BranchRetentionSlotV1::Survival => 3,
        BranchRetentionSlotV1::Frontload => 4,
        BranchRetentionSlotV1::CleanDeck => 5,
        BranchRetentionSlotV1::Diversity => 6,
    }
}

#[derive(Clone, Debug)]
struct CardRewardBranchOption {
    label: String,
    command: String,
    card: CardId,
    upgrades: u8,
}

fn card_reward_branch_options(session: &RunControlSession) -> Option<Vec<CardRewardBranchOption>> {
    let cards = active_or_visible_reward_cards(session)?;
    let options = cards
        .iter()
        .enumerate()
        .map(|(idx, card)| CardRewardBranchOption {
            label: format_reward_card_label(card),
            command: format!("rp {idx}"),
            card: card.id,
            upgrades: card.upgrades,
        })
        .collect::<Vec<_>>();
    if options.is_empty() {
        return None;
    }
    Some(options)
}

fn active_or_visible_reward_cards(session: &RunControlSession) -> Option<Vec<RewardCard>> {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => reward
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward(reward)),
        EngineState::RewardOverlay { reward_state, .. } => reward_state
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward(reward_state)),
        _ => None,
    }
}

fn first_visible_card_reward(
    reward: &crate::state::rewards::RewardState,
) -> Option<Vec<RewardCard>> {
    reward.items.iter().find_map(|item| match item {
        RewardItem::Card { cards } => Some(cards.clone()),
        _ => None,
    })
}

fn apply_card_reward_branch_choice(
    session: &mut RunControlSession,
    command: &str,
) -> Result<(), String> {
    let command = parse_run_control_command(command)?;
    session.apply_command(command).map(|_| ())
}

fn current_boundary_title(session: &RunControlSession) -> String {
    build_decision_surface(session).view.header.title
}

fn first_reason_line(message: &str) -> Option<String> {
    message
        .lines()
        .find_map(|line| line.strip_prefix("Reason: ").map(str::to_string))
}

fn is_terminal(session: &RunControlSession) -> bool {
    matches!(session.engine_state, EngineState::GameOver(_))
}

fn branch_rank_key(branch: &BranchWork) -> i32 {
    match branch.status {
        BranchExperimentBranchStatusV1::TerminalVictory => 1_000_000,
        BranchExperimentBranchStatusV1::TerminalDefeat => -1_000_000,
        BranchExperimentBranchStatusV1::Failed => -900_000,
        BranchExperimentBranchStatusV1::Pruned => -800_000,
        BranchExperimentBranchStatusV1::Active
        | BranchExperimentBranchStatusV1::NeedsHumanBoundary => {
            branch.session.run_state.act_num as i32 * 10_000
                + branch.session.run_state.floor_num * 100
                + branch.session.run_state.current_hp * 10
                + branch.session.run_state.gold
        }
    }
}

fn run_summary(session: &RunControlSession) -> BranchExperimentRunSummaryV1 {
    BranchExperimentRunSummaryV1 {
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        hp: session.run_state.current_hp,
        max_hp: session.run_state.max_hp,
        gold: session.run_state.gold,
        deck_count: session.run_state.master_deck.len(),
        relic_count: session.run_state.relics.len(),
        potion_count: session
            .run_state
            .potions
            .iter()
            .filter(|potion| potion.is_some())
            .count(),
        boundary_title: current_boundary_title(session),
    }
}

fn branch_frontier(session: &RunControlSession) -> BranchExperimentFrontierV1 {
    let next_card_reward_offer = active_or_visible_reward_cards(session).map(card_offer_labels);
    let boundary_title = current_boundary_title(session);
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

fn card_offer_labels(cards: Vec<RewardCard>) -> Vec<String> {
    cards
        .into_iter()
        .map(|card| format_reward_card_label(&card))
        .collect()
}

fn format_reward_card_label(card: &RewardCard) -> String {
    let name = crate::content::cards::get_card_definition(card.id).name;
    match card.upgrades {
        0 => name.to_string(),
        1 => format!("{name}+"),
        upgrades => format!("{name}+{upgrades}"),
    }
}

fn frontier_groups(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::relics::RelicState;
    use crate::state::rewards::RewardState;

    #[test]
    fn branch_experiment_expands_pending_card_reward_choices() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert_eq!(report.explored_branch_points, 1);
        assert_eq!(report.branches.len(), 2);
        assert!(report.branches.iter().any(|branch| {
            branch.choices[0].command == "rp 0" && branch.choices[0].label == "Twin Strike"
        }));
        assert!(report.branches.iter().any(|branch| {
            branch.choices[0].command == "rp 1" && branch.choices[0].label == "Cleave"
        }));
    }

    #[test]
    fn recorded_card_reward_pick_does_not_consume_card_reward_rng() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);
        let card_rng_counter_before = session.run_state.rng_pool.card_rng.counter;

        session
            .apply_command(RunControlCommand::RecordedCardRewardPick(0))
            .expect("recorded pick applies");

        assert_eq!(
            session.run_state.rng_pool.card_rng.counter, card_rng_counter_before,
            "card reward choices are generated before the player picks; picking a card must not consume card reward RNG"
        );
    }

    #[test]
    fn branch_lineage_is_privileged_and_not_public_policy_input() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![RewardCard::new(CardId::TwinStrike, 0)]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        let lineage = &report.branches[0].frontier.lineage;
        assert_eq!(lineage.visibility, "privileged_simulator_diagnostic");
        assert!(!lineage.public_policy_input);
        assert!(!lineage.direct_pick_consumes_card_rng);
        assert!(lineage.sequence_breakers_present.is_empty());
    }

    #[test]
    fn branch_lineage_reports_reward_sequence_breakers() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.relics.clear();
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::QuestionCard));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::PrayerWheel));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::PrismaticShard));
        session
            .run_state
            .relics
            .push(RelicState::new(RelicId::NlothsGift));
        session.run_state.card_upgraded_chance = 0.25;
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![RewardCard::new(CardId::TwinStrike, 1)]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        let lineage = &report.branches[0].frontier.lineage;
        assert!(lineage
            .reward_count_modifiers
            .contains(&"question_card_reward_count_plus_1".to_string()));
        assert!(lineage
            .reward_count_modifiers
            .contains(&"prayer_wheel_extra_normal_combat_card_reward".to_string()));
        assert!(lineage
            .card_pool_modifiers
            .contains(&"prismatic_shard_any_color_pool".to_string()));
        assert!(lineage
            .rarity_modifiers
            .contains(&"nloths_gift_triple_rare_chance".to_string()));
        assert!(lineage
            .preview_modifiers
            .contains(&"card_upgrade_chance_rng_0.250".to_string()));
        assert_eq!(
            report.frontier_groups[0].lineage_flags,
            lineage.sequence_breakers_present
        );
    }

    #[test]
    fn branch_experiment_settles_after_last_depth_choice() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                auto_max_operations: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert!(
            report
                .branches
                .iter()
                .all(|branch| branch.stop_reason != "card reward branch applied"),
            "depth-exhausted branch results should be settled to a readable frontier, not left at an internal transition"
        );
    }

    #[test]
    fn branch_experiment_prunes_to_max_branches() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 2,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert!(report.branch_limit_hit);
        assert_eq!(report.branches.len(), 2);
    }

    #[test]
    fn branch_experiment_caps_same_frontier_group_variants() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut reward = RewardState::new();
        reward.pending_card_choice = Some(vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
        ]);
        session.engine_state = EngineState::RewardScreen(reward);

        let report = run_branch_experiment_from_session(
            session,
            &BranchExperimentConfigV1 {
                max_depth: 1,
                max_branches: 4,
                max_branches_per_frontier_group: Some(1),
                auto_max_operations: 0,
                ..BranchExperimentConfigV1::default()
            },
        );

        assert!(report.frontier_group_limit_hit);
        assert_eq!(report.pruned_branch_count, 1);
        assert_eq!(report.branches.len(), 1);
        assert_eq!(report.frontier_groups.len(), 1);
    }
}
