use serde::{Deserialize, Serialize};

use super::{
    CampfireEvaluationContext, CampfireEvidenceLimitation, CampfireEvidenceProvenance,
    CampfireEvidenceStatus, CampfireFieldEvidence, CampfireProspectField, CampfireRunGoal,
};
use crate::engine::campfire_candidates::CampfireCandidate;
use crate::eval::campfire_projection::CampfireProjection;
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampfireRubyKeyObligation {
    NotRequired,
    AlreadySatisfied,
    SatisfiedByCandidate,
    DeferrableOnEveryVisiblePath,
    DeferrableOnSomeVisiblePath,
    ViolatedAtVisibleAct3BossDeadline,
    UnresolvedBeyondVisibleWindow,
    UnresolvedByChanceOutcome,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampfireRunFeasibility {
    pub declared_goal: CampfireRunGoal,
    pub ruby_key: CampfireRubyKeyObligation,
    pub sapphire_key_held: bool,
    pub emerald_key_held: bool,
}

pub(super) struct CampfireRunFeasibilityAssessment {
    pub value: CampfireRunFeasibility,
    pub evidence: CampfireFieldEvidence,
}

pub(super) fn assess_run_feasibility(
    root: &RunState,
    context: &CampfireEvaluationContext,
    candidate: CampfireCandidate,
    projection: &CampfireProjection,
) -> CampfireRunFeasibilityAssessment {
    let goal = context.spec.run_goal;
    let sapphire_key_held = root.keys[1];
    let emerald_key_held = root.keys[2];
    let (ruby_key, evidence) = match goal {
        CampfireRunGoal::Act3Victory => (
            CampfireRubyKeyObligation::NotRequired,
            exact_evidence(CampfireEvidenceProvenance::PublicRootObservation),
        ),
        CampfireRunGoal::HeartEligibility | CampfireRunGoal::HeartVictory => {
            if root.keys[0] {
                (
                    CampfireRubyKeyObligation::AlreadySatisfied,
                    satisfied_heart_evidence(
                        sapphire_key_held,
                        emerald_key_held,
                        CampfireEvidenceProvenance::PublicRootObservation,
                    ),
                )
            } else if candidate == CampfireCandidate::Recall && projected_ruby_key(projection) {
                (
                    CampfireRubyKeyObligation::SatisfiedByCandidate,
                    satisfied_heart_evidence(
                        sapphire_key_held,
                        emerald_key_held,
                        CampfireEvidenceProvenance::EngineTransition,
                    ),
                )
            } else {
                (
                    CampfireRubyKeyObligation::UnresolvedBeyondVisibleWindow,
                    partial_evidence(
                        CampfireEvidenceProvenance::NoProducer,
                        vec![
                            CampfireEvidenceLimitation::VisibleDeadlineNotProven,
                            CampfireEvidenceLimitation::OtherHeartKeysNotEvaluated,
                        ],
                    ),
                )
            }
        }
    };

    CampfireRunFeasibilityAssessment {
        value: CampfireRunFeasibility {
            declared_goal: goal,
            ruby_key,
            sapphire_key_held,
            emerald_key_held,
        },
        evidence,
    }
}

fn projected_ruby_key(projection: &CampfireProjection) -> bool {
    matches!(
        projection,
        CampfireProjection::Exact(exact) if exact.run_state.keys[0]
    )
}

fn satisfied_heart_evidence(
    sapphire_key_held: bool,
    emerald_key_held: bool,
    provenance: CampfireEvidenceProvenance,
) -> CampfireFieldEvidence {
    if sapphire_key_held && emerald_key_held {
        exact_evidence(provenance)
    } else {
        partial_evidence(
            provenance,
            vec![CampfireEvidenceLimitation::OtherHeartKeysNotEvaluated],
        )
    }
}

fn exact_evidence(provenance: CampfireEvidenceProvenance) -> CampfireFieldEvidence {
    CampfireFieldEvidence {
        field: CampfireProspectField::RunFeasibility,
        status: CampfireEvidenceStatus::Exact,
        provenance,
        limitations: Vec::new(),
    }
}

fn partial_evidence(
    provenance: CampfireEvidenceProvenance,
    limitations: Vec<CampfireEvidenceLimitation>,
) -> CampfireFieldEvidence {
    CampfireFieldEvidence {
        field: CampfireProspectField::RunFeasibility,
        status: CampfireEvidenceStatus::Partial,
        provenance,
        limitations,
    }
}
