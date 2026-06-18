use std::collections::BTreeMap;

use crate::eval::run_control::RunControlSession;

use super::model::{BranchCampaignBranchV1, BranchCampaignStateStoreSummaryV1};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct BranchStateNodeIdV1(usize);

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BranchStateNodeV1 {
    id: BranchStateNodeIdV1,
    parent_id: Option<BranchStateNodeIdV1>,
    commands: Vec<String>,
    added_commands: Vec<String>,
}

impl BranchStateNodeV1 {
    #[cfg(test)]
    pub(super) fn parent_id(&self) -> Option<BranchStateNodeIdV1> {
        self.parent_id
    }

    #[cfg(test)]
    pub(super) fn added_commands(&self) -> &[String] {
        &self.added_commands
    }
}

#[derive(Clone, Debug, Default)]
pub(super) struct BranchStateStoreV1 {
    sessions_by_commands: BTreeMap<Vec<String>, RunControlSession>,
    node_ids_by_commands: BTreeMap<Vec<String>, BranchStateNodeIdV1>,
    nodes: Vec<BranchStateNodeV1>,
    lookup_hits: usize,
    lookup_misses: usize,
    inserts: usize,
    retains: usize,
}

impl BranchStateStoreV1 {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn insert_session(&mut self, commands: Vec<String>, session: RunControlSession) {
        self.upsert_node(commands.clone(), None, commands.clone());
        self.sessions_by_commands.insert(commands, session);
        self.inserts = self.inserts.saturating_add(1);
    }

    pub(super) fn insert_child_session(
        &mut self,
        parent_commands: &[String],
        child_commands: Vec<String>,
        session: RunControlSession,
    ) {
        let parent_id = self.node_ids_by_commands.get(parent_commands).copied();
        let (parent_id, added_commands) =
            if parent_id.is_some() && child_commands.starts_with(parent_commands) {
                (
                    parent_id,
                    child_commands[parent_commands.len()..].to_vec(),
                )
            } else {
                (None, child_commands.clone())
            };
        self.upsert_node(child_commands.clone(), parent_id, added_commands);
        self.sessions_by_commands.insert(child_commands, session);
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

    #[cfg(test)]
    pub(super) fn node_id_for_commands(
        &self,
        commands: &[String],
    ) -> Option<BranchStateNodeIdV1> {
        self.node_ids_by_commands.get(commands).copied()
    }

    #[cfg(test)]
    pub(super) fn node_for_commands(
        &self,
        commands: &[String],
    ) -> Option<&BranchStateNodeV1> {
        let id = self.node_ids_by_commands.get(commands)?;
        self.nodes.get(id.0)
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
        self.retains = self.retains.saturating_add(1);
    }

    fn upsert_node(
        &mut self,
        commands: Vec<String>,
        parent_id: Option<BranchStateNodeIdV1>,
        added_commands: Vec<String>,
    ) -> BranchStateNodeIdV1 {
        if let Some(id) = self.node_ids_by_commands.get(&commands).copied() {
            if let Some(node) = self.nodes.get_mut(id.0) {
                if node.parent_id.is_none() && parent_id.is_some() {
                    node.parent_id = parent_id;
                    node.added_commands = added_commands;
                }
            }
            return id;
        }
        let id = BranchStateNodeIdV1(self.nodes.len());
        self.nodes.push(BranchStateNodeV1 {
            id,
            parent_id,
            commands: commands.clone(),
            added_commands,
        });
        self.node_ids_by_commands.insert(commands, id);
        id
    }

    pub(super) fn summary(&self) -> BranchCampaignStateStoreSummaryV1 {
        BranchCampaignStateStoreSummaryV1 {
            sessions: self.sessions_by_commands.len(),
            nodes: self.nodes.len(),
            linked_nodes: self
                .nodes
                .iter()
                .filter(|node| node.parent_id.is_some())
                .count(),
            lookup_hits: self.lookup_hits,
            lookup_misses: self.lookup_misses,
            inserts: self.inserts,
            retains: self.retains,
        }
    }
}
