use std::path::PathBuf;

use sts_simulator::eval::branch_campaign::{
    BranchCampaignConfigV1, BranchCampaignProgressDetailV1, BranchCampaignReportDetailV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use sts_simulator::eval::branch_experiment_search_options::parse_branch_experiment_search_options_v1;
use sts_simulator::eval::run_control::RunControlSearchCombatOptions;

use super::{
    campaign_config_from_args, campaign_search_options_from_args, parse_hp_loss_limit, Args,
};

#[derive(Clone, Debug)]
pub(super) struct RunCommandInput {
    pub(super) config: BranchCampaignConfigV1,
    pub(super) progress: bool,
    pub(super) progress_detail: BranchCampaignProgressDetailV1,
    pub(super) json: bool,
    pub(super) resume: Option<PathBuf>,
    pub(super) resume_checkpoint: Option<PathBuf>,
    pub(super) out: Option<PathBuf>,
    pub(super) checkpoint_out: Option<PathBuf>,
    pub(super) export_outcome_dataset: Option<PathBuf>,
    pub(super) export_learning_dataset: Option<PathBuf>,
    pub(super) export_decision_outcome_dataset: Option<PathBuf>,
    pub(super) branch_examples: usize,
    pub(super) report_detail: BranchCampaignReportDetailV1,
}

impl RunCommandInput {
    pub(super) fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            config: campaign_config_from_args(args)?,
            progress: args.progress,
            progress_detail: BranchCampaignProgressDetailV1::from(args.progress_detail),
            json: args.json,
            resume: args.resume.clone(),
            resume_checkpoint: args.resume_checkpoint.clone(),
            out: args.out.clone(),
            checkpoint_out: args.checkpoint_out.clone(),
            export_outcome_dataset: args.export_outcome_dataset.clone(),
            export_learning_dataset: args.export_learning_dataset.clone(),
            export_decision_outcome_dataset: args.export_decision_outcome_dataset.clone(),
            branch_examples: args.branch_examples,
            report_detail: BranchCampaignReportDetailV1::from(args.report_detail),
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct InspectCommandInput {
    pub(super) checkpoint_path: Option<PathBuf>,
    pub(super) report_path: Option<PathBuf>,
    pub(super) summary: bool,
    pub(super) filters: InspectFiltersInput,
    pub(super) modes: InspectModesInput,
    pub(super) search_options: RunControlSearchCombatOptions,
    pub(super) branch_examples: usize,
    pub(super) shop_challenge: ShopChallengeInput,
}

impl InspectCommandInput {
    pub(super) fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            checkpoint_path: args.inspect_checkpoint.clone(),
            report_path: args.inspect_report.clone(),
            summary: args.inspect_summary,
            filters: InspectFiltersInput {
                act: args.inspect_act,
                floor: args.inspect_floor,
                boundary: args.inspect_boundary.clone(),
                hp: args.inspect_hp,
                index: args.inspect_index,
            },
            modes: InspectModesInput {
                search: args.inspect_search,
                last_auto_combat: args.inspect_last_auto_combat,
                combat_lab: args.inspect_combat_lab,
                probe_boss: args.probe_boss,
                shop_evidence: args.inspect_shop_evidence,
                shop_challenge: args.challenge_shop_plans,
                card_reward_evidence: args.inspect_card_reward_evidence,
                campfire_evidence: args.inspect_campfire_evidence,
                deck_mutation: args.inspect_deck_mutation,
                route_evidence: args.inspect_route_evidence,
            },
            search_options: inspect_search_options_from_args(args)?,
            branch_examples: args.branch_examples,
            shop_challenge: ShopChallengeInput::from_args(args)?,
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct InspectFiltersInput {
    pub(super) act: Option<u8>,
    pub(super) floor: Option<i32>,
    pub(super) boundary: Option<String>,
    pub(super) hp: Option<i32>,
    pub(super) index: Option<usize>,
}

#[derive(Clone, Debug)]
pub(super) struct InspectModesInput {
    pub(super) search: bool,
    pub(super) last_auto_combat: bool,
    pub(super) combat_lab: bool,
    pub(super) probe_boss: bool,
    pub(super) shop_evidence: bool,
    pub(super) shop_challenge: bool,
    pub(super) card_reward_evidence: bool,
    pub(super) campfire_evidence: bool,
    pub(super) deck_mutation: bool,
    pub(super) route_evidence: bool,
}

#[derive(Clone, Debug)]
pub(super) struct ShopChallengeInput {
    pub(super) final_act: bool,
    pub(super) max_reward_options_per_branch: Option<usize>,
    pub(super) max_campfire_options_per_branch: usize,
    pub(super) auto_max_operations: usize,
    pub(super) experiment_wall_ms: u64,
    pub(super) challenge_max_plans: usize,
    pub(super) challenge_depth: usize,
    pub(super) challenge_max_branches: usize,
    pub(super) search_max_nodes: Option<usize>,
    pub(super) search_wall_ms: u64,
    pub(super) search_options: RunControlSearchCombatOptions,
    pub(super) retention_budget_profile: BranchRetentionBudgetProfileV1,
}

impl ShopChallengeInput {
    fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            final_act: args.final_act,
            max_reward_options_per_branch: if args.all_reward_options {
                None
            } else {
                Some(args.max_reward_options.unwrap_or(2))
            },
            max_campfire_options_per_branch: args.max_campfire_options,
            auto_max_operations: args.auto_max_ops,
            experiment_wall_ms: args.experiment_wall_ms,
            challenge_max_plans: args.challenge_max_plans,
            challenge_depth: args.challenge_depth,
            challenge_max_branches: args.challenge_max_branches,
            search_max_nodes: args.search_max_nodes,
            search_wall_ms: args.search_wall_ms,
            search_options: campaign_search_options_from_args(args)?,
            retention_budget_profile: args
                .retention_profile
                .parse::<BranchRetentionBudgetProfileV1>()?,
        })
    }
}

