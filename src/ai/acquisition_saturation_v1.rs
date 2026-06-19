use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use crate::ai::strategic::{
    AcquisitionThesisRole, AcquisitionThesisSignal, AcquisitionThesisStatus, CandidateDelta,
    LedgerDelta, PressureKind, StrategicDebt, StrategicJob,
};

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AcquisitionRoleV1 {
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

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum AcquisitionSaturationStatusV1 {
    Missing,
    Useful,
    Saturated,
    OverBudget,
    Unsupported,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AcquisitionSaturationInputV1 {
    pub act: u8,
    pub floor: i32,
    pub deck_size: usize,
    pub frontload_cards: usize,
    pub weak_sources: usize,
    pub block_cards: usize,
    pub draw_sources: usize,
    pub exhaust_generators: usize,
    pub exhaust_payoffs: usize,
    pub scaling_sources: usize,
    pub status_generators: usize,
    pub status_payoffs: usize,
    pub block_engine_pieces: usize,
    pub same_card_count: usize,
    pub starter_strikes: usize,
    pub strength_sources: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AcquisitionSaturationSignalV1 {
    pub role: AcquisitionRoleV1,
    pub status: AcquisitionSaturationStatusV1,
    pub amount: f32,
    pub reason: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AcquisitionSaturationReportV1 {
    pub signals: Vec<AcquisitionSaturationSignalV1>,
    pub positive_tags: Vec<String>,
    pub negative_tags: Vec<String>,
    pub notes: Vec<String>,
}

impl AcquisitionSaturationReportV1 {
    pub fn has_signal(
        &self,
        role: AcquisitionRoleV1,
        status: AcquisitionSaturationStatusV1,
    ) -> bool {
        self.signals
            .iter()
            .any(|signal| signal.role == role && signal.status == status)
    }

    pub fn has_positive_tag(&self, tag: &str) -> bool {
        self.positive_tags.iter().any(|candidate| candidate == tag)
    }

    pub fn has_negative_tag(&self, tag: &str) -> bool {
        self.negative_tags.iter().any(|candidate| candidate == tag)
    }

    fn push(
        &mut self,
        role: AcquisitionRoleV1,
        status: AcquisitionSaturationStatusV1,
        amount: f32,
        reason: &'static str,
    ) {
        self.signals.push(AcquisitionSaturationSignalV1 {
            role,
            status,
            amount: amount.clamp(0.0, 1.0),
            reason: reason.to_string(),
        });
        let tag = format!("{}:{}", status_tag(status), role_tag(role));
        match status {
            AcquisitionSaturationStatusV1::Missing | AcquisitionSaturationStatusV1::Useful => {
                push_unique(&mut self.positive_tags, tag);
            }
            AcquisitionSaturationStatusV1::Saturated
            | AcquisitionSaturationStatusV1::OverBudget
            | AcquisitionSaturationStatusV1::Unsupported => {
                push_unique(&mut self.negative_tags, tag);
            }
        }
        self.notes
            .push(format!("{reason}:{status:?} amount={amount:.2}"));
    }
}

pub fn evaluate_acquisition_saturation_v1(
    input: &AcquisitionSaturationInputV1,
    profile: &CardRewardSemanticProfileV1,
) -> AcquisitionSaturationReportV1 {
    let mut report = AcquisitionSaturationReportV1::default();

    if has_any(
        profile,
        &[
            CardRewardSemanticRoleV1::FrontloadDamage,
            CardRewardSemanticRoleV1::AoeDamage,
            CardRewardSemanticRoleV1::Vulnerable,
            CardRewardSemanticRoleV1::TemporaryStrengthBurst,
        ],
    ) {
        let (status, amount, reason) =
            if input.frontload_cards >= 7 || (input.same_card_count > 0 && input.deck_size >= 18) {
                (
                    AcquisitionSaturationStatusV1::Saturated,
                    0.35,
                    "transition_frontload_already_dense",
                )
            } else if input.frontload_cards <= 3 {
                (
                    AcquisitionSaturationStatusV1::Missing,
                    0.45,
                    "candidate_fills_missing_transition_frontload",
                )
            } else {
                (
                    AcquisitionSaturationStatusV1::Useful,
                    0.22,
                    "candidate_adds_transition_frontload",
                )
            };
        report.push(
            AcquisitionRoleV1::TransitionFrontload,
            status,
            amount,
            reason,
        );
    }

    if has_any(
        profile,
        &[
            CardRewardSemanticRoleV1::Weak,
            CardRewardSemanticRoleV1::EnemyStrengthDown,
        ],
    ) {
        let (status, amount, reason) = if input.weak_sources >= 2 || input.same_card_count > 0 {
            (
                AcquisitionSaturationStatusV1::Saturated,
                0.45,
                "mitigation_coverage_already_present",
            )
        } else {
            (
                AcquisitionSaturationStatusV1::Useful,
                0.35,
                "candidate_adds_mitigation_coverage",
            )
        };
        report.push(
            AcquisitionRoleV1::MitigationCoverage,
            status,
            amount,
            reason,
        );
    }

    if has(profile, CardRewardSemanticRoleV1::Block) {
        let has_access = has_any(
            profile,
            &[
                CardRewardSemanticRoleV1::CardDraw,
                CardRewardSemanticRoleV1::EnergySource,
                CardRewardSemanticRoleV1::ExhaustGenerator,
                CardRewardSemanticRoleV1::BlockRetention,
                CardRewardSemanticRoleV1::BlockMultiplier,
            ],
        );
        let (status, amount, reason) = if input.block_cards >= 8 && !has_access {
            (
                AcquisitionSaturationStatusV1::Saturated,
                0.30,
                "plain_block_density_already_high",
            )
        } else if input.block_cards <= 3 {
            (
                AcquisitionSaturationStatusV1::Missing,
                0.35,
                "candidate_fills_missing_plain_block",
            )
        } else {
            (
                AcquisitionSaturationStatusV1::Useful,
                0.18,
                "candidate_adds_plain_block",
            )
        };
        report.push(AcquisitionRoleV1::PlainBlock, status, amount, reason);
    }

    if has_any(
        profile,
        &[
            CardRewardSemanticRoleV1::CardDraw,
            CardRewardSemanticRoleV1::EnergySource,
        ],
    ) {
        let missing = input.draw_sources == 0;
        report.push(
            AcquisitionRoleV1::DrawAccess,
            if missing {
                AcquisitionSaturationStatusV1::Missing
            } else {
                AcquisitionSaturationStatusV1::Useful
            },
            if missing { 0.55 } else { 0.30 },
            if missing {
                "candidate_fills_missing_draw_or_energy_access"
            } else {
                "candidate_adds_draw_or_energy_access"
            },
        );
    }

    if has(profile, CardRewardSemanticRoleV1::ExhaustGenerator) {
        let missing = input.exhaust_generators == 0;
        report.push(
            AcquisitionRoleV1::ExhaustAccess,
            if missing {
                AcquisitionSaturationStatusV1::Missing
            } else {
                AcquisitionSaturationStatusV1::Useful
            },
            if missing { 0.50 } else { 0.25 },
            if missing {
                "candidate_fills_missing_exhaust_access"
            } else {
                "candidate_adds_exhaust_access"
            },
        );
    }

    if run_is_ready_to_seed_ceiling(input)
        && !deck_has_ceiling_or_win_condition(input)
        && candidate_opens_ceiling_path(profile, input)
    {
        report.push(
            AcquisitionRoleV1::WinConditionOrCeiling,
            AcquisitionSaturationStatusV1::Missing,
            0.60,
            "candidate_opens_missing_win_condition_or_ceiling",
        );
    }

    if has(profile, CardRewardSemanticRoleV1::StrikePayoff) && input.starter_strikes < 3 {
        report.push(
            AcquisitionRoleV1::LiabilityOrDependency,
            AcquisitionSaturationStatusV1::Unsupported,
            0.55,
            "strike_payoff_without_density",
        );
        push_unique(
            &mut report.negative_tags,
            "acquisition_unsupported:strike_payoff_without_density".to_string(),
        );
    }
    if has(profile, CardRewardSemanticRoleV1::StrengthPayoff) && input.strength_sources == 0 {
        report.push(
            AcquisitionRoleV1::LiabilityOrDependency,
            AcquisitionSaturationStatusV1::Unsupported,
            0.45,
            "strength_payoff_without_strength_source",
        );
        push_unique(
            &mut report.negative_tags,
            "acquisition_unsupported:strength_payoff_without_source".to_string(),
        );
    }

    if input.same_card_count > 0
        && report.signals.iter().any(|signal| {
            matches!(
                signal.role,
                AcquisitionRoleV1::TransitionFrontload
                    | AcquisitionRoleV1::MitigationCoverage
                    | AcquisitionRoleV1::PlainBlock
            )
        })
    {
        report.push(
            AcquisitionRoleV1::RedundantCoverage,
            AcquisitionSaturationStatusV1::OverBudget,
            (0.25 + input.same_card_count.min(3) as f32 * 0.10).clamp(0.25, 0.55),
            "duplicate_transition_or_mitigation",
        );
        report.notes.push(format!(
            "duplicate_transition_or_mitigation same_card_count={}",
            input.same_card_count
        ));
    }

    report
}

fn run_is_ready_to_seed_ceiling(input: &AcquisitionSaturationInputV1) -> bool {
    let past_opening = input.act >= 2 || input.floor >= 8 || input.deck_size >= 16;
    let transition_not_empty = input.frontload_cards >= 4 || input.block_cards >= 4;
    past_opening && transition_not_empty
}

fn deck_has_ceiling_or_win_condition(input: &AcquisitionSaturationInputV1) -> bool {
    input.scaling_sources > 0
        || (input.exhaust_generators > 0 && input.exhaust_payoffs > 0)
        || (input.status_generators > 0 && input.status_payoffs > 0)
        || input.block_engine_pieces >= 2
}

fn candidate_opens_ceiling_path(
    profile: &CardRewardSemanticProfileV1,
    input: &AcquisitionSaturationInputV1,
) -> bool {
    let opens_scaling_or_package = has_any(
        profile,
        &[
            CardRewardSemanticRoleV1::ScalingSource,
            CardRewardSemanticRoleV1::ExhaustPayoff,
            CardRewardSemanticRoleV1::StatusPayoff,
        ],
    ) || (has(profile, CardRewardSemanticRoleV1::ExhaustGenerator)
        && input.exhaust_payoffs > 0)
        || (has(profile, CardRewardSemanticRoleV1::StatusGenerator) && input.status_payoffs > 0);

    let opens_block_engine = has_any(
        profile,
        &[
            CardRewardSemanticRoleV1::BlockRetention,
            CardRewardSemanticRoleV1::BlockPayoff,
            CardRewardSemanticRoleV1::BlockMultiplier,
        ],
    ) && (input.block_cards >= 5 || input.block_engine_pieces > 0);

    opens_scaling_or_package || opens_block_engine
}

pub fn apply_acquisition_saturation_to_delta_v1(
    delta: &mut CandidateDelta,
    report: &AcquisitionSaturationReportV1,
) {
    for signal in &report.signals {
        delta.acquisition_theses.push(AcquisitionThesisSignal {
            role: thesis_role(signal.role),
            status: thesis_status(signal.status),
            amount: signal.amount,
            reason: signal.reason.clone(),
            source: "acquisition_saturation_v1".to_string(),
        });
        match signal.status {
            AcquisitionSaturationStatusV1::Missing | AcquisitionSaturationStatusV1::Useful => {
                if let Some(kind) = positive_pressure(signal.role) {
                    delta.positive.push(LedgerDelta {
                        kind,
                        amount: signal.amount,
                        reason: signal.reason.clone(),
                    });
                }
            }
            AcquisitionSaturationStatusV1::Saturated
            | AcquisitionSaturationStatusV1::OverBudget
            | AcquisitionSaturationStatusV1::Unsupported => delta.negative.push(LedgerDelta {
                kind: negative_pressure(signal),
                amount: signal.amount,
                reason: signal.reason.clone(),
            }),
        }
    }
    for tag in report
        .positive_tags
        .iter()
        .chain(report.negative_tags.iter())
    {
        push_unique(&mut delta.evidence, tag.clone());
    }
    for note in &report.notes {
        delta.notes.push(format!("acquisition_saturation:{note}"));
    }
}

fn thesis_role(role: AcquisitionRoleV1) -> AcquisitionThesisRole {
    match role {
        AcquisitionRoleV1::TransitionFrontload => AcquisitionThesisRole::TransitionFrontload,
        AcquisitionRoleV1::MitigationCoverage => AcquisitionThesisRole::MitigationCoverage,
        AcquisitionRoleV1::PlainBlock => AcquisitionThesisRole::PlainBlock,
        AcquisitionRoleV1::DrawAccess => AcquisitionThesisRole::DrawAccess,
        AcquisitionRoleV1::ExhaustAccess => AcquisitionThesisRole::ExhaustAccess,
        AcquisitionRoleV1::ScalingOrEngine => AcquisitionThesisRole::ScalingOrEngine,
        AcquisitionRoleV1::WinConditionOrCeiling => AcquisitionThesisRole::WinConditionOrCeiling,
        AcquisitionRoleV1::BossSpecificAnswer => AcquisitionThesisRole::BossSpecificAnswer,
        AcquisitionRoleV1::RedundantCoverage => AcquisitionThesisRole::RedundantCoverage,
        AcquisitionRoleV1::LiabilityOrDependency => AcquisitionThesisRole::LiabilityOrDependency,
    }
}

fn thesis_status(status: AcquisitionSaturationStatusV1) -> AcquisitionThesisStatus {
    match status {
        AcquisitionSaturationStatusV1::Missing => AcquisitionThesisStatus::Missing,
        AcquisitionSaturationStatusV1::Useful => AcquisitionThesisStatus::Useful,
        AcquisitionSaturationStatusV1::Saturated => AcquisitionThesisStatus::Saturated,
        AcquisitionSaturationStatusV1::OverBudget => AcquisitionThesisStatus::OverBudget,
        AcquisitionSaturationStatusV1::Unsupported => AcquisitionThesisStatus::Unsupported,
    }
}

fn positive_pressure(role: AcquisitionRoleV1) -> Option<PressureKind> {
    match role {
        AcquisitionRoleV1::TransitionFrontload => {
            Some(PressureKind::MissingJob(StrategicJob::Frontload))
        }
        AcquisitionRoleV1::MitigationCoverage | AcquisitionRoleV1::PlainBlock => {
            Some(PressureKind::MissingJob(StrategicJob::Block))
        }
        AcquisitionRoleV1::DrawAccess => Some(PressureKind::MissingJob(StrategicJob::DrawEnergy)),
        AcquisitionRoleV1::ExhaustAccess => {
            Some(PressureKind::MissingJob(StrategicJob::ExhaustAccess))
        }
        AcquisitionRoleV1::ScalingOrEngine | AcquisitionRoleV1::WinConditionOrCeiling => {
            Some(PressureKind::MissingJob(StrategicJob::Scaling))
        }
        AcquisitionRoleV1::BossSpecificAnswer
        | AcquisitionRoleV1::RedundantCoverage
        | AcquisitionRoleV1::LiabilityOrDependency => None,
    }
}

fn negative_pressure(signal: &AcquisitionSaturationSignalV1) -> PressureKind {
    if signal.status == AcquisitionSaturationStatusV1::Unsupported {
        return PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler);
    }
    match signal.role {
        AcquisitionRoleV1::TransitionFrontload
        | AcquisitionRoleV1::PlainBlock
        | AcquisitionRoleV1::DrawAccess
        | AcquisitionRoleV1::ExhaustAccess
        | AcquisitionRoleV1::RedundantCoverage => PressureKind::DeckDebt(StrategicDebt::CycleTime),
        AcquisitionRoleV1::MitigationCoverage
        | AcquisitionRoleV1::ScalingOrEngine
        | AcquisitionRoleV1::WinConditionOrCeiling
        | AcquisitionRoleV1::BossSpecificAnswer => {
            PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk)
        }
        AcquisitionRoleV1::LiabilityOrDependency => {
            PressureKind::DeckDebt(StrategicDebt::PayoffWithoutEnabler)
        }
    }
}

fn status_tag(status: AcquisitionSaturationStatusV1) -> &'static str {
    match status {
        AcquisitionSaturationStatusV1::Missing => "acquisition_missing",
        AcquisitionSaturationStatusV1::Useful => "acquisition_useful",
        AcquisitionSaturationStatusV1::Saturated => "acquisition_saturated",
        AcquisitionSaturationStatusV1::OverBudget => "acquisition_over_budget",
        AcquisitionSaturationStatusV1::Unsupported => "acquisition_unsupported",
    }
}

fn role_tag(role: AcquisitionRoleV1) -> &'static str {
    match role {
        AcquisitionRoleV1::TransitionFrontload => "transition_frontload",
        AcquisitionRoleV1::MitigationCoverage => "mitigation_coverage",
        AcquisitionRoleV1::PlainBlock => "plain_block",
        AcquisitionRoleV1::DrawAccess => "draw_access",
        AcquisitionRoleV1::ExhaustAccess => "exhaust_access",
        AcquisitionRoleV1::ScalingOrEngine => "scaling_or_engine",
        AcquisitionRoleV1::WinConditionOrCeiling => "win_condition_or_ceiling",
        AcquisitionRoleV1::BossSpecificAnswer => "boss_specific_answer",
        AcquisitionRoleV1::RedundantCoverage => "redundant_coverage",
        AcquisitionRoleV1::LiabilityOrDependency => "liability_or_dependency",
    }
}

