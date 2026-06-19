use clap::error::ErrorKind;
use clap::parser::ValueSource;
use clap::{ArgMatches, Args as ClapArgs, CommandFactory, FromArgMatches, Parser, Subcommand};
use std::path::PathBuf;

use super::{
    apply_preset_defaults, Args, BranchCampaignAscensionDomainArgV1,
    BranchCampaignCombatRetryArgV1, BranchCampaignExplicitCommandV1, BranchCampaignPresetV1,
    CampaignProgressDetailArg, CampaignReportDetailArg,
};

#[derive(Debug, Parser)]
#[command(
    name = "branch_campaign_driver",
    about = "Advance a small campaign of noncombat branches until victory, budget, or strategy boundary"
)]
struct CliRootV1 {
    #[command(subcommand)]
    command: Option<BranchCampaignCliCommandV1>,

    #[command(flatten)]
    legacy_args: Args,
}

#[derive(Debug, Subcommand)]
enum BranchCampaignCliCommandV1 {
    #[command(about = "Run or resume a branch campaign")]
    Run(RunCommandArgs),
    #[command(about = "Inspect campaign checkpoints and report artifacts")]
    Inspect(InspectCommandArgs),
    #[command(about = "Export or analyze campaign outcome datasets")]
    Dataset(DatasetCommandArgs),
    #[command(about = "Plan, execute, or compare targeted sibling continuations")]
    Continue(ContinueCommandArgs),
    #[command(
        name = "self-check",
        about = "Run internal campaign driver self-checks"
    )]
    SelfCheck(SelfCheckCommandArgs),
}

#[derive(Debug)]
pub(super) enum BranchCampaignCliInputV1 {
    Legacy(Args),
    Explicit {
        command: BranchCampaignExplicitCommandV1,
        args: Args,
    },
}

impl BranchCampaignCliInputV1 {
    pub(super) fn args(&self) -> &Args {
        match self {
            Self::Legacy(args) => args,
            Self::Explicit { args, .. } => args,
        }
    }

    #[cfg(test)]
    pub(super) fn into_args(self) -> Args {
        match self {
            Self::Legacy(args) => args,
            Self::Explicit { args, .. } => args,
        }
    }

    pub(super) fn explicit_command(&self) -> Option<BranchCampaignExplicitCommandV1> {
        match self {
            Self::Legacy(_) => None,
            Self::Explicit { command, .. } => Some(*command),
        }
    }
}

// Public CLI surfaces are command-specific. Keep new flags in these structs
// unless they truly belong to the legacy top-level compatibility parser.
#[derive(Debug, ClapArgs)]
struct RunCommandArgs {
    #[command(flatten)]
    domain: CampaignDomainArgs,

    #[command(flatten)]
    branching: CampaignBranchingArgs,

    #[command(flatten)]
    search: CampaignSearchArgs,

    #[command(flatten)]
    retry: CampaignCombatRetryArgs,

    #[command(flatten)]
    prefix: CampaignPrefixArgs,

    #[command(flatten)]
    output: CampaignRunOutputArgs,
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
struct DatasetCommandArgs {
    #[command(flatten)]
    paths: DatasetPathArgs,
}

#[derive(Debug, ClapArgs)]
struct ContinueCommandArgs {
    #[command(flatten)]
    domain: CampaignDomainArgs,

    #[command(flatten)]
    branching: CampaignBranchingArgs,

    #[command(flatten)]
    search: CampaignSearchArgs,

    #[command(flatten)]
    retry: CampaignCombatRetryArgs,

    #[command(flatten)]
    prefix: CampaignPrefixArgs,

    #[command(flatten)]
    output: CampaignRunOutputArgs,

    #[command(flatten)]
    continuation: ContinuationArgs,
}

#[derive(Debug, ClapArgs)]
struct SelfCheckCommandArgs {}

#[derive(Debug, ClapArgs)]
struct CampaignDomainArgs {
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
}

#[derive(Debug, ClapArgs)]
struct CampaignBranchingArgs {
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
}

#[derive(Debug, ClapArgs)]
struct CampaignSearchArgs {
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
}

#[derive(Debug, ClapArgs)]
struct CampaignCombatRetryArgs {
    #[arg(long, value_enum, default_value_t = BranchCampaignCombatRetryArgV1::OnStall)]
    combat_retry: BranchCampaignCombatRetryArgV1,

    #[arg(
        long,
        help = "Override the wall-clock budget used by the one-shot combat retry pass"
    )]
    combat_retry_wall_ms: Option<u64>,

    #[arg(long, default_value_t = 20)]
    min_acceptable_victory_hp_percent: u8,
}

#[derive(Debug, ClapArgs)]
struct CampaignPrefixArgs {
    #[arg(long = "prefix", value_name = "COMMAND")]
    prefix_commands: Vec<String>,

