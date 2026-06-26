use super::{CandidateAction, PressureKind, StrategicDebt};
use crate::ai::card_component_signal_v1::{CardComponentSignalKindV1, CardComponentSignalReportV1};
use crate::ai::deck_startup_profile_v1::{
    startup_snecko_cost_conversion_candidate_v1, DeckStartupProfileV1,
};
use crate::content::cards::CardId;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const COMPONENT_SIGNAL_LEDGER_AMOUNT: f32 = 0.35;

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
    SustainOrRecovery,
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AcquisitionExplorationAxisV1 {
    TransitionFrontload,
    DefenseCoverage,
    DrawAccess,
    ExhaustAccess,
    ScalingEngine,
    FutureCeiling,
    SustainOrRecovery,
    BossAnswer,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AcquisitionThesisProfileV1 {
    pub axes: Vec<AcquisitionExplorationAxisV1>,
    pub liability_count: usize,
    pub redundancy_count: usize,
    pub exploration_milli: i32,
    pub caution_milli: i32,
    pub retention_rank_adjustment: i32,
    pub rendered: Vec<String>,
}

impl AcquisitionThesisProfileV1 {
    pub fn has_axis(&self, axis: AcquisitionExplorationAxisV1) -> bool {
        self.axes.contains(&axis)
    }

    pub fn branch_exploration_worthy(&self) -> bool {
        self.has_axis(AcquisitionExplorationAxisV1::BossAnswer)
            || self.has_axis(AcquisitionExplorationAxisV1::ScalingEngine)
            || self.has_axis(AcquisitionExplorationAxisV1::FutureCeiling)
            || self.has_axis(AcquisitionExplorationAxisV1::SustainOrRecovery)
    }
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

    pub fn apply_component_signals_v1(&mut self, report: &CardComponentSignalReportV1) {
        if self.role == CandidateRole::Unknown {
            self.role = component_role(report);
        }
        for signal in &report.positive_signals {
            let kind = positive_component_signal_pressure(*signal);
            if kind == PressureKind::BranchDiversityNeed {
                self.notes
                    .push(format!("component_report_only:{}", signal.label()));
                continue;
            }
            self.positive.push(LedgerDelta {
                kind,
                amount: COMPONENT_SIGNAL_LEDGER_AMOUNT,
                reason: signal.label().to_string(),
            });
        }
        self.negative
            .extend(report.debt_signals.iter().map(|signal| LedgerDelta {
                kind: negative_component_signal_pressure(*signal),
                amount: COMPONENT_SIGNAL_LEDGER_AMOUNT,
                reason: signal.label().to_string(),
            }));
        self.notes.extend(
            report
                .note_signals
                .iter()
                .map(|note| note.label().to_string()),
        );
        self.evidence
            .push("card_component_signal contributor".to_string());
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

    pub fn acquisition_thesis_profile_v1(&self) -> AcquisitionThesisProfileV1 {
        let mut axes = BTreeSet::new();
        let mut liability_count = 0usize;
        let mut redundancy_count = 0usize;
        let mut exploration_milli = 0i32;
        let mut caution_milli = 0i32;
        let mut retention_rank_adjustment = 0i32;
        let mut rendered = Vec::new();

        for thesis in &self.acquisition_theses {
            if let Some(axis) = thesis.exploration_axis_v1() {
                axes.insert(axis);
            }
            if thesis.role == AcquisitionThesisRole::LiabilityOrDependency {
                liability_count += 1;
            }
            if thesis.role == AcquisitionThesisRole::RedundantCoverage {
                redundancy_count += 1;
            }
            exploration_milli =
                exploration_milli.saturating_add(thesis.branch_exploration_milli_v1());
            caution_milli = caution_milli.saturating_add(thesis.caution_milli_v1());
            retention_rank_adjustment = retention_rank_adjustment
                .saturating_add(thesis.retention_rank_adjustment_milli_v1());
            rendered.push(thesis.render_v1());
        }

        AcquisitionThesisProfileV1 {
            axes: axes.into_iter().collect(),
            liability_count,
            redundancy_count,
            exploration_milli,
            caution_milli,
            retention_rank_adjustment,
            rendered,
        }
    }
}

impl AcquisitionThesisSignal {
    pub fn render_v1(&self) -> String {
        format!("{:?}/{:?}:{}", self.role, self.status, self.reason)
    }

    pub fn exploration_axis_v1(&self) -> Option<AcquisitionExplorationAxisV1> {
        match (self.role, self.status) {
            (
                AcquisitionThesisRole::TransitionFrontload,
                AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful,
            ) => Some(AcquisitionExplorationAxisV1::TransitionFrontload),
            (
                AcquisitionThesisRole::MitigationCoverage | AcquisitionThesisRole::PlainBlock,
                AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful,
            ) => Some(AcquisitionExplorationAxisV1::DefenseCoverage),
            (
                AcquisitionThesisRole::DrawAccess,
                AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful,
            ) => Some(AcquisitionExplorationAxisV1::DrawAccess),
            (
                AcquisitionThesisRole::ExhaustAccess,
                AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful,
            ) => Some(AcquisitionExplorationAxisV1::ExhaustAccess),
            (
                AcquisitionThesisRole::ScalingOrEngine,
                AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful,
            ) => Some(AcquisitionExplorationAxisV1::ScalingEngine),
            (
                AcquisitionThesisRole::WinConditionOrCeiling,
                AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful,
            ) => Some(AcquisitionExplorationAxisV1::FutureCeiling),
            (
                AcquisitionThesisRole::SustainOrRecovery,
                AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful,
            ) => Some(AcquisitionExplorationAxisV1::SustainOrRecovery),
            (
                AcquisitionThesisRole::BossSpecificAnswer,
                AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful,
            ) => Some(AcquisitionExplorationAxisV1::BossAnswer),
            _ => None,
        }
    }

    pub fn branch_exploration_milli_v1(&self) -> i32 {
        if self.exploration_axis_v1().is_none() {
            return 0;
        }
        (self.amount.clamp(0.0, 1.0) * 1000.0).round() as i32
    }

    pub fn caution_milli_v1(&self) -> i32 {
        match self.status {
            AcquisitionThesisStatus::Saturated => 450,
            AcquisitionThesisStatus::OverBudget => 800,
            AcquisitionThesisStatus::Unsupported => 1000,
            AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful => 0,
        }
    }

    pub fn retention_rank_adjustment_milli_v1(&self) -> i32 {
        match (self.role, self.status) {
            (AcquisitionThesisRole::WinConditionOrCeiling, AcquisitionThesisStatus::Missing)
            | (AcquisitionThesisRole::SustainOrRecovery, AcquisitionThesisStatus::Missing) => {
                (self.amount * 1000.0).round() as i32
            }
            (_, AcquisitionThesisStatus::Missing | AcquisitionThesisStatus::Useful) => 0,
            (_, AcquisitionThesisStatus::Saturated) => -450,
            (_, AcquisitionThesisStatus::OverBudget) => -800,
            (_, AcquisitionThesisStatus::Unsupported) => -1000,
        }
    }
}

pub fn add_snecko_cost_conversion_delta_v1(
    delta: &mut CandidateDelta,
    startup: &DeckStartupProfileV1,
    candidate: CardId,
) {
    let Some(reason) = startup_snecko_cost_conversion_candidate_v1(startup, candidate) else {
        return;
    };

    if !delta.positive.iter().any(|entry| entry.reason != reason) {
        delta.notes.push(format!(
            "{reason}:candidate_has_no_positive_function_signal"
        ));
        return;
    }
    if delta.positive.iter().any(|entry| entry.reason == reason) {
        return;
    }

    delta.positive.push(LedgerDelta {
        kind: PressureKind::DeckDebt(StrategicDebt::SetupDebt),
        amount: 0.35,
        reason: reason.to_string(),
    });
    delta.evidence.push(reason.to_string());
}

fn component_role(report: &CardComponentSignalReportV1) -> CandidateRole {
    use crate::ai::card_component_signal_v1::CardComponentRoleV1;
    if report.roles.contains(&CardComponentRoleV1::Mitigation) {
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

fn positive_component_signal_pressure(signal: CardComponentSignalKindV1) -> PressureKind {
    use super::StrategicJob;
    match signal {
        CardComponentSignalKindV1::DamageMitigation => {
            PressureKind::MissingJob(StrategicJob::Block)
        }
        CardComponentSignalKindV1::DrawEnergyAccess => {
            PressureKind::MissingJob(StrategicJob::DrawEnergy)
        }
        CardComponentSignalKindV1::ExhaustAccess
        | CardComponentSignalKindV1::ExhaustEngineEnabler
        | CardComponentSignalKindV1::FnpEngineUnlock
        | CardComponentSignalKindV1::ExhaustPayoffSupported => {
            PressureKind::MissingJob(StrategicJob::ExhaustAccess)
        }
        CardComponentSignalKindV1::SelfDamagePayoffSupported
        | CardComponentSignalKindV1::StrengthPayoffConvertibleBurstSupported
        | CardComponentSignalKindV1::StrengthPayoffSupported => {
            PressureKind::MissingJob(StrategicJob::Scaling)
        }
        CardComponentSignalKindV1::FormationNeedCoverage => PressureKind::BranchDiversityNeed,
        _ => PressureKind::BranchDiversityNeed,
    }
}

fn negative_component_signal_pressure(signal: CardComponentSignalKindV1) -> PressureKind {
    use super::StrategicDebt;
    match signal {
        CardComponentSignalKindV1::PayoffWithoutVisibleGapFill
        | CardComponentSignalKindV1::ExhaustPayoffUnsupported
        | CardComponentSignalKindV1::SelfDamagePayoffUnsupported
        | CardComponentSignalKindV1::StrengthPayoffWithoutStableGenerator
        | CardComponentSignalKindV1::StrengthPayoffUnsupported => {
            PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler)
        }
        CardComponentSignalKindV1::SneckoEnergyDiscountDebt => {
            PressureKind::DeckDebt(StrategicDebt::SetupDebt)
        }
        _ => PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk),
    }
}