fn has(profile: &CardRewardSemanticProfileV1, role: CardRewardSemanticRoleV1) -> bool {
    profile.roles.contains(&role)
}

fn has_any(profile: &CardRewardSemanticProfileV1, roles: &[CardRewardSemanticRoleV1]) -> bool {
    roles.iter().any(|role| has(profile, *role))
}

fn push_unique(items: &mut Vec<String>, item: String) {
    if !items.iter().any(|existing| existing == &item) {
        items.push(item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
    use crate::ai::strategic::{CandidateAction, CandidateDelta};
    use crate::content::cards::CardId;
    use crate::state::rewards::RewardCard;

    fn profile(card: CardId) -> crate::ai::card_reward_policy_v1::CardRewardSemanticProfileV1 {
        card_reward_semantic_profile_v1(&RewardCard::new(card, 0))
    }

    fn input() -> AcquisitionSaturationInputV1 {
        AcquisitionSaturationInputV1 {
            act: 2,
            floor: 18,
            deck_size: 28,
            frontload_cards: 7,
            weak_sources: 2,
            block_cards: 7,
            draw_sources: 1,
            exhaust_generators: 1,
            exhaust_payoffs: 1,
            scaling_sources: 1,
            status_generators: 0,
            status_payoffs: 0,
            block_engine_pieces: 0,
            same_card_count: 1,
            starter_strikes: 0,
            strength_sources: 1,
        }
    }

    #[test]
    fn clothesline_duplicate_reports_saturated_mitigation_and_transition() {
        let report = evaluate_acquisition_saturation_v1(&input(), &profile(CardId::Clothesline));

        assert!(report.has_signal(
            AcquisitionRoleV1::MitigationCoverage,
            AcquisitionSaturationStatusV1::Saturated
        ));
        assert!(report.has_negative_tag("acquisition_saturated:mitigation_coverage"));
        assert!(report.has_negative_tag("acquisition_over_budget:redundant_coverage"));
    }

    #[test]
    fn first_burning_pact_reports_missing_draw_and_exhaust_access() {
        let report = evaluate_acquisition_saturation_v1(
            &AcquisitionSaturationInputV1 {
                draw_sources: 0,
                exhaust_generators: 0,
                same_card_count: 0,
                ..input()
            },
            &profile(CardId::BurningPact),
        );

        assert!(report.has_signal(
            AcquisitionRoleV1::DrawAccess,
            AcquisitionSaturationStatusV1::Missing
        ));
        assert!(report.has_signal(
            AcquisitionRoleV1::ExhaustAccess,
            AcquisitionSaturationStatusV1::Missing
        ));
    }

    #[test]
    fn unsupported_payoffs_and_saturation_use_shared_strategic_delta_channels() {
        let payoff =
            evaluate_acquisition_saturation_v1(&input(), &profile(CardId::PerfectedStrike));
        assert!(payoff.has_negative_tag("acquisition_unsupported:strike_payoff_without_density"));

        let report = evaluate_acquisition_saturation_v1(&input(), &profile(CardId::Clothesline));
        let mut delta = CandidateDelta::empty(CandidateAction::TakeCard {
            index: 0,
            card: CardId::Clothesline,
        });
        apply_acquisition_saturation_to_delta_v1(&mut delta, &report);

        assert!(delta.negative.iter().any(|entry| {
            entry.kind == PressureKind::DeckDebt(StrategicDebt::CycleTime)
                && entry.reason == "transition_frontload_already_dense"
        }));
        assert!(delta.negative.iter().any(|entry| {
            entry.kind == PressureKind::DeckDebt(StrategicDebt::CombatShapeRisk)
                && entry.reason == "mitigation_coverage_already_present"
        }));
        assert!(delta
            .evidence
            .iter()
            .any(|entry| entry == "acquisition_over_budget:redundant_coverage"));
        assert!(delta.negative.iter().any(|entry| {
            entry.kind == PressureKind::DeckDebt(StrategicDebt::CycleTime)
                && entry.reason == "duplicate_transition_or_mitigation"
        }));
    }

    #[test]
    fn missing_draw_and_exhaust_access_become_positive_delta_jobs() {
        let report = evaluate_acquisition_saturation_v1(
            &AcquisitionSaturationInputV1 {
                draw_sources: 0,
                exhaust_generators: 0,
                same_card_count: 0,
                ..input()
            },
            &profile(CardId::BurningPact),
        );
        let mut delta = CandidateDelta::empty(CandidateAction::TakeCard {
            index: 1,
            card: CardId::BurningPact,
        });

        apply_acquisition_saturation_to_delta_v1(&mut delta, &report);

        assert!(delta.positive.iter().any(|entry| {
            entry.kind == PressureKind::MissingJob(StrategicJob::DrawEnergy)
                && entry.reason == "candidate_fills_missing_draw_or_energy_access"
        }));
        assert!(delta.positive.iter().any(|entry| {
            entry.kind == PressureKind::MissingJob(StrategicJob::ExhaustAccess)
                && entry.reason == "candidate_fills_missing_exhaust_access"
        }));
    }

    #[test]
    fn midgame_missing_ceiling_marks_engine_candidate_as_win_condition_thesis() {
        let report = evaluate_acquisition_saturation_v1(
            &AcquisitionSaturationInputV1 {
                act: 2,
                deck_size: 18,
                frontload_cards: 6,
                block_cards: 5,
                draw_sources: 1,
                exhaust_generators: 0,
                scaling_sources: 0,
                strength_sources: 0,
                same_card_count: 0,
                ..input()
            },
            &profile(CardId::FeelNoPain),
        );

        assert!(report.has_signal(
            AcquisitionRoleV1::WinConditionOrCeiling,
            AcquisitionSaturationStatusV1::Missing
        ));

        let mut delta = CandidateDelta::empty(CandidateAction::TakeCard {
            index: 2,
            card: CardId::FeelNoPain,
        });
        apply_acquisition_saturation_to_delta_v1(&mut delta, &report);

        assert!(delta.acquisition_theses.iter().any(|thesis| {
            thesis.role == AcquisitionThesisRole::WinConditionOrCeiling
                && thesis.status == AcquisitionThesisStatus::Missing
                && thesis.reason == "candidate_opens_missing_win_condition_or_ceiling"
        }));
        assert!(delta.positive.iter().any(|entry| {
            entry.kind == PressureKind::MissingJob(StrategicJob::Scaling)
                && entry.reason == "candidate_opens_missing_win_condition_or_ceiling"
        }));
    }

    #[test]
    fn early_opening_does_not_seed_ceiling_before_transition_patch_exists() {
        let report = evaluate_acquisition_saturation_v1(
            &AcquisitionSaturationInputV1 {
                act: 1,
                floor: 3,
                deck_size: 12,
                frontload_cards: 2,
                block_cards: 2,
                draw_sources: 0,
                exhaust_generators: 0,
                exhaust_payoffs: 0,
                scaling_sources: 0,
                strength_sources: 0,
                same_card_count: 0,
                ..input()
            },
            &profile(CardId::FeelNoPain),
        );

        assert!(!report.has_signal(
            AcquisitionRoleV1::WinConditionOrCeiling,
            AcquisitionSaturationStatusV1::Missing
        ));
    }

    #[test]
    fn existing_ceiling_does_not_emit_missing_ceiling_thesis() {
        let report = evaluate_acquisition_saturation_v1(
            &AcquisitionSaturationInputV1 {
                act: 2,
                deck_size: 18,
                frontload_cards: 6,
                block_cards: 5,
                draw_sources: 1,
                exhaust_generators: 0,
                scaling_sources: 1,
                strength_sources: 1,
                same_card_count: 0,
                ..input()
            },
            &profile(CardId::FeelNoPain),
        );

        assert!(!report.has_signal(
            AcquisitionRoleV1::WinConditionOrCeiling,
            AcquisitionSaturationStatusV1::Missing
        ));
    }

    #[test]
    fn pure_transition_card_does_not_emit_missing_ceiling_thesis() {
        let report = evaluate_acquisition_saturation_v1(
            &AcquisitionSaturationInputV1 {
                act: 2,
                deck_size: 18,
                frontload_cards: 5,
                block_cards: 5,
                draw_sources: 1,
                exhaust_generators: 0,
                scaling_sources: 0,
                strength_sources: 0,
                same_card_count: 0,
                ..input()
            },
            &profile(CardId::Clothesline),
        );

        assert!(!report.has_signal(
            AcquisitionRoleV1::WinConditionOrCeiling,
            AcquisitionSaturationStatusV1::Missing
        ));
    }

    #[test]
    fn block_engine_seed_requires_some_block_density_or_support() {
        let report = evaluate_acquisition_saturation_v1(
            &AcquisitionSaturationInputV1 {
                act: 2,
                deck_size: 18,
                frontload_cards: 6,
                block_cards: 2,
                draw_sources: 1,
                exhaust_generators: 0,
                scaling_sources: 0,
                strength_sources: 0,
                block_engine_pieces: 0,
                same_card_count: 0,
                ..input()
            },
            &profile(CardId::Barricade),
        );

        assert!(!report.has_signal(
            AcquisitionRoleV1::WinConditionOrCeiling,
            AcquisitionSaturationStatusV1::Missing
        ));
    }
}
