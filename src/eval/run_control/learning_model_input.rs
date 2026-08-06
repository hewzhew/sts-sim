//! In-process model-facing views over [`LearningBoundaryV1`].
//!
//! This module deliberately does not serialize anything. It removes artifact
//! ids and schema labels from the inference view, keeps variable candidate
//! sets ragged, and decodes symbolic combat selections without eagerly
//! enumerating their combinatorial payloads.

use std::fmt;

use crate::ai::combat_learning_observation::{
    CombatLearningCardZonesV1, CombatLearningEncounterV1, CombatLearningMonsterStateV1,
    CombatLearningPlayerStateV1, CombatLearningPotionV1, CombatLearningTurnV1,
};
use crate::ai::combat_public_observation::HiddenInformationReasonV1;
use crate::ai::planner_core::{
    PlannerAction, PlannerCardObservation, PlannerDecisionContext, PlannerDecisionSite,
    PlannerPotionSlotObservation, PlannerPublicHistory, PlannerPublicMap, PlannerRelicObservation,
    PlannerRunGoal, PlannerRunScalars,
};
use crate::sim::combat_action_surface::{
    CombatIndexedChoiceCandidateV2, CombatLegalActionSurfaceV2, CombatSelectionActionFamilyV2,
    CombatSelectionDistinctByV2, CombatSelectionDomainCandidateV2, CombatSelectionInputEncodingV2,
    CombatSelectionPayloadLanguageV2, CombatSelectionStatusV2,
};
use crate::state::core::ClientInput;
use crate::state::selection::{SelectionResolution, SelectionScope};

use super::{
    LearningActionV1, LearningBoundaryV1, LearningCombatBoundaryV1, LearningStrategicBoundaryV1,
};

/// The semantic strategic state visible to a model.
///
/// Artifact identity, schema labels, and mechanics manifests remain outside
/// this view. They are dataset/runtime compatibility metadata, not features.
#[derive(Clone, Copy, Debug)]
pub struct LearningStrategicModelObservationV1<'a> {
    pub run_goal: PlannerRunGoal,
    pub decision_site: PlannerDecisionSite,
    pub run: &'a PlannerRunScalars,
    pub cards: &'a [PlannerCardObservation],
    pub relics: &'a [PlannerRelicObservation],
    pub potions: &'a [PlannerPotionSlotObservation],
    pub public_map: &'a PlannerPublicMap,
    pub context: &'a PlannerDecisionContext,
    pub public_history: &'a PlannerPublicHistory,
}

/// The semantic combat state visible to a model.
///
/// The learning environment has already established that this observation is
/// public and complete. Schema labels and completeness markers are therefore
/// validation metadata rather than model inputs.
#[derive(Clone, Copy, Debug)]
pub struct LearningCombatModelObservationV1<'a> {
    pub potions: &'a [Option<CombatLearningPotionV1>],
    pub hidden_reasons: &'a [HiddenInformationReasonV1],
    pub encounter: &'a CombatLearningEncounterV1,
    pub turn: &'a CombatLearningTurnV1,
    pub player: &'a CombatLearningPlayerStateV1,
    pub cards: &'a CombatLearningCardZonesV1,
    pub monsters: &'a [CombatLearningMonsterStateV1],
}

