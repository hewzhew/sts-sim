//! Shared root/selection ordinal driver for run batches and same-root combat groups.
//!
//! This module converts typed boundaries into model decisions, preserves bridge-local symbolic
//! prefixes, and prepares complete action batches. It owns no environment mutation or policy.

use sts_oracle_eval::eval::run_control::{
    CombatLearningBoundaryV1, CombatLearningEnvPoolV1, CombatLearningPotionPolicyV1,
    LearningActionV1, LearningBoundaryV1, LearningCombatAtomicActionV1,
    LearningCombatModelObservationV1, LearningCombatSelectionDomainSemanticsV1, LearningEnvPoolV1,
    LearningModelCandidateSemanticsV1, LearningModelChoiceV1, LearningModelDecisionV1,
    LearningModelObservationV1, LearningSelectionCandidateSemanticsV1, LearningSelectionStepV1,
};

use super::semantic::{SemanticBatch, SemanticBatchBuilder};
use super::{
    BridgeSlotState, DecisionSnapshot, PHASE_COMBAT_ROOT, PHASE_SELECTION, PHASE_STRATEGIC_ROOT,
};

pub(super) trait BridgeDecisionSource {
    fn bridge_slot_count(&self) -> usize;
    fn bridge_is_terminal(&self, slot_index: usize) -> Result<bool, String>;
    fn bridge_root_decision(
        &self,
        slot_index: usize,
    ) -> Result<LearningModelDecisionV1<'_>, String>;
}

pub(super) struct LearningBatchDecisionSource<'a> {
    pool: &'a LearningEnvPoolV1,
    potion_policy: &'a CombatLearningPotionPolicyV1,
}

impl<'a> LearningBatchDecisionSource<'a> {
    pub(super) fn new(
        pool: &'a LearningEnvPoolV1,
        potion_policy: &'a CombatLearningPotionPolicyV1,
    ) -> Self {
        Self {
            pool,
            potion_policy,
        }
    }
}

impl BridgeDecisionSource for LearningBatchDecisionSource<'_> {
    fn bridge_slot_count(&self) -> usize {
        self.pool.slot_count()
    }

    fn bridge_is_terminal(&self, slot_index: usize) -> Result<bool, String> {
        self.pool
            .boundary(slot_index)
            .map(LearningBoundaryV1::is_terminal)
            .ok_or_else(|| format!("missing pool slot {slot_index}"))
    }

    fn bridge_root_decision(
        &self,
        slot_index: usize,
    ) -> Result<LearningModelDecisionV1<'_>, String> {
        let boundary = self
            .pool
            .boundary(slot_index)
            .ok_or_else(|| format!("missing pool slot {slot_index}"))?;
        LearningModelDecisionV1::from_boundary_with_potion_policy(
            boundary,
            self.potion_policy,
        )
        .map_err(|error| error.to_string())
    }
}

impl BridgeDecisionSource for LearningEnvPoolV1 {
    fn bridge_slot_count(&self) -> usize {
        self.slot_count()
    }

    fn bridge_is_terminal(&self, slot_index: usize) -> Result<bool, String> {
        self.boundary(slot_index)
            .map(LearningBoundaryV1::is_terminal)
            .ok_or_else(|| format!("missing pool slot {slot_index}"))
    }

    fn bridge_root_decision(
        &self,
        slot_index: usize,
    ) -> Result<LearningModelDecisionV1<'_>, String> {
        let boundary = self
            .boundary(slot_index)
            .ok_or_else(|| format!("missing pool slot {slot_index}"))?;
        LearningModelDecisionV1::from_boundary(boundary).map_err(|error| error.to_string())
    }
}

impl BridgeDecisionSource for CombatLearningEnvPoolV1 {
    fn bridge_slot_count(&self) -> usize {
        self.replicate_count()
    }

    fn bridge_is_terminal(&self, slot_index: usize) -> Result<bool, String> {
        let replicate_index = u32::try_from(slot_index)
            .map_err(|_| format!("combat replicate index {slot_index} exceeds u32"))?;
        self.boundary(replicate_index)
            .map(CombatLearningBoundaryV1::is_terminal)
            .ok_or_else(|| format!("missing combat replicate {replicate_index}"))
    }

