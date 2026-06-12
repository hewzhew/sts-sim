use crate::ai::card_reward_policy_v1::{
    CardRewardCandidateEvidenceV1, CardRewardDecisionV1, CardRewardPolicyActionV1,
    CardRewardValueEstimateV1, CardRewardValueSourceV1, CardRewardValueStatusV1,
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
            .value_arbitration
            .gate_value_estimates
            .iter()
            .map(card_reward_value_estimate)
            .collect::<Vec<_>>();
        let selection = card_reward_selection(self);
        let mut allowed_inputs = vec![InformationClassV1::PublicObservation];
        if self.context.route.is_some() {
            allowed_inputs.push(InformationClassV1::KnownDistribution);
            allowed_inputs.push(InformationClassV1::Belief);
        }

        NonCombatDecisionRecordV1 {
            schema_name: NONCOMBAT_DECISION_RECORD_SCHEMA_NAME.to_string(),
            schema_version: NONCOMBAT_DECISION_RECORD_SCHEMA_VERSION,
            site: DecisionSiteKindV1::CardReward,
            data_role: DataRoleV1::BehaviorPolicyNotTeacher,
            information_boundary: InformationBoundaryV1::hidden_free(allowed_inputs),
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
                    "card reward policy records mechanical facts and evidence gaps, not an optimal action label".to_string(),
                    "automatic selection may use either the strict calibrated value gate or the behavior autopick gate; both remain behavior_policy_not_teacher".to_string(),
                ],
                warnings: self
                    .evidence_gaps
                    .iter()
                    .map(|gap| format!("{gap:?}"))
                    .collect(),
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

fn card_reward_candidate_id(candidate: &CardRewardCandidateEvidenceV1) -> String {
    format!("card_reward:{}:{:?}", candidate.index, candidate.card)
}

fn card_reward_candidate_descriptor(
    candidate: &CardRewardCandidateEvidenceV1,
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
        uncertainty_notes: card_reward_uncertainty_notes(candidate),
    }
}

fn card_reward_uncertainty_notes(candidate: &CardRewardCandidateEvidenceV1) -> Vec<String> {
    let mut notes = candidate.impact.evidence_notes.clone();
    notes.extend(
        candidate
            .plan_delta
            .effects
            .iter()
            .map(|effect| format!("plan effect: {effect:?}")),
    );
    notes.extend(candidate.plan_delta.notes.clone());
    notes.extend(
        candidate
            .impact
            .approval_blockers
            .iter()
            .map(|gap| format!("autopilot blocker: {gap:?}")),
    );
    notes
}

fn card_reward_evidence_item(
    candidate: &CardRewardCandidateEvidenceV1,
    id: &str,
) -> EvidenceItemV1 {
    EvidenceItemV1 {
        kind: EvidenceKindV1::CandidateFacts,
        candidate_id: Some(id.to_string()),
        label: format!("card reward facts for {}", candidate.name),
        information_class: InformationClassV1::PublicObservation,
        components: card_reward_fact_components(candidate),
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
            selection_mode: decision
                .decision_approval
                .as_ref()
                .map(|approval| approval.selection_mode)
                .unwrap_or("card_reward_policy_pick")
                .to_string(),
        },
        CardRewardPolicyActionV1::Stop { reason, .. } => PolicySelectionV1 {
            status: if decision.candidates.is_empty() {
                PolicySelectionStatusV1::NoCandidates
            } else {
                PolicySelectionStatusV1::Stopped
            },
            selected_candidate_id: None,
            reason: reason.clone(),
            confidence: 0.0,
            selection_mode: "autopilot_value_gate".to_string(),
        },
    }
}

fn card_reward_fact_components(candidate: &CardRewardCandidateEvidenceV1) -> Vec<ValueComponentV1> {
    let mut components = vec![
        ValueComponentV1::new(
            "frontload_damage_delta",
            candidate.impact.frontload_damage_delta as f32,
        ),
        ValueComponentV1::new("block_delta", candidate.impact.block_delta as f32),
        ValueComponentV1::new("draw_delta", candidate.impact.draw_delta as f32),
        ValueComponentV1::new("energy_delta", candidate.impact.energy_delta as f32),
        ValueComponentV1::new("vulnerable", candidate.facts.vulnerable as f32),
        ValueComponentV1::new("weak", candidate.facts.weak as f32),
        ValueComponentV1::new("strength_gain", candidate.facts.strength_gain as f32),
        ValueComponentV1::new(
            "enemy_strength_down",
            candidate.facts.enemy_strength_down as f32,
        ),
        ValueComponentV1::new(
            "approval_blockers",
            candidate.impact.approval_blockers.len() as f32,
        ),
        ValueComponentV1::new(
            format!("plan_support_{:?}", candidate.plan_delta.support),
            1.0,
        ),
    ];
    components.extend(
        candidate
            .plan_delta
            .effects
            .iter()
            .map(|effect| ValueComponentV1::new(format!("plan_effect_{effect:?}"), 1.0)),
    );
    components
}

