use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use crate::sim::combat_action::combat_action_key;
use crate::state::core::ClientInput;
use std::collections::{BTreeMap, BTreeSet};

use super::{OracleAnalysisSessionV1, OracleRunBoundaryV1};

mod delta;
mod search;
#[cfg(test)]
mod tests;
mod types;
mod view;

pub use types::*;
use view::{
    action_surface_view, exact_hash, position_view, resolve_action_ref, resolve_action_selector,
};

#[derive(Clone)]
pub(super) struct OracleAnalysisCombatScratchV1 {
    run_node_id: usize,
    context: OracleAnalysisCombatScratchContextV1,
    root_exact_state_hash: String,
    max_engine_steps_per_transition: usize,
    cursor_scratch_node_id: u64,
    next_scratch_node_id: u64,
    nodes: BTreeMap<u64, OracleAnalysisCombatScratchNodeCheckpointV1>,
    positions: BTreeMap<u64, CombatPosition>,
}

impl OracleAnalysisCombatScratchV1 {
    pub(super) fn start(
        run_node_id: usize,
        context: OracleAnalysisCombatScratchContextV1,
        root: CombatPosition,
        max_engine_steps_per_transition: usize,
    ) -> Result<Self, String> {
        if max_engine_steps_per_transition == 0 {
            return Err("combat scratch transition budget must be positive".to_string());
        }
        if EngineCombatStepper.terminal(&root) != CombatTerminal::Unresolved {
            return Err("combat scratch requires an unresolved combat root".to_string());
        }
        let root_exact_state_hash = exact_hash(&root);
        let root_node = OracleAnalysisCombatScratchNodeCheckpointV1 {
            scratch_node_id: 0,
            parent_scratch_node_id: None,
            input: None,
            exact_state_hash: root_exact_state_hash.clone(),
        };
        Ok(Self {
            run_node_id,
            context,
            root_exact_state_hash,
            max_engine_steps_per_transition,
            cursor_scratch_node_id: 0,
            next_scratch_node_id: 1,
            nodes: BTreeMap::from([(0, root_node)]),
            positions: BTreeMap::from([(0, root)]),
        })
    }

