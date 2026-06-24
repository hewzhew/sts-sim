use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{
    ArgMatches, Args as ClapArgs, Command, CommandFactory, FromArgMatches, Parser, Subcommand,
    ValueEnum,
};
use std::ffi::OsString;
use std::path::PathBuf;

use sts_simulator::eval::branch_campaign::{
    BranchCampaignProgressDetailV1, BranchCampaignReportDetailV1,
};

pub(super) const QUICK_PRESET_MAX_ROUNDS: usize = 2;
pub(super) const QUICK_PRESET_ROUND_DEPTH: usize = 2;
pub(super) const QUICK_PRESET_MAX_ACTIVE: usize = 2;
pub(super) const QUICK_PRESET_MAX_FROZEN: usize = 16;
pub(super) const QUICK_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
pub(super) const QUICK_PRESET_EXPERIMENT_WALL_MS: u64 = 5_000;
pub(super) const QUICK_PRESET_SEARCH_WALL_MS: u64 = 300;
pub(super) const QUICK_PRESET_SEARCH_MAX_NODES: usize = 50_000;
pub(super) const QUICK_PRESET_BRANCH_EXAMPLES: usize = 3;

const FOCUSED_PRESET_MAX_ROUNDS: usize = 6;
const FOCUSED_PRESET_ROUND_DEPTH: usize = 2;
const FOCUSED_PRESET_MAX_ACTIVE: usize = 2;
const FOCUSED_PRESET_MAX_FROZEN: usize = 16;
const FOCUSED_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
const FOCUSED_PRESET_EXPERIMENT_WALL_MS: u64 = 10_000;
const FOCUSED_PRESET_SEARCH_WALL_MS: u64 = 300;
const FOCUSED_PRESET_SEARCH_MAX_NODES: usize = 50_000;
const FOCUSED_PRESET_BRANCH_EXAMPLES: usize = 4;

const DEEP_PRESET_MAX_ROUNDS: usize = 10;
const DEEP_PRESET_ROUND_DEPTH: usize = 2;
const DEEP_PRESET_MAX_ACTIVE: usize = 6;
const DEEP_PRESET_MAX_FROZEN: usize = 48;
const DEEP_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 8;
const DEEP_PRESET_EXPERIMENT_WALL_MS: u64 = 30_000;
const DEEP_PRESET_SEARCH_WALL_MS: u64 = 1_000;
const DEEP_PRESET_SEARCH_MAX_NODES: usize = 200_000;
const DEEP_PRESET_BRANCH_EXAMPLES: usize = 6;

