use serde::{Deserialize, Serialize};

use crate::eval::run_control::{
    build_decision_surface, parse_run_control_command, RunControlAutoStepOptions,
    RunControlCommand, RunControlConfig, RunControlHpLossLimit, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSession,
};
use crate::state::core::{EngineState, RunResult};
use crate::state::rewards::{RewardCard, RewardItem};

pub const BRANCH_EXPERIMENT_SCHEMA_NAME: &str = "BranchExperimentV1";
pub const BRANCH_EXPERIMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct BranchExperimentConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub player_class: &'static str,
    pub final_act: bool,
    pub max_branches: usize,
    pub max_depth: usize,
    pub auto_max_operations: usize,
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
            max_depth: 4,
            auto_max_operations: 128,
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
    pub branches: Vec<BranchExperimentBranchReportV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchExperimentBranchReportV1 {
    pub branch_id: String,
    pub status: BranchExperimentBranchStatusV1,
    pub score: i32,
    pub choices: Vec<BranchExperimentChoiceV1>,
    pub stop_reason: String,
    pub summary: BranchExperimentRunSummaryV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug)]
struct BranchWork {
    id: String,
    session: RunControlSession,
    choices: Vec<BranchExperimentChoiceV1>,
    status: BranchExperimentBranchStatusV1,
    stop_reason: String,
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
    let mut branches = vec![BranchWork {
        id: "root".to_string(),
        session,
        choices: Vec::new(),
        status: BranchExperimentBranchStatusV1::Active,
        stop_reason: "initial".to_string(),
    }];
    let mut explored_branch_points = 0usize;
    let mut branch_limit_hit = false;

    for depth in 0..config.max_depth {
        let mut next = Vec::new();
        let mut expanded_any = false;

        for mut branch in branches {
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
                    label: option.label,
                    command: option.command.clone(),
                });
                match apply_card_reward_branch_choice(&mut child.session, &option.command) {
                    Ok(()) => {
                        child.stop_reason = "card reward branch applied".to_string();
                    }
                    Err(err) => {
                        child.status = BranchExperimentBranchStatusV1::Failed;
                        child.stop_reason = err;
                    }
                }
                next.push(child);
            }
        }

        next.sort_by(|left, right| branch_score(right).cmp(&branch_score(left)));
        if next.len() > config.max_branches {
            branch_limit_hit = true;
            for pruned in next.iter_mut().skip(config.max_branches) {
                pruned.status = BranchExperimentBranchStatusV1::Pruned;
                pruned.stop_reason = "pruned by max_branches".to_string();
            }
            next.truncate(config.max_branches);
        }

        branches = next;
        if !expanded_any {
            break;
        }
    }

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
        branches: branches
            .into_iter()
            .map(|branch| BranchExperimentBranchReportV1 {
                score: branch_score(&branch),
                branch_id: branch.id,
                status: branch.status,
                choices: branch.choices,
                stop_reason: branch.stop_reason,
                summary: run_summary(&branch.session),
            })
            .collect(),
    }
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

#[derive(Clone, Debug)]
struct CardRewardBranchOption {
    label: String,
    command: String,
}

fn card_reward_branch_options(session: &RunControlSession) -> Option<Vec<CardRewardBranchOption>> {
    let cards = active_or_visible_reward_cards(session)?;
    let options = cards
        .iter()
        .enumerate()
        .map(|(idx, card)| CardRewardBranchOption {
            label: format!("{:?}+{}", card.id, card.upgrades),
            command: format!("rp {idx}"),
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

fn branch_score(branch: &BranchWork) -> i32 {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
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
            branch.choices[0].command == "rp 0" && branch.choices[0].label == "TwinStrike+0"
        }));
        assert!(report.branches.iter().any(|branch| {
            branch.choices[0].command == "rp 1" && branch.choices[0].label == "Cleave+0"
        }));
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
}