    fn bridge_root_decision(
        &self,
        slot_index: usize,
    ) -> Result<LearningModelDecisionV1<'_>, String> {
        let replicate_index = u32::try_from(slot_index)
            .map_err(|_| format!("combat replicate index {slot_index} exceeds u32"))?;
        let boundary = self
            .boundary(replicate_index)
            .ok_or_else(|| format!("missing combat replicate {replicate_index}"))?;
        let CombatLearningBoundaryV1::Decision { boundary, .. } = boundary else {
            return Err(format!("combat replicate {replicate_index} is terminal"));
        };
        LearningModelDecisionV1::from_combat_boundary_with_potion_policy(
            boundary,
            self.potion_policy(),
        )
        .map_err(|error| error.to_string())
    }
}

pub(super) fn decision_snapshot_from_source(
    source: &impl BridgeDecisionSource,
    states: &[BridgeSlotState],
) -> Result<DecisionSnapshot, String> {
    if states.len() != source.bridge_slot_count() {
        return Err("bridge state count does not match its decision source".to_owned());
    }
    let mut slot_indices = Vec::new();
    let mut phases = Vec::new();
    let mut candidate_counts = Vec::new();
    let mut candidate_row_splits = vec![0];

    for (slot_index, state) in states.iter().enumerate() {
        let (phase, candidate_count) = match state {
            BridgeSlotState::Root => {
                let decision = source.bridge_root_decision(slot_index)?;
                let phase = match decision.observation {
                    LearningModelObservationV1::Strategic(_) => PHASE_STRATEGIC_ROOT,
                    LearningModelObservationV1::Combat(_) => PHASE_COMBAT_ROOT,
                };
                (phase, decision.candidates.len())
            }
            BridgeSlotState::Selection { draft, .. } => {
                (PHASE_SELECTION, draft.decision().candidates.len())
            }
            BridgeSlotState::Terminal | BridgeSlotState::Ready { .. } => continue,
        };
        if candidate_count == 0 {
            return Err(format!("slot {slot_index} exposed an empty decision row"));
        }
        let next = candidate_row_splits
            .last()
            .copied()
            .unwrap_or(0usize)
            .checked_add(candidate_count)
            .ok_or_else(|| "candidate count overflow".to_owned())?;
        slot_indices.push(slot_index);
        phases.push(phase);
        candidate_counts.push(candidate_count);
        candidate_row_splits.push(next);
    }

    Ok(DecisionSnapshot {
        slot_indices,
        phases,
        candidate_counts,
        candidate_row_splits,
    })
}

pub(super) fn semantic_snapshot_from_source(
    source: &impl BridgeDecisionSource,
    states: &[BridgeSlotState],
) -> Result<SemanticBatch, String> {
    if states.len() != source.bridge_slot_count() {
        return Err("bridge state count does not match its decision source".to_owned());
    }
    let mut builder = SemanticBatchBuilder::new();
    for (slot_index, state) in states.iter().enumerate() {
        match state {
            BridgeSlotState::Root => {
                let decision = source.bridge_root_decision(slot_index)?;
                builder
                    .push_decision(&decision)
                    .map_err(|error| error.to_string())?;
            }
            BridgeSlotState::Selection { draft, .. } => {
                let decision = source.bridge_root_decision(slot_index)?;
                builder
                    .push_selection(decision.observation, draft)
                    .map_err(|error| error.to_string())?;
            }
            BridgeSlotState::Terminal | BridgeSlotState::Ready { .. } => {}
        }
    }
    Ok(builder.finish())
}