    pub(super) fn restore(
        checkpoint: OracleAnalysisCombatScratchCheckpointV1,
        context: OracleAnalysisCombatScratchContextV1,
        root: CombatPosition,
    ) -> Result<Self, String> {
        if checkpoint.schema_name != ORACLE_ANALYSIS_COMBAT_SCRATCH_SCHEMA_NAME
            || checkpoint.schema_version != ORACLE_ANALYSIS_COMBAT_SCRATCH_SCHEMA_VERSION
        {
            return Err("unsupported oracle analysis combat scratch schema".to_string());
        }
        if checkpoint.max_engine_steps_per_transition == 0 {
            return Err("combat scratch transition budget must be positive".to_string());
        }
        let actual_root_hash = exact_hash(&root);
        if checkpoint.root_exact_state_hash != actual_root_hash {
            return Err(format!(
                "combat scratch root changed: checkpoint {}, run node {}",
                checkpoint.root_exact_state_hash, actual_root_hash
            ));
        }
        let mut nodes = BTreeMap::new();
        for node in checkpoint.nodes {
            if nodes.insert(node.scratch_node_id, node).is_some() {
                return Err("combat scratch checkpoint contains duplicate node ids".to_string());
            }
        }
        let root_node = nodes
            .get(&0)
            .ok_or_else(|| "combat scratch checkpoint has no root node 0".to_string())?;
        if root_node.parent_scratch_node_id.is_some()
            || root_node.input.is_some()
            || root_node.exact_state_hash != actual_root_hash
        {
            return Err("combat scratch root node is not the bound exact root".to_string());
        }
        if nodes.values().any(|node| {
            node.scratch_node_id != 0
                && (node.parent_scratch_node_id.is_none() || node.input.is_none())
        }) {
            return Err("combat scratch non-root node is missing its delta".to_string());
        }
        if !nodes.contains_key(&checkpoint.cursor_scratch_node_id) {
            return Err(format!(
                "combat scratch cursor references missing node {}",
                checkpoint.cursor_scratch_node_id
            ));
        }
        let maximum_node_id = nodes.keys().next_back().copied().unwrap_or(0);
        if checkpoint.next_scratch_node_id <= maximum_node_id {
            return Err("combat scratch next node id is not above retained nodes".to_string());
        }

        let mut positions = BTreeMap::from([(0, root)]);
        let mut pending = nodes
            .keys()
            .copied()
            .filter(|node_id| *node_id != 0)
            .collect::<BTreeSet<_>>();
        while !pending.is_empty() {
            let mut progressed = false;
            for node_id in pending.iter().copied().collect::<Vec<_>>() {
                let node = nodes
                    .get(&node_id)
                    .expect("pending scratch node comes from node map");
                let parent_id = node
                    .parent_scratch_node_id
                    .expect("non-root parent validated above");
                let Some(parent) = positions.get(&parent_id) else {
                    continue;
                };
                let input = node.input.clone().expect("non-root input validated above");
                let successor =
                    apply_exact(parent, input, checkpoint.max_engine_steps_per_transition)?;
                let actual = exact_hash(&successor);
                if node.exact_state_hash != actual {
                    return Err(format!(
                        "combat scratch node {node_id} exact hash changed: checkpoint {}, replay {actual}",
                        node.exact_state_hash
                    ));
                }
                positions.insert(node_id, successor);
                pending.remove(&node_id);
                progressed = true;
            }
            if !progressed {
                return Err(
                    "combat scratch checkpoint contains a cycle or missing parent".to_string(),
                );
            }
        }

        Ok(Self {
            run_node_id: checkpoint.run_node_id,
            context,
            root_exact_state_hash: checkpoint.root_exact_state_hash,
            max_engine_steps_per_transition: checkpoint.max_engine_steps_per_transition,
            cursor_scratch_node_id: checkpoint.cursor_scratch_node_id,
            next_scratch_node_id: checkpoint.next_scratch_node_id,
            nodes,
            positions,
        })
    }

    pub(super) fn checkpoint(&self) -> OracleAnalysisCombatScratchCheckpointV1 {
        OracleAnalysisCombatScratchCheckpointV1 {
            schema_name: ORACLE_ANALYSIS_COMBAT_SCRATCH_SCHEMA_NAME.to_string(),
            schema_version: ORACLE_ANALYSIS_COMBAT_SCRATCH_SCHEMA_VERSION,
            run_node_id: self.run_node_id,
            root_exact_state_hash: self.root_exact_state_hash.clone(),
            max_engine_steps_per_transition: self.max_engine_steps_per_transition,
            cursor_scratch_node_id: self.cursor_scratch_node_id,
            next_scratch_node_id: self.next_scratch_node_id,
            nodes: self.nodes.values().cloned().collect(),
        }
    }

    fn view(
        &self,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        self.view_at(
            self.cursor_scratch_node_id,
            selection_offset,
            selection_limit,
        )
    }

    fn view_at(
        &self,
        scratch_node_id: u64,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        if selection_limit == 0 || selection_limit > 64 {
            return Err("combat scratch selection page limit must be in 1..=64".to_string());
        }
        let node = self
            .nodes
            .get(&scratch_node_id)
            .ok_or_else(|| format!("unknown combat scratch node {scratch_node_id}"))?;
        let position = self
            .positions
            .get(&scratch_node_id)
            .ok_or_else(|| format!("combat scratch node {scratch_node_id} has no position"))?;
        Ok(OracleAnalysisCombatScratchViewV1 {
            run_node_id: self.run_node_id,
            context: self.context.clone(),
            root_exact_state_hash: self.root_exact_state_hash.clone(),
            max_engine_steps_per_transition: self.max_engine_steps_per_transition,
            cursor_scratch_node_id: scratch_node_id,
            scratch_node_count: self.nodes.len(),
            parent_scratch_node_id: node.parent_scratch_node_id,
            input_from_parent: node.input.clone(),
            position: position_view(position),
            legal_actions: action_surface_view(
                position,
                scratch_node_id,
                selection_offset,
                selection_limit,
            )?,
        })
    }