#[derive(Clone, Copy, Debug)]
pub enum LearningModelObservationV1<'a> {
    Strategic(LearningStrategicModelObservationV1<'a>),
    Combat(LearningCombatModelObservationV1<'a>),
}

/// Candidate semantics supplied to an encoder.
///
/// The candidate ordinal is the only value returned by a policy. Opaque
/// candidate ids and exact runtime handles stay in the private resolution
/// table owned by [`LearningModelDecisionV1`].
#[derive(Clone, Copy, Debug)]
pub enum LearningModelCandidateSemanticsV1<'a> {
    Strategic {
        action: &'a PlannerAction,
    },
    CombatAtomic {
        input: &'a ClientInput,
        indexed_choice: Option<&'a CombatIndexedChoiceCandidateV2>,
    },
    CombatSelectionFamily {
        family: &'a CombatSelectionActionFamilyV2,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct LearningModelCandidateV1<'a> {
    pub semantics: LearningModelCandidateSemanticsV1<'a>,
    resolution: LearningCandidateResolutionV1<'a>,
}

#[derive(Clone, Copy, Debug)]
enum LearningCandidateResolutionV1<'a> {
    StrategicCandidate {
        candidate_id: &'a str,
    },
    CombatAtomic {
        input: &'a ClientInput,
    },
    CombatSelectionFamily {
        family: &'a CombatSelectionActionFamilyV2,
    },
}

#[derive(Clone, Debug)]
pub struct LearningModelDecisionV1<'a> {
    pub observation: LearningModelObservationV1<'a>,
    pub candidates: Vec<LearningModelCandidateV1<'a>>,
}

impl<'a> LearningModelDecisionV1<'a> {
    pub fn from_boundary(
        boundary: &'a LearningBoundaryV1,
    ) -> Result<Self, LearningModelInputError> {
        match boundary {
            LearningBoundaryV1::Strategic { boundary } => Self::from_strategic(boundary),
            LearningBoundaryV1::Combat { boundary } => Self::from_combat(boundary),
            LearningBoundaryV1::Terminal { .. } => Err(LearningModelInputError::TerminalBoundary),
            LearningBoundaryV1::Unsupported => Err(LearningModelInputError::UnsupportedBoundary),
        }
    }

    pub fn choose(
        &self,
        candidate_ordinal: usize,
    ) -> Result<LearningModelChoiceV1, LearningModelInputError> {
        let candidate = self.candidates.get(candidate_ordinal).ok_or(
            LearningModelInputError::CandidateOrdinalOutOfRange {
                candidate_ordinal,
                candidate_count: self.candidates.len(),
            },
        )?;
        Ok(match candidate.resolution {
            LearningCandidateResolutionV1::StrategicCandidate { candidate_id } => {
                LearningModelChoiceV1::Apply(LearningActionV1::StrategicCandidate {
                    candidate_id: candidate_id.to_string(),
                })
            }
            LearningCandidateResolutionV1::CombatAtomic { input } => {
                LearningModelChoiceV1::Apply(LearningActionV1::CombatInput {
                    input: input.clone(),
                })
            }
            LearningCandidateResolutionV1::CombatSelectionFamily { family } => {
                LearningModelChoiceV1::DecodeSelection(LearningSelectionDraftV1 {
                    family: family.clone(),
                    selected_domain_indices: Vec::new(),
                })
            }
        })
    }

    fn from_strategic(
        boundary: &'a LearningStrategicBoundaryV1,
    ) -> Result<Self, LearningModelInputError> {
        if !boundary.legal_candidates.completeness.is_complete() {
            return Err(LearningModelInputError::IncompleteStrategicCandidateSet);
        }
        if boundary.legal_candidates.observation_id != boundary.observation.observation_id {
            return Err(LearningModelInputError::StrategicObservationIdentityMismatch);
        }
        if boundary.legal_candidates.site != boundary.observation.decision_site {
            return Err(LearningModelInputError::StrategicDecisionSiteMismatch);
        }
        if boundary
            .legal_candidates
            .candidates
            .iter()
            .any(|candidate| candidate.mechanics != boundary.observation.mechanics)
        {
            return Err(LearningModelInputError::StrategicMechanicsMismatch);
        }

        let observation = &boundary.observation;
        let candidates = boundary
            .legal_candidates
            .candidates
            .iter()
            .map(|candidate| LearningModelCandidateV1 {
                semantics: LearningModelCandidateSemanticsV1::Strategic {
                    action: &candidate.action,
                },
                resolution: LearningCandidateResolutionV1::StrategicCandidate {
                    candidate_id: &candidate.candidate_id,
                },
            })
            .collect::<Vec<_>>();
        ensure_nonempty(candidates.len())?;

        Ok(Self {
            observation: LearningModelObservationV1::Strategic(
                LearningStrategicModelObservationV1 {
                    run_goal: observation.run_goal,
                    decision_site: observation.decision_site,
                    run: &observation.run,
                    cards: &observation.cards,
                    relics: &observation.relics,
                    potions: &observation.potions,
                    public_map: &observation.public_map,
                    context: &observation.context,
                    public_history: &observation.public_history,
                },
            ),
            candidates,
        })
    }

