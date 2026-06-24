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

use super::cli_args::{
    Args, ArtifactCommandArgs, ArtifactKindArgV1, ArtifactSubcommandV1,
    BranchCampaignCombatRetryArgV1, CampaignBranchingArgs, CampaignCombatRetryArgs,
    CampaignCoverageExecuteCommandArgs, CampaignCoverageExecuteTargetArgs,
    CampaignCoveragePlanCommandArgs, CampaignDomainArgs, CampaignPrefixArgs, CampaignSearchArgs,
    ContinueCommandArgs, DatasetCommandArgs, InspectChallengeArgs, InspectCommandArgs,
    InspectEvidenceDetailArg, InspectModeArgs, RunCommandArgs,
};

#[derive(Clone, Debug)]
pub(super) struct RunCommandInput {
    pub(super) config: BranchCampaignConfigV1,
    pub(super) round_budget: RoundBudgetRequestV1,
    pub(super) milestone: MilestoneContinuationRequestV1,
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
    pub(super) fn from_run_args(args: RunCommandArgs) -> Result<Self, String> {
        let config = campaign_config_from_cli_parts(
            &args.domain,
            &args.branching,
            &args.search,
            &args.retry,
            &args.prefix,
            AutoCaptureInputParts {
                enabled: args.output.auto_capture_combat,
                root: args.output.auto_capture_root.clone(),
            },
        )?;
        Ok(Self {
            config,
            round_budget: RoundBudgetRequestV1::from_branching_args(&args.branching),
            milestone: MilestoneContinuationRequestV1::from_branching_args(&args.branching)?,
            progress: args.output.progress,
            progress_detail: BranchCampaignProgressDetailV1::from(args.output.progress_detail),
            json: args.output.json,
            resume: args.output.resume,
            resume_checkpoint: args.output.resume_checkpoint,
            out: args.output.out,
            checkpoint_out: args.output.checkpoint_out,
            export_outcome_dataset: args.output.export_outcome_dataset,
            export_learning_dataset: args.output.export_learning_dataset,
            export_decision_outcome_dataset: args.output.export_decision_outcome_dataset,
            branch_examples: args.output.branch_examples,
            report_detail: BranchCampaignReportDetailV1::from(args.output.report_detail),
        })
    }

    pub(super) fn from_continue_args(args: ContinueCommandArgs) -> Result<Self, String> {
        let config = campaign_config_from_cli_parts(
            &args.domain,
            &args.branching,
            &args.search,
            &args.retry,
            &args.prefix,
            AutoCaptureInputParts {
                enabled: args.output.auto_capture_combat,
                root: args.output.auto_capture_root.clone(),
            },
        )?;
        Ok(Self {
            config,
            round_budget: RoundBudgetRequestV1::from_branching_args(&args.branching),
            milestone: MilestoneContinuationRequestV1::from_branching_args(&args.branching)?,
            progress: args.output.progress,
            progress_detail: BranchCampaignProgressDetailV1::from(args.output.progress_detail),
            json: args.output.json,
            resume: args.output.resume,
            resume_checkpoint: args.output.resume_checkpoint,
            out: args.output.out,
            checkpoint_out: args.output.checkpoint_out,
            export_outcome_dataset: args.output.export_outcome_dataset,
            export_learning_dataset: args.output.export_learning_dataset,
            export_decision_outcome_dataset: args.output.export_decision_outcome_dataset,
            branch_examples: args.output.branch_examples,
            report_detail: BranchCampaignReportDetailV1::from(args.output.report_detail),
        })
    }

    pub(super) fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            config: campaign_config_from_args(args)?,
            round_budget: RoundBudgetRequestV1::from_args(args),
            milestone: MilestoneContinuationRequestV1::from_args(args)?,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CampaignMilestoneTargetV1 {
    Act1Boss,
    Act2Start,
    Act2Boss,
    Act3Boss,
    CurrentActBoss,
}

impl CampaignMilestoneTargetV1 {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().replace(['-', '_'], "").as_str() {
            "act1boss" => Ok(Self::Act1Boss),
            "act2start" => Ok(Self::Act2Start),
            "act2boss" => Ok(Self::Act2Boss),
            "act3boss" => Ok(Self::Act3Boss),
            "currentactboss" => Ok(Self::CurrentActBoss),
            _ => Err(format!(
                "invalid --until-milestone `{value}`; expected Act1Boss, Act2Start, Act2Boss, Act3Boss, or CurrentActBoss"
            )),
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Act1Boss => "Act1Boss",
            Self::Act2Start => "Act2Start",
            Self::Act2Boss => "Act2Boss",
            Self::Act3Boss => "Act3Boss",
            Self::CurrentActBoss => "CurrentActBoss",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CampaignMilestoneStopV1 {
    Auto,
    FirstHit,
    RoundCap,
}

impl CampaignMilestoneStopV1 {
    fn parse(value: &str) -> Result<Self, String> {
        match value.to_ascii_lowercase().replace('-', "_").as_str() {
            "auto" => Ok(Self::Auto),
            "first_hit" => Ok(Self::FirstHit),
            "round_cap" => Ok(Self::RoundCap),
            _ => Err(format!(
                "invalid --milestone-stop `{value}`; expected auto, first_hit, or round_cap"
            )),
        }
    }

