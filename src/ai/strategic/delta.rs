use super::{CandidateAction, PressureKind};
use crate::ai::card_component_marginal_value_v1::{
    CardComponentMarginalReportV1, CardComponentMarginalVerdictV1,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CandidateRole {
    Enabler,
    Payoff,
    Lubricant,
    Transition,
    BossAnswer,
    DeckCleaning,
    ResourceConversion,
    DefensivePatch,
    Unknown,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum VerdictHint {
    Reject,
    SkipPreferred,
    Speculative,
    ContextTake,
    StrongTake,
    MustTake,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct LedgerDelta {
    pub kind: PressureKind,
    pub amount: f32,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct OpportunityCost {
    pub label: String,
    pub severity: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct StrategicContraindication {
    pub label: String,
    pub severity: f32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CandidateDelta {
    pub action: CandidateAction,
    pub positive: Vec<LedgerDelta>,
    pub negative: Vec<LedgerDelta>,
    pub role: CandidateRole,
    pub verdict_hint: VerdictHint,
    pub opportunity_costs: Vec<OpportunityCost>,
    pub contraindications: Vec<StrategicContraindication>,
    pub notes: Vec<String>,
    pub evidence: Vec<String>,
}

impl CandidateDelta {
    pub fn empty(action: CandidateAction) -> Self {
        Self {
            action,
            positive: Vec::new(),
            negative: Vec::new(),
            role: CandidateRole::Unknown,
            verdict_hint: VerdictHint::Speculative,
            opportunity_costs: Vec::new(),
            contraindications: Vec::new(),
            notes: Vec::new(),
            evidence: Vec::new(),
        }
    }

    pub fn from_component_report(
        action: CandidateAction,
        report: &CardComponentMarginalReportV1,
    ) -> Self {
        let mut delta = Self::empty(action);
        delta.role = component_role(report);
        delta.verdict_hint = component_verdict(report.verdict);
        delta.positive = report
            .positive_components
            .iter()
            .map(|reason| LedgerDelta {
                kind: component_reason_pressure(reason),
                amount: 0.35,
                reason: (*reason).to_string(),
            })
            .collect();
        delta.negative = report
            .debts
            .iter()
            .chain(report.boss_taxes.iter())
            .map(|reason| LedgerDelta {
                kind: component_reason_pressure(reason),
                amount: 0.35,
                reason: (*reason).to_string(),
            })
            .collect();
        delta.notes = report
            .notes
            .iter()
            .map(|note| (*note).to_string())
            .collect();
        delta
            .evidence
            .push("card_component_marginal_value contributor".to_string());
        delta
    }

    pub fn positive_amount(&self) -> f32 {
        self.positive.iter().map(|delta| delta.amount).sum()
    }

    pub fn negative_amount(&self) -> f32 {
        self.negative.iter().map(|delta| delta.amount).sum::<f32>()
            + self
                .opportunity_costs
                .iter()
                .map(|cost| cost.severity)
                .sum::<f32>()
            + self
                .contraindications
                .iter()
                .map(|contraindication| contraindication.severity * 2.0)
                .sum::<f32>()
    }
}

fn component_verdict(verdict: CardComponentMarginalVerdictV1) -> VerdictHint {
    match verdict {
        CardComponentMarginalVerdictV1::Reject => VerdictHint::Reject,
        CardComponentMarginalVerdictV1::SkipPreferred => VerdictHint::SkipPreferred,
        CardComponentMarginalVerdictV1::Speculative => VerdictHint::Speculative,
        CardComponentMarginalVerdictV1::ContextTake => VerdictHint::ContextTake,
        CardComponentMarginalVerdictV1::StrongTake => VerdictHint::StrongTake,
        CardComponentMarginalVerdictV1::MustTake => VerdictHint::MustTake,
    }
}

fn component_role(report: &CardComponentMarginalReportV1) -> CandidateRole {
    use crate::ai::card_component_marginal_value_v1::CardComponentRoleV1;
    if report.roles.contains(&CardComponentRoleV1::BossAnswer) {
        CandidateRole::BossAnswer
    } else if report.roles.contains(&CardComponentRoleV1::Enabler) {
        CandidateRole::Enabler
    } else if report.roles.contains(&CardComponentRoleV1::Lubricant) {
        CandidateRole::Lubricant
    } else if report.roles.contains(&CardComponentRoleV1::Payoff) {
        CandidateRole::Payoff
    } else if report.roles.contains(&CardComponentRoleV1::Transition) {
        CandidateRole::Transition
    } else if report.roles.contains(&CardComponentRoleV1::Liability) {
        CandidateRole::Unknown
    } else {
        CandidateRole::Unknown
    }
}

fn component_reason_pressure(reason: &str) -> PressureKind {
    use super::{StrategicBossTax, StrategicDebt, StrategicJob};
    if reason.contains("awakened_one") || reason.contains("power_tax") {
        PressureKind::BossTax(StrategicBossTax::AwakenedPowerTax)
    } else if reason.contains("automaton") || reason.contains("hyperbeam") {
        PressureKind::BossTax(StrategicBossTax::AutomatonHyperbeamPlan)
    } else if reason.contains("strength") {
        PressureKind::MissingJob(StrategicJob::Scaling)
    } else if reason.contains("draw") || reason.contains("access") || reason.contains("conversion")
    {
        PressureKind::MissingJob(StrategicJob::DrawEnergy)
    } else if reason.contains("block") {
        PressureKind::MissingJob(StrategicJob::Block)
    } else if reason.contains("payoff_without")
        || reason.contains("without_generator")
        || reason.contains("setup")
    {
        PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler)
    } else {
        PressureKind::DeckDebt(StrategicDebt::CycleTime)
    }
}
