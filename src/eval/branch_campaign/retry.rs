use std::collections::BTreeMap;

use crate::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1,
};
use crate::eval::run_control::RunControlHpLossLimit;
use serde::{Deserialize, Serialize};

use super::model::BranchCampaignSelectionV1;
use super::selection_key::act_boss_floor_v1;
use super::{normalized_campaign_boundary_title, BranchCampaignConfigV1};

const COMBAT_RETRY_NODE_MULTIPLIER: usize = 4;
const COMBAT_RETRY_WALL_MULTIPLIER: u64 = 4;
const COMBAT_RETRY_MIN_NODES: usize = 200_000;
const COMBAT_RETRY_MAX_NODES: usize = 500_000;
const COMBAT_RETRY_MIN_WALL_MS: u64 = 1_000;
const COMBAT_RETRY_MAX_WALL_MS: u64 = 1_000;
pub(super) const BOSS_GATE_RETRY_ATTEMPTS_PER_GATE: usize = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BranchCampaignCombatRetryPolicyV1 {
    /// Keep moving through available branches first. If all routes stall on combat,
    /// the campaign will surface that as an intervention instead of retrying every parent.
    OnStall,
    /// Legacy behavior: immediately rerun a parent with a larger combat budget when all
    /// produced children are pruned combat branches.
    Immediate,
    Disabled,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCombatRetryLedgerV1 {
    #[serde(default)]
    pub boss_gate_attempts: Vec<BranchCampaignCombatRetryLedgerEntryV1>,
}

impl BranchCampaignCombatRetryLedgerV1 {
    pub(crate) fn is_empty(&self) -> bool {
        self.boss_gate_attempts.is_empty()
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignCombatRetryLedgerEntryV1 {
    pub act: u8,
    pub floor: i32,
    pub attempts: usize,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct BranchCampaignBossGateRetryKeyV1 {
    act: u8,
    floor: i32,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) struct BranchCampaignCombatRetryLedgerStateV1 {
    boss_gate_attempts: BTreeMap<BranchCampaignBossGateRetryKeyV1, usize>,
}

impl BranchCampaignCombatRetryLedgerStateV1 {
    pub(super) fn from_report_v1(report: &BranchCampaignCombatRetryLedgerV1) -> Self {
        let mut ledger = Self::default();
        for entry in &report.boss_gate_attempts {
            let key = BranchCampaignBossGateRetryKeyV1 {
                act: entry.act,
                floor: entry.floor,
            };
            ledger
                .boss_gate_attempts
                .insert(key, entry.attempts.min(BOSS_GATE_RETRY_ATTEMPTS_PER_GATE));
        }
        ledger
    }

    pub(super) fn to_report_v1(&self) -> BranchCampaignCombatRetryLedgerV1 {
        BranchCampaignCombatRetryLedgerV1 {
            boss_gate_attempts: self
                .boss_gate_attempts
                .iter()
                .map(|(key, attempts)| BranchCampaignCombatRetryLedgerEntryV1 {
                    act: key.act,
                    floor: key.floor,
                    attempts: *attempts,
                })
                .collect(),
        }
    }

    pub(super) fn try_consume_selection_boss_gate_retry_v1(
        &mut self,
        selection: &BranchCampaignSelectionV1,
    ) -> bool {
        campaign_selection_act_boss_gate_retry_key_v1(selection)
            .map(|key| self.try_consume_boss_gate_retry_v1(key))
            .unwrap_or(true)
    }

    fn try_consume_boss_gate_retry_v1(&mut self, key: BranchCampaignBossGateRetryKeyV1) -> bool {
        let attempts = self.boss_gate_attempts.entry(key).or_default();
        if *attempts >= BOSS_GATE_RETRY_ATTEMPTS_PER_GATE {
            return false;
        }
        *attempts = attempts.saturating_add(1);
        true
    }
}

pub(super) fn combat_retry_campaign_config_v1(
    config: &BranchCampaignConfigV1,
) -> Option<BranchCampaignConfigV1> {
    let retry_nodes = retry_node_budget_v1(config.search_max_nodes);
    let retry_wall_ms = config
        .combat_retry_wall_ms
        .or_else(|| retry_wall_budget_v1(config.search_wall_ms));
    if retry_nodes == config.search_max_nodes && retry_wall_ms == config.search_wall_ms {
        return None;
    }

    let mut retry_config = config.clone();
    retry_config.search_max_nodes = retry_nodes;
    retry_config.search_wall_ms = retry_wall_ms;
    retry_config.max_branches_per_active = combat_retry_branch_width_v1(config);
    retry_config.search_max_hp_loss = config
        .search_max_hp_loss
        .or(Some(RunControlHpLossLimit::Unlimited));
    Some(retry_config)
}

fn combat_retry_branch_width_v1(config: &BranchCampaignConfigV1) -> usize {
    config.max_branches_per_active.min(config.max_active.max(1))
}

fn retry_node_budget_v1(current: Option<usize>) -> Option<usize> {
    let base = current.unwrap_or(COMBAT_RETRY_MIN_NODES);
    Some(
        base.saturating_mul(COMBAT_RETRY_NODE_MULTIPLIER)
            .max(COMBAT_RETRY_MIN_NODES)
            .min(COMBAT_RETRY_MAX_NODES),
    )
}

fn retry_wall_budget_v1(current: Option<u64>) -> Option<u64> {
    let base = current.unwrap_or(COMBAT_RETRY_MIN_WALL_MS);
    Some(
        base.saturating_mul(COMBAT_RETRY_WALL_MULTIPLIER)
            .max(COMBAT_RETRY_MIN_WALL_MS)
            .min(COMBAT_RETRY_MAX_WALL_MS),
    )
}

pub(super) fn branch_report_needs_combat_budget_retry_v1(
    branches: &[BranchExperimentBranchReportV1],
) -> bool {
    !branches.is_empty()
        && branches
            .iter()
            .all(|branch| branch.status == BranchExperimentBranchStatusV1::Pruned)
        && branches.iter().all(|branch| {
            normalized_campaign_boundary_title(&branch.summary.boundary_title) == "combat"
        })
}

pub(super) fn campaign_parent_should_retry_combat_budget_now_v1(
    config: &BranchCampaignConfigV1,
    branches: &[BranchExperimentBranchReportV1],
) -> bool {
    if matches!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::Disabled
    ) {
        return false;
    }
    if !branch_report_needs_combat_budget_retry_v1(branches) {
        return false;
    }
    matches!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::Immediate
    ) || branch_report_is_act_boss_gate_combat_retry_candidate_v1(branches)
}

fn branch_report_is_act_boss_gate_combat_retry_candidate_v1(
    branches: &[BranchExperimentBranchReportV1],
) -> bool {
    branch_report_act_boss_gate_retry_key_v1(branches).is_some()
}

pub(super) fn try_consume_branch_report_act_boss_gate_retry_v1(
    ledger: &mut BranchCampaignCombatRetryLedgerStateV1,
    branches: &[BranchExperimentBranchReportV1],
) -> bool {
    branch_report_act_boss_gate_retry_key_v1(branches)
        .map(|key| ledger.try_consume_boss_gate_retry_v1(key))
        .unwrap_or(true)
}

fn branch_report_act_boss_gate_retry_key_v1(
    branches: &[BranchExperimentBranchReportV1],
) -> Option<BranchCampaignBossGateRetryKeyV1> {
    branches.iter().find_map(|branch| {
        let act = branch.summary.act;
        let floor = branch.summary.floor;
        (normalized_campaign_boundary_title(&branch.summary.boundary_title) == "combat"
            && floor >= act_boss_floor_v1(act))
        .then_some(BranchCampaignBossGateRetryKeyV1 {
            act,
            floor: act_boss_floor_v1(act),
        })
    })
}

fn campaign_selection_act_boss_gate_retry_key_v1(
    selection: &BranchCampaignSelectionV1,
) -> Option<BranchCampaignBossGateRetryKeyV1> {
    selection.abandoned.iter().find_map(|branch| {
        let summary = branch.summary.as_ref()?;
        (normalized_campaign_boundary_title(&branch.frontier_title) == "combat"
            && summary.floor >= act_boss_floor_v1(summary.act))
        .then_some(BranchCampaignBossGateRetryKeyV1 {
            act: summary.act,
            floor: act_boss_floor_v1(summary.act),
        })
    })
}

pub(super) fn campaign_round_should_retry_combat_budget_on_stall_v1(
    config: &BranchCampaignConfigV1,
    selection: &BranchCampaignSelectionV1,
    existing_frozen_branches: usize,
) -> bool {
    matches!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::OnStall
    ) && combat_retry_campaign_config_v1(config).is_some()
        && existing_frozen_branches == 0
        && selection.active.is_empty()
        && selection.frozen.is_empty()
        && selection.victories.is_empty()
        && selection.dead.is_empty()
        && selection.stuck.is_empty()
        && !selection.abandoned.is_empty()
        && selection
            .abandoned
            .iter()
            .all(|branch| normalized_campaign_boundary_title(&branch.frontier_title) == "combat")
}