    fn from_combat(
        boundary: &'a LearningCombatBoundaryV1,
    ) -> Result<Self, LearningModelInputError> {
        validate_indexed_choice_alignment(&boundary.legal_actions)?;

        let observation = &boundary.observation;
        let mut candidates = Vec::with_capacity(
            boundary.legal_actions.atomic_actions.len()
                + boundary.legal_actions.selection_families.len(),
        );
        for input in &boundary.legal_actions.atomic_actions {
            candidates.push(LearningModelCandidateV1 {
                semantics: LearningModelCandidateSemanticsV1::CombatAtomic {
                    input,
                    indexed_choice: indexed_choice_semantics(&boundary.legal_actions, input),
                },
                resolution: LearningCandidateResolutionV1::CombatAtomic { input },
            });
        }
        for family in &boundary.legal_actions.selection_families {
            if family.selection_status == CombatSelectionStatusV2::Enabled {
                candidates.push(LearningModelCandidateV1 {
                    semantics: LearningModelCandidateSemanticsV1::CombatSelectionFamily { family },
                    resolution: LearningCandidateResolutionV1::CombatSelectionFamily { family },
                });
            }
        }
        ensure_nonempty(candidates.len())?;

        Ok(Self {
            observation: LearningModelObservationV1::Combat(LearningCombatModelObservationV1 {
                potions: &observation.potions,
                hidden_reasons: &observation.hidden_reasons,
                encounter: &observation.encounter,
                turn: &observation.turn,
                player: &observation.player,
                cards: &observation.cards,
                monsters: &observation.monsters,
            }),
            candidates,
        })
    }
}

/// A ragged batch of decisions.
///
/// `candidate_row_splits` is the zero-copy/default action-mask contract:
/// candidates in each half-open row range are legal and no padding exists.
/// Backends that require rectangular tensors may request [`dense_action_mask`].
#[derive(Clone, Debug)]
pub struct LearningModelBatchV1<'a> {
    pub decisions: Vec<LearningModelDecisionV1<'a>>,
    pub candidate_row_splits: Vec<usize>,
}

impl<'a> LearningModelBatchV1<'a> {
    pub fn from_boundaries(
        boundaries: &'a [LearningBoundaryV1],
    ) -> Result<Self, LearningModelInputError> {
        let mut decisions = Vec::with_capacity(boundaries.len());
        let mut candidate_row_splits = Vec::with_capacity(boundaries.len() + 1);
        candidate_row_splits.push(0);
        for boundary in boundaries {
            let decision = LearningModelDecisionV1::from_boundary(boundary)?;
            let next = candidate_row_splits
                .last()
                .copied()
                .unwrap_or(0usize)
                .checked_add(decision.candidates.len())
                .ok_or(LearningModelInputError::CandidateCountOverflow)?;
            decisions.push(decision);
            candidate_row_splits.push(next);
        }
        Ok(Self {
            decisions,
            candidate_row_splits,
        })
    }

    pub fn flattened_candidate_count(&self) -> usize {
        self.candidate_row_splits.last().copied().unwrap_or(0)
    }

