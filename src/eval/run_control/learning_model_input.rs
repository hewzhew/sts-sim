//! In-process model-facing views over [`LearningBoundaryV1`].
//!
//! This module deliberately does not serialize anything. It removes artifact
//! ids and schema labels from the inference view, keeps variable candidate
//! sets ragged, and decodes symbolic combat or run selections without eagerly
//! enumerating their combinatorial payloads.

use std::fmt;

use crate::ai::combat_learning_observation::{
    CombatLearningCardZonesV1, CombatLearningEncounterV1, CombatLearningMonsterStateV1,
    CombatLearningPlayerStateV1, CombatLearningPotionV1, CombatLearningTurnV1,
};
use crate::ai::combat_public_observation::HiddenInformationReasonV1;
use crate::ai::planner_core::{
    CandidateRepresentationGap, CandidateSetCompleteness, PlannerAction, PlannerCardObservation,
    PlannerDecisionContext, PlannerDecisionSite, PlannerPotionSlotObservation,
    PlannerPublicHistory, PlannerPublicMap, PlannerRelicObservation, PlannerRunGoal,
    PlannerRunScalars,
};
use crate::sim::combat_action_surface::{
    CombatIndexedChoiceCandidateV2, CombatIndexedChoiceInputEncodingV2,
    CombatIndexedChoiceReasonV2, CombatLegalActionSurfaceV2, CombatSelectionActionFamilyV2,
    CombatSelectionDistinctByV2, CombatSelectionDomainCandidateV2, CombatSelectionInputEncodingV2,
    CombatSelectionPayloadLanguageV2, CombatSelectionReasonV2, CombatSelectionStatusV2,
};
use crate::state::core::{ClientInput, PileType};
use crate::state::selection::{SelectionReason, SelectionResolution, SelectionScope};

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
    potions: &'a [PlannerPotionSlotObservation],
    pub public_map: &'a PlannerPublicMap,
    pub context: &'a PlannerDecisionContext,
    pub public_history: &'a PlannerPublicHistory,
}

impl<'a> LearningStrategicModelObservationV1<'a> {
    pub fn potion_slots(
        &self,
    ) -> impl ExactSizeIterator<Item = LearningStrategicPotionSlotV1<'a>> + '_ {
        self.potions
            .iter()
            .map(|slot| LearningStrategicPotionSlotV1 { slot })
    }
}

#[derive(Clone, Copy)]
pub struct LearningStrategicPotionSlotV1<'a> {
    slot: &'a PlannerPotionSlotObservation,
}

impl<'a> LearningStrategicPotionSlotV1<'a> {
    pub fn slot(&self) -> usize {
        self.slot.slot
    }

    pub fn potion(&self) -> Option<LearningStrategicPotionV1<'a>> {
        self.slot
            .potion
            .as_ref()
            .map(|potion| LearningStrategicPotionV1 { potion })
    }
}

#[derive(Clone, Copy)]
pub struct LearningStrategicPotionV1<'a> {
    potion: &'a crate::ai::planner_core::PlannerPotionObservation,
}

impl LearningStrategicPotionV1<'_> {
    pub fn potion(&self) -> crate::content::potions::PotionId {
        self.potion.potion
    }

    pub fn relation_key(&self) -> u32 {
        self.potion.potion_uuid
    }

    pub fn can_use(&self) -> bool {
        self.potion.can_use
    }

    pub fn can_discard(&self) -> bool {
        self.potion.can_discard
    }

    pub fn requires_target(&self) -> bool {
        self.potion.requires_target
    }
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
    pub monsters: LearningCombatMonstersV1<'a>,
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
        action: LearningCombatAtomicActionV1<'a>,
    },
    CombatSelectionFamily {
        family: LearningCombatSelectionFamilyV1<'a>,
    },
}

#[derive(Clone, Copy, Debug)]
pub struct LearningCombatIndexedChoiceV1<'a> {
    pub input_encoding: CombatIndexedChoiceInputEncodingV2,
    pub reason: &'a CombatIndexedChoiceReasonV2,
    pub candidate: &'a CombatIndexedChoiceCandidateV2,
}

#[derive(Clone, Copy, Debug)]
pub enum LearningCombatAtomicActionV1<'a> {
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
        indexed: LearningCombatIndexedChoiceV1<'a>,
    },
    Proceed,
    Cancel,
}

#[derive(Clone, Copy)]
pub struct LearningCombatMonstersV1<'a> {
    monsters: &'a [CombatLearningMonsterStateV1],
}

impl LearningCombatMonstersV1<'_> {
    pub fn len(&self) -> usize {
        self.monsters.len()
    }

    pub fn is_empty(&self) -> bool {
        self.monsters.is_empty()
    }

    pub fn get(&self, index: usize) -> Option<LearningCombatMonsterV1<'_>> {
        self.monsters
            .get(index)
            .map(|monster| LearningCombatMonsterV1 { monster })
    }
}

