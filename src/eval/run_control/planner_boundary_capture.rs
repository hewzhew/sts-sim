use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::ai::planner_core::{
    stable_planner_id, CandidateCompletenessBasis, CandidateRepresentationGap,
    CandidateSetCompleteness, LegalCandidate, LegalCandidateSet, PlannerAction,
    PlannerCardObservation, PlannerDecisionContext, PlannerDecisionSite, PlannerMechanicsManifest,
    PlannerObservation, PlannerOfferedCard, PlannerPlayerClass, PlannerPotionObservation,
    PlannerPotionSlotObservation, PlannerPublicHistory, PlannerPublicMap, PlannerPublicMapEdge,
    PlannerPublicMapNode, PlannerRelicObservation, PlannerRewardDescriptor, PlannerRunGoal,
    PlannerRunScalars, LEGAL_CANDIDATE_SET_SCHEMA_NAME, LEGAL_CANDIDATE_SET_SCHEMA_VERSION,
    PLANNER_MECHANICS_ID, PLANNER_MECHANICS_VERSION, PLANNER_OBSERVATION_SCHEMA_NAME,
    PLANNER_OBSERVATION_SCHEMA_VERSION,
};
use crate::state::core::{CampfireChoice, ClientInput, EngineState};
use crate::state::rewards::{RewardItem, RewardState};

use super::view_model::{CandidateAction, DecisionCandidate, DecisionCandidateKey};
use super::{
    build_decision_surface, RunControlSession, RunDecisionAction, RunDecisionBoundaryV1,
    RunDecisionSelectionSourceV1, RunProgressStepV1,
};