    pub fn dense_action_mask(&self) -> LearningDenseActionMaskV1 {
        let width = self
            .decisions
            .iter()
            .map(|decision| decision.candidates.len())
            .max()
            .unwrap_or(0);
        let mut values = vec![false; self.decisions.len().saturating_mul(width)];
        for (row, decision) in self.decisions.iter().enumerate() {
            let start = row * width;
            values[start..start + decision.candidates.len()].fill(true);
        }
        LearningDenseActionMaskV1 {
            rows: self.decisions.len(),
            width,
            values,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LearningDenseActionMaskV1 {
    pub rows: usize,
    pub width: usize,
    /// Row-major mask. Real ragged candidates are true; padding is false.
    pub values: Vec<bool>,
}

#[derive(Clone, Debug)]
pub enum LearningModelChoiceV1 {
    Apply(LearningActionV1),
    DecodeSelection(LearningSelectionDraftV1),
}

/// An uncommitted symbolic selection.
///
/// Appending to this draft does not step the simulator. Only selecting the
/// explicit submit candidate produces a [`LearningActionV1`].
#[derive(Clone, Debug)]
pub struct LearningSelectionDraftV1 {
    family: CombatSelectionActionFamilyV2,
    selected_domain_indices: Vec<usize>,
}

impl LearningSelectionDraftV1 {
    pub fn selected_domain_indices(&self) -> &[usize] {
        &self.selected_domain_indices
    }

    pub fn decision(&self) -> LearningSelectionDecisionV1<'_> {
        let mut candidates = Vec::new();
        if self.can_submit() {
            candidates.push(LearningSelectionCandidateV1 {
                semantics: LearningSelectionCandidateSemanticsV1::Submit,
                resolution: LearningSelectionCandidateResolutionV1::Submit,
            });
        }
        if self.selected_domain_indices.len() < u64_to_usize(self.family.effective_max) {
            for (domain_index, domain) in self.family.raw_domain.iter().enumerate() {
                if self.can_append(domain_index) {
                    candidates.push(LearningSelectionCandidateV1 {
                        semantics: LearningSelectionCandidateSemanticsV1::Append { domain },
                        resolution: LearningSelectionCandidateResolutionV1::Append { domain_index },
                    });
                }
            }
        }
        LearningSelectionDecisionV1 { candidates }
    }

    pub fn choose(
        &mut self,
        candidate_ordinal: usize,
    ) -> Result<LearningSelectionStepV1, LearningModelInputError> {
        let decision = self.decision();
        let candidate = decision.candidates.get(candidate_ordinal).ok_or(
            LearningModelInputError::SelectionCandidateOrdinalOutOfRange {
                candidate_ordinal,
                candidate_count: decision.candidates.len(),
            },
        )?;
        match candidate.resolution {
            LearningSelectionCandidateResolutionV1::Append { domain_index } => {
                self.selected_domain_indices.push(domain_index);
                Ok(LearningSelectionStepV1::Continue)
            }
            LearningSelectionCandidateResolutionV1::Submit => {
                Ok(LearningSelectionStepV1::Apply(self.to_learning_action()?))
            }
        }
    }

    fn can_submit(&self) -> bool {
        self.family.selection_status == CombatSelectionStatusV2::Enabled
            && self.selected_domain_indices.len() >= u64_to_usize(self.family.declared_min)
            && self.selected_domain_indices.len() <= u64_to_usize(self.family.effective_max)
    }

    fn can_append(&self, domain_index: usize) -> bool {
        if self.family.selection_status != CombatSelectionStatusV2::Enabled {
            return false;
        }
        let Some(candidate) = self.family.raw_domain.get(domain_index) else {
            return false;
        };
        if !domain_candidate_is_eligible(candidate) {
            return false;
        }
        !self
            .selected_domain_indices
            .iter()
            .copied()
            .any(|selected| same_selection_identity(&self.family, selected, domain_index))
    }

    fn to_learning_action(&self) -> Result<LearningActionV1, LearningModelInputError> {
        if !self.can_submit() {
            return Err(LearningModelInputError::SelectionCannotSubmit);
        }
        let input = match self.family.input_encoding {
            CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids => {
                ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                    SelectionScope::Hand,
                    self.selected_card_uuids()?,
                ))
            }
            CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids => {
                ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                    SelectionScope::Grid,
                    self.selected_card_uuids()?,
                ))
            }
            CombatSelectionInputEncodingV2::SubmitScryDiscardIndices => {
                ClientInput::SubmitScryDiscard(self.selected_scry_indices()?)
            }
        };
        Ok(LearningActionV1::CombatInput { input })
    }

    fn selected_card_uuids(&self) -> Result<Vec<u32>, LearningModelInputError> {
        self.selected_domain_indices
            .iter()
            .map(|index| match self.family.raw_domain.get(*index) {
                Some(CombatSelectionDomainCandidateV2::CardUuid { uuid, .. }) => Ok(*uuid),
                _ => Err(LearningModelInputError::SelectionDomainEncodingMismatch),
            })
            .collect()
    }

    fn selected_scry_indices(&self) -> Result<Vec<usize>, LearningModelInputError> {
        self.selected_domain_indices
            .iter()
            .map(|index| match self.family.raw_domain.get(*index) {
                Some(CombatSelectionDomainCandidateV2::ScryIndex { index, .. }) => {
                    usize::try_from(*index)
                        .map_err(|_| LearningModelInputError::SelectionIndexOverflow)
                }
                _ => Err(LearningModelInputError::SelectionDomainEncodingMismatch),
            })
            .collect()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum LearningSelectionCandidateSemanticsV1<'a> {
    Submit,
    Append {
        domain: &'a CombatSelectionDomainCandidateV2,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct LearningSelectionCandidateV1<'a> {
    pub semantics: LearningSelectionCandidateSemanticsV1<'a>,
    resolution: LearningSelectionCandidateResolutionV1,
}

#[derive(Clone, Copy, Debug)]
enum LearningSelectionCandidateResolutionV1 {
    Submit,
    Append { domain_index: usize },
}

#[derive(Clone, Debug)]
pub struct LearningSelectionDecisionV1<'a> {
    pub candidates: Vec<LearningSelectionCandidateV1<'a>>,
}

#[derive(Clone, Debug)]
pub enum LearningSelectionStepV1 {
    Continue,
    Apply(LearningActionV1),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearningModelInputError {
    TerminalBoundary,
    UnsupportedBoundary,
    IncompleteStrategicCandidateSet,
    StrategicObservationIdentityMismatch,
    StrategicDecisionSiteMismatch,
    StrategicMechanicsMismatch,
    IndexedChoiceMetadataMissing,
    IndexedChoiceCandidateMissing {
        index: usize,
    },
    IndexedChoiceAtomicActionMissing {
        index: usize,
    },
    NoLegalCandidates,
    CandidateCountOverflow,
    CandidateOrdinalOutOfRange {
        candidate_ordinal: usize,
        candidate_count: usize,
    },
    SelectionCandidateOrdinalOutOfRange {
        candidate_ordinal: usize,
        candidate_count: usize,
    },
    SelectionCannotSubmit,
    SelectionDomainEncodingMismatch,
    SelectionIndexOverflow,
}

impl fmt::Display for LearningModelInputError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for LearningModelInputError {}

fn ensure_nonempty(candidate_count: usize) -> Result<(), LearningModelInputError> {
    if candidate_count == 0 {
        Err(LearningModelInputError::NoLegalCandidates)
    } else {
        Ok(())
    }
}

fn validate_indexed_choice_alignment(
    surface: &CombatLegalActionSurfaceV2,
) -> Result<(), LearningModelInputError> {
    for input in &surface.atomic_actions {
        if let ClientInput::SubmitDiscoverChoice(index) = input {
            let indexed = surface
                .indexed_choice
                .as_ref()
                .ok_or(LearningModelInputError::IndexedChoiceMetadataMissing)?;
            if indexed.candidates.get(*index).is_none() {
                return Err(LearningModelInputError::IndexedChoiceCandidateMissing {
                    index: *index,
                });
            }
        }
    }
    if let Some(indexed) = &surface.indexed_choice {
        for index in 0..indexed.candidates.len() {
            if !surface
                .atomic_actions
                .contains(&ClientInput::SubmitDiscoverChoice(index))
            {
                return Err(LearningModelInputError::IndexedChoiceAtomicActionMissing { index });
            }
        }
    }
    Ok(())
}

fn indexed_choice_semantics<'a>(
    surface: &'a CombatLegalActionSurfaceV2,
    input: &ClientInput,
) -> Option<&'a CombatIndexedChoiceCandidateV2> {
    let ClientInput::SubmitDiscoverChoice(index) = input else {
        return None;
    };
    surface
        .indexed_choice
        .as_ref()
        .and_then(|choice| choice.candidates.get(*index))
}

