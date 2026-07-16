use crate::ai::combat_policy_v1::{
    select_forced_or_strictly_dominant_combat_action_v1, CombatScenarioActionPortfolioErrorV1,
    CombatScenarioActionPortfolioLimitsV1, CombatScenarioActionPortfolioSelectionGapV1,
    CombatScenarioBoundedWinProofLimitsV1,
};

use super::types::{
    CombatLabPolicyDecisionGapV1, CombatLabPublicPolicyDecisionV1, CombatLabPublicPolicyV1,
};
use crate::ai::combat_policy_v1::CombatPublicActionV1;

pub struct CombatLabOneStepDominancePolicyV1 {
    portfolio_limits: CombatScenarioActionPortfolioLimitsV1,
}

impl CombatLabOneStepDominancePolicyV1 {
    pub fn new(portfolio_limits: CombatScenarioActionPortfolioLimitsV1) -> Self {
        Self { portfolio_limits }
    }

    pub fn portfolio_limits(&self) -> CombatScenarioActionPortfolioLimitsV1 {
        self.portfolio_limits
    }
}

impl CombatLabPublicPolicyV1 for CombatLabOneStepDominancePolicyV1 {
    fn choose_action(
        &mut self,
        decision: CombatLabPublicPolicyDecisionV1<'_>,
    ) -> Result<CombatPublicActionV1, CombatLabPolicyDecisionGapV1> {
        let portfolio = decision
            .action_portfolio
            .evaluate(self.portfolio_limits)
            .map_err(map_portfolio_error)?;
        select_forced_or_strictly_dominant_combat_action_v1(&portfolio)
            .map(|selection| selection.action)
            .map_err(map_selection_gap)
    }
}

pub struct CombatLabBoundedWinProofPolicyV1 {
    portfolio_limits: CombatScenarioActionPortfolioLimitsV1,
    proof_limits: CombatScenarioBoundedWinProofLimitsV1,
}

impl CombatLabBoundedWinProofPolicyV1 {
    pub fn new(
        portfolio_limits: CombatScenarioActionPortfolioLimitsV1,
        proof_limits: CombatScenarioBoundedWinProofLimitsV1,
    ) -> Self {
        Self {
            portfolio_limits,
            proof_limits,
        }
    }

    pub fn portfolio_limits(&self) -> CombatScenarioActionPortfolioLimitsV1 {
        self.portfolio_limits
    }

    pub fn proof_limits(&self) -> CombatScenarioBoundedWinProofLimitsV1 {
        self.proof_limits
    }
}

impl CombatLabPublicPolicyV1 for CombatLabBoundedWinProofPolicyV1 {
    fn choose_action(
        &mut self,
        decision: CombatLabPublicPolicyDecisionV1<'_>,
    ) -> Result<CombatPublicActionV1, CombatLabPolicyDecisionGapV1> {
        let portfolio = decision
            .action_portfolio
            .evaluate(self.portfolio_limits)
            .map_err(map_portfolio_error)?;
        match select_forced_or_strictly_dominant_combat_action_v1(&portfolio) {
            Ok(selection) => Ok(selection.action),
            Err(CombatScenarioActionPortfolioSelectionGapV1::NoCandidates) => {
                Err(CombatLabPolicyDecisionGapV1::NoAcceptableAction)
            }
            Err(CombatScenarioActionPortfolioSelectionGapV1::NoStrictDominance) => decision
                .action_portfolio
                .prove_bounded_public_win(self.proof_limits)
                .map(|selection| selection.action)
                .map_err(|gap| CombatLabPolicyDecisionGapV1::BoundedWinProof { gap }),
        }
    }
}

fn map_portfolio_error(
    error: CombatScenarioActionPortfolioErrorV1,
) -> CombatLabPolicyDecisionGapV1 {
    match error {
        CombatScenarioActionPortfolioErrorV1::CandidateCountExceeds { .. } => {
            CombatLabPolicyDecisionGapV1::PortfolioTooLarge
        }
        CombatScenarioActionPortfolioErrorV1::InvalidLimit { .. }
        | CombatScenarioActionPortfolioErrorV1::ActionEvaluationFailed { .. } => {
            CombatLabPolicyDecisionGapV1::PortfolioEvaluationFailed
        }
    }
}

fn map_selection_gap(
    gap: CombatScenarioActionPortfolioSelectionGapV1,
) -> CombatLabPolicyDecisionGapV1 {
    match gap {
        CombatScenarioActionPortfolioSelectionGapV1::NoCandidates => {
            CombatLabPolicyDecisionGapV1::NoAcceptableAction
        }
        CombatScenarioActionPortfolioSelectionGapV1::NoStrictDominance => {
            CombatLabPolicyDecisionGapV1::NoStrictDominance
        }
    }
}