impl fmt::Debug for LearningCombatMonstersV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LearningCombatMonstersV1")
            .field("len", &self.len())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct LearningCombatMonsterV1<'a> {
    monster: &'a CombatLearningMonsterStateV1,
}

impl LearningCombatMonsterV1<'_> {
    pub fn slot(&self) -> u8 {
        self.monster.slot
    }

    pub fn enemy(&self) -> crate::ai::combat_learning_observation::CombatLearningEnemyIdentityV1 {
        self.monster.enemy
    }

    pub fn hp(&self) -> i32 {
        self.monster.hp
    }

    pub fn max_hp(&self) -> i32 {
        self.monster.max_hp
    }

    pub fn block(&self) -> i32 {
        self.monster.block
    }

    pub fn alive(&self) -> bool {
        self.monster.alive
    }

    pub fn escaped(&self) -> bool {
        self.monster.escaped
    }

    pub fn dying(&self) -> bool {
        self.monster.dying
    }

    pub fn half_dead(&self) -> bool {
        self.monster.half_dead
    }

    pub fn intent(&self) -> &crate::ai::combat_learning_observation::CombatLearningIntentV1 {
        &self.monster.intent
    }

    pub fn executed_moves(
        &self,
    ) -> &crate::ai::combat_learning_observation::CombatLearningMonsterMoveHistoryV1 {
        &self.monster.executed_moves
    }

    pub fn public_counters(
        &self,
    ) -> &[crate::ai::combat_learning_observation::CombatLearningMonsterPublicCounterV1] {
        &self.monster.public_counters
    }

    pub fn powers(&self) -> &[crate::ai::combat_learning_observation::CombatLearningPowerV1] {
        &self.monster.powers
    }
}

impl fmt::Debug for LearningCombatMonsterV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LearningCombatMonsterV1")
            .field("slot", &self.slot())
            .field("enemy", &self.enemy())
            .field("hp", &self.hp())
            .field("max_hp", &self.max_hp())
            .field("block", &self.block())
            .field("alive", &self.alive())
            .field("escaped", &self.escaped())
            .field("dying", &self.dying())
            .field("half_dead", &self.half_dead())
            .field("intent", &self.intent())
            .field("executed_moves", &self.executed_moves())
            .field("public_counters", &self.public_counters())
            .field("powers", &self.powers())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct LearningCombatSelectionFamilyV1<'a> {
    family: &'a CombatSelectionActionFamilyV2,
}

impl fmt::Debug for LearningCombatSelectionFamilyV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LearningCombatSelectionFamilyV1")
            .field("input_encoding", &self.input_encoding())
            .field("reason", &self.reason())
            .field("source_pile", &self.source_pile())
            .field("raw_domain_count", &self.raw_domain_count())
            .field("eligible_domain_count", &self.eligible_domain_count())
            .field("declared_min", &self.declared_min())
            .field("declared_max", &self.declared_max())
            .field("effective_max", &self.effective_max())
            .field("payload_language", &self.payload_language())
            .finish_non_exhaustive()
    }
}

impl<'a> LearningCombatSelectionFamilyV1<'a> {
    pub fn input_encoding(&self) -> CombatSelectionInputEncodingV2 {
        self.family.input_encoding
    }

    pub fn reason(&self) -> &CombatSelectionReasonV2 {
        &self.family.reason
    }

    pub fn source_pile(&self) -> Option<PileType> {
        self.family.source_pile
    }

    pub fn raw_domain_count(&self) -> u64 {
        self.family.raw_domain_count
    }

    pub fn eligible_domain_count(&self) -> u64 {
        self.family.eligible_domain_count
    }

    pub fn max_distinct_selection_count(&self) -> u64 {
        self.family.max_distinct_selection_count
    }

    pub fn declared_min(&self) -> u64 {
        self.family.declared_min
    }

    pub fn declared_max(&self) -> u64 {
        self.family.declared_max
    }

    pub fn effective_max(&self) -> u64 {
        self.family.effective_max
    }

    pub fn payload_language(&self) -> CombatSelectionPayloadLanguageV2 {
        self.family.payload_language
    }

    pub fn domain_count(&self) -> usize {
        self.family.raw_domain.len()
    }

    pub fn domain(&self, index: usize) -> Option<LearningCombatSelectionDomainV1<'a>> {
        self.family
            .raw_domain
            .get(index)
            .map(|domain| LearningCombatSelectionDomainV1 { domain })
    }
}

#[derive(Clone, Copy)]
pub struct LearningCombatSelectionDomainV1<'a> {
    domain: &'a CombatSelectionDomainCandidateV2,
}

impl fmt::Debug for LearningCombatSelectionDomainV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_tuple("LearningCombatSelectionDomainV1")
            .field(&self.semantics())
            .finish()
    }
}