fn domain_candidate_is_eligible(candidate: &CombatSelectionDomainCandidateV2) -> bool {
    match candidate {
        CombatSelectionDomainCandidateV2::CardUuid { eligible, .. } => *eligible,
        CombatSelectionDomainCandidateV2::ScryIndex {
            currently_present, ..
        } => *currently_present,
    }
}

fn same_selection_identity(
    family: &CombatSelectionActionFamilyV2,
    left_index: usize,
    right_index: usize,
) -> bool {
    let (Some(left), Some(right)) = (
        family.raw_domain.get(left_index),
        family.raw_domain.get(right_index),
    ) else {
        return false;
    };
    match family.payload_language {
        CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
            CombatSelectionDistinctByV2::CardUuid,
        ) => match (left, right) {
            (
                CombatSelectionDomainCandidateV2::CardUuid { uuid: left, .. },
                CombatSelectionDomainCandidateV2::CardUuid { uuid: right, .. },
            ) => left == right,
            _ => false,
        },
        CombatSelectionPayloadLanguageV2::OrderedDistinctSequence(
            CombatSelectionDistinctByV2::ScryIndexAndCardUuid,
        ) => match (left, right) {
            (
                CombatSelectionDomainCandidateV2::ScryIndex {
                    index: left_index,
                    card_uuid: left_uuid,
                    ..
                },
                CombatSelectionDomainCandidateV2::ScryIndex {
                    index: right_index,
                    card_uuid: right_uuid,
                    ..
                },
            ) => left_index == right_index || left_uuid == right_uuid,
            _ => false,
        },
    }
}

