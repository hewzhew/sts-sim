use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{CommandFactory, FromArgMatches, Parser, ValueEnum};
use std::fs;
use std::path::PathBuf;

mod campaign_run;
mod checkpoint_evidence;
mod checkpoint_inspection;
mod combat_lab;
mod driver_command;
mod final_boss_combat;
mod inspect_summary;
mod outcome_dataset;
mod shop_challenge;

use campaign_run::{run_ancestor_replay_self_check, run_campaign_command};
use checkpoint_inspection::{run_checkpoint_inspection, run_final_boss_combat_report_inspection};
use driver_command::{driver_command_from_args, BranchCampaignDriverCommandV1};
use outcome_dataset::{
    run_branch_outcome_dataset_analysis, run_branch_outcome_dataset_export,
    run_continuation_effect_report, run_decision_outcome_dataset_analysis,
    run_decision_outcome_dataset_export, run_learning_dataset_export, run_learning_readiness_probe,
    run_targeted_continuation_execution, run_targeted_continuation_plan,
};
use sts_simulator::eval::branch_campaign::{
    BranchCampaignCheckpointV1, BranchCampaignCombatRetryPolicyV1, BranchCampaignConfigV1,
    BranchCampaignProgressDetailV1, BranchCampaignReportDetailV1, BranchCampaignReportV1,
    BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME, BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
};
use sts_simulator::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use sts_simulator::eval::branch_experiment_search_options::parse_branch_experiment_search_options_v1;
use sts_simulator::eval::neow_guided_prefix::{
    neow_guided_prefix_commands_v1, NeowGuidedPrefixConfigV1,
};
use sts_simulator::eval::run_control::{canonical_player_class, RunControlHpLossLimit};
use sts_simulator::eval::run_control::{
    RunControlCombatSegmentMode, RunControlSearchCombatOptions,
};

const QUICK_PRESET_MAX_ROUNDS: usize = 2;
const QUICK_PRESET_ROUND_DEPTH: usize = 2;
const QUICK_PRESET_MAX_ACTIVE: usize = 2;
const QUICK_PRESET_MAX_FROZEN: usize = 16;
const QUICK_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
const QUICK_PRESET_EXPERIMENT_WALL_MS: u64 = 5_000;
const QUICK_PRESET_SEARCH_WALL_MS: u64 = 300;
const QUICK_PRESET_SEARCH_MAX_NODES: usize = 50_000;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CampaignReportDetailArg {
    Human,
    Diagnose,
    Perf,
}

impl From<CampaignReportDetailArg> for BranchCampaignReportDetailV1 {
    fn from(value: CampaignReportDetailArg) -> Self {
        match value {
            CampaignReportDetailArg::Human => BranchCampaignReportDetailV1::Human,
            CampaignReportDetailArg::Diagnose => BranchCampaignReportDetailV1::Diagnose,
            CampaignReportDetailArg::Perf => BranchCampaignReportDetailV1::Perf,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum CampaignProgressDetailArg {
    Summary,
    Verbose,
}

impl From<CampaignProgressDetailArg> for BranchCampaignProgressDetailV1 {
    fn from(value: CampaignProgressDetailArg) -> Self {
        match value {
            CampaignProgressDetailArg::Summary => BranchCampaignProgressDetailV1::Summary,
            CampaignProgressDetailArg::Verbose => BranchCampaignProgressDetailV1::Verbose,
        }
    }
}
const QUICK_PRESET_BRANCH_EXAMPLES: usize = 3;

const FOCUSED_PRESET_MAX_ROUNDS: usize = 6;
const FOCUSED_PRESET_ROUND_DEPTH: usize = 2;
const FOCUSED_PRESET_MAX_ACTIVE: usize = 2;
const FOCUSED_PRESET_MAX_FROZEN: usize = 16;
const FOCUSED_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
const FOCUSED_PRESET_ACTIVE_LINEAGE_DIVERSITY: usize = 2;
const FOCUSED_PRESET_EXPERIMENT_WALL_MS: u64 = 10_000;
const FOCUSED_PRESET_SEARCH_WALL_MS: u64 = 300;
const FOCUSED_PRESET_SEARCH_MAX_NODES: usize = 50_000;
const FOCUSED_PRESET_BRANCH_EXAMPLES: usize = 4;

const DEEP_PRESET_MAX_ROUNDS: usize = 10;
const DEEP_PRESET_ROUND_DEPTH: usize = 2;
const DEEP_PRESET_MAX_ACTIVE: usize = 2;
const DEEP_PRESET_MAX_FROZEN: usize = 16;
const DEEP_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
const DEEP_PRESET_ACTIVE_LINEAGE_DIVERSITY: usize = 2;
const DEEP_PRESET_EXPERIMENT_WALL_MS: u64 = 30_000;
const DEEP_PRESET_SEARCH_WALL_MS: u64 = 1_000;
const DEEP_PRESET_SEARCH_MAX_NODES: usize = 200_000;
const DEEP_PRESET_BRANCH_EXAMPLES: usize = 6;

const EXPLORE_PRESET_MAX_ROUNDS: usize = 4;
const EXPLORE_PRESET_ROUND_DEPTH: usize = 1;
const EXPLORE_PRESET_MAX_ACTIVE: usize = 6;
const EXPLORE_PRESET_MAX_FROZEN: usize = 48;
const EXPLORE_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 6;
const EXPLORE_PRESET_ACTIVE_LINEAGE_DIVERSITY: usize = 4;
const EXPLORE_PRESET_EXPERIMENT_WALL_MS: u64 = 8_000;
const EXPLORE_PRESET_SEARCH_WALL_MS: u64 = 200;
const EXPLORE_PRESET_SEARCH_MAX_NODES: usize = 30_000;
const EXPLORE_PRESET_BRANCH_EXAMPLES: usize = 8;

#[derive(Debug, Parser)]
#[command(
    name = "branch_campaign_driver",
    about = "Advance a small campaign of noncombat branches until victory, budget, or strategy boundary"
)]
struct Args {
    #[arg(long, value_enum)]
    preset: Option<BranchCampaignPresetV1>,

