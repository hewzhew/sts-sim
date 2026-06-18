use serde::{Deserialize, Serialize};

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