pub const PLANNER_BOUNDARY_CAPTURE_SEGMENT_SCHEMA_NAME: &str = "PlannerBoundaryCaptureSegment";
pub const PLANNER_BOUNDARY_CAPTURE_SEGMENT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerBoundaryYieldKindV1 {
    CallbackStop,
    ProgressBudgetExhausted,
    WallDeadlineReached,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PlannerBoundaryMutationKindV1 {
    ForcedTransition,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum PlannerBoundaryVisitOutcomeV1 {
    Selected {
        selection_source: RunDecisionSelectionSourceV1,
        run_candidate_id: String,
        planner_candidate_id: String,
    },
    SelectionNotRepresented {
        selection_source: RunDecisionSelectionSourceV1,
        run_candidate_id: String,
    },
    Yielded {
        yield_kind: PlannerBoundaryYieldKindV1,
    },
    MutationWithoutSelection {
        mutation_kind: PlannerBoundaryMutationKindV1,
    },
    ExecutionFailed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerBoundaryCandidateLinkV1 {
    pub run_candidate_id: String,
    pub planner_candidate_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerBoundaryVisitV1 {
    pub visit_id: String,
    pub decision_step: u64,
    pub observation: PlannerObservation,
    pub legal_candidate_set: LegalCandidateSet,
    pub candidate_links: Vec<PlannerBoundaryCandidateLinkV1>,
    pub outcome: PlannerBoundaryVisitOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerBoundaryCaptureSegmentV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub visits: Vec<PlannerBoundaryVisitV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerBoundaryCaptureCoverageReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub total_visits: usize,
    pub selected_visits: usize,
    pub yielded_visits: usize,
    pub mutation_without_selection_visits: usize,
    pub unrepresented_selection_visits: usize,
    pub execution_failed_visits: usize,
    pub duplicate_visit_ids: usize,
    pub sites: Vec<PlannerBoundarySiteCoverageV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PlannerBoundarySiteCoverageV1 {
    pub site: PlannerDecisionSite,
    pub visits: usize,
    pub selected_candidates_linked: usize,
    pub complete_candidate_sets: usize,
    pub incomplete_candidate_sets: usize,
    pub representation_gaps: Vec<CandidateRepresentationGap>,
}

#[derive(Clone, Debug)]
pub struct PlannerBoundaryCaptureTicketV1 {
    pending: PendingPlannerBoundaryCaptureV1,
}

impl PlannerBoundaryCaptureTicketV1 {
    pub fn finish_for_progress(
        self,
        progress_steps: &[RunProgressStepV1],
    ) -> PlannerBoundaryCaptureSegmentV1 {
        let mut segment = PlannerBoundaryCaptureSegmentV1::default();
        segment.push(
            self.pending
                .finish_for_progress(progress_steps, PlannerBoundaryYieldKindV1::CallbackStop),
        );
        segment
    }

    pub fn finish_failed(self) -> PlannerBoundaryCaptureSegmentV1 {
        let mut segment = PlannerBoundaryCaptureSegmentV1::default();
        segment.push(
            self.pending
                .finish(PlannerBoundaryVisitOutcomeV1::ExecutionFailed),
        );
        segment
    }
}

impl Default for PlannerBoundaryCaptureSegmentV1 {
    fn default() -> Self {
        Self {
            schema_name: PLANNER_BOUNDARY_CAPTURE_SEGMENT_SCHEMA_NAME.to_string(),
            schema_version: PLANNER_BOUNDARY_CAPTURE_SEGMENT_SCHEMA_VERSION,
            visits: Vec::new(),
        }
    }
}

impl PlannerBoundaryCaptureSegmentV1 {
    pub fn append(&mut self, mut other: Self) {
        self.visits.append(&mut other.visits);
    }

    pub fn push(&mut self, visit: PlannerBoundaryVisitV1) {
        self.visits.push(visit);
    }

    pub fn is_empty(&self) -> bool {
        self.visits.is_empty()
    }
}

pub fn build_planner_boundary_capture_coverage_report_v1(
    segment: &PlannerBoundaryCaptureSegmentV1,
) -> PlannerBoundaryCaptureCoverageReportV1 {
    use std::collections::BTreeMap;

    let mut seen = BTreeSet::new();
    let mut duplicate_visit_ids = 0;
    let mut selected_visits = 0;
    let mut yielded_visits = 0;
    let mut mutation_without_selection_visits = 0;
    let mut unrepresented_selection_visits = 0;
    let mut execution_failed_visits = 0;
    let mut sites = BTreeMap::<String, PlannerBoundarySiteCoverageV1>::new();
    for visit in &segment.visits {
        if !seen.insert(visit.visit_id.as_str()) {
            duplicate_visit_ids += 1;
            continue;
        }
        let row = sites
            .entry(format!("{:?}", visit.observation.decision_site))
            .or_insert_with(|| PlannerBoundarySiteCoverageV1 {
                site: visit.observation.decision_site,
                visits: 0,
                selected_candidates_linked: 0,
                complete_candidate_sets: 0,
                incomplete_candidate_sets: 0,
                representation_gaps: Vec::new(),
            });
        row.visits += 1;
        match &visit.legal_candidate_set.completeness {
            CandidateSetCompleteness::Complete { .. } => row.complete_candidate_sets += 1,
            CandidateSetCompleteness::Incomplete { gaps, .. } => {
                row.incomplete_candidate_sets += 1;
                row.representation_gaps.extend(gaps.iter().cloned());
            }
        }
        match &visit.outcome {
            PlannerBoundaryVisitOutcomeV1::Selected {
                run_candidate_id,
                planner_candidate_id,
                ..
            } => {
                selected_visits += 1;
                let candidate_is_present = visit
                    .legal_candidate_set
                    .candidates
                    .iter()
                    .any(|candidate| candidate.candidate_id == *planner_candidate_id);
                let link_is_present = visit.candidate_links.iter().any(|link| {
                    link.run_candidate_id == *run_candidate_id
                        && link.planner_candidate_id == *planner_candidate_id
                });
                if candidate_is_present && link_is_present {
                    row.selected_candidates_linked += 1;
                }
            }
            PlannerBoundaryVisitOutcomeV1::SelectionNotRepresented { .. } => {
                unrepresented_selection_visits += 1;
            }
            PlannerBoundaryVisitOutcomeV1::Yielded { .. } => yielded_visits += 1,
            PlannerBoundaryVisitOutcomeV1::MutationWithoutSelection { .. } => {
                mutation_without_selection_visits += 1;
            }
            PlannerBoundaryVisitOutcomeV1::ExecutionFailed => execution_failed_visits += 1,
        }
    }
    let mut sites = sites.into_values().collect::<Vec<_>>();
    for row in &mut sites {
        row.representation_gaps.sort();
        row.representation_gaps.dedup();
    }
    PlannerBoundaryCaptureCoverageReportV1 {
        schema_name: "PlannerBoundaryCaptureCoverageReport".to_string(),
        schema_version: 1,
        total_visits: seen.len(),
        selected_visits,
        yielded_visits,
        mutation_without_selection_visits,
        unrepresented_selection_visits,
        execution_failed_visits,
        duplicate_visit_ids,
        sites,
    }
}

#[derive(Clone, Debug)]
pub(in crate::eval::run_control) struct PendingPlannerBoundaryCaptureV1 {
    visit_id: String,
    decision_step: u64,
    observation: PlannerObservation,
    legal_candidate_set: LegalCandidateSet,
    candidate_links: Vec<PlannerBoundaryCandidateLinkV1>,
    run_boundary: RunDecisionBoundaryV1,
}

impl PendingPlannerBoundaryCaptureV1 {
    pub(in crate::eval::run_control) fn finish(
        self,
        outcome: PlannerBoundaryVisitOutcomeV1,
    ) -> PlannerBoundaryVisitV1 {
        PlannerBoundaryVisitV1 {
            visit_id: self.visit_id,
            decision_step: self.decision_step,
            observation: self.observation,
            legal_candidate_set: self.legal_candidate_set,
            candidate_links: self.candidate_links,
            outcome,
        }
    }

    pub(in crate::eval::run_control) fn finish_for_progress(
        self,
        progress_steps: &[RunProgressStepV1],
        yield_kind: PlannerBoundaryYieldKindV1,
    ) -> PlannerBoundaryVisitV1 {
        let outcome = match progress_steps {
            [RunProgressStepV1::Decision(transaction)]
                if transaction.before == self.run_boundary =>
            {
                let run_candidate_id = transaction.selection.candidate_id.clone();
                match self
                    .candidate_links
                    .iter()
                    .find(|link| link.run_candidate_id == run_candidate_id)
                {
                    Some(link) => PlannerBoundaryVisitOutcomeV1::Selected {
                        selection_source: transaction.selection.source,
                        run_candidate_id,
                        planner_candidate_id: link.planner_candidate_id.clone(),
                    },
                    None => PlannerBoundaryVisitOutcomeV1::SelectionNotRepresented {
                        selection_source: transaction.selection.source,
                        run_candidate_id,
                    },
                }
            }
            [RunProgressStepV1::Decision(_)] => PlannerBoundaryVisitOutcomeV1::ExecutionFailed,
            [RunProgressStepV1::ForcedTransition(transition)]
                if transition.before == self.run_boundary =>
            {
                PlannerBoundaryVisitOutcomeV1::MutationWithoutSelection {
                    mutation_kind: PlannerBoundaryMutationKindV1::ForcedTransition,
                }
            }
            [RunProgressStepV1::ForcedTransition(_)] => {
                PlannerBoundaryVisitOutcomeV1::ExecutionFailed
            }
            [] => PlannerBoundaryVisitOutcomeV1::Yielded { yield_kind },
            _ => PlannerBoundaryVisitOutcomeV1::ExecutionFailed,
        };
        self.finish(outcome)
    }
}

pub(in crate::eval::run_control) fn capture_planner_boundary_v1(
    session: &RunControlSession,
) -> Result<Option<PendingPlannerBoundaryCaptureV1>, String> {
    let Some((site, context)) = planner_site_and_context(session)? else {
        return Ok(None);
    };
    let mechanics = planner_mechanics();
    let mut observation = PlannerObservation {
        schema_name: PLANNER_OBSERVATION_SCHEMA_NAME.to_string(),
        schema_version: PLANNER_OBSERVATION_SCHEMA_VERSION,
        observation_id: String::new(),
        mechanics: mechanics.clone(),
        run_goal: if session.run_state.is_final_act_available {
            PlannerRunGoal::HeartVictory
        } else {
            PlannerRunGoal::ActThreeVictory
        },
        decision_site: site,
        run: PlannerRunScalars {
            player_class: planner_player_class(session.run_state.player_class)?,
            ascension_level: session.run_state.ascension_level,
            act: session.run_state.act_num,
            floor: session.run_state.floor_num,
            current_hp: session.run_state.current_hp,
            max_hp: session.run_state.max_hp,
            gold: session.run_state.gold,
            keys: session.run_state.keys,
            potion_capacity: session.run_state.potions.len(),
        },
        cards: session
            .run_state
            .master_deck
            .iter()
            .map(|card| PlannerCardObservation {
                card_uuid: card.uuid,
                card: card.id,
                upgrades: card.upgrades,
                misc_value: card.misc_value,
                base_damage_override: card.base_damage_override,
                base_block_override: card.base_block_override,
                cost_modifier: card.cost_modifier,
            })
            .collect(),
        relics: session
            .run_state
            .relics
            .iter()
            .map(|relic| PlannerRelicObservation {
                relic: relic.id,
                counter: relic.counter,
                used_up: relic.used_up,
                amount: relic.amount,
            })
            .collect(),
        potions: session
            .run_state
            .potions
            .iter()
            .enumerate()
            .map(|(slot, potion)| PlannerPotionSlotObservation {
                slot,
                potion: potion.as_ref().map(|potion| PlannerPotionObservation {
                    potion: potion.id,
                    potion_uuid: potion.uuid,
                    can_use: potion.can_use,
                    can_discard: potion.can_discard,
                    requires_target: potion.requires_target,
                }),
            })
            .collect(),
        public_map: planner_public_map(session),
        context,
        public_history: PlannerPublicHistory {
            shop_purge_count: session.run_state.shop_purge_count,
        },
    };
    observation.observation_id = stable_planner_id("observation", &observation)?;

    let surface = build_decision_surface(session);
    let run_boundary = RunDecisionBoundaryV1::capture(session);
    let mut candidates = Vec::new();
    let mut candidate_links = Vec::new();
    let mut gaps = BTreeSet::new();
    for candidate in &surface.view.candidates {
        match planner_action_for_candidate(session, site, candidate)? {
            CandidateProjection::Legal(action) => {
                let candidate_id = stable_planner_id("candidate", &action)?;
                if candidates
                    .iter()
                    .any(|existing: &LegalCandidate| existing.candidate_id == candidate_id)
                {
                    gaps.insert(CandidateRepresentationGap::DuplicateTypedIdentity);
                    continue;
                }
                candidate_links.push(PlannerBoundaryCandidateLinkV1 {
                    run_candidate_id: candidate.id.clone(),
                    planner_candidate_id: candidate_id.clone(),
                });
                candidates.push(LegalCandidate {
                    candidate_id,
                    action,
                    mechanics: mechanics.clone(),
                });
            }
            CandidateProjection::Gap(gap) => {
                gaps.insert(gap);
            }
            CandidateProjection::Unavailable => {}
        }
    }
    let visible_legal_count = surface
        .view
        .candidates
        .iter()
        .filter(|candidate| !matches!(candidate.action, CandidateAction::Unavailable { .. }))
        .count();
    if visible_legal_count > 0 && candidates.is_empty() {
        gaps.insert(CandidateRepresentationGap::NoRepresentedLegalCandidate);
    }
    let basis = CandidateCompletenessBasis::RunControlBoundaryEnumerator;
    let completeness = if gaps.is_empty() {
        CandidateSetCompleteness::Complete { basis }
    } else {
        CandidateSetCompleteness::Incomplete {
            basis,
            gaps: gaps.into_iter().collect(),
        }
    };
    let decision_id = stable_planner_id(
        "decision",
        &(
            session.decision_step,
            observation.observation_id.as_str(),
            site,
        ),
    )?;
    let mut legal_candidate_set = LegalCandidateSet {
        schema_name: LEGAL_CANDIDATE_SET_SCHEMA_NAME.to_string(),
        schema_version: LEGAL_CANDIDATE_SET_SCHEMA_VERSION,
        candidate_set_id: String::new(),
        decision_id,
        observation_id: observation.observation_id.clone(),
        site,
        candidates,
        completeness,
    };
    legal_candidate_set.candidate_set_id =
        stable_planner_id("candidate_set", &legal_candidate_set)?;
    let visit_id = stable_planner_id(
        "boundary_visit",
        &(
            session.decision_step,
            observation.observation_id.as_str(),
            legal_candidate_set.candidate_set_id.as_str(),
        ),
    )?;

    Ok(Some(PendingPlannerBoundaryCaptureV1 {
        visit_id,
        decision_step: session.decision_step,
        observation,
        legal_candidate_set,
        candidate_links,
        run_boundary,
    }))
}

pub fn capture_planner_boundary_ticket_v1(
    session: &RunControlSession,
) -> Result<Option<PlannerBoundaryCaptureTicketV1>, String> {
    Ok(capture_planner_boundary_v1(session)?
        .map(|pending| PlannerBoundaryCaptureTicketV1 { pending }))
}

pub fn capture_planner_boundary_yield_v1(
    session: &RunControlSession,
    yield_kind: PlannerBoundaryYieldKindV1,
) -> Result<PlannerBoundaryCaptureSegmentV1, String> {
    let mut segment = PlannerBoundaryCaptureSegmentV1::default();
    if let Some(pending) = capture_planner_boundary_v1(session)? {
        segment.push(pending.finish(PlannerBoundaryVisitOutcomeV1::Yielded { yield_kind }));
    }
    Ok(segment)
}

enum CandidateProjection {
    Legal(PlannerAction),
    Gap(CandidateRepresentationGap),
    Unavailable,
}

fn planner_action_for_candidate(
    session: &RunControlSession,
    site: PlannerDecisionSite,
    candidate: &DecisionCandidate,
) -> Result<CandidateProjection, String> {
    if matches!(candidate.action, CandidateAction::Unavailable { .. }) {
        return Ok(CandidateProjection::Unavailable);
    }
    if matches!(candidate.action, CandidateAction::Parameterized { .. }) {
        return Ok(CandidateProjection::Gap(
            CandidateRepresentationGap::ParameterizedActionFamily,
        ));
    }
    let action = match candidate.key.as_ref() {
        Some(DecisionCandidateKey::EventOption {
            event_id,
            screen,
            option_index,
            action,
        }) => Some(PlannerAction::ChooseEventOption {
            event: *event_id,
            screen: *screen,
            option_index: *option_index,
            action: *action,
        }),
        Some(DecisionCandidateKey::CardRewardPick {
            reward_item_index,
            option_index,
            card,
            upgrades,
        }) => Some(PlannerAction::TakeCard {
            reward_item_index: Some(*reward_item_index),
            option_index: *option_index,
            card: *card,
            upgrades: *upgrades,
        }),
        Some(DecisionCandidateKey::CardRewardOpen { reward_item_index }) => {
            Some(PlannerAction::OpenCardReward {
                reward_item_index: *reward_item_index,
            })
        }
        Some(DecisionCandidateKey::CardRewardSingingBowl {
            reward_item_index, ..
        }) => Some(PlannerAction::SingingBowl {
            reward_item_index: Some(*reward_item_index),
        }),
        Some(DecisionCandidateKey::CardRewardSkip { reward_item_index }) => {
            Some(PlannerAction::SkipCardReward {
                reward_item_index: *reward_item_index,
            })
        }
        Some(DecisionCandidateKey::BossRelicPick {
            option_index,
            relic,
        }) => Some(PlannerAction::TakeBossRelic {
            option_index: *option_index,
            relic: *relic,
        }),
        Some(DecisionCandidateKey::BossRelicSkip) => Some(PlannerAction::SkipBossRelic),
        Some(DecisionCandidateKey::ShopPurgeCard {
            deck_index,
            card,
            upgrades,
        }) => session
            .run_state
            .master_deck
            .get(*deck_index)
            .map(|deck_card| PlannerAction::RemoveCard {
                card_uuid: deck_card.uuid,
                card: *card,
                upgrades: *upgrades,
                price: current_shop(session).map_or(0, |shop| shop.purge_cost),
            }),
        Some(DecisionCandidateKey::ShopBuyCard {
            shop_slot,
            card,
            upgrades,
            price,
        }) => Some(PlannerAction::BuyCard {
            shop_slot: *shop_slot,
            card: *card,
            upgrades: *upgrades,
            price: *price,
        }),
        Some(DecisionCandidateKey::ShopBuyRelic {
            shop_slot,
            relic,
            price,
        }) => Some(PlannerAction::BuyRelic {
            shop_slot: *shop_slot,
            relic: *relic,
            price: *price,
        }),
        Some(DecisionCandidateKey::ShopBuyPotion {
            shop_slot,
            potion,
            price,
        }) => Some(PlannerAction::BuyPotion {
            shop_slot: *shop_slot,
            potion: *potion,
            price: *price,
        }),
        Some(DecisionCandidateKey::ShopOpenRewards) => Some(PlannerAction::OpenPendingRewards),
        Some(DecisionCandidateKey::SelectionSubmit { .. }) => None,
        Some(DecisionCandidateKey::ShopLeave) => Some(PlannerAction::LeaveShop),
        None => planner_action_from_execution(session, site, candidate)?,
    };
    Ok(match action {
        Some(action) => CandidateProjection::Legal(action),
        None if matches!(
            candidate.key,
            Some(DecisionCandidateKey::SelectionSubmit { .. })
        ) =>
        {
            CandidateProjection::Gap(CandidateRepresentationGap::ParameterizedActionFamily)
        }
        None => CandidateProjection::Gap(CandidateRepresentationGap::UnsupportedBoundaryAction),
    })
}

fn planner_action_from_execution(
    session: &RunControlSession,
    site: PlannerDecisionSite,
    candidate: &DecisionCandidate,
) -> Result<Option<PlannerAction>, String> {
    let CandidateAction::Execute(action) = &candidate.action else {
        return Ok(None);
    };
    let planner_action = match action {
        RunDecisionAction::SkipCardReward { reward_item_index } => {
            Some(PlannerAction::SkipCardReward {
                reward_item_index: *reward_item_index,
            })
        }
        RunDecisionAction::SingingBowlCardReward { reward_item_index } => {
            Some(PlannerAction::SingingBowl {
                reward_item_index: Some(*reward_item_index),
            })
        }
        RunDecisionAction::Input(input) => match input {
            ClientInput::SelectMapNode(x) => Some(PlannerAction::ChooseRouteNode {
                x: *x as i32,
                y: next_map_y(session),
                flight: false,
            }),
            ClientInput::FlyToNode(x, y) => Some(PlannerAction::ChooseRouteNode {
                x: *x as i32,
                y: *y as i32,
                flight: true,
            }),
            ClientInput::ClaimReward(reward_item_index) => {
                let reward = current_reward(session)
                    .and_then(|reward| reward.items.get(*reward_item_index))
                    .map(planner_reward_descriptor);
                reward.map(|reward| PlannerAction::ClaimReward {
                    reward_item_index: *reward_item_index,
                    reward,
                })
            }
            ClientInput::SelectCard(option_index) => current_reward(session)
                .and_then(|reward| {
                    reward
                        .pending_card_choice
                        .as_ref()
                        .map(|cards| (reward, cards))
                })
                .and_then(|(reward, cards)| {
                    cards
                        .get(*option_index)
                        .map(|card| PlannerAction::TakeCard {
                            reward_item_index: reward.pending_card_reward_index,
                            option_index: *option_index,
                            card: card.id,
                            upgrades: card.upgrades,
                        })
                }),
            ClientInput::CampfireOption(choice) => planner_campfire_action(session, *choice),
            ClientInput::SubmitSelection(resolution) => Some(PlannerAction::SubmitRunSelection {
                scope: resolution.scope,
                selected_card_uuids: resolution.selected_card_uuids(),
            }),
            ClientInput::OpenRewardOverlay => Some(PlannerAction::OpenPendingRewards),
            ClientInput::OpenChest => Some(PlannerAction::OpenChest),
            ClientInput::Proceed => Some(PlannerAction::Proceed { site }),
            ClientInput::Cancel => Some(PlannerAction::Cancel { site }),
            _ => None,
        },
    };
    Ok(planner_action)
}

fn planner_campfire_action(
    session: &RunControlSession,
    choice: CampfireChoice,
) -> Option<PlannerAction> {
    match choice {
        CampfireChoice::Rest => Some(PlannerAction::Rest),
        CampfireChoice::Smith(index) => {
            session
                .run_state
                .master_deck
                .get(index)
                .map(|card| PlannerAction::Smith {
                    card_uuid: card.uuid,
                    card: card.id,
                    upgrades: card.upgrades,
                })
        }
        CampfireChoice::Dig => Some(PlannerAction::Dig),
        CampfireChoice::Lift => Some(PlannerAction::Lift),
        CampfireChoice::Toke(index) => {
            session
                .run_state
                .master_deck
                .get(index)
                .map(|card| PlannerAction::Toke {
                    card_uuid: card.uuid,
                    card: card.id,
                    upgrades: card.upgrades,
                })
        }
        CampfireChoice::Recall => Some(PlannerAction::Recall),
    }
}

fn planner_site_and_context(
    session: &RunControlSession,
) -> Result<Option<(PlannerDecisionSite, PlannerDecisionContext)>, String> {
    let value = match &session.engine_state {
        EngineState::MapNavigation => Some((
            PlannerDecisionSite::Map,
            PlannerDecisionContext::Map { overlay: false },
        )),
        EngineState::MapOverlay { .. } => Some((
            PlannerDecisionSite::Map,
            PlannerDecisionContext::Map { overlay: true },
        )),
        EngineState::EventRoom => {
            let event = session
                .run_state
                .event_state
                .as_ref()
                .ok_or_else(|| "event boundary is missing public event state".to_string())?;
            let site = if event.id == crate::state::events::EventId::Neow {
                PlannerDecisionSite::Neow
            } else {
                PlannerDecisionSite::Event
            };
            Some((
                site,
                PlannerDecisionContext::Event {
                    event: event.id,
                    screen: event.current_screen,
                },
            ))
        }
        EngineState::RewardScreen(reward)
        | EngineState::RewardOverlay {
            reward_state: reward,
            ..
        } if reward.pending_card_choice.is_some() => Some((
            PlannerDecisionSite::CardReward,
            PlannerDecisionContext::CardReward {
                reward_item_index: reward.pending_card_reward_index,
            },
        )),
        EngineState::RewardScreen(_) | EngineState::RewardOverlay { .. } => {
            Some((PlannerDecisionSite::Reward, PlannerDecisionContext::Reward))
        }
        EngineState::TreasureRoom(_) => Some((
            PlannerDecisionSite::Treasure,
            PlannerDecisionContext::Treasure,
        )),
        EngineState::Campfire => Some((
            PlannerDecisionSite::Campfire,
            PlannerDecisionContext::Campfire,
        )),
        EngineState::Shop(shop) => Some((
            PlannerDecisionSite::Shop,
            PlannerDecisionContext::Shop {
                purge_cost: shop.purge_cost,
                purge_available: shop.purge_available,
            },
        )),
        EngineState::RunPendingChoice(_) => Some((
            PlannerDecisionSite::RunChoice,
            PlannerDecisionContext::RunChoice,
        )),
        EngineState::BossRelicSelect(_) => Some((
            PlannerDecisionSite::BossRelic,
            PlannerDecisionContext::BossRelic,
        )),
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::CombatStart(_)
        | EngineState::PendingChoice(_)
        | EngineState::GameOver(_) => None,
    };
    Ok(value)
}

fn planner_public_map(session: &RunControlSession) -> PlannerPublicMap {
    PlannerPublicMap {
        current_x: session.run_state.map.current_x,
        current_y: session.run_state.map.current_y,
        boss: session.run_state.boss_key,
        nodes: session
            .run_state
            .map
            .graph
            .iter()
            .flat_map(|row| row.iter())
            .map(|node| PlannerPublicMapNode {
                x: node.x,
                y: node.y,
                room: node.class,
                has_emerald_key: node.has_emerald_key,
                edges: node
                    .edges
                    .iter()
                    .map(|edge| PlannerPublicMapEdge {
                        destination_x: edge.dst_x,
                        destination_y: edge.dst_y,
                    })
                    .collect(),
            })
            .collect(),
    }
}

fn planner_player_class(player_class: &str) -> Result<PlannerPlayerClass, String> {
    match player_class {
        "Ironclad" => Ok(PlannerPlayerClass::Ironclad),
        "Silent" => Ok(PlannerPlayerClass::Silent),
        "Defect" => Ok(PlannerPlayerClass::Defect),
        "Watcher" => Ok(PlannerPlayerClass::Watcher),
        other => Err(format!("unsupported planner player class: {other}")),
    }
}

fn planner_mechanics() -> PlannerMechanicsManifest {
    PlannerMechanicsManifest {
        mechanics_id: PLANNER_MECHANICS_ID.to_string(),
        mechanics_version: PLANNER_MECHANICS_VERSION,
    }
}

fn planner_reward_descriptor(reward: &RewardItem) -> PlannerRewardDescriptor {
    match reward {
        RewardItem::Gold { amount } => PlannerRewardDescriptor::Gold { amount: *amount },
        RewardItem::StolenGold { amount } => {
            PlannerRewardDescriptor::StolenGold { amount: *amount }
        }
        RewardItem::Card { cards } => PlannerRewardDescriptor::CardReward {
            cards: cards
                .iter()
                .map(|card| PlannerOfferedCard {
                    card: card.id,
                    upgrades: card.upgrades,
                })
                .collect(),
        },
        RewardItem::Relic { relic_id } => PlannerRewardDescriptor::Relic { relic: *relic_id },
        RewardItem::Potion { potion_id } => PlannerRewardDescriptor::Potion { potion: *potion_id },
        RewardItem::EmeraldKey => PlannerRewardDescriptor::EmeraldKey,
        RewardItem::SapphireKey => PlannerRewardDescriptor::SapphireKey,
    }
}

fn current_reward(session: &RunControlSession) -> Option<&RewardState> {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => Some(reward),
        EngineState::RewardOverlay { reward_state, .. } => Some(reward_state),
        _ => None,
    }
}

fn current_shop(session: &RunControlSession) -> Option<&crate::state::shop::ShopState> {
    match &session.engine_state {
        EngineState::Shop(shop) => Some(shop),
        _ => None,
    }
}

fn next_map_y(session: &RunControlSession) -> i32 {
    if session.run_state.map.current_y == -1 {
        0
    } else {
        session.run_state.map.current_y + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::events::{EventId, EventState};

    #[test]
    fn hidden_rng_and_encounter_queues_do_not_change_public_capture_ids() {
        let mut left = RunControlSession::new(Default::default());
        left.engine_state = EngineState::EventRoom;
        left.run_state.event_state = Some(EventState::new(EventId::Neow));
        let mut right = left.clone();
        right.run_state.monster_list.reverse();
        right.run_state.elite_monster_list.reverse();
        right.run_state.boss_list.reverse();
        right.run_state.rng_pool.misc_rng.random(10_000);

        let left = capture_planner_boundary_v1(&left)
            .expect("capture left")
            .expect("planner boundary");
        let right = capture_planner_boundary_v1(&right)
            .expect("capture right")
            .expect("planner boundary");

        assert_eq!(left.visit_id, right.visit_id);
        assert_eq!(left.observation, right.observation);
        assert_eq!(left.legal_candidate_set, right.legal_candidate_set);
    }

    #[test]
    fn visible_candidates_are_represented_or_report_a_typed_gap() {
        let session = RunControlSession::new(Default::default());
        let pending = capture_planner_boundary_v1(&session)
            .expect("capture")
            .expect("initial Neow boundary");
        let visible = build_decision_surface(&session)
            .view
            .candidates
            .into_iter()
            .filter(|candidate| !matches!(candidate.action, CandidateAction::Unavailable { .. }))
            .count();
        let represented = pending.candidate_links.len();
        let gap_count = match &pending.legal_candidate_set.completeness {
            CandidateSetCompleteness::Complete { .. } => 0,
            CandidateSetCompleteness::Incomplete { gaps, .. } => gaps.len(),
        };

        assert!(represented == visible || gap_count > 0);
    }

    #[test]
    fn coverage_denominator_counts_yielded_visits_without_behavior_events() {
        let session = RunControlSession::new(Default::default());
        let visit = capture_planner_boundary_v1(&session)
            .expect("capture")
            .expect("planner boundary")
            .finish(PlannerBoundaryVisitOutcomeV1::Yielded {
                yield_kind: PlannerBoundaryYieldKindV1::ProgressBudgetExhausted,
            });
        let mut segment = PlannerBoundaryCaptureSegmentV1::default();
        segment.push(visit);

        let report = build_planner_boundary_capture_coverage_report_v1(&segment);

        assert_eq!(report.total_visits, 1);
        assert_eq!(report.yielded_visits, 1);
        assert_eq!(report.selected_visits, 0);
        assert_eq!(report.sites[0].visits, 1);
    }

    #[test]
    fn capture_ticket_rejects_a_transaction_from_another_boundary() {
        let session = RunControlSession::new(Default::default());
        let ticket = capture_planner_boundary_ticket_v1(&session)
            .expect("capture")
            .expect("planner boundary");
        let mut other = session.clone();
        other.decision_step += 1;
        let candidate_id = build_decision_surface(&other).view.candidates[0].id.clone();
        let outcome = other
            .apply_candidate_id(&candidate_id)
            .expect("other transaction");

        let segment = ticket.finish_for_progress(&outcome.progress_steps);

        assert!(matches!(
            segment.visits[0].outcome,
            PlannerBoundaryVisitOutcomeV1::ExecutionFailed
        ));
    }
}