const EXPLORE_PRESET_MAX_ROUNDS: usize = 4;
const EXPLORE_PRESET_ROUND_DEPTH: usize = 1;
const EXPLORE_PRESET_MAX_ACTIVE: usize = 8;
const EXPLORE_PRESET_MAX_FROZEN: usize = 64;
const EXPLORE_PRESET_MAX_BRANCHES_PER_ACTIVE: usize = 6;
const EXPLORE_PRESET_EXPERIMENT_WALL_MS: u64 = 8_000;
const EXPLORE_PRESET_SEARCH_WALL_MS: u64 = 200;
const EXPLORE_PRESET_SEARCH_MAX_NODES: usize = 30_000;
const EXPLORE_PRESET_BRANCH_EXAMPLES: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum CampaignReportDetailArg {
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
pub(super) enum CampaignProgressDetailArg {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum InspectEvidenceDetailArg {
    Compact,
    Full,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum ArtifactKindArgV1 {
    Run,
    Scratch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ArtifactActionV1 {
    Resolve,
    SourceInfo,
    Allocate,
    WriteLatest,
    WriteManifest,
    Prune,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BranchCampaignExplicitCommandV1 {
    Run,
    Inspect,
    Dataset,
    Continue,
    PlanCoverageGapContinuation,
    ExecuteCoverageGapContinuation,
    Artifact,
    SelfCheck,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum BranchCampaignPresetV1 {
    Quick,
    Focused,
    Explore,
    Deep,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub(super) enum BranchCampaignAscensionDomainArgV1 {
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
pub(super) enum BranchCampaignCombatRetryArgV1 {
    OnStall,
    Immediate,
    Disabled,
}

// Internal compatibility shape used by legacy top-level flags and by the
// command-specific parsers before they are converted into narrow handler inputs.
#[derive(Debug, ClapArgs)]
pub(super) struct Args {
    #[arg(skip)]
    pub(crate) explicit_command: Option<BranchCampaignExplicitCommandV1>,

    #[arg(skip)]
    pub(super) artifact_selector: Option<String>,

    #[arg(skip)]
    pub(super) artifact_campaign_dir: Option<PathBuf>,

    #[arg(skip)]
    pub(super) artifact_json: bool,

    #[arg(skip)]
    pub(super) artifact_action: Option<ArtifactActionV1>,

    #[arg(skip)]
    pub(super) artifact_kind: Option<ArtifactKindArgV1>,

    #[arg(skip)]
    pub(super) artifact_label: Option<String>,

    #[arg(skip)]
    pub(super) artifact_stamp: Option<String>,

    #[arg(skip)]
    pub(super) artifact_suffix: Option<String>,

    #[arg(skip)]
    pub(super) artifact_id: Option<String>,

    #[arg(skip)]
    pub(super) artifact_updated_at: Option<String>,

    #[arg(skip)]
    pub(super) artifact_manifest_path: Option<PathBuf>,

    #[arg(skip)]
    pub(super) artifact_payload_schema_name: Option<String>,

    #[arg(skip)]
    pub(super) artifact_created_at: Option<String>,

    #[arg(skip)]
    pub(super) artifact_keep_runs: usize,

    #[arg(skip)]
    pub(super) artifact_keep_scratch: usize,

    #[arg(skip)]
    pub(super) artifact_apply: bool,

    #[arg(long, value_enum)]
    pub(super) preset: Option<BranchCampaignPresetV1>,

    #[arg(long, default_value_t = 1)]
    pub(super) seed: u64,

    #[arg(long, default_value_t = 0)]
    pub(super) ascension: u8,

    #[arg(
        long,
        value_enum,
        help = "Set ascension from a named curriculum/target domain"
    )]
    pub(super) ascension_domain: Option<BranchCampaignAscensionDomainArgV1>,

    #[arg(long = "class", default_value = "ironclad")]
    pub(super) player_class: String,

    #[arg(long)]
    pub(super) final_act: bool,

    #[arg(long, default_value_t = 8)]
    pub(super) max_rounds: usize,

    #[arg(
        long,
        value_name = "N",
        conflicts_with = "until_round",
        help = "Run N additional campaign rounds in this invocation; clearer alias for the legacy per-call --max-rounds budget"
    )]
    pub(super) rounds: Option<usize>,

    #[arg(
        long = "until-round",
        value_name = "N",
        conflicts_with = "rounds",
        help = "When resuming, run only enough additional rounds to reach total completed round N"
    )]
    pub(super) until_round: Option<usize>,

    #[arg(
        long = "until-milestone",
        value_name = "MILESTONE",
        help = "Rust-owned milestone continuation target: Act1Boss, Act2Start, Act2Boss, Act3Boss, or CurrentActBoss"
    )]
    pub(super) until_milestone: Option<String>,

    #[arg(long = "milestone-step-rounds", default_value_t = 2)]
    pub(super) milestone_step_rounds: usize,

    #[arg(long = "milestone-max-rounds", default_value_t = 24)]
    pub(super) milestone_max_rounds: usize,

    #[arg(long = "milestone-stop", default_value = "auto")]
    pub(super) milestone_stop: String,

    #[arg(long, default_value_t = 1)]
    pub(super) round_depth: usize,

    #[arg(long, default_value_t = 8)]
    pub(super) max_active: usize,

    #[arg(long, default_value_t = 32)]
    pub(super) max_frozen: usize,

    #[arg(long, default_value_t = 12)]
    pub(super) max_branches_per_active: usize,

    #[arg(
        long,
        help = "Isolate branch campaign scheduled/parked budgets by boss relic lineage"
    )]
    pub(super) boss_relic_axes: bool,

    #[arg(long, default_value = "package")]
    pub(super) retention_profile: String,

    #[arg(long)]
    pub(super) max_reward_options: Option<usize>,

    #[arg(long)]
    pub(super) all_reward_options: bool,

    #[arg(long, default_value_t = 3)]
    pub(super) max_campfire_options: usize,

    #[arg(long, default_value_t = 128)]
    pub(super) auto_max_ops: usize,

    #[arg(long, default_value_t = 10_000)]
    pub(super) experiment_wall_ms: u64,

    #[arg(long)]
    pub(super) search_max_nodes: Option<usize>,

    #[arg(long, default_value_t = 200)]
    pub(super) search_wall_ms: u64,

    #[arg(long)]
    pub(super) max_hp_loss: Option<String>,

    #[arg(
        long = "combat-search-option",
        value_name = "KEY=VALUE",
        help = "Additional run_control search-combat option forwarded to branch experiments"
    )]
    pub(super) combat_search_options: Vec<String>,

    #[arg(long, value_enum, default_value_t = BranchCampaignCombatRetryArgV1::OnStall)]
    pub(super) combat_retry: BranchCampaignCombatRetryArgV1,

    #[arg(
        long,
        help = "Override the wall-clock budget used by the one-shot combat retry pass"
    )]
    pub(super) combat_retry_wall_ms: Option<u64>,

    #[arg(long, default_value_t = 20)]
    pub(super) min_acceptable_victory_hp_percent: u8,

    #[arg(long = "prefix", value_name = "COMMAND")]
    pub(super) prefix_commands: Vec<String>,

    #[arg(long)]
    pub(super) no_neow_guidance: bool,

    #[arg(long, default_value_t = 4)]
    pub(super) branch_examples: usize,

    #[arg(long)]
    pub(super) json: bool,

    #[arg(long, value_enum, default_value_t = CampaignReportDetailArg::Human)]
    pub(super) report_detail: CampaignReportDetailArg,

    #[arg(long, help = "Print coarse campaign progress to stderr while running")]
    pub(super) progress: bool,

    #[arg(long, value_enum, default_value_t = CampaignProgressDetailArg::Summary)]
    pub(super) progress_detail: CampaignProgressDetailArg,

    #[arg(
        long = "self-check-ancestor-replay",
        help = "Run a deterministic replay-cache self-check and print machine-readable counters"
    )]
    pub(super) self_check_ancestor_replay: bool,

    #[arg(
        long,
        value_name = "PATH",
        help = "Resume from a previous BranchCampaignV1 JSON report"
    )]
    pub(super) resume: Option<PathBuf>,

    #[arg(
        long = "resume-checkpoint",
        value_name = "PATH",
        help = "Resume exact branch sessions from a BranchCampaignCheckpointV2 sidecar"
    )]
    pub(super) resume_checkpoint: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Write the resulting BranchCampaignV1 JSON report"
    )]
    pub(super) out: Option<PathBuf>,

    #[arg(
        long = "checkpoint-out",
        value_name = "PATH",
        help = "Write the resulting BranchCampaignCheckpointV2 exact session sidecar"
    )]
    pub(super) checkpoint_out: Option<PathBuf>,

    #[arg(
        long = "auto-capture-combat",
        help = "Auto-save current-version CombatCaptureV1 cases whenever campaign automation enters a fresh combat"
    )]
    pub(super) auto_capture_combat: bool,

    #[arg(
        long = "auto-capture-root",
        value_name = "PATH",
        help = "Benchmark root for --auto-capture-combat; defaults to tools/artifacts/benchmarks/seed{seed}_act{act}"
    )]
    pub(super) auto_capture_root: Option<PathBuf>,

    #[arg(
        long = "inspect-checkpoint",
        value_name = "PATH",
        help = "Inspect a saved BranchCampaignCheckpointV2 session instead of running a campaign"
    )]
    pub(super) inspect_checkpoint: Option<PathBuf>,

    #[arg(
        long = "inspect-report",
        value_name = "PATH",
        help = "Pair --inspect-checkpoint with a BranchCampaignV1 report for scheduled/parked/abandoned labels"
    )]
    pub(super) inspect_report: Option<PathBuf>,

    #[arg(
        long = "inspect-summary",
        help = "Print compact deck/resource/strategy summaries for checkpoint sessions"
    )]
    pub(super) inspect_summary: bool,

    #[arg(
        long = "inspect-act",
        help = "Filter inspected checkpoint sessions by act"
    )]
    pub(super) inspect_act: Option<u8>,

    #[arg(
        long = "inspect-floor",
        help = "Filter inspected checkpoint sessions by floor"
    )]
    pub(super) inspect_floor: Option<i32>,

    #[arg(
        long = "inspect-boundary",
        help = "Filter inspected checkpoint sessions by visible boundary title, e.g. Shop or Card Reward"
    )]
    pub(super) inspect_boundary: Option<String>,

    #[arg(
        long = "inspect-hp",
        help = "Filter inspected checkpoint sessions by current HP"
    )]
    pub(super) inspect_hp: Option<i32>,

    #[arg(
        long = "inspect-index",
        help = "Select the Nth matching checkpoint session after filters"
    )]
    pub(super) inspect_index: Option<usize>,

    #[arg(
        long = "inspect-search",
        help = "Run search-combat from the selected checkpoint session and print the result"
    )]
    pub(super) inspect_search: bool,

    #[arg(
        long = "inspect-last-auto-combat",
        help = "Print the last stored automated combat trajectory for the selected checkpoint session"
    )]
    pub(super) inspect_last_auto_combat: bool,

    #[arg(
        long = "inspect-combat-lab",
        help = "Print a report-only CombatLabProbeV1 packet for the selected checkpoint branch"
    )]
    pub(super) inspect_combat_lab: bool,

    #[arg(
        long = "probe-boss",
        help = "When used with --inspect-combat-lab, run a report-only search preview against the current act boss"
    )]
    pub(super) probe_boss: bool,

    #[arg(
        long = "inspect-shop-evidence",
        help = "Print current-code shop candidate evidence and strategic deltas for the selected checkpoint session"
    )]
    pub(super) inspect_shop_evidence: bool,

    #[arg(
        long = "inspect-evidence-detail",
        value_enum,
        default_value_t = InspectEvidenceDetailArg::Compact,
        help = "Detail level for evidence inspect probes; compact hides full candidate tables"
    )]
    pub(super) inspect_evidence_detail: InspectEvidenceDetailArg,

    #[arg(
        long = "challenge-shop-plans",
        help = "From a selected shop checkpoint, force compiled shop plans and rollout each branch for comparison"
    )]
    pub(super) challenge_shop_plans: bool,

    #[arg(
        long = "challenge-max-plans",
        default_value_t = 6,
        help = "Maximum selected+alternative shop plans to challenge"
    )]
    pub(super) challenge_max_plans: usize,

    #[arg(
        long = "challenge-depth",
        default_value_t = 4,
        help = "Branch experiment depth after each challenged shop plan"
    )]
    pub(super) challenge_depth: usize,

    #[arg(
        long = "challenge-max-branches",
        default_value_t = 12,
        help = "Branch cap for each challenged shop plan rollout"
    )]
    pub(super) challenge_max_branches: usize,

    #[arg(
        long = "inspect-card-reward-evidence",
        help = "Print current-code card reward candidate evidence and strategic deltas for the selected checkpoint session"
    )]
    pub(super) inspect_card_reward_evidence: bool,

    #[arg(
        long = "inspect-campfire-evidence",
        help = "Print current-code campfire candidate evidence and selected plan for the selected checkpoint session"
    )]
    pub(super) inspect_campfire_evidence: bool,

    #[arg(
        long = "inspect-deck-mutation",
        help = "Print current-code DeckMutationCompiler plan groups for the selected checkpoint session"
    )]
    pub(super) inspect_deck_mutation: bool,

    #[arg(
        long = "inspect-route-evidence",
        help = "Print current-code route planner candidate evidence for the selected map checkpoint session"
    )]
    pub(super) inspect_route_evidence: bool,

    #[arg(
        long = "inspect-decision-observations",
        help = "Print historical branch decision observations stored in a BranchCampaignV1 report"
    )]
    pub(super) inspect_decision_observations: bool,

    #[arg(
        long = "inspect-journal",
        help = "Print CampaignJournal decision events stored in a BranchCampaignV1 report"
    )]
    pub(super) inspect_journal: bool,

    #[arg(
        long = "inspect-lineage-decisions",
        help = "Print CampaignJournal candidate pools along a selected report branch lineage"
    )]
    pub(super) inspect_lineage_decisions: bool,

    #[arg(
        long = "inspect-decision-coverage",
        help = "Print journal candidate continuation coverage from a BranchCampaignV1 report"
    )]
    pub(super) inspect_decision_coverage: bool,

    #[arg(
        long = "inspect-coverage-gap-milestone-summary",
        help = "Print milestone progress for coverage-gap continuation result branches"
    )]
    pub(super) inspect_coverage_gap_milestone_summary: bool,

    #[arg(
        long = "inspect-coverage-gap-target-state",
        help = "Inspect the checkpoint state for the selected coverage-gap milestone target group"
    )]
    pub(super) inspect_coverage_gap_target_state: bool,

    #[arg(
        long = "coverage-gap-milestone-target",
        default_value = "Act2Start",
        help = "Milestone target for --inspect-coverage-gap-milestone-summary: Act1Boss, Act2Start, Act2Boss, or Act3Boss"
    )]
    pub(super) coverage_gap_milestone_target: String,

    #[arg(
        long = "inspect-query",
        value_name = "TEXT",
        help = "Filter decision observations by candidate label, semantic class, frontier key, or parent choices"
    )]
    pub(super) inspect_query: Option<String>,

    #[arg(
        long = "inspect-final-boss-combat",
        help = "Print a final boss combat timeline from a BranchCampaignV1 report"
    )]
    pub(super) inspect_final_boss_combat: bool,

    #[arg(
        long = "export-outcome-dataset",
        value_name = "PATH",
        help = "Write BranchOutcomeRecordV1 JSONL from a campaign report and optional checkpoint sidecar"
    )]
    pub(super) export_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "analyze-outcome-dataset",
        value_name = "PATH",
        help = "Print structural issue counts from a BranchOutcomeRecordV1 JSONL file"
    )]
    pub(super) analyze_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "analyze-decision-outcome-dataset",
        value_name = "PATH",
        help = "Print sibling decision group coverage and outcome divergence from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    pub(super) analyze_decision_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "probe-learning-readiness",
        value_name = "PATH",
        help = "Diagnose whether a LearningDecisionOutcomeSampleV1 JSONL is blocked by censoring, scheduling, combat budget, missing context, or missing siblings"
    )]
    pub(super) probe_learning_readiness: Option<PathBuf>,

    #[arg(
        long = "plan-targeted-continuation",
        value_name = "PATH",
        help = "Print targeted sibling continuation groups from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    pub(super) plan_targeted_continuation: Option<PathBuf>,

    #[arg(
        long = "execute-targeted-continuation",
        value_name = "PATH",
        help = "Resume selected censored sibling branches from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    pub(super) execute_targeted_continuation: Option<PathBuf>,

    #[arg(
        long = "plan-coverage-gap-continuation",
        help = "Print unobserved CampaignJournal candidate branches to continue from a BranchCampaignV1 report"
    )]
    pub(super) plan_coverage_gap_continuation: bool,

    #[arg(
        long = "execute-coverage-gap-continuation",
        help = "Resume unobserved CampaignJournal candidate branches from --resume and --resume-checkpoint"
    )]
    pub(super) execute_coverage_gap_continuation: bool,

    #[arg(
        long = "continuation-effect-before",
        value_name = "PATH",
        help = "Before LearningDecisionOutcomeSampleV1 JSONL for targeted continuation effect comparison"
    )]
    pub(super) continuation_effect_before: Option<PathBuf>,

    #[arg(
        long = "continuation-effect-after",
        value_name = "PATH",
        help = "After LearningDecisionOutcomeSampleV1 JSONL for targeted continuation effect comparison"
    )]
    pub(super) continuation_effect_after: Option<PathBuf>,

    #[arg(
        long = "targeted-continuation-limit",
        default_value_t = 4,
        help = "Maximum targeted sibling groups to continue"
    )]
    pub(super) targeted_continuation_limit: usize,

    #[arg(
        long = "targeted-continuation-candidates-per-target",
        default_value_t = 1,
        help = "Maximum censored candidate branches to continue per targeted sibling group"
    )]
    pub(super) targeted_continuation_candidates_per_target: usize,

    #[arg(
        long = "coverage-gap-limit",
        default_value_t = 8,
        help = "Maximum unobserved journal candidate branches to plan or execute"
    )]
    pub(super) coverage_gap_limit: usize,

    #[arg(
        long = "coverage-gap-candidates-per-decision",
        default_value_t = 1,
        help = "Maximum unobserved candidate branches to continue per journal decision"
    )]
    pub(super) coverage_gap_candidates_per_decision: usize,

    #[arg(
        long = "coverage-gap-bucket",
        help = "Only plan or execute coverage-gap targets from this bucket, e.g. event, route, shop, reward"
    )]
    pub(super) coverage_gap_bucket: Option<String>,

    #[arg(
        long = "coverage-gap-event-id",
        help = "Only plan or execute coverage-gap targets whose event id/frontier/candidate text matches this event id"
    )]
    pub(super) coverage_gap_event_id: Option<String>,

    #[arg(
        long = "coverage-gap-lane",
        help = "Only plan or execute coverage-gap targets whose lane matches this text, e.g. effect:event_card_reward"
    )]
    pub(super) coverage_gap_lane: Option<String>,

    #[arg(
        long = "coverage-gap-origin-source",
        help = "Only plan or execute coverage-gap targets from this target_origin source, e.g. route_candidate_pool, map_decision_packet, event_boundary_packet"
    )]
    pub(super) coverage_gap_origin_source: Option<String>,

    #[arg(
        long = "coverage-gap-progress",
        help = "Only plan or execute coverage-gap targets with this existing progress, e.g. missing, target_only, extended"
    )]
    pub(super) coverage_gap_progress: Option<String>,

    #[arg(
        long = "coverage-gap-budget-intent",
        default_value = "gap_closure",
        help = "Select and interpret coverage-gap continuation targets as gap_closure or frontier_expansion"
    )]
    pub(super) coverage_gap_budget_intent: String,

    #[arg(
        long = "coverage-gap-execution-mode",
        default_value = "advance_rounds",
        help = "Execute coverage-gap targets as target_only or advance_rounds"
    )]
    pub(super) coverage_gap_execution_mode: String,

    #[arg(
        long = "export-learning-dataset",
        value_name = "PATH",
        help = "Write LearningBranchSampleV1 JSONL from a campaign report/run without treating choices as teacher labels"
    )]
    pub(super) export_learning_dataset: Option<PathBuf>,

    #[arg(
        long = "export-decision-outcome-dataset",
        value_name = "PATH",
        help = "Write LearningDecisionOutcomeSampleV1 JSONL with observed sibling candidates and later outcomes"
    )]
    pub(super) export_decision_outcome_dataset: Option<PathBuf>,
}

#[cfg(test)]
impl Args {
    pub(super) fn try_parse_from<I, T>(itr: I) -> Result<Self, clap::Error>
    where
        I: IntoIterator<Item = T>,
        T: Into<std::ffi::OsString> + Clone,
    {
        parse_args_from(itr)
    }
}

#[derive(Debug, Parser)]
#[command(
    name = "branch_campaign_driver",
    about = "Advance a small campaign of noncombat branches until victory, budget, or strategy boundary"
)]
struct CliRootV1 {
    #[command(subcommand)]
    command: Option<BranchCampaignCliCommandV1>,
}

