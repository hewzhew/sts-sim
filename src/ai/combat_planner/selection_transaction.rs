use std::collections::HashSet;

use crate::sim::combat_action_surface::{
    CombatSelectionActionFamilyV2, CombatSelectionDomainCandidateV2,
    CombatSelectionInputEncodingV2, CombatSelectionStatusV2,
};
use crate::state::core::ClientInput;
use crate::state::selection::{SelectionResolution, SelectionScope};

use super::types::TurnOptionGenerationGapKind;

#[derive(Clone, Debug)]
pub(super) struct SelectionTransactionCursor {
    encoding: CombatSelectionInputEncodingV2,
    candidates: Vec<SelectionCandidate>,
    min_len: usize,
    max_len: usize,
    next_indices: Option<Vec<usize>>,
}

#[derive(Clone, Copy, Debug)]
struct SelectionCandidate {
    payload: usize,
    distinct_key: u32,
}

impl SelectionTransactionCursor {
    pub(super) fn new(
        family: &CombatSelectionActionFamilyV2,
    ) -> Result<Self, TurnOptionGenerationGapKind> {
        if !matches!(family.selection_status, CombatSelectionStatusV2::Enabled) {
            return Err(TurnOptionGenerationGapKind::DisabledStructuredChoice);
        }

        let candidates = family
            .raw_domain
            .iter()
            .filter_map(|candidate| match candidate {
                CombatSelectionDomainCandidateV2::CardUuid { uuid, eligible, .. } if *eligible => {
                    usize::try_from(*uuid)
                        .ok()
                        .map(|payload| SelectionCandidate {
                            payload,
                            distinct_key: *uuid,
                        })
                }
                CombatSelectionDomainCandidateV2::ScryIndex {
                    index,
                    card_uuid: Some(card_uuid),
                    currently_present: true,
                    ..
                } => usize::try_from(*index)
                    .ok()
                    .map(|payload| SelectionCandidate {
                        payload,
                        distinct_key: *card_uuid,
                    }),
                _ => None,
            })
            .collect::<Vec<_>>();
        let distinct_count = candidates
            .iter()
            .map(|candidate| candidate.distinct_key)
            .collect::<HashSet<_>>()
            .len();
        let min_len = usize::try_from(family.declared_min).unwrap_or(usize::MAX);
        let max_len = usize::try_from(family.effective_max)
            .unwrap_or(usize::MAX)
            .min(distinct_count);
        let next_indices = first_valid_indices(&candidates, min_len);

        Ok(Self {
            encoding: family.input_encoding,
            candidates,
            min_len,
            max_len,
            next_indices,
        })
    }

    pub(super) fn next_input(&mut self) -> Option<ClientInput> {
        let indices = self.next_indices.clone()?;
        let input = self.compile(&indices);
        self.next_indices =
            next_valid_indices(&self.candidates, &indices, self.min_len, self.max_len);
        Some(input)
    }

    pub(super) fn is_exhausted(&self) -> bool {
        self.next_indices.is_none()
    }

    fn compile(&self, indices: &[usize]) -> ClientInput {
        let payloads = indices
            .iter()
            .map(|index| self.candidates[*index].payload)
            .collect::<Vec<_>>();
        match self.encoding {
            CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids => {
                ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                    SelectionScope::Hand,
                    payloads.into_iter().map(|value| value as u32),
                ))
            }
            CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids => {
                ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                    SelectionScope::Grid,
                    payloads.into_iter().map(|value| value as u32),
                ))
            }
            CombatSelectionInputEncodingV2::SubmitScryDiscardIndices => {
                ClientInput::SubmitScryDiscard(payloads)
            }
        }
    }
}

fn first_valid_indices(candidates: &[SelectionCandidate], len: usize) -> Option<Vec<usize>> {
    let mut indices = Vec::with_capacity(len);
    fill_smallest_suffix(candidates, &mut indices, len).then_some(indices)
}

fn next_valid_indices(
    candidates: &[SelectionCandidate],
    current: &[usize],
    min_len: usize,
    max_len: usize,
) -> Option<Vec<usize>> {
    if !current.is_empty() {
        for pivot in (0..current.len()).rev() {
            let mut prefix = current[..pivot].to_vec();
            for candidate_index in current[pivot].saturating_add(1)..candidates.len() {
                if distinct_key_is_available(candidates, &prefix, candidate_index) {
                    prefix.push(candidate_index);
                    if fill_smallest_suffix(candidates, &mut prefix, current.len()) {
                        return Some(prefix);
                    }
                    prefix.pop();
                }
            }
        }
    }

    let next_len = current.len().max(min_len).saturating_add(1);
    (next_len <= max_len)
        .then(|| first_valid_indices(candidates, next_len))
        .flatten()
}

fn fill_smallest_suffix(
    candidates: &[SelectionCandidate],
    indices: &mut Vec<usize>,
    target_len: usize,
) -> bool {
    while indices.len() < target_len {
        let Some(next) = (0..candidates.len())
            .find(|candidate| distinct_key_is_available(candidates, indices, *candidate))
        else {
            return false;
        };
        indices.push(next);
    }
    true
}

fn distinct_key_is_available(
    candidates: &[SelectionCandidate],
    selected_indices: &[usize],
    candidate_index: usize,
) -> bool {
    selected_indices.iter().all(|selected| {
        candidates[*selected].distinct_key != candidates[candidate_index].distinct_key
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sim::combat_action_surface::{
        CombatSelectionDistinctByV2, CombatSelectionPayloadLanguageV2, CombatSelectionReasonV2,
    };
    use crate::state::core::HandSelectReason;

    #[test]
    fn ordered_cursor_emits_permutations_without_eager_materialization() {
        let family = CombatSelectionActionFamilyV2 {
            input_encoding: CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids,
            reason: CombatSelectionReasonV2::Hand(HandSelectReason::Discard),
            source_pile: None,
            raw_domain: [11, 22, 33]
                .into_iter()
                .enumerate()
                .map(
                    |(ordinal, uuid)| CombatSelectionDomainCandidateV2::CardUuid {
                        ordinal: ordinal as u64,
                        uuid,
                        card_id: None,
                        upgrades: None,
                        eligible: true,
                    },
                )
                .collect(),
            raw_domain_count: 3,
            eligible_domain_count: 3,
            max_distinct_selection_count: 3,
            declared_min: 2,
            declared_max: 2,
            effective_max: 2,
            selection_status: CombatSelectionStatusV2::Enabled,
            payload_language: CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
                CombatSelectionDistinctByV2::CardUuid,
            ),
        };
        let mut cursor = SelectionTransactionCursor::new(&family).unwrap();
        let inputs = std::iter::from_fn(|| cursor.next_input()).collect::<Vec<_>>();

        assert_eq!(inputs.len(), 6);
        assert_ne!(inputs[0], inputs[1]);
        assert!(cursor.is_exhausted());
    }
}