    fn play(
        &mut self,
        action_ref: &str,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        let parent = self.current_position()?.clone();
        let input = resolve_action_ref(&parent, action_ref)?;
        self.play_input(input)?;
        self.view(selection_offset, selection_limit)
    }

    fn play_selector(
        &mut self,
        selector: OracleAnalysisCombatScratchActionSelectorV1,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        let mut candidate = self.clone();
        let source_node_id = selector.scratch_node_id();
        if !candidate.nodes.contains_key(&source_node_id) {
            return Err(format!("unknown combat scratch node {source_node_id}"));
        }
        candidate.cursor_scratch_node_id = source_node_id;
        let input = resolve_action_selector(candidate.current_position()?, selector)?;
        candidate.play_input(input)?;
        let view = candidate.view(selection_offset, selection_limit)?;
        *self = candidate;
        Ok(view)
    }

    fn play_input(&mut self, input: ClientInput) -> Result<u64, String> {
        let parent_id = self.cursor_scratch_node_id;
        let parent = self.current_position()?.clone();
        if EngineCombatStepper.terminal(&parent) != CombatTerminal::Unresolved {
            return Err(
                "combat scratch cursor is terminal and cannot accept another action".to_string(),
            );
        }
        let successor = apply_exact(&parent, input.clone(), self.max_engine_steps_per_transition)?;
        let successor_hash = exact_hash(&successor);

        if let Some(existing) = self.nodes.values().find(|node| {
            node.parent_scratch_node_id == Some(parent_id)
                && node.input.as_ref() == Some(&input)
                && node.exact_state_hash == successor_hash
        }) {
            self.cursor_scratch_node_id = existing.scratch_node_id;
            return Ok(existing.scratch_node_id);
        }

        let scratch_node_id = self.next_scratch_node_id;
        self.next_scratch_node_id = self.next_scratch_node_id.saturating_add(1);
        self.nodes.insert(
            scratch_node_id,
            OracleAnalysisCombatScratchNodeCheckpointV1 {
                scratch_node_id,
                parent_scratch_node_id: Some(parent_id),
                input: Some(input),
                exact_state_hash: successor_hash,
            },
        );
        self.positions.insert(scratch_node_id, successor);
        self.cursor_scratch_node_id = scratch_node_id;
        Ok(scratch_node_id)
    }

    fn append_inputs_atomically(
        &mut self,
        inputs: &[ClientInput],
    ) -> Result<(Option<u64>, Option<u64>), String> {
        let mut candidate = self.clone();
        let mut first = None;
        let mut last = None;
        for input in inputs {
            let node_id = candidate.play_input(input.clone())?;
            first.get_or_insert(node_id);
            last = Some(node_id);
        }
        *self = candidate;
        Ok((first, last))
    }

    fn focus(
        &mut self,
        scratch_node_id: u64,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        if !self.nodes.contains_key(&scratch_node_id) {
            return Err(format!("unknown combat scratch node {scratch_node_id}"));
        }
        self.cursor_scratch_node_id = scratch_node_id;
        self.view(selection_offset, selection_limit)
    }

    fn back(
        &mut self,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        let parent = self
            .current_node()?
            .parent_scratch_node_id
            .ok_or_else(|| "combat scratch cursor is already at its root".to_string())?;
        self.cursor_scratch_node_id = parent;
        self.view(selection_offset, selection_limit)
    }

