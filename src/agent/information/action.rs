//! Canonical public combat candidates and private simulator resolution.

use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::content::cards::{CardId, CardType};
use crate::runtime::action::CardDestination;
use crate::runtime::combat::{CombatState, StanceId};
use crate::sim::combat_action_equivalence::canonical_combat_action_representatives_v1;
use crate::sim::combat_action_surface::{
    combat_legal_action_surface_v2, CombatIndexedChoiceCandidateV2,
    CombatIndexedChoiceInputEncodingV2, CombatIndexedChoiceReasonV2, CombatSelectionActionFamilyV2,
    CombatSelectionDistinctByV2, CombatSelectionDomainCandidateV2, CombatSelectionInputEncodingV2,
    CombatSelectionPayloadLanguageV2, CombatSelectionReasonV2, CombatSelectionStatusV2,
};
use crate::state::core::{ClientInput, EngineState, PileType};
use crate::state::selection::{SelectionResolution, SelectionScope};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCombatActionSurfaceV1 {
    /// One public semantic action for each retained canonical exact input.
    pub atomic_actions: Vec<PublicCombatAtomicActionV1>,
    /// Enabled symbolic families. Disabled diagnostic families are not legal
    /// agent candidates and therefore do not appear here.
    pub selection_families: Vec<PublicCombatSelectionFamilyV1>,
    pub indexed_choice: Option<PublicCombatIndexedChoiceSurfaceV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PublicCombatAtomicActionV1 {
    PlayCard {
        hand_index: usize,
        target_monster_index: Option<usize>,
    },
    UsePotion {
        potion_index: usize,
        target_monster_index: Option<usize>,
    },
    DiscardPotion {
        potion_index: usize,
    },
    EndTurn,
    SubmitIndexedChoice {
        choice_index: usize,
    },
    Proceed,
    Cancel,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCombatIndexedChoiceSurfaceV1 {
    pub input_encoding: CombatIndexedChoiceInputEncodingV2,
    pub reason: PublicCombatIndexedChoiceReasonV1,
    pub candidates: Vec<PublicCombatIndexedChoiceCandidateV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PublicCombatIndexedChoiceReasonV1 {
    Discovery {
        colorless: bool,
        card_type: Option<CardType>,
        amount: u8,
    },
    CardReward {
        destination: CardDestination,
    },
    ForeignInfluence {
        upgraded: bool,
    },
    ChooseOne,
    Stance,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PublicCombatIndexedChoiceCandidateV1 {
    Card { card_id: CardId, upgrades: u8 },
    Stance { stance: StanceId },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicCombatSelectionFamilyV1 {
    pub input_encoding: CombatSelectionInputEncodingV2,
    pub reason: CombatSelectionReasonV2,
    pub source_pile: Option<PileType>,
    pub domain: Vec<PublicCombatSelectionDomainCandidateV1>,
    pub raw_domain_count: u64,
    pub eligible_domain_count: u64,
    pub max_distinct_selection_count: u64,
    pub declared_min: u64,
    pub declared_max: u64,
    pub effective_max: u64,
    pub distinct_by: PublicCombatSelectionDistinctByV1,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PublicCombatSelectionDomainCandidateV1 {
    Card {
        ordinal: u64,
        card_id: Option<CardId>,
        upgrades: Option<u8>,
        eligible: bool,
    },
    Scry {
        index: u64,
        card_id: Option<CardId>,
        currently_present: bool,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicCombatSelectionDistinctByV1 {
    CardOccurrence,
    ScryOccurrence,
}

/// Exact handles aligned with [`PublicCombatActionSurfaceV1`].
///
/// A policy never observes this table. The environment uses it only after a
/// public candidate ordinal has been selected.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatActionResolutionTableV1 {
    pub atomic_inputs: Vec<ClientInput>,
    pub selection_families: Vec<CombatSelectionFamilyResolutionV1>,
    pub monster_entity_ids_by_order: Vec<crate::EntityId>,
    pub potion_uuids_by_slot: Vec<Option<u32>>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSelectionFamilyResolutionV1 {
    pub input_encoding: CombatSelectionInputEncodingV2,
    pub distinct_by: PublicCombatSelectionDistinctByV1,
    pub domain: Vec<CombatSelectionDomainResolutionV1>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatSelectionDomainResolutionV1 {
    Card { uuid: u32 },
    Scry { index: u64, card_uuid: Option<u32> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatActionProjectionV1 {
    pub public: PublicCombatActionSurfaceV1,
    pub private_resolution: CombatActionResolutionTableV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PublicCombatActionChoiceV1 {
    Atomic {
        action_ordinal: usize,
    },
    Selection {
        family_ordinal: usize,
        selected_domain_ordinals: Vec<usize>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatActionProjectionErrorV1 {
    MissingMonsterTarget(crate::EntityId),
    UnsupportedAtomicInput,
    IndexedChoiceMetadataMissing,
    IndexedChoiceCandidateMissing(usize),
    SelectionPayloadMismatch,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatActionResolutionErrorV1 {
    AtomicSurfaceMisaligned,
    AtomicOrdinalOutOfRange(usize),
    SelectionSurfaceMisaligned,
    SelectionFamilyOrdinalOutOfRange(usize),
    SelectionFamilyMismatch,
    SelectionCountOutOfRange,
    SelectionDomainOrdinalOutOfRange(usize),
    SelectionDomainIneligible(usize),
    DuplicateSelectionOccurrence(usize),
    SelectionDomainEncodingMismatch,
}

impl Display for CombatActionProjectionErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingMonsterTarget(entity_id) => write!(
                formatter,
                "combat action target entity {entity_id} has no public monster index"
            ),
            Self::UnsupportedAtomicInput => {
                formatter.write_str("combat action surface contains a non-combat atomic input")
            }
            Self::IndexedChoiceMetadataMissing => {
                formatter.write_str("indexed combat action has no public choice metadata")
            }
            Self::IndexedChoiceCandidateMissing(index) => write!(
                formatter,
                "indexed combat action {index} has no aligned public candidate"
            ),
            Self::SelectionPayloadMismatch => formatter
                .write_str("combat selection domain does not match its declared input encoding"),
        }
    }
}

impl Error for CombatActionProjectionErrorV1 {}

impl Display for CombatActionResolutionErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AtomicSurfaceMisaligned => {
                formatter.write_str("public atomic actions and private inputs are misaligned")
            }
            Self::AtomicOrdinalOutOfRange(ordinal) => {
                write!(
                    formatter,
                    "public atomic action ordinal {ordinal} is out of range"
                )
            }
            Self::SelectionSurfaceMisaligned => formatter
                .write_str("public selection families and private resolutions are misaligned"),
            Self::SelectionFamilyOrdinalOutOfRange(ordinal) => write!(
                formatter,
                "public selection family ordinal {ordinal} is out of range"
            ),
            Self::SelectionFamilyMismatch => {
                formatter.write_str("public and private selection families do not match")
            }
            Self::SelectionCountOutOfRange => {
                formatter.write_str("public selection count is outside the legal bounds")
            }
            Self::SelectionDomainOrdinalOutOfRange(ordinal) => write!(
                formatter,
                "public selection domain ordinal {ordinal} is out of range"
            ),
            Self::SelectionDomainIneligible(ordinal) => write!(
                formatter,
                "public selection domain ordinal {ordinal} is not eligible"
            ),
            Self::DuplicateSelectionOccurrence(ordinal) => write!(
                formatter,
                "public selection domain ordinal {ordinal} repeats one exact occurrence"
            ),
            Self::SelectionDomainEncodingMismatch => {
                formatter.write_str("public selection does not match its private input encoding")
            }
        }
    }
}

impl Error for CombatActionResolutionErrorV1 {}

pub fn project_public_combat_actions_v1(
    engine: &EngineState,
    combat: &CombatState,
) -> Result<CombatActionProjectionV1, CombatActionProjectionErrorV1> {
    let exact = combat_legal_action_surface_v2(engine, combat);
    let representatives =
        canonical_combat_action_representatives_v1(engine, combat, &exact.atomic_actions);
    let indexed_choice = exact.indexed_choice.as_ref().map(project_indexed_choice);

    let mut atomic_actions = Vec::new();
    let mut atomic_inputs = Vec::new();
    for (ordinal, input) in exact.atomic_actions.iter().enumerate() {
        if representatives.get(ordinal).copied() != Some(ordinal) {
            continue;
        }
        atomic_actions.push(project_atomic_action(
            input,
            combat,
            indexed_choice.as_ref(),
        )?);
        atomic_inputs.push(input.clone());
    }

    let mut selection_families = Vec::new();
    let mut selection_resolutions = Vec::new();
    for family in exact
        .selection_families
        .iter()
        .filter(|family| family.selection_status == CombatSelectionStatusV2::Enabled)
    {
        let (public, private_resolution) = project_selection_family(family)?;
        selection_families.push(public);
        selection_resolutions.push(private_resolution);
    }

    Ok(CombatActionProjectionV1 {
        public: PublicCombatActionSurfaceV1 {
            atomic_actions,
            selection_families,
            indexed_choice,
        },
        private_resolution: CombatActionResolutionTableV1 {
            atomic_inputs,
            selection_families: selection_resolutions,
            monster_entity_ids_by_order: combat
                .entities
                .monsters
                .iter()
                .map(|monster| monster.id)
                .collect(),
            potion_uuids_by_slot: combat
                .entities
                .potions
                .iter()
                .map(|potion| potion.as_ref().map(|potion| potion.uuid))
                .collect(),
        },
    })
}

pub fn resolve_public_combat_action_v1(
    projection: &CombatActionProjectionV1,
    choice: &PublicCombatActionChoiceV1,
) -> Result<ClientInput, CombatActionResolutionErrorV1> {
    match choice {
        PublicCombatActionChoiceV1::Atomic { action_ordinal } => {
            if projection.public.atomic_actions.len()
                != projection.private_resolution.atomic_inputs.len()
            {
                return Err(CombatActionResolutionErrorV1::AtomicSurfaceMisaligned);
            }
            projection
                .private_resolution
                .atomic_inputs
                .get(*action_ordinal)
                .cloned()
                .ok_or(CombatActionResolutionErrorV1::AtomicOrdinalOutOfRange(
                    *action_ordinal,
                ))
        }
        PublicCombatActionChoiceV1::Selection {
            family_ordinal,
            selected_domain_ordinals,
        } => resolve_public_combat_selection_v1(
            projection,
            *family_ordinal,
            selected_domain_ordinals,
        ),
    }
}

fn resolve_public_combat_selection_v1(
    projection: &CombatActionProjectionV1,
    family_ordinal: usize,
    selected_domain_ordinals: &[usize],
) -> Result<ClientInput, CombatActionResolutionErrorV1> {
    if projection.public.selection_families.len()
        != projection.private_resolution.selection_families.len()
    {
        return Err(CombatActionResolutionErrorV1::SelectionSurfaceMisaligned);
    }
    let public = projection
        .public
        .selection_families
        .get(family_ordinal)
        .ok_or(CombatActionResolutionErrorV1::SelectionFamilyOrdinalOutOfRange(family_ordinal))?;
    let private = projection
        .private_resolution
        .selection_families
        .get(family_ordinal)
        .ok_or(CombatActionResolutionErrorV1::SelectionSurfaceMisaligned)?;
    if public.input_encoding != private.input_encoding
        || public.distinct_by != private.distinct_by
        || public.domain.len() != private.domain.len()
    {
        return Err(CombatActionResolutionErrorV1::SelectionFamilyMismatch);
    }
    let selected_count = selected_domain_ordinals.len() as u64;
    if selected_count < public.declared_min || selected_count > public.effective_max {
        return Err(CombatActionResolutionErrorV1::SelectionCountOutOfRange);
    }

    let mut public_occurrences = HashSet::new();
    let mut private_occurrences = HashSet::new();
    let mut card_uuids = Vec::with_capacity(selected_domain_ordinals.len());
    let mut scry_indices = Vec::with_capacity(selected_domain_ordinals.len());
    for domain_ordinal in selected_domain_ordinals.iter().copied() {
        let public_candidate = public.domain.get(domain_ordinal).ok_or(
            CombatActionResolutionErrorV1::SelectionDomainOrdinalOutOfRange(domain_ordinal),
        )?;
        let private_candidate = private.domain.get(domain_ordinal).ok_or(
            CombatActionResolutionErrorV1::SelectionDomainOrdinalOutOfRange(domain_ordinal),
        )?;
        match (public_candidate, private_candidate, public.input_encoding) {
            (
                PublicCombatSelectionDomainCandidateV1::Card {
                    ordinal,
                    eligible: true,
                    ..
                },
                CombatSelectionDomainResolutionV1::Card { uuid },
                CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids
                | CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids,
            ) => {
                if !public_occurrences.insert(*ordinal)
                    || !private_occurrences.insert((0_u8, *uuid as u64))
                {
                    return Err(CombatActionResolutionErrorV1::DuplicateSelectionOccurrence(
                        domain_ordinal,
                    ));
                }
                card_uuids.push(*uuid);
            }
            (
                PublicCombatSelectionDomainCandidateV1::Scry {
                    index,
                    currently_present: true,
                    ..
                },
                CombatSelectionDomainResolutionV1::Scry {
                    index: private_index,
                    card_uuid,
                },
                CombatSelectionInputEncodingV2::SubmitScryDiscardIndices,
            ) if index == private_index => {
                let private_identity = card_uuid
                    .map(|uuid| (1_u8, u64::from(uuid)))
                    .unwrap_or((2_u8, *private_index));
                if !public_occurrences.insert(*index)
                    || !private_occurrences.insert(private_identity)
                {
                    return Err(CombatActionResolutionErrorV1::DuplicateSelectionOccurrence(
                        domain_ordinal,
                    ));
                }
                scry_indices.push(
                    usize::try_from(*index).map_err(|_| {
                        CombatActionResolutionErrorV1::SelectionDomainEncodingMismatch
                    })?,
                );
            }
            (
                PublicCombatSelectionDomainCandidateV1::Card {
                    eligible: false, ..
                },
                _,
                _,
            )
            | (
                PublicCombatSelectionDomainCandidateV1::Scry {
                    currently_present: false,
                    ..
                },
                _,
                _,
            ) => {
                return Err(CombatActionResolutionErrorV1::SelectionDomainIneligible(
                    domain_ordinal,
                ));
            }
            _ => {
                return Err(CombatActionResolutionErrorV1::SelectionDomainEncodingMismatch);
            }
        }
    }

    Ok(match public.input_encoding {
        CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids => {
            ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                SelectionScope::Hand,
                card_uuids,
            ))
        }
        CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids => {
            ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                SelectionScope::Grid,
                card_uuids,
            ))
        }
        CombatSelectionInputEncodingV2::SubmitScryDiscardIndices => {
            ClientInput::SubmitScryDiscard(scry_indices)
        }
    })
}

fn project_atomic_action(
    input: &ClientInput,
    combat: &CombatState,
    indexed: Option<&PublicCombatIndexedChoiceSurfaceV1>,
) -> Result<PublicCombatAtomicActionV1, CombatActionProjectionErrorV1> {
    let target_index = |target: Option<crate::EntityId>| {
        target
            .map(|entity_id| {
                combat
                    .entities
                    .monsters
                    .iter()
                    .position(|monster| monster.id == entity_id)
                    .ok_or(CombatActionProjectionErrorV1::MissingMonsterTarget(
                        entity_id,
                    ))
            })
            .transpose()
    };
    Ok(match input {
        ClientInput::PlayCard { card_index, target } => PublicCombatAtomicActionV1::PlayCard {
            hand_index: *card_index,
            target_monster_index: target_index(*target)?,
        },
        ClientInput::UsePotion {
            potion_index,
            target,
        } => PublicCombatAtomicActionV1::UsePotion {
            potion_index: *potion_index,
            target_monster_index: target_index(*target)?,
        },
        ClientInput::DiscardPotion(potion_index) => PublicCombatAtomicActionV1::DiscardPotion {
            potion_index: *potion_index,
        },
        ClientInput::EndTurn => PublicCombatAtomicActionV1::EndTurn,
        ClientInput::SubmitDiscoverChoice(choice_index) => {
            let indexed =
                indexed.ok_or(CombatActionProjectionErrorV1::IndexedChoiceMetadataMissing)?;
            if indexed.candidates.get(*choice_index).is_none() {
                return Err(
                    CombatActionProjectionErrorV1::IndexedChoiceCandidateMissing(*choice_index),
                );
            }
            PublicCombatAtomicActionV1::SubmitIndexedChoice {
                choice_index: *choice_index,
            }
        }
        ClientInput::Proceed => PublicCombatAtomicActionV1::Proceed,
        ClientInput::Cancel => PublicCombatAtomicActionV1::Cancel,
        _ => return Err(CombatActionProjectionErrorV1::UnsupportedAtomicInput),
    })
}

fn project_indexed_choice(
    indexed: &crate::sim::combat_action_surface::CombatIndexedChoiceSurfaceV2,
) -> PublicCombatIndexedChoiceSurfaceV1 {
    PublicCombatIndexedChoiceSurfaceV1 {
        input_encoding: indexed.input_encoding,
        reason: match &indexed.reason {
            CombatIndexedChoiceReasonV2::Discovery {
                colorless,
                card_type,
                amount,
            } => PublicCombatIndexedChoiceReasonV1::Discovery {
                colorless: *colorless,
                card_type: *card_type,
                amount: *amount,
            },
            CombatIndexedChoiceReasonV2::CardReward { destination } => {
                PublicCombatIndexedChoiceReasonV1::CardReward {
                    destination: *destination,
                }
            }
            CombatIndexedChoiceReasonV2::ForeignInfluence { upgraded } => {
                PublicCombatIndexedChoiceReasonV1::ForeignInfluence {
                    upgraded: *upgraded,
                }
            }
            CombatIndexedChoiceReasonV2::ChooseOne => PublicCombatIndexedChoiceReasonV1::ChooseOne,
            CombatIndexedChoiceReasonV2::Stance => PublicCombatIndexedChoiceReasonV1::Stance,
        },
        candidates: indexed
            .candidates
            .iter()
            .map(|candidate| match candidate {
                CombatIndexedChoiceCandidateV2::Card { card_id, upgrades } => {
                    PublicCombatIndexedChoiceCandidateV1::Card {
                        card_id: *card_id,
                        upgrades: *upgrades,
                    }
                }
                CombatIndexedChoiceCandidateV2::Stance { stance } => {
                    PublicCombatIndexedChoiceCandidateV1::Stance { stance: *stance }
                }
            })
            .collect(),
    }
}

fn project_selection_family(
    family: &CombatSelectionActionFamilyV2,
) -> Result<
    (
        PublicCombatSelectionFamilyV1,
        CombatSelectionFamilyResolutionV1,
    ),
    CombatActionProjectionErrorV1,
> {
    let distinct_by = match family.payload_language {
        CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
            CombatSelectionDistinctByV2::CardUuid,
        ) => PublicCombatSelectionDistinctByV1::CardOccurrence,
        CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
            CombatSelectionDistinctByV2::ScryIndexAndCardUuid,
        ) => PublicCombatSelectionDistinctByV1::ScryOccurrence,
    };
    let mut domain = Vec::with_capacity(family.raw_domain.len());
    let mut private_domain = Vec::with_capacity(family.raw_domain.len());
    for candidate in &family.raw_domain {
        match candidate {
            CombatSelectionDomainCandidateV2::CardUuid {
                ordinal,
                uuid,
                card_id,
                upgrades,
                eligible,
            } if family.input_encoding
                != CombatSelectionInputEncodingV2::SubmitScryDiscardIndices =>
            {
                domain.push(PublicCombatSelectionDomainCandidateV1::Card {
                    ordinal: *ordinal,
                    card_id: *card_id,
                    upgrades: *upgrades,
                    eligible: *eligible,
                });
                private_domain.push(CombatSelectionDomainResolutionV1::Card { uuid: *uuid });
            }
            CombatSelectionDomainCandidateV2::ScryIndex {
                index,
                card_id,
                card_uuid,
                currently_present,
            } if family.input_encoding
                == CombatSelectionInputEncodingV2::SubmitScryDiscardIndices =>
            {
                domain.push(PublicCombatSelectionDomainCandidateV1::Scry {
                    index: *index,
                    card_id: *card_id,
                    currently_present: *currently_present,
                });
                private_domain.push(CombatSelectionDomainResolutionV1::Scry {
                    index: *index,
                    card_uuid: *card_uuid,
                });
            }
            _ => return Err(CombatActionProjectionErrorV1::SelectionPayloadMismatch),
        }
    }
    Ok((
        PublicCombatSelectionFamilyV1 {
            input_encoding: family.input_encoding,
            reason: family.reason.clone(),
            source_pile: family.source_pile,
            domain,
            raw_domain_count: family.raw_domain_count,
            eligible_domain_count: family.eligible_domain_count,
            max_distinct_selection_count: family.max_distinct_selection_count,
            declared_min: family.declared_min,
            declared_max: family.declared_max,
            effective_max: family.effective_max,
            distinct_by,
        },
        CombatSelectionFamilyResolutionV1 {
            input_encoding: family.input_encoding,
            distinct_by,
            domain: private_domain,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::{Potion, PotionId};
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{HandSelectReason, PendingChoice};

    #[test]
    fn public_atomic_surface_ignores_private_ids_and_keeps_potions() {
        let mut left = combat_with_target_and_potion(7, 41);
        let mut right = left.clone();
        right.entities.monsters[0].id = 700;
        right.entities.potions[0].as_mut().unwrap().uuid = 4100;

        let left_projection =
            project_public_combat_actions_v1(&EngineState::CombatPlayerTurn, &left).unwrap();
        let right_projection =
            project_public_combat_actions_v1(&EngineState::CombatPlayerTurn, &right).unwrap();

        assert_eq!(left_projection.public, right_projection.public);
        assert_ne!(
            left_projection.private_resolution,
            right_projection.private_resolution
        );
        assert!(left_projection.public.atomic_actions.iter().any(|action| {
            matches!(
                action,
                PublicCombatAtomicActionV1::UsePotion {
                    potion_index: 0,
                    target_monster_index: Some(0)
                }
            )
        }));
        left.entities.monsters[0].id = 7;
    }

    #[test]
    fn symbolic_card_uuid_is_private_resolution_only() {
        let mut combat = combat_with_target_and_potion(7, 41);
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 101),
            CombatCard::new(CardId::Defend, 202),
        ];
        let engine = EngineState::PendingChoice(PendingChoice::HandSelect {
            candidate_uuids: vec![101, 202],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: HandSelectReason::Discard,
        });

        let projection = project_public_combat_actions_v1(&engine, &combat).unwrap();

        assert_eq!(projection.public.selection_families.len(), 1);
        assert_eq!(
            projection.public.selection_families[0].domain[0],
            PublicCombatSelectionDomainCandidateV1::Card {
                ordinal: 0,
                card_id: Some(CardId::Strike),
                upgrades: Some(0),
                eligible: true,
            }
        );
        assert_eq!(
            projection.private_resolution.selection_families[0].domain[0],
            CombatSelectionDomainResolutionV1::Card { uuid: 101 }
        );
        assert_eq!(
            resolve_public_combat_action_v1(
                &projection,
                &PublicCombatActionChoiceV1::Selection {
                    family_ordinal: 0,
                    selected_domain_ordinals: vec![0],
                },
            )
            .unwrap(),
            ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                SelectionScope::Hand,
                [101],
            ))
        );
    }

    #[test]
    fn public_scry_selection_cannot_address_one_private_card_twice() {
        let mut combat = combat_with_target_and_potion(7, 41);
        combat.zones.draw_pile = vec![CombatCard::new(CardId::Strike, 7)].into();
        let projection = project_public_combat_actions_v1(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Strike],
                card_uuids: vec![7, 7],
            }),
            &combat,
        )
        .unwrap();

        assert_eq!(
            resolve_public_combat_action_v1(
                &projection,
                &PublicCombatActionChoiceV1::Selection {
                    family_ordinal: 0,
                    selected_domain_ordinals: vec![0, 1],
                },
            ),
            Err(CombatActionResolutionErrorV1::SelectionCountOutOfRange)
        );
    }

    fn combat_with_target_and_potion(monster_id: usize, potion_uuid: u32) -> CombatState {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 11)];
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = monster_id;
        monster.slot = 0;
        combat.entities.monsters.push(monster);
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, potion_uuid))];
        combat
    }
}
