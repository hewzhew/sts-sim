use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::{Args, BoundarySite, BranchStatus, Owner, RunContract, TerminalOutcome};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunSliceResult {
    pub contract: RunContract,
    pub request_kind: RunSliceRequestKind,
    pub generation_start: usize,
    pub generation_end: usize,
    pub next_branch_id: usize,
    pub stop: RunStop,
    pub frontier: FrontierSummary,
    pub selected_branch: Option<BranchSummary>,
    pub budget: SliceBudgetSummary,
    pub combat_search: CombatSearchTelemetrySummary,
    pub primary_search: PrimarySearchOutcomeSummary,
    pub artifacts: ArtifactWriteSummary,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunSliceRequestKind {
    Start,
    ResumeFrontier,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RunStop {
    Real(RealStop),
    SoftPause(SoftPause),
    FrontierExhausted(FrontierExhausted),
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum RealStop {
    Terminal {
        generation: usize,
        branch_id: usize,
        outcome: TerminalOutcome,
    },
    ObjectiveSatisfied {
        generation: usize,
        reason: String,
    },
    AutomationGap {
        generation: usize,
        branch_id: usize,
        boundary: String,
        site: BoundarySite,
    },
    CombatGap {
        generation: usize,
        branch_id: usize,
        boundary: String,
        reason: String,
    },
    OperationBudgetExhausted {
        generation: usize,
        branch_id: usize,
        boundary: String,
        reason: String,
    },
    BudgetGap {
        generation: usize,
        branch_id: usize,
        boundary: String,
        reason: String,
    },
    ApplyFailed {
        generation: usize,
        branch_id: usize,
        reason: String,
    },
    AdvanceFailed {
        generation: usize,
        branch_id: usize,
        reason: String,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum SoftPause {
    GenerationLimit {
        generation: usize,
        frontier_running_count: usize,
    },
    SliceDeadline {
        generation: usize,
        frontier_running_count: usize,
    },
    AwaitingAutoBoundary {
        generation: usize,
        frontier_running_count: usize,
    },
    SearchBudgetCappedBeforeGeneration {
        generation: usize,
        frontier_running_count: usize,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum FrontierExhausted {
    NoRunningBranches { generation: usize },
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct FrontierSummary {
    pub total_count: usize,
    pub running_count: usize,
    pub expandable_count: usize,
    pub terminal_count: usize,
    pub gap_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BranchSummary {
    pub branch_id: usize,
    pub parent_id: Option<usize>,
    pub status_kind: BranchStatusKind,
    pub boundary: Option<String>,
    pub owner: Option<String>,
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum BranchStatusKind {
    Running,
    AwaitingAuto,
    Terminal,
    AutomationGap,
    CombatGap,
    OperationBudgetExhausted,
    BudgetGap,
    ApplyFailed,
    AdvanceFailed,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SliceBudgetSummary {
    pub slice_ms: Option<u64>,
    pub remaining_ms: Option<u64>,
    pub elapsed_ms: u64,
    pub search_budget_was_capped: bool,
    pub boss_budget_was_capped: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchTelemetrySummary {
    pub attempt_count: u64,
    pub complete_win_count: u64,
    pub terminal_win_count: u64,
    pub nodes_expanded: u64,
    pub total_us: u64,
    pub timing: CombatSearchTimingSummary,
    pub by_source: Vec<CombatSearchTelemetrySourceSummary>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchTimingSummary {
    pub rollout_us: u64,
    pub expansion_us: u64,
    pub engine_step_us: u64,
    pub pre_expand_us: u64,
    pub frontier_pop_us: u64,
    pub child_bookkeeping_us: u64,
    pub turn_plan_seed_us: u64,
    pub shadow_audit_us: u64,
    pub root_turn_plan_diag_us: u64,
    pub unattributed_us: u64,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchTelemetrySourceSummary {
    pub source: String,
    pub attempt_count: u64,
    pub complete_win_count: u64,
    pub terminal_win_count: u64,
    pub nodes_expanded: u64,
    pub total_us: u64,
    pub timing: CombatSearchTimingSummary,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PrimarySearchOutcomeSummary {
    pub status: String,
    pub profile: PrimarySearchProfileSummary,
    pub telemetry: PrimarySearchTelemetrySummary,
    pub accepted_line: Option<PrimarySearchLineSummary>,
    pub best_complete_line: Option<PrimarySearchLineSummary>,
    pub best_partial_line: Option<PrimarySearchLineSummary>,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PrimarySearchProfileSummary {
    pub profile_id: Option<String>,
    pub stakes: Option<String>,
    pub max_nodes: Option<usize>,
    pub wall_ms: Option<u64>,
    pub potion_policy: Option<String>,
    pub max_potions_used: Option<u32>,
    pub internal_no_win_rescue_enabled: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct PrimarySearchTelemetrySummary {
    pub elapsed_ms: Option<u64>,
    pub deadline_hit: Option<bool>,
    pub expanded_nodes: Option<u64>,
    pub terminal_wins: Option<u64>,
    pub us_per_node: Option<u64>,
    pub first_win_node: Option<u64>,
    pub first_win_ms: Option<u64>,
    pub first_accepted_node: Option<u64>,
    pub first_accepted_ms: Option<u64>,
    pub rollout_us: Option<u64>,
    pub expansion_us: Option<u64>,
    pub transition_us: Option<u64>,
    pub rollout_pct: Option<u64>,
    pub expansion_pct: Option<u64>,
    pub transition_pct: Option<u64>,
    pub diagnostic_pct: Option<u64>,
    pub unattributed_pct: Option<u64>,
    pub selected_first_action: Option<String>,
    pub top_root_actions: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PrimarySearchLineSummary {
    pub terminal: String,
    pub line_len: usize,
    pub final_player_hp: i32,
    pub hp_delta: i32,
    pub potions_used: u32,
    pub first_action_label: Option<String>,
    pub first_action_kind: Option<String>,
}

impl CombatSearchTelemetrySummary {
    pub fn record_attempt(
        &mut self,
        source: impl Into<String>,
        complete_win: bool,
        terminal_wins: u64,
        nodes_expanded: u64,
        total_us: u64,
    ) {
        self.record_attempt_with_timing(
            source,
            complete_win,
            terminal_wins,
            nodes_expanded,
            total_us,
            CombatSearchTimingSummary::default(),
        );
    }

    pub fn record_attempt_with_timing(
        &mut self,
        source: impl Into<String>,
        complete_win: bool,
        terminal_wins: u64,
        nodes_expanded: u64,
        total_us: u64,
        timing: CombatSearchTimingSummary,
    ) {
        self.attempt_count += 1;
        self.complete_win_count += u64::from(complete_win);
        self.terminal_win_count += terminal_wins;
        self.nodes_expanded += nodes_expanded;
        self.total_us += total_us;
        self.timing.merge(&timing);
        self.record_source_attempt(
            source.into(),
            complete_win,
            terminal_wins,
            nodes_expanded,
            total_us,
            timing,
        );
    }

    pub fn merge(&mut self, other: Self) {
        self.attempt_count += other.attempt_count;
        self.complete_win_count += other.complete_win_count;
        self.terminal_win_count += other.terminal_win_count;
        self.nodes_expanded += other.nodes_expanded;
        self.total_us += other.total_us;
        self.timing.merge(&other.timing);
        for source in other.by_source {
            self.record_source_summary(source);
        }
    }

    fn record_source_attempt(
        &mut self,
        source: String,
        complete_win: bool,
        terminal_wins: u64,
        nodes_expanded: u64,
        total_us: u64,
        timing: CombatSearchTimingSummary,
    ) {
        self.record_source_summary(CombatSearchTelemetrySourceSummary {
            source,
            attempt_count: 1,
            complete_win_count: u64::from(complete_win),
            terminal_win_count: terminal_wins,
            nodes_expanded,
            total_us,
            timing,
        });
    }

    fn record_source_summary(&mut self, source_summary: CombatSearchTelemetrySourceSummary) {
        if let Some(existing) = self
            .by_source
            .iter_mut()
            .find(|existing| existing.source == source_summary.source)
        {
            existing.attempt_count += source_summary.attempt_count;
            existing.complete_win_count += source_summary.complete_win_count;
            existing.terminal_win_count += source_summary.terminal_win_count;
            existing.nodes_expanded += source_summary.nodes_expanded;
            existing.total_us += source_summary.total_us;
            existing.timing.merge(&source_summary.timing);
        } else {
            self.by_source.push(source_summary);
            self.by_source
                .sort_by(|left, right| left.source.cmp(&right.source));
        }
    }
}

impl CombatSearchTimingSummary {
    pub fn merge(&mut self, other: &Self) {
        self.rollout_us += other.rollout_us;
        self.expansion_us += other.expansion_us;
        self.engine_step_us += other.engine_step_us;
        self.pre_expand_us += other.pre_expand_us;
        self.frontier_pop_us += other.frontier_pop_us;
        self.child_bookkeeping_us += other.child_bookkeeping_us;
        self.turn_plan_seed_us += other.turn_plan_seed_us;
        self.shadow_audit_us += other.shadow_audit_us;
        self.root_turn_plan_diag_us += other.root_turn_plan_diag_us;
        self.unattributed_us += other.unattributed_us;
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Manifest,
    Frontier,
    Result,
    Path,
    Summary,
    Terminal,
    CombatCase,
    AcceptedCombatDiagnostic,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactRef {
    pub kind: ArtifactKind,
    pub path: PathBuf,
    pub schema: String,
    pub created_by: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct ArtifactWriteSummary {
    pub manifest_written: bool,
    pub frontier_written: bool,
    pub result_written: bool,
    pub path_written: bool,
    pub summary_written: bool,
    pub terminal_written: bool,
    pub combat_case_written: bool,
    pub manifest_ref: Option<ArtifactRef>,
    pub frontier_ref: Option<ArtifactRef>,
    pub result_ref: Option<ArtifactRef>,
    pub path_ref: Option<ArtifactRef>,
    pub summary_ref: Option<ArtifactRef>,
    pub terminal_ref: Option<ArtifactRef>,
    pub combat_case_ref: Option<ArtifactRef>,
    #[serde(default)]
    pub accepted_combat_diagnostic_refs: Vec<ArtifactRef>,
}

impl ArtifactWriteSummary {
    pub fn merge(&mut self, other: Self) {
        self.manifest_written |= other.manifest_written;
        self.frontier_written |= other.frontier_written;
        self.result_written |= other.result_written;
        self.path_written |= other.path_written;
        self.summary_written |= other.summary_written;
        self.terminal_written |= other.terminal_written;
        self.combat_case_written |= other.combat_case_written;
        self.manifest_ref = other.manifest_ref.or(self.manifest_ref.take());
        self.frontier_ref = other.frontier_ref.or(self.frontier_ref.take());
        self.result_ref = other.result_ref.or(self.result_ref.take());
        self.path_ref = other.path_ref.or(self.path_ref.take());
        self.summary_ref = other.summary_ref.or(self.summary_ref.take());
        self.terminal_ref = other.terminal_ref.or(self.terminal_ref.take());
        self.combat_case_ref = other.combat_case_ref.or(self.combat_case_ref.take());
        self.accepted_combat_diagnostic_refs
            .extend(other.accepted_combat_diagnostic_refs);
    }

    pub fn frontier_checkpoint_at(path: impl Into<PathBuf>) -> Self {
        Self::single_ref(ArtifactRef::new(
            ArtifactKind::Frontier,
            path,
            "branch_tiny_frontier_checkpoint",
            "owner_audit_runtime",
        ))
    }

    pub fn single_ref(artifact: ArtifactRef) -> Self {
        let mut summary = Self::default();
        summary.record_ref(artifact);
        summary
    }

    pub fn record_ref(&mut self, artifact: ArtifactRef) {
        match artifact.kind {
            ArtifactKind::Manifest => {
                self.manifest_written = true;
                self.manifest_ref = Some(artifact);
            }
            ArtifactKind::Frontier => {
                self.frontier_written = true;
                self.frontier_ref = Some(artifact);
            }
            ArtifactKind::Result => {
                self.result_written = true;
                self.result_ref = Some(artifact);
            }
            ArtifactKind::Path => {
                self.path_written = true;
                self.path_ref = Some(artifact);
            }
            ArtifactKind::Summary => {
                self.summary_written = true;
                self.summary_ref = Some(artifact);
            }
            ArtifactKind::Terminal => {
                self.terminal_written = true;
                self.terminal_ref = Some(artifact);
            }
            ArtifactKind::CombatCase => {
                self.combat_case_written = true;
                self.combat_case_ref = Some(artifact);
            }
            ArtifactKind::AcceptedCombatDiagnostic => {
                self.accepted_combat_diagnostic_refs.push(artifact);
            }
        }
    }

    pub fn refs(&self) -> Vec<ArtifactRef> {
        [
            self.manifest_ref.clone(),
            self.frontier_ref.clone(),
            self.result_ref.clone(),
            self.path_ref.clone(),
            self.summary_ref.clone(),
            self.terminal_ref.clone(),
            self.combat_case_ref.clone(),
        ]
        .into_iter()
        .flatten()
        .chain(self.accepted_combat_diagnostic_refs.iter().cloned())
        .collect()
    }
}

impl ArtifactRef {
    pub fn new(
        kind: ArtifactKind,
        path: impl Into<PathBuf>,
        schema: impl Into<String>,
        created_by: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            path: path.into(),
            schema: schema.into(),
            created_by: created_by.into(),
        }
    }
}

#[cfg(test)]
mod accepted_combat_diagnostic_tests {
    use super::*;

    #[test]
    fn artifact_summary_retains_multiple_accepted_combat_diagnostic_refs() {
        let mut summary = ArtifactWriteSummary::default();
        summary.record_ref(ArtifactRef::new(
            ArtifactKind::AcceptedCombatDiagnostic,
            "one.capture.json",
            "CombatCaptureV1",
            "owner_audit_runtime",
        ));
        summary.record_ref(ArtifactRef::new(
            ArtifactKind::AcceptedCombatDiagnostic,
            "one.evidence.json",
            "accepted_high_loss_combat_evidence_v1",
            "owner_audit_runtime",
        ));
        summary.record_ref(ArtifactRef::new(
            ArtifactKind::AcceptedCombatDiagnostic,
            "two.evidence.json",
            "accepted_high_loss_combat_evidence_v2",
            "owner_audit_runtime",
        ));

        assert_eq!(summary.accepted_combat_diagnostic_refs.len(), 3);
        assert_eq!(summary.refs().len(), 3);
        assert_eq!(
            summary.accepted_combat_diagnostic_refs[1].schema,
            "accepted_high_loss_combat_evidence_v1"
        );
        assert_eq!(
            summary.accepted_combat_diagnostic_refs[2].schema,
            "accepted_high_loss_combat_evidence_v2"
        );
    }
}

impl RunSliceResult {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        args: Args,
        request_kind: RunSliceRequestKind,
        generation_start: usize,
        generation_end: usize,
        next_branch_id: usize,
        stop: RunStop,
        frontier: FrontierSummary,
        remaining_ms: Option<u64>,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            contract: RunContract::from_args(args),
            request_kind,
            generation_start,
            generation_end,
            next_branch_id,
            stop,
            frontier,
            selected_branch: None,
            budget: SliceBudgetSummary {
                slice_ms: args.wall_ms,
                remaining_ms,
                elapsed_ms,
                search_budget_was_capped: args.wall_capped_search_budget,
                boss_budget_was_capped: args.wall_capped_boss_budget,
            },
            combat_search: CombatSearchTelemetrySummary::default(),
            primary_search: PrimarySearchOutcomeSummary::default(),
            artifacts: ArtifactWriteSummary::default(),
        }
    }

    pub fn with_selected_branch_summary(mut self, branch: BranchSummary) -> Self {
        self.selected_branch = Some(branch);
        self
    }

    pub fn with_combat_search_telemetry(
        mut self,
        combat_search: CombatSearchTelemetrySummary,
    ) -> Self {
        self.combat_search = combat_search;
        self
    }

    pub fn with_primary_search_outcome(
        mut self,
        primary_search: PrimarySearchOutcomeSummary,
    ) -> Self {
        self.primary_search = primary_search;
        self
    }

    pub fn with_artifacts(mut self, artifacts: ArtifactWriteSummary) -> Self {
        self.artifacts = artifacts;
        self
    }
}

impl BranchSummary {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        branch_id: usize,
        parent_id: Option<usize>,
        status: &BranchStatus,
        act: u8,
        floor: i32,
        hp: i32,
        max_hp: i32,
        gold: i32,
        deck_size: usize,
    ) -> Self {
        Self {
            branch_id,
            parent_id,
            status_kind: BranchStatusKind::from_status(status),
            boundary: status_boundary(status),
            owner: status_owner(status),
            act,
            floor,
            hp,
            max_hp,
            gold,
            deck_size,
        }
    }
}

impl BranchStatusKind {
    fn from_status(status: &BranchStatus) -> Self {
        match status {
            BranchStatus::Running { .. } => Self::Running,
            BranchStatus::AwaitingAuto { .. } => Self::AwaitingAuto,
            BranchStatus::Terminal(_) => Self::Terminal,
            BranchStatus::AutomationGap { .. } => Self::AutomationGap,
            BranchStatus::CombatGap { .. } => Self::CombatGap,
            BranchStatus::OperationBudgetExhausted { .. } => Self::OperationBudgetExhausted,
            BranchStatus::BudgetGap { .. } => Self::BudgetGap,
            BranchStatus::ApplyFailed(_) => Self::ApplyFailed,
            BranchStatus::AdvanceFailed(_) => Self::AdvanceFailed,
        }
    }
}

fn status_boundary(status: &BranchStatus) -> Option<String> {
    match status {
        BranchStatus::Running { boundary, .. }
        | BranchStatus::AwaitingAuto { boundary, .. }
        | BranchStatus::AutomationGap { boundary, .. }
        | BranchStatus::CombatGap { boundary, .. }
        | BranchStatus::OperationBudgetExhausted { boundary, .. }
        | BranchStatus::BudgetGap { boundary, .. } => Some(boundary.clone()),
        BranchStatus::Terminal(_)
        | BranchStatus::ApplyFailed(_)
        | BranchStatus::AdvanceFailed(_) => None,
    }
}

fn status_owner(status: &BranchStatus) -> Option<String> {
    match status {
        BranchStatus::Running { owner, .. } => Some(owner_label(*owner).to_string()),
        _ => None,
    }
}

fn owner_label(owner: Owner) -> &'static str {
    match owner {
        Owner::NeowStart => "neow_start",
        Owner::CardReward => "card_reward",
        Owner::BossRelic => "boss_relic",
        Owner::Event(_) => "event",
        Owner::RewardTiny => "reward_tiny",
        Owner::ShopTiny => "shop_tiny",
        Owner::Campfire => "campfire",
        Owner::RunChoice => "run_choice",
    }
}

impl RunStop {
    pub fn from_stopped_branch_status(
        generation: usize,
        branch_id: usize,
        status: &BranchStatus,
    ) -> Option<Self> {
        Some(match status {
            BranchStatus::Terminal(outcome) => Self::Real(RealStop::Terminal {
                generation,
                branch_id,
                outcome: *outcome,
            }),
            BranchStatus::AutomationGap { boundary, site } => Self::Real(RealStop::AutomationGap {
                generation,
                branch_id,
                boundary: boundary.clone(),
                site: *site,
            }),
            BranchStatus::CombatGap { boundary, reason } => Self::Real(RealStop::CombatGap {
                generation,
                branch_id,
                boundary: boundary.clone(),
                reason: reason.clone(),
            }),
            BranchStatus::OperationBudgetExhausted { boundary, reason } => {
                Self::Real(RealStop::OperationBudgetExhausted {
                    generation,
                    branch_id,
                    boundary: boundary.clone(),
                    reason: reason.clone(),
                })
            }
            BranchStatus::BudgetGap { boundary, reason } => Self::Real(RealStop::BudgetGap {
                generation,
                branch_id,
                boundary: boundary.clone(),
                reason: reason.clone(),
            }),
            BranchStatus::ApplyFailed(reason) => Self::Real(RealStop::ApplyFailed {
                generation,
                branch_id,
                reason: reason.clone(),
            }),
            BranchStatus::AdvanceFailed(reason) => Self::Real(RealStop::AdvanceFailed {
                generation,
                branch_id,
                reason: reason.clone(),
            }),
            BranchStatus::Running { .. } | BranchStatus::AwaitingAuto { .. } => {
                return None;
            }
        })
    }
}

impl FrontierSummary {
    pub fn from_statuses<'a>(statuses: impl IntoIterator<Item = &'a BranchStatus>) -> Self {
        let mut summary = Self::default();
        for status in statuses {
            summary.total_count += 1;
            if status.is_resumable() {
                summary.running_count += 1;
            }
            if status.is_expandable_now() {
                summary.expandable_count += 1;
            }
            if matches!(status, BranchStatus::Terminal(_)) {
                summary.terminal_count += 1;
            }
            if matches!(
                status,
                BranchStatus::AutomationGap { .. }
                    | BranchStatus::CombatGap { .. }
                    | BranchStatus::OperationBudgetExhausted { .. }
                    | BranchStatus::BudgetGap { .. }
                    | BranchStatus::ApplyFailed(_)
                    | BranchStatus::AdvanceFailed(_)
            ) {
                summary.gap_count += 1;
            }
        }
        summary
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::{default_branch_args, BoundarySite, BranchStatus, Owner};

    #[test]
    fn run_stop_classifies_public_branch_status() {
        let status = BranchStatus::CombatGap {
            boundary: "A2F32 Combat".to_string(),
            reason: "no win".to_string(),
        };

        let stop = RunStop::from_stopped_branch_status(7, 42, &status).unwrap();

        assert_eq!(
            stop,
            RunStop::Real(RealStop::CombatGap {
                generation: 7,
                branch_id: 42,
                boundary: "A2F32 Combat".to_string(),
                reason: "no win".to_string(),
            })
        );
    }

    #[test]
    fn frontier_summary_counts_public_branch_statuses() {
        let statuses = [
            BranchStatus::Running {
                boundary: "Reward".to_string(),
                owner: Owner::CardReward,
            },
            BranchStatus::AutomationGap {
                boundary: "Event".to_string(),
                site: BoundarySite::Unknown,
            },
        ];

        let summary = FrontierSummary::from_statuses(statuses.iter());

        assert_eq!(summary.total_count, 2);
        assert_eq!(summary.running_count, 1);
        assert_eq!(summary.expandable_count, 1);
        assert_eq!(summary.gap_count, 1);
    }

    #[test]
    fn run_slice_result_is_structured_runtime_output() {
        let mut args = default_branch_args(12);
        args.wall_ms = Some(13);
        args.wall_capped_search_budget = true;

        let result = RunSliceResult::new(
            args,
            RunSliceRequestKind::Start,
            1,
            2,
            99,
            RunStop::SoftPause(SoftPause::SliceDeadline {
                generation: 2,
                frontier_running_count: 1,
            }),
            FrontierSummary {
                total_count: 2,
                running_count: 1,
                expandable_count: 1,
                terminal_count: 0,
                gap_count: 1,
            },
            Some(3),
            10,
        );

        assert_eq!(result.contract.game.seed, 12);
        assert_eq!(result.request_kind, RunSliceRequestKind::Start);
        assert_eq!(result.budget.slice_ms, Some(13));
        assert!(result.budget.search_budget_was_capped);
    }
}