pub(super) fn strategic_decision_audit_json_from_source(
    source: &impl BridgeDecisionSource,
    states: &[BridgeSlotState],
    slot_index: usize,
) -> Result<Option<String>, String> {
    let state = states
        .get(slot_index)
        .ok_or_else(|| format!("missing bridge slot {slot_index}"))?;
    if !matches!(state, BridgeSlotState::Root) {
        return Ok(None);
    }
    let decision = source.bridge_root_decision(slot_index)?;
    let LearningModelObservationV1::Strategic(observation) = decision.observation else {
        return Ok(None);
    };
    let candidates = decision
        .candidates
        .iter()
        .map(|candidate| match candidate.semantics {
            LearningModelCandidateSemanticsV1::Strategic { action } => Ok(action),
            LearningModelCandidateSemanticsV1::CombatAtomic { .. }
            | LearningModelCandidateSemanticsV1::CombatSelectionFamily { .. } => Err(format!(
                "strategic bridge slot {slot_index} exposed a non-strategic candidate"
            )),
        })
        .collect::<Result<Vec<_>, String>>()?;
    serde_json::to_string(&serde_json::json!({
        "schema": "sts-learning-strategic-decision-audit-v1",
        "decision_site": observation.decision_site,
        "candidates": candidates,
    }))
    .map(Some)
    .map_err(|error| format!("cannot encode strategic decision audit: {error}"))
}

pub(super) fn combat_decision_audit_json_from_source(
    source: &impl BridgeDecisionSource,
    states: &[BridgeSlotState],
    slot_index: usize,
) -> Result<Option<String>, String> {
    let state = states
        .get(slot_index)
        .ok_or_else(|| format!("missing bridge slot {slot_index}"))?;
    let (phase, selection_prefix, candidates) = match state {
        BridgeSlotState::Root => {
            let decision = source.bridge_root_decision(slot_index)?;
            let LearningModelObservationV1::Combat(observation) = decision.observation else {
                return Ok(None);
            };
            let candidates = decision
                .candidates
                .iter()
                .map(|candidate| match candidate.semantics {
                    LearningModelCandidateSemanticsV1::CombatAtomic { action } => {
                        combat_atomic_candidate_audit(action, observation)
                    }
                    LearningModelCandidateSemanticsV1::CombatSelectionFamily { family } => {
                        let domains = (0..family.domain_count())
                            .map(|index| {
                                family
                                    .domain(index)
                                    .map(|domain| combat_selection_domain_audit(domain.semantics()))
                                    .ok_or_else(|| {
                                        format!("combat selection family lost domain index {index}")
                                    })
                            })
                            .collect::<Result<Vec<_>, String>>()?;
                        Ok(serde_json::json!({
                            "kind": "selection_family",
                            "input_encoding": family.input_encoding(),
                            "reason": family.reason(),
                            "source_pile": family.source_pile(),
                            "raw_domain_count": family.raw_domain_count(),
                            "eligible_domain_count": family.eligible_domain_count(),
                            "max_distinct_selection_count": family.max_distinct_selection_count(),
                            "declared_min": family.declared_min(),
                            "declared_max": family.declared_max(),
                            "effective_max": family.effective_max(),
                            "payload_language": family.payload_language(),
                            "domain": domains,
                        }))
                    }
                    LearningModelCandidateSemanticsV1::Strategic { .. } => Err(format!(
                        "combat bridge slot {slot_index} exposed a strategic candidate"
                    )),
                })
                .collect::<Result<Vec<_>, String>>()?;
            ("combat_root", Vec::new(), candidates)
        }
        BridgeSlotState::Selection { draft, .. } => {
            let Some(family) = draft.combat_family() else {
                return Ok(None);
            };
            let decision = draft.decision();
            let candidates = decision
                .candidates
                .iter()
                .map(|candidate| match candidate.semantics {
                    LearningSelectionCandidateSemanticsV1::Submit => {
                        Ok(serde_json::json!({"kind": "selection_submit"}))
                    }
                    LearningSelectionCandidateSemanticsV1::Append { domain_index } => {
                        let domain = family.domain(domain_index).ok_or_else(|| {
                            format!(
                                "combat selection candidate references missing domain {domain_index}"
                            )
                        })?;
                        Ok(serde_json::json!({
                            "kind": "selection_append",
                            "domain_index": domain_index,
                            "domain": combat_selection_domain_audit(domain.semantics()),
                        }))
                    }
                })
                .collect::<Result<Vec<_>, String>>()?;
            (
                "combat_selection",
                draft.selected_domain_indices().to_vec(),
                candidates,
            )
        }
        BridgeSlotState::Terminal | BridgeSlotState::Ready { .. } => return Ok(None),
    };
    serde_json::to_string(&serde_json::json!({
        "schema": "sts-learning-combat-decision-audit-v1",
        "phase": phase,
        "selection_prefix": selection_prefix,
        "candidates": candidates,
    }))
    .map(Some)
    .map_err(|error| format!("cannot encode combat decision audit: {error}"))
}

