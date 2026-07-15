use crate::ai::upgrade_planner_v1::{
    upgrade_candidate_for_card_uuid_v1, UpgradeDebtKindV1, UpgradeDebtSeverityV1,
    UpgradeMechanicalDeltaV1, UpgradeRoleV1, UpgradeVerdictV1,
};
use crate::content::cards::CardId;
use crate::engine::campfire_candidates::CampfireCandidate;
use crate::eval::campfire_projection::CampfireProjection;
use crate::state::run::RunState;

use super::{
    unsupported_evidence, with_future_limitations, CampfireEvidenceLimitation,
    CampfireEvidenceProvenance, CampfireEvidenceStatus, CampfireFieldEvidence,
    CampfireProspectField,
};

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CampfireGrowth {
    pub smith: Option<CampfireSmithGrowth>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CampfireSmithGrowth {
    pub card_uuid: u32,
    pub card: CardId,
    pub upgrades_before: u8,
    pub upgrades_after: u8,
    pub mechanical_delta: UpgradeMechanicalDeltaV1,
    pub roles: Vec<UpgradeRoleV1>,
    pub pays_debts: Vec<UpgradeDebtKindV1>,
    pub opportunity_costs: Vec<String>,
    pub urgency: UpgradeDebtSeverityV1,
    pub verdict: UpgradeVerdictV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CampfireGrowthError {
    MissingUpgradeCandidate {
        card_uuid: u32,
    },
    ExpectedExactProjection {
        candidate: CampfireCandidate,
    },
    MissingProjectedCard {
        card_uuid: u32,
    },
    AmbiguousProjectedCard {
        card_uuid: u32,
    },
    UpgradeCountMismatch {
        card_uuid: u32,
        before: u8,
        after: u8,
    },
}

pub(super) struct CampfireGrowthAssessment {
    pub value: CampfireGrowth,
    pub evidence: CampfireFieldEvidence,
}

pub(super) fn assess_growth(
    root: &RunState,
    candidate: CampfireCandidate,
    projection: &CampfireProjection,
) -> Result<CampfireGrowthAssessment, CampfireGrowthError> {
    let CampfireCandidate::Smith { card_uuid } = candidate else {
        return Ok(unsupported_growth(projection));
    };
    let CampfireProjection::Exact(exact) = projection else {
        return Err(CampfireGrowthError::ExpectedExactProjection { candidate });
    };
    let planner = upgrade_candidate_for_card_uuid_v1(root, card_uuid)
        .ok_or(CampfireGrowthError::MissingUpgradeCandidate { card_uuid })?;

    let mut projected_cards = exact
        .run_state
        .master_deck
        .iter()
        .filter(|card| card.uuid == card_uuid);
    let projected = projected_cards
        .next()
        .ok_or(CampfireGrowthError::MissingProjectedCard { card_uuid })?;
    if projected_cards.next().is_some() {
        return Err(CampfireGrowthError::AmbiguousProjectedCard { card_uuid });
    }
    let expected_after = planner.upgrades.saturating_add(1);
    if projected.upgrades != expected_after {
        return Err(CampfireGrowthError::UpgradeCountMismatch {
            card_uuid,
            before: planner.upgrades,
            after: projected.upgrades,
        });
    }

    Ok(CampfireGrowthAssessment {
        value: CampfireGrowth {
            smith: Some(CampfireSmithGrowth {
                card_uuid,
                card: planner.card,
                upgrades_before: planner.upgrades,
                upgrades_after: projected.upgrades,
                mechanical_delta: planner.mechanical_delta,
                roles: planner.roles,
                pays_debts: planner.pays_debts,
                opportunity_costs: planner.opportunity_costs,
                urgency: planner.urgency,
                verdict: planner.verdict,
            }),
        },
        evidence: CampfireFieldEvidence {
            field: CampfireProspectField::GrowthDistribution,
            status: CampfireEvidenceStatus::Partial,
            provenance: CampfireEvidenceProvenance::EngineTransitionAndUpgradePlanner,
            limitations: vec![CampfireEvidenceLimitation::DownstreamGrowthDistributionNotEvaluated],
        },
    })
}

fn unsupported_growth(projection: &CampfireProjection) -> CampfireGrowthAssessment {
    let future_limitations = match projection {
        CampfireProjection::Exact(_) => Vec::new(),
        CampfireProjection::Chance(_) => {
            vec![CampfireEvidenceLimitation::ChanceOutcomeNotEstimated]
        }
        CampfireProjection::ChanceThenDecision(_) => vec![
            CampfireEvidenceLimitation::ChanceOutcomeNotEstimated,
            CampfireEvidenceLimitation::PostRevealRecourseNotEvaluated,
        ],
    };

    CampfireGrowthAssessment {
        value: CampfireGrowth::default(),
        evidence: unsupported_evidence(
            CampfireProspectField::GrowthDistribution,
            with_future_limitations(
                CampfireEvidenceLimitation::GrowthNotEvaluated,
                &future_limitations,
            ),
        ),
    }
}