impl LearningCombatSelectionDomainV1<'_> {
    pub fn semantics(&self) -> LearningCombatSelectionDomainSemanticsV1 {
        match self.domain {
            CombatSelectionDomainCandidateV2::CardUuid {
                ordinal,
                card_id,
                upgrades,
                eligible,
                ..
            } => LearningCombatSelectionDomainSemanticsV1::Card {
                ordinal: *ordinal,
                card_id: *card_id,
                upgrades: *upgrades,
                eligible: *eligible,
            },
            CombatSelectionDomainCandidateV2::ScryIndex {
                index,
                card_id,
                currently_present,
                ..
            } => LearningCombatSelectionDomainSemanticsV1::Scry {
                index: *index,
                card_id: *card_id,
                currently_present: *currently_present,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LearningCombatSelectionDomainSemanticsV1 {
    Card {
        ordinal: u64,
        card_id: Option<crate::content::cards::CardId>,
        upgrades: Option<u8>,
        eligible: bool,
    },
    Scry {
        index: u64,
        card_id: Option<crate::content::cards::CardId>,
        currently_present: bool,
    },
}

#[derive(Clone, Copy)]
pub struct LearningModelCandidateV1<'a> {
    pub semantics: LearningModelCandidateSemanticsV1<'a>,
    resolution: LearningCandidateResolutionV1<'a>,
}

impl fmt::Debug for LearningModelCandidateV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LearningModelCandidateV1")
            .field("semantics", &self.semantics)
            .finish()
    }
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
    RunSelectionFamily {
        planner_candidate_id: &'a str,
        scope: SelectionScope,
        reason: SelectionReason,
        min_choices: usize,
        max_choices: usize,
        selectable_card_uuids: &'a [u32],
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
                LearningModelChoiceV1::DecodeSelection(LearningSelectionDraftV1::from_combat(
                    family.clone(),
                ))
            }
            LearningCandidateResolutionV1::RunSelectionFamily {
                planner_candidate_id,
                scope,
                reason,
                min_choices,
                max_choices,
                selectable_card_uuids,
            } => LearningModelChoiceV1::DecodeSelection(LearningSelectionDraftV1::from_run(
                planner_candidate_id.to_string(),
                scope,
                reason,
                min_choices,
                max_choices,
                selectable_card_uuids.to_vec(),
            )),
        })
    }

    pub fn from_combat_boundary(
        boundary: &'a LearningCombatBoundaryV1,
    ) -> Result<Self, LearningModelInputError> {
        Self::from_combat(boundary)
    }

    fn from_strategic(
        boundary: &'a LearningStrategicBoundaryV1,
    ) -> Result<Self, LearningModelInputError> {
        if let CandidateSetCompleteness::Incomplete { gaps, .. } =
            &boundary.legal_candidates.completeness
        {
            return Err(LearningModelInputError::IncompleteStrategicCandidateSet {
                site: boundary.observation.decision_site,
                gaps: gaps.clone(),
            });
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
            .map(|candidate| {
                let resolution = match &candidate.action {
                    PlannerAction::BeginRunCardSelection {
                        scope,
                        reason,
                        min_choices,
                        max_choices,
                        selectable_card_uuids,
                    } => LearningCandidateResolutionV1::RunSelectionFamily {
                        planner_candidate_id: &candidate.candidate_id,
                        scope: *scope,
                        reason: *reason,
                        min_choices: *min_choices,
                        max_choices: *max_choices,
                        selectable_card_uuids,
                    },
                    _ => LearningCandidateResolutionV1::StrategicCandidate {
                        candidate_id: &candidate.candidate_id,
                    },
                };
                LearningModelCandidateV1 {
                    semantics: LearningModelCandidateSemanticsV1::Strategic {
                        action: &candidate.action,
                    },
                    resolution,
                }
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
                    action: combat_atomic_semantics(boundary, input)?,
                },
                resolution: LearningCandidateResolutionV1::CombatAtomic { input },
            });
        }
        for family in &boundary.legal_actions.selection_families {
            if family.selection_status == CombatSelectionStatusV2::Enabled {
                candidates.push(LearningModelCandidateV1 {
                    semantics: LearningModelCandidateSemanticsV1::CombatSelectionFamily {
                        family: LearningCombatSelectionFamilyV1 { family },
                    },
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
                monsters: LearningCombatMonstersV1 {
                    monsters: &observation.monsters,
                },
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
        Self::from_boundary_refs(boundaries.iter())
    }

    pub fn from_boundary_refs(
        boundaries: impl IntoIterator<Item = &'a LearningBoundaryV1>,
    ) -> Result<Self, LearningModelInputError> {
        Self::from_decision_results(
            boundaries
                .into_iter()
                .map(LearningModelDecisionV1::from_boundary),
        )
    }

    pub fn from_combat_boundary_refs(
        boundaries: impl IntoIterator<Item = &'a LearningCombatBoundaryV1>,
    ) -> Result<Self, LearningModelInputError> {
        Self::from_decision_results(
            boundaries
                .into_iter()
                .map(LearningModelDecisionV1::from_combat_boundary),
        )
    }

    fn from_decision_results(
        decisions: impl IntoIterator<
            Item = Result<LearningModelDecisionV1<'a>, LearningModelInputError>,
        >,
    ) -> Result<Self, LearningModelInputError> {
        let decision_results = decisions.into_iter();
        let (lower_bound, _) = decision_results.size_hint();
        let mut decisions = Vec::with_capacity(lower_bound);
        let mut candidate_row_splits = Vec::with_capacity(lower_bound.saturating_add(1));
        candidate_row_splits.push(0);
        for decision in decision_results {
            let decision = decision?;
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
        dense_action_mask(
            self.decisions
                .iter()
                .map(|decision| decision.candidates.len()),
        )
    }
}

/// Batched autoregressive selection decisions.
///
/// Each row carries the unchanged parent observation alongside the current
/// append-or-submit candidates, so a backend can batch symbolic decoding
/// across environment slots without losing state context.
#[derive(Clone, Debug)]
pub struct LearningSelectionModelBatchV1<'a> {
    pub rows: Vec<LearningSelectionModelRowV1<'a>>,
    pub candidate_row_splits: Vec<usize>,
}

impl<'a> LearningSelectionModelBatchV1<'a> {
    pub fn from_rows(
        rows: impl IntoIterator<Item = (LearningModelObservationV1<'a>, &'a LearningSelectionDraftV1)>,
    ) -> Result<Self, LearningModelInputError> {
        let rows = rows.into_iter();
        let (lower_bound, _) = rows.size_hint();
        let mut batch_rows = Vec::with_capacity(lower_bound);
        let mut candidate_row_splits = Vec::with_capacity(lower_bound.saturating_add(1));
        candidate_row_splits.push(0);
        for (observation, draft) in rows {
            let decision = draft.decision();
            ensure_nonempty(decision.candidates.len())?;
            let next = candidate_row_splits
                .last()
                .copied()
                .unwrap_or(0usize)
                .checked_add(decision.candidates.len())
                .ok_or(LearningModelInputError::CandidateCountOverflow)?;
            batch_rows.push(LearningSelectionModelRowV1 {
                observation,
                decision,
            });
            candidate_row_splits.push(next);
        }
        Ok(Self {
            rows: batch_rows,
            candidate_row_splits,
        })
    }

    pub fn flattened_candidate_count(&self) -> usize {
        self.candidate_row_splits.last().copied().unwrap_or(0)
    }

    pub fn dense_action_mask(&self) -> LearningDenseActionMaskV1 {
        dense_action_mask(self.rows.iter().map(|row| row.decision.candidates.len()))
    }
}

#[derive(Clone, Debug)]
pub struct LearningSelectionModelRowV1<'a> {
    pub observation: LearningModelObservationV1<'a>,
    pub decision: LearningSelectionDecisionV1,
}

fn dense_action_mask(
    candidate_counts: impl IntoIterator<Item = usize>,
) -> LearningDenseActionMaskV1 {
    let candidate_counts = candidate_counts.into_iter().collect::<Vec<_>>();
    let width = candidate_counts.iter().copied().max().unwrap_or(0);
    let mut values = vec![false; candidate_counts.len().saturating_mul(width)];
    for (row, candidate_count) in candidate_counts.iter().copied().enumerate() {
        let start = row * width;
        values[start..start + candidate_count].fill(true);
    }
    LearningDenseActionMaskV1 {
        rows: candidate_counts.len(),
        width,
        values,
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
#[derive(Clone)]
enum LearningSelectionFamilyStateV1 {
    Combat(CombatSelectionActionFamilyV2),
    Run(LearningRunSelectionFamilyStateV1),
}

#[derive(Clone)]
struct LearningRunSelectionFamilyStateV1 {
    planner_candidate_id: String,
    scope: SelectionScope,
    reason: SelectionReason,
    min_choices: usize,
    max_choices: usize,
    selectable_card_uuids: Vec<u32>,
}

#[derive(Clone, Copy)]
pub struct LearningRunSelectionFamilyV1<'a> {
    family: &'a LearningRunSelectionFamilyStateV1,
}

impl fmt::Debug for LearningRunSelectionFamilyV1<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LearningRunSelectionFamilyV1")
            .field("scope", &self.scope())
            .field("reason", &self.reason())
            .field("declared_min", &self.declared_min())
            .field("declared_max", &self.declared_max())
            .field("effective_max", &self.effective_max())
            .field("domain_count", &self.domain_count())
            .finish()
    }
}

