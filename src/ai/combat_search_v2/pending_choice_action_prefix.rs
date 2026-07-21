use std::collections::VecDeque;
use std::sync::Arc;

use crate::state::core::{ClientInput, PendingChoice};
use crate::state::selection::{SelectionResolution, SelectionScope};

/// A search-only partial assignment for one atomic pending-choice action.
///
/// Advancing this value never advances the simulator.  Only
/// `complete_input` compiles a finished prefix into the single `ClientInput`
/// that crosses the engine boundary.
#[derive(Clone, Debug)]
pub(in crate::ai::combat_search_v2) enum PendingChoiceActionPrefix {
    Selection(SelectionActionPrefix),
    Cancel,
}

#[derive(Clone, Debug)]
pub(in crate::ai::combat_search_v2) struct PendingChoiceActionFamily {
    root: SelectionActionPrefix,
    can_cancel: bool,
}

#[derive(Clone, Debug)]
pub(in crate::ai::combat_search_v2) struct PendingChoiceActionWork {
    stack: Vec<PendingChoiceActionPrefix>,
    legal_input_seen: bool,
    next_action_ordinal: usize,
}

#[derive(Clone, Debug)]
pub(in crate::ai::combat_search_v2) struct SelectionActionPrefix {
    completion: SelectionCompletion,
    min_selected: usize,
    max_selected: usize,
    next_index: usize,
    selected_indices: Vec<usize>,
}

#[derive(Clone, Debug)]
enum SelectionCompletion {
    Cards {
        scope: SelectionScope,
        candidate_uuids: Arc<[u32]>,
    },
    Scry {
        candidate_count: usize,
    },
}

impl PendingChoiceActionFamily {
    pub(in crate::ai::combat_search_v2) fn from_choice(choice: &PendingChoice) -> Option<Self> {
        let (completion, min_selected, max_selected, can_cancel) = match choice {
            PendingChoice::HandSelect {
                candidate_uuids,
                min_cards,
                max_cards,
                can_cancel,
                ..
            } => (
                SelectionCompletion::Cards {
                    scope: SelectionScope::Hand,
                    candidate_uuids: Arc::from(unique_uuid_domain(candidate_uuids)),
                },
                *min_cards as usize,
                *max_cards as usize,
                *can_cancel,
            ),
            PendingChoice::GridSelect {
                candidate_uuids,
                min_cards,
                max_cards,
                can_cancel,
                ..
            } => (
                SelectionCompletion::Cards {
                    scope: SelectionScope::Grid,
                    candidate_uuids: Arc::from(unique_uuid_domain(candidate_uuids)),
                },
                *min_cards as usize,
                *max_cards as usize,
                *can_cancel,
            ),
            PendingChoice::ScrySelect { cards, card_uuids } if cards.len() == card_uuids.len() => (
                SelectionCompletion::Scry {
                    candidate_count: cards.len(),
                },
                0,
                cards.len(),
                false,
            ),
            PendingChoice::ScrySelect { .. } => (
                // Malformed frozen domains stay inside the prefix owner.  An
                // infeasible root records one unresolved concrete parent
                // instead of reopening the legacy eager powerset path.
                SelectionCompletion::Scry { candidate_count: 0 },
                1,
                0,
                false,
            ),
            _ => return None,
        };

        let candidate_count = completion_candidate_count(&completion);
        let mut root = SelectionActionPrefix {
            completion,
            min_selected,
            max_selected: max_selected.min(candidate_count),
            next_index: 0,
            selected_indices: Vec::new(),
        };
        root.advance_forced_suffix();
        Some(Self { root, can_cancel })
    }

    #[cfg(test)]
    pub(in crate::ai::combat_search_v2) fn into_initial_prefixes(
        self,
    ) -> Vec<PendingChoiceActionPrefix> {
        let mut prefixes = Vec::with_capacity(usize::from(self.can_cancel) + 1);
        if self.can_cancel {
            prefixes.push(PendingChoiceActionPrefix::Cancel);
        }
        if self.root.is_feasible() {
            prefixes.push(PendingChoiceActionPrefix::Selection(self.root));
        }
        prefixes
    }