#[derive(Debug, Subcommand)]
enum BranchCampaignCliCommandV1 {
    #[command(
        about = "Rust-owned campaign application namespace: run, continue, coverage, inspect, artifacts, export"
    )]
    Campaign(CampaignCommandArgs),
    #[command(hide = true, about = "Run or resume a branch campaign")]
    Run(RunCommandArgs),
    #[command(
        hide = true,
        about = "Inspect campaign checkpoints and report artifacts"
    )]
    Inspect(InspectCommandArgs),
    #[command(hide = true, about = "Export or analyze campaign outcome datasets")]
    Dataset(DatasetCommandArgs),
    #[command(
        hide = true,
        about = "Plan, execute, or compare targeted sibling continuations"
    )]
    Continue(ContinueCommandArgs),
    #[command(hide = true, about = "Resolve campaign artifact selectors and paths")]
    Artifact(ArtifactCommandArgs),
    #[command(
        hide = true,
        name = "self-check",
        about = "Run internal campaign driver self-checks"
    )]
    SelfCheck(SelfCheckCommandArgs),
}

#[derive(Debug, ClapArgs)]
struct CampaignCommandArgs {
    #[command(subcommand)]
    command: CampaignSubcommandV1,
}

#[derive(Debug, Subcommand)]
enum CampaignSubcommandV1 {
    #[command(about = "Run a new campaign through the campaign namespace")]
    Run(RunCommandArgs),
    #[command(about = "Continue an existing campaign through the campaign namespace")]
    Continue(ContinueCommandArgs),
    #[command(about = "Inspect campaign artifacts through the campaign namespace")]
    Inspect(InspectCommandArgs),
    #[command(about = "Export or analyze campaign datasets through the campaign namespace")]
    Export(DatasetCommandArgs),
    #[command(about = "Plan or execute candidate coverage through the campaign namespace")]
    Coverage(CampaignCoverageCommandArgs),
    #[command(about = "Resolve, show, or prune campaign artifacts through the campaign namespace")]
    Artifacts(ArtifactCommandArgs),
}

#[derive(Debug)]
pub(super) enum BranchCampaignCliInputV1 {
    Legacy(Args),
    Explicit {
        command: BranchCampaignExplicitCommandV1,
        args: Args,
    },
    CampaignRun(RunCommandArgs),
    CampaignContinue(ContinueCommandArgs),
    CampaignArtifact(ArtifactCommandArgs),
    CampaignCoveragePlan(CampaignCoveragePlanCommandArgs),
    CampaignCoverageExecute(CampaignCoverageExecuteCommandArgs),
    CampaignDataset(DatasetCommandArgs),
}

impl BranchCampaignCliInputV1 {
    #[cfg(test)]
    pub(super) fn args(&self) -> &Args {
        match self {
            Self::Legacy(args) => args,
            Self::Explicit { args, .. } => args,
            Self::CampaignRun(_) => {
                panic!("campaign namespace direct requests do not expose legacy Args")
            }
            Self::CampaignContinue(_) => {
                panic!("campaign namespace direct requests do not expose legacy Args")
            }
            Self::CampaignArtifact(_) => {
                panic!("campaign namespace direct requests do not expose legacy Args")
            }
            Self::CampaignCoveragePlan(_) => {
                panic!("campaign namespace direct requests do not expose legacy Args")
            }
            Self::CampaignCoverageExecute(_) => {
                panic!("campaign namespace direct requests do not expose legacy Args")
            }
            Self::CampaignDataset(_) => {
                panic!("campaign namespace direct requests do not expose legacy Args")
            }
        }
    }

    #[cfg(test)]
    pub(super) fn into_args(self) -> Args {
        match self {
            Self::Legacy(args) => args,
            Self::Explicit { args, .. } => args,
            Self::CampaignRun(args) => args.into_args(),
            Self::CampaignContinue(args) => args.into_args(),
            Self::CampaignArtifact(args) => args.into_args(),
            Self::CampaignCoveragePlan(args) => {
                (CampaignCoverageCommandArgs {
                    command: CampaignCoverageSubcommandV1::Plan(args),
                })
                .into_args_and_command()
                .0
            }
            Self::CampaignCoverageExecute(args) => {
                (CampaignCoverageCommandArgs {
                    command: CampaignCoverageSubcommandV1::Execute(args),
                })
                .into_args_and_command()
                .0
            }
            Self::CampaignDataset(args) => args.into_args(),
        }
    }

    #[cfg(test)]
    pub(super) fn explicit_command(&self) -> Option<BranchCampaignExplicitCommandV1> {
        match self {
            Self::Legacy(_) => None,
            Self::Explicit { command, .. } => Some(*command),
            Self::CampaignRun(_) => Some(BranchCampaignExplicitCommandV1::Run),
            Self::CampaignContinue(_) => Some(BranchCampaignExplicitCommandV1::Continue),
            Self::CampaignArtifact(_) => Some(BranchCampaignExplicitCommandV1::Artifact),
            Self::CampaignCoveragePlan(_) => {
                Some(BranchCampaignExplicitCommandV1::PlanCoverageGapContinuation)
            }
            Self::CampaignCoverageExecute(_) => {
                Some(BranchCampaignExplicitCommandV1::ExecuteCoverageGapContinuation)
            }
            Self::CampaignDataset(_) => Some(BranchCampaignExplicitCommandV1::Dataset),
        }
    }
}

// Public CLI surfaces are command-specific. Keep new flags in these structs
// unless they truly belong to the legacy top-level compatibility parser.
#[derive(Debug, ClapArgs)]
pub(super) struct RunCommandArgs {
    #[command(flatten)]
    pub(super) domain: CampaignDomainArgs,

    #[command(flatten)]
    pub(super) branching: CampaignBranchingArgs,

    #[command(flatten)]
    pub(super) search: CampaignSearchArgs,

    #[command(flatten)]
    pub(super) retry: CampaignCombatRetryArgs,

    #[command(flatten)]
    pub(super) prefix: CampaignPrefixArgs,

    #[command(flatten)]
    pub(super) output: CampaignRunOutputArgs,
}

#[derive(Debug, ClapArgs)]
struct InspectCommandArgs {
    #[command(flatten)]
    target: InspectTargetArgs,

    #[command(flatten)]
    modes: InspectModeArgs,

    #[command(flatten)]
    search: CampaignSearchArgs,

    #[command(flatten)]
    challenge: InspectChallengeArgs,

    #[command(flatten)]
    display: InspectDisplayArgs,
}

#[derive(Debug, ClapArgs)]
pub(super) struct DatasetCommandArgs {
    #[command(flatten)]
    pub(super) paths: DatasetPathArgs,
}

#[derive(Debug, ClapArgs)]
pub(super) struct ContinueCommandArgs {
    #[command(flatten)]
    pub(super) domain: CampaignDomainArgs,

    #[command(flatten)]
    pub(super) branching: CampaignBranchingArgs,

    #[command(flatten)]
    pub(super) search: CampaignSearchArgs,

    #[command(flatten)]
    pub(super) retry: CampaignCombatRetryArgs,

    #[command(flatten)]
    pub(super) prefix: CampaignPrefixArgs,

    #[command(flatten)]
    pub(super) output: CampaignRunOutputArgs,

    #[command(flatten)]
    pub(super) continuation: ContinuationArgs,
}

#[derive(Debug, ClapArgs)]
pub(super) struct ArtifactCommandArgs {
    #[command(subcommand)]
    pub(super) command: ArtifactSubcommandV1,
}

#[derive(Debug, Subcommand)]
pub(super) enum ArtifactSubcommandV1 {
    #[command(about = "Resolve latest, scratch-latest, run:<id>, scratch:<id>, or path:<report>")]
    Resolve(ArtifactResolveCommandArgs),
    #[command(about = "Resolve an artifact selector and summarize reusable run identity")]
    SourceInfo(ArtifactSourceInfoCommandArgs),
    #[command(about = "Allocate run/scratch artifact output paths")]
    Allocate(ArtifactAllocateCommandArgs),
    #[command(about = "Write latest or scratch-latest pointer for an artifact id")]
    WriteLatest(ArtifactWriteLatestCommandArgs),
    #[command(about = "Write a campaign artifact manifest envelope from JSON payload on stdin")]
    WriteManifest(ArtifactWriteManifestCommandArgs),
    #[command(about = "List or delete old campaign artifacts while protecting latest pointers")]
    Prune(ArtifactPruneCommandArgs),
}

#[derive(Debug, ClapArgs)]
pub(super) struct ArtifactResolveCommandArgs {
    #[arg(value_name = "SELECTOR")]
    pub(super) selector: String,

    #[arg(
        long = "campaign-dir",
        value_name = "PATH",
        default_value = "tools/artifacts/campaigns",
        help = "Campaign artifact root that contains latest.json, runs/, and scratch/"
    )]
    pub(super) campaign_dir: PathBuf,

    #[arg(long, help = "Print resolved artifact paths as JSON")]
    pub(super) json: bool,
}

#[derive(Debug, ClapArgs)]
pub(super) struct ArtifactSourceInfoCommandArgs {
    #[arg(value_name = "SELECTOR")]
    pub(super) selector: String,

    #[arg(
        long = "campaign-dir",
        value_name = "PATH",
        default_value = "tools/artifacts/campaigns",
        help = "Campaign artifact root that contains latest.json, runs/, and scratch/"
    )]
    pub(super) campaign_dir: PathBuf,

    #[arg(long, help = "Print source info as JSON")]
    pub(super) json: bool,
}

#[derive(Debug, ClapArgs)]
pub(super) struct ArtifactAllocateCommandArgs {
    #[arg(long, value_enum, default_value_t = ArtifactKindArgV1::Run)]
    pub(super) kind: ArtifactKindArgV1,

    #[arg(long, value_name = "TEXT")]
    pub(super) label: String,

    #[arg(
        long,
        value_name = "YYYYMMDD-HHMMSS",
        help = "Optional deterministic timestamp component; defaults to a Rust-generated UTC stamp"
    )]
    pub(super) stamp: Option<String>,

    #[arg(
        long,
        value_name = "TEXT",
        help = "Optional deterministic suffix component; defaults to a Rust-generated short suffix"
    )]
    pub(super) suffix: Option<String>,

    #[arg(
        long = "campaign-dir",
        value_name = "PATH",
        default_value = "tools/artifacts/campaigns",
        help = "Campaign artifact root that contains latest.json, runs/, and scratch/"
    )]
    pub(super) campaign_dir: PathBuf,

    #[arg(long, help = "Print allocated artifact paths as JSON")]
    pub(super) json: bool,
}

#[derive(Debug, ClapArgs)]
pub(super) struct ArtifactWriteLatestCommandArgs {
    #[arg(long, value_enum, default_value_t = ArtifactKindArgV1::Run)]
    pub(super) kind: ArtifactKindArgV1,

    #[arg(value_name = "ARTIFACT_ID")]
    pub(super) artifact_id: String,

    #[arg(long = "updated-at", value_name = "TIMESTAMP")]
    pub(super) updated_at: String,

    #[arg(
        long = "campaign-dir",
        value_name = "PATH",
        default_value = "tools/artifacts/campaigns",
        help = "Campaign artifact root that contains latest.json, runs/, and scratch/"
    )]
    pub(super) campaign_dir: PathBuf,

    #[arg(long, help = "Print written artifact paths as JSON")]
    pub(super) json: bool,
}

#[derive(Debug, ClapArgs)]
pub(super) struct ArtifactWriteManifestCommandArgs {
    #[arg(long = "manifest-path", value_name = "PATH")]
    pub(super) manifest_path: PathBuf,

    #[arg(
        long = "payload-schema-name",
        default_value = "CampaignWrapperManifestPayloadV1"
    )]
    pub(super) payload_schema_name: String,

    #[arg(long = "created-at", value_name = "TIMESTAMP")]
    pub(super) created_at: String,

    #[arg(long, help = "Print written manifest summary as JSON")]
    pub(super) json: bool,
}

