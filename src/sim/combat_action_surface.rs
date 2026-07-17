use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::content::cards::CardId;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::core::{
    ClientInput, EngineState, GridSelectReason, HandSelectReason, PendingChoice, PileType,
};
use crate::state::selection::SelectionScope;

/// A linear-size description of every input accepted at one combat boundary.
///
/// Atomic inputs remain explicit.  Combinatorial pending choices are described
/// as an ordered-input language instead of materializing every payload.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLegalActionSurfaceV2 {
    pub atomic_actions: Vec<ClientInput>,
    pub selection_families: Vec<CombatSelectionActionFamilyV2>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSelectionActionFamilyV2 {
    pub input_encoding: CombatSelectionInputEncodingV2,
    pub reason: CombatSelectionReasonV2,
    pub source_pile: Option<PileType>,
    pub raw_domain: Vec<CombatSelectionDomainCandidateV2>,
    pub raw_domain_count: u64,
    pub eligible_domain_count: u64,
    pub max_distinct_selection_count: u64,
    pub declared_min: u64,
    pub declared_max: u64,
    pub effective_max: u64,
    pub selection_status: CombatSelectionStatusV2,
    pub payload_language: CombatSelectionPayloadLanguageV2,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSelectionInputEncodingV2 {
    SubmitSelectionHandCardUuids,
    SubmitSelectionGridCardUuids,
    SubmitScryDiscardIndices,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum CombatSelectionReasonV2 {
    Hand(HandSelectReason),
    Grid(GridSelectReason),
    ScryDiscard,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "address_kind", rename_all = "snake_case")]
pub enum CombatSelectionDomainCandidateV2 {
    CardUuid {
        ordinal: u64,
        uuid: u32,
        card_id: Option<CardId>,
        upgrades: Option<u8>,
        eligible: bool,
    },
    ScryIndex {
        index: u64,
        card_id: Option<CardId>,
        card_uuid: Option<u32>,
        currently_present: bool,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", content = "reason", rename_all = "snake_case")]
pub enum CombatSelectionStatusV2 {
    Enabled,
    Disabled(CombatSelectionDisabledReasonV2),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSelectionDisabledReasonV2 {
    InvalidGridSource,
    MalformedScryDomain,
    InfeasibleBounds,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "language", content = "distinct_by", rename_all = "snake_case")]
pub enum CombatSelectionPayloadLanguageV2 {
    OrderedDistinctSequence(CombatSelectionDistinctByV2),
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSelectionDistinctByV2 {
    CardUuid,
    ScryIndexAndCardUuid,
}

pub fn combat_legal_action_surface_v2(
    engine: &EngineState,
    combat: &CombatState,
) -> CombatLegalActionSurfaceV2 {
    match engine {
        EngineState::PendingChoice(choice) => pending_choice_surface(choice, combat),
        _ => CombatLegalActionSurfaceV2 {
            atomic_actions: crate::sim::combat_legal_actions::engine_atomic_actions(engine, combat),
            selection_families: Vec::new(),
        },
    }
}

/// Exact pending-choice membership without constructing an eager candidate
/// list.  The symbolic surface projector and the simulator membership gate
/// share this semantic owner.
pub fn pending_choice_input_is_legal(
    choice: &PendingChoice,
    combat: &CombatState,
    input: &ClientInput,
) -> bool {
    match (choice, input) {
        (
            PendingChoice::HandSelect {
                min_cards,
                max_cards,
                candidate_uuids,
                ..
            },
            ClientInput::SubmitSelection(resolution),
        ) if resolution.scope == SelectionScope::Hand => selection_is_legal(
            &resolution.selected_card_uuids(),
            candidate_uuids,
            *min_cards as usize,
            *max_cards as usize,
            &combat.zones.hand,
        ),
        (
            PendingChoice::GridSelect {
                source_pile,
                min_cards,
                max_cards,
                candidate_uuids,
                reason,
                ..
            },
            ClientInput::SubmitSelection(resolution),
        ) if resolution.scope == SelectionScope::Grid => {
            grid_source_is_legal(*reason, *source_pile)
                && selection_is_legal(
                    &resolution.selected_card_uuids(),
                    candidate_uuids,
                    *min_cards as usize,
                    *max_cards as usize,
                    cards_for_pile(combat, *source_pile),
                )
        }
        (
            PendingChoice::HandSelect { can_cancel, .. }
            | PendingChoice::GridSelect { can_cancel, .. },
            ClientInput::Cancel,
        ) => *can_cancel,
        (
            PendingChoice::ScrySelect { cards, card_uuids },
            ClientInput::SubmitScryDiscard(indices),
        ) => {
            let selected_uuids = indices
                .iter()
                .filter_map(|index| card_uuids.get(*index).copied())
                .collect::<Vec<_>>();
            cards.len() == card_uuids.len()
                && indices.iter().all(|index| *index < cards.len())
                && all_unique(indices)
                && selected_uuids.len() == indices.len()
                && all_unique(&selected_uuids)
                && indices.iter().all(|index| {
                    combat
                        .zones
                        .draw_pile
                        .iter()
                        .any(|card| card.uuid == card_uuids[*index])
                })
        }
        (PendingChoice::DiscoverySelect(choice), ClientInput::SubmitDiscoverChoice(index)) => {
            *index < choice.cards.len()
        }
        (PendingChoice::DiscoverySelect(choice), ClientInput::Cancel) => choice.can_skip,
        (
            PendingChoice::CardRewardSelect { cards, .. },
            ClientInput::SubmitDiscoverChoice(index),
        ) => *index < cards.len(),
        (PendingChoice::CardRewardSelect { can_skip, .. }, ClientInput::Cancel) => *can_skip,
        (
            PendingChoice::ForeignInfluenceSelect { cards, .. },
            ClientInput::SubmitDiscoverChoice(index),
        ) => *index < cards.len(),
        (PendingChoice::ChooseOneSelect { choices }, ClientInput::SubmitDiscoverChoice(index)) => {
            *index < choices.len()
        }
        (PendingChoice::StanceChoice, ClientInput::SubmitDiscoverChoice(index)) => *index < 2,
        _ => false,
    }
}

fn pending_choice_surface(
    choice: &PendingChoice,
    combat: &CombatState,
) -> CombatLegalActionSurfaceV2 {
    match choice {
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => {
            let family = card_uuid_family(
                CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids,
                CombatSelectionReasonV2::Hand(*reason),
                None,
                candidate_uuids,
                *min_cards as usize,
                *max_cards as usize,
                &combat.zones.hand,
                true,
            );
            symbolic_surface(family, *can_cancel)
        }
        PendingChoice::GridSelect {
            source_pile,
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason,
        } => {
            let source_is_valid = grid_source_is_legal(*reason, *source_pile);
            let family = card_uuid_family(
                CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids,
                CombatSelectionReasonV2::Grid(*reason),
                Some(*source_pile),
                candidate_uuids,
                *min_cards as usize,
                *max_cards as usize,
                cards_for_pile(combat, *source_pile),
                source_is_valid,
            );
            symbolic_surface(family, *can_cancel)
        }
        PendingChoice::ScrySelect { cards, card_uuids } => CombatLegalActionSurfaceV2 {
            atomic_actions: Vec::new(),
            selection_families: vec![scry_family(cards, card_uuids, combat)],
        },
        PendingChoice::DiscoverySelect(choice) => {
            let mut actions = (0..choice.cards.len())
                .map(ClientInput::SubmitDiscoverChoice)
                .collect::<Vec<_>>();
            if choice.can_skip {
                actions.push(ClientInput::Cancel);
            }
            explicit_surface(actions)
        }
        PendingChoice::CardRewardSelect {
            cards, can_skip, ..
        } => {
            let mut actions = (0..cards.len())
                .map(ClientInput::SubmitDiscoverChoice)
                .collect::<Vec<_>>();
            if *can_skip {
                actions.push(ClientInput::Cancel);
            }
            explicit_surface(actions)
        }
        PendingChoice::ForeignInfluenceSelect { cards, .. } => explicit_surface(
            (0..cards.len())
                .map(ClientInput::SubmitDiscoverChoice)
                .collect(),
        ),
        PendingChoice::ChooseOneSelect { choices } => explicit_surface(
            (0..choices.len())
                .map(ClientInput::SubmitDiscoverChoice)
                .collect(),
        ),
        PendingChoice::StanceChoice => explicit_surface(vec![
            ClientInput::SubmitDiscoverChoice(0),
            ClientInput::SubmitDiscoverChoice(1),
        ]),
    }
}

fn explicit_surface(actions: Vec<ClientInput>) -> CombatLegalActionSurfaceV2 {
    CombatLegalActionSurfaceV2 {
        atomic_actions: actions,
        selection_families: Vec::new(),
    }
}

fn symbolic_surface(
    family: CombatSelectionActionFamilyV2,
    can_cancel: bool,
) -> CombatLegalActionSurfaceV2 {
    CombatLegalActionSurfaceV2 {
        atomic_actions: if can_cancel {
            vec![ClientInput::Cancel]
        } else {
            Vec::new()
        },
        selection_families: vec![family],
    }
}

#[allow(clippy::too_many_arguments)]
fn card_uuid_family(
    input_encoding: CombatSelectionInputEncodingV2,
    reason: CombatSelectionReasonV2,
    source_pile: Option<PileType>,
    candidate_uuids: &[u32],
    declared_min: usize,
    declared_max: usize,
    source_cards: &[CombatCard],
    source_is_valid: bool,
) -> CombatSelectionActionFamilyV2 {
    let mut seen = HashSet::new();
    let mut eligible_count = 0usize;
    let raw_domain = candidate_uuids
        .iter()
        .copied()
        .enumerate()
        .map(|(ordinal, uuid)| {
            let card = source_cards.iter().find(|card| card.uuid == uuid);
            let eligible = source_is_valid && card.is_some() && seen.insert(uuid);
            eligible_count += usize::from(eligible);
            CombatSelectionDomainCandidateV2::CardUuid {
                ordinal: to_u64(ordinal),
                uuid,
                card_id: card.map(|card| card.id),
                upgrades: card.map(|card| card.upgrades),
                eligible,
            }
        })
        .collect::<Vec<_>>();
    let effective_max = declared_max.min(eligible_count);
    let selection_status = if !source_is_valid {
        CombatSelectionStatusV2::Disabled(CombatSelectionDisabledReasonV2::InvalidGridSource)
    } else if declared_min > effective_max {
        CombatSelectionStatusV2::Disabled(CombatSelectionDisabledReasonV2::InfeasibleBounds)
    } else {
        CombatSelectionStatusV2::Enabled
    };

    CombatSelectionActionFamilyV2 {
        input_encoding,
        reason,
        source_pile,
        raw_domain_count: to_u64(candidate_uuids.len()),
        eligible_domain_count: to_u64(eligible_count),
        max_distinct_selection_count: to_u64(eligible_count),
        raw_domain,
        declared_min: to_u64(declared_min),
        declared_max: to_u64(declared_max),
        effective_max: to_u64(effective_max),
        selection_status,
        payload_language: CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
            CombatSelectionDistinctByV2::CardUuid,
        ),
    }
}

fn scry_family(
    cards: &[CardId],
    card_uuids: &[u32],
    combat: &CombatState,
) -> CombatSelectionActionFamilyV2 {
    let raw_domain_count = cards.len().max(card_uuids.len());
    let raw_domain = (0..raw_domain_count)
        .map(|index| {
            let card_uuid = card_uuids.get(index).copied();
            let currently_present = card_uuid
                .is_some_and(|uuid| combat.zones.draw_pile.iter().any(|card| card.uuid == uuid));
            CombatSelectionDomainCandidateV2::ScryIndex {
                index: to_u64(index),
                card_id: cards.get(index).copied(),
                card_uuid,
                currently_present,
            }
        })
        .collect::<Vec<_>>();
    let lengths_match = cards.len() == card_uuids.len();
    let present_indices = card_uuids
        .iter()
        .filter(|uuid| {
            combat
                .zones
                .draw_pile
                .iter()
                .any(|card| card.uuid == **uuid)
        })
        .count();
    let unique_present = card_uuids
        .iter()
        .copied()
        .filter(|uuid| combat.zones.draw_pile.iter().any(|card| card.uuid == *uuid))
        .collect::<HashSet<_>>()
        .len();
    let selection_status = if lengths_match {
        CombatSelectionStatusV2::Enabled
    } else {
        CombatSelectionStatusV2::Disabled(CombatSelectionDisabledReasonV2::MalformedScryDomain)
    };

    CombatSelectionActionFamilyV2 {
        input_encoding: CombatSelectionInputEncodingV2::SubmitScryDiscardIndices,
        reason: CombatSelectionReasonV2::ScryDiscard,
        source_pile: Some(PileType::Draw),
        raw_domain,
        raw_domain_count: to_u64(raw_domain_count),
        eligible_domain_count: if lengths_match {
            to_u64(present_indices)
        } else {
            0
        },
        max_distinct_selection_count: if lengths_match {
            to_u64(unique_present)
        } else {
            0
        },
        declared_min: 0,
        declared_max: to_u64(cards.len()),
        effective_max: if lengths_match {
            to_u64(unique_present)
        } else {
            0
        },
        selection_status,
        payload_language: CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
            CombatSelectionDistinctByV2::ScryIndexAndCardUuid,
        ),
    }
}

fn cards_for_pile(combat: &CombatState, pile: PileType) -> &[CombatCard] {
    match pile {
        PileType::Draw => &combat.zones.draw_pile,
        PileType::Discard => &combat.zones.discard_pile,
        PileType::Exhaust => &combat.zones.exhaust_pile,
        PileType::Hand => &combat.zones.hand,
        PileType::Limbo => &combat.zones.limbo,
        PileType::MasterDeck => &combat.meta.master_deck_snapshot,
    }
}

fn grid_source_is_legal(reason: GridSelectReason, source_pile: PileType) -> bool {
    match reason {
        GridSelectReason::DiscardToHand
        | GridSelectReason::DiscardToHandNoCostChange
        | GridSelectReason::DiscardToHandRetain => source_pile == PileType::Discard,
        GridSelectReason::MoveToDrawPile => {
            matches!(source_pile, PileType::Discard | PileType::Exhaust)
        }
        GridSelectReason::Exhume { .. } => source_pile == PileType::Exhaust,
        GridSelectReason::DrawPileToHand
        | GridSelectReason::SkillFromDeckToHand
        | GridSelectReason::AttackFromDeckToHand
        | GridSelectReason::Omniscience { .. } => source_pile == PileType::Draw,
    }
}

fn selection_is_legal(
    selected: &[u32],
    candidates: &[u32],
    min_cards: usize,
    max_cards: usize,
    source_cards: &[CombatCard],
) -> bool {
    selected.len() >= min_cards
        && selected.len() <= max_cards.min(candidates.len())
        && all_unique(selected)
        && selected.iter().all(|uuid| candidates.contains(uuid))
        && selected
            .iter()
            .all(|uuid| source_cards.iter().any(|card| card.uuid == *uuid))
}

fn all_unique<T: Ord + Copy>(values: &[T]) -> bool {
    let mut sorted = values.to_vec();
    sorted.sort_unstable();
    !sorted.windows(2).any(|pair| pair[0] == pair[1])
}

fn to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::runtime::combat::CombatCard;
    use crate::sim::combat::{CombatPosition, CombatStepper, EngineCombatStepper};
    use crate::state::selection::SelectionResolution;

    #[test]
    fn sixty_four_card_scry_is_one_linear_symbolic_family() {
        let mut combat = crate::test_support::blank_test_combat();
        let cards = vec![CardId::Strike; 64];
        let card_uuids = (1..=64).collect::<Vec<_>>();
        combat.zones.draw_pile = card_uuids
            .iter()
            .map(|uuid| CombatCard::new(CardId::Strike, *uuid))
            .collect();
        let surface = combat_legal_action_surface_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect { cards, card_uuids }),
            &combat,
        );

        assert!(surface.atomic_actions.is_empty());
        assert_eq!(surface.selection_families.len(), 1);
        let family = &surface.selection_families[0];
        assert_eq!(family.raw_domain_count, 64);
        assert_eq!(family.raw_domain.len(), 64);
        assert_eq!(family.declared_min, 0);
        assert_eq!(family.declared_max, 64);
        assert_eq!(family.selection_status, CombatSelectionStatusV2::Enabled);
    }

    #[test]
    fn engine_stepper_separates_atomic_actions_from_exact_structured_membership() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = (0..24)
            .map(|index| CombatCard::new(CardId::Strike, 100 + index))
            .collect();
        let choice = PendingChoice::HandSelect {
            candidate_uuids: (100..124).collect(),
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: HandSelectReason::Discard,
        };
        let position = CombatPosition::new(EngineState::PendingChoice(choice), combat);
        let stepper = EngineCombatStepper;
        let surface = stepper.legal_action_surface(&position);
        let last_candidate = ClientInput::SubmitSelection(SelectionResolution::card_uuids(
            SelectionScope::Hand,
            [123],
        ));

        assert!(surface.atomic_actions.is_empty());
        assert_eq!(surface.selection_families.len(), 1);
        assert!(stepper.is_legal_action(&position, &last_candidate));
        assert!(stepper
            .choice_for_legal_input(&position, &last_candidate)
            .is_some());
    }

    #[test]
    fn empty_hand_submission_and_cancel_remain_distinct() {
        let combat = crate::test_support::blank_test_combat();
        let choice = PendingChoice::HandSelect {
            candidate_uuids: Vec::new(),
            min_cards: 0,
            max_cards: 0,
            can_cancel: true,
            reason: crate::state::core::HandSelectReason::Discard,
        };
        let surface =
            combat_legal_action_surface_v2(&EngineState::PendingChoice(choice.clone()), &combat);

        assert_eq!(surface.atomic_actions, vec![ClientInput::Cancel]);
        assert_eq!(surface.selection_families.len(), 1);
        assert_eq!(
            surface.selection_families[0].selection_status,
            CombatSelectionStatusV2::Enabled
        );
        assert!(pending_choice_input_is_legal(
            &choice,
            &combat,
            &ClientInput::Cancel
        ));
        assert!(pending_choice_input_is_legal(
            &choice,
            &combat,
            &ClientInput::SubmitSelection(
                crate::state::selection::SelectionResolution::card_uuids(SelectionScope::Hand, [],)
            )
        ));
    }

    #[test]
    fn hand_family_preserves_candidates_beyond_legacy_preview_cap() {
        let mut combat = crate::test_support::blank_test_combat();
        let candidate_uuids = (100..124).collect::<Vec<_>>();
        combat.zones.hand = candidate_uuids
            .iter()
            .map(|uuid| CombatCard::new(CardId::Strike, *uuid))
            .collect();
        let surface = combat_legal_action_surface_v2(
            &EngineState::PendingChoice(PendingChoice::HandSelect {
                candidate_uuids,
                min_cards: 1,
                max_cards: 3,
                can_cancel: false,
                reason: crate::state::core::HandSelectReason::Discard,
            }),
            &combat,
        );

        let family = &surface.selection_families[0];
        assert_eq!(family.raw_domain_count, 24);
        assert_eq!(family.raw_domain.len(), 24);
        assert_eq!(family.eligible_domain_count, 24);
        assert_eq!(family.effective_max, 3);
    }

    #[test]
    fn invalid_grid_source_disables_submit_but_preserves_cancel() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 7)];
        let choice = PendingChoice::GridSelect {
            source_pile: PileType::Draw,
            candidate_uuids: vec![7],
            min_cards: 1,
            max_cards: 1,
            can_cancel: true,
            reason: GridSelectReason::DiscardToHand,
        };
        let surface =
            combat_legal_action_surface_v2(&EngineState::PendingChoice(choice.clone()), &combat);

        assert_eq!(surface.atomic_actions, vec![ClientInput::Cancel]);
        let family = &surface.selection_families[0];
        assert_eq!(family.eligible_domain_count, 0);
        assert_eq!(family.effective_max, 0);
        assert_eq!(
            family.selection_status,
            CombatSelectionStatusV2::Disabled(CombatSelectionDisabledReasonV2::InvalidGridSource)
        );
        assert!(pending_choice_input_is_legal(
            &choice,
            &combat,
            &ClientInput::Cancel
        ));
        assert!(!pending_choice_input_is_legal(
            &choice,
            &combat,
            &ClientInput::SubmitSelection(
                crate::state::selection::SelectionResolution::card_uuids(SelectionScope::Grid, [7],)
            )
        ));
    }

    #[test]
    fn duplicate_scry_uuids_keep_both_addresses_but_limit_distinct_payloads() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 7)];
        let choice = PendingChoice::ScrySelect {
            cards: vec![CardId::Strike, CardId::Strike],
            card_uuids: vec![7, 7],
        };
        let surface =
            combat_legal_action_surface_v2(&EngineState::PendingChoice(choice.clone()), &combat);

        let family = &surface.selection_families[0];
        assert_eq!(family.eligible_domain_count, 2);
        assert_eq!(family.max_distinct_selection_count, 1);
        assert_eq!(family.effective_max, 1);
        assert_eq!(
            family.payload_language,
            CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
                CombatSelectionDistinctByV2::ScryIndexAndCardUuid
            )
        );
        assert!(pending_choice_input_is_legal(
            &choice,
            &combat,
            &ClientInput::SubmitScryDiscard(vec![0])
        ));
        assert!(pending_choice_input_is_legal(
            &choice,
            &combat,
            &ClientInput::SubmitScryDiscard(vec![1])
        ));
        assert!(!pending_choice_input_is_legal(
            &choice,
            &combat,
            &ClientInput::SubmitScryDiscard(vec![0, 1])
        ));
    }

    #[test]
    fn malformed_scry_is_disabled_without_eager_fallback() {
        let combat = crate::test_support::blank_test_combat();
        let surface = combat_legal_action_surface_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Defend],
                card_uuids: vec![1],
            }),
            &combat,
        );

        assert_eq!(surface.selection_families.len(), 1);
        assert_eq!(
            surface.selection_families[0].selection_status,
            CombatSelectionStatusV2::Disabled(CombatSelectionDisabledReasonV2::MalformedScryDomain)
        );
    }
}