    pub(in crate::ai::combat_search_v2) fn into_work_items(self) -> Vec<PendingChoiceActionWork> {
        let mut stack = Vec::with_capacity(usize::from(self.can_cancel) + 1);
        if self.root.is_feasible() {
            stack.push(PendingChoiceActionPrefix::Selection(self.root));
        }
        // Work is a LIFO stack.  Keeping Cancel last makes it the first real
        // engine action considered, without cloning the same concrete parent
        // into a second frontier entry.
        if self.can_cancel {
            stack.push(PendingChoiceActionPrefix::Cancel);
        }
        if stack.is_empty() {
            Vec::new()
        } else {
            vec![PendingChoiceActionWork::from_stack(stack)]
        }
    }

    /// The current family enumerates member sets in frozen candidate order.
    /// When two or more cards may be selected, the engine can observe a
    /// different submitted order, so exhausting this family is not yet an
    /// exhaustive proof over the full `ClientInput` surface.
    pub(in crate::ai::combat_search_v2) fn omits_ordered_variants(&self) -> bool {
        self.root.is_feasible() && self.root.candidate_count() >= 2 && self.root.max_selected >= 2
    }
}

pub struct CanonicalPendingChoiceInputIter {
    work: VecDeque<PendingChoiceActionWork>,
}

pub fn canonical_pending_choice_inputs(
    choice: &PendingChoice,
) -> Option<CanonicalPendingChoiceInputIter> {
    let family = PendingChoiceActionFamily::from_choice(choice)?;
    Some(CanonicalPendingChoiceInputIter {
        work: family.into_work_items().into(),
    })
}

impl Iterator for CanonicalPendingChoiceInputIter {
    type Item = ClientInput;

    fn next(&mut self) -> Option<Self::Item> {
        while let Some(mut work) = self.work.pop_front() {
            while let Some(prefix) = work.pop() {
                if let Some(input) = prefix.complete_input() {
                    if !work.is_empty() {
                        self.work.push_back(work);
                    }
                    return Some(input);
                }
                work.push_ordered(prefix.expand(true));
            }
        }
        None
    }
}

impl PendingChoiceActionWork {
    fn from_stack(stack: Vec<PendingChoiceActionPrefix>) -> Self {
        Self {
            stack,
            legal_input_seen: false,
            next_action_ordinal: 0,
        }
    }

    pub(in crate::ai::combat_search_v2) fn pop(&mut self) -> Option<PendingChoiceActionPrefix> {
        self.stack.pop()
    }

    pub(in crate::ai::combat_search_v2) fn push_next(&mut self, prefix: PendingChoiceActionPrefix) {
        self.stack.push(prefix);
    }

    pub(in crate::ai::combat_search_v2) fn push_ordered(
        &mut self,
        prefixes: Vec<PendingChoiceActionPrefix>,
    ) {
        self.stack.extend(prefixes.into_iter().rev());
    }

    pub(in crate::ai::combat_search_v2) fn is_empty(&self) -> bool {
        self.stack.is_empty()
    }

    pub(in crate::ai::combat_search_v2) fn len(&self) -> usize {
        self.stack.len()
    }

    pub(in crate::ai::combat_search_v2) fn note_legal_input(&mut self) {
        self.legal_input_seen = true;
    }

    pub(in crate::ai::combat_search_v2) fn legal_input_seen(&self) -> bool {
        self.legal_input_seen
    }

    pub(in crate::ai::combat_search_v2) fn current_action_ordinal(&self) -> usize {
        self.next_action_ordinal
    }

    pub(in crate::ai::combat_search_v2) fn note_action_applied(&mut self) {
        self.next_action_ordinal = self.next_action_ordinal.saturating_add(1);
    }
}

impl PendingChoiceActionPrefix {
    #[cfg(test)]
    pub(in crate::ai::combat_search_v2) fn depth(&self) -> usize {
        match self {
            Self::Selection(prefix) => prefix.next_index,
            Self::Cancel => 0,
        }
    }

    pub(in crate::ai::combat_search_v2) fn complete_input(&self) -> Option<ClientInput> {
        match self {
            Self::Selection(prefix) => prefix.complete_input(),
            Self::Cancel => Some(ClientInput::Cancel),
        }
    }

    pub(in crate::ai::combat_search_v2) fn probe_inputs(
        &self,
    ) -> Option<(ClientInput, ClientInput)> {
        let Self::Selection(prefix) = self else {
            return None;
        };
        prefix.probe_inputs()
    }

    pub(in crate::ai::combat_search_v2) fn expand(&self, include_first: bool) -> Vec<Self> {
        let Self::Selection(prefix) = self else {
            return Vec::new();
        };
        prefix
            .expand(include_first)
            .into_iter()
            .map(Self::Selection)
            .collect()
    }
}