#[derive(Debug, ClapArgs)]
pub(super) struct ArtifactPruneCommandArgs {
    #[arg(
        long = "campaign-dir",
        value_name = "PATH",
        default_value = "tools/artifacts/campaigns",
        help = "Campaign artifact root that contains latest.json, runs/, and scratch/"
    )]
    pub(super) campaign_dir: PathBuf,

    #[arg(long = "keep-runs", default_value_t = 5)]
    pub(super) keep_runs: usize,

    #[arg(long = "keep-scratch", default_value_t = 1)]
    pub(super) keep_scratch: usize,

    #[arg(long, help = "Delete prune candidates instead of only listing them")]
    pub(super) apply: bool,

    #[arg(long, help = "Print prune report as JSON")]
    pub(super) json: bool,
}

#[derive(Debug, ClapArgs)]
struct SelfCheckCommandArgs {}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignDomainArgs {
    #[arg(long, value_enum)]
    pub(super) preset: Option<BranchCampaignPresetV1>,

    #[arg(long, default_value_t = 1)]
    pub(super) seed: u64,

    #[arg(long, default_value_t = 0)]
    pub(super) ascension: u8,

    #[arg(
        long,
        value_enum,
        help = "Set ascension from a named curriculum/target domain"
    )]
    pub(super) ascension_domain: Option<BranchCampaignAscensionDomainArgV1>,

    #[arg(long = "class", default_value = "ironclad")]
    pub(super) player_class: String,

    #[arg(long)]
    pub(super) final_act: bool,
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignBranchingArgs {
    #[arg(long, default_value_t = 8)]
    pub(super) max_rounds: usize,

    #[arg(
        long,
        value_name = "N",
        conflicts_with = "until_round",
        help = "Run N additional campaign rounds in this invocation; clearer alias for the legacy per-call --max-rounds budget"
    )]
    pub(super) rounds: Option<usize>,

    #[arg(
        long = "until-round",
        value_name = "N",
        conflicts_with = "rounds",
        help = "When resuming, run only enough additional rounds to reach total completed round N"
    )]
    pub(super) until_round: Option<usize>,

    #[arg(
        long = "until-milestone",
        value_name = "MILESTONE",
        help = "Rust-owned milestone continuation target: Act1Boss, Act2Start, Act2Boss, Act3Boss, or CurrentActBoss"
    )]
    pub(super) until_milestone: Option<String>,

    #[arg(long = "milestone-step-rounds", default_value_t = 2)]
    pub(super) milestone_step_rounds: usize,

    #[arg(long = "milestone-max-rounds", default_value_t = 24)]
    pub(super) milestone_max_rounds: usize,

    #[arg(long = "milestone-stop", default_value = "auto")]
    pub(super) milestone_stop: String,

    #[arg(long, default_value_t = 1)]
    pub(super) round_depth: usize,

    #[arg(long, default_value_t = 8)]
    pub(super) max_active: usize,

    #[arg(long, default_value_t = 32)]
    pub(super) max_frozen: usize,

    #[arg(long, default_value_t = 12)]
    pub(super) max_branches_per_active: usize,

    #[arg(
        long,
        help = "Isolate branch campaign scheduled/parked budgets by boss relic lineage"
    )]
    pub(super) boss_relic_axes: bool,

    #[arg(long, default_value = "package")]
    pub(super) retention_profile: String,

    #[arg(long)]
    pub(super) max_reward_options: Option<usize>,

    #[arg(long)]
    pub(super) all_reward_options: bool,

    #[arg(long, default_value_t = 3)]
    pub(super) max_campfire_options: usize,

    #[arg(long, default_value_t = 128)]
    pub(super) auto_max_ops: usize,

    #[arg(long, default_value_t = 10_000)]
    pub(super) experiment_wall_ms: u64,
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignSearchArgs {
    #[arg(long)]
    pub(super) search_max_nodes: Option<usize>,

    #[arg(long, default_value_t = 200)]
    pub(super) search_wall_ms: u64,

    #[arg(long)]
    pub(super) max_hp_loss: Option<String>,

    #[arg(
        long = "combat-search-option",
        value_name = "KEY=VALUE",
        help = "Additional run_control search-combat option forwarded to branch experiments"
    )]
    pub(super) combat_search_options: Vec<String>,
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignCombatRetryArgs {
    #[arg(long, value_enum, default_value_t = BranchCampaignCombatRetryArgV1::OnStall)]
    pub(super) combat_retry: BranchCampaignCombatRetryArgV1,

    #[arg(
        long,
        help = "Override the wall-clock budget used by the one-shot combat retry pass"
    )]
    pub(super) combat_retry_wall_ms: Option<u64>,

    #[arg(long, default_value_t = 20)]
    pub(super) min_acceptable_victory_hp_percent: u8,
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignPrefixArgs {
    #[arg(long = "prefix", value_name = "COMMAND")]
    pub(super) prefix_commands: Vec<String>,

    #[arg(long)]
    pub(super) no_neow_guidance: bool,
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignRunOutputArgs {
    #[arg(long, default_value_t = 4)]
    pub(super) branch_examples: usize,

    #[arg(long)]
    pub(super) json: bool,

    #[arg(long, value_enum, default_value_t = CampaignReportDetailArg::Human)]
    pub(super) report_detail: CampaignReportDetailArg,

    #[arg(long, help = "Print coarse campaign progress to stderr while running")]
    pub(super) progress: bool,

    #[arg(long, value_enum, default_value_t = CampaignProgressDetailArg::Summary)]
    pub(super) progress_detail: CampaignProgressDetailArg,

    #[arg(
        long,
        value_name = "PATH",
        help = "Resume from a previous BranchCampaignV1 JSON report"
    )]
    pub(super) resume: Option<PathBuf>,

    #[arg(
        long = "resume-checkpoint",
        value_name = "PATH",
        help = "Resume exact branch sessions from a BranchCampaignCheckpointV2 sidecar"
    )]
    pub(super) resume_checkpoint: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Write the resulting BranchCampaignV1 JSON report"
    )]
    pub(super) out: Option<PathBuf>,

    #[arg(
        long = "checkpoint-out",
        value_name = "PATH",
        help = "Write the resulting BranchCampaignCheckpointV2 exact session sidecar"
    )]
    pub(super) checkpoint_out: Option<PathBuf>,

    #[arg(
        long = "auto-capture-combat",
        help = "Auto-save current-version CombatCaptureV1 cases whenever campaign automation enters a fresh combat"
    )]
    pub(super) auto_capture_combat: bool,

    #[arg(
        long = "auto-capture-root",
        value_name = "PATH",
        help = "Benchmark root for --auto-capture-combat"
    )]
    pub(super) auto_capture_root: Option<PathBuf>,

    #[arg(
        long = "export-outcome-dataset",
        value_name = "PATH",
        help = "Write BranchOutcomeRecordV1 JSONL from the resulting campaign report"
    )]
    pub(super) export_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "export-learning-dataset",
        value_name = "PATH",
        help = "Write LearningBranchSampleV1 JSONL from the resulting campaign report"
    )]
    pub(super) export_learning_dataset: Option<PathBuf>,

    #[arg(
        long = "export-decision-outcome-dataset",
        value_name = "PATH",
        help = "Write LearningDecisionOutcomeSampleV1 JSONL from the resulting campaign report"
    )]
    pub(super) export_decision_outcome_dataset: Option<PathBuf>,
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignContinuationOutputArgs {
    #[arg(long, default_value_t = 4)]
    pub(super) branch_examples: usize,

    #[arg(long)]
    pub(super) json: bool,

    #[arg(long, value_enum, default_value_t = CampaignReportDetailArg::Human)]
    pub(super) report_detail: CampaignReportDetailArg,

    #[arg(long, help = "Print coarse campaign progress to stderr while running")]
    pub(super) progress: bool,

    #[arg(long, value_enum, default_value_t = CampaignProgressDetailArg::Summary)]
    pub(super) progress_detail: CampaignProgressDetailArg,

    #[arg(
        long,
        value_name = "PATH",
        help = "Resume from a previous BranchCampaignV1 JSON report"
    )]
    pub(super) resume: Option<PathBuf>,

    #[arg(
        long = "resume-checkpoint",
        value_name = "PATH",
        help = "Resume exact branch sessions from a BranchCampaignCheckpointV2 sidecar"
    )]
    pub(super) resume_checkpoint: Option<PathBuf>,

    #[arg(
        long,
        value_name = "PATH",
        help = "Write the resulting BranchCampaignV1 JSON report"
    )]
    pub(super) out: Option<PathBuf>,

    #[arg(
        long = "checkpoint-out",
        value_name = "PATH",
        help = "Write the resulting BranchCampaignCheckpointV2 exact session sidecar"
    )]
    pub(super) checkpoint_out: Option<PathBuf>,
}

#[derive(Debug, ClapArgs)]
struct InspectTargetArgs {
    #[arg(
        long = "inspect-checkpoint",
        value_name = "PATH",
        help = "Inspect a saved BranchCampaignCheckpointV2 session instead of running a campaign"
    )]
    inspect_checkpoint: Option<PathBuf>,

    #[arg(
        long = "inspect-report",
        value_name = "PATH",
        help = "Pair --inspect-checkpoint with a BranchCampaignV1 report for scheduled/parked/abandoned labels"
    )]
    inspect_report: Option<PathBuf>,

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
}

#[derive(Debug, ClapArgs)]
struct InspectModeArgs {
    #[arg(
        long = "inspect-summary",
        help = "Print compact deck/resource/strategy summaries for checkpoint sessions"
    )]
    inspect_summary: bool,

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
        long = "inspect-evidence-detail",
        value_enum,
        default_value_t = InspectEvidenceDetailArg::Compact,
        help = "Detail level for evidence inspect probes; compact hides full candidate tables"
    )]
    inspect_evidence_detail: InspectEvidenceDetailArg,

    #[arg(
        long = "challenge-shop-plans",
        help = "From a selected shop checkpoint, force compiled shop plans and rollout each branch for comparison"
    )]
    challenge_shop_plans: bool,

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
        long = "inspect-decision-observations",
        help = "Print historical branch decision observations stored in a BranchCampaignV1 report"
    )]
    inspect_decision_observations: bool,

    #[arg(
        long = "inspect-journal",
        help = "Print CampaignJournal decision events stored in a BranchCampaignV1 report"
    )]
    inspect_journal: bool,

    #[arg(
        long = "inspect-lineage-decisions",
        help = "Print CampaignJournal candidate pools along a selected report branch lineage"
    )]
    inspect_lineage_decisions: bool,

    #[arg(
        long = "inspect-decision-coverage",
        help = "Print journal candidate continuation coverage from a BranchCampaignV1 report"
    )]
    inspect_decision_coverage: bool,

    #[arg(
        long = "inspect-coverage-gap-milestone-summary",
        help = "Print milestone progress for coverage-gap continuation result branches"
    )]
    inspect_coverage_gap_milestone_summary: bool,

    #[arg(
        long = "inspect-coverage-gap-target-state",
        help = "Inspect the checkpoint state for the selected coverage-gap milestone target group"
    )]
    inspect_coverage_gap_target_state: bool,

    #[arg(
        long = "coverage-gap-milestone-target",
        default_value = "Act2Start",
        help = "Milestone target for --inspect-coverage-gap-milestone-summary: Act1Boss, Act2Start, Act2Boss, or Act3Boss"
    )]
    coverage_gap_milestone_target: String,

    #[arg(
        long = "coverage-gap-bucket",
        help = "Only inspect coverage-gap milestone rows from this bucket, e.g. event, route, shop, reward"
    )]
    coverage_gap_bucket: Option<String>,

    #[arg(
        long = "coverage-gap-event-id",
        help = "Only inspect coverage-gap milestone rows whose target key/label/command matches this event id"
    )]
    coverage_gap_event_id: Option<String>,

    #[arg(
        long = "coverage-gap-lane",
        help = "Only inspect coverage-gap milestone rows whose target lane matches this text"
    )]
    coverage_gap_lane: Option<String>,

    #[arg(
        long = "coverage-gap-origin-source",
        help = "Only inspect coverage-gap milestone rows from this target_origin source"
    )]
    coverage_gap_origin_source: Option<String>,

    #[arg(
        long = "coverage-gap-progress",
        help = "Only inspect coverage-gap milestone rows with this target progress"
    )]
    coverage_gap_progress: Option<String>,

    #[arg(
        long = "inspect-query",
        value_name = "TEXT",
        help = "Filter decision observations by candidate label, semantic class, frontier key, or parent choices"
    )]
    inspect_query: Option<String>,

    #[arg(
        long = "inspect-final-boss-combat",
        help = "Print a final boss combat timeline from a BranchCampaignV1 report"
    )]
    inspect_final_boss_combat: bool,
}