    #[arg(long, default_value_t = 1)]
    seed: u64,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(
        long,
        value_enum,
        help = "Set ascension from a named curriculum/target domain"
    )]
    ascension_domain: Option<BranchCampaignAscensionDomainArgV1>,

    #[arg(long = "class", default_value = "ironclad")]
    player_class: String,

    #[arg(long)]
    final_act: bool,

    #[arg(long, default_value_t = 8)]
    max_rounds: usize,

    #[arg(long, default_value_t = 1)]
    round_depth: usize,

    #[arg(long, default_value_t = 8)]
    max_active: usize,

    #[arg(long, default_value_t = 32)]
    max_frozen: usize,

    #[arg(long, default_value_t = 12)]
    max_branches_per_active: usize,

    #[arg(
        long,
        default_value_t = 0,
        help = "Reserve active slots for distinct first-choice branch lineages; intended for exploration presets"
    )]
    active_lineage_diversity: usize,

    #[arg(long, default_value = "package")]
    retention_profile: String,

    #[arg(long)]
    max_reward_options: Option<usize>,

    #[arg(long)]
    all_reward_options: bool,

    #[arg(long, default_value_t = 3)]
    max_campfire_options: usize,

    #[arg(long, default_value_t = 128)]
    auto_max_ops: usize,

    #[arg(long, default_value_t = 10_000)]
    experiment_wall_ms: u64,

    #[arg(long)]
    search_max_nodes: Option<usize>,

    #[arg(long, default_value_t = 200)]
    search_wall_ms: u64,

    #[arg(long)]
    max_hp_loss: Option<String>,

    #[arg(
        long = "combat-search-option",
        value_name = "KEY=VALUE",
        help = "Additional run_control search-combat option forwarded to branch experiments"
    )]
    combat_search_options: Vec<String>,

    #[arg(long, value_enum, default_value_t = BranchCampaignCombatRetryArgV1::OnStall)]
    combat_retry: BranchCampaignCombatRetryArgV1,

    #[arg(
        long,
        help = "Override the wall-clock budget used by the one-shot combat retry pass"
    )]
    combat_retry_wall_ms: Option<u64>,

    #[arg(long, default_value_t = 20)]
    min_acceptable_victory_hp_percent: u8,

    #[arg(long = "prefix", value_name = "COMMAND")]
    prefix_commands: Vec<String>,

    #[arg(long)]
    no_neow_guidance: bool,

    #[arg(long, default_value_t = 4)]
    branch_examples: usize,

    #[arg(long)]
    json: bool,

    #[arg(long, value_enum, default_value_t = CampaignReportDetailArg::Human)]
    report_detail: CampaignReportDetailArg,

    #[arg(long, help = "Print coarse campaign progress to stderr while running")]
    progress: bool,

    #[arg(long, value_enum, default_value_t = CampaignProgressDetailArg::Summary)]
    progress_detail: CampaignProgressDetailArg,

    #[arg(
        long = "self-check-ancestor-replay",
        help = "Run a deterministic replay-cache self-check and print machine-readable counters"
    )]
    self_check_ancestor_replay: bool,

    #[arg(
        long,
        value_name = "PATH",
        help = "Resume from a previous BranchCampaignV1 JSON report"
    )]
    resume: Option<PathBuf>,

    #[arg(
        long = "resume-checkpoint",
        value_name = "PATH",
        help = "Resume exact branch sessions from a BranchCampaignCheckpointV2 sidecar"
    )]
    resume_checkpoint: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Write the resulting BranchCampaignV1 JSON report"
    )]
    out: Option<PathBuf>,

    #[arg(
        long = "checkpoint-out",
        value_name = "PATH",
        help = "Write the resulting BranchCampaignCheckpointV2 exact session sidecar"
    )]
    checkpoint_out: Option<PathBuf>,

    #[arg(
        long = "inspect-checkpoint",
        value_name = "PATH",
        help = "Inspect a saved BranchCampaignCheckpointV2 session instead of running a campaign"
    )]
    inspect_checkpoint: Option<PathBuf>,

    #[arg(
        long = "inspect-report",
        value_name = "PATH",
        help = "Pair --inspect-checkpoint with a BranchCampaignV1 report for active/frozen/abandoned labels"
    )]
    inspect_report: Option<PathBuf>,

    #[arg(
        long = "inspect-summary",
        help = "Print compact deck/resource/strategy summaries for checkpoint sessions"
    )]
    inspect_summary: bool,

    #[arg(
        long = "inspect-act",
        help = "Filter inspected checkpoint sessions by act"
    )]
    inspect_act: Option<u8>,

    #[arg(
        long = "inspect-floor",
        help = "Filter inspected checkpoint sessions by floor"
    )]
    inspect_floor: Option<i32>,

    #[arg(
        long = "inspect-boundary",
        help = "Filter inspected checkpoint sessions by visible boundary title, e.g. Shop or Card Reward"
    )]
    inspect_boundary: Option<String>,

    #[arg(
        long = "inspect-hp",
        help = "Filter inspected checkpoint sessions by current HP"
    )]
    inspect_hp: Option<i32>,

    #[arg(
        long = "inspect-index",
        help = "Select the Nth matching checkpoint session after filters"
    )]
    inspect_index: Option<usize>,

    #[arg(
        long = "inspect-search",
        help = "Run search-combat from the selected checkpoint session and print the result"
    )]
    inspect_search: bool,

    #[arg(
        long = "inspect-last-auto-combat",
        help = "Print the last stored automated combat trajectory for the selected checkpoint session"
    )]
    inspect_last_auto_combat: bool,

    #[arg(
        long = "inspect-combat-lab",
        help = "Print a report-only CombatLabProbeV1 packet for the selected checkpoint branch"
    )]
    inspect_combat_lab: bool,

    #[arg(
        long = "probe-boss",
        help = "When used with --inspect-combat-lab, run a report-only search preview against the current act boss"
    )]
    probe_boss: bool,

    #[arg(
        long = "inspect-shop-evidence",
        help = "Print current-code shop candidate evidence and strategic deltas for the selected checkpoint session"
    )]
    inspect_shop_evidence: bool,

    #[arg(
        long = "challenge-shop-plans",
        help = "From a selected shop checkpoint, force compiled shop plans and rollout each branch for comparison"
    )]
    challenge_shop_plans: bool,

    #[arg(
        long = "challenge-max-plans",
        default_value_t = 6,
        help = "Maximum selected+alternative shop plans to challenge"
    )]
    challenge_max_plans: usize,

    #[arg(
        long = "challenge-depth",
        default_value_t = 4,
        help = "Branch experiment depth after each challenged shop plan"
    )]
    challenge_depth: usize,

    #[arg(
        long = "challenge-max-branches",
        default_value_t = 12,
        help = "Branch cap for each challenged shop plan rollout"
    )]
    challenge_max_branches: usize,

    #[arg(
        long = "inspect-card-reward-evidence",
        help = "Print current-code card reward candidate evidence and strategic deltas for the selected checkpoint session"
    )]
    inspect_card_reward_evidence: bool,

    #[arg(
        long = "inspect-campfire-evidence",
        help = "Print current-code campfire candidate evidence and selected plan for the selected checkpoint session"
    )]
    inspect_campfire_evidence: bool,

    #[arg(
        long = "inspect-deck-mutation",
        help = "Print current-code DeckMutationCompiler plan groups for the selected checkpoint session"
    )]
    inspect_deck_mutation: bool,

    #[arg(
        long = "inspect-route-evidence",
        help = "Print current-code route planner candidate evidence for the selected map checkpoint session"
    )]
    inspect_route_evidence: bool,

    #[arg(
        long = "inspect-final-boss-combat",
        help = "Print a final boss combat timeline from a BranchCampaignV1 report"
    )]
    inspect_final_boss_combat: bool,

    #[arg(
        long = "export-outcome-dataset",
        value_name = "PATH",
        help = "Write BranchOutcomeRecordV1 JSONL from a campaign report and optional checkpoint sidecar"
    )]
    export_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "analyze-outcome-dataset",
        value_name = "PATH",
        help = "Print structural issue counts from a BranchOutcomeRecordV1 JSONL file"
    )]
    analyze_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "analyze-decision-outcome-dataset",
        value_name = "PATH",
        help = "Print sibling decision group coverage and outcome divergence from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    analyze_decision_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "probe-learning-readiness",
        value_name = "PATH",
        help = "Diagnose whether a LearningDecisionOutcomeSampleV1 JSONL is blocked by censoring, scheduling, combat budget, missing context, or missing siblings"
    )]
    probe_learning_readiness: Option<PathBuf>,

    #[arg(
        long = "plan-targeted-continuation",
        value_name = "PATH",
        help = "Print targeted sibling continuation groups from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    plan_targeted_continuation: Option<PathBuf>,

    #[arg(
        long = "execute-targeted-continuation",
        value_name = "PATH",
        help = "Resume selected censored sibling branches from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    execute_targeted_continuation: Option<PathBuf>,

    #[arg(
        long = "continuation-effect-before",
        value_name = "PATH",
        help = "Before LearningDecisionOutcomeSampleV1 JSONL for targeted continuation effect comparison"
    )]
    continuation_effect_before: Option<PathBuf>,

    #[arg(
        long = "continuation-effect-after",
        value_name = "PATH",
        help = "After LearningDecisionOutcomeSampleV1 JSONL for targeted continuation effect comparison"
    )]
    continuation_effect_after: Option<PathBuf>,

    #[arg(
        long = "targeted-continuation-limit",
        default_value_t = 4,
        help = "Maximum targeted sibling groups to continue"
    )]
    targeted_continuation_limit: usize,

    #[arg(
        long = "targeted-continuation-candidates-per-target",
        default_value_t = 1,
        help = "Maximum censored candidate branches to continue per targeted sibling group"
    )]
    targeted_continuation_candidates_per_target: usize,

    #[arg(
        long = "export-learning-dataset",
        value_name = "PATH",
        help = "Write LearningBranchSampleV1 JSONL from a campaign report/run without treating choices as teacher labels"
    )]
    export_learning_dataset: Option<PathBuf>,

    #[arg(
        long = "export-decision-outcome-dataset",
        value_name = "PATH",
        help = "Write LearningDecisionOutcomeSampleV1 JSONL with observed sibling candidates and later outcomes"
    )]
    export_decision_outcome_dataset: Option<PathBuf>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum BranchCampaignPresetV1 {
    Quick,
    Focused,
    Explore,
    Deep,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum BranchCampaignAscensionDomainArgV1 {
    A0,
    A10,
    A15,
    A17,
    A20,
}

impl BranchCampaignAscensionDomainArgV1 {
    fn ascension_level(self) -> u8 {
        match self {
            Self::A0 => 0,
            Self::A10 => 10,
            Self::A15 => 15,
            Self::A17 => 17,
            Self::A20 => 20,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum BranchCampaignCombatRetryArgV1 {
    OnStall,
    Immediate,
    Disabled,
}

fn main() {
    let args = parse_args();
    if let Err(err) = run(args) {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn parse_args() -> Args {
    parse_args_from(std::env::args_os()).unwrap_or_else(|err| err.exit())
}

fn parse_args_from<I, T>(itr: I) -> Result<Args, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let matches = Args::command().try_get_matches_from(itr)?;
    let mut args = Args::from_arg_matches(&matches)?;
    if let Some(domain) = args.ascension_domain {
        let domain_ascension = domain.ascension_level();
        let ascension_was_explicit =
            matches.value_source("ascension") == Some(ValueSource::CommandLine);
        if ascension_was_explicit && args.ascension != domain_ascension {
            return Err(clap::Error::raw(
                ErrorKind::ValueValidation,
                format!(
                    "--ascension-domain {:?} implies --ascension {}, but --ascension {} was provided",
                    domain, domain_ascension, args.ascension
                ),
            ));
        }
        if !ascension_was_explicit {
            args.ascension = domain_ascension;
        }
    }
    apply_preset_defaults(&mut args, |name| {
        matches.value_source(name) == Some(ValueSource::CommandLine)
    });
    Ok(args)
}

fn apply_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    match args.preset {
        Some(BranchCampaignPresetV1::Quick) => apply_quick_preset_defaults(args, was_explicit),
        Some(BranchCampaignPresetV1::Focused) => apply_focused_preset_defaults(args, was_explicit),
        Some(BranchCampaignPresetV1::Explore) => apply_explore_preset_defaults(args, was_explicit),
        Some(BranchCampaignPresetV1::Deep) => apply_deep_preset_defaults(args, was_explicit),
        None => {}
    }
}

fn apply_quick_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    apply_campaign_preset_defaults(
        args,
        was_explicit,
        CampaignPresetDefaults {
            max_rounds: QUICK_PRESET_MAX_ROUNDS,
            round_depth: QUICK_PRESET_ROUND_DEPTH,
            max_active: QUICK_PRESET_MAX_ACTIVE,
            max_frozen: QUICK_PRESET_MAX_FROZEN,
            max_branches_per_active: QUICK_PRESET_MAX_BRANCHES_PER_ACTIVE,
            active_lineage_diversity: 0,
            experiment_wall_ms: QUICK_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: QUICK_PRESET_SEARCH_WALL_MS,
            search_max_nodes: QUICK_PRESET_SEARCH_MAX_NODES,
            branch_examples: QUICK_PRESET_BRANCH_EXAMPLES,
        },
    );
}

fn apply_focused_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    apply_campaign_preset_defaults(
        args,
        was_explicit,
        CampaignPresetDefaults {
            max_rounds: FOCUSED_PRESET_MAX_ROUNDS,
            round_depth: FOCUSED_PRESET_ROUND_DEPTH,
            max_active: FOCUSED_PRESET_MAX_ACTIVE,
            max_frozen: FOCUSED_PRESET_MAX_FROZEN,
            max_branches_per_active: FOCUSED_PRESET_MAX_BRANCHES_PER_ACTIVE,
            active_lineage_diversity: FOCUSED_PRESET_ACTIVE_LINEAGE_DIVERSITY,
            experiment_wall_ms: FOCUSED_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: FOCUSED_PRESET_SEARCH_WALL_MS,
            search_max_nodes: FOCUSED_PRESET_SEARCH_MAX_NODES,
            branch_examples: FOCUSED_PRESET_BRANCH_EXAMPLES,
        },
    );
}

fn apply_deep_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    apply_campaign_preset_defaults(
        args,
        was_explicit,
        CampaignPresetDefaults {
            max_rounds: DEEP_PRESET_MAX_ROUNDS,
            round_depth: DEEP_PRESET_ROUND_DEPTH,
            max_active: DEEP_PRESET_MAX_ACTIVE,
            max_frozen: DEEP_PRESET_MAX_FROZEN,
            max_branches_per_active: DEEP_PRESET_MAX_BRANCHES_PER_ACTIVE,
            active_lineage_diversity: DEEP_PRESET_ACTIVE_LINEAGE_DIVERSITY,
            experiment_wall_ms: DEEP_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: DEEP_PRESET_SEARCH_WALL_MS,
            search_max_nodes: DEEP_PRESET_SEARCH_MAX_NODES,
            branch_examples: DEEP_PRESET_BRANCH_EXAMPLES,
        },
    );
}

fn apply_explore_preset_defaults<F>(args: &mut Args, was_explicit: F)
where
    F: Fn(&'static str) -> bool,
{
    apply_campaign_preset_defaults(
        args,
        was_explicit,
        CampaignPresetDefaults {
            max_rounds: EXPLORE_PRESET_MAX_ROUNDS,
            round_depth: EXPLORE_PRESET_ROUND_DEPTH,
            max_active: EXPLORE_PRESET_MAX_ACTIVE,
            max_frozen: EXPLORE_PRESET_MAX_FROZEN,
            max_branches_per_active: EXPLORE_PRESET_MAX_BRANCHES_PER_ACTIVE,
            active_lineage_diversity: EXPLORE_PRESET_ACTIVE_LINEAGE_DIVERSITY,
            experiment_wall_ms: EXPLORE_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: EXPLORE_PRESET_SEARCH_WALL_MS,
            search_max_nodes: EXPLORE_PRESET_SEARCH_MAX_NODES,
            branch_examples: EXPLORE_PRESET_BRANCH_EXAMPLES,
        },
    );
}

#[derive(Clone, Copy, Debug)]
struct CampaignPresetDefaults {
    max_rounds: usize,
    round_depth: usize,
    max_active: usize,
    max_frozen: usize,
    max_branches_per_active: usize,
    active_lineage_diversity: usize,
    experiment_wall_ms: u64,
    search_wall_ms: u64,
    search_max_nodes: usize,
    branch_examples: usize,
}

fn apply_campaign_preset_defaults<F>(
    args: &mut Args,
    was_explicit: F,
    defaults: CampaignPresetDefaults,
) where
    F: Fn(&'static str) -> bool,
{
    if !was_explicit("max_rounds") {
        args.max_rounds = defaults.max_rounds;
    }
    if !was_explicit("round_depth") {
        args.round_depth = defaults.round_depth;
    }
    if !was_explicit("max_active") {
        args.max_active = defaults.max_active;
    }
    if !was_explicit("max_frozen") {
        args.max_frozen = defaults.max_frozen;
    }
    if !was_explicit("max_branches_per_active") {
        args.max_branches_per_active = defaults.max_branches_per_active;
    }
    if !was_explicit("active_lineage_diversity") {
        args.active_lineage_diversity = defaults.active_lineage_diversity;
    }
    if !was_explicit("experiment_wall_ms") {
        args.experiment_wall_ms = defaults.experiment_wall_ms;
    }
    if !was_explicit("search_wall_ms") {
        args.search_wall_ms = defaults.search_wall_ms;
    }
    if !was_explicit("search_max_nodes") {
        args.search_max_nodes = Some(defaults.search_max_nodes);
    }
    if !was_explicit("branch_examples") {
        args.branch_examples = defaults.branch_examples;
    }
}

fn run(args: Args) -> Result<(), String> {
    match driver_command_from_args(&args) {
        BranchCampaignDriverCommandV1::SelfCheckAncestorReplay => run_ancestor_replay_self_check(),
        BranchCampaignDriverCommandV1::AnalyzeOutcomeDataset => {
            run_branch_outcome_dataset_analysis(&args)
        }
        BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset => {
            run_decision_outcome_dataset_analysis(&args)
        }
        BranchCampaignDriverCommandV1::ProbeLearningReadiness => {
            run_learning_readiness_probe(&args)
        }
        BranchCampaignDriverCommandV1::PlanTargetedContinuation => {
            run_targeted_continuation_plan(&args)
        }
        BranchCampaignDriverCommandV1::ExecuteTargetedContinuation => {
            run_targeted_continuation_execution(&args)
        }
        BranchCampaignDriverCommandV1::ContinuationEffectReport => {
            run_continuation_effect_report(&args)
        }
        BranchCampaignDriverCommandV1::ExportOutcomeDataset => {
            run_branch_outcome_dataset_export(&args)
        }
        BranchCampaignDriverCommandV1::ExportLearningDataset => run_learning_dataset_export(&args),
        BranchCampaignDriverCommandV1::ExportDecisionOutcomeDataset => {
            run_decision_outcome_dataset_export(&args)
        }
        BranchCampaignDriverCommandV1::InspectFinalBossCombat => {
            run_final_boss_combat_report_inspection(&args)
        }
        BranchCampaignDriverCommandV1::InspectCheckpoint => run_checkpoint_inspection(&args),
        BranchCampaignDriverCommandV1::RunCampaign => run_campaign_command(&args),
    }
}

fn read_campaign_report_v1(path: &PathBuf) -> Result<BranchCampaignReportV1, String> {
    let text = fs::read_to_string(path)
        .map_err(|err| format!("failed to read --resume {}: {err}", path.display()))?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume {} as BranchCampaignV1: {err}",
            path.display()
        )
    })
}