    fn tree(&self) -> Result<OracleAnalysisCombatScratchTreeV1, String> {
        let mut views = Vec::with_capacity(self.nodes.len());
        for node in self.nodes.values() {
            let position = self.positions.get(&node.scratch_node_id).ok_or_else(|| {
                format!(
                    "combat scratch node {} has no replayed position",
                    node.scratch_node_id
                )
            })?;
            let action_key_from_parent = match (node.parent_scratch_node_id, node.input.as_ref()) {
                (Some(parent_id), Some(input)) => {
                    let parent = self.positions.get(&parent_id).ok_or_else(|| {
                        format!(
                            "combat scratch node {} has no parent position",
                            node.scratch_node_id
                        )
                    })?;
                    Some(combat_action_key(&parent.combat, input))
                }
                _ => None,
            };
            views.push(OracleAnalysisCombatScratchTreeNodeV1 {
                scratch_node_id: node.scratch_node_id,
                parent_scratch_node_id: node.parent_scratch_node_id,
                is_cursor: node.scratch_node_id == self.cursor_scratch_node_id,
                input_from_parent: node.input.clone(),
                action_key_from_parent,
                exact_state_hash: node.exact_state_hash.clone(),
                terminal: EngineCombatStepper.terminal(position),
                turn: position.combat.turn.turn_count,
                player_hp: position.combat.entities.player.current_hp,
                player_block: position.combat.entities.player.block,
                enemy_hp_total: position
                    .combat
                    .entities
                    .monsters
                    .iter()
                    .filter(|monster| monster.is_alive_for_action())
                    .map(|monster| monster.current_hp.max(0))
                    .sum(),
            });
        }
        Ok(OracleAnalysisCombatScratchTreeV1 {
            run_node_id: self.run_node_id,
            root_exact_state_hash: self.root_exact_state_hash.clone(),
            cursor_scratch_node_id: self.cursor_scratch_node_id,
            nodes: views,
        })
    }

    fn current_node(&self) -> Result<&OracleAnalysisCombatScratchNodeCheckpointV1, String> {
        self.nodes.get(&self.cursor_scratch_node_id).ok_or_else(|| {
            format!(
                "combat scratch cursor references missing node {}",
                self.cursor_scratch_node_id
            )
        })
    }

    fn current_position(&self) -> Result<&CombatPosition, String> {
        self.positions
            .get(&self.cursor_scratch_node_id)
            .ok_or_else(|| {
                format!(
                    "combat scratch cursor has no replayed position {}",
                    self.cursor_scratch_node_id
                )
            })
    }

    fn cursor_actions(&self) -> Result<Vec<ClientInput>, String> {
        let mut actions = Vec::new();
        let mut node_id = self.cursor_scratch_node_id;
        let mut visited = BTreeSet::new();
        while node_id != 0 {
            if !visited.insert(node_id) {
                return Err("combat scratch path contains a cycle".to_string());
            }
            let node = self
                .nodes
                .get(&node_id)
                .ok_or_else(|| format!("combat scratch path references missing node {node_id}"))?;
            actions.push(
                node.input
                    .clone()
                    .ok_or_else(|| format!("combat scratch node {node_id} has no input delta"))?,
            );
            node_id = node.parent_scratch_node_id.ok_or_else(|| {
                format!("combat scratch node {node_id} has no parent before root")
            })?;
        }
        actions.reverse();
        Ok(actions)
    }
}

impl OracleAnalysisSessionV1 {
    pub fn start_combat_scratch(
        &mut self,
        run_node_id: Option<usize>,
        max_engine_steps_per_transition: usize,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        if self.combat_scratch.is_some() {
            return Err(
                "oracle analysis workspace already has an active combat scratch".to_string(),
            );
        }
        let run_node_id = run_node_id.unwrap_or(self.cursor_node_id);
        let branch = self.require_branch(run_node_id)?;
        if branch.boundary != OracleRunBoundaryV1::Combat {
            return Err(format!(
                "oracle analysis node {run_node_id} is at {:?}, not combat",
                branch.boundary
            ));
        }
        let context = OracleAnalysisCombatScratchContextV1 {
            act: branch.session.run_state.act_num,
            floor: branch.session.run_state.floor_num,
            gold: branch.session.run_state.gold,
        };
        let root = branch.session.current_active_combat_position()?;
        let scratch = OracleAnalysisCombatScratchV1::start(
            run_node_id,
            context,
            root,
            max_engine_steps_per_transition,
        )?;
        let view = scratch.view(selection_offset, selection_limit)?;
        self.combat_scratch = Some(scratch);
        Ok(view)
    }

