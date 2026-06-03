use crate::ai::card_reward_policy_v1::{
    CardRewardCandidateScoreV1, CardRewardDecisionV1, CardRewardPolicyActionV1,
    CardRewardScoreTermsV1,
};
use crate::ai::route_planner_v1::{
    NeedVectorV1, RouteCandidateTraceV1, RouteDecisionTraceV1, RouteSafetyFlagV1, RouteScoreTermsV1,
};

use super::types::{
    CandidateDescriptorV1, DataRoleV1, DecisionSiteKindV1, EvidenceBundleV1, EvidenceItemV1,
    EvidenceKindV1, InformationBoundaryV1, InformationClassV1, NonCombatDecisionRecordV1,
    PolicyProvenanceV1, PolicySelectionStatusV1, PolicySelectionV1, PublicActionPlanV1,
    ValueComponentV1, ValueEstimateV1, NONCOMBAT_DECISION_RECORD_SCHEMA_NAME,
    NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
};

impl RouteDecisionTraceV1 {
    pub fn to_noncombat_decision_record_v1(&self) -> NonCombatDecisionRecordV1 {
        let candidate_ids = self
            .candidates
            .iter()
            .map(route_candidate_id)
            .collect::<Vec<_>>();
        let candidates = self
            .candidates
            .iter()
            .zip(candidate_ids.iter())
            .map(|(candidate, id)| route_candidate_descriptor(candidate, id))
            .collect::<Vec<_>>();
        let evidence_items = self
            .candidates
            .iter()
            .zip(candidate_ids.iter())
            .flat_map(|(candidate, id)| route_evidence_items(candidate, id))
            .collect::<Vec<_>>();
        let values = self
            .candidates
            .iter()
            .zip(candidate_ids.iter())
            .enumerate()
            .map(|(idx, (candidate, id))| route_value_estimate(candidate, id, idx * 2))
            .collect::<Vec<_>>();
        let selected_candidate_id = self
            .selected_index
            .and_then(|idx| candidate_ids.get(idx))
            .cloned();
        let selection = PolicySelectionV1 {
            status: if selected_candidate_id.is_some() {
                PolicySelectionStatusV1::Selected
            } else if self.candidates.is_empty() {
                PolicySelectionStatusV1::NoCandidates
            } else {
                PolicySelectionStatusV1::Stopped
            },
            selected_candidate_id,
            reason: route_selection_reason(self),
            confidence: route_selection_confidence(self),
            selection_mode: format!("{:?}", self.selection_mode),
        };

        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::Map,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(vec![
                InformationClassV1::PublicObservation,
                InformationClassV1::KnownDistribution,
                InformationClassV1::Belief,
            ]),
            provenance: PolicyProvenanceV1 {
                source_policy: "route_planner_v1".to_string(),
                source_schema_name: self.schema_name.clone(),
                source_schema_version: self.schema_version,
            },
            candidates,
            evidence: EvidenceBundleV1 {
                items: evidence_items,
                assumptions: vec![
                    "route candidates are scored from visible map structure and configured route policy weights"
                        .to_string(),
                    "unknown rooms are represented as distribution/belief evidence, not resolved hidden outcomes"
                        .to_string(),
                ],
                warnings: self.warnings.clone(),
            },
            values,
            selection,
        }
    }
}

impl CardRewardDecisionV1 {
    pub fn to_noncombat_decision_record_v1(&self) -> NonCombatDecisionRecordV1 {
        let candidate_ids = self
            .candidates
            .iter()
            .map(card_reward_candidate_id)
            .collect::<Vec<_>>();
        let candidates = self
            .candidates
            .iter()
            .zip(candidate_ids.iter())
            .map(|(candidate, id)| card_reward_candidate_descriptor(candidate, id))
            .collect::<Vec<_>>();
        let evidence_items = self
            .candidates
            .iter()
            .zip(candidate_ids.iter())
            .map(|(candidate, id)| card_reward_evidence_item(candidate, id))
            .collect::<Vec<_>>();
        let values = self
            .candidates
            .iter()
            .zip(candidate_ids.iter())
            .enumerate()
            .map(|(idx, (candidate, id))| card_reward_value_estimate(candidate, id, idx))
            .collect::<Vec<_>>();
        let selection = card_reward_selection(self);

        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::CardReward,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(vec![
                InformationClassV1::PublicObservation,
            ]),
            provenance: PolicyProvenanceV1 {
                source_policy: "card_reward_policy_v1".to_string(),
                source_schema_name: "CardRewardPolicyV1".to_string(),
                source_schema_version: 1,
            },
            candidates,
            evidence: EvidenceBundleV1 {
                items: evidence_items,
                assumptions: vec![
                    "visible reward cards are public observations after the card reward is opened"
                        .to_string(),
                    "score terms are behavior-policy evidence, not a teacher label".to_string(),
                ],
                warnings: Vec::new(),
            },
            values,
            selection,
        }
    }
}