fn read_campaign_checkpoint_v1(path: &PathBuf) -> Result<BranchCampaignCheckpointV1, String> {
    let text = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read --resume-checkpoint {}: {err}",
            path.display()
        )
    })?;
    let checkpoint: BranchCampaignCheckpointV1 = serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse --resume-checkpoint {} as {BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME}: {err}",
            path.display()
        )
    })?;
    if checkpoint.schema_name != BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME
        || checkpoint.schema_version != BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION
    {
        return Err(format!(
            "checkpoint {} uses {} v{}; expected {} v{}. Rerun campaign to create a fresh checkpoint.",
            path.display(),
            checkpoint.schema_name,
            checkpoint.schema_version,
            BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME,
            BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION
        ));
    }
    Ok(checkpoint)
}

fn write_campaign_report_v1(path: &PathBuf, report: &BranchCampaignReportV1) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --out directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(report)
        .map_err(|err| format!("failed to serialize BranchCampaignV1 report: {err}"))?;
    fs::write(path, text).map_err(|err| format!("failed to write --out {}: {err}", path.display()))
}

fn write_campaign_checkpoint_v1(
    path: &PathBuf,
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create --checkpoint-out directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(checkpoint)
        .map_err(|err| format!("failed to serialize BranchCampaignCheckpointV2: {err}"))?;
    fs::write(path, text)
        .map_err(|err| format!("failed to write --checkpoint-out {}: {err}", path.display()))
}