#[derive(Clone, Debug)]
pub(super) struct DatasetCommandInput {
    pub(super) inspect_checkpoint: Option<PathBuf>,
    pub(super) inspect_report: Option<PathBuf>,
    pub(super) export_outcome_dataset: Option<PathBuf>,
    pub(super) analyze_outcome_dataset: Option<PathBuf>,
    pub(super) analyze_decision_outcome_dataset: Option<PathBuf>,
    pub(super) probe_learning_readiness: Option<PathBuf>,
    pub(super) export_learning_dataset: Option<PathBuf>,
    pub(super) export_decision_outcome_dataset: Option<PathBuf>,
}

impl DatasetCommandInput {
    pub(super) fn from_args(args: &Args) -> Self {
        Self {
            inspect_checkpoint: args.inspect_checkpoint.clone(),
            inspect_report: args.inspect_report.clone(),
            export_outcome_dataset: args.export_outcome_dataset.clone(),
            analyze_outcome_dataset: args.analyze_outcome_dataset.clone(),
            analyze_decision_outcome_dataset: args.analyze_decision_outcome_dataset.clone(),
            probe_learning_readiness: args.probe_learning_readiness.clone(),
            export_learning_dataset: args.export_learning_dataset.clone(),
            export_decision_outcome_dataset: args.export_decision_outcome_dataset.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ContinuationCommandInput {
    pub(super) config: BranchCampaignConfigV1,
    pub(super) resume: Option<PathBuf>,
    pub(super) resume_checkpoint: Option<PathBuf>,
    pub(super) out: Option<PathBuf>,
    pub(super) checkpoint_out: Option<PathBuf>,
    pub(super) plan_targeted_continuation: Option<PathBuf>,
    pub(super) execute_targeted_continuation: Option<PathBuf>,
    pub(super) continuation_effect_before: Option<PathBuf>,
    pub(super) continuation_effect_after: Option<PathBuf>,
    pub(super) targeted_continuation_limit: usize,
    pub(super) targeted_continuation_candidates_per_target: usize,
    pub(super) branch_examples: usize,
    pub(super) report_detail: BranchCampaignReportDetailV1,
}

impl ContinuationCommandInput {
    pub(super) fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            config: campaign_config_from_args(args)?,
            resume: args.resume.clone(),
            resume_checkpoint: args.resume_checkpoint.clone(),
            out: args.out.clone(),
            checkpoint_out: args.checkpoint_out.clone(),
            plan_targeted_continuation: args.plan_targeted_continuation.clone(),
            execute_targeted_continuation: args.execute_targeted_continuation.clone(),
            continuation_effect_before: args.continuation_effect_before.clone(),
            continuation_effect_after: args.continuation_effect_after.clone(),
            targeted_continuation_limit: args.targeted_continuation_limit,
            targeted_continuation_candidates_per_target: args
                .targeted_continuation_candidates_per_target,
            branch_examples: args.branch_examples,
            report_detail: BranchCampaignReportDetailV1::from(args.report_detail),
        })
    }
}

fn inspect_search_options_from_args(args: &Args) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = parse_branch_experiment_search_options_v1(&args.combat_search_options)?;
    options.max_nodes = args.search_max_nodes.or(options.max_nodes);
    options.wall_ms = options.wall_ms.or(Some(args.search_wall_ms));
    options.max_hp_loss = parse_hp_loss_limit(args.max_hp_loss.as_deref())?.or(options.max_hp_loss);
    Ok(options)
}
