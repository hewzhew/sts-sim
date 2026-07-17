use std::collections::{BTreeMap, HashMap, HashSet};

use super::super::frontier::{RootLineage, RootLineageId, SearchNode};
use super::super::*;
use super::loop_state::SearchLoopState;
use super::node_action_ordering::OrderedNodeAction;

const ROOT_RANKING_POLICY: &str =
    "best_exact_complete_or_live_open_observation_by_combat_outcome_score_v1";
const ROOT_WORK_ACCOUNTING_SCOPE: &str =
    "concrete_search_nodes_by_typed_root_lineage; hierarchical_turn_boundary_macro_inner_work_remains_unattributed";

pub(super) struct RootEvidenceBook {
    next_id: u32,
    by_action_key: HashMap<String, RootLineageId>,
    entries: BTreeMap<RootLineageId, RootEvidenceEntry>,
    unattributed: CombatSearchV2RootWorkEvidence,
    materialization: CombatSearchV2RootMaterializationStatus,
    unmaterialized_pending_work_items: Option<usize>,
}

struct RootEvidenceEntry {
    identity: CombatSearchV2RootActionIdentity,
    work: CombatSearchV2RootWorkEvidence,
}

impl Default for RootEvidenceBook {
    fn default() -> Self {
        Self {
            next_id: 0,
            by_action_key: HashMap::new(),
            entries: BTreeMap::new(),
            unattributed: CombatSearchV2RootWorkEvidence::default(),
            materialization: CombatSearchV2RootMaterializationStatus::NotStarted,
            unmaterialized_pending_work_items: None,
        }
    }
}

impl RootEvidenceBook {
    pub(super) fn materialize_node(&mut self, node: &mut SearchNode) {
        if !matches!(node.root_lineage, RootLineage::Unmaterialized) {
            return;
        }
        let Some(action) = node.actions.first() else {
            return;
        };
        let id = self.register_identity(identity_from_trace(action));
        node.root_lineage = RootLineage::Action(id);
    }

    pub(super) fn observe_enumerated_root_surface(
        &mut self,
        node: &SearchNode,
        choices: &[OrderedNodeAction],
    ) {
        if !matches!(node.root_lineage, RootLineage::Unmaterialized) || !node.actions.is_empty() {
            return;
        }
        for choice in choices {
            self.register_authoritative_identity(CombatSearchV2RootActionIdentity {
                action_id: choice.choice.original_action_id,
                action_key: choice.choice.choice.action_key.clone(),
                action_debug: choice.choice.choice.action_debug.clone(),
            });
        }
        self.materialization = CombatSearchV2RootMaterializationStatus::Complete;
    }

    pub(super) fn mark_unmaterialized_surface_complete(&mut self, node: &SearchNode) {
        if matches!(node.root_lineage, RootLineage::Unmaterialized) && node.actions.is_empty() {
            self.materialization = CombatSearchV2RootMaterializationStatus::Complete;
        }
    }

    pub(super) fn begin_unmaterialized_pending_work(
        &mut self,
        node: &SearchNode,
        work_items: usize,
    ) {
        if !matches!(node.root_lineage, RootLineage::Unmaterialized) || !node.actions.is_empty() {
            return;
        }
        self.unmaterialized_pending_work_items = Some(work_items);
        self.materialization = if work_items == 0 {
            CombatSearchV2RootMaterializationStatus::Complete
        } else {
            CombatSearchV2RootMaterializationStatus::Partial
        };
    }

    pub(super) fn finish_unmaterialized_pending_work(&mut self, node: &SearchNode) {
        if !matches!(node.root_lineage, RootLineage::Unmaterialized) || !node.actions.is_empty() {
            return;
        }
        let Some(remaining) = self.unmaterialized_pending_work_items.as_mut() else {
            return;
        };
        *remaining = remaining.saturating_sub(1);
        if *remaining == 0 {
            self.materialization = CombatSearchV2RootMaterializationStatus::Complete;
        }
    }

    pub(super) fn record_generated(&mut self, node: &SearchNode) {
        let work = self.work_for_node_mut(node);
        work.generated_concrete_nodes = work.generated_concrete_nodes.saturating_add(1);
    }

    pub(super) fn record_expanded(&mut self, node: &SearchNode) {
        let work = self.work_for_node_mut(node);
        work.expanded_concrete_nodes = work.expanded_concrete_nodes.saturating_add(1);
    }

    pub(super) fn record_bulk_work(
        &mut self,
        source: &SearchNode,
        nodes_expanded: usize,
        nodes_generated: usize,
    ) {
        let work = self.work_for_node_mut(source);
        work.expanded_concrete_nodes = work
            .expanded_concrete_nodes
            .saturating_add(nodes_expanded as u64);
        work.generated_concrete_nodes = work
            .generated_concrete_nodes
            .saturating_add(nodes_generated as u64);
    }

    pub(super) fn observe_exact_complete(&mut self, node: &SearchNode) {
        let value = observed_value(node);
        remember_best(&mut self.work_for_node_mut(node).best_exact_complete, value);
    }