fn campaign_config_from_args(args: &Args) -> Result<BranchCampaignConfigV1, String> {
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
        max_rounds: args.max_rounds,
        round_depth: args.round_depth,
        max_active: args.max_active,
        max_frozen: args.max_frozen,
        max_branches_per_active: args.max_branches_per_active,
        active_lineage_diversity_slots: args.active_lineage_diversity,
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

fn campaign_search_options_from_args(args: &Args) -> Result<RunControlSearchCombatOptions, String> {
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

    #[test]
    fn campaign_cli_defaults_to_bounded_reward_branching() {
        let args = Args::try_parse_from(["branch_campaign_driver"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_reward_options_per_branch, Some(2));
        assert_eq!(
            config.search_options.segment_mode,
            Some(RunControlCombatSegmentMode::NonBossTurnBoundary)
        );
        assert_eq!(config.max_active, 8);
        assert_eq!(config.max_frozen, 32);
        assert_eq!(config.round_depth, 1);
        assert_eq!(config.active_lineage_diversity_slots, 0);
    }

    #[test]
    fn campaign_cli_can_disable_segment_combat_fallback() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--combat-search-option",
            "segment=off",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.search_options.segment_mode, None);
    }

    #[test]
    fn campaign_cli_accepts_resume_and_out_paths() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--resume",
            "old.campaign.json",
            "--resume-checkpoint",
            "old.checkpoint.json",
            "--out",
            "new.campaign.json",
            "--checkpoint-out",
            "new.checkpoint.json",
        ])
        .expect("args parse");

        assert_eq!(args.resume, Some(PathBuf::from("old.campaign.json")));
        assert_eq!(
            args.resume_checkpoint,
            Some(PathBuf::from("old.checkpoint.json"))
        );
        assert_eq!(args.out, Some(PathBuf::from("new.campaign.json")));
        assert_eq!(
            args.checkpoint_out,
            Some(PathBuf::from("new.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_ancestor_replay_self_check() {
        let args = parse_args_from(["branch_campaign_driver", "--self-check-ancestor-replay"])
            .expect("args parse");

        assert!(args.self_check_ancestor_replay);
    }

    #[test]
    fn driver_command_defaults_to_campaign_run() {
        let args = Args::try_parse_from(["branch_campaign_driver"]).expect("args parse");

        assert_eq!(
            driver_command_from_args(&args),
            BranchCampaignDriverCommandV1::RunCampaign
        );
    }

    #[test]
    fn driver_command_classifies_checkpoint_inspection() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
        ])
        .expect("args parse");

        assert_eq!(
            driver_command_from_args(&args),
            BranchCampaignDriverCommandV1::InspectCheckpoint
        );
    }

    #[test]
    fn driver_command_classifies_learning_dataset_modes() {
        let analyze_args = Args::try_parse_from([
            "branch_campaign_driver",
            "--analyze-decision-outcome-dataset",
            "decision_outcomes.jsonl",
        ])
        .expect("args parse");
        let continue_args = Args::try_parse_from([
            "branch_campaign_driver",
            "--execute-targeted-continuation",
            "decision_outcomes.jsonl",
        ])
        .expect("args parse");

        assert_eq!(
            driver_command_from_args(&analyze_args),
            BranchCampaignDriverCommandV1::AnalyzeDecisionOutcomeDataset
        );
        assert_eq!(
            driver_command_from_args(&continue_args),
            BranchCampaignDriverCommandV1::ExecuteTargetedContinuation
        );
    }

    #[test]
    fn driver_command_keeps_legacy_self_check_precedence() {
        let args = Args::try_parse_from([
            "branch_campaign_driver",
            "--self-check-ancestor-replay",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
        ])
        .expect("args parse");

        assert_eq!(
            driver_command_from_args(&args),
            BranchCampaignDriverCommandV1::SelfCheckAncestorReplay
        );
    }

    #[test]
    fn campaign_checkpoint_reader_rejects_v1_schema() {
        let path = std::env::temp_dir().join(format!(
            "old-branch-campaign-checkpoint-{}.json",
            std::process::id()
        ));
        fs::write(
            &path,
            r#"{
  "schema_name": "BranchCampaignCheckpointV1",
  "schema_version": 1,
  "seed": 1,
  "rounds_completed": 0,
  "sessions": []
}"#,
        )
        .expect("write old checkpoint fixture");

        let err =
            read_campaign_checkpoint_v1(&path).expect_err("old checkpoint should be rejected");
        let _ = fs::remove_file(&path);

        assert!(err.contains("BranchCampaignCheckpointV2"));
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_summary_inspection_paths() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-summary",
            "--branch-examples",
            "2",
        ])
        .expect("args parse");

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_summary);
        assert_eq!(args.branch_examples, 2);
        assert_eq!(args.inspect_index, None);
    }

    #[test]
    fn campaign_cli_accepts_optional_checkpoint_inspect_index() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-summary",
            "--inspect-index",
            "18",
        ])
        .expect("args parse");

        assert_eq!(args.inspect_index, Some(18));
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_shop_evidence_inspection() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-act",
            "2",
            "--inspect-floor",
            "18",
            "--inspect-shop-evidence",
        ])
        .expect("args parse");

        assert_eq!(args.inspect_act, Some(2));
        assert_eq!(args.inspect_floor, Some(18));
        assert!(args.inspect_shop_evidence);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_shop_plan_challenge() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-boundary",
            "Shop",
            "--challenge-shop-plans",
            "--challenge-max-plans",
            "5",
            "--challenge-depth",
            "3",
        ])
        .expect("args parse");

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(args.inspect_boundary.as_deref(), Some("Shop"));
        assert!(args.challenge_shop_plans);
        assert_eq!(args.challenge_max_plans, 5);
        assert_eq!(args.challenge_depth, 3);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_card_reward_evidence_inspection() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-act",
            "1",
            "--inspect-floor",
            "11",
            "--inspect-card-reward-evidence",
        ])
        .expect("args parse");

        assert_eq!(args.inspect_act, Some(1));
        assert_eq!(args.inspect_floor, Some(11));
        assert!(args.inspect_card_reward_evidence);
    }

    #[test]
    fn inspect_search_keeps_combat_search_wall_ms_option() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-search",
            "--combat-search-option",
            "wall_ms=5000",
        ])
        .expect("args parse");

        let options =
            checkpoint_inspection::inspect_search_options_from_args(&args).expect("options parse");

        assert_eq!(options.wall_ms, Some(5_000));
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_deck_mutation_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-deck-mutation",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_deck_mutation);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_campfire_evidence_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-campfire-evidence",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_campfire_evidence);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_route_evidence_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-route-evidence",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_route_evidence);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_last_auto_combat_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-last-auto-combat",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert!(args.inspect_last_auto_combat);
    }

    #[test]
    fn campaign_cli_accepts_checkpoint_combat_lab_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-combat-lab",
            "--probe-boss",
            "--combat-search-option",
            "wall_ms=1000",
        ]);

        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
        assert!(args.inspect_combat_lab);
        assert!(args.probe_boss);
        assert_eq!(args.combat_search_options, vec!["wall_ms=1000"]);
    }

    #[test]
    fn campaign_cli_accepts_final_boss_combat_report_inspection() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-final-boss-combat",
        ]);

        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert!(args.inspect_final_boss_combat);
    }

    #[test]
    fn campaign_cli_accepts_branch_outcome_dataset_export() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--export-outcome-dataset",
            "branch_outcomes.jsonl",
        ]);

        assert_eq!(
            args.export_outcome_dataset,
            Some(PathBuf::from("branch_outcomes.jsonl"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_branch_outcome_dataset_analysis() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--analyze-outcome-dataset",
            "branch_outcomes.jsonl",
        ]);

        assert_eq!(
            args.analyze_outcome_dataset,
            Some(PathBuf::from("branch_outcomes.jsonl"))
        );
    }
    #[test]
    fn campaign_cli_accepts_learning_dataset_export() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--export-learning-dataset",
            "learning.jsonl",
        ]);

        assert_eq!(
            args.export_learning_dataset,
            Some(PathBuf::from("learning.jsonl"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_decision_outcome_dataset_export() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--inspect-report",
            "latest.campaign.json",
            "--inspect-checkpoint",
            "latest.checkpoint.json",
            "--export-decision-outcome-dataset",
            "decision_outcomes.jsonl",
        ]);

        assert_eq!(
            args.export_decision_outcome_dataset,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
        assert_eq!(
            args.inspect_report,
            Some(PathBuf::from("latest.campaign.json"))
        );
        assert_eq!(
            args.inspect_checkpoint,
            Some(PathBuf::from("latest.checkpoint.json"))
        );
    }

    #[test]
    fn campaign_cli_accepts_decision_outcome_dataset_analysis() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--analyze-decision-outcome-dataset",
            "decision_outcomes.jsonl",
        ]);

        assert_eq!(
            args.analyze_decision_outcome_dataset,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
    }

    #[test]
    fn campaign_cli_accepts_learning_readiness_probe() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--probe-learning-readiness",
            "decision_outcomes.jsonl",
        ]);

        assert_eq!(
            args.probe_learning_readiness,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
    }

    #[test]
    fn campaign_cli_accepts_targeted_continuation_plan() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--plan-targeted-continuation",
            "decision_outcomes.jsonl",
        ]);

        assert_eq!(
            args.plan_targeted_continuation,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
    }

    #[test]
    fn campaign_cli_accepts_targeted_continuation_execution() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--resume",
            "latest.campaign.json",
            "--resume-checkpoint",
            "latest.checkpoint.json",
            "--execute-targeted-continuation",
            "decision_outcomes.jsonl",
            "--targeted-continuation-limit",
            "3",
            "--targeted-continuation-candidates-per-target",
            "2",
        ]);

        assert_eq!(
            args.execute_targeted_continuation,
            Some(PathBuf::from("decision_outcomes.jsonl"))
        );
        assert_eq!(args.targeted_continuation_limit, 3);
        assert_eq!(args.targeted_continuation_candidates_per_target, 2);
    }

    #[test]
    fn campaign_cli_accepts_continuation_effect_report() {
        let args = Args::parse_from([
            "branch_campaign_driver",
            "--continuation-effect-before",
            "before.jsonl",
            "--continuation-effect-after",
            "after.jsonl",
        ]);

        assert_eq!(
            args.continuation_effect_before,
            Some(PathBuf::from("before.jsonl"))
        );
        assert_eq!(
            args.continuation_effect_after,
            Some(PathBuf::from("after.jsonl"))
        );
    }

    #[test]
    fn campaign_cli_can_branch_all_reward_options() {
        let args = Args::try_parse_from(["branch_campaign_driver", "--all-reward-options"])
            .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_reward_options_per_branch, None);
    }

    #[test]
    fn focused_preset_uses_deeper_fewer_active_branches() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "focused"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 6);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.active_lineage_diversity_slots, 2);
        assert_eq!(config.experiment_wall_ms, Some(10_000));
        assert_eq!(config.search_wall_ms, Some(300));
        assert_eq!(config.search_max_nodes, Some(50_000));
        assert_eq!(args.branch_examples, 4);
    }

    #[test]
    fn quick_preset_uses_short_smoke_budgets() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "quick"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 2);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.active_lineage_diversity_slots, 0);
        assert_eq!(config.experiment_wall_ms, Some(5_000));
        assert_eq!(config.search_wall_ms, Some(300));
        assert_eq!(config.search_max_nodes, Some(50_000));
        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(
            config.combat_retry_policy,
            BranchCampaignCombatRetryPolicyV1::OnStall
        );
        assert_eq!(args.branch_examples, 3);
    }

    #[test]
    fn ascension_domain_sets_curriculum_ascension_when_not_explicit() {
        let args = parse_args_from(["branch_campaign_driver", "--ascension-domain", "a20"])
            .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.ascension_level, 20);
    }

    #[test]
    fn ascension_domain_rejects_conflicting_explicit_ascension() {
        let err = parse_args_from([
            "branch_campaign_driver",
            "--ascension-domain",
            "a20",
            "--ascension",
            "10",
        ])
        .expect_err("conflicting ascension should fail");

        assert_eq!(err.kind(), ErrorKind::ValueValidation);
    }

    #[test]
    fn deep_preset_uses_larger_budgets() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "deep"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 10);
        assert_eq!(config.round_depth, 2);
        assert_eq!(config.max_active, 2);
        assert_eq!(config.max_frozen, 16);
        assert_eq!(config.max_branches_per_active, 8);
        assert_eq!(config.active_lineage_diversity_slots, 2);
        assert_eq!(config.experiment_wall_ms, Some(30_000));
        assert_eq!(config.search_wall_ms, Some(1_000));
        assert_eq!(config.search_max_nodes, Some(200_000));
        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(args.branch_examples, 6);
    }

    #[test]
    fn explore_preset_uses_wider_shallower_branching() {
        let args =
            parse_args_from(["branch_campaign_driver", "--preset", "explore"]).expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.max_rounds, 4);
        assert_eq!(config.round_depth, 1);
        assert_eq!(config.max_active, 6);
        assert_eq!(config.max_frozen, 48);
        assert_eq!(config.max_branches_per_active, 6);
        assert_eq!(config.active_lineage_diversity_slots, 4);
        assert_eq!(config.experiment_wall_ms, Some(8_000));
        assert_eq!(config.search_wall_ms, Some(200));
        assert_eq!(config.search_max_nodes, Some(30_000));
        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Unlimited)
        );
        assert_eq!(args.branch_examples, 8);
    }

    #[test]
    fn campaign_cli_keeps_explicit_hp_loss_limit() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "quick",
            "--max-hp-loss",
            "12",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(
            config.search_max_hp_loss,
            Some(RunControlHpLossLimit::Limit(12))
        );
    }

    #[test]
    fn campaign_cli_can_enable_immediate_combat_retry_for_comparison() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "quick",
            "--combat-retry",
            "immediate",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(
            config.combat_retry_policy,
            BranchCampaignCombatRetryPolicyV1::Immediate
        );
    }

    #[test]
    fn campaign_cli_accepts_explicit_combat_retry_wall_budget() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "quick",
            "--combat-retry-wall-ms",
            "1000",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.combat_retry_wall_ms, Some(1_000));
    }

    #[test]
    fn focused_preset_keeps_explicit_branch_overrides() {
        let args = parse_args_from([
            "branch_campaign_driver",
            "--preset",
            "focused",
            "--round-depth",
            "1",
            "--max-active",
            "4",
            "--max-frozen",
            "8",
            "--max-branches-per-active",
            "12",
        ])
        .expect("args parse");
        let config = campaign_config_from_args(&args).expect("config builds");

        assert_eq!(config.round_depth, 1);
        assert_eq!(config.max_active, 4);
        assert_eq!(config.max_frozen, 8);
        assert_eq!(config.max_branches_per_active, 12);
    }
}
