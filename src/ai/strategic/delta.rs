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

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionThesisRole {
    TransitionFrontload,
    MitigationCoverage,
    PlainBlock,
    DrawAccess,
    ExhaustAccess,
    ScalingOrEngine,
    WinConditionOrCeiling,
    BossSpecificAnswer,
    RedundantCoverage,
    LiabilityOrDependency,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AcquisitionThesisStatus {
    Missing,
    Useful,
    Saturated,
    OverBudget,
    Unsupported,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct AcquisitionThesisSignal {
    pub role: AcquisitionThesisRole,
    pub status: AcquisitionThesisStatus,
    pub amount: f32,
    pub reason: String,
    pub source: String,
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
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub acquisition_theses: Vec<AcquisitionThesisSignal>,
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
            acquisition_theses: Vec::new(),
        }
    }

    pub fn from_component_report(
        action: CandidateAction,
        report: &CardComponentMarginalReportV1,
    ) -> Self {
        let mut delta = Self::empty(action);
        delta.role = component_role(report);
        delta.verdict_hint = component_verdict(report.verdict);
        for reason in &report.positive_components {
            let kind = positive_component_reason_pressure(reason);
            if kind == PressureKind::BranchDiversityNeed {
                delta.notes.push(format!("component_report_only:{reason}"));
                continue;
            }
            delta.positive.push(LedgerDelta {
                kind,
                amount: 0.35,
                reason: (*reason).to_string(),
            });
        }
        delta.negative = report
            .debts
            .iter()
            .chain(report.boss_taxes.iter())
            .map(|reason| LedgerDelta {
                kind: negative_component_reason_pressure(reason),
                amount: 0.35,
                reason: (*reason).to_string(),
            })
            .collect();
        delta
            .notes
            .extend(report.notes.iter().map(|note| (*note).to_string()));
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

fn positive_component_reason_pressure(reason: &str) -> PressureKind {
    use super::{StrategicBossTax, StrategicJob};
    match reason {
        "direct_strength_down_answer" => PressureKind::MissingJob(StrategicJob::EnemyStrengthDown),
        "mitigates_enemy_damage" => PressureKind::MissingJob(StrategicJob::Block),
        "improves_access_or_conversion" => PressureKind::MissingJob(StrategicJob::DrawEnergy),
        "improves_exhaust_access" => PressureKind::MissingJob(StrategicJob::ExhaustAccess),
        "exhaust_engine_enabler" | "unlocks_fnp_engine" | "exhaust_payoff_has_generator" => {
            PressureKind::MissingJob(StrategicJob::ExhaustAccess)
        }
        "self_damage_payoff_has_enabler"
        | "strength_payoff_has_convertible_burst_source"
        | "strength_payoff_has_generator"
        | "hp_loss_payoff_has_support" => PressureKind::MissingJob(StrategicJob::Scaling),
        "block_payoff_has_block_density" | "big_block_doubles_as_exhaust_material" => {
            PressureKind::MissingJob(StrategicJob::Block)
        }
        "awakened_one_multi_hit_strength_answer" => {
            PressureKind::BossTax(StrategicBossTax::AwakenedPowerTax)
        }
        "automaton_big_turn_or_multi_hit_answer" => {
            PressureKind::BossTax(StrategicBossTax::AutomatonHyperbeamPlan)
        }
        "time_eater_high_impact_or_access" => PressureKind::CardPlayCap,
        "fills_current_formation_need" => PressureKind::BranchDiversityNeed,
        _ => PressureKind::BranchDiversityNeed,
    }
}

fn negative_component_reason_pressure(reason: &str) -> PressureKind {
    use super::{StrategicBossTax, StrategicDebt};
    match reason {
        "awakened_one_minor_power_tax" => PressureKind::BossTax(StrategicBossTax::AwakenedPowerTax),
        "payoff_without_visible_gap_fill"
        | "exhaust_payoff_without_generator"
        | "self_damage_payoff_without_enabler"
        | "strength_payoff_without_stable_generator"
        | "strength_payoff_without_generator"
        | "block_payoff_without_block_engine" => {
            PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler)
        }
        "snecko_random_cost_discounts_energy_startup" => {
            PressureKind::DeckDebt(StrategicDebt::SetupDebt)
        }
        "status_payoff_low_trigger_or_access"
        | "plain_block_redundancy"
        | "hp_loss_payoff_relies_on_accidental_damage" => {
            PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk)
        }
        _ => PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk),
    }
}