    pub(super) fn observe_exact_win(&mut self, node: &SearchNode) {
        let value = observed_value(node);
        let work = self.work_for_node_mut(node);
        remember_best(&mut work.best_exact_complete, value.clone());
        remember_best(&mut work.best_exact_win, value);
    }

    fn register_identity(&mut self, identity: CombatSearchV2RootActionIdentity) -> RootLineageId {
        if let Some(id) = self.by_action_key.get(&identity.action_key).copied() {
            return id;
        }
        let id = RootLineageId(self.next_id);
        self.next_id = self.next_id.saturating_add(1);
        self.by_action_key.insert(identity.action_key.clone(), id);
        self.entries.insert(
            id,
            RootEvidenceEntry {
                identity,
                work: CombatSearchV2RootWorkEvidence::default(),
            },
        );
        if self.materialization == CombatSearchV2RootMaterializationStatus::NotStarted {
            self.materialization = CombatSearchV2RootMaterializationStatus::Partial;
        }
        id
    }

    fn register_authoritative_identity(
        &mut self,
        identity: CombatSearchV2RootActionIdentity,
    ) -> RootLineageId {
        let id = self.register_identity(identity.clone());
        if let Some(entry) = self.entries.get_mut(&id) {
            entry.identity = identity;
        }
        id
    }

    fn work_for_node_mut(&mut self, node: &SearchNode) -> &mut CombatSearchV2RootWorkEvidence {
        match node.root_lineage {
            RootLineage::Action(id) => self
                .entries
                .get_mut(&id)
                .map(|entry| &mut entry.work)
                .unwrap_or(&mut self.unattributed),
            RootLineage::Unmaterialized => &mut self.unattributed,
        }
    }
}

impl SearchLoopState {
    pub(in crate::ai::combat_search_v2::search) fn materialize_root_lineage(
        &mut self,
        node: &mut SearchNode,
    ) {
        self.root_evidence.materialize_node(node);
    }

    pub(in crate::ai::combat_search_v2::search) fn observe_enumerated_root_surface(
        &mut self,
        node: &SearchNode,
        choices: &[OrderedNodeAction],
    ) {
        self.root_evidence
            .observe_enumerated_root_surface(node, choices);
    }

    pub(in crate::ai::combat_search_v2::search) fn mark_unmaterialized_root_surface_complete(
        &mut self,
        node: &SearchNode,
    ) {
        self.root_evidence
            .mark_unmaterialized_surface_complete(node);
    }

    pub(in crate::ai::combat_search_v2::search) fn begin_unmaterialized_root_pending_work(
        &mut self,
        node: &SearchNode,
        work_items: usize,
    ) {
        self.root_evidence
            .begin_unmaterialized_pending_work(node, work_items);
    }

    pub(in crate::ai::combat_search_v2::search) fn finish_unmaterialized_root_pending_work(
        &mut self,
        node: &SearchNode,
    ) {
        self.root_evidence.finish_unmaterialized_pending_work(node);
    }

    pub(in crate::ai::combat_search_v2::search) fn observe_exact_root_terminal(
        &mut self,
        node: &SearchNode,
    ) {
        match terminal_label(&node.engine, &node.combat) {
            SearchTerminalLabel::Win => self.root_evidence.observe_exact_win(node),
            SearchTerminalLabel::Loss => self.root_evidence.observe_exact_complete(node),
            SearchTerminalLabel::Unresolved => {}
        }
    }
}