fn card_reward_value_estimate(estimate: &CardRewardValueEstimateV1) -> ValueEstimateV1 {
    ValueEstimateV1 {
        candidate_id: format!("card_reward:{}:{:?}", estimate.index, estimate.card),
        mean_utility: 0.0,
        risk_adjusted_utility: 0.0,
        confidence: match estimate.status {
            CardRewardValueStatusV1::UncalibratedPrior => 0.0,
            CardRewardValueStatusV1::StrategyPackageEstimate => 0.25,
            CardRewardValueStatusV1::StrategyPackageCalibrated => 0.35,
            CardRewardValueStatusV1::PublicCombatHeuristic => 0.30,
            CardRewardValueStatusV1::CounterfactualProbe => 0.5,
            CardRewardValueStatusV1::OutcomeCalibrated => 0.75,
            CardRewardValueStatusV1::RouteRiskEstimate => 0.35,
            CardRewardValueStatusV1::RouteRiskCalibrated => 0.45,
        },
        components: card_reward_value_components(estimate),
        evidence_refs: vec![estimate.index],
    }
}

fn card_reward_value_components(estimate: &CardRewardValueEstimateV1) -> Vec<ValueComponentV1> {
    let mut components = estimate
        .components
        .iter()
        .map(|component| ValueComponentV1::new(component.name.clone(), component.value))
        .collect::<Vec<_>>();
    components.push(ValueComponentV1::new(
        match estimate.source {
            CardRewardValueSourceV1::UncalibratedImpactPrior => {
                "value_source_uncalibrated_impact_prior"
            }
            CardRewardValueSourceV1::StrategyPackage => "value_source_strategy_package",
            CardRewardValueSourceV1::OutcomeCalibration => "value_source_outcome_calibration",
            CardRewardValueSourceV1::PublicCombatHeuristic => {
                "value_source_public_combat_heuristic"
            }
            CardRewardValueSourceV1::CombatProbe => "value_source_combat_probe",
            CardRewardValueSourceV1::RouteRisk => "value_source_route_risk",
            CardRewardValueSourceV1::LearnedValue => "value_source_learned_value",
        },
        1.0,
    ));
    components.push(ValueComponentV1::new(
        match estimate.status {
            CardRewardValueStatusV1::UncalibratedPrior => "value_status_uncalibrated_prior",
            CardRewardValueStatusV1::StrategyPackageEstimate => {
                "value_status_strategy_package_estimate"
            }
            CardRewardValueStatusV1::StrategyPackageCalibrated => {
                "value_status_strategy_package_calibrated"
            }
            CardRewardValueStatusV1::PublicCombatHeuristic => {
                "value_status_public_combat_heuristic"
            }
            CardRewardValueStatusV1::CounterfactualProbe => "value_status_counterfactual_probe",
            CardRewardValueStatusV1::OutcomeCalibrated => "value_status_outcome_calibrated",
            CardRewardValueStatusV1::RouteRiskEstimate => "value_status_route_risk_estimate",
            CardRewardValueStatusV1::RouteRiskCalibrated => "value_status_route_risk_calibrated",
        },
        1.0,
    ));
    components.push(ValueComponentV1::new(
        "value_usable_for_value_estimate",
        if estimate.eligibility.usable_for_value_estimate {
            1.0
        } else {
            0.0
        },
    ));
    components.push(ValueComponentV1::new(
        "value_usable_for_autopilot_gate",
        if estimate.eligibility.usable_for_autopilot_gate {
            1.0
        } else {
            0.0
        },
    ));
    components.extend(
        estimate.eligibility.reasons.iter().map(|reason| {
            ValueComponentV1::new(format!("value_eligibility_reason_{reason:?}"), 1.0)
        }),
    );
    components
}