fn route_candidate_id(candidate: &RouteCandidateTraceV1) -> String {
    format!(
        "map:{:?}:x{}:y{}",
        candidate.target.move_kind, candidate.target.x, candidate.target.y
    )
}

fn route_candidate_descriptor(
    candidate: &RouteCandidateTraceV1,
    id: &str,
) -> CandidateDescriptorV1 {
    CandidateDescriptorV1 {
        candidate_id: id.to_string(),
        site: DecisionSiteKindV1::Map,
        label: format!(
            "Route to x={} y={} {:?}",
            candidate.target.x, candidate.target.y, candidate.target.room_type
        ),
        action_plan: PublicActionPlanV1 {
            summary: candidate
                .suggested_command
                .clone()
                .unwrap_or_else(|| "map move unavailable on current screen".to_string()),
            command: candidate.suggested_command.clone(),
        },
        information_classes: vec![
            InformationClassV1::PublicObservation,
            InformationClassV1::KnownDistribution,
            InformationClassV1::Belief,
        ],
        uncertainty_notes: route_uncertainty_notes(candidate),
    }
}

fn route_uncertainty_notes(candidate: &RouteCandidateTraceV1) -> Vec<String> {
    let mut notes = Vec::new();
    if candidate.features.is_question_mark {
        notes.push("unknown room outcome modeled as belief/distribution".to_string());
    }
    if candidate.safety == RouteSafetyFlagV1::RejectUnlessNoAlternative {
        notes.push("route rejected unless forced by safety gate".to_string());
    }
    notes.extend(candidate.cautions.iter().cloned());
    notes
}

fn route_evidence_items(candidate: &RouteCandidateTraceV1, id: &str) -> Vec<EvidenceItemV1> {
    vec![
        EvidenceItemV1 {
            kind: EvidenceKindV1::NeedVector,
            candidate_id: Some(id.to_string()),
            label: "route needs".to_string(),
            information_class: InformationClassV1::PublicObservation,
            components: need_components(&candidate.needs),
        },
        EvidenceItemV1 {
            kind: EvidenceKindV1::ScoreTerms,
            candidate_id: Some(id.to_string()),
            label: "route score terms".to_string(),
            information_class: InformationClassV1::Belief,
            components: route_score_components(&candidate.score_terms),
        },
    ]
}

fn route_value_estimate(
    candidate: &RouteCandidateTraceV1,
    id: &str,
    first_evidence_ref: usize,
) -> ValueEstimateV1 {
    let confidence = match candidate.safety {
        RouteSafetyFlagV1::Ok => 0.75,
        RouteSafetyFlagV1::RiskyButAllowed => 0.55,
        RouteSafetyFlagV1::RejectUnlessNoAlternative => 0.25,
    };
    ValueEstimateV1 {
        candidate_id: id.to_string(),
        mean_utility: candidate.total_score,
        risk_adjusted_utility: candidate.total_score,
        confidence,
        components: route_score_components(&candidate.score_terms),
        evidence_refs: vec![first_evidence_ref, first_evidence_ref + 1],
    }
}

fn route_selection_reason(trace: &RouteDecisionTraceV1) -> String {
    match trace.selected_index {
        Some(idx) => format!(
            "selected route candidate rank {idx} under {:?}",
            trace.selection_mode
        ),
        None if trace.candidates.is_empty() => "no route candidates available".to_string(),
        None => "route planner stopped without executable selection".to_string(),
    }
}

fn route_selection_confidence(trace: &RouteDecisionTraceV1) -> f32 {
    trace
        .selected_index
        .and_then(|idx| trace.candidates.get(idx))
        .map(|candidate| match candidate.safety {
            RouteSafetyFlagV1::Ok => 0.75,
            RouteSafetyFlagV1::RiskyButAllowed => 0.55,
            RouteSafetyFlagV1::RejectUnlessNoAlternative => 0.25,
        })
        .unwrap_or(0.0)
}

fn need_components(needs: &NeedVectorV1) -> Vec<ValueComponentV1> {
    vec![
        ValueComponentV1::new("need_card_rewards", needs.need_card_rewards),
        ValueComponentV1::new("need_relics", needs.need_relics),
        ValueComponentV1::new("need_remove", needs.need_remove),
        ValueComponentV1::new("need_upgrade", needs.need_upgrade),
        ValueComponentV1::new("need_heal", needs.need_heal),
        ValueComponentV1::new("need_shop", needs.need_shop),
        ValueComponentV1::new("need_event", needs.need_event),
        ValueComponentV1::new("need_potion", needs.need_potion),
        ValueComponentV1::new("can_take_elite", needs.can_take_elite),
        ValueComponentV1::new("avoid_damage", needs.avoid_damage),
        ValueComponentV1::new("value_flexibility", needs.value_flexibility),
    ]
}