    pub fn combat_scratch_view(
        &self,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        self.combat_scratch
            .as_ref()
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())?
            .view(selection_offset, selection_limit)
    }

    pub fn combat_scratch_decision_view(
        &self,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchDecisionViewV1, String> {
        self.combat_scratch_view(selection_offset, selection_limit)
            .map(Into::into)
    }

    pub fn combat_scratch_decision_view_at(
        &self,
        scratch_node_id: u64,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchDecisionViewV1, String> {
        self.combat_scratch
            .as_ref()
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())?
            .view_at(scratch_node_id, selection_offset, selection_limit)
            .map(Into::into)
    }

    pub fn combat_scratch_cursor_node_id(&self) -> Result<u64, String> {
        self.combat_scratch
            .as_ref()
            .map(|scratch| scratch.cursor_scratch_node_id)
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())
    }

    pub fn play_combat_scratch_action(
        &mut self,
        action_ref: &str,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        self.combat_scratch
            .as_mut()
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())?
            .play(action_ref, selection_offset, selection_limit)
    }

    pub fn play_combat_scratch_selector(
        &mut self,
        selector: OracleAnalysisCombatScratchActionSelectorV1,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        self.combat_scratch
            .as_mut()
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())?
            .play_selector(selector, selection_offset, selection_limit)
    }

    pub fn play_combat_scratch_selector_delta(
        &mut self,
        selector: OracleAnalysisCombatScratchActionSelectorV1,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchDecisionDeltaV1, String> {
        let source_node_id = selector.scratch_node_id();
        let base = self.combat_scratch_decision_view_at(
            source_node_id,
            selection_offset,
            selection_limit,
        )?;
        let result = self
            .play_combat_scratch_selector(selector, selection_offset, selection_limit)
            .map(OracleAnalysisCombatScratchDecisionViewV1::from)?;
        OracleAnalysisCombatScratchDecisionDeltaV1::between(&base, &result)
    }

    pub fn focus_combat_scratch_node(
        &mut self,
        scratch_node_id: u64,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        self.combat_scratch
            .as_mut()
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())?
            .focus(scratch_node_id, selection_offset, selection_limit)
    }

    pub fn back_combat_scratch(
        &mut self,
        selection_offset: usize,
        selection_limit: usize,
    ) -> Result<OracleAnalysisCombatScratchViewV1, String> {
        self.combat_scratch
            .as_mut()
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())?
            .back(selection_offset, selection_limit)
    }

    pub fn combat_scratch_tree(&self) -> Result<OracleAnalysisCombatScratchTreeV1, String> {
        self.combat_scratch
            .as_ref()
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())?
            .tree()
    }

    pub fn clear_combat_scratch(&mut self) -> bool {
        self.combat_scratch.take().is_some()
    }

    pub fn commit_combat_scratch(&mut self) -> Result<usize, String> {
        let scratch = self
            .combat_scratch
            .as_ref()
            .ok_or_else(|| "oracle analysis workspace has no active combat scratch".to_string())?;
        if EngineCombatStepper.terminal(scratch.current_position()?) != CombatTerminal::Win {
            return Err("combat scratch cursor is not a terminal victory".to_string());
        }
        let source_node_id = scratch.run_node_id;
        let actions = scratch.cursor_actions()?;
        let child_node_id = self.accept_combat_actions_from_node(source_node_id, &actions)?;
        self.combat_scratch = None;
        Ok(child_node_id)
    }
}

fn apply_exact(
    position: &CombatPosition,
    input: ClientInput,
    max_engine_steps_per_transition: usize,
) -> Result<CombatPosition, String> {
    if !EngineCombatStepper.is_legal_action(position, &input) {
        return Err(format!(
            "combat scratch input is not legal at exact state {}",
            exact_hash(position)
        ));
    }
    let result = EngineCombatStepper.apply_to_stable(
        position,
        input,
        CombatStepLimits {
            max_engine_steps: max_engine_steps_per_transition,
            deadline: None,
        },
    );
    if result.timed_out || result.truncated {
        return Err(format!(
            "combat scratch transition did not reach a stable boundary: timed_out={} truncated={} engine_steps={} limit={max_engine_steps_per_transition}",
            result.timed_out, result.truncated, result.engine_steps
        ));
    }
    Ok(result.position)
}