#[derive(Debug, ClapArgs)]
struct InspectChallengeArgs {
    #[arg(long)]
    final_act: bool,

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
}

#[derive(Debug, ClapArgs)]
struct InspectDisplayArgs {
    #[arg(long, default_value_t = 4)]
    branch_examples: usize,
}

#[derive(Debug, ClapArgs)]
pub(super) struct DatasetPathArgs {
    #[arg(
        long = "inspect-checkpoint",
        value_name = "PATH",
        help = "Optional BranchCampaignCheckpointV2 sidecar for dataset exports"
    )]
    pub(super) inspect_checkpoint: Option<PathBuf>,

    #[arg(
        long = "inspect-report",
        value_name = "PATH",
        help = "BranchCampaignV1 report used by dataset exports"
    )]
    pub(super) inspect_report: Option<PathBuf>,

    #[arg(
        long = "export-outcome-dataset",
        value_name = "PATH",
        help = "Write BranchOutcomeRecordV1 JSONL from a campaign report and optional checkpoint sidecar"
    )]
    pub(super) export_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "analyze-outcome-dataset",
        value_name = "PATH",
        help = "Print structural issue counts from a BranchOutcomeRecordV1 JSONL file"
    )]
    pub(super) analyze_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "analyze-decision-outcome-dataset",
        value_name = "PATH",
        help = "Print sibling decision group coverage and outcome divergence from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    pub(super) analyze_decision_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "probe-learning-readiness",
        value_name = "PATH",
        help = "Diagnose whether a LearningDecisionOutcomeSampleV1 JSONL is blocked by censoring, scheduling, combat budget, missing context, or missing siblings"
    )]
    pub(super) probe_learning_readiness: Option<PathBuf>,

    #[arg(
        long = "export-learning-dataset",
        value_name = "PATH",
        help = "Write LearningBranchSampleV1 JSONL from a campaign report/run without treating choices as teacher labels"
    )]
    pub(super) export_learning_dataset: Option<PathBuf>,

    #[arg(
        long = "export-decision-outcome-dataset",
        value_name = "PATH",
        help = "Write LearningDecisionOutcomeSampleV1 JSONL with observed sibling candidates and later outcomes"
    )]
    pub(super) export_decision_outcome_dataset: Option<PathBuf>,
}

#[derive(Debug, ClapArgs)]
pub(super) struct ContinuationArgs {
    #[arg(
        long = "plan-targeted-continuation",
        value_name = "PATH",
        help = "Print targeted sibling continuation groups from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    pub(super) plan_targeted_continuation: Option<PathBuf>,

    #[arg(
        long = "execute-targeted-continuation",
        value_name = "PATH",
        help = "Resume selected censored sibling branches from a LearningDecisionOutcomeSampleV1 JSONL file"
    )]
    pub(super) execute_targeted_continuation: Option<PathBuf>,

    #[arg(
        long = "execute-coverage-gap-continuation",
        help = "Resume unobserved CampaignJournal candidate branches from --resume and --resume-checkpoint"
    )]
    pub(super) execute_coverage_gap_continuation: bool,

    #[arg(
        long = "continuation-effect-before",
        value_name = "PATH",
        help = "Before LearningDecisionOutcomeSampleV1 JSONL for targeted continuation effect comparison"
    )]
    pub(super) continuation_effect_before: Option<PathBuf>,

    #[arg(
        long = "continuation-effect-after",
        value_name = "PATH",
        help = "After LearningDecisionOutcomeSampleV1 JSONL for targeted continuation effect comparison"
    )]
    pub(super) continuation_effect_after: Option<PathBuf>,

    #[arg(
        long = "targeted-continuation-limit",
        default_value_t = 4,
        help = "Maximum targeted sibling groups to continue"
    )]
    pub(super) targeted_continuation_limit: usize,

    #[arg(
        long = "targeted-continuation-candidates-per-target",
        default_value_t = 1,
        help = "Maximum censored candidate branches to continue per targeted sibling group"
    )]
    pub(super) targeted_continuation_candidates_per_target: usize,

    #[arg(
        long = "coverage-gap-limit",
        default_value_t = 8,
        help = "Maximum unobserved journal candidate branches to execute"
    )]
    pub(super) coverage_gap_limit: usize,

    #[arg(
        long = "coverage-gap-candidates-per-decision",
        default_value_t = 1,
        help = "Maximum unobserved candidate branches to continue per journal decision"
    )]
    pub(super) coverage_gap_candidates_per_decision: usize,

    #[arg(
        long = "coverage-gap-bucket",
        help = "Only execute coverage-gap targets from this bucket, e.g. event, route, shop, reward"
    )]
    pub(super) coverage_gap_bucket: Option<String>,

    #[arg(
        long = "coverage-gap-event-id",
        help = "Only execute coverage-gap targets whose event id/frontier/candidate text matches this event id"
    )]
    pub(super) coverage_gap_event_id: Option<String>,

    #[arg(
        long = "coverage-gap-lane",
        help = "Only execute coverage-gap targets whose lane matches this text, e.g. effect:event_card_reward"
    )]
    pub(super) coverage_gap_lane: Option<String>,

    #[arg(
        long = "coverage-gap-origin-source",
        help = "Only execute coverage-gap targets from this target_origin source, e.g. route_candidate_pool, map_decision_packet, event_boundary_packet"
    )]
    pub(super) coverage_gap_origin_source: Option<String>,

    #[arg(
        long = "coverage-gap-progress",
        help = "Only execute coverage-gap targets with this existing progress, e.g. missing, target_only, extended"
    )]
    pub(super) coverage_gap_progress: Option<String>,

    #[arg(
        long = "coverage-gap-budget-intent",
        default_value = "gap_closure",
        help = "Select and interpret coverage-gap continuation targets as gap_closure or frontier_expansion"
    )]
    pub(super) coverage_gap_budget_intent: String,

    #[arg(
        long = "coverage-gap-execution-mode",
        default_value = "advance_rounds",
        help = "Execute coverage-gap targets as target_only or advance_rounds"
    )]
    pub(super) coverage_gap_execution_mode: String,
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignCoverageExecuteTargetArgs {
    #[arg(
        long = "coverage-gap-limit",
        default_value_t = 8,
        help = "Maximum unobserved journal candidate branches to execute"
    )]
    pub(super) coverage_gap_limit: usize,

    #[arg(
        long = "coverage-gap-candidates-per-decision",
        default_value_t = 1,
        help = "Maximum unobserved candidate branches to continue per journal decision"
    )]
    pub(super) coverage_gap_candidates_per_decision: usize,

    #[arg(
        long = "coverage-gap-bucket",
        help = "Only execute coverage-gap targets from this bucket, e.g. event, route, shop, reward"
    )]
    pub(super) coverage_gap_bucket: Option<String>,

    #[arg(
        long = "coverage-gap-event-id",
        help = "Only execute coverage-gap targets whose event id/frontier/candidate text matches this event id"
    )]
    pub(super) coverage_gap_event_id: Option<String>,

    #[arg(
        long = "coverage-gap-lane",
        help = "Only execute coverage-gap targets whose lane matches this text, e.g. effect:event_card_reward"
    )]
    pub(super) coverage_gap_lane: Option<String>,

    #[arg(
        long = "coverage-gap-origin-source",
        help = "Only execute coverage-gap targets from this target_origin source, e.g. route_candidate_pool"
    )]
    pub(super) coverage_gap_origin_source: Option<String>,

    #[arg(
        long = "coverage-gap-progress",
        help = "Only execute coverage-gap targets with this existing progress, e.g. missing, target_only, extended"
    )]
    pub(super) coverage_gap_progress: Option<String>,

    #[arg(
        long = "coverage-gap-budget-intent",
        default_value = "gap_closure",
        help = "Select and interpret coverage-gap continuation targets as gap_closure or frontier_expansion"
    )]
    pub(super) coverage_gap_budget_intent: String,

    #[arg(
        long = "coverage-gap-execution-mode",
        default_value = "advance_rounds",
        help = "Execute coverage-gap targets as target_only or advance_rounds"
    )]
    pub(super) coverage_gap_execution_mode: String,
}

impl Args {
    fn compat_defaults() -> Self {
        Self {
            explicit_command: None,
            artifact_selector: None,
            artifact_campaign_dir: None,
            artifact_json: false,
            artifact_action: None,
            artifact_kind: None,
            artifact_label: None,
            artifact_stamp: None,
            artifact_suffix: None,
            artifact_id: None,
            artifact_updated_at: None,
            artifact_manifest_path: None,
            artifact_payload_schema_name: None,
            artifact_created_at: None,
            artifact_keep_runs: 5,
            artifact_keep_scratch: 1,
            artifact_apply: false,
            preset: None,
            seed: 1,
            ascension: 0,
            ascension_domain: None,
            player_class: "ironclad".to_string(),
            final_act: false,
            max_rounds: 8,
            rounds: None,
            until_round: None,
            until_milestone: None,
            milestone_step_rounds: 2,
            milestone_max_rounds: 24,
            milestone_stop: "auto".to_string(),
            round_depth: 1,
            max_active: 8,
            max_frozen: 32,
            max_branches_per_active: 12,
            boss_relic_axes: false,
            retention_profile: "package".to_string(),
            max_reward_options: None,
            all_reward_options: false,
            max_campfire_options: 3,
            auto_max_ops: 128,
            experiment_wall_ms: 10_000,
            search_max_nodes: None,
            search_wall_ms: 200,
            max_hp_loss: None,
            combat_search_options: Vec::new(),
            combat_retry: BranchCampaignCombatRetryArgV1::OnStall,
            combat_retry_wall_ms: None,
            min_acceptable_victory_hp_percent: 20,
            prefix_commands: Vec::new(),
            no_neow_guidance: false,
            branch_examples: 4,
            json: false,
            report_detail: CampaignReportDetailArg::Human,
            progress: false,
            progress_detail: CampaignProgressDetailArg::Summary,
            self_check_ancestor_replay: false,
            resume: None,
            resume_checkpoint: None,
            out: None,
            checkpoint_out: None,
            auto_capture_combat: false,
            auto_capture_root: None,
            inspect_checkpoint: None,
            inspect_report: None,
            inspect_summary: false,
            inspect_act: None,
            inspect_floor: None,
            inspect_boundary: None,
            inspect_hp: None,
            inspect_index: None,
            inspect_search: false,
            inspect_last_auto_combat: false,
            inspect_combat_lab: false,
            probe_boss: false,
            inspect_shop_evidence: false,
            inspect_evidence_detail: InspectEvidenceDetailArg::Compact,
            challenge_shop_plans: false,
            challenge_max_plans: 6,
            challenge_depth: 4,
            challenge_max_branches: 12,
            inspect_card_reward_evidence: false,
            inspect_campfire_evidence: false,
            inspect_deck_mutation: false,
            inspect_route_evidence: false,
            inspect_decision_observations: false,
            inspect_journal: false,
            inspect_lineage_decisions: false,
            inspect_decision_coverage: false,
            inspect_coverage_gap_milestone_summary: false,
            inspect_coverage_gap_target_state: false,
            coverage_gap_milestone_target: "Act2Start".to_string(),
            inspect_query: None,
            inspect_final_boss_combat: false,
            export_outcome_dataset: None,
            analyze_outcome_dataset: None,
            analyze_decision_outcome_dataset: None,
            probe_learning_readiness: None,
            plan_targeted_continuation: None,
            execute_targeted_continuation: None,
            plan_coverage_gap_continuation: false,
            execute_coverage_gap_continuation: false,
            continuation_effect_before: None,
            continuation_effect_after: None,
            targeted_continuation_limit: 4,
            targeted_continuation_candidates_per_target: 1,
            coverage_gap_limit: 8,
            coverage_gap_candidates_per_decision: 1,
            coverage_gap_bucket: None,
            coverage_gap_event_id: None,
            coverage_gap_lane: None,
            coverage_gap_origin_source: None,
            coverage_gap_progress: None,
            coverage_gap_budget_intent: "gap_closure".to_string(),
            coverage_gap_execution_mode: "advance_rounds".to_string(),
            export_learning_dataset: None,
            export_decision_outcome_dataset: None,
        }
    }
}

