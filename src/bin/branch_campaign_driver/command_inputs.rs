use std::path::PathBuf;

use sts_simulator::eval::branch_campaign::{
    BranchCampaignCombatRetryPolicyV1, BranchCampaignConfigV1, BranchCampaignProgressDetailV1,
    BranchCampaignReportDetailV1,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use sts_simulator::eval::branch_experiment_search_options::parse_branch_experiment_search_options_v1;
use sts_simulator::eval::learning_dataset_v1::CoverageGapContinuationFilterV1;
use sts_simulator::eval::neow_guided_prefix::{
    neow_guided_prefix_commands_v1, NeowGuidedPrefixConfigV1,
};
use sts_simulator::eval::run_control::{
    canonical_player_class, AutoCombatCaptureConfig, RunControlCombatSegmentMode,
    RunControlHpLossLimit, RunControlSearchCombatOptions,
};

use super::cli_args::{Args, BranchCampaignCombatRetryArgV1};

#[derive(Clone, Debug)]
pub(super) struct RunCommandInput {
    pub(super) config: BranchCampaignConfigV1,
    pub(super) round_budget: RoundBudgetRequestV1,
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
            round_budget: RoundBudgetRequestV1::from_args(args),
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
    pub(super) query: Option<String>,
    pub(super) coverage_gap_milestone_target: String,
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
            query: args.inspect_query.clone(),
            coverage_gap_milestone_target: args.coverage_gap_milestone_target.clone(),
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
    pub(super) resume_checkpoint: Option<PathBuf>,
    pub(super) inspect_report: Option<PathBuf>,
    pub(super) export_outcome_dataset: Option<PathBuf>,
    pub(super) analyze_outcome_dataset: Option<PathBuf>,
    pub(super) analyze_decision_outcome_dataset: Option<PathBuf>,
    pub(super) probe_learning_readiness: Option<PathBuf>,
    pub(super) export_learning_dataset: Option<PathBuf>,
    pub(super) export_decision_outcome_dataset: Option<PathBuf>,
    pub(super) plan_coverage_gap_continuation: bool,
    pub(super) coverage_gap_limit: usize,
    pub(super) coverage_gap_candidates_per_decision: usize,
    pub(super) coverage_gap_filter: CoverageGapContinuationFilterV1,
}

impl DatasetCommandInput {
    pub(super) fn from_args(args: &Args) -> Self {
        Self {
            inspect_checkpoint: args.inspect_checkpoint.clone(),
            resume_checkpoint: args.resume_checkpoint.clone(),
            inspect_report: args.inspect_report.clone(),
            export_outcome_dataset: args.export_outcome_dataset.clone(),
            analyze_outcome_dataset: args.analyze_outcome_dataset.clone(),
            analyze_decision_outcome_dataset: args.analyze_decision_outcome_dataset.clone(),
            probe_learning_readiness: args.probe_learning_readiness.clone(),
            export_learning_dataset: args.export_learning_dataset.clone(),
            export_decision_outcome_dataset: args.export_decision_outcome_dataset.clone(),
            plan_coverage_gap_continuation: args.plan_coverage_gap_continuation,
            coverage_gap_limit: args.coverage_gap_limit,
            coverage_gap_candidates_per_decision: args.coverage_gap_candidates_per_decision,
            coverage_gap_filter: coverage_gap_filter_from_args(args),
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct ContinuationCommandInput {
    pub(super) config: BranchCampaignConfigV1,
    pub(super) round_budget: RoundBudgetRequestV1,
    pub(super) resume: Option<PathBuf>,
    pub(super) resume_checkpoint: Option<PathBuf>,
    pub(super) out: Option<PathBuf>,
    pub(super) checkpoint_out: Option<PathBuf>,
    pub(super) plan_targeted_continuation: Option<PathBuf>,
    pub(super) execute_targeted_continuation: Option<PathBuf>,
    pub(super) execute_coverage_gap_continuation: bool,
    pub(super) continuation_effect_before: Option<PathBuf>,
    pub(super) continuation_effect_after: Option<PathBuf>,
    pub(super) targeted_continuation_limit: usize,
    pub(super) targeted_continuation_candidates_per_target: usize,
    pub(super) coverage_gap_limit: usize,
    pub(super) coverage_gap_candidates_per_decision: usize,
    pub(super) coverage_gap_filter: CoverageGapContinuationFilterV1,
    pub(super) coverage_gap_budget_intent: CoverageGapBudgetIntentV1,
    pub(super) coverage_gap_execution_mode: CoverageGapExecutionModeV1,
    pub(super) branch_examples: usize,
    pub(super) report_detail: BranchCampaignReportDetailV1,
}

impl ContinuationCommandInput {
    pub(super) fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            config: campaign_config_from_args(args)?,
            round_budget: RoundBudgetRequestV1::from_args(args),
            resume: args.resume.clone(),
            resume_checkpoint: args.resume_checkpoint.clone(),
            out: args.out.clone(),
            checkpoint_out: args.checkpoint_out.clone(),
            plan_targeted_continuation: args.plan_targeted_continuation.clone(),
            execute_targeted_continuation: args.execute_targeted_continuation.clone(),
            execute_coverage_gap_continuation: args.execute_coverage_gap_continuation,
            continuation_effect_before: args.continuation_effect_before.clone(),
            continuation_effect_after: args.continuation_effect_after.clone(),
            targeted_continuation_limit: args.targeted_continuation_limit,
            targeted_continuation_candidates_per_target: args
                .targeted_continuation_candidates_per_target,
            coverage_gap_limit: args.coverage_gap_limit,
            coverage_gap_candidates_per_decision: args.coverage_gap_candidates_per_decision,
            coverage_gap_filter: coverage_gap_filter_from_args(args),
            coverage_gap_budget_intent: CoverageGapBudgetIntentV1::parse(
                &args.coverage_gap_budget_intent,
            )?,
            coverage_gap_execution_mode: CoverageGapExecutionModeV1::parse(
                &args.coverage_gap_execution_mode,
            )?,
            branch_examples: args.branch_examples,
            report_detail: BranchCampaignReportDetailV1::from(args.report_detail),
        })
    }
}

fn coverage_gap_filter_from_args(args: &Args) -> CoverageGapContinuationFilterV1 {
    CoverageGapContinuationFilterV1 {
        bucket: args.coverage_gap_bucket.clone(),
        event_id: args.coverage_gap_event_id.clone(),
        lane: args.coverage_gap_lane.clone(),
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CoverageGapBudgetIntentV1 {
    GapClosure,
    FrontierExpansion,
}

impl CoverageGapBudgetIntentV1 {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().replace('-', "_").as_str() {
            "gap_closure" => Ok(Self::GapClosure),
            "frontier_expansion" => Ok(Self::FrontierExpansion),
            _ => Err(format!(
                "invalid --coverage-gap-budget-intent `{value}`; expected gap_closure or frontier_expansion"
            )),
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::GapClosure => "gap_closure",
            Self::FrontierExpansion => "frontier_expansion",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CoverageGapExecutionModeV1 {
    AdvanceRounds,
    TargetOnly,
}

impl CoverageGapExecutionModeV1 {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().replace('-', "_").as_str() {
            "advance_rounds" => Ok(Self::AdvanceRounds),
            "target_only" => Ok(Self::TargetOnly),
            _ => Err(format!(
                "invalid --coverage-gap-execution-mode `{value}`; expected advance_rounds or target_only"
            )),
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::AdvanceRounds => "advance_rounds",
            Self::TargetOnly => "target_only",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum RoundBudgetModeV1 {
    LegacyMaxRounds,
    Rounds,
    UntilRound,
}

impl RoundBudgetModeV1 {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::LegacyMaxRounds => "legacy_max_rounds",
            Self::Rounds => "rounds",
            Self::UntilRound => "until_round",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RoundBudgetResolutionV1 {
    pub(super) mode: RoundBudgetModeV1,
    pub(super) source_rounds: usize,
    pub(super) round_budget: usize,
    pub(super) target_total_rounds: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct RoundBudgetRequestV1 {
    legacy_max_rounds: usize,
    rounds: Option<usize>,
    until_round: Option<usize>,
}

impl RoundBudgetRequestV1 {
    fn from_args(args: &Args) -> Self {
        Self {
            legacy_max_rounds: args.max_rounds,
            rounds: args.rounds,
            until_round: args.until_round,
        }
    }

    pub(super) fn resolve_for_source_rounds(
        self,
        source_rounds: usize,
    ) -> Result<RoundBudgetResolutionV1, String> {
        if self.rounds.is_some() && self.until_round.is_some() {
            return Err("--rounds conflicts with --until-round".to_string());
        }
        let (mode, round_budget) = if let Some(rounds) = self.rounds {
            (RoundBudgetModeV1::Rounds, rounds)
        } else if let Some(until_round) = self.until_round {
            (
                RoundBudgetModeV1::UntilRound,
                until_round.saturating_sub(source_rounds),
            )
        } else {
            (RoundBudgetModeV1::LegacyMaxRounds, self.legacy_max_rounds)
        };
        Ok(RoundBudgetResolutionV1 {
            mode,
            source_rounds,
            round_budget,
            target_total_rounds: source_rounds.saturating_add(round_budget),
        })
    }
}

#[cfg(test)]
pub(super) fn round_budget_for_source_from_args(
    args: &Args,
    source_rounds: usize,
) -> Result<RoundBudgetResolutionV1, String> {
    RoundBudgetRequestV1::from_args(args).resolve_for_source_rounds(source_rounds)
}

pub(super) fn render_round_budget_resolution_v1(resolution: RoundBudgetResolutionV1) -> String {
    format!(
        "RoundBudgetV1 mode={} source_rounds={} round_budget={} target_total_rounds={}",
        resolution.mode.as_str(),
        resolution.source_rounds,
        resolution.round_budget,
        resolution.target_total_rounds
    )
}

fn inspect_search_options_from_args(args: &Args) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = parse_branch_experiment_search_options_v1(&args.combat_search_options)?;
    options.max_nodes = args.search_max_nodes.or(options.max_nodes);
    options.wall_ms = options.wall_ms.or(Some(args.search_wall_ms));
    options.max_hp_loss = parse_hp_loss_limit(args.max_hp_loss.as_deref())?.or(options.max_hp_loss);
    Ok(options)
}

pub(super) fn campaign_config_from_args(args: &Args) -> Result<BranchCampaignConfigV1, String> {
    let player_class = canonical_player_class(&args.player_class)?;
    let mut prefix_commands = Vec::new();
    if !args.no_neow_guidance {
        prefix_commands.extend(neow_guided_prefix_commands_v1(&NeowGuidedPrefixConfigV1 {
            seed: args.seed,
            ascension_level: args.ascension,
            final_act: args.final_act,
            player_class,
            search_max_nodes: args.search_max_nodes,
            search_wall_ms: Some(args.search_wall_ms),
        })?);
    } else {
        prefix_commands.push("0".to_string());
    }
    prefix_commands.extend(args.prefix_commands.iter().cloned());

    let search_max_hp_loss = parse_hp_loss_limit(args.max_hp_loss.as_deref())?
        .or(Some(RunControlHpLossLimit::Unlimited));

    Ok(BranchCampaignConfigV1 {
        seed: args.seed,
        ascension_level: args.ascension,
        player_class,
        final_act: args.final_act,
        max_rounds: args.rounds.unwrap_or(args.max_rounds),
        round_depth: args.round_depth,
        max_active: args.max_active,
        max_frozen: args.max_frozen,
        max_branches_per_active: args.max_branches_per_active,
        active_lineage_diversity_slots: args.active_lineage_diversity,
        boss_relic_axis_isolation: args.boss_relic_axes,
        retention_budget_profile: args
            .retention_profile
            .parse::<BranchRetentionBudgetProfileV1>()?,
        max_reward_options_per_branch: if args.all_reward_options {
            None
        } else {
            Some(args.max_reward_options.unwrap_or(2))
        },
        max_campfire_options_per_branch: args.max_campfire_options,
        auto_max_operations: args.auto_max_ops,
        experiment_wall_ms: Some(args.experiment_wall_ms),
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: Some(args.search_wall_ms),
        search_max_hp_loss,
        search_options: campaign_search_options_from_args(args)?,
        auto_capture: AutoCombatCaptureConfig {
            enabled: args.auto_capture_combat,
            root: args.auto_capture_root.clone(),
        },
        combat_retry_policy: match args.combat_retry {
            BranchCampaignCombatRetryArgV1::OnStall => BranchCampaignCombatRetryPolicyV1::OnStall,
            BranchCampaignCombatRetryArgV1::Immediate => {
                BranchCampaignCombatRetryPolicyV1::Immediate
            }
            BranchCampaignCombatRetryArgV1::Disabled => BranchCampaignCombatRetryPolicyV1::Disabled,
        },
        combat_retry_wall_ms: args.combat_retry_wall_ms,
        include_event_reward_skip: false,
        min_acceptable_victory_hp_percent: args.min_acceptable_victory_hp_percent,
        prefix_commands,
    })
}

pub(super) fn campaign_search_options_from_args(
    args: &Args,
) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = parse_branch_experiment_search_options_v1(&args.combat_search_options)?;
    if !combat_search_options_include_segment_mode(&args.combat_search_options) {
        options.segment_mode = Some(RunControlCombatSegmentMode::NonBossTurnBoundary);
    }
    Ok(options)
}

fn combat_search_options_include_segment_mode(tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        token.split_once('=').is_some_and(|(key, _)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "segment" | "segment_mode" | "partial" | "partial_mode"
            )
        })
    })
}

fn parse_hp_loss_limit(value: Option<&str>) -> Result<Option<RunControlHpLossLimit>, String> {
    let Some(value) = value else {
        return Ok(None);
    };
    match value.to_ascii_lowercase().as_str() {
        "off" | "none" | "unlimited" | "no_limit" | "no-limit" => {
            Ok(Some(RunControlHpLossLimit::Unlimited))
        }
        _ => value
            .parse::<u32>()
            .map(RunControlHpLossLimit::Limit)
            .map(Some)
            .map_err(|err| format!("invalid --max-hp-loss `{value}`: {err}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli_args::Args;

    #[test]
    fn continuation_input_parses_coverage_gap_budget_intent() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "continue",
            "--execute-coverage-gap-continuation",
            "--coverage-gap-budget-intent",
            "frontier-expansion",
        ])
        .expect("coverage gap budget intent should parse");

        let input = ContinuationCommandInput::from_args(&args).expect("input should build");

        assert_eq!(
            input.coverage_gap_budget_intent,
            CoverageGapBudgetIntentV1::FrontierExpansion
        );
    }

    #[test]
    fn continuation_input_parses_coverage_gap_execution_mode() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "continue",
            "--execute-coverage-gap-continuation",
            "--coverage-gap-execution-mode",
            "target-only",
        ])
        .expect("coverage gap execution mode should parse");

        let input = ContinuationCommandInput::from_args(&args).expect("input should build");

        assert_eq!(
            input.coverage_gap_execution_mode,
            CoverageGapExecutionModeV1::TargetOnly
        );
    }

    #[test]
    fn dataset_input_parses_coverage_gap_filter() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "dataset",
            "--plan-coverage-gap-continuation",
            "--coverage-gap-bucket",
            "event",
            "--coverage-gap-event-id",
            "TheLibrary",
            "--coverage-gap-lane",
            "effect:event_card_reward",
        ])
        .expect("coverage gap filter should parse");

        let input = DatasetCommandInput::from_args(&args);

        assert_eq!(input.coverage_gap_filter.bucket.as_deref(), Some("event"));
        assert_eq!(
            input.coverage_gap_filter.event_id.as_deref(),
            Some("TheLibrary")
        );
        assert_eq!(
            input.coverage_gap_filter.lane.as_deref(),
            Some("effect:event_card_reward")
        );
    }

    #[test]
    fn continuation_input_parses_coverage_gap_filter() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "continue",
            "--execute-coverage-gap-continuation",
            "--coverage-gap-bucket",
            "event",
            "--coverage-gap-event-id",
            "TheLibrary",
            "--coverage-gap-lane",
            "effect:event_card_reward",
        ])
        .expect("coverage gap filter should parse");

        let input = ContinuationCommandInput::from_args(&args).expect("input should build");

        assert_eq!(input.coverage_gap_filter.bucket.as_deref(), Some("event"));
        assert_eq!(
            input.coverage_gap_filter.event_id.as_deref(),
            Some("TheLibrary")
        );
        assert_eq!(
            input.coverage_gap_filter.lane.as_deref(),
            Some("effect:event_card_reward")
        );
    }
}