fn combat_atomic_candidate_audit(
    action: LearningCombatAtomicActionV1<'_>,
    observation: LearningCombatModelObservationV1<'_>,
) -> Result<serde_json::Value, String> {
    Ok(match action {
        LearningCombatAtomicActionV1::PlayCard {
            hand_index,
            target_monster_index,
        } => {
            let card = observation
                .cards
                .hand
                .cards
                .get(hand_index)
                .ok_or_else(|| format!("combat play candidate lost hand index {hand_index}"))?;
            serde_json::json!({
                "kind": "play_card",
                "hand_index": hand_index,
                "card": card,
                "target": combat_target_audit(observation, target_monster_index)?,
            })
        }
        LearningCombatAtomicActionV1::UsePotion {
            potion_index,
            target_monster_index,
        } => {
            let potion = observation
                .potions
                .get(potion_index)
                .ok_or_else(|| format!("combat potion candidate lost slot {potion_index}"))?;
            serde_json::json!({
                "kind": "use_potion",
                "potion_index": potion_index,
                "potion": potion,
                "target": combat_target_audit(observation, target_monster_index)?,
            })
        }
        LearningCombatAtomicActionV1::DiscardPotion { potion_index } => {
            let potion = observation
                .potions
                .get(potion_index)
                .ok_or_else(|| format!("combat potion candidate lost slot {potion_index}"))?;
            serde_json::json!({
                "kind": "discard_potion",
                "potion_index": potion_index,
                "potion": potion,
            })
        }
        LearningCombatAtomicActionV1::EndTurn => serde_json::json!({"kind": "end_turn"}),
        LearningCombatAtomicActionV1::SubmitIndexedChoice {
            choice_index,
            indexed,
        } => serde_json::json!({
            "kind": "submit_indexed_choice",
            "choice_index": choice_index,
            "input_encoding": indexed.input_encoding,
            "reason": indexed.reason,
            "candidate": indexed.candidate,
        }),
        LearningCombatAtomicActionV1::Proceed => serde_json::json!({"kind": "proceed"}),
        LearningCombatAtomicActionV1::Cancel => serde_json::json!({"kind": "cancel"}),
    })
}

fn combat_target_audit(
    observation: LearningCombatModelObservationV1<'_>,
    target_monster_index: Option<usize>,
) -> Result<Option<serde_json::Value>, String> {
    target_monster_index
        .map(|monster_index| {
            let monster = observation.monsters.get(monster_index).ok_or_else(|| {
                format!("combat candidate lost target monster index {monster_index}")
            })?;
            Ok(serde_json::json!({
                "monster_index": monster_index,
                "slot": monster.slot(),
                "enemy": monster.enemy(),
            }))
        })
        .transpose()
}

fn combat_selection_domain_audit(
    semantics: LearningCombatSelectionDomainSemanticsV1,
) -> serde_json::Value {
    match semantics {
        LearningCombatSelectionDomainSemanticsV1::Card {
            ordinal,
            card_id,
            upgrades,
            eligible,
        } => serde_json::json!({
            "kind": "card",
            "ordinal": ordinal,
            "card_id": card_id,
            "upgrades": upgrades,
            "eligible": eligible,
        }),
        LearningCombatSelectionDomainSemanticsV1::Scry {
            index,
            card_id,
            currently_present,
        } => serde_json::json!({
            "kind": "scry",
            "index": index,
            "card_id": card_id,
            "currently_present": currently_present,
        }),
    }
}