    pub(super) fn resolve_for_run(self) -> Self {
        match self {
            Self::Auto => Self::FirstHit,
            other => other,
        }
    }

    pub(super) fn resolve_for_coverage_gap(self) -> Self {
        match self {
            Self::Auto => Self::RoundCap,
            other => other,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::FirstHit => "first_hit",
            Self::RoundCap => "round_cap",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct MilestoneContinuationRequestV1 {
    pub(super) target: Option<CampaignMilestoneTargetV1>,
    pub(super) step_rounds: usize,
    pub(super) max_rounds: usize,
    pub(super) stop: CampaignMilestoneStopV1,
}

impl MilestoneContinuationRequestV1 {
    fn from_branching_args(branching: &CampaignBranchingArgs) -> Result<Self, String> {
        if branching.until_milestone.is_some()
            && (branching.rounds.is_some() || branching.until_round.is_some())
        {
            return Err("--until-milestone owns the round budget; use --milestone-step-rounds and --milestone-max-rounds instead of --rounds or --until-round".to_string());
        }
        if branching.until_milestone.is_some() && branching.milestone_step_rounds == 0 {
            return Err("--milestone-step-rounds must be greater than zero".to_string());
        }
        let target = branching
            .until_milestone
            .as_deref()
            .map(CampaignMilestoneTargetV1::parse)
            .transpose()?;
        Ok(Self {
            target,
            step_rounds: branching.milestone_step_rounds,
            max_rounds: branching.milestone_max_rounds,
            stop: CampaignMilestoneStopV1::parse(&branching.milestone_stop)?,
        })
    }

    fn from_args(args: &Args) -> Result<Self, String> {
        if args.until_milestone.is_some() && (args.rounds.is_some() || args.until_round.is_some()) {
            return Err("--until-milestone owns the round budget; use --milestone-step-rounds and --milestone-max-rounds instead of --rounds or --until-round".to_string());
        }
        if args.until_milestone.is_some() && args.milestone_step_rounds == 0 {
            return Err("--milestone-step-rounds must be greater than zero".to_string());
        }
        let target = args
            .until_milestone
            .as_deref()
            .map(CampaignMilestoneTargetV1::parse)
            .transpose()?;
        Ok(Self {
            target,
            step_rounds: args.milestone_step_rounds,
            max_rounds: args.milestone_max_rounds,
            stop: CampaignMilestoneStopV1::parse(&args.milestone_stop)?,
        })
    }

    pub(super) fn enabled(self) -> bool {
        self.target.is_some()
    }
}

#[derive(Clone, Debug)]
pub(super) enum ArtifactCommandInput {
    Resolve {
        campaign_dir: PathBuf,
        selector: String,
        json: bool,
    },
    SourceInfo {
        campaign_dir: PathBuf,
        selector: String,
        json: bool,
    },
    Allocate {
        campaign_dir: PathBuf,
        kind: ArtifactKindArgV1,
        label: String,
        stamp: Option<String>,
        suffix: Option<String>,
        json: bool,
    },
    WriteLatest {
        campaign_dir: PathBuf,
        kind: ArtifactKindArgV1,
        artifact_id: String,
        updated_at: String,
        json: bool,
    },
    WriteManifest {
        manifest_path: PathBuf,
        payload_schema_name: String,
        created_at: String,
        json: bool,
    },
    Prune {
        campaign_dir: PathBuf,
        keep_runs: usize,
        keep_scratch: usize,
        apply: bool,
        json: bool,
    },
}

impl ArtifactCommandInput {
    pub(super) fn from_artifact_command_args(args: ArtifactCommandArgs) -> Self {
        match args.command {
            ArtifactSubcommandV1::Resolve(resolve) => Self::Resolve {
                campaign_dir: resolve.campaign_dir,
                selector: resolve.selector,
                json: resolve.json,
            },
            ArtifactSubcommandV1::SourceInfo(source_info) => Self::SourceInfo {
                campaign_dir: source_info.campaign_dir,
                selector: source_info.selector,
                json: source_info.json,
            },
            ArtifactSubcommandV1::Allocate(allocate) => Self::Allocate {
                campaign_dir: allocate.campaign_dir,
                kind: allocate.kind,
                label: allocate.label,
                stamp: allocate.stamp,
                suffix: allocate.suffix,
                json: allocate.json,
            },
            ArtifactSubcommandV1::WriteLatest(write_latest) => Self::WriteLatest {
                campaign_dir: write_latest.campaign_dir,
                kind: write_latest.kind,
                artifact_id: write_latest.artifact_id,
                updated_at: write_latest.updated_at,
                json: write_latest.json,
            },
            ArtifactSubcommandV1::WriteManifest(write_manifest) => Self::WriteManifest {
                manifest_path: write_manifest.manifest_path,
                payload_schema_name: write_manifest.payload_schema_name,
                created_at: write_manifest.created_at,
                json: write_manifest.json,
            },
            ArtifactSubcommandV1::Prune(prune) => Self::Prune {
                campaign_dir: prune.campaign_dir,
                keep_runs: prune.keep_runs,
                keep_scratch: prune.keep_scratch,
                apply: prune.apply,
                json: prune.json,
            },
        }
    }
}

#[derive(Clone, Debug)]
pub(super) struct InspectCommandInput {
    pub(super) checkpoint_path: Option<PathBuf>,
    pub(super) report_path: Option<PathBuf>,
    pub(super) summary: bool,
    pub(super) query: Option<String>,
    pub(super) coverage_gap_milestone_target: String,
    pub(super) coverage_gap_filter: CoverageGapContinuationFilterV1,
    pub(super) filters: InspectFiltersInput,
    pub(super) modes: InspectModesInput,
    pub(super) search_options: RunControlSearchCombatOptions,
    pub(super) branch_examples: usize,
    pub(super) shop_challenge: ShopChallengeInput,
}

impl InspectCommandInput {
    pub(super) fn from_inspect_args(args: InspectCommandArgs) -> Result<Self, String> {
        let coverage_gap_filter = coverage_gap_filter_from_inspect_mode_args(&args.modes);
        let search_options = inspect_search_options_from_search_args(&args.search)?;
        let shop_challenge = ShopChallengeInput::from_inspect_parts(&args.challenge, &args.search)?;
        Ok(Self {
            checkpoint_path: args.target.inspect_checkpoint,
            report_path: args.target.inspect_report,
            summary: args.modes.inspect_summary,
            query: args.modes.inspect_query,
            coverage_gap_milestone_target: args.modes.coverage_gap_milestone_target,
            coverage_gap_filter,
            filters: InspectFiltersInput {
                act: args.target.inspect_act,
                floor: args.target.inspect_floor,
                boundary: args.target.inspect_boundary,
                hp: args.target.inspect_hp,
                index: args.target.inspect_index,
            },
            modes: InspectModesInput {
                search: args.modes.inspect_search,
                last_auto_combat: args.modes.inspect_last_auto_combat,
                combat_lab: args.modes.inspect_combat_lab,
                probe_boss: args.modes.probe_boss,
                shop_evidence: args.modes.inspect_shop_evidence,
                evidence_detail: InspectEvidenceDetailV1::from(args.modes.inspect_evidence_detail),
                shop_challenge: args.modes.challenge_shop_plans,
                card_reward_evidence: args.modes.inspect_card_reward_evidence,
                campfire_evidence: args.modes.inspect_campfire_evidence,
                deck_mutation: args.modes.inspect_deck_mutation,
                route_evidence: args.modes.inspect_route_evidence,
            },
            search_options,
            branch_examples: args.display.branch_examples,
            shop_challenge,
        })
    }

    pub(super) fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            checkpoint_path: args.inspect_checkpoint.clone(),
            report_path: args.inspect_report.clone(),
            summary: args.inspect_summary,
            query: args.inspect_query.clone(),
            coverage_gap_milestone_target: args.coverage_gap_milestone_target.clone(),
            coverage_gap_filter: coverage_gap_filter_from_args(args),
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
                evidence_detail: InspectEvidenceDetailV1::from(args.inspect_evidence_detail),
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
    pub(super) evidence_detail: InspectEvidenceDetailV1,
    pub(super) shop_challenge: bool,
    pub(super) card_reward_evidence: bool,
    pub(super) campfire_evidence: bool,
    pub(super) deck_mutation: bool,
    pub(super) route_evidence: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum InspectEvidenceDetailV1 {
    Compact,
    Full,
}

impl From<InspectEvidenceDetailArg> for InspectEvidenceDetailV1 {
    fn from(value: InspectEvidenceDetailArg) -> Self {
        match value {
            InspectEvidenceDetailArg::Compact => Self::Compact,
            InspectEvidenceDetailArg::Full => Self::Full,
        }
    }
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
    fn from_inspect_parts(
        challenge: &InspectChallengeArgs,
        search: &CampaignSearchArgs,
    ) -> Result<Self, String> {
        Ok(Self {
            final_act: challenge.final_act,
            max_reward_options_per_branch: if challenge.all_reward_options {
                None
            } else {
                Some(challenge.max_reward_options.unwrap_or(2))
            },
            max_campfire_options_per_branch: challenge.max_campfire_options,
            auto_max_operations: challenge.auto_max_ops,
            experiment_wall_ms: challenge.experiment_wall_ms,
            challenge_max_plans: challenge.challenge_max_plans,
            challenge_depth: challenge.challenge_depth,
            challenge_max_branches: challenge.challenge_max_branches,
            search_max_nodes: search.search_max_nodes,
            search_wall_ms: search.search_wall_ms,
            search_options: campaign_search_options_from_parts(&search.combat_search_options)?,
            retention_budget_profile: "package".parse::<BranchRetentionBudgetProfileV1>()?,
        })
    }

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
    pub(super) fn from_inspect_args_for_decision_coverage(args: InspectCommandArgs) -> Self {
        Self {
            inspect_checkpoint: args.target.inspect_checkpoint,
            inspect_report: args.target.inspect_report,
            export_outcome_dataset: None,
            analyze_outcome_dataset: None,
            analyze_decision_outcome_dataset: None,
            probe_learning_readiness: None,
            export_learning_dataset: None,
            export_decision_outcome_dataset: None,
        }
    }

    pub(super) fn from_dataset_args(args: DatasetCommandArgs) -> Self {
        let paths = args.paths;
        Self {
            inspect_checkpoint: paths.inspect_checkpoint,
            inspect_report: paths.inspect_report,
            export_outcome_dataset: paths.export_outcome_dataset,
            analyze_outcome_dataset: paths.analyze_outcome_dataset,
            analyze_decision_outcome_dataset: paths.analyze_decision_outcome_dataset,
            probe_learning_readiness: paths.probe_learning_readiness,
            export_learning_dataset: paths.export_learning_dataset,
            export_decision_outcome_dataset: paths.export_decision_outcome_dataset,
        }
    }

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
pub(super) struct CoverageGapPlanCommandInput {
    pub(super) inspect_checkpoint: Option<PathBuf>,
    pub(super) resume_checkpoint: Option<PathBuf>,
    pub(super) inspect_report: Option<PathBuf>,
    pub(super) coverage_gap_limit: usize,
    pub(super) coverage_gap_candidates_per_decision: usize,
    pub(super) coverage_gap_filter: CoverageGapContinuationFilterV1,
    pub(super) coverage_gap_budget_intent: CoverageGapBudgetIntentV1,
}

impl CoverageGapPlanCommandInput {
    pub(super) fn from_coverage_plan_args(
        args: CampaignCoveragePlanCommandArgs,
    ) -> Result<Self, String> {
        let target = args.target;
        Ok(Self {
            inspect_checkpoint: target.inspect_checkpoint,
            resume_checkpoint: None,
            inspect_report: target.inspect_report,
            coverage_gap_limit: target.coverage_gap_limit,
            coverage_gap_candidates_per_decision: target.coverage_gap_candidates_per_decision,
            coverage_gap_filter: CoverageGapContinuationFilterV1 {
                bucket: target.coverage_gap_bucket,
                event_id: target.coverage_gap_event_id,
                lane: target.coverage_gap_lane,
                origin_source: target.coverage_gap_origin_source,
                progress: target.coverage_gap_progress,
            },
            coverage_gap_budget_intent: CoverageGapBudgetIntentV1::parse(
                &target.coverage_gap_budget_intent,
            )?,
        })
    }

    pub(super) fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            inspect_checkpoint: args.inspect_checkpoint.clone(),
            resume_checkpoint: args.resume_checkpoint.clone(),
            inspect_report: args.inspect_report.clone(),
            coverage_gap_limit: args.coverage_gap_limit,
            coverage_gap_candidates_per_decision: args.coverage_gap_candidates_per_decision,
            coverage_gap_filter: coverage_gap_filter_from_args(args),
            coverage_gap_budget_intent: CoverageGapBudgetIntentV1::parse(
                &args.coverage_gap_budget_intent,
            )?,
        })
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
    pub(super) continuation_effect_before: Option<PathBuf>,
    pub(super) continuation_effect_after: Option<PathBuf>,
    pub(super) targeted_continuation_limit: usize,
    pub(super) targeted_continuation_candidates_per_target: usize,
    pub(super) branch_examples: usize,
    pub(super) report_detail: BranchCampaignReportDetailV1,
}

impl ContinuationCommandInput {
    pub(super) fn from_continue_args(args: ContinueCommandArgs) -> Result<Self, String> {
        Ok(Self {
            config: campaign_config_from_cli_parts(
                &args.domain,
                &args.branching,
                &args.search,
                &args.retry,
                &args.prefix,
                AutoCaptureInputParts {
                    enabled: args.output.auto_capture_combat,
                    root: args.output.auto_capture_root.clone(),
                },
            )?,
            round_budget: RoundBudgetRequestV1::from_branching_args(&args.branching),
            resume: args.output.resume,
            resume_checkpoint: args.output.resume_checkpoint,
            out: args.output.out,
            checkpoint_out: args.output.checkpoint_out,
            plan_targeted_continuation: args.continuation.plan_targeted_continuation,
            execute_targeted_continuation: args.continuation.execute_targeted_continuation,
            continuation_effect_before: args.continuation.continuation_effect_before,
            continuation_effect_after: args.continuation.continuation_effect_after,
            targeted_continuation_limit: args.continuation.targeted_continuation_limit,
            targeted_continuation_candidates_per_target: args
                .continuation
                .targeted_continuation_candidates_per_target,
            branch_examples: args.output.branch_examples,
            report_detail: BranchCampaignReportDetailV1::from(args.output.report_detail),
        })
    }

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

#[derive(Clone, Debug)]
pub(super) struct CoverageGapExecutionCommandInput {
    pub(super) config: BranchCampaignConfigV1,
    pub(super) round_budget: RoundBudgetRequestV1,
    pub(super) milestone: MilestoneContinuationRequestV1,
    pub(super) resume: Option<PathBuf>,
    pub(super) resume_checkpoint: Option<PathBuf>,
    pub(super) out: Option<PathBuf>,
    pub(super) checkpoint_out: Option<PathBuf>,
    pub(super) coverage_gap_limit: usize,
    pub(super) coverage_gap_candidates_per_decision: usize,
    pub(super) coverage_gap_filter: CoverageGapContinuationFilterV1,
    pub(super) coverage_gap_budget_intent: CoverageGapBudgetIntentV1,
    pub(super) coverage_gap_execution_mode: CoverageGapExecutionModeV1,
    pub(super) branch_examples: usize,
    pub(super) report_detail: BranchCampaignReportDetailV1,
}

impl CoverageGapExecutionCommandInput {
    pub(super) fn from_continue_args(args: ContinueCommandArgs) -> Result<Self, String> {
        let continuation = args.continuation;
        let coverage_gap_filter = coverage_gap_filter_from_continuation_args(&continuation);
        let coverage_gap_budget_intent =
            CoverageGapBudgetIntentV1::parse(&continuation.coverage_gap_budget_intent)?;
        let coverage_gap_execution_mode =
            CoverageGapExecutionModeV1::parse(&continuation.coverage_gap_execution_mode)?;
        Ok(Self {
            config: campaign_config_from_cli_parts(
                &args.domain,
                &args.branching,
                &args.search,
                &args.retry,
                &args.prefix,
                AutoCaptureInputParts {
                    enabled: args.output.auto_capture_combat,
                    root: args.output.auto_capture_root.clone(),
                },
            )?,
            round_budget: RoundBudgetRequestV1::from_branching_args(&args.branching),
            milestone: MilestoneContinuationRequestV1::from_branching_args(&args.branching)?,
            resume: args.output.resume,
            resume_checkpoint: args.output.resume_checkpoint,
            out: args.output.out,
            checkpoint_out: args.output.checkpoint_out,
            coverage_gap_limit: continuation.coverage_gap_limit,
            coverage_gap_candidates_per_decision: continuation.coverage_gap_candidates_per_decision,
            coverage_gap_filter,
            coverage_gap_budget_intent,
            coverage_gap_execution_mode,
            branch_examples: args.output.branch_examples,
            report_detail: BranchCampaignReportDetailV1::from(args.output.report_detail),
        })
    }

    pub(super) fn from_coverage_execute_args(
        args: CampaignCoverageExecuteCommandArgs,
    ) -> Result<Self, String> {
        let coverage = args.coverage;
        let coverage_gap_filter = coverage_gap_filter_from_coverage_execute_args(&coverage);
        let coverage_gap_budget_intent =
            CoverageGapBudgetIntentV1::parse(&coverage.coverage_gap_budget_intent)?;
        let coverage_gap_execution_mode =
            CoverageGapExecutionModeV1::parse(&coverage.coverage_gap_execution_mode)?;
        Ok(Self {
            config: campaign_config_from_cli_parts(
                &args.domain,
                &args.branching,
                &args.search,
                &args.retry,
                &args.prefix,
                AutoCaptureInputParts::disabled(),
            )?,
            round_budget: RoundBudgetRequestV1::from_branching_args(&args.branching),
            milestone: MilestoneContinuationRequestV1::from_branching_args(&args.branching)?,
            resume: args.output.resume,
            resume_checkpoint: args.output.resume_checkpoint,
            out: args.output.out,
            checkpoint_out: args.output.checkpoint_out,
            coverage_gap_limit: coverage.coverage_gap_limit,
            coverage_gap_candidates_per_decision: coverage.coverage_gap_candidates_per_decision,
            coverage_gap_filter,
            coverage_gap_budget_intent,
            coverage_gap_execution_mode,
            branch_examples: args.output.branch_examples,
            report_detail: BranchCampaignReportDetailV1::from(args.output.report_detail),
        })
    }

    pub(super) fn from_args(args: &Args) -> Result<Self, String> {
        Ok(Self {
            config: campaign_config_from_args(args)?,
            round_budget: RoundBudgetRequestV1::from_args(args),
            milestone: MilestoneContinuationRequestV1::from_args(args)?,
            resume: args.resume.clone(),
            resume_checkpoint: args.resume_checkpoint.clone(),
            out: args.out.clone(),
            checkpoint_out: args.checkpoint_out.clone(),
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
        origin_source: args.coverage_gap_origin_source.clone(),
        progress: args.coverage_gap_progress.clone(),
    }
}

fn coverage_gap_filter_from_coverage_execute_args(
    args: &CampaignCoverageExecuteTargetArgs,
) -> CoverageGapContinuationFilterV1 {
    CoverageGapContinuationFilterV1 {
        bucket: args.coverage_gap_bucket.clone(),
        event_id: args.coverage_gap_event_id.clone(),
        lane: args.coverage_gap_lane.clone(),
        origin_source: args.coverage_gap_origin_source.clone(),
        progress: args.coverage_gap_progress.clone(),
    }
}

fn coverage_gap_filter_from_continuation_args(
    args: &super::cli_args::ContinuationArgs,
) -> CoverageGapContinuationFilterV1 {
    CoverageGapContinuationFilterV1 {
        bucket: args.coverage_gap_bucket.clone(),
        event_id: args.coverage_gap_event_id.clone(),
        lane: args.coverage_gap_lane.clone(),
        origin_source: args.coverage_gap_origin_source.clone(),
        progress: args.coverage_gap_progress.clone(),
    }
}

fn coverage_gap_filter_from_inspect_mode_args(
    args: &InspectModeArgs,
) -> CoverageGapContinuationFilterV1 {
    CoverageGapContinuationFilterV1 {
        bucket: args.coverage_gap_bucket.clone(),
        event_id: args.coverage_gap_event_id.clone(),
        lane: args.coverage_gap_lane.clone(),
        origin_source: args.coverage_gap_origin_source.clone(),
        progress: args.coverage_gap_progress.clone(),
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
    UntilMilestone,
}

impl RoundBudgetModeV1 {
    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::LegacyMaxRounds => "legacy_max_rounds",
            Self::Rounds => "rounds",
            Self::UntilRound => "until_round",
            Self::UntilMilestone => "until_milestone",
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
    fn from_branching_args(branching: &CampaignBranchingArgs) -> Self {
        Self {
            legacy_max_rounds: branching.max_rounds,
            rounds: branching.rounds,
            until_round: branching.until_round,
        }
    }

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

fn inspect_search_options_from_search_args(
    search: &CampaignSearchArgs,
) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = parse_branch_experiment_search_options_v1(&search.combat_search_options)?;
    options.max_nodes = search.search_max_nodes.or(options.max_nodes);
    options.wall_ms = options.wall_ms.or(Some(search.search_wall_ms));
    options.max_hp_loss =
        parse_hp_loss_limit(search.max_hp_loss.as_deref())?.or(options.max_hp_loss);
    Ok(options)
}

#[derive(Clone, Debug)]
struct AutoCaptureInputParts {
    enabled: bool,
    root: Option<PathBuf>,
}

impl AutoCaptureInputParts {
    fn disabled() -> Self {
        Self {
            enabled: false,
            root: None,
        }
    }
}

struct CampaignConfigInputParts<'a> {
    seed: u64,
    ascension: u8,
    player_class: &'a str,
    final_act: bool,
    max_rounds: usize,
    rounds: Option<usize>,
    round_depth: usize,
    max_active: usize,
    max_frozen: usize,
    max_branches_per_active: usize,
    boss_relic_axes: bool,
    retention_profile: &'a str,
    all_reward_options: bool,
    max_reward_options: Option<usize>,
    max_campfire_options: usize,
    auto_max_ops: usize,
    experiment_wall_ms: u64,
    search_max_nodes: Option<usize>,
    search_wall_ms: u64,
    max_hp_loss: Option<&'a str>,
    combat_search_options: &'a [String],
    combat_retry: BranchCampaignCombatRetryArgV1,
    combat_retry_wall_ms: Option<u64>,
    min_acceptable_victory_hp_percent: u8,
    prefix_commands: &'a [String],
    no_neow_guidance: bool,
    auto_capture: AutoCaptureInputParts,
}

pub(super) fn campaign_config_from_args(args: &Args) -> Result<BranchCampaignConfigV1, String> {
    campaign_config_from_parts(CampaignConfigInputParts {
        seed: args.seed,
        ascension: args.ascension,
        player_class: &args.player_class,
        final_act: args.final_act,
        max_rounds: args.max_rounds,
        rounds: args.rounds,
        round_depth: args.round_depth,
        max_active: args.max_active,
        max_frozen: args.max_frozen,
        max_branches_per_active: args.max_branches_per_active,
        boss_relic_axes: args.boss_relic_axes,
        retention_profile: &args.retention_profile,
        all_reward_options: args.all_reward_options,
        max_reward_options: args.max_reward_options,
        max_campfire_options: args.max_campfire_options,
        auto_max_ops: args.auto_max_ops,
        experiment_wall_ms: args.experiment_wall_ms,
        search_max_nodes: args.search_max_nodes,
        search_wall_ms: args.search_wall_ms,
        max_hp_loss: args.max_hp_loss.as_deref(),
        combat_search_options: &args.combat_search_options,
        combat_retry: args.combat_retry,
        combat_retry_wall_ms: args.combat_retry_wall_ms,
        min_acceptable_victory_hp_percent: args.min_acceptable_victory_hp_percent,
        prefix_commands: &args.prefix_commands,
        no_neow_guidance: args.no_neow_guidance,
        auto_capture: AutoCaptureInputParts {
            enabled: args.auto_capture_combat,
            root: args.auto_capture_root.clone(),
        },
    })
}

fn campaign_config_from_cli_parts(
    domain: &CampaignDomainArgs,
    branching: &CampaignBranchingArgs,
    search: &CampaignSearchArgs,
    retry: &CampaignCombatRetryArgs,
    prefix: &CampaignPrefixArgs,
    auto_capture: AutoCaptureInputParts,
) -> Result<BranchCampaignConfigV1, String> {
    campaign_config_from_parts(CampaignConfigInputParts {
        seed: domain.seed,
        ascension: domain.ascension,
        player_class: &domain.player_class,
        final_act: domain.final_act,
        max_rounds: branching.max_rounds,
        rounds: branching.rounds,
        round_depth: branching.round_depth,
        max_active: branching.max_active,
        max_frozen: branching.max_frozen,
        max_branches_per_active: branching.max_branches_per_active,
        boss_relic_axes: branching.boss_relic_axes,
        retention_profile: &branching.retention_profile,
        all_reward_options: branching.all_reward_options,
        max_reward_options: branching.max_reward_options,
        max_campfire_options: branching.max_campfire_options,
        auto_max_ops: branching.auto_max_ops,
        experiment_wall_ms: branching.experiment_wall_ms,
        search_max_nodes: search.search_max_nodes,
        search_wall_ms: search.search_wall_ms,
        max_hp_loss: search.max_hp_loss.as_deref(),
        combat_search_options: &search.combat_search_options,
        combat_retry: retry.combat_retry,
        combat_retry_wall_ms: retry.combat_retry_wall_ms,
        min_acceptable_victory_hp_percent: retry.min_acceptable_victory_hp_percent,
        prefix_commands: &prefix.prefix_commands,
        no_neow_guidance: prefix.no_neow_guidance,
        auto_capture,
    })
}

fn campaign_config_from_parts(
    parts: CampaignConfigInputParts<'_>,
) -> Result<BranchCampaignConfigV1, String> {
    let player_class = canonical_player_class(parts.player_class)?;
    let mut prefix_commands = Vec::new();
    if !parts.no_neow_guidance {
        prefix_commands.extend(neow_guided_prefix_commands_v1(&NeowGuidedPrefixConfigV1 {
            seed: parts.seed,
            ascension_level: parts.ascension,
            final_act: parts.final_act,
            player_class,
            search_max_nodes: parts.search_max_nodes,
            search_wall_ms: Some(parts.search_wall_ms),
        })?);
    } else {
        prefix_commands.push("0".to_string());
    }
    prefix_commands.extend(parts.prefix_commands.iter().cloned());

    let search_max_hp_loss =
        parse_hp_loss_limit(parts.max_hp_loss)?.or(Some(RunControlHpLossLimit::Unlimited));

    Ok(BranchCampaignConfigV1 {
        seed: parts.seed,
        ascension_level: parts.ascension,
        player_class,
        final_act: parts.final_act,
        max_rounds: parts.rounds.unwrap_or(parts.max_rounds),
        round_depth: parts.round_depth,
        max_active: parts.max_active,
        max_frozen: parts.max_frozen,
        max_branches_per_active: parts.max_branches_per_active,
        boss_relic_axis_isolation: parts.boss_relic_axes,
        retention_budget_profile: parts
            .retention_profile
            .parse::<BranchRetentionBudgetProfileV1>()?,
        max_reward_options_per_branch: if parts.all_reward_options {
            None
        } else {
            Some(parts.max_reward_options.unwrap_or(2))
        },
        max_campfire_options_per_branch: parts.max_campfire_options,
        auto_max_operations: parts.auto_max_ops,
        experiment_wall_ms: Some(parts.experiment_wall_ms),
        search_max_nodes: parts.search_max_nodes,
        search_wall_ms: Some(parts.search_wall_ms),
        search_max_hp_loss,
        search_options: campaign_search_options_from_parts(parts.combat_search_options)?,
        auto_capture: AutoCombatCaptureConfig {
            enabled: parts.auto_capture.enabled,
            root: parts.auto_capture.root,
        },
        combat_retry_policy: match parts.combat_retry {
            BranchCampaignCombatRetryArgV1::OnStall => BranchCampaignCombatRetryPolicyV1::OnStall,
            BranchCampaignCombatRetryArgV1::Immediate => {
                BranchCampaignCombatRetryPolicyV1::Immediate
            }
            BranchCampaignCombatRetryArgV1::Disabled => BranchCampaignCombatRetryPolicyV1::Disabled,
        },
        combat_retry_wall_ms: parts.combat_retry_wall_ms,
        include_event_reward_skip: false,
        min_acceptable_victory_hp_percent: parts.min_acceptable_victory_hp_percent,
        prefix_commands,
    })
}

pub(super) fn campaign_search_options_from_args(
    args: &Args,
) -> Result<RunControlSearchCombatOptions, String> {
    campaign_search_options_from_parts(&args.combat_search_options)
}

fn campaign_search_options_from_parts(
    combat_search_options: &[String],
) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = parse_branch_experiment_search_options_v1(combat_search_options)?;
    if !combat_search_options_include_segment_mode(combat_search_options) {
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
    use crate::cli_args::{parse_cli_from, Args, BranchCampaignCliInputV1};

    #[test]
    fn coverage_gap_execution_input_parses_budget_intent() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "continue",
            "--execute-coverage-gap-continuation",
            "--coverage-gap-budget-intent",
            "frontier-expansion",
        ])
        .expect("coverage gap budget intent should parse");

        let input = CoverageGapExecutionCommandInput::from_args(&args).expect("input should build");

        assert_eq!(
            input.coverage_gap_budget_intent,
            CoverageGapBudgetIntentV1::FrontierExpansion
        );
    }

    #[test]
    fn coverage_gap_execution_input_parses_execution_mode() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "continue",
            "--execute-coverage-gap-continuation",
            "--coverage-gap-execution-mode",
            "target-only",
        ])
        .expect("coverage gap execution mode should parse");

        let input = CoverageGapExecutionCommandInput::from_args(&args).expect("input should build");

        assert_eq!(
            input.coverage_gap_execution_mode,
            CoverageGapExecutionModeV1::TargetOnly
        );
    }

