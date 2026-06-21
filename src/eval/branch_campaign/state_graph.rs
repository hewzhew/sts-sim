use std::collections::BTreeMap;
use std::collections::BTreeSet;

use crate::eval::branch_experiment::{
    branch_experiment_commands_include_decision_parent_coordinate_v1,
    branch_experiment_commands_include_route_decision_parent_coordinate_v1,
};
use crate::eval::run_control::RunControlSession;

use super::model::{
    BranchCampaignBranchV1, BranchCampaignCheckpointNodeV1, BranchCampaignStateStoreSummaryV1,
};

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
    replay_exact_hits: usize,
    replay_ancestor_hits: usize,
    replay_misses: usize,
    replay_suffix_commands_sum: usize,
    replay_suffix_commands_max: usize,
    sessions_pruned: usize,
    anchor_sessions_kept: usize,
    inserts: usize,
    retains: usize,
}

#[derive(Clone, Debug)]
pub(super) struct BranchStateReplayStartV1 {
    pub(super) session: RunControlSession,
    pub(super) suffix_commands: Vec<String>,
    #[cfg(test)]
    pub(super) source: BranchStateReplayStartSourceV1,
}

#[cfg(test)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum BranchStateReplayStartSourceV1 {
    Exact,
    Ancestor,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct BranchStateSessionRetentionPolicyV1 {
    pub(super) max_frozen_exact_sessions: usize,
    pub(super) max_stuck_exact_sessions: usize,
    pub(super) max_abandoned_exact_sessions: usize,
    pub(super) max_suffix_commands_without_session: usize,
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
                (parent_id, child_commands[parent_commands.len()..].to_vec())
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

    pub(super) fn replay_start_for_commands(
        &mut self,
        commands: &[String],
    ) -> Option<BranchStateReplayStartV1> {
        if let Some(session) = self.sessions_by_commands.get(commands).cloned() {
            self.lookup_hits = self.lookup_hits.saturating_add(1);
            self.replay_exact_hits = self.replay_exact_hits.saturating_add(1);
            return Some(BranchStateReplayStartV1 {
                session,
                suffix_commands: Vec::new(),
                #[cfg(test)]
                source: BranchStateReplayStartSourceV1::Exact,
            });
        }

        if let Some(replay_start) = self.longest_session_prefix_replay_start(commands) {
            return Some(replay_start);
        }

        let mut current = self.node_ids_by_commands.get(commands).copied();
        let mut suffix_segments = Vec::<Vec<String>>::new();
        while let Some(id) = current {
            let Some(node) = self.nodes.get(id.0) else {
                break;
            };
            if let Some(session) = self.sessions_by_commands.get(&node.commands).cloned() {
                suffix_segments.reverse();
                let suffix_commands = suffix_segments.into_iter().flatten().collect::<Vec<_>>();
                self.lookup_hits = self.lookup_hits.saturating_add(1);
                self.replay_ancestor_hits = self.replay_ancestor_hits.saturating_add(1);
                self.replay_suffix_commands_sum = self
                    .replay_suffix_commands_sum
                    .saturating_add(suffix_commands.len());
                self.replay_suffix_commands_max =
                    self.replay_suffix_commands_max.max(suffix_commands.len());
                return Some(BranchStateReplayStartV1 {
                    session,
                    suffix_commands,
                    #[cfg(test)]
                    source: BranchStateReplayStartSourceV1::Ancestor,
                });
            }
            suffix_segments.push(node.added_commands.clone());
            current = node.parent_id;
        }

        self.lookup_misses = self.lookup_misses.saturating_add(1);
        self.replay_misses = self.replay_misses.saturating_add(1);
        None
    }

    fn longest_session_prefix_replay_start(
        &mut self,
        commands: &[String],
    ) -> Option<BranchStateReplayStartV1> {
        if let Some((prefix_commands, session)) = self
            .sessions_by_commands
            .iter()
            .filter(|(prefix_commands, _)| {
                !prefix_commands.is_empty()
                    && prefix_commands.len() < commands.len()
                    && commands.starts_with(prefix_commands)
            })
            .max_by_key(|(prefix_commands, _)| prefix_commands.len())
            .map(|(prefix_commands, session)| (prefix_commands.clone(), session.clone()))
        {
            let suffix_commands = commands[prefix_commands.len()..].to_vec();
            self.lookup_hits = self.lookup_hits.saturating_add(1);
            self.replay_ancestor_hits = self.replay_ancestor_hits.saturating_add(1);
            self.replay_suffix_commands_sum = self
                .replay_suffix_commands_sum
                .saturating_add(suffix_commands.len());
            self.replay_suffix_commands_max =
                self.replay_suffix_commands_max.max(suffix_commands.len());
            return Some(BranchStateReplayStartV1 {
                session,
                suffix_commands,
                #[cfg(test)]
                source: BranchStateReplayStartSourceV1::Ancestor,
            });
        }
        None
    }

    pub(super) fn contains_commands(&self, commands: &[String]) -> bool {
        self.sessions_by_commands.contains_key(commands)
    }

    #[cfg(test)]
    pub(super) fn node_id_for_commands(&self, commands: &[String]) -> Option<BranchStateNodeIdV1> {
        self.node_ids_by_commands.get(commands).copied()
    }

    #[cfg(test)]
    pub(super) fn node_for_commands(&self, commands: &[String]) -> Option<&BranchStateNodeV1> {
        let id = self.node_ids_by_commands.get(commands)?;
        self.nodes.get(id.0)
    }

    pub(super) fn is_empty(&self) -> bool {
        self.sessions_by_commands.is_empty()
    }

    pub(super) fn checkpoint_nodes(&self) -> Vec<BranchCampaignCheckpointNodeV1> {
        self.nodes
            .iter()
            .map(|node| BranchCampaignCheckpointNodeV1 {
                node_id: node.id.0,
                parent_id: node.parent_id.map(|parent_id| parent_id.0),
                commands: node.commands.clone(),
                added_commands: node.added_commands.clone(),
            })
            .collect()
    }

    pub(super) fn restore_checkpoint_nodes(
        &mut self,
        nodes: &[BranchCampaignCheckpointNodeV1],
    ) -> Result<(), String> {
        self.nodes.clear();
        self.node_ids_by_commands.clear();

        let mut records = nodes.to_vec();
        records.sort_by_key(|node| node.node_id);
        for (expected_id, node) in records.iter().enumerate() {
            if node.node_id != expected_id {
                return Err(format!(
                    "campaign checkpoint node ids must be contiguous: expected {}, found {}",
                    expected_id, node.node_id
                ));
            }
            if let Some(parent_id) = node.parent_id {
                if parent_id > node.node_id {
                    return Err(format!(
                        "campaign checkpoint node {} has invalid parent {}",
                        node.node_id, parent_id
                    ));
                }
            }
        }

        for node in records {
            let id = BranchStateNodeIdV1(node.node_id);
            let parent_id = node
                .parent_id
                .filter(|parent_id| *parent_id != node.node_id)
                .map(BranchStateNodeIdV1);
            self.nodes.push(BranchStateNodeV1 {
                id,
                parent_id,
                commands: node.commands.clone(),
                added_commands: node.added_commands.clone(),
            });
            self.node_ids_by_commands.insert(node.commands, id);
        }
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn retain_for_branches(
        &mut self,
        active: &[BranchCampaignBranchV1],
        frozen: &[BranchCampaignBranchV1],
        abandoned: &[BranchCampaignBranchV1],
        stuck: &[BranchCampaignBranchV1],
    ) {
        self.retain_for_branches_with_session_policy(
            active,
            frozen,
            abandoned,
            stuck,
            BranchStateSessionRetentionPolicyV1 {
                max_frozen_exact_sessions: usize::MAX,
                max_stuck_exact_sessions: usize::MAX,
                max_abandoned_exact_sessions: usize::MAX,
                max_suffix_commands_without_session: usize::MAX,
            },
        );
    }

    pub(super) fn retain_for_branches_with_session_policy(
        &mut self,
        active: &[BranchCampaignBranchV1],
        frozen: &[BranchCampaignBranchV1],
        abandoned: &[BranchCampaignBranchV1],
        stuck: &[BranchCampaignBranchV1],
        policy: BranchStateSessionRetentionPolicyV1,
    ) {
        self.retain_for_branches_with_session_policy_and_anchors(
            active,
            frozen,
            abandoned,
            stuck,
            &BTreeSet::new(),
            policy,
        );
    }

    pub(super) fn retain_for_branches_with_session_policy_and_anchors(
        &mut self,
        active: &[BranchCampaignBranchV1],
        frozen: &[BranchCampaignBranchV1],
        abandoned: &[BranchCampaignBranchV1],
        stuck: &[BranchCampaignBranchV1],
        anchor_commands: &BTreeSet<Vec<String>>,
        policy: BranchStateSessionRetentionPolicyV1,
    ) {
        let keep = active
            .iter()
            .chain(frozen.iter())
            .chain(abandoned.iter())
            .chain(stuck.iter())
            .map(|branch| branch.commands.clone())
            .chain(anchor_commands.iter().cloned())
            .collect::<BTreeSet<_>>();
        let mut keep_sessions = active
            .iter()
            .map(|branch| branch.commands.clone())
            .collect::<BTreeSet<_>>();
        keep_sessions.extend(anchor_commands.iter().cloned());
        keep_sessions.extend(
            frozen
                .iter()
                .take(policy.max_frozen_exact_sessions)
                .map(|branch| branch.commands.clone()),
        );
        keep_sessions.extend(
            stuck
                .iter()
                .take(policy.max_stuck_exact_sessions)
                .map(|branch| branch.commands.clone()),
        );
        keep_sessions.extend(
            abandoned
                .iter()
                .take(policy.max_abandoned_exact_sessions)
                .map(|branch| branch.commands.clone()),
        );
        self.add_long_suffix_session_anchors(&keep, &mut keep_sessions, policy);
        let before_sessions = self.sessions_by_commands.len();
        self.sessions_by_commands
            .retain(|commands, _| keep_sessions.contains(commands));
        self.sessions_pruned = self
            .sessions_pruned
            .saturating_add(before_sessions.saturating_sub(self.sessions_by_commands.len()));
        self.retain_nodes_for_commands_and_ancestors(&keep);
        self.retains = self.retains.saturating_add(1);
    }

    fn add_long_suffix_session_anchors(
        &mut self,
        branch_commands: &BTreeSet<Vec<String>>,
        keep_sessions: &mut BTreeSet<Vec<String>>,
        policy: BranchStateSessionRetentionPolicyV1,
    ) {
        if policy.max_suffix_commands_without_session == usize::MAX {
            return;
        }
        for commands in branch_commands {
            if keep_sessions.contains(commands) || !self.sessions_by_commands.contains_key(commands)
            {
                continue;
            }
            if self.suffix_command_len_to_nearest_kept_session(commands, keep_sessions)
                > policy.max_suffix_commands_without_session
            {
                keep_sessions.insert(commands.clone());
                self.anchor_sessions_kept = self.anchor_sessions_kept.saturating_add(1);
            }
        }
    }

    fn suffix_command_len_to_nearest_kept_session(
        &self,
        commands: &[String],
        keep_sessions: &BTreeSet<Vec<String>>,
    ) -> usize {
        let mut current = self.node_ids_by_commands.get(commands).copied();
        let mut suffix_len = 0usize;
        while let Some(id) = current {
            let Some(node) = self.nodes.get(id.0) else {
                break;
            };
            if keep_sessions.contains(&node.commands) {
                return suffix_len;
            }
            suffix_len = suffix_len.saturating_add(node.added_commands.len());
            current = node.parent_id;
        }
        commands.len()
    }

    fn upsert_node(
        &mut self,
        commands: Vec<String>,
        parent_id: Option<BranchStateNodeIdV1>,
        added_commands: Vec<String>,
    ) -> BranchStateNodeIdV1 {
        if let Some(id) = self.node_ids_by_commands.get(&commands).copied() {
            if let Some(node) = self.nodes.get_mut(id.0) {
                if node.parent_id.is_none() && parent_id.is_some_and(|parent_id| parent_id != id) {
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

    fn retain_nodes_for_commands_and_ancestors(&mut self, commands: &BTreeSet<Vec<String>>) {
        let mut keep_ids = BTreeSet::<BranchStateNodeIdV1>::new();
        for command_path in commands {
            let mut current = self.node_ids_by_commands.get(command_path).copied();
            while let Some(id) = current {
                if !keep_ids.insert(id) {
                    break;
                }
                current = self.nodes.get(id.0).and_then(|node| node.parent_id);
            }
        }
        if keep_ids.len() == self.nodes.len() {
            return;
        }

        let mut old_to_new = BTreeMap::<BranchStateNodeIdV1, BranchStateNodeIdV1>::new();
        let mut new_nodes = Vec::<BranchStateNodeV1>::new();
        for node in &self.nodes {
            if keep_ids.contains(&node.id) {
                let new_id = BranchStateNodeIdV1(new_nodes.len());
                old_to_new.insert(node.id, new_id);
                new_nodes.push(BranchStateNodeV1 {
                    id: new_id,
                    parent_id: node.parent_id,
                    commands: node.commands.clone(),
                    added_commands: node.added_commands.clone(),
                });
            }
        }
        for node in &mut new_nodes {
            node.parent_id = node
                .parent_id
                .and_then(|parent_id| old_to_new.get(&parent_id).copied());
        }
        self.node_ids_by_commands.clear();
        for node in &new_nodes {
            self.node_ids_by_commands
                .insert(node.commands.clone(), node.id);
        }
        self.nodes = new_nodes;
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
            replay_exact_hits: self.replay_exact_hits,
            replay_ancestor_hits: self.replay_ancestor_hits,
            replay_misses: self.replay_misses,
            replay_suffix_commands_sum: self.replay_suffix_commands_sum,
            replay_suffix_commands_max: self.replay_suffix_commands_max,
            sessions_pruned: self.sessions_pruned,
            anchor_sessions_kept: self.anchor_sessions_kept,
            decision_coordinate_nodes: self
                .nodes
                .iter()
                .filter(|node| {
                    branch_experiment_commands_include_decision_parent_coordinate_v1(&node.commands)
                })
                .count(),
            route_decision_coordinate_nodes: self
                .nodes
                .iter()
                .filter(|node| {
                    branch_experiment_commands_include_route_decision_parent_coordinate_v1(
                        &node.commands,
                    )
                })
                .count(),
            decision_coordinate_sessions: self
                .sessions_by_commands
                .keys()
                .filter(|commands| {
                    branch_experiment_commands_include_decision_parent_coordinate_v1(commands)
                })
                .count(),
            route_decision_coordinate_sessions: self
                .sessions_by_commands
                .keys()
                .filter(|commands| {
                    branch_experiment_commands_include_route_decision_parent_coordinate_v1(commands)
                })
                .count(),
            inserts: self.inserts,
            retains: self.retains,
        }
    }
}