fn route_score_components(terms: &RouteScoreTermsV1) -> Vec<ValueComponentV1> {
    vec![
        ValueComponentV1::new("card_reward", terms.card_reward),
        ValueComponentV1::new("relic", terms.relic),
        ValueComponentV1::new("remove", terms.remove),
        ValueComponentV1::new("upgrade", terms.upgrade),
        ValueComponentV1::new("heal", terms.heal),
        ValueComponentV1::new("shop", terms.shop),
        ValueComponentV1::new("event", terms.event),
        ValueComponentV1::new("potion", terms.potion),
        ValueComponentV1::new("hp_loss", terms.hp_loss),
        ValueComponentV1::new("death_risk", terms.death_risk),
        ValueComponentV1::new("flexibility", terms.flexibility),
        ValueComponentV1::new("wing_boots_cost", terms.wing_boots_cost),
        ValueComponentV1::new("forced_path_penalty", terms.forced_path_penalty),
        ValueComponentV1::new("burning_elite_key_value", terms.burning_elite_key_value),
    ]
}

fn card_reward_candidate_id(candidate: &CardRewardCandidateScoreV1) -> String {
    format!("card_reward:{}:{:?}", candidate.index, candidate.card)
}

fn card_reward_candidate_descriptor(
    candidate: &CardRewardCandidateScoreV1,
    id: &str,
) -> CandidateDescriptorV1 {
    CandidateDescriptorV1 {
        candidate_id: id.to_string(),
        site: DecisionSiteKindV1::CardReward,
        label: candidate.name.to_string(),
        action_plan: PublicActionPlanV1 {
            summary: format!("pick visible card reward {}", candidate.name),
            command: Some(format!("{}", candidate.index)),
        },
        information_classes: vec![InformationClassV1::PublicObservation],
        uncertainty_notes: candidate
            .notes
            .iter()
            .map(|note| (*note).to_string())
            .collect(),
    }
}

fn card_reward_evidence_item(candidate: &CardRewardCandidateScoreV1, id: &str) -> EvidenceItemV1 {
    EvidenceItemV1 {
        kind: EvidenceKindV1::ScoreTerms,
        candidate_id: Some(id.to_string()),
        label: format!("card reward score for {}", candidate.name),
        information_class: InformationClassV1::PublicObservation,
        components: card_reward_score_components(&candidate.terms),
    }
}

fn card_reward_value_estimate(
    candidate: &CardRewardCandidateScoreV1,
    id: &str,
    evidence_ref: usize,
) -> ValueEstimateV1 {
    ValueEstimateV1 {
        candidate_id: id.to_string(),
        mean_utility: candidate.score,
        risk_adjusted_utility: candidate.score,
        confidence: 0.65,
        components: card_reward_score_components(&candidate.terms),
        evidence_refs: vec![evidence_ref],
    }
}

fn card_reward_selection(decision: &CardRewardDecisionV1) -> PolicySelectionV1 {
    match &decision.action {
        CardRewardPolicyActionV1::Pick {
            index,
            card,
            confidence,
            reason,
        } => PolicySelectionV1 {
            status: PolicySelectionStatusV1::Selected,
            selected_candidate_id: Some(format!("card_reward:{index}:{card:?}")),
            reason: reason.clone(),
            confidence: *confidence,
            selection_mode: "score_and_margin_gate".to_string(),
        },
        CardRewardPolicyActionV1::Stop { reason } => PolicySelectionV1 {
            status: if decision.candidates.is_empty() {
                PolicySelectionStatusV1::NoCandidates
            } else {
                PolicySelectionStatusV1::Stopped
            },
            selected_candidate_id: None,
            reason: reason.clone(),
            confidence: 0.0,
            selection_mode: "score_and_margin_gate".to_string(),
        },
    }
}

fn card_reward_score_components(terms: &CardRewardScoreTermsV1) -> Vec<ValueComponentV1> {
    vec![
        ValueComponentV1::new("frontload", terms.frontload),
        ValueComponentV1::new("block", terms.block),
        ValueComponentV1::new("draw", terms.draw),
        ValueComponentV1::new("scaling", terms.scaling),
        ValueComponentV1::new("aoe", terms.aoe),
        ValueComponentV1::new("exhaust_synergy", terms.exhaust_synergy),
        ValueComponentV1::new("rarity", terms.rarity),
        ValueComponentV1::new("premium", terms.premium),
        ValueComponentV1::new("risk", terms.risk),
        ValueComponentV1::new("bloat", terms.bloat),
    ]
}