    #[test]
    fn coverage_gap_plan_input_parses_filter() {
        let cli_input = parse_cli_from([
            "branch_campaign_driver",
            "campaign",
            "coverage",
            "plan",
            "--coverage-gap-bucket",
            "event",
            "--coverage-gap-event-id",
            "TheLibrary",
            "--coverage-gap-lane",
            "effect:event_card_reward",
            "--coverage-gap-origin-source",
            "event_boundary_packet",
            "--coverage-gap-progress",
            "missing",
            "--coverage-gap-budget-intent",
            "frontier-expansion",
        ])
        .expect("coverage gap filter should parse");

        let BranchCampaignCliInputV1::CampaignCoveragePlan(args) = cli_input else {
            panic!("coverage plan should use the direct campaign input path");
        };
        let input =
            CoverageGapPlanCommandInput::from_coverage_plan_args(args).expect("input should build");

        assert_eq!(input.coverage_gap_filter.bucket.as_deref(), Some("event"));
        assert_eq!(
            input.coverage_gap_filter.event_id.as_deref(),
            Some("TheLibrary")
        );
        assert_eq!(
            input.coverage_gap_filter.lane.as_deref(),
            Some("effect:event_card_reward")
        );
        assert_eq!(
            input.coverage_gap_filter.origin_source.as_deref(),
            Some("event_boundary_packet")
        );
        assert_eq!(
            input.coverage_gap_filter.progress.as_deref(),
            Some("missing")
        );
        assert_eq!(
            input.coverage_gap_budget_intent,
            CoverageGapBudgetIntentV1::FrontierExpansion
        );
    }

    #[test]
    fn coverage_gap_execution_input_parses_filter() {
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
            "--coverage-gap-origin-source",
            "event_boundary_packet",
            "--coverage-gap-progress",
            "missing",
        ])
        .expect("coverage gap filter should parse");

        let input = CoverageGapExecutionCommandInput::from_args(&args).expect("input should build");

        assert_eq!(input.coverage_gap_filter.bucket.as_deref(), Some("event"));
        assert_eq!(
            input.coverage_gap_filter.event_id.as_deref(),
            Some("TheLibrary")
        );
        assert_eq!(
            input.coverage_gap_filter.lane.as_deref(),
            Some("effect:event_card_reward")
        );
        assert_eq!(
            input.coverage_gap_filter.origin_source.as_deref(),
            Some("event_boundary_packet")
        );
        assert_eq!(
            input.coverage_gap_filter.progress.as_deref(),
            Some("missing")
        );
    }
}