    #[arg(long)]
    no_neow_guidance: bool,
}

#[derive(Debug, ClapArgs)]
struct CampaignRunOutputArgs {
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
        long = "export-outcome-dataset",
        value_name = "PATH",
        help = "Write BranchOutcomeRecordV1 JSONL from the resulting campaign report"
    )]
    export_outcome_dataset: Option<PathBuf>,

    #[arg(
        long = "export-learning-dataset",
        value_name = "PATH",
        help = "Write LearningBranchSampleV1 JSONL from the resulting campaign report"
    )]
    export_learning_dataset: Option<PathBuf>,

    #[arg(
        long = "export-decision-outcome-dataset",
        value_name = "PATH",
        help = "Write LearningDecisionOutcomeSampleV1 JSONL from the resulting campaign report"
    )]
    export_decision_outcome_dataset: Option<PathBuf>,
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
        help = "Pair --inspect-checkpoint with a BranchCampaignV1 report for active/frozen/abandoned labels"
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
struct DatasetPathArgs {
    #[arg(
        long = "inspect-checkpoint",
        value_name = "PATH",
        help = "Optional BranchCampaignCheckpointV2 sidecar for dataset exports"
    )]
    inspect_checkpoint: Option<PathBuf>,

    #[arg(
        long = "inspect-report",
        value_name = "PATH",
        help = "BranchCampaignV1 report used by dataset exports"
    )]
    inspect_report: Option<PathBuf>,

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

#[derive(Debug, ClapArgs)]
struct ContinuationArgs {
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
}

impl Args {
    fn compat_defaults() -> Self {
        Self {
            explicit_command: None,
            preset: None,
            seed: 1,
            ascension: 0,
            ascension_domain: None,
            player_class: "ironclad".to_string(),
            final_act: false,
            max_rounds: 8,
            round_depth: 1,
            max_active: 8,
            max_frozen: 32,
            max_branches_per_active: 12,
            active_lineage_diversity: 0,
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
            challenge_shop_plans: false,
            challenge_max_plans: 6,
            challenge_depth: 4,
            challenge_max_branches: 12,
            inspect_card_reward_evidence: false,
            inspect_campfire_evidence: false,
            inspect_deck_mutation: false,
            inspect_route_evidence: false,
            inspect_final_boss_combat: false,
            export_outcome_dataset: None,
            analyze_outcome_dataset: None,
            analyze_decision_outcome_dataset: None,
            probe_learning_readiness: None,
            plan_targeted_continuation: None,
            execute_targeted_continuation: None,
            continuation_effect_before: None,
            continuation_effect_after: None,
            targeted_continuation_limit: 4,
            targeted_continuation_candidates_per_target: 1,
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

impl DatasetCommandArgs {
    fn into_args(self) -> Args {
        let mut args = Args::compat_defaults();
        self.paths.apply_to(&mut args);
        args
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
        args.round_depth = self.round_depth;
        args.max_active = self.max_active;
        args.max_frozen = self.max_frozen;
        args.max_branches_per_active = self.max_branches_per_active;
        args.active_lineage_diversity = self.active_lineage_diversity;
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
        args.export_outcome_dataset = self.export_outcome_dataset;
        args.export_learning_dataset = self.export_learning_dataset;
        args.export_decision_outcome_dataset = self.export_decision_outcome_dataset;
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
        args.challenge_shop_plans = self.challenge_shop_plans;
        args.inspect_card_reward_evidence = self.inspect_card_reward_evidence;
        args.inspect_campfire_evidence = self.inspect_campfire_evidence;
        args.inspect_deck_mutation = self.inspect_deck_mutation;
        args.inspect_route_evidence = self.inspect_route_evidence;
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
        args.continuation_effect_before = self.continuation_effect_before;
        args.continuation_effect_after = self.continuation_effect_after;
        args.targeted_continuation_limit = self.targeted_continuation_limit;
        args.targeted_continuation_candidates_per_target =
            self.targeted_continuation_candidates_per_target;
    }
}

pub(super) fn parse_cli() -> BranchCampaignCliInputV1 {
    parse_cli_from(std::env::args_os()).unwrap_or_else(|err| err.exit())
}

pub(super) fn parse_cli_from<I, T>(itr: I) -> Result<BranchCampaignCliInputV1, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let matches = CliRootV1::command().try_get_matches_from(itr)?;
    let cli = CliRootV1::from_arg_matches(&matches)?;
    let (mut args, explicit_command) = match cli.command {
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
        Some(BranchCampaignCliCommandV1::SelfCheck(args)) => (
            args.into_args(),
            Some(BranchCampaignExplicitCommandV1::SelfCheck),
        ),
        None => (cli.legacy_args, None),
    };
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
    matches
        .subcommand()
        .and_then(|(_, sub_matches)| sub_matches.value_source(name))
        .or_else(|| matches.value_source(name))
}