impl RunCommandArgs {
    fn into_args(self) -> Args {
        let mut args = Args::compat_defaults();
        self.domain.apply_to(&mut args);
        self.branching.apply_to(&mut args);
        self.search.apply_to(&mut args);
        self.retry.apply_to(&mut args);
        self.prefix.apply_to(&mut args);
        self.output.apply_to(&mut args);
        args
    }
}

impl CampaignCommandArgs {
    fn into_args_and_command(self) -> (Args, BranchCampaignExplicitCommandV1) {
        match self.command {
            CampaignSubcommandV1::Run(args) => {
                (args.into_args(), BranchCampaignExplicitCommandV1::Run)
            }
            CampaignSubcommandV1::Continue(args) => {
                (args.into_args(), BranchCampaignExplicitCommandV1::Continue)
            }
            CampaignSubcommandV1::Inspect(args) => {
                (args.into_args(), BranchCampaignExplicitCommandV1::Inspect)
            }
            CampaignSubcommandV1::Export(args) => {
                (args.into_args(), BranchCampaignExplicitCommandV1::Dataset)
            }
            CampaignSubcommandV1::Coverage(args) => args.into_args_and_command(),
            CampaignSubcommandV1::Artifacts(args) => {
                (args.into_args(), BranchCampaignExplicitCommandV1::Artifact)
            }
        }
    }

    fn into_cli_input(self, matches: &ArgMatches) -> Result<BranchCampaignCliInputV1, clap::Error> {
        match self.command {
            CampaignSubcommandV1::Run(args) => Ok(BranchCampaignCliInputV1::CampaignRun(
                normalize_run_command_args(args, matches)?,
            )),
            CampaignSubcommandV1::Continue(args) => Ok(BranchCampaignCliInputV1::CampaignContinue(
                normalize_continue_command_args(args, matches)?,
            )),
            CampaignSubcommandV1::Artifacts(args) => {
                Ok(BranchCampaignCliInputV1::CampaignArtifact(args))
            }
            CampaignSubcommandV1::Coverage(args) => args.into_cli_input(matches),
            CampaignSubcommandV1::Export(args) => {
                Ok(BranchCampaignCliInputV1::CampaignDataset(args))
            }
            command => {
                let (args, explicit_command) =
                    (CampaignCommandArgs { command }).into_args_and_command();
                normalize_cli_args_from_matches(args, Some(explicit_command), matches)
            }
        }
    }
}

impl InspectCommandArgs {
    fn into_args(self) -> Args {
        let mut args = Args::compat_defaults();
        self.target.apply_to(&mut args);
        self.modes.apply_to(&mut args);
        self.search.apply_to(&mut args);
        self.challenge.apply_to(&mut args);
        self.display.apply_to(&mut args);
        args
    }
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignCoverageCommandArgs {
    #[command(subcommand)]
    pub(super) command: CampaignCoverageSubcommandV1,
}

#[derive(Debug, Subcommand)]
pub(super) enum CampaignCoverageSubcommandV1 {
    #[command(about = "Plan unobserved journal candidate coverage targets")]
    Plan(CampaignCoveragePlanCommandArgs),
    #[command(about = "Execute unobserved journal candidate coverage targets")]
    Execute(CampaignCoverageExecuteCommandArgs),
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignCoveragePlanCommandArgs {
    #[command(flatten)]
    pub(super) target: CampaignCoveragePlanTargetArgs,
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignCoverageExecuteCommandArgs {
    #[command(flatten)]
    pub(super) domain: CampaignDomainArgs,

    #[command(flatten)]
    pub(super) branching: CampaignBranchingArgs,

    #[command(flatten)]
    pub(super) search: CampaignSearchArgs,

    #[command(flatten)]
    pub(super) retry: CampaignCombatRetryArgs,

    #[command(flatten)]
    pub(super) prefix: CampaignPrefixArgs,

    #[command(flatten)]
    pub(super) output: CampaignContinuationOutputArgs,

    #[command(flatten)]
    pub(super) coverage: CampaignCoverageExecuteTargetArgs,
}

impl CampaignCoverageCommandArgs {
    fn into_cli_input(self, matches: &ArgMatches) -> Result<BranchCampaignCliInputV1, clap::Error> {
        match self.command {
            CampaignCoverageSubcommandV1::Plan(args) => {
                Ok(BranchCampaignCliInputV1::CampaignCoveragePlan(args))
            }
            CampaignCoverageSubcommandV1::Execute(args) => {
                Ok(BranchCampaignCliInputV1::CampaignCoverageExecute(
                    normalize_coverage_execute_command_args(args, matches)?,
                ))
            }
        }
    }

    fn into_args_and_command(self) -> (Args, BranchCampaignExplicitCommandV1) {
        match self.command {
            CampaignCoverageSubcommandV1::Plan(args) => {
                let mut converted = Args::compat_defaults();
                args.target.apply_to(&mut converted);
                converted.plan_coverage_gap_continuation = true;
                (
                    converted,
                    BranchCampaignExplicitCommandV1::PlanCoverageGapContinuation,
                )
            }
            CampaignCoverageSubcommandV1::Execute(args) => {
                let mut converted = ContinueCommandArgs {
                    domain: args.domain,
                    branching: args.branching,
                    search: args.search,
                    retry: args.retry,
                    prefix: args.prefix,
                    output: args.output.into_run_output_args(),
                    continuation: args.coverage.into_continuation_args(),
                }
                .into_args();
                converted.execute_coverage_gap_continuation = true;
                (
                    converted,
                    BranchCampaignExplicitCommandV1::ExecuteCoverageGapContinuation,
                )
            }
        }
    }
}

impl DatasetCommandArgs {
    fn into_args(self) -> Args {
        let mut args = Args::compat_defaults();
        self.paths.apply_to(&mut args);
        args
    }
}

#[derive(Debug, ClapArgs)]
pub(super) struct CampaignCoveragePlanTargetArgs {
    #[arg(
        long = "inspect-checkpoint",
        value_name = "PATH",
        help = "Optional BranchCampaignCheckpointV2 sidecar paired with --inspect-report"
    )]
    pub(super) inspect_checkpoint: Option<PathBuf>,

    #[arg(
        long = "inspect-report",
        value_name = "PATH",
        help = "BranchCampaignV1 report whose unobserved journal candidates should be planned"
    )]
    pub(super) inspect_report: Option<PathBuf>,

    #[arg(
        long = "coverage-gap-limit",
        default_value_t = 8,
        help = "Maximum unobserved journal candidate branches to plan"
    )]
    pub(super) coverage_gap_limit: usize,

    #[arg(
        long = "coverage-gap-candidates-per-decision",
        default_value_t = 1,
        help = "Maximum unobserved candidate branches to continue per journal decision"
    )]
    pub(super) coverage_gap_candidates_per_decision: usize,

    #[arg(
        long = "coverage-gap-bucket",
        help = "Only plan coverage-gap targets from this bucket, e.g. event, route, shop, reward"
    )]
    pub(super) coverage_gap_bucket: Option<String>,

    #[arg(
        long = "coverage-gap-event-id",
        help = "Only plan coverage-gap targets whose event id/frontier/candidate text matches this event id"
    )]
    pub(super) coverage_gap_event_id: Option<String>,

    #[arg(
        long = "coverage-gap-lane",
        help = "Only plan coverage-gap targets whose lane matches this text, e.g. effect:event_card_reward"
    )]
    pub(super) coverage_gap_lane: Option<String>,

    #[arg(
        long = "coverage-gap-origin-source",
        help = "Only plan coverage-gap targets from this target_origin source, e.g. route_candidate_pool"
    )]
    pub(super) coverage_gap_origin_source: Option<String>,

    #[arg(
        long = "coverage-gap-progress",
        help = "Only plan coverage-gap targets with this existing progress, e.g. missing, target_only, extended"
    )]
    pub(super) coverage_gap_progress: Option<String>,

    #[arg(
        long = "coverage-gap-budget-intent",
        default_value = "gap_closure",
        help = "Select and interpret coverage-gap continuation targets as gap_closure or frontier_expansion"
    )]
    pub(super) coverage_gap_budget_intent: String,
}

impl CampaignCoveragePlanTargetArgs {
    fn apply_to(self, args: &mut Args) {
        args.inspect_checkpoint = self.inspect_checkpoint;
        args.inspect_report = self.inspect_report;
        args.coverage_gap_limit = self.coverage_gap_limit;
        args.coverage_gap_candidates_per_decision = self.coverage_gap_candidates_per_decision;
        args.coverage_gap_bucket = self.coverage_gap_bucket;
        args.coverage_gap_event_id = self.coverage_gap_event_id;
        args.coverage_gap_lane = self.coverage_gap_lane;
        args.coverage_gap_origin_source = self.coverage_gap_origin_source;
        args.coverage_gap_progress = self.coverage_gap_progress;
        args.coverage_gap_budget_intent = self.coverage_gap_budget_intent;
    }
}

impl ContinueCommandArgs {
    fn into_args(self) -> Args {
        let mut args = Args::compat_defaults();
        self.domain.apply_to(&mut args);
        self.branching.apply_to(&mut args);
        self.search.apply_to(&mut args);
        self.retry.apply_to(&mut args);
        self.prefix.apply_to(&mut args);
        self.output.apply_to(&mut args);
        self.continuation.apply_to(&mut args);
        args
    }
}

impl ArtifactCommandArgs {
    fn into_args(self) -> Args {
        let mut args = Args::compat_defaults();
        match self.command {
            ArtifactSubcommandV1::Resolve(resolve) => {
                args.artifact_action = Some(ArtifactActionV1::Resolve);
                args.artifact_selector = Some(resolve.selector);
                args.artifact_campaign_dir = Some(resolve.campaign_dir);
                args.artifact_json = resolve.json;
            }
            ArtifactSubcommandV1::SourceInfo(source_info) => {
                args.artifact_action = Some(ArtifactActionV1::SourceInfo);
                args.artifact_selector = Some(source_info.selector);
                args.artifact_campaign_dir = Some(source_info.campaign_dir);
                args.artifact_json = source_info.json;
            }
            ArtifactSubcommandV1::Allocate(allocate) => {
                args.artifact_action = Some(ArtifactActionV1::Allocate);
                args.artifact_kind = Some(allocate.kind);
                args.artifact_label = Some(allocate.label);
                args.artifact_stamp = allocate.stamp;
                args.artifact_suffix = allocate.suffix;
                args.artifact_campaign_dir = Some(allocate.campaign_dir);
                args.artifact_json = allocate.json;
            }
            ArtifactSubcommandV1::WriteLatest(write_latest) => {
                args.artifact_action = Some(ArtifactActionV1::WriteLatest);
                args.artifact_kind = Some(write_latest.kind);
                args.artifact_id = Some(write_latest.artifact_id);
                args.artifact_updated_at = Some(write_latest.updated_at);
                args.artifact_campaign_dir = Some(write_latest.campaign_dir);
                args.artifact_json = write_latest.json;
            }
            ArtifactSubcommandV1::WriteManifest(write_manifest) => {
                args.artifact_action = Some(ArtifactActionV1::WriteManifest);
                args.artifact_manifest_path = Some(write_manifest.manifest_path);
                args.artifact_payload_schema_name = Some(write_manifest.payload_schema_name);
                args.artifact_created_at = Some(write_manifest.created_at);
                args.artifact_json = write_manifest.json;
            }
            ArtifactSubcommandV1::Prune(prune) => {
                args.artifact_action = Some(ArtifactActionV1::Prune);
                args.artifact_campaign_dir = Some(prune.campaign_dir);
                args.artifact_keep_runs = prune.keep_runs;
                args.artifact_keep_scratch = prune.keep_scratch;
                args.artifact_apply = prune.apply;
                args.artifact_json = prune.json;
            }
        }
        args
    }
}