fn u64_to_usize(value: u64) -> usize {
    usize::try_from(value).unwrap_or(usize::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::sim::combat_action_surface::combat_legal_action_surface_v2;
    use crate::state::core::{
        ActiveCombat, CombatContext, EngineState, HandSelectReason, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;
    use crate::state::PendingChoice;

    use super::super::{LearningEnvV1, RunControlConfig, RunControlSession};

    #[test]
    fn strategic_model_view_resolves_an_ordinal_without_exposing_candidate_ids() {
        let env = LearningEnvV1::new(RunControlConfig::default());
        let boundary = env.observe().expect("observe initial boundary");
        let decision =
            LearningModelDecisionV1::from_boundary(&boundary).expect("build model decision");

        assert!(matches!(
            decision.observation,
            LearningModelObservationV1::Strategic(_)
        ));
        assert!(decision.candidates.iter().all(|candidate| matches!(
            candidate.semantics,
            LearningModelCandidateSemanticsV1::Strategic { .. }
        )));
        assert!(matches!(
            decision.choose(0).expect("resolve first ordinal"),
            LearningModelChoiceV1::Apply(LearningActionV1::StrategicCandidate { .. })
        ));
    }

    #[test]
    fn ragged_batch_owns_row_splits_and_builds_padding_only_on_request() {
        let first_env = LearningEnvV1::new(RunControlConfig::default());
        let first = first_env.observe().expect("first strategic boundary");
        let mut second_session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Bash, 51)];
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 7;
        combat.entities.monsters.push(monster);
        second_session.engine_state = EngineState::CombatPlayerTurn;
        second_session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let second = LearningEnvV1::from_session(second_session)
            .observe()
            .expect("combat boundary");
        let boundaries = vec![first, second];
        let batch = LearningModelBatchV1::from_boundaries(&boundaries).expect("build ragged batch");

        assert_eq!(batch.candidate_row_splits.len(), 3);
        assert_eq!(
            batch.flattened_candidate_count(),
            batch
                .decisions
                .iter()
                .map(|row| row.candidates.len())
                .sum::<usize>()
        );
        let mask = batch.dense_action_mask();
        assert_eq!(mask.rows, 2);
        for (row, decision) in batch.decisions.iter().enumerate() {
            for column in 0..mask.width {
                assert_eq!(
                    mask.values[row * mask.width + column],
                    column < decision.candidates.len()
                );
            }
        }
    }

    #[test]
    fn indexed_atomic_choice_carries_typed_semantics() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let choice = PendingChoice::DiscoverySelect(crate::state::DiscoveryChoiceState {
            cards: vec![CardId::Bash, CardId::FiendFire],
            colorless: false,
            card_type: None,
            amount: 1,
            can_skip: true,
        });
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let boundary = LearningEnvV1::from_session(session)
            .observe()
            .expect("indexed boundary");
        let decision =
            LearningModelDecisionV1::from_boundary(&boundary).expect("build model decision");

        assert!(decision.candidates.iter().any(|candidate| matches!(
            candidate.semantics,
            LearningModelCandidateSemanticsV1::CombatAtomic {
                input: ClientInput::SubmitDiscoverChoice(1),
                indexed_choice: Some(CombatIndexedChoiceCandidateV2::Card {
                    card_id: CardId::FiendFire,
                    upgrades: 0,
                }),
            }
        )));
    }

    #[test]
    fn symbolic_scry_decodes_linearly_and_submits_only_after_explicit_stop() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.draw_pile = ((1..=64)
            .map(|uuid| CombatCard::new(CardId::Strike, uuid))
            .collect::<Vec<_>>())
        .into();
        let surface = combat_legal_action_surface_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike; 64],
                card_uuids: (1..=64).collect(),
            }),
            &combat,
        );
        let family = surface.selection_families[0].clone();
        let mut draft = LearningSelectionDraftV1 {
            family,
            selected_domain_indices: Vec::new(),
        };

        let first = draft.decision();
        assert_eq!(first.candidates.len(), 65);
        assert!(matches!(
            first.candidates[0].semantics,
            LearningSelectionCandidateSemanticsV1::Submit
        ));
        assert!(matches!(
            draft.choose(1).expect("append first card"),
            LearningSelectionStepV1::Continue
        ));
        assert_eq!(draft.selected_domain_indices(), &[0]);
        assert_eq!(draft.decision().candidates.len(), 64);
        assert_eq!(
            match draft.choose(0).expect("submit prefix") {
                LearningSelectionStepV1::Apply(action) => action,
                LearningSelectionStepV1::Continue => panic!("submit must produce an action"),
            },
            LearningActionV1::CombatInput {
                input: ClientInput::SubmitScryDiscard(vec![0]),
            }
        );
    }

    #[test]
    fn duplicate_scry_uuid_is_removed_from_the_decoder_after_one_address_is_chosen() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.draw_pile = (vec![CombatCard::new(CardId::Strike, 7)]).into();
        let surface = combat_legal_action_surface_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Strike],
                card_uuids: vec![7, 7],
            }),
            &combat,
        );
        let mut draft = LearningSelectionDraftV1 {
            family: surface.selection_families[0].clone(),
            selected_domain_indices: Vec::new(),
        };

        assert_eq!(draft.decision().candidates.len(), 3);
        assert!(matches!(
            draft.choose(1).expect("choose first address"),
            LearningSelectionStepV1::Continue
        ));
        assert_eq!(draft.decision().candidates.len(), 1);
        assert!(matches!(
            draft.decision().candidates[0].semantics,
            LearningSelectionCandidateSemanticsV1::Submit
        ));
    }

    #[test]
    fn symbolic_hand_selection_resolves_from_root_ordinal_to_one_exact_input() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::Defend, 12),
        ];
        let choice = PendingChoice::HandSelect {
            candidate_uuids: vec![11, 12],
            min_cards: 1,
            max_cards: 2,
            can_cancel: false,
            reason: HandSelectReason::Discard,
        };
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let boundary = LearningEnvV1::from_session(session)
            .observe()
            .expect("hand-selection boundary");
        let decision =
            LearningModelDecisionV1::from_boundary(&boundary).expect("build root decision");
        assert_eq!(decision.candidates.len(), 1);
        let LearningModelChoiceV1::DecodeSelection(mut draft) =
            decision.choose(0).expect("choose symbolic family")
        else {
            panic!("symbolic family must start a draft");
        };

        assert_eq!(draft.decision().candidates.len(), 2);
        assert!(matches!(
            draft.choose(0).expect("append first hand card"),
            LearningSelectionStepV1::Continue
        ));
        assert_eq!(
            match draft.choose(0).expect("submit selected card") {
                LearningSelectionStepV1::Apply(action) => action,
                LearningSelectionStepV1::Continue => panic!("submit must produce an action"),
            },
            LearningActionV1::CombatInput {
                input: ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                    SelectionScope::Hand,
                    [11],
                )),
            }
        );
    }

    #[test]
    fn incomplete_strategic_surface_is_rejected_before_inference() {
        let env = LearningEnvV1::new(RunControlConfig::default());
        let LearningBoundaryV1::Strategic { mut boundary } =
            env.observe().expect("initial strategic boundary")
        else {
            panic!("expected strategic boundary");
        };
        boundary.legal_candidates.completeness =
            crate::ai::planner_core::CandidateSetCompleteness::Incomplete {
                basis: crate::ai::planner_core::CandidateCompletenessBasis::RunControlBoundaryEnumerator,
                gaps: vec![
                    crate::ai::planner_core::CandidateRepresentationGap::UnsupportedBoundaryAction,
                ],
            };

        assert_eq!(
            LearningModelDecisionV1::from_boundary(&LearningBoundaryV1::Strategic { boundary })
                .expect_err("incomplete surface must not reach inference"),
            LearningModelInputError::IncompleteStrategicCandidateSet
        );
    }

    #[test]
    fn malformed_indexed_surface_is_rejected_before_inference() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let LearningBoundaryV1::Combat { mut boundary } = LearningEnvV1::from_session(session)
            .observe()
            .expect("combat boundary")
        else {
            panic!("expected combat boundary");
        };
        boundary.legal_actions.atomic_actions = vec![ClientInput::SubmitDiscoverChoice(0)];

        assert_eq!(
            LearningModelDecisionV1::from_boundary(&LearningBoundaryV1::Combat { boundary })
                .expect_err("missing typed indexed metadata must fail"),
            LearningModelInputError::IndexedChoiceMetadataMissing
        );
    }

    #[test]
    fn disabled_symbolic_family_is_not_a_model_candidate() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.draw_pile = (vec![CombatCard::new(CardId::Strike, 7)]).into();
        let surface = combat_legal_action_surface_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike, CardId::Defend],
                card_uuids: vec![7],
            }),
            &combat,
        );
        assert_eq!(
            surface.selection_families[0].selection_status,
            CombatSelectionStatusV2::Disabled(
                crate::sim::combat_action_surface::CombatSelectionDisabledReasonV2::MalformedScryDomain
            )
        );
        let boundary = LearningBoundaryV1::Combat {
            boundary: LearningCombatBoundaryV1 {
                observation: crate::ai::combat_learning_observation::combat_learning_observation_v1(
                    &combat,
                ),
                observation_completeness: super::super::LearningObservationCompletenessV1::Complete,
                legal_actions: surface,
            },
        };
        assert_eq!(
            LearningModelDecisionV1::from_boundary(&boundary)
                .expect_err("disabled-only action surface must not reach a model"),
            LearningModelInputError::NoLegalCandidates
        );
    }
}