impl LearningRunSelectionFamilyV1<'_> {
    pub fn scope(&self) -> SelectionScope {
        self.family.scope
    }

    pub fn reason(&self) -> SelectionReason {
        self.family.reason
    }

    pub fn declared_min(&self) -> usize {
        self.family.min_choices
    }

    pub fn declared_max(&self) -> usize {
        self.family.max_choices
    }

    pub fn effective_max(&self) -> usize {
        self.family
            .max_choices
            .min(self.family.selectable_card_uuids.len())
    }

    pub fn domain_count(&self) -> usize {
        self.family.selectable_card_uuids.len()
    }

    pub fn domain_card_uuid(&self, index: usize) -> Option<u32> {
        self.family.selectable_card_uuids.get(index).copied()
    }
}

#[derive(Clone)]
pub struct LearningSelectionDraftV1 {
    family: LearningSelectionFamilyStateV1,
    selected_domain_indices: Vec<usize>,
}

impl fmt::Debug for LearningSelectionDraftV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("LearningSelectionDraftV1")
            .field("combat_family", &self.combat_family())
            .field("run_family", &self.run_family())
            .field("selected_domain_indices", &self.selected_domain_indices)
            .finish()
    }
}

impl LearningSelectionDraftV1 {
    fn from_combat(family: CombatSelectionActionFamilyV2) -> Self {
        Self {
            family: LearningSelectionFamilyStateV1::Combat(family),
            selected_domain_indices: Vec::new(),
        }
    }