impl SelfCheckCommandArgs {
    fn into_args(self) -> Args {
        let mut args = Args::compat_defaults();
        args.self_check_ancestor_replay = true;
        args
    }
}

impl CampaignDomainArgs {
    fn apply_to(self, args: &mut Args) {
        args.preset = self.preset;
        args.seed = self.seed;
        args.ascension = self.ascension;
        args.ascension_domain = self.ascension_domain;
        args.player_class = self.player_class;
        args.final_act = self.final_act;
    }
}

impl CampaignBranchingArgs {
    fn apply_to(self, args: &mut Args) {
        args.max_rounds = self.max_rounds;
        args.rounds = self.rounds;
        args.until_round = self.until_round;
        args.until_milestone = self.until_milestone;
        args.milestone_step_rounds = self.milestone_step_rounds;
        args.milestone_max_rounds = self.milestone_max_rounds;
        args.milestone_stop = self.milestone_stop;
        args.round_depth = self.round_depth;
        args.max_active = self.max_active;
        args.max_frozen = self.max_frozen;
        args.max_branches_per_active = self.max_branches_per_active;
        args.boss_relic_axes = self.boss_relic_axes;
        args.retention_profile = self.retention_profile;
        args.max_reward_options = self.max_reward_options;
        args.all_reward_options = self.all_reward_options;
        args.max_campfire_options = self.max_campfire_options;
        args.auto_max_ops = self.auto_max_ops;
        args.experiment_wall_ms = self.experiment_wall_ms;
    }
}

impl CampaignSearchArgs {
    fn apply_to(self, args: &mut Args) {
        args.search_max_nodes = self.search_max_nodes;
        args.search_wall_ms = self.search_wall_ms;
        args.max_hp_loss = self.max_hp_loss;
        args.combat_search_options = self.combat_search_options;
    }
}

impl CampaignCombatRetryArgs {
    fn apply_to(self, args: &mut Args) {
        args.combat_retry = self.combat_retry;
        args.combat_retry_wall_ms = self.combat_retry_wall_ms;
        args.min_acceptable_victory_hp_percent = self.min_acceptable_victory_hp_percent;
    }
}

impl CampaignPrefixArgs {
    fn apply_to(self, args: &mut Args) {
        args.prefix_commands = self.prefix_commands;
        args.no_neow_guidance = self.no_neow_guidance;
    }
}

impl CampaignRunOutputArgs {
    fn apply_to(self, args: &mut Args) {
        args.branch_examples = self.branch_examples;
        args.json = self.json;
        args.report_detail = self.report_detail;
        args.progress = self.progress;
        args.progress_detail = self.progress_detail;
        args.resume = self.resume;
        args.resume_checkpoint = self.resume_checkpoint;
        args.out = self.out;
        args.checkpoint_out = self.checkpoint_out;
        args.auto_capture_combat = self.auto_capture_combat;
        args.auto_capture_root = self.auto_capture_root;
        args.export_outcome_dataset = self.export_outcome_dataset;
        args.export_learning_dataset = self.export_learning_dataset;
        args.export_decision_outcome_dataset = self.export_decision_outcome_dataset;
    }
}

impl CampaignContinuationOutputArgs {
    fn into_run_output_args(self) -> CampaignRunOutputArgs {
        CampaignRunOutputArgs {
            branch_examples: self.branch_examples,
            json: self.json,
            report_detail: self.report_detail,
            progress: self.progress,
            progress_detail: self.progress_detail,
            resume: self.resume,
            resume_checkpoint: self.resume_checkpoint,
            out: self.out,
            checkpoint_out: self.checkpoint_out,
            auto_capture_combat: false,
            auto_capture_root: None,
            export_outcome_dataset: None,
            export_learning_dataset: None,
            export_decision_outcome_dataset: None,
        }
    }
}

impl CampaignCoverageExecuteTargetArgs {
    fn into_continuation_args(self) -> ContinuationArgs {
        ContinuationArgs {
            plan_targeted_continuation: None,
            execute_targeted_continuation: None,
            execute_coverage_gap_continuation: true,
            continuation_effect_before: None,
            continuation_effect_after: None,
            targeted_continuation_limit: 4,
            targeted_continuation_candidates_per_target: 1,
            coverage_gap_limit: self.coverage_gap_limit,
            coverage_gap_candidates_per_decision: self.coverage_gap_candidates_per_decision,
            coverage_gap_bucket: self.coverage_gap_bucket,
            coverage_gap_event_id: self.coverage_gap_event_id,
            coverage_gap_lane: self.coverage_gap_lane,
            coverage_gap_origin_source: self.coverage_gap_origin_source,
            coverage_gap_progress: self.coverage_gap_progress,
            coverage_gap_budget_intent: self.coverage_gap_budget_intent,
            coverage_gap_execution_mode: self.coverage_gap_execution_mode,
        }
    }
}

impl InspectTargetArgs {
    fn apply_to(self, args: &mut Args) {
        args.inspect_checkpoint = self.inspect_checkpoint;
        args.inspect_report = self.inspect_report;
        args.inspect_act = self.inspect_act;
        args.inspect_floor = self.inspect_floor;
        args.inspect_boundary = self.inspect_boundary;
        args.inspect_hp = self.inspect_hp;
        args.inspect_index = self.inspect_index;
    }
}

impl InspectModeArgs {
    fn apply_to(self, args: &mut Args) {
        args.inspect_summary = self.inspect_summary;
        args.inspect_search = self.inspect_search;
        args.inspect_last_auto_combat = self.inspect_last_auto_combat;
        args.inspect_combat_lab = self.inspect_combat_lab;
        args.probe_boss = self.probe_boss;
        args.inspect_shop_evidence = self.inspect_shop_evidence;
        args.inspect_evidence_detail = self.inspect_evidence_detail;
        args.challenge_shop_plans = self.challenge_shop_plans;
        args.inspect_card_reward_evidence = self.inspect_card_reward_evidence;
        args.inspect_campfire_evidence = self.inspect_campfire_evidence;
        args.inspect_deck_mutation = self.inspect_deck_mutation;
        args.inspect_route_evidence = self.inspect_route_evidence;
        args.inspect_decision_observations = self.inspect_decision_observations;
        args.inspect_journal = self.inspect_journal;
        args.inspect_lineage_decisions = self.inspect_lineage_decisions;
        args.inspect_decision_coverage = self.inspect_decision_coverage;
        args.inspect_coverage_gap_milestone_summary = self.inspect_coverage_gap_milestone_summary;
        args.inspect_coverage_gap_target_state = self.inspect_coverage_gap_target_state;
        args.coverage_gap_milestone_target = self.coverage_gap_milestone_target;
        args.coverage_gap_bucket = self.coverage_gap_bucket;
        args.coverage_gap_event_id = self.coverage_gap_event_id;
        args.coverage_gap_lane = self.coverage_gap_lane;
        args.coverage_gap_origin_source = self.coverage_gap_origin_source;
        args.coverage_gap_progress = self.coverage_gap_progress;
        args.inspect_query = self.inspect_query;
        args.inspect_final_boss_combat = self.inspect_final_boss_combat;
    }
}

impl InspectChallengeArgs {
    fn apply_to(self, args: &mut Args) {
        args.final_act = self.final_act;
        args.max_reward_options = self.max_reward_options;
        args.all_reward_options = self.all_reward_options;
        args.max_campfire_options = self.max_campfire_options;
        args.auto_max_ops = self.auto_max_ops;
        args.experiment_wall_ms = self.experiment_wall_ms;
        args.challenge_max_plans = self.challenge_max_plans;
        args.challenge_depth = self.challenge_depth;
        args.challenge_max_branches = self.challenge_max_branches;
    }
}

impl InspectDisplayArgs {
    fn apply_to(self, args: &mut Args) {
        args.branch_examples = self.branch_examples;
    }
}

impl DatasetPathArgs {
    fn apply_to(self, args: &mut Args) {
        args.inspect_checkpoint = self.inspect_checkpoint;
        args.inspect_report = self.inspect_report;
        args.export_outcome_dataset = self.export_outcome_dataset;
        args.analyze_outcome_dataset = self.analyze_outcome_dataset;
        args.analyze_decision_outcome_dataset = self.analyze_decision_outcome_dataset;
        args.probe_learning_readiness = self.probe_learning_readiness;
        args.export_learning_dataset = self.export_learning_dataset;
        args.export_decision_outcome_dataset = self.export_decision_outcome_dataset;
    }
}

impl ContinuationArgs {
    fn apply_to(self, args: &mut Args) {
        args.plan_targeted_continuation = self.plan_targeted_continuation;
        args.execute_targeted_continuation = self.execute_targeted_continuation;
        args.execute_coverage_gap_continuation = self.execute_coverage_gap_continuation;
        args.continuation_effect_before = self.continuation_effect_before;
        args.continuation_effect_after = self.continuation_effect_after;
        args.targeted_continuation_limit = self.targeted_continuation_limit;
        args.targeted_continuation_candidates_per_target =
            self.targeted_continuation_candidates_per_target;
        args.coverage_gap_limit = self.coverage_gap_limit;
        args.coverage_gap_candidates_per_decision = self.coverage_gap_candidates_per_decision;
        args.coverage_gap_bucket = self.coverage_gap_bucket;
        args.coverage_gap_event_id = self.coverage_gap_event_id;
        args.coverage_gap_lane = self.coverage_gap_lane;
        args.coverage_gap_origin_source = self.coverage_gap_origin_source;
        args.coverage_gap_progress = self.coverage_gap_progress;
        args.coverage_gap_budget_intent = self.coverage_gap_budget_intent;
        args.coverage_gap_execution_mode = self.coverage_gap_execution_mode;
    }
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

pub(super) fn parse_cli() -> BranchCampaignCliInputV1 {
    parse_cli_from(std::env::args_os()).unwrap_or_else(|err| err.exit())
}

pub(super) fn parse_cli_from<I, T>(itr: I) -> Result<BranchCampaignCliInputV1, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let argv = itr.into_iter().map(Into::into).collect::<Vec<OsString>>();
    let matches = match CliRootV1::command().try_get_matches_from(argv.clone()) {
        Ok(matches) => matches,
        Err(err) if should_fallback_to_legacy_root_args(&err) => {
            return parse_legacy_cli_from(argv);
        }
        Err(err) => return Err(err),
    };
    let cli = CliRootV1::from_arg_matches(&matches)?;
    let (args, explicit_command) = match cli.command {
        Some(BranchCampaignCliCommandV1::Campaign(args)) => return args.into_cli_input(&matches),
        Some(BranchCampaignCliCommandV1::Run(args)) => {
            (args.into_args(), Some(BranchCampaignExplicitCommandV1::Run))
        }
        Some(BranchCampaignCliCommandV1::Inspect(args)) => (
            args.into_args(),
            Some(BranchCampaignExplicitCommandV1::Inspect),
        ),
        Some(BranchCampaignCliCommandV1::Dataset(args)) => (
            args.into_args(),
            Some(BranchCampaignExplicitCommandV1::Dataset),
        ),
        Some(BranchCampaignCliCommandV1::Continue(args)) => (
            args.into_args(),
            Some(BranchCampaignExplicitCommandV1::Continue),
        ),
        Some(BranchCampaignCliCommandV1::Artifact(args)) => (
            args.into_args(),
            Some(BranchCampaignExplicitCommandV1::Artifact),
        ),
        Some(BranchCampaignCliCommandV1::SelfCheck(args)) => (
            args.into_args(),
            Some(BranchCampaignExplicitCommandV1::SelfCheck),
        ),
        None => return parse_legacy_cli_from(argv),
    };
    normalize_cli_args_from_matches(args, explicit_command, &matches)
}