impl SelectionActionPrefix {
    fn candidate_count(&self) -> usize {
        match &self.completion {
            SelectionCompletion::Cards {
                candidate_uuids, ..
            } => candidate_uuids.len(),
            SelectionCompletion::Scry { candidate_count } => *candidate_count,
        }
    }

    fn is_feasible(&self) -> bool {
        let selected = self.selected_indices.len();
        let remaining = self.candidate_count().saturating_sub(self.next_index);
        selected <= self.max_selected && selected.saturating_add(remaining) >= self.min_selected
    }

    fn is_complete(&self) -> bool {
        self.next_index == self.candidate_count()
            && self.selected_indices.len() >= self.min_selected
            && self.selected_indices.len() <= self.max_selected
    }

    fn complete_input(&self) -> Option<ClientInput> {
        if !self.is_complete() {
            return None;
        }
        Some(self.input_for_indices(&self.selected_indices))
    }

    fn probe_inputs(&self) -> Option<(ClientInput, ClientInput)> {
        if self.next_index >= self.candidate_count() {
            return None;
        }
        let mut included = self.selected_indices.clone();
        included.push(self.next_index);
        Some((
            self.input_for_indices(&included),
            self.input_for_indices(&self.selected_indices),
        ))
    }

    fn input_for_indices(&self, indices: &[usize]) -> ClientInput {
        match &self.completion {
            SelectionCompletion::Cards {
                scope,
                candidate_uuids,
            } => ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                *scope,
                indices.iter().map(|index| candidate_uuids[*index]),
            )),
            SelectionCompletion::Scry { .. } => ClientInput::SubmitScryDiscard(indices.to_vec()),
        }
    }

    fn expand(&self, include_first: bool) -> Vec<Self> {
        if self.is_complete() || !self.is_feasible() || self.next_index >= self.candidate_count() {
            return Vec::new();
        }

        let mut included = self.clone();
        included.selected_indices.push(included.next_index);
        included.next_index += 1;
        included.advance_forced_suffix();

        let mut excluded = self.clone();
        excluded.next_index += 1;
        excluded.advance_forced_suffix();

        let mut branches = Vec::with_capacity(2);
        let ordered = if include_first {
            [included, excluded]
        } else {
            [excluded, included]
        };
        for branch in ordered {
            if branch.is_feasible() {
                branches.push(branch);
            }
        }
        branches
    }

    fn advance_forced_suffix(&mut self) {
        if !self.is_feasible() {
            return;
        }
        let remaining = self.candidate_count().saturating_sub(self.next_index);
        if self.selected_indices.len() == self.max_selected {
            self.next_index = self.candidate_count();
        } else if self.selected_indices.len().saturating_add(remaining) == self.min_selected {
            self.selected_indices
                .extend(self.next_index..self.candidate_count());
            self.next_index = self.candidate_count();
        }
    }
}

fn unique_uuid_domain(candidate_uuids: &[u32]) -> Vec<u32> {
    let mut unique = Vec::with_capacity(candidate_uuids.len());
    for uuid in candidate_uuids {
        if !unique.contains(uuid) {
            unique.push(*uuid);
        }
    }
    unique
}