    fn from_run(
        planner_candidate_id: String,
        scope: SelectionScope,
        reason: SelectionReason,
        min_choices: usize,
        max_choices: usize,
        selectable_card_uuids: Vec<u32>,
    ) -> Self {
        Self {
            family: LearningSelectionFamilyStateV1::Run(LearningRunSelectionFamilyStateV1 {
                planner_candidate_id,
                scope,
                reason,
                min_choices,
                max_choices,
                selectable_card_uuids,
            }),
            selected_domain_indices: Vec::new(),
        }
    }

    pub fn combat_family(&self) -> Option<LearningCombatSelectionFamilyV1<'_>> {
        match &self.family {
            LearningSelectionFamilyStateV1::Combat(family) => {
                Some(LearningCombatSelectionFamilyV1 { family })
            }
            LearningSelectionFamilyStateV1::Run(_) => None,
        }
    }

    pub fn run_family(&self) -> Option<LearningRunSelectionFamilyV1<'_>> {
        match &self.family {
            LearningSelectionFamilyStateV1::Combat(_) => None,
            LearningSelectionFamilyStateV1::Run(family) => {
                Some(LearningRunSelectionFamilyV1 { family })
            }
        }
    }

    pub fn selected_domain_indices(&self) -> &[usize] {
        &self.selected_domain_indices
    }

    pub fn decision(&self) -> LearningSelectionDecisionV1 {
        let mut candidates = Vec::new();
        if self.can_submit() {
            candidates.push(LearningSelectionCandidateV1 {
                semantics: LearningSelectionCandidateSemanticsV1::Submit,
                resolution: LearningSelectionCandidateResolutionV1::Submit,
            });
        }
        let domain_count = match &self.family {
            LearningSelectionFamilyStateV1::Combat(family) => family.raw_domain.len(),
            LearningSelectionFamilyStateV1::Run(family) => family.selectable_card_uuids.len(),
        };
        for domain_index in 0..domain_count {
            if self.can_append(domain_index) {
                candidates.push(LearningSelectionCandidateV1 {
                    semantics: LearningSelectionCandidateSemanticsV1::Append { domain_index },
                    resolution: LearningSelectionCandidateResolutionV1::Append { domain_index },
                });
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
        match &self.family {
            LearningSelectionFamilyStateV1::Combat(family) => {
                family.selection_status == CombatSelectionStatusV2::Enabled
                    && self.selected_domain_indices.len() >= u64_to_usize(family.declared_min)
                    && self.selected_domain_indices.len() <= u64_to_usize(family.effective_max)
            }
            LearningSelectionFamilyStateV1::Run(family) => {
                self.selected_domain_indices.len() >= family.min_choices
                    && self.selected_domain_indices.len()
                        <= family.max_choices.min(family.selectable_card_uuids.len())
            }
        }
    }

    fn can_append(&self, domain_index: usize) -> bool {
        match &self.family {
            LearningSelectionFamilyStateV1::Combat(family) => {
                if family.selection_status != CombatSelectionStatusV2::Enabled
                    || self.selected_domain_indices.len() >= u64_to_usize(family.effective_max)
                {
                    return false;
                }
                let Some(candidate) = family.raw_domain.get(domain_index) else {
                    return false;
                };
                domain_candidate_is_eligible(candidate)
                    && !self
                        .selected_domain_indices
                        .iter()
                        .copied()
                        .any(|selected| same_selection_identity(family, selected, domain_index))
            }
            LearningSelectionFamilyStateV1::Run(family) => {
                if self.selected_domain_indices.len()
                    >= family.max_choices.min(family.selectable_card_uuids.len())
                {
                    return false;
                }
                let Some(candidate_uuid) = family.selectable_card_uuids.get(domain_index) else {
                    return false;
                };
                !self.selected_domain_indices.iter().any(|selected| {
                    family.selectable_card_uuids.get(*selected) == Some(candidate_uuid)
                })
            }
        }
    }

    fn to_learning_action(&self) -> Result<LearningActionV1, LearningModelInputError> {
        if !self.can_submit() {
            return Err(LearningModelInputError::SelectionCannotSubmit);
        }
        match &self.family {
            LearningSelectionFamilyStateV1::Combat(family) => {
                let input = match family.input_encoding {
                    CombatSelectionInputEncodingV2::SubmitSelectionHandCardUuids => {
                        ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                            SelectionScope::Hand,
                            self.selected_combat_card_uuids(family)?,
                        ))
                    }
                    CombatSelectionInputEncodingV2::SubmitSelectionGridCardUuids => {
                        ClientInput::SubmitSelection(SelectionResolution::card_uuids(
                            SelectionScope::Grid,
                            self.selected_combat_card_uuids(family)?,
                        ))
                    }
                    CombatSelectionInputEncodingV2::SubmitScryDiscardIndices => {
                        ClientInput::SubmitScryDiscard(self.selected_scry_indices(family)?)
                    }
                };
                Ok(LearningActionV1::CombatInput { input })
            }
            LearningSelectionFamilyStateV1::Run(family) => {
                let selected = self
                    .selected_domain_indices
                    .iter()
                    .map(|index| {
                        family
                            .selectable_card_uuids
                            .get(*index)
                            .copied()
                            .ok_or(LearningModelInputError::SelectionDomainEncodingMismatch)
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                Ok(LearningActionV1::RunSelection {
                    candidate_id: family.planner_candidate_id.clone(),
                    resolution: SelectionResolution::card_uuids(family.scope, selected),
                })
            }
        }
    }

    fn selected_combat_card_uuids(
        &self,
        family: &CombatSelectionActionFamilyV2,
    ) -> Result<Vec<u32>, LearningModelInputError> {
        self.selected_domain_indices
            .iter()
            .map(|index| match family.raw_domain.get(*index) {
                Some(CombatSelectionDomainCandidateV2::CardUuid { uuid, .. }) => Ok(*uuid),
                _ => Err(LearningModelInputError::SelectionDomainEncodingMismatch),
            })
            .collect()
    }

    fn selected_scry_indices(
        &self,
        family: &CombatSelectionActionFamilyV2,
    ) -> Result<Vec<usize>, LearningModelInputError> {
        self.selected_domain_indices
            .iter()
            .map(|index| match family.raw_domain.get(*index) {
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
pub enum LearningSelectionCandidateSemanticsV1 {
    Submit,
    Append { domain_index: usize },
}

#[derive(Clone, Copy, Debug)]
pub struct LearningSelectionCandidateV1 {
    pub semantics: LearningSelectionCandidateSemanticsV1,
    resolution: LearningSelectionCandidateResolutionV1,
}

#[derive(Clone, Copy, Debug)]
enum LearningSelectionCandidateResolutionV1 {
    Submit,
    Append { domain_index: usize },
}

#[derive(Clone, Debug)]
pub struct LearningSelectionDecisionV1 {
    pub candidates: Vec<LearningSelectionCandidateV1>,
}

#[derive(Clone, Debug)]
pub enum LearningSelectionStepV1 {
    Continue,
    Apply(LearningActionV1),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LearningModelInputError {
    TerminalBoundary,
    UnsupportedBoundary,
    IncompleteStrategicCandidateSet {
        site: PlannerDecisionSite,
        gaps: Vec<CandidateRepresentationGap>,
    },
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
    UnsupportedCombatAtomicInput,
    CombatTargetMissing,
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
) -> Option<LearningCombatIndexedChoiceV1<'a>> {
    let ClientInput::SubmitDiscoverChoice(index) = input else {
        return None;
    };
    let indexed = surface.indexed_choice.as_ref()?;
    Some(LearningCombatIndexedChoiceV1 {
        input_encoding: indexed.input_encoding,
        reason: &indexed.reason,
        candidate: indexed.candidates.get(*index)?,
    })
}

fn combat_atomic_semantics<'a>(
    boundary: &'a LearningCombatBoundaryV1,
    input: &ClientInput,
) -> Result<LearningCombatAtomicActionV1<'a>, LearningModelInputError> {
    let target_index = |target: Option<usize>| {
        target
            .map(|entity_id| {
                boundary
                    .observation
                    .monsters
                    .iter()
                    .position(|monster| monster.entity_id == entity_id)
                    .ok_or(LearningModelInputError::CombatTargetMissing)
            })
            .transpose()
    };
    match input {
        ClientInput::PlayCard { card_index, target } => {
            Ok(LearningCombatAtomicActionV1::PlayCard {
                hand_index: *card_index,
                target_monster_index: target_index(*target)?,
            })
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => Ok(LearningCombatAtomicActionV1::UsePotion {
            potion_index: *potion_index,
            target_monster_index: target_index(*target)?,
        }),
        ClientInput::DiscardPotion(potion_index) => {
            Ok(LearningCombatAtomicActionV1::DiscardPotion {
                potion_index: *potion_index,
            })
        }
        ClientInput::EndTurn => Ok(LearningCombatAtomicActionV1::EndTurn),
        ClientInput::SubmitDiscoverChoice(choice_index) => {
            Ok(LearningCombatAtomicActionV1::SubmitIndexedChoice {
                choice_index: *choice_index,
                indexed: indexed_choice_semantics(&boundary.legal_actions, input)
                    .ok_or(LearningModelInputError::IndexedChoiceMetadataMissing)?,
            })
        }
        ClientInput::Proceed => Ok(LearningCombatAtomicActionV1::Proceed),
        ClientInput::Cancel => Ok(LearningCombatAtomicActionV1::Cancel),
        ClientInput::SubmitCardChoice(_)
        | ClientInput::SelectMapNode(_)
        | ClientInput::FlyToNode(_, _)
        | ClientInput::SelectEventOption(_)
        | ClientInput::CampfireOption(_)
        | ClientInput::EventChoice(_)
        | ClientInput::SubmitScryDiscard(_)
        | ClientInput::SubmitSelection(_)
        | ClientInput::ClaimReward(_)
        | ClientInput::OpenRewardOverlay
        | ClientInput::OpenChest
        | ClientInput::SelectCard(_)
        | ClientInput::BuyCard(_)
        | ClientInput::BuyRelic(_)
        | ClientInput::BuyPotion(_)
        | ClientInput::PurgeCard(_)
        | ClientInput::SubmitRelicChoice(_) => {
            Err(LearningModelInputError::UnsupportedCombatAtomicInput)
        }
    }
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
        RunPendingChoiceReason, RunPendingChoiceState,
    };
    use crate::state::map::node::RoomType;
    use crate::state::selection::DomainEventSource;
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
    fn combat_model_views_use_local_monster_indices_and_hide_runtime_entity_ids() {
        const PRIVATE_ENTITY_ID: usize = 3_000_000_001;
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Bash, 51)];
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = PRIVATE_ENTITY_ID;
        combat.entities.monsters.push(monster);
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let boundary = LearningEnvV1::from_session(session)
            .observe()
            .expect("targeted combat boundary");
        let decision =
            LearningModelDecisionV1::from_boundary(&boundary).expect("build model decision");

        assert!(decision.candidates.iter().any(|candidate| matches!(
            candidate.semantics,
            LearningModelCandidateSemanticsV1::CombatAtomic {
                action: LearningCombatAtomicActionV1::PlayCard {
                    hand_index: 0,
                    target_monster_index: Some(0),
                },
            }
        )));
        assert!(matches!(
            decision.observation,
            LearningModelObservationV1::Combat(LearningCombatModelObservationV1 {
                monsters,
                ..
            }) if monsters.len() == 1 && monsters.get(0).is_some()
        ));
        let rendered = format!("{decision:?}");
        assert!(
            !rendered.contains(&PRIVATE_ENTITY_ID.to_string()),
            "model-facing debug output leaked a private monster entity id: {rendered}"
        );
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

        assert!(decision.candidates.iter().any(|candidate| {
            let LearningModelCandidateSemanticsV1::CombatAtomic {
                action:
                    LearningCombatAtomicActionV1::SubmitIndexedChoice {
                        choice_index: 1,
                        indexed,
                    },
            } = candidate.semantics
            else {
                return false;
            };
            indexed.input_encoding == CombatIndexedChoiceInputEncodingV2::SubmitDiscoverChoiceIndex
                && matches!(
                    indexed.reason,
                    CombatIndexedChoiceReasonV2::Discovery {
                        colorless: false,
                        card_type: None,
                        amount: 1,
                    }
                )
                && matches!(
                    indexed.candidate,
                    CombatIndexedChoiceCandidateV2::Card {
                        card_id: CardId::FiendFire,
                        upgrades: 0,
                    }
                )
        }));
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
        let mut draft = LearningSelectionDraftV1::from_combat(family);

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
    fn symbolic_run_card_selection_applies_only_after_explicit_submit() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.master_deck = vec![
            CombatCard::new(CardId::Strike, 11),
            CombatCard::new(CardId::Defend, 22),
            CombatCard::new(CardId::Bash, 33),
        ];
        session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
            min_choices: 2,
            max_choices: 2,
            reason: RunPendingChoiceReason::Transform,
            source: DomainEventSource::Selection(SelectionReason::Transform),
            return_state: Box::new(EngineState::MapNavigation),
        });
        let mut env = LearningEnvV1::from_session(session);
        let boundary = env.observe().expect("run selection boundary");
        let decision =
            LearningModelDecisionV1::from_boundary(&boundary).expect("root model decision");
        let LearningModelChoiceV1::DecodeSelection(mut draft) =
            decision.choose(0).expect("start run selection")
        else {
            panic!("run selection root must start a symbolic decoder");
        };

        assert!(draft.run_family().is_some());
        assert!(matches!(
            draft.choose(0).expect("append first card"),
            LearningSelectionStepV1::Continue
        ));
        assert!(matches!(
            draft.choose(0).expect("append second card"),
            LearningSelectionStepV1::Continue
        ));
        let LearningSelectionStepV1::Apply(action) = draft.choose(0).expect("submit selection")
        else {
            panic!("explicit submit must produce a run action");
        };
        let LearningActionV1::RunSelection { resolution, .. } = &action else {
            panic!("run decoder must produce a run selection action");
        };
        assert_eq!(resolution.scope, SelectionScope::Deck);
        assert_eq!(resolution.selected_card_uuids(), vec![11, 22]);

        let step = env.step(action).expect("apply typed run selection");
        assert!(matches!(
            step.boundary,
            LearningBoundaryV1::Strategic { .. }
        ));
    }

    #[test]
    fn symbolic_model_views_and_debug_output_hide_runtime_card_uuids() {
        const PRIVATE_UUID: u32 = 3_000_000_001;
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.draw_pile = (vec![CombatCard::new(CardId::Strike, PRIVATE_UUID)]).into();
        let surface = combat_legal_action_surface_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike],
                card_uuids: vec![PRIVATE_UUID],
            }),
            &combat,
        );
        let draft = LearningSelectionDraftV1::from_combat(surface.selection_families[0].clone());
        let family = draft.combat_family().expect("combat family");
        assert_eq!(
            family.domain(0).expect("public domain").semantics(),
            LearningCombatSelectionDomainSemanticsV1::Scry {
                index: 0,
                card_id: Some(CardId::Strike),
                currently_present: true,
            }
        );
        for rendered in [
            format!("{family:?}"),
            format!("{:?}", family.domain(0).expect("public domain")),
            format!("{draft:?}"),
            format!("{:?}", draft.decision()),
        ] {
            assert!(
                !rendered.contains(&PRIVATE_UUID.to_string()),
                "model-facing debug output leaked a private card UUID: {rendered}"
            );
        }
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
        let mut draft =
            LearningSelectionDraftV1::from_combat(surface.selection_families[0].clone());

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
    fn symbolic_decoder_rows_batch_with_their_parent_observations() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let boundary = LearningEnvV1::from_session(session)
            .observe()
            .expect("combat boundary");
        let observation = LearningModelDecisionV1::from_boundary(&boundary)
            .expect("model decision")
            .observation;

        let mut selection_combat = crate::test_support::blank_test_combat();
        selection_combat.zones.draw_pile = ((1..=3)
            .map(|uuid| CombatCard::new(CardId::Strike, uuid))
            .collect::<Vec<_>>())
        .into();
        let surface = combat_legal_action_surface_v2(
            &EngineState::PendingChoice(PendingChoice::ScrySelect {
                cards: vec![CardId::Strike; 3],
                card_uuids: vec![1, 2, 3],
            }),
            &selection_combat,
        );
        let first = LearningSelectionDraftV1::from_combat(surface.selection_families[0].clone());
        let mut second = first.clone();
        assert!(matches!(
            second.choose(1).expect("append first domain item"),
            LearningSelectionStepV1::Continue
        ));

        let batch = LearningSelectionModelBatchV1::from_rows([
            (observation, &first),
            (observation, &second),
        ])
        .expect("batch selection decisions");
        assert_eq!(batch.candidate_row_splits, vec![0, 4, 7]);
        assert_eq!(batch.flattened_candidate_count(), 7);
        assert_eq!(
            batch.dense_action_mask(),
            LearningDenseActionMaskV1 {
                rows: 2,
                width: 4,
                values: vec![true, true, true, true, true, true, true, false],
            }
        );
        assert!(matches!(
            batch.rows[0].observation,
            LearningModelObservationV1::Combat(_)
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
            LearningModelInputError::IncompleteStrategicCandidateSet {
                site: crate::ai::planner_core::PlannerDecisionSite::Neow,
                gaps: vec![
                    crate::ai::planner_core::CandidateRepresentationGap::UnsupportedBoundaryAction,
                ],
            }
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