fn parse_legacy_cli_from(argv: Vec<OsString>) -> Result<BranchCampaignCliInputV1, clap::Error> {
    let legacy_command = <Args as ClapArgs>::augment_args(Command::new("branch_campaign_driver"));
    let matches = legacy_command.try_get_matches_from(argv)?;
    let args = Args::from_arg_matches(&matches)?;
    normalize_cli_args_from_matches(args, None, &matches)
}

fn should_fallback_to_legacy_root_args(err: &clap::Error) -> bool {
    matches!(err.kind(), ErrorKind::UnknownArgument)
}

fn normalize_run_command_args(
    mut args: RunCommandArgs,
    matches: &ArgMatches,
) -> Result<RunCommandArgs, clap::Error> {
    normalize_campaign_domain_args(&mut args.domain, matches)?;
    apply_preset_defaults_to_campaign_cli_parts(
        args.domain.preset,
        &mut args.branching,
        &mut args.search,
        &mut args.output.branch_examples,
        matches,
    );
    Ok(args)
}

fn normalize_continue_command_args(
    mut args: ContinueCommandArgs,
    matches: &ArgMatches,
) -> Result<ContinueCommandArgs, clap::Error> {
    normalize_campaign_domain_args(&mut args.domain, matches)?;
    apply_preset_defaults_to_campaign_cli_parts(
        args.domain.preset,
        &mut args.branching,
        &mut args.search,
        &mut args.output.branch_examples,
        matches,
    );
    Ok(args)
}

fn normalize_coverage_execute_command_args(
    mut args: CampaignCoverageExecuteCommandArgs,
    matches: &ArgMatches,
) -> Result<CampaignCoverageExecuteCommandArgs, clap::Error> {
    normalize_campaign_domain_args(&mut args.domain, matches)?;
    apply_preset_defaults_to_campaign_cli_parts(
        args.domain.preset,
        &mut args.branching,
        &mut args.search,
        &mut args.output.branch_examples,
        matches,
    );
    Ok(args)
}

fn normalize_campaign_domain_args(
    domain: &mut CampaignDomainArgs,
    matches: &ArgMatches,
) -> Result<(), clap::Error> {
    if let Some(ascension_domain) = domain.ascension_domain {
        let domain_ascension = ascension_domain.ascension_level();
        let ascension_was_explicit =
            selected_value_source(matches, "ascension") == Some(ValueSource::CommandLine);
        if ascension_was_explicit && domain.ascension != domain_ascension {
            return Err(clap::Error::raw(
                ErrorKind::ValueValidation,
                format!(
                    "--ascension-domain {:?} implies --ascension {}, but --ascension {} was provided",
                    ascension_domain, domain_ascension, domain.ascension
                ),
            ));
        }
        if !ascension_was_explicit {
            domain.ascension = domain_ascension;
        }
    }
    Ok(())
}

fn apply_preset_defaults_to_campaign_cli_parts(
    preset: Option<BranchCampaignPresetV1>,
    branching: &mut CampaignBranchingArgs,
    search: &mut CampaignSearchArgs,
    branch_examples: &mut usize,
    matches: &ArgMatches,
) {
    let defaults = match preset {
        Some(BranchCampaignPresetV1::Quick) => CampaignPresetDefaults {
            max_rounds: QUICK_PRESET_MAX_ROUNDS,
            round_depth: QUICK_PRESET_ROUND_DEPTH,
            max_active: QUICK_PRESET_MAX_ACTIVE,
            max_frozen: QUICK_PRESET_MAX_FROZEN,
            max_branches_per_active: QUICK_PRESET_MAX_BRANCHES_PER_ACTIVE,
            experiment_wall_ms: QUICK_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: QUICK_PRESET_SEARCH_WALL_MS,
            search_max_nodes: QUICK_PRESET_SEARCH_MAX_NODES,
            branch_examples: QUICK_PRESET_BRANCH_EXAMPLES,
        },
        Some(BranchCampaignPresetV1::Focused) => CampaignPresetDefaults {
            max_rounds: FOCUSED_PRESET_MAX_ROUNDS,
            round_depth: FOCUSED_PRESET_ROUND_DEPTH,
            max_active: FOCUSED_PRESET_MAX_ACTIVE,
            max_frozen: FOCUSED_PRESET_MAX_FROZEN,
            max_branches_per_active: FOCUSED_PRESET_MAX_BRANCHES_PER_ACTIVE,
            experiment_wall_ms: FOCUSED_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: FOCUSED_PRESET_SEARCH_WALL_MS,
            search_max_nodes: FOCUSED_PRESET_SEARCH_MAX_NODES,
            branch_examples: FOCUSED_PRESET_BRANCH_EXAMPLES,
        },
        Some(BranchCampaignPresetV1::Explore) => CampaignPresetDefaults {
            max_rounds: EXPLORE_PRESET_MAX_ROUNDS,
            round_depth: EXPLORE_PRESET_ROUND_DEPTH,
            max_active: EXPLORE_PRESET_MAX_ACTIVE,
            max_frozen: EXPLORE_PRESET_MAX_FROZEN,
            max_branches_per_active: EXPLORE_PRESET_MAX_BRANCHES_PER_ACTIVE,
            experiment_wall_ms: EXPLORE_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: EXPLORE_PRESET_SEARCH_WALL_MS,
            search_max_nodes: EXPLORE_PRESET_SEARCH_MAX_NODES,
            branch_examples: EXPLORE_PRESET_BRANCH_EXAMPLES,
        },
        Some(BranchCampaignPresetV1::Deep) => CampaignPresetDefaults {
            max_rounds: DEEP_PRESET_MAX_ROUNDS,
            round_depth: DEEP_PRESET_ROUND_DEPTH,
            max_active: DEEP_PRESET_MAX_ACTIVE,
            max_frozen: DEEP_PRESET_MAX_FROZEN,
            max_branches_per_active: DEEP_PRESET_MAX_BRANCHES_PER_ACTIVE,
            experiment_wall_ms: DEEP_PRESET_EXPERIMENT_WALL_MS,
            search_wall_ms: DEEP_PRESET_SEARCH_WALL_MS,
            search_max_nodes: DEEP_PRESET_SEARCH_MAX_NODES,
            branch_examples: DEEP_PRESET_BRANCH_EXAMPLES,
        },
        None => return,
    };

    let was_explicit =
        |name| selected_value_source(matches, name) == Some(ValueSource::CommandLine);
    if !was_explicit("max_rounds") {
        branching.max_rounds = defaults.max_rounds;
    }
    if !was_explicit("round_depth") {
        branching.round_depth = defaults.round_depth;
    }
    if !was_explicit("max_active") {
        branching.max_active = defaults.max_active;
    }
    if !was_explicit("max_frozen") {
        branching.max_frozen = defaults.max_frozen;
    }
    if !was_explicit("max_branches_per_active") {
        branching.max_branches_per_active = defaults.max_branches_per_active;
    }
    if !was_explicit("experiment_wall_ms") {
        branching.experiment_wall_ms = defaults.experiment_wall_ms;
    }
    if !was_explicit("search_wall_ms") {
        search.search_wall_ms = defaults.search_wall_ms;
    }
    if !was_explicit("search_max_nodes") {
        search.search_max_nodes = Some(defaults.search_max_nodes);
    }
    if !was_explicit("branch_examples") {
        *branch_examples = defaults.branch_examples;
    }
}

fn normalize_cli_args_from_matches(
    mut args: Args,
    explicit_command: Option<BranchCampaignExplicitCommandV1>,
    matches: &ArgMatches,
) -> Result<BranchCampaignCliInputV1, clap::Error> {
    args.explicit_command = explicit_command;
    if let Some(domain) = args.ascension_domain {
        let domain_ascension = domain.ascension_level();
        let ascension_was_explicit =
            selected_value_source(&matches, "ascension") == Some(ValueSource::CommandLine);
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
        selected_value_source(&matches, name) == Some(ValueSource::CommandLine)
    });
    Ok(match explicit_command {
        Some(command) => BranchCampaignCliInputV1::Explicit { command, args },
        None => BranchCampaignCliInputV1::Legacy(args),
    })
}

#[cfg(test)]
pub(super) fn parse_args_from<I, T>(itr: I) -> Result<Args, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    parse_cli_from(itr).map(BranchCampaignCliInputV1::into_args)
}

fn selected_value_source(matches: &ArgMatches, name: &'static str) -> Option<ValueSource> {
    let mut source = value_source_if_present(matches, name);
    let mut current = matches;
    while let Some((_, sub_matches)) = current.subcommand() {
        if let Some(sub_source) = value_source_if_present(sub_matches, name) {
            source = Some(sub_source);
        }
        current = sub_matches;
    }
    source
}

fn value_source_if_present(matches: &ArgMatches, name: &'static str) -> Option<ValueSource> {
    if matches.ids().any(|id| id.as_str() == name) {
        matches.value_source(name)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_inputs::campaign_config_from_args;

    #[test]
    fn campaign_namespace_artifacts_prune_routes_to_artifact_command() {
        let input = parse_cli_from([
            "branch_campaign_driver",
            "campaign",
            "artifacts",
            "prune",
            "--keep-runs",
            "2",
            "--keep-scratch",
            "1",
        ])
        .expect("campaign artifacts prune should parse");

        assert_eq!(
            input.explicit_command(),
            Some(BranchCampaignExplicitCommandV1::Artifact)
        );
        let args = input.into_args();
        assert_eq!(args.artifact_action, Some(ArtifactActionV1::Prune));
        assert_eq!(args.artifact_keep_runs, 2);
        assert_eq!(args.artifact_keep_scratch, 1);
    }

    #[test]
    fn campaign_namespace_preserves_nested_explicit_preset_overrides() {
        let input = parse_cli_from([
            "branch_campaign_driver",
            "campaign",
            "run",
            "--preset",
            "quick",
            "--max-active",
            "3",
            "--rounds",
            "1",
        ])
        .expect("campaign run should parse");
        let args = input.into_args();
        let config = campaign_config_from_args(&args).expect("config should build");

        assert_eq!(config.max_active, 3);
        assert_eq!(config.max_rounds, 1);
        assert_eq!(config.round_depth, QUICK_PRESET_ROUND_DEPTH);
    }

    #[test]
    fn campaign_namespace_coverage_plan_uses_coverage_request_surface() {
        let input = parse_cli_from([
            "branch_campaign_driver",
            "campaign",
            "coverage",
            "plan",
            "--inspect-report",
            "run.json.gz",
            "--coverage-gap-limit",
            "5",
        ])
        .expect("campaign coverage plan should parse");

        assert_eq!(
            input.explicit_command(),
            Some(BranchCampaignExplicitCommandV1::PlanCoverageGapContinuation)
        );
        let args = input.into_args();
        assert!(args.plan_coverage_gap_continuation);
        assert_eq!(args.inspect_report, Some(PathBuf::from("run.json.gz")));
        assert_eq!(args.coverage_gap_limit, 5);
        assert!(args.export_outcome_dataset.is_none());
    }

    #[test]
    fn campaign_namespace_coverage_execute_uses_coverage_request_surface() {
        let input = parse_cli_from([
            "branch_campaign_driver",
            "campaign",
            "coverage",
            "execute",
            "--resume",
            "run.json.gz",
            "--resume-checkpoint",
            "checkpoint.json.gz",
            "--coverage-gap-limit",
            "3",
        ])
        .expect("campaign coverage execute should parse");

        assert_eq!(
            input.explicit_command(),
            Some(BranchCampaignExplicitCommandV1::ExecuteCoverageGapContinuation)
        );
        let args = input.into_args();
        assert!(args.execute_coverage_gap_continuation);
        assert_eq!(args.resume, Some(PathBuf::from("run.json.gz")));
        assert_eq!(
            args.resume_checkpoint,
            Some(PathBuf::from("checkpoint.json.gz"))
        );
        assert_eq!(args.coverage_gap_limit, 3);
        assert!(args.execute_targeted_continuation.is_none());
    }
}