fn completion_candidate_count(completion: &SelectionCompletion) -> usize {
    match completion {
        SelectionCompletion::Cards {
            candidate_uuids, ..
        } => candidate_uuids.len(),
        SelectionCompletion::Scry { candidate_count } => *candidate_count,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::state::core::{GridSelectReason, PileType};
    use std::collections::BTreeSet;

    #[test]
    fn scry_prefix_family_lazily_covers_every_subset_once() {
        let choice = PendingChoice::ScrySelect {
            cards: vec![CardId::Strike; 7],
            card_uuids: (0..7).collect(),
        };

        let inputs = collect_inputs(choice);

        assert_eq!(inputs.len(), 128);
        assert_eq!(inputs.iter().cloned().collect::<BTreeSet<_>>().len(), 128);
    }

    #[test]
    fn bounded_grid_selection_covers_cap_external_combinations_and_cancel() {
        let choice = PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: (10..17).collect(),
            min_cards: 2,
            max_cards: 3,
            can_cancel: true,
            reason: GridSelectReason::MoveToDrawPile,
        };

        let inputs = collect_inputs(choice);

        assert_eq!(inputs.len(), 21 + 35 + 1);
        assert!(inputs.contains(&format!("{:?}", ClientInput::Cancel)));
    }

    #[test]
    fn incomplete_prefix_never_compiles_an_engine_input() {
        let choice = PendingChoice::ScrySelect {
            cards: vec![CardId::Strike; 3],
            card_uuids: vec![1, 2, 3],
        };
        let root = PendingChoiceActionFamily::from_choice(&choice)
            .unwrap()
            .into_initial_prefixes()
            .remove(0);

        assert!(root.complete_input().is_none());
        assert!(root.expand(true).iter().all(|prefix| prefix.depth() > 0));
    }

    #[test]
    fn cancel_is_an_independent_work_item_not_buried_under_the_subset_tree() {
        let choice = PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: (10..20).collect(),
            min_cards: 1,
            max_cards: 5,
            can_cancel: true,
            reason: GridSelectReason::MoveToDrawPile,
        };

        let mut work = PendingChoiceActionFamily::from_choice(&choice)
            .unwrap()
            .into_work_items();

        assert_eq!(work.len(), 1);
        assert_eq!(
            work[0].pop().unwrap().complete_input(),
            Some(ClientInput::Cancel)
        );
        assert!(work[0].pop().unwrap().complete_input().is_none());
    }

    #[test]
    fn lazy_input_iterator_yields_cancel_before_descending_the_subset_tree() {
        let choice = PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: (10..20).collect(),
            min_cards: 1,
            max_cards: 5,
            can_cancel: true,
            reason: GridSelectReason::MoveToDrawPile,
        };

        let first_two = canonical_pending_choice_inputs(&choice)
            .unwrap()
            .take(2)
            .collect::<Vec<_>>();

        assert_eq!(first_two[0], ClientInput::Cancel);
        assert!(matches!(first_two[1], ClientInput::SubmitSelection(_)));
    }

    #[test]
    fn duplicate_card_uuids_compile_each_complete_selection_once() {
        let choice = PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: vec![10, 10, 11],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: GridSelectReason::MoveToDrawPile,
        };

        let inputs = collect_inputs(choice);

        assert_eq!(inputs.len(), 2);
        assert_eq!(inputs.iter().cloned().collect::<BTreeSet<_>>().len(), 2);
    }

    #[test]
    fn infeasible_selection_without_cancel_has_no_virtual_work() {
        let choice = PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: vec![10],
            min_cards: 2,
            max_cards: 2,
            can_cancel: false,
            reason: GridSelectReason::MoveToDrawPile,
        };

        let work = PendingChoiceActionFamily::from_choice(&choice)
            .unwrap()
            .into_work_items();

        assert!(work.is_empty());
    }

    #[test]
    fn multi_card_family_declares_omitted_order_variants() {
        let choice = PendingChoice::ScrySelect {
            cards: vec![CardId::Strike; 3],
            card_uuids: vec![1, 2, 3],
        };
        let family = PendingChoiceActionFamily::from_choice(&choice).unwrap();

        assert!(family.omits_ordered_variants());
    }

    #[test]
    fn single_card_family_has_no_order_variant_gap() {
        let choice = PendingChoice::GridSelect {
            source_pile: PileType::Discard,
            candidate_uuids: vec![10, 11],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: GridSelectReason::MoveToDrawPile,
        };
        let family = PendingChoiceActionFamily::from_choice(&choice).unwrap();

        assert!(!family.omits_ordered_variants());
    }

    #[test]
    fn malformed_scry_domain_becomes_infeasible_instead_of_falling_back_eagerly() {
        let choice = PendingChoice::ScrySelect {
            cards: vec![CardId::Strike; 12],
            card_uuids: vec![1],
        };
        let family = PendingChoiceActionFamily::from_choice(&choice)
            .expect("malformed combinatorial choices must remain prefix-owned");

        assert!(family.into_work_items().is_empty());
    }

    fn collect_inputs(choice: PendingChoice) -> Vec<String> {
        let mut stack = PendingChoiceActionFamily::from_choice(&choice)
            .unwrap()
            .into_initial_prefixes();
        let mut inputs = Vec::new();
        while let Some(prefix) = stack.pop() {
            if let Some(input) = prefix.complete_input() {
                inputs.push(format!("{input:?}"));
            } else {
                stack.extend(prefix.expand(true));
            }
        }
        inputs
    }
}
