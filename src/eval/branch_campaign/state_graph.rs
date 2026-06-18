use std::collections::BTreeMap;

use crate::eval::run_control::RunControlSession;

use super::model::BranchCampaignBranchV1;

#[cfg(test)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct BranchStateStoreStatsV1 {
    pub snapshot_count: usize,
    pub lookup_hits: usize,
    pub lookup_misses: usize,
    pub inserts: usize,
    pub retained: usize,
}

#[derive(Clone, Debug, Default)]
pub(super) struct BranchStateStoreV1 {
    sessions_by_commands: BTreeMap<Vec<String>, RunControlSession>,
    lookup_hits: usize,
    lookup_misses: usize,
    inserts: usize,
    retained: usize,
}

impl BranchStateStoreV1 {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn insert_session(&mut self, commands: Vec<String>, session: RunControlSession) {
        self.sessions_by_commands.insert(commands, session);
        self.inserts = self.inserts.saturating_add(1);
    }

    pub(super) fn get_session(&self, commands: &[String]) -> Option<&RunControlSession> {
        self.sessions_by_commands.get(commands)
    }

    pub(super) fn get_session_cloned(
        &mut self,
        commands: &[String],
    ) -> Option<RunControlSession> {
        let session = self.sessions_by_commands.get(commands).cloned();
        if session.is_some() {
            self.lookup_hits = self.lookup_hits.saturating_add(1);
        } else {
            self.lookup_misses = self.lookup_misses.saturating_add(1);
        }
        session
    }

    pub(super) fn contains_commands(&self, commands: &[String]) -> bool {
        self.sessions_by_commands.contains_key(commands)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.sessions_by_commands.is_empty()
    }

    pub(super) fn retain_for_branches(
        &mut self,
        active: &[BranchCampaignBranchV1],
        frozen: &[BranchCampaignBranchV1],
        abandoned: &[BranchCampaignBranchV1],
        stuck: &[BranchCampaignBranchV1],
    ) {
        let keep = active
            .iter()
            .chain(frozen.iter())
            .chain(abandoned.iter())
            .chain(stuck.iter())
            .map(|branch| branch.commands.clone())
            .collect::<std::collections::BTreeSet<_>>();
        self.sessions_by_commands
            .retain(|commands, _| keep.contains(commands));
        self.retained = self.retained.saturating_add(1);
    }

    #[cfg(test)]
    pub(super) fn stats(&self) -> BranchStateStoreStatsV1 {
        BranchStateStoreStatsV1 {
            snapshot_count: self.sessions_by_commands.len(),
            lookup_hits: self.lookup_hits,
            lookup_misses: self.lookup_misses,
            inserts: self.inserts,
            retained: self.retained,
        }
    }
}
