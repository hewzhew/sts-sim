use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{
    decide_manifest_reuse, Args, ArtifactRef, BranchArtifactStore, CapsuleReuseDecision,
    CombatSearchTelemetrySummary, OwnerAuditRuntime, OwnerAuditSliceRequest, PanelLedgerEvent,
    PrimarySearchOutcomeSummary, RunContract, RunSliceResult, SourceIdentity,
};

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
pub struct PanelSeedArtifacts {
    pub manifest: Option<Value>,
    pub result_exists: bool,
    pub frontier_exists: bool,
    pub terminal_exists: bool,
    pub summary_exists: bool,
    pub capsule_ledger_exists: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelIdentityStatus {
    Missing,
    Exact,
    Unknown,
    Incompatible,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelReuseDecision {
    CreateNewCapsule,
    ReuseRealStop,
    ContinueSoftPause,
    FreshReplacedCapsule,
    RejectUnknownIdentity,
    RejectIncompatibleIdentity,
    RejectIncompleteCapsule,
    RejectMalformedCapsule,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelSeedAction {
    StartNew,
    ContinueCapsule,
    ReuseRealStop,
    RejectCapsule,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelRowStatus {
    Scheduled,
    RealStopped,
    SoftPaused,
    ReusedRealStop,
    Skipped,
    ToolFailed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PanelSeedDecision {
    pub identity_status: PanelIdentityStatus,
    pub reuse_decision: PanelReuseDecision,
    pub artifact_facts: PanelArtifactFacts,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelSeedRequest {
    pub seed: u64,
    pub capsule_path: PathBuf,
    pub artifacts: Result<PanelSeedArtifacts, String>,
    pub artifact_facts: PanelArtifactFacts,
    pub contract: RunContract,
    pub source_identity: SourceIdentity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PanelSeedResolution {
    pub seed: u64,
    pub capsule_path: PathBuf,
    pub decision: PanelSeedDecision,
    pub read_error: Option<String>,
}

pub struct PanelScheduler;

pub struct PanelSmokeRunner;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PanelRunMode {
    Smoke,
    Continue,
    Drain,
    Compare,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PanelRunOptions {
    pub mode: PanelRunMode,
    pub max_slices: usize,
    pub fresh: bool,
}

impl PanelRunOptions {
    pub fn smoke(max_slices: usize) -> Self {
        Self {
            mode: PanelRunMode::Smoke,
            max_slices,
            fresh: false,
        }
    }

    pub fn continue_existing(max_slices: usize) -> Self {
        Self {
            mode: PanelRunMode::Continue,
            max_slices,
            fresh: false,
        }
    }

    pub fn drain(max_slices: usize) -> Self {
        Self {
            mode: PanelRunMode::Drain,
            max_slices,
            fresh: false,
        }
    }

    pub fn compare(max_slices: usize) -> Self {
        Self {
            mode: PanelRunMode::Compare,
            max_slices,
            fresh: false,
        }
    }

    pub fn fresh(mut self) -> Self {
        self.fresh = true;
        self
    }
}

#[derive(Clone)]
pub struct PanelInspectConfig {
    pub seeds: Vec<u64>,
    pub artifact_store: BranchArtifactStore,
    pub args_template: Args,
    pub source_identity: SourceIdentity,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct PanelArtifactFacts {
    pub manifest_exists: bool,
    pub result_exists: bool,
    pub frontier_exists: bool,
    pub terminal_exists: bool,
    pub summary_exists: bool,
    pub capsule_ledger_exists: bool,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PanelRow {
    pub profile: Option<String>,
    pub seed: u64,
    pub capsule_path: String,
    pub row_status: PanelRowStatus,
    pub identity_status: PanelIdentityStatus,
    pub reuse_decision: PanelReuseDecision,
    pub scheduler_action: PanelSeedAction,
    pub manifest_exists: bool,
    pub result_exists: bool,
    pub frontier_exists: bool,
    pub terminal_exists: bool,
    pub summary_exists: bool,
    pub capsule_ledger_exists: bool,
    pub artifact_refs: Vec<ArtifactRef>,
    pub combat_search: CombatSearchTelemetrySummary,
    pub primary_search: PrimarySearchOutcomeSummary,
    pub read_error: Option<String>,
    pub tool_error: Option<String>,
    pub archived_capsule_path: Option<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct PanelSummary {
    pub schema: &'static str,
    pub run_mode: Option<PanelRunMode>,
    pub max_slices: Option<usize>,
    pub profiles: Vec<String>,
    pub total_rows: usize,
    pub counts_by_status: BTreeMap<String, usize>,
    pub counts_by_reuse_decision: BTreeMap<String, usize>,
    pub rows: Vec<PanelRow>,
}

pub fn decide_seed_capsule(
    artifacts: PanelSeedArtifacts,
    expected_contract: RunContract,
    expected_source: &SourceIdentity,
) -> PanelSeedDecision {
    let artifact_facts = artifacts.facts();
    let Some(manifest) = artifacts.manifest else {
        return PanelSeedDecision {
            identity_status: PanelIdentityStatus::Missing,
            reuse_decision: PanelReuseDecision::CreateNewCapsule,
            artifact_facts,
        };
    };
    match decide_manifest_reuse(&manifest, expected_contract, expected_source) {
        CapsuleReuseDecision::Exact => exact_identity_decision(artifact_facts),
        CapsuleReuseDecision::UnknownLegacy => PanelSeedDecision {
            identity_status: PanelIdentityStatus::Unknown,
            reuse_decision: PanelReuseDecision::RejectUnknownIdentity,
            artifact_facts,
        },
        CapsuleReuseDecision::Incompatible => PanelSeedDecision {
            identity_status: PanelIdentityStatus::Incompatible,
            reuse_decision: PanelReuseDecision::RejectIncompatibleIdentity,
            artifact_facts,
        },
    }
}

impl PanelSeedRequest {
    pub fn resolve(self) -> PanelSeedResolution {
        match self.artifacts {
            Ok(artifacts) => {
                let decision = decide_seed_capsule(artifacts, self.contract, &self.source_identity);
                PanelSeedResolution {
                    seed: self.seed,
                    capsule_path: self.capsule_path,
                    decision,
                    read_error: None,
                }
            }
            Err(error) => PanelSeedResolution {
                seed: self.seed,
                capsule_path: self.capsule_path,
                decision: PanelSeedDecision {
                    identity_status: PanelIdentityStatus::Unknown,
                    reuse_decision: PanelReuseDecision::RejectMalformedCapsule,
                    artifact_facts: self.artifact_facts,
                },
                read_error: Some(error),
            },
        }
    }
}

impl PanelSeedResolution {
    pub fn scheduler_action(&self) -> PanelSeedAction {
        match self.decision.reuse_decision {
            PanelReuseDecision::CreateNewCapsule => PanelSeedAction::StartNew,
            PanelReuseDecision::FreshReplacedCapsule => PanelSeedAction::StartNew,
            PanelReuseDecision::ReuseRealStop => PanelSeedAction::ReuseRealStop,
            PanelReuseDecision::ContinueSoftPause => PanelSeedAction::ContinueCapsule,
            PanelReuseDecision::RejectUnknownIdentity
            | PanelReuseDecision::RejectIncompatibleIdentity
            | PanelReuseDecision::RejectIncompleteCapsule
            | PanelReuseDecision::RejectMalformedCapsule => PanelSeedAction::RejectCapsule,
        }
    }
}

impl PanelScheduler {
    pub fn resolve_requests(
        requests: impl IntoIterator<Item = PanelSeedRequest>,
    ) -> Vec<PanelSeedResolution> {
        requests
            .into_iter()
            .map(PanelSeedRequest::resolve)
            .collect()
    }

    pub fn summarize_requests(
        requests: impl IntoIterator<Item = PanelSeedRequest>,
    ) -> PanelSummary {
        PanelSummary::from_rows(
            Self::resolve_requests(requests)
                .into_iter()
                .map(PanelRow::from_resolution)
                .collect(),
        )
    }
}

impl PanelInspectConfig {
    pub fn requests(&self) -> Vec<PanelSeedRequest> {
        self.seeds
            .iter()
            .copied()
            .map(|seed| {
                let args = self.args_for_seed(seed);
                PanelSeedRequest {
                    seed,
                    capsule_path: self.artifact_store.capsule_path(seed),
                    artifacts: self.artifact_store.read_seed_artifacts(seed),
                    artifact_facts: self.artifact_store.read_seed_artifact_facts(seed),
                    contract: RunContract::from_args(args),
                    source_identity: self.source_identity.clone(),
                }
            })
            .collect()
    }

    pub fn summarize(&self) -> PanelSummary {
        PanelScheduler::summarize_requests(self.requests())
    }

    fn summarize_with_execution(
        &self,
        failures: &BTreeMap<u64, String>,
        status_overrides: &BTreeMap<u64, PanelRowStatus>,
        reuse_overrides: &BTreeMap<u64, PanelReuseDecision>,
        action_overrides: &BTreeMap<u64, PanelSeedAction>,
        artifact_refs: &BTreeMap<u64, Vec<ArtifactRef>>,
        combat_search: &BTreeMap<u64, CombatSearchTelemetrySummary>,
        primary_search: &BTreeMap<u64, PrimarySearchOutcomeSummary>,
        archive_paths: &BTreeMap<u64, PathBuf>,
        options: PanelRunOptions,
    ) -> PanelSummary {
        PanelSummary::from_rows_with_run_options(
            PanelScheduler::resolve_requests(self.requests())
                .into_iter()
                .map(|resolution| {
                    let tool_error = failures.get(&resolution.seed).cloned();
                    let status_override = status_overrides.get(&resolution.seed).copied();
                    let reuse_override = reuse_overrides.get(&resolution.seed).copied();
                    let action_override = action_overrides.get(&resolution.seed).copied();
                    let row_artifact_refs = artifact_refs
                        .get(&resolution.seed)
                        .cloned()
                        .unwrap_or_default();
                    let archived_capsule_path = archive_paths
                        .get(&resolution.seed)
                        .map(|path| path.display().to_string());
                    let row_combat_search = combat_search
                        .get(&resolution.seed)
                        .cloned()
                        .unwrap_or_default();
                    let row_primary_search = primary_search
                        .get(&resolution.seed)
                        .cloned()
                        .unwrap_or_default();
                    PanelRow::from_resolution_with_execution(
                        resolution,
                        status_override,
                        reuse_override,
                        action_override,
                        row_artifact_refs,
                        row_combat_search,
                        row_primary_search,
                        tool_error,
                        archived_capsule_path,
                    )
                })
                .collect(),
            options,
        )
    }

    fn args_for_seed(&self, seed: u64) -> Args {
        let mut args = self.args_template;
        args.seed = seed;
        args
    }
}

impl PanelSmokeRunner {
    pub fn run_once(config: PanelInspectConfig) -> Result<PanelSummary, String> {
        Self::run_slices(config, PanelRunOptions::smoke(1))
    }

    pub fn run_slices(
        config: PanelInspectConfig,
        options: PanelRunOptions,
    ) -> Result<PanelSummary, String> {
        run_slices_with_executor(config, options, OwnerAuditSliceExecutor)
    }
}

trait PanelSliceExecutor {
    fn run_slice(&mut self, request: OwnerAuditSliceRequest) -> Result<RunSliceResult, String>;
}

struct OwnerAuditSliceExecutor;

impl PanelSliceExecutor for OwnerAuditSliceExecutor {
    fn run_slice(&mut self, request: OwnerAuditSliceRequest) -> Result<RunSliceResult, String> {
        OwnerAuditRuntime::run_capsule_slice(request)
    }
}

fn run_slices_with_executor(
    config: PanelInspectConfig,
    options: PanelRunOptions,
    mut executor: impl PanelSliceExecutor,
) -> Result<PanelSummary, String> {
    if options.max_slices == 0 {
        return Err("max_slices must be greater than zero".to_string());
    }
    let mut failures = BTreeMap::new();
    let mut status_overrides = BTreeMap::new();
    let mut reuse_overrides = BTreeMap::new();
    let mut action_overrides = BTreeMap::new();
    let mut artifact_refs = BTreeMap::new();
    let mut combat_search = BTreeMap::<u64, CombatSearchTelemetrySummary>::new();
    let mut primary_search = BTreeMap::<u64, PrimarySearchOutcomeSummary>::new();
    let mut archive_paths = BTreeMap::new();
    let mut fresh_prepared = BTreeSet::new();
    for slice_index in 0..options.max_slices {
        let mut ran_slice = false;
        for resolution in PanelScheduler::resolve_requests(config.requests()) {
            if failures.contains_key(&resolution.seed) {
                continue;
            }
            let mut action = resolution.scheduler_action();
            if options.fresh && fresh_prepared.insert(resolution.seed) {
                match config.artifact_store.archive_capsule(resolution.seed) {
                    Ok(Some(archive_path)) => {
                        archive_paths.insert(resolution.seed, archive_path);
                        reuse_overrides
                            .insert(resolution.seed, PanelReuseDecision::FreshReplacedCapsule);
                    }
                    Ok(None) => {}
                    Err(error) => {
                        append_panel_event(
                            &config,
                            resolution.seed,
                            action,
                            "failed",
                            options.mode,
                            slice_index,
                            Some(error.clone()),
                            Vec::new(),
                        )?;
                        failures.insert(resolution.seed, error);
                        continue;
                    }
                }
                action = PanelSeedAction::StartNew;
            }
            match action {
                PanelSeedAction::StartNew => {
                    if options.mode == PanelRunMode::Continue && !options.fresh {
                        append_panel_event(
                            &config,
                            resolution.seed,
                            action,
                            "skipped",
                            options.mode,
                            slice_index,
                            None,
                            Vec::new(),
                        )?;
                        status_overrides.insert(resolution.seed, PanelRowStatus::Skipped);
                        continue;
                    }
                    let (error, refs, row_status, telemetry, primary) = run_panel_seed_slice(
                        &config,
                        resolution.seed,
                        action,
                        resolution.capsule_path,
                        false,
                        &mut executor,
                        options.mode,
                        slice_index,
                    )?;
                    status_overrides.insert(resolution.seed, row_status);
                    combat_search
                        .entry(resolution.seed)
                        .or_default()
                        .merge(telemetry);
                    primary_search.insert(resolution.seed, primary);
                    action_overrides.insert(resolution.seed, action);
                    reuse_overrides
                        .entry(resolution.seed)
                        .or_insert(resolution.decision.reuse_decision);
                    if !refs.is_empty() {
                        artifact_refs.insert(resolution.seed, refs);
                    }
                    if let Some(error) = error {
                        failures.insert(resolution.seed, error);
                    }
                    ran_slice = true;
                }
                PanelSeedAction::ContinueCapsule => {
                    let (error, refs, row_status, telemetry, primary) = run_panel_seed_slice(
                        &config,
                        resolution.seed,
                        action,
                        resolution.capsule_path,
                        true,
                        &mut executor,
                        options.mode,
                        slice_index,
                    )?;
                    status_overrides.insert(resolution.seed, row_status);
                    combat_search
                        .entry(resolution.seed)
                        .or_default()
                        .merge(telemetry);
                    primary_search.insert(resolution.seed, primary);
                    action_overrides.insert(resolution.seed, action);
                    reuse_overrides
                        .entry(resolution.seed)
                        .or_insert(resolution.decision.reuse_decision);
                    if !refs.is_empty() {
                        artifact_refs.insert(resolution.seed, refs);
                    }
                    if let Some(error) = error {
                        failures.insert(resolution.seed, error);
                    }
                    ran_slice = true;
                }
                PanelSeedAction::ReuseRealStop | PanelSeedAction::RejectCapsule => {
                    append_panel_event(
                        &config,
                        resolution.seed,
                        action,
                        "skipped",
                        options.mode,
                        slice_index,
                        None,
                        Vec::new(),
                    )?;
                }
            }
        }
        if !ran_slice {
            break;
        }
    }
    Ok(config.summarize_with_execution(
        &failures,
        &status_overrides,
        &reuse_overrides,
        &action_overrides,
        &artifact_refs,
        &combat_search,
        &primary_search,
        &archive_paths,
        options,
    ))
}

fn run_panel_seed_slice(
    config: &PanelInspectConfig,
    seed: u64,
    action: PanelSeedAction,
    capsule_path: PathBuf,
    resume: bool,
    executor: &mut impl PanelSliceExecutor,
    run_mode: PanelRunMode,
    slice_index: usize,
) -> Result<
    (
        Option<String>,
        Vec<ArtifactRef>,
        PanelRowStatus,
        CombatSearchTelemetrySummary,
        PrimarySearchOutcomeSummary,
    ),
    String,
> {
    match executor.run_slice(OwnerAuditSliceRequest {
        args: config.args_for_seed(seed),
        capsule_path,
        resume,
        human_output: false,
    }) {
        Ok(result) => {
            let row_status = row_status_from_stop(&result.stop);
            let refs = result.artifacts.refs();
            let telemetry = result.combat_search;
            let primary = result.primary_search;
            append_panel_event(
                config,
                seed,
                action,
                "executed",
                run_mode,
                slice_index,
                None,
                refs.clone(),
            )?;
            Ok((None, refs, row_status, telemetry, primary))
        }
        Err(error) => {
            append_panel_event(
                config,
                seed,
                action,
                "failed",
                run_mode,
                slice_index,
                Some(error.clone()),
                Vec::new(),
            )?;
            Ok((
                Some(error),
                Vec::new(),
                PanelRowStatus::ToolFailed,
                CombatSearchTelemetrySummary::default(),
                PrimarySearchOutcomeSummary::default(),
            ))
        }
    }
}

fn append_panel_event(
    config: &PanelInspectConfig,
    seed: u64,
    action: PanelSeedAction,
    event: &'static str,
    run_mode: PanelRunMode,
    slice_index: usize,
    error: Option<String>,
    artifact_refs: Vec<ArtifactRef>,
) -> Result<(), String> {
    let event = PanelLedgerEvent::for_slice(
        seed,
        action,
        event,
        run_mode,
        slice_index,
        error,
        artifact_refs,
    );
    config
        .artifact_store
        .append_panel_ledger_event(None, &event)
        .map(|_| ())
}

impl PanelRow {
    pub fn from_resolution(resolution: PanelSeedResolution) -> Self {
        Self::from_resolution_with_execution(
            resolution,
            None,
            None,
            None,
            Vec::new(),
            CombatSearchTelemetrySummary::default(),
            PrimarySearchOutcomeSummary::default(),
            None,
            None,
        )
    }

    fn from_resolution_with_execution(
        resolution: PanelSeedResolution,
        status_override: Option<PanelRowStatus>,
        reuse_override: Option<PanelReuseDecision>,
        action_override: Option<PanelSeedAction>,
        artifact_refs: Vec<ArtifactRef>,
        combat_search: CombatSearchTelemetrySummary,
        primary_search: PrimarySearchOutcomeSummary,
        tool_error: Option<String>,
        archived_capsule_path: Option<String>,
    ) -> Self {
        let artifacts = resolution.decision.artifact_facts;
        let scheduler_action = action_override.unwrap_or_else(|| resolution.scheduler_action());
        let row_status = if tool_error.is_some() {
            PanelRowStatus::ToolFailed
        } else {
            status_override.unwrap_or_else(|| row_status_from_action(scheduler_action))
        };
        Self {
            profile: None,
            seed: resolution.seed,
            capsule_path: resolution.capsule_path.display().to_string(),
            row_status,
            identity_status: resolution.decision.identity_status,
            reuse_decision: reuse_override.unwrap_or(resolution.decision.reuse_decision),
            scheduler_action,
            manifest_exists: artifacts.manifest_exists,
            result_exists: artifacts.result_exists,
            frontier_exists: artifacts.frontier_exists,
            terminal_exists: artifacts.terminal_exists,
            summary_exists: artifacts.summary_exists,
            capsule_ledger_exists: artifacts.capsule_ledger_exists,
            artifact_refs,
            combat_search,
            primary_search,
            read_error: resolution.read_error,
            tool_error,
            archived_capsule_path,
        }
    }
}

impl PanelSummary {
    pub fn from_rows(rows: Vec<PanelRow>) -> Self {
        Self::from_rows_with_context(rows, None, None, Vec::new())
    }

    pub fn from_rows_with_run_options(rows: Vec<PanelRow>, options: PanelRunOptions) -> Self {
        Self::from_rows_with_context(
            rows,
            Some(options.mode),
            Some(options.max_slices),
            Vec::new(),
        )
    }

    pub fn from_rows_with_compare(
        rows: Vec<PanelRow>,
        max_slices: usize,
        profiles: Vec<String>,
    ) -> Self {
        Self::from_rows_with_context(
            rows,
            Some(PanelRunMode::Compare),
            Some(max_slices),
            profiles,
        )
    }

    fn from_rows_with_context(
        rows: Vec<PanelRow>,
        run_mode: Option<PanelRunMode>,
        max_slices: Option<usize>,
        profiles: Vec<String>,
    ) -> Self {
        let mut counts_by_status = BTreeMap::new();
        let mut counts_by_reuse_decision = BTreeMap::new();
        for row in &rows {
            *counts_by_status
                .entry(row_status_key(row.row_status))
                .or_insert(0) += 1;
            *counts_by_reuse_decision
                .entry(reuse_decision_key(row.reuse_decision))
                .or_insert(0) += 1;
        }
        Self {
            schema: "branch_panel_summary_v0",
            run_mode,
            max_slices,
            profiles,
            total_rows: rows.len(),
            counts_by_status,
            counts_by_reuse_decision,
            rows,
        }
    }
}

fn row_status_from_action(action: PanelSeedAction) -> PanelRowStatus {
    match action {
        PanelSeedAction::StartNew => PanelRowStatus::Scheduled,
        PanelSeedAction::ContinueCapsule => PanelRowStatus::SoftPaused,
        PanelSeedAction::ReuseRealStop => PanelRowStatus::ReusedRealStop,
        PanelSeedAction::RejectCapsule => PanelRowStatus::Skipped,
    }
}

fn row_status_from_stop(stop: &super::RunStop) -> PanelRowStatus {
    match stop {
        super::RunStop::Real(_) | super::RunStop::FrontierExhausted(_) => {
            PanelRowStatus::RealStopped
        }
        super::RunStop::SoftPause(_) => PanelRowStatus::SoftPaused,
    }
}

fn row_status_key(status: PanelRowStatus) -> String {
    serde_json::to_value(status)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn reuse_decision_key(decision: PanelReuseDecision) -> String {
    serde_json::to_value(decision)
        .ok()
        .and_then(|value| value.as_str().map(str::to_string))
        .unwrap_or_else(|| "unknown".to_string())
}

fn exact_identity_decision(artifact_facts: PanelArtifactFacts) -> PanelSeedDecision {
    let reuse_decision = if artifact_facts.result_exists {
        PanelReuseDecision::ReuseRealStop
    } else if artifact_facts.frontier_exists {
        PanelReuseDecision::ContinueSoftPause
    } else {
        PanelReuseDecision::RejectIncompleteCapsule
    };
    PanelSeedDecision {
        identity_status: PanelIdentityStatus::Exact,
        reuse_decision,
        artifact_facts,
    }
}

impl PanelSeedArtifacts {
    fn facts(&self) -> PanelArtifactFacts {
        PanelArtifactFacts::from_artifacts(self)
    }
}

impl PanelArtifactFacts {
    fn from_artifacts(artifacts: &PanelSeedArtifacts) -> Self {
        Self {
            manifest_exists: artifacts.manifest.is_some(),
            result_exists: artifacts.result_exists,
            frontier_exists: artifacts.frontier_exists,
            terminal_exists: artifacts.terminal_exists,
            summary_exists: artifacts.summary_exists,
            capsule_ledger_exists: artifacts.capsule_ledger_exists,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use serde_json::json;

    use super::*;
    use crate::runtime::branch::{
        Args, ArtifactKind, CombatSearchTimingSummary, FrontierSummary,
        PrimarySearchProfileSummary, PrimarySearchTelemetrySummary, RealStop, RunObjective,
        RunSliceRequestKind, RunSliceResult, RunStop, SoftPause,
    };

    fn args(seed: u64) -> Args {
        Args {
            seed,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 1,
            max_branches: 1,
            auto_ops: 1,
            search_nodes: 1,
            search_ms: 1,
            rescue_search_nodes: 1,
            rescue_search_ms: 1,
            boss_search_nodes: 1,
            boss_search_ms: 1,
            wall_ms: Some(1),
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    fn source_identity() -> SourceIdentity {
        SourceIdentity {
            git_commit: Some("abc123".to_string()),
            git_dirty: Some(false),
        }
    }

    fn ok_slice_result(args: Args) -> RunSliceResult {
        RunSliceResult::new(
            args,
            RunSliceRequestKind::Start,
            0,
            0,
            1,
            RunStop::SoftPause(SoftPause::GenerationLimit {
                generation: 0,
                frontier_running_count: 1,
            }),
            FrontierSummary {
                total_count: 1,
                running_count: 1,
                expandable_count: 1,
                terminal_count: 0,
                gap_count: 0,
            },
            Some(0),
            0,
        )
    }

    fn combat_gap_slice_result(args: Args) -> RunSliceResult {
        RunSliceResult::new(
            args,
            RunSliceRequestKind::Start,
            0,
            13,
            21,
            RunStop::Real(RealStop::CombatGap {
                generation: 13,
                branch_id: 7,
                boundary: "Combat".to_string(),
                reason: "combat search did not find an executable complete win".to_string(),
            }),
            FrontierSummary {
                total_count: 1,
                running_count: 0,
                expandable_count: 0,
                terminal_count: 0,
                gap_count: 1,
            },
            Some(7),
            0,
        )
    }

    fn request_for_path(seed: u64, capsule_path: PathBuf) -> PanelSeedRequest {
        let store = BranchArtifactStore::new("unused");
        let artifact_facts = store.read_capsule_artifact_facts(&capsule_path);
        PanelSeedRequest {
            seed,
            artifacts: PanelSeedArtifacts::from_capsule_path(&capsule_path),
            artifact_facts,
            capsule_path,
            contract: RunContract::from_args(args(seed)),
            source_identity: source_identity(),
        }
    }

    fn exact_manifest(contract: RunContract) -> serde_json::Value {
        json!({
            "run_contract": contract,
            "source_identity": source_identity(),
            "status": "paused",
            "reason": "wall_deadline",
        })
    }

    #[test]
    fn missing_capsule_manifest_starts_new_capsule() {
        let decision = decide_seed_capsule(
            PanelSeedArtifacts::default(),
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(
            decision.reuse_decision,
            PanelReuseDecision::CreateNewCapsule
        );
        assert_eq!(decision.identity_status, PanelIdentityStatus::Missing);
    }

    #[test]
    fn exact_identity_with_result_reuses_real_stop() {
        let artifacts = PanelSeedArtifacts {
            manifest: Some(exact_manifest(RunContract::from_args(args(1)))),
            result_exists: true,
            summary_exists: true,
            ..PanelSeedArtifacts::default()
        };

        let decision = decide_seed_capsule(
            artifacts,
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(decision.reuse_decision, PanelReuseDecision::ReuseRealStop);
        assert_eq!(decision.identity_status, PanelIdentityStatus::Exact);
    }

    #[test]
    fn exact_paused_capsule_with_frontier_continues_soft_pause() {
        let artifacts = PanelSeedArtifacts {
            manifest: Some(exact_manifest(RunContract::from_args(args(1)))),
            frontier_exists: true,
            summary_exists: true,
            ..PanelSeedArtifacts::default()
        };

        let decision = decide_seed_capsule(
            artifacts,
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(
            decision.reuse_decision,
            PanelReuseDecision::ContinueSoftPause
        );
        assert_eq!(decision.identity_status, PanelIdentityStatus::Exact);
    }

    #[test]
    fn legacy_capsule_is_not_silently_reused() {
        let artifacts = PanelSeedArtifacts {
            manifest: Some(json!({"args": {"seed": 1}, "status": "terminal"})),
            result_exists: true,
            summary_exists: true,
            ..PanelSeedArtifacts::default()
        };

        let decision = decide_seed_capsule(
            artifacts,
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(
            decision.reuse_decision,
            PanelReuseDecision::RejectUnknownIdentity
        );
        assert_eq!(decision.identity_status, PanelIdentityStatus::Unknown);
    }

    #[test]
    fn exact_paused_capsule_without_frontier_is_incomplete() {
        let artifacts = PanelSeedArtifacts {
            manifest: Some(exact_manifest(RunContract::from_args(args(1)))),
            summary_exists: true,
            ..PanelSeedArtifacts::default()
        };

        let decision = decide_seed_capsule(
            artifacts,
            RunContract::from_args(args(1)),
            &source_identity(),
        );

        assert_eq!(
            decision.reuse_decision,
            PanelReuseDecision::RejectIncompleteCapsule
        );
        assert_eq!(decision.identity_status, PanelIdentityStatus::Exact);
    }

    #[test]
    fn reads_capsule_artifact_presence_from_directory() {
        let dir = std::env::temp_dir().join("runtime_branch_panel_artifacts");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("manifest.json"),
            exact_manifest(RunContract::from_args(args(1))).to_string(),
        )
        .unwrap();
        fs::write(dir.join("result.json"), "{}").unwrap();
        fs::write(dir.join("summary.json"), "{}").unwrap();

        let artifacts = PanelSeedArtifacts::from_capsule_path(&dir).unwrap();

        assert!(artifacts.manifest.is_some());
        assert!(artifacts.result_exists);
        assert!(!artifacts.frontier_exists);
        assert!(!artifacts.terminal_exists);
        assert!(artifacts.summary_exists);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn malformed_manifest_is_a_capsule_read_error() {
        let dir = std::env::temp_dir().join("runtime_branch_panel_bad_manifest");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.json"), "{bad").unwrap();

        let err = PanelSeedArtifacts::from_capsule_path(&dir).unwrap_err();

        assert!(err.contains("manifest.json"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn seed_resolution_preserves_malformed_capsule_as_a_row_decision() {
        let dir = std::env::temp_dir().join("runtime_branch_panel_bad_row");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.json"), "{bad").unwrap();

        let resolution = PanelSeedRequest {
            seed: 1,
            artifacts: PanelSeedArtifacts::from_capsule_path(&dir),
            artifact_facts: BranchArtifactStore::new("unused").read_capsule_artifact_facts(&dir),
            capsule_path: dir.clone(),
            contract: RunContract::from_args(args(1)),
            source_identity: source_identity(),
        }
        .resolve();

        assert_eq!(resolution.seed, 1);
        assert_eq!(
            resolution.decision.reuse_decision,
            PanelReuseDecision::RejectMalformedCapsule
        );
        assert!(resolution.read_error.unwrap().contains("manifest.json"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn scheduler_resolution_keeps_one_row_per_seed() {
        let root = std::env::temp_dir().join("runtime_branch_panel_rows");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let good = root.join("good");
        let bad = root.join("bad");
        fs::create_dir_all(&good).unwrap();
        fs::create_dir_all(&bad).unwrap();
        fs::write(
            good.join("manifest.json"),
            exact_manifest(RunContract::from_args(args(1))).to_string(),
        )
        .unwrap();
        fs::write(good.join("frontier.json"), "{}").unwrap();
        fs::write(bad.join("manifest.json"), "{bad").unwrap();

        let rows = PanelScheduler::resolve_requests(vec![
            request_for_path(1, good),
            request_for_path(2, bad),
            request_for_path(3, root.join("missing")),
        ]);

        assert_eq!(rows.len(), 3);
        assert_eq!(
            rows[0].decision.reuse_decision,
            PanelReuseDecision::ContinueSoftPause
        );
        assert_eq!(
            rows[1].decision.reuse_decision,
            PanelReuseDecision::RejectMalformedCapsule
        );
        assert_eq!(
            rows[2].decision.reuse_decision,
            PanelReuseDecision::CreateNewCapsule
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn panel_row_serializes_resolution_as_structured_fields() {
        let resolution = PanelSeedResolution {
            seed: 7,
            capsule_path: PathBuf::from("target/example"),
            decision: PanelSeedDecision {
                identity_status: PanelIdentityStatus::Exact,
                reuse_decision: PanelReuseDecision::ContinueSoftPause,
                artifact_facts: PanelArtifactFacts {
                    manifest_exists: true,
                    result_exists: false,
                    frontier_exists: true,
                    terminal_exists: false,
                    summary_exists: true,
                    capsule_ledger_exists: true,
                },
            },
            read_error: None,
        };

        let value = serde_json::to_value(PanelRow::from_resolution(resolution)).unwrap();

        assert_eq!(value["seed"], 7);
        assert_eq!(value["capsule_path"], "target/example");
        assert_eq!(value["identity_status"], "exact");
        assert_eq!(value["reuse_decision"], "continue_soft_pause");
        assert_eq!(value["scheduler_action"], "continue_capsule");
        assert_eq!(value["row_status"], "soft_paused");
        assert_eq!(value["frontier_exists"], true);
        assert_eq!(value["result_exists"], false);
        assert_eq!(value["capsule_ledger_exists"], true);
        assert_eq!(value["read_error"], serde_json::Value::Null);
        assert_eq!(value["tool_error"], serde_json::Value::Null);
    }

    #[test]
    fn panel_summary_counts_rows_by_reuse_decision() {
        let rows = vec![
            PanelRow {
                profile: None,
                seed: 1,
                capsule_path: "one".to_string(),
                row_status: PanelRowStatus::SoftPaused,
                identity_status: PanelIdentityStatus::Exact,
                reuse_decision: PanelReuseDecision::ContinueSoftPause,
                scheduler_action: PanelSeedAction::ContinueCapsule,
                manifest_exists: true,
                result_exists: false,
                frontier_exists: true,
                terminal_exists: false,
                summary_exists: true,
                capsule_ledger_exists: true,
                artifact_refs: Vec::new(),
                combat_search: CombatSearchTelemetrySummary::default(),
                primary_search: PrimarySearchOutcomeSummary::default(),
                read_error: None,
                tool_error: None,
                archived_capsule_path: None,
            },
            PanelRow {
                profile: None,
                seed: 2,
                capsule_path: "two".to_string(),
                row_status: PanelRowStatus::Scheduled,
                identity_status: PanelIdentityStatus::Missing,
                reuse_decision: PanelReuseDecision::CreateNewCapsule,
                scheduler_action: PanelSeedAction::StartNew,
                manifest_exists: false,
                result_exists: false,
                frontier_exists: false,
                terminal_exists: false,
                summary_exists: false,
                capsule_ledger_exists: false,
                artifact_refs: Vec::new(),
                combat_search: CombatSearchTelemetrySummary::default(),
                primary_search: PrimarySearchOutcomeSummary::default(),
                read_error: None,
                tool_error: None,
                archived_capsule_path: None,
            },
        ];

        let value = serde_json::to_value(PanelSummary::from_rows(rows)).unwrap();

        assert_eq!(value["run_mode"], serde_json::Value::Null);
        assert_eq!(value["max_slices"], serde_json::Value::Null);
        assert_eq!(value["total_rows"], 2);
        assert_eq!(value["counts_by_status"]["soft_paused"], 1);
        assert_eq!(value["counts_by_status"]["scheduled"], 1);
        assert_eq!(value["counts_by_reuse_decision"]["continue_soft_pause"], 1);
        assert_eq!(value["counts_by_reuse_decision"]["create_new_capsule"], 1);
    }

    #[test]
    fn compare_summary_rows_carry_profile_identity() {
        let rows = vec![PanelRow {
            profile: Some("baseline".to_string()),
            seed: 1,
            capsule_path: "target/panel/_compare/baseline/1".to_string(),
            row_status: PanelRowStatus::Scheduled,
            identity_status: PanelIdentityStatus::Missing,
            reuse_decision: PanelReuseDecision::CreateNewCapsule,
            scheduler_action: PanelSeedAction::StartNew,
            manifest_exists: false,
            result_exists: false,
            frontier_exists: false,
            terminal_exists: false,
            summary_exists: false,
            capsule_ledger_exists: false,
            artifact_refs: Vec::new(),
            combat_search: CombatSearchTelemetrySummary::default(),
            primary_search: PrimarySearchOutcomeSummary::default(),
            read_error: None,
            tool_error: None,
            archived_capsule_path: None,
        }];

        let value = serde_json::to_value(PanelSummary::from_rows_with_compare(
            rows,
            1,
            vec!["baseline".to_string()],
        ))
        .unwrap();

        assert_eq!(value["profiles"], json!(["baseline"]));
        assert_eq!(value["rows"][0]["profile"], json!("baseline"));
    }

    #[test]
    fn resolution_maps_reuse_decision_to_scheduler_action() {
        let resolution = PanelSeedResolution {
            seed: 1,
            capsule_path: PathBuf::from("target/example"),
            decision: PanelSeedDecision {
                identity_status: PanelIdentityStatus::Exact,
                reuse_decision: PanelReuseDecision::ContinueSoftPause,
                artifact_facts: PanelArtifactFacts {
                    manifest_exists: true,
                    result_exists: false,
                    frontier_exists: true,
                    terminal_exists: false,
                    summary_exists: true,
                    capsule_ledger_exists: true,
                },
            },
            read_error: None,
        };

        assert_eq!(
            resolution.scheduler_action(),
            PanelSeedAction::ContinueCapsule
        );
    }

    #[test]
    fn exact_result_capsule_summarizes_as_reuse_real_stop() {
        let root = std::env::temp_dir().join("runtime_branch_panel_summary");
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let capsule = root.join("seed1");
        fs::create_dir_all(&capsule).unwrap();
        fs::write(
            capsule.join("manifest.json"),
            exact_manifest(RunContract::from_args(args(1))).to_string(),
        )
        .unwrap();
        fs::write(capsule.join("result.json"), "{}").unwrap();

        let summary = PanelScheduler::summarize_requests(vec![request_for_path(1, capsule)]);

        assert_eq!(summary.total_rows, 1);
        assert_eq!(
            summary.rows[0].reuse_decision,
            PanelReuseDecision::ReuseRealStop
        );
        assert_eq!(
            summary.rows[0].scheduler_action,
            PanelSeedAction::ReuseRealStop
        );
        assert_eq!(summary.counts_by_reuse_decision["reuse_real_stop"], 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn inspect_config_materializes_seed_requests_from_a_template() {
        let mut template = args(999);
        template.ascension = 3;
        template.generations = 9;
        let source = source_identity();
        let config = PanelInspectConfig {
            seeds: vec![7, 8],
            artifact_store: BranchArtifactStore::new("target/panel-config"),
            args_template: template,
            source_identity: source.clone(),
        };

        let requests = config.requests();

        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].seed, 7);
        assert_eq!(
            requests[0].capsule_path,
            PathBuf::from("target/panel-config/7")
        );
        assert_eq!(requests[0].contract.game.seed, 7);
        assert_eq!(requests[0].contract.game.ascension, 3);
        assert_eq!(requests[0].contract.branching.generations, 9);
        assert_eq!(requests[0].source_identity, source);
        assert_eq!(requests[1].contract.game.seed, 8);
    }

    #[test]
    fn smoke_runner_executes_missing_seed_capsules_in_process() {
        let root = std::env::temp_dir().join("runtime_branch_panel_smoke_runner");
        let _ = fs::remove_dir_all(&root);
        let mut template = args(0);
        template.generations = 0;
        template.max_branches = 1;
        template.search_nodes = 1;
        template.search_ms = 1;
        template.rescue_search_nodes = 1;
        template.rescue_search_ms = 1;
        template.boss_search_nodes = 1;
        template.boss_search_ms = 1;
        template.wall_ms = Some(1_000);
        let config = PanelInspectConfig {
            seeds: vec![123],
            artifact_store: BranchArtifactStore::new(&root),
            args_template: template,
            source_identity: source_identity(),
        };

        let summary = PanelSmokeRunner::run_once(config).unwrap();

        assert_eq!(summary.total_rows, 1);
        assert!(root.join("123").join("manifest.json").exists());
        let ledger = fs::read_to_string(root.join("panel_ledger.jsonl")).unwrap();
        assert!(ledger.contains("\"event\":\"executed\""));
        let ledger_row: serde_json::Value =
            serde_json::from_str(ledger.lines().next().unwrap()).unwrap();
        assert_eq!(
            ledger_row["artifact_refs"][1]["kind"],
            serde_json::json!("frontier")
        );
        assert_eq!(
            ledger_row["artifact_refs"][1]["schema"],
            serde_json::json!("branch_tiny_frontier_checkpoint")
        );
        assert_eq!(
            summary.rows[0].artifact_refs[1].kind,
            ArtifactKind::Frontier
        );
        assert!(summary.rows[0].artifact_refs[1]
            .path
            .ends_with("frontier.json"));
        assert!(summary.rows[0].manifest_exists);
        assert_eq!(summary.rows[0].scheduler_action, PanelSeedAction::StartNew);
        assert_eq!(
            summary.rows[0].reuse_decision,
            PanelReuseDecision::CreateNewCapsule
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_runner_carries_combat_search_telemetry() {
        struct TelemetryExecutor;

        impl PanelSliceExecutor for TelemetryExecutor {
            fn run_slice(
                &mut self,
                request: OwnerAuditSliceRequest,
            ) -> Result<RunSliceResult, String> {
                let mut telemetry = CombatSearchTelemetrySummary::default();
                telemetry.record_attempt("primary", true, 3, 44, 55);
                telemetry.record_attempt_with_timing(
                    "quality",
                    false,
                    0,
                    6,
                    70,
                    CombatSearchTimingSummary {
                        rollout_us: 40,
                        expansion_us: 5,
                        engine_step_us: 7,
                        pre_expand_us: 2,
                        frontier_pop_us: 11,
                        child_bookkeeping_us: 3,
                        turn_plan_seed_us: 13,
                        shadow_audit_us: 17,
                        root_turn_plan_diag_us: 19,
                        unattributed_us: 13,
                    },
                );
                let primary = super::PrimarySearchOutcomeSummary {
                    status: "no_accepted_line".to_string(),
                    profile: PrimarySearchProfileSummary {
                        profile_id: Some("primary".to_string()),
                        stakes: Some("hallway".to_string()),
                        max_nodes: Some(100),
                        wall_ms: Some(500),
                        potion_policy: Some("never".to_string()),
                        max_potions_used: Some(0),
                        internal_no_win_rescue_enabled: false,
                    },
                    telemetry: PrimarySearchTelemetrySummary {
                        elapsed_ms: Some(125),
                        deadline_hit: Some(true),
                        expanded_nodes: Some(44),
                        terminal_wins: Some(0),
                        us_per_node: Some(2),
                        first_win_node: Some(17),
                        first_win_ms: None,
                        first_accepted_node: None,
                        first_accepted_ms: None,
                        rollout_us: Some(40),
                        expansion_us: Some(5),
                        transition_us: Some(7),
                        rollout_pct: Some(32),
                        expansion_pct: Some(4),
                        transition_pct: Some(6),
                        diagnostic_pct: Some(29),
                        unattributed_pct: Some(10),
                        selected_first_action: Some("combat/play:Strike:target0".to_string()),
                        top_root_actions: vec!["combat/play:Strike:target0".to_string()],
                    },
                    accepted_line: None,
                    best_complete_line: None,
                    best_partial_line: None,
                };
                Ok(ok_slice_result(request.args)
                    .with_combat_search_telemetry(telemetry)
                    .with_primary_search_outcome(primary))
            }
        }

        let root = std::env::temp_dir().join("runtime_branch_panel_combat_search_telemetry");
        let _ = fs::remove_dir_all(&root);
        let config = PanelInspectConfig {
            seeds: vec![1],
            artifact_store: BranchArtifactStore::new(&root),
            args_template: args(0),
            source_identity: source_identity(),
        };

        let summary =
            run_slices_with_executor(config, PanelRunOptions::smoke(1), TelemetryExecutor).unwrap();

        let search = &summary.rows[0].combat_search;
        assert_eq!(search.attempt_count, 2);
        assert_eq!(search.complete_win_count, 1);
        assert_eq!(search.terminal_win_count, 3);
        assert_eq!(search.nodes_expanded, 50);
        assert_eq!(search.total_us, 125);
        assert_eq!(search.timing.rollout_us, 40);
        assert_eq!(search.timing.frontier_pop_us, 11);
        assert_eq!(search.timing.turn_plan_seed_us, 13);
        assert_eq!(search.timing.shadow_audit_us, 17);
        assert_eq!(search.timing.root_turn_plan_diag_us, 19);
        assert_eq!(search.timing.unattributed_us, 13);
        assert_eq!(search.by_source[0].source, "primary");
        assert_eq!(search.by_source[0].nodes_expanded, 44);
        assert_eq!(search.by_source[1].source, "quality");
        assert_eq!(search.by_source[1].total_us, 70);
        assert_eq!(search.by_source[1].timing.engine_step_us, 7);
        assert_eq!(search.by_source[1].timing.root_turn_plan_diag_us, 19);
        assert_eq!(summary.rows[0].primary_search.status, "no_accepted_line");
        assert_eq!(
            summary.rows[0].primary_search.profile.profile_id.as_deref(),
            Some("primary")
        );
        assert_eq!(
            summary.rows[0].primary_search.telemetry.first_win_node,
            Some(17)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_runner_honors_max_slices() {
        let root = std::env::temp_dir().join("runtime_branch_panel_two_slices");
        let _ = fs::remove_dir_all(&root);
        let mut template = args(0);
        template.generations = 0;
        template.max_branches = 1;
        template.search_nodes = 1;
        template.search_ms = 1;
        template.rescue_search_nodes = 1;
        template.rescue_search_ms = 1;
        template.boss_search_nodes = 1;
        template.boss_search_ms = 1;
        template.wall_ms = Some(1_000);
        let config = PanelInspectConfig {
            seeds: vec![123],
            artifact_store: BranchArtifactStore::new(&root),
            args_template: template,
            source_identity: source_identity(),
        };

        let summary = PanelSmokeRunner::run_slices(config, PanelRunOptions::smoke(2)).unwrap();

        let ledger = fs::read_to_string(root.join("panel_ledger.jsonl")).unwrap();
        let ledger_rows = ledger
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
            .collect::<Vec<_>>();

        assert_eq!(summary.total_rows, 1);
        assert_eq!(summary.run_mode, Some(PanelRunMode::Smoke));
        assert_eq!(summary.max_slices, Some(2));
        assert_eq!(ledger_rows.len(), 2);
        assert_eq!(ledger_rows[0]["run_mode"], "smoke");
        assert_eq!(ledger_rows[0]["slice_index"], 0);
        assert_eq!(ledger_rows[1]["run_mode"], "smoke");
        assert_eq!(ledger_rows[1]["slice_index"], 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_runner_keeps_tool_failed_rows() {
        struct FailingSeedExecutor {
            failed_seed: u64,
        }

        impl PanelSliceExecutor for FailingSeedExecutor {
            fn run_slice(
                &mut self,
                request: OwnerAuditSliceRequest,
            ) -> Result<RunSliceResult, String> {
                if request.args.seed == self.failed_seed {
                    Err("synthetic slice failure".to_string())
                } else {
                    Ok(ok_slice_result(request.args))
                }
            }
        }

        let root = std::env::temp_dir().join("runtime_branch_panel_tool_failed");
        let _ = fs::remove_dir_all(&root);
        let config = PanelInspectConfig {
            seeds: vec![1, 2],
            artifact_store: BranchArtifactStore::new(&root),
            args_template: args(0),
            source_identity: source_identity(),
        };

        let summary = run_slices_with_executor(
            config,
            PanelRunOptions::smoke(1),
            FailingSeedExecutor { failed_seed: 2 },
        )
        .unwrap();

        assert_eq!(summary.total_rows, 2);
        assert_eq!(summary.counts_by_status["soft_paused"], 1);
        assert_eq!(summary.counts_by_status["tool_failed"], 1);
        assert_eq!(summary.rows[0].row_status, PanelRowStatus::SoftPaused);
        assert_eq!(summary.rows[1].row_status, PanelRowStatus::ToolFailed);
        assert_eq!(
            summary.rows[1].tool_error.as_deref(),
            Some("synthetic slice failure")
        );

        let ledger = fs::read_to_string(root.join("panel_ledger.jsonl")).unwrap();
        assert!(ledger.contains("\"event\":\"executed\""));
        assert!(ledger.contains("\"event\":\"failed\""));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn smoke_runner_reports_real_stop_rows_after_execution() {
        struct CombatGapExecutor;

        impl PanelSliceExecutor for CombatGapExecutor {
            fn run_slice(
                &mut self,
                request: OwnerAuditSliceRequest,
            ) -> Result<RunSliceResult, String> {
                Ok(combat_gap_slice_result(request.args))
            }
        }

        let root = std::env::temp_dir().join("runtime_branch_panel_real_stop_status");
        let _ = fs::remove_dir_all(&root);
        let config = PanelInspectConfig {
            seeds: vec![1],
            artifact_store: BranchArtifactStore::new(&root),
            args_template: args(0),
            source_identity: source_identity(),
        };

        let summary =
            run_slices_with_executor(config, PanelRunOptions::smoke(1), CombatGapExecutor).unwrap();

        assert_eq!(summary.total_rows, 1);
        assert_eq!(summary.counts_by_status["real_stopped"], 1);
        assert_eq!(summary.rows[0].row_status, PanelRowStatus::RealStopped);
        assert_eq!(summary.rows[0].scheduler_action, PanelSeedAction::StartNew);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn continue_mode_skips_missing_capsules() {
        struct PanicExecutor;

        impl PanelSliceExecutor for PanicExecutor {
            fn run_slice(
                &mut self,
                _request: OwnerAuditSliceRequest,
            ) -> Result<RunSliceResult, String> {
                panic!("continue mode should not start missing capsules")
            }
        }

        let root = std::env::temp_dir().join("runtime_branch_panel_continue_missing");
        let _ = fs::remove_dir_all(&root);
        let config = PanelInspectConfig {
            seeds: vec![1],
            artifact_store: BranchArtifactStore::new(&root),
            args_template: args(0),
            source_identity: source_identity(),
        };

        let summary =
            run_slices_with_executor(config, PanelRunOptions::continue_existing(1), PanicExecutor)
                .unwrap();

        assert_eq!(summary.total_rows, 1);
        assert_eq!(summary.rows[0].scheduler_action, PanelSeedAction::StartNew);
        assert_eq!(summary.rows[0].row_status, PanelRowStatus::Skipped);
        assert_eq!(summary.counts_by_status["skipped"], 1);
        assert!(!root.join("1").join("manifest.json").exists());

        let ledger = fs::read_to_string(root.join("panel_ledger.jsonl")).unwrap();
        assert!(ledger.contains("\"event\":\"skipped\""));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn fresh_run_archives_existing_capsule_before_starting_new_slice() {
        struct OkExecutor;

        impl PanelSliceExecutor for OkExecutor {
            fn run_slice(
                &mut self,
                request: OwnerAuditSliceRequest,
            ) -> Result<RunSliceResult, String> {
                Ok(ok_slice_result(request.args))
            }
        }

        let root = std::env::temp_dir().join("runtime_branch_panel_fresh_archive");
        let _ = fs::remove_dir_all(&root);
        let capsule = root.join("1");
        fs::create_dir_all(&capsule).unwrap();
        fs::write(
            capsule.join("manifest.json"),
            exact_manifest(RunContract::from_args(args(1))).to_string(),
        )
        .unwrap();
        fs::write(capsule.join("frontier.json"), "{}").unwrap();
        fs::write(capsule.join("old-evidence.txt"), "old").unwrap();
        let config = PanelInspectConfig {
            seeds: vec![1],
            artifact_store: BranchArtifactStore::new(&root),
            args_template: args(0),
            source_identity: source_identity(),
        };

        let summary =
            run_slices_with_executor(config, PanelRunOptions::drain(1).fresh(), OkExecutor)
                .unwrap();

        let archived_path = summary.rows[0].archived_capsule_path.as_ref().unwrap();

        assert_eq!(
            summary.rows[0].reuse_decision,
            PanelReuseDecision::FreshReplacedCapsule
        );
        assert_eq!(summary.rows[0].scheduler_action, PanelSeedAction::StartNew);
        assert!(!capsule.join("old-evidence.txt").exists());
        assert!(PathBuf::from(archived_path)
            .join("old-evidence.txt")
            .exists());

        let _ = fs::remove_dir_all(root);
    }
}