pub(super) fn bridge_states_ready(states: &[BridgeSlotState]) -> bool {
    states.iter().all(|state| {
        matches!(
            state,
            BridgeSlotState::Terminal | BridgeSlotState::Ready { .. }
        )
    })
}

pub(super) fn choose_bridge_ordinals(
    source: &impl BridgeDecisionSource,
    states: &mut Vec<BridgeSlotState>,
    ordinals: Vec<usize>,
) -> Result<(), String> {
    let snapshot = decision_snapshot_from_source(source, states)?;
    if ordinals.len() != snapshot.slot_indices.len() {
        return Err(format!(
            "expected {} candidate ordinals, received {}",
            snapshot.slot_indices.len(),
            ordinals.len()
        ));
    }

    let mut next_states = states.clone();
    for ((slot_index, candidate_count), ordinal) in snapshot
        .slot_indices
        .into_iter()
        .zip(snapshot.candidate_counts)
        .zip(ordinals)
    {
        if ordinal >= candidate_count {
            return Err(format!(
                "slot {slot_index} candidate ordinal {ordinal} is outside 0..{candidate_count}"
            ));
        }
        apply_bridge_ordinal(source, slot_index, &mut next_states[slot_index], ordinal)?;
    }
    *states = next_states;
    Ok(())
}

pub(super) fn collect_ready_actions(
    states: &[BridgeSlotState],
) -> Result<Vec<LearningActionV1>, String> {
    let mut actions = Vec::new();
    for state in states {
        match state {
            BridgeSlotState::Ready { action, .. } => actions.push(action.clone()),
            BridgeSlotState::Terminal => {}
            BridgeSlotState::Root | BridgeSlotState::Selection { .. } => {
                return Err("driver readiness changed while collecting actions".to_owned());
            }
        }
    }
    Ok(actions)
}

fn apply_bridge_ordinal(
    source: &impl BridgeDecisionSource,
    slot_index: usize,
    state: &mut BridgeSlotState,
    ordinal: usize,
) -> Result<(), String> {
    let next = match state.clone() {
        BridgeSlotState::Root => {
            let decision = source.bridge_root_decision(slot_index)?;
            match decision
                .choose(ordinal)
                .map_err(|error| error.to_string())?
            {
                LearningModelChoiceV1::Apply(action) => BridgeSlotState::Ready {
                    action,
                    decision_ordinals: vec![ordinal],
                },
                LearningModelChoiceV1::DecodeSelection(draft) => BridgeSlotState::Selection {
                    draft,
                    decision_ordinals: vec![ordinal],
                },
            }
        }
        BridgeSlotState::Selection {
            mut draft,
            mut decision_ordinals,
        } => {
            decision_ordinals.push(ordinal);
            match draft.choose(ordinal).map_err(|error| error.to_string())? {
                LearningSelectionStepV1::Continue => BridgeSlotState::Selection {
                    draft,
                    decision_ordinals,
                },
                LearningSelectionStepV1::Apply(action) => BridgeSlotState::Ready {
                    action,
                    decision_ordinals,
                },
            }
        }
        BridgeSlotState::Terminal | BridgeSlotState::Ready { .. } => {
            return Err(format!(
                "slot {slot_index} decision prefix continues after readiness"
            ));
        }
    };
    *state = next;
    Ok(())
}

pub(super) fn replay_bridge_state(
    source: &impl BridgeDecisionSource,
    slot_index: usize,
    decision_ordinals: &[usize],
) -> Result<BridgeSlotState, String> {
    let mut state = if source.bridge_is_terminal(slot_index)? {
        BridgeSlotState::Terminal
    } else {
        BridgeSlotState::Root
    };
    for ordinal in decision_ordinals {
        apply_bridge_ordinal(source, slot_index, &mut state, *ordinal)?;
    }
    Ok(state)
}

pub(super) fn states_from_source(
    source: &impl BridgeDecisionSource,
) -> Result<Vec<BridgeSlotState>, String> {
    (0..source.bridge_slot_count())
        .map(|slot_index| {
            Ok(if source.bridge_is_terminal(slot_index)? {
                BridgeSlotState::Terminal
            } else {
                BridgeSlotState::Root
            })
        })
        .collect()
}