pub(super) fn root_evidence_snapshot(
    loop_state: &SearchLoopState,
) -> CombatSearchV2RootEvidenceSnapshot {
    let mut contenders = loop_state
        .root_evidence
        .entries
        .iter()
        .map(|(id, entry)| {
            (
                *id,
                CombatSearchV2RootActionEvidence {
                    rank: None,
                    root_action: entry.identity.clone(),
                    work: entry.work.clone(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut unattributed = loop_state.root_evidence.unattributed.clone();
    let mut open_states = HashMap::<RootLineageId, HashSet<CombatExactStateKey>>::new();
    let mut unattributed_open_states = HashSet::new();

    for entry in loop_state.frontier.iter() {
        let value = observed_value(&entry.node);
        let key = combat_exact_state_key(&entry.node.engine, &entry.node.combat);
        match entry.node.root_lineage {
            RootLineage::Action(id) if contenders.contains_key(&id) => {
                let contender = contenders.get_mut(&id).expect("checked root lineage");
                remember_best(&mut contender.work.best_open_observed, value);
                open_states.entry(id).or_default().insert(key);
                if entry.pending_choice_work.is_some() {
                    contender.work.open_pending_choice_work_items = contender
                        .work
                        .open_pending_choice_work_items
                        .saturating_add(1);
                }
            }
            RootLineage::Action(_) | RootLineage::Unmaterialized => {
                remember_best(&mut unattributed.best_open_observed, value);
                unattributed_open_states.insert(key);
                if entry.pending_choice_work.is_some() {
                    unattributed.open_pending_choice_work_items = unattributed
                        .open_pending_choice_work_items
                        .saturating_add(1);
                }
            }
        }
    }
    for (id, states) in open_states {
        if let Some(contender) = contenders.get_mut(&id) {
            contender.work.open_concrete_states = states.len();
        }
    }
    unattributed.open_concrete_states = unattributed_open_states.len();

    let mut contenders = contenders.into_values().collect::<Vec<_>>();
    contenders.sort_by(|left, right| {
        leading_value(&right.work)
            .map(|value| &value.outcome_order_key)
            .cmp(&leading_value(&left.work).map(|value| &value.outcome_order_key))
            .then_with(|| {
                left.root_action
                    .action_key
                    .cmp(&right.root_action.action_key)
            })
    });
    let mut next_rank = 1usize;
    for contender in &mut contenders {
        if leading_value(&contender.work).is_some() {
            contender.rank = Some(next_rank);
            next_rank = next_rank.saturating_add(1);
        }
    }
    let leader = contenders
        .iter()
        .find(|contender| contender.rank == Some(1))
        .map(|contender| contender.root_action.clone());

    let mut closure_blockers = Vec::new();
    if loop_state.root_evidence.materialization != CombatSearchV2RootMaterializationStatus::Complete
    {
        closure_blockers
            .push(CombatSearchV2RootClosureBlocker::RootActionSurfaceNotFullyMaterialized);
    }
    if unattributed.open_concrete_states > 0
        || contenders
            .iter()
            .any(|contender| contender.work.open_concrete_states > 0)
    {
        closure_blockers.push(CombatSearchV2RootClosureBlocker::OpenConcreteWork);
    }
    if unattributed.open_pending_choice_work_items > 0
        || contenders
            .iter()
            .any(|contender| contender.work.open_pending_choice_work_items > 0)
    {
        closure_blockers.push(CombatSearchV2RootClosureBlocker::OpenPendingChoiceWork);
    }
    if loop_state.unresolved_leaf_count > 0 {
        closure_blockers.push(CombatSearchV2RootClosureBlocker::UnresolvedLeaf);
    }
    if loop_state.stats.action_surface_incomplete {
        closure_blockers
            .push(CombatSearchV2RootClosureBlocker::PendingChoiceOrderedVariantsOmitted);
    }
    if loop_state.max_actions_cut_count > 0 {
        closure_blockers.push(CombatSearchV2RootClosureBlocker::MaxActionsPerLine);
    }
    if loop_state.engine_step_limit_count > 0 {
        closure_blockers.push(CombatSearchV2RootClosureBlocker::EngineStepLimit);
    }
    if loop_state.potion_budget_cut_count > 0 {
        closure_blockers.push(CombatSearchV2RootClosureBlocker::PotionBudget);
    }
    if loop_state.performance.turn_boundary_macro_calls > 0 {
        closure_blockers
            .push(CombatSearchV2RootClosureBlocker::HierarchicalTurnBoundaryPortfolioSelection);
    }
    // Closure cannot become authoritative until exact-equivalence, dominance,
    // and terminal dispositions are attributed to their root lineage.  Keep
    // this evidence shadow-only instead of turning an observed empty queue
    // into a false proof.
    closure_blockers.push(CombatSearchV2RootClosureBlocker::ProofDispositionsUnavailable);

    CombatSearchV2RootEvidenceSnapshot {
        ranking_policy: ROOT_RANKING_POLICY.to_string(),
        work_accounting_scope: ROOT_WORK_ACCOUNTING_SCOPE.to_string(),
        materialization: loop_state.root_evidence.materialization,
        closure_status: CombatSearchV2RootClosureStatus::NotProven,
        closure_blockers,
        leader,
        contenders,
        unattributed,
    }
}

fn identity_from_trace(action: &CombatSearchV2ActionTrace) -> CombatSearchV2RootActionIdentity {
    CombatSearchV2RootActionIdentity {
        action_id: action.action_id,
        action_key: action.action_key.clone(),
        action_debug: action.action_debug.clone(),
    }
}

fn observed_value(node: &SearchNode) -> CombatSearchV2RootObservedValue {
    CombatSearchV2RootObservedValue {
        terminal: terminal_label(&node.engine, &node.combat),
        outcome_order_key: CombatOutcomeScore::from_node(node).to_report_key(),
        final_hp: node.combat.entities.player.current_hp,
        hp_loss: (node.initial_hp - node.combat.entities.player.current_hp).max(0),
        turns: node.combat.turn.turn_count,
        potions_used: node.potions_used,
        potions_discarded: node.potions_discarded,
        cards_played: node.cards_played,
        actions_taken: node.actions.len(),
    }
}

fn remember_best(
    best: &mut Option<CombatSearchV2RootObservedValue>,
    candidate: CombatSearchV2RootObservedValue,
) {
    if best
        .as_ref()
        .is_none_or(|current| candidate.outcome_order_key > current.outcome_order_key)
    {
        *best = Some(candidate);
    }
}

fn leading_value(
    work: &CombatSearchV2RootWorkEvidence,
) -> Option<&CombatSearchV2RootObservedValue> {
    [
        work.best_exact_complete.as_ref(),
        work.best_open_observed.as_ref(),
    ]
    .into_iter()
    .flatten()
    .max_by_key(|value| &value.outcome_order_key)
}
