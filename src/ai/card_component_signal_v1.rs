use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use crate::ai::card_semantics_v1::card_mechanics_profile_v1;
use crate::ai::deck_startup_profile_v1::{
    startup_energy_candidate_discounted_by_snecko_v1, DeckStartupProfileV1,
};
use crate::ai::noncombat_strategy_v1::StrategyDeckFormationNeedV1;
use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardComponentRoleV1 {
    Enabler,
    Payoff,
    Lubricant,
    Transition,
    Mitigation,
    Liability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardComponentSignalKindV1 {
    FormationNeedCoverage,
    DamageMitigation,
    DrawEnergyAccess,
    ExhaustAccess,
    ExhaustPayoffSupported,
    ExhaustPayoffUnsupported,
    DuplicateNoDrawAccessDebt,
    DuplicateAccessRequiresTurnPlanning,
    PayoffWithoutVisibleGapFill,
    SneckoEnergyDiscountDebt,
    OfferingEnergyGainLessReliableUnderSnecko,
    EnergyGainLessReliableUnderSnecko,
    CorruptionDuplicateWithoutPayoffDebt,
    ExhaustEngineEnabler,
    FnpEngineUnlock,
    SelfDamagePayoffSupported,
    SelfDamagePayoffUnsupported,
    StrengthPayoffConvertibleBurstSupported,
    ConvertibleStrengthRequiresDrawTiming,
    StrengthPayoffTemporaryBurstOnly,
    StrengthPayoffWithoutStableGenerator,
    StrengthPayoffUnsupported,
    StrengthPayoffSupported,
}

impl CardComponentSignalKindV1 {
    pub fn label(self) -> &'static str {
        match self {
            Self::FormationNeedCoverage => "fills_current_formation_need",
            Self::DamageMitigation => "mitigates_enemy_damage",
            Self::DrawEnergyAccess => "improves_access_or_conversion",
            Self::ExhaustAccess => "improves_exhaust_access",
            Self::ExhaustPayoffSupported => "exhaust_payoff_has_generator",
            Self::ExhaustPayoffUnsupported => "exhaust_payoff_without_generator",
            Self::DuplicateNoDrawAccessDebt => "duplicate_draw_access_applies_no_draw_debuff",
            Self::DuplicateAccessRequiresTurnPlanning => "duplicate_access_requires_turn_planning",
            Self::PayoffWithoutVisibleGapFill => "payoff_without_visible_gap_fill",
            Self::SneckoEnergyDiscountDebt => "snecko_random_cost_discounts_energy_startup",
            Self::OfferingEnergyGainLessReliableUnderSnecko => {
                "offering_energy_gain_is_less_reliable_under_snecko"
            }
            Self::EnergyGainLessReliableUnderSnecko => "energy_gain_is_less_reliable_under_snecko",
            Self::CorruptionDuplicateWithoutPayoffDebt => {
                "deck_shape_nonstacking_power_duplicate_without_payoff"
            }
            Self::ExhaustEngineEnabler => "exhaust_engine_enabler",
            Self::FnpEngineUnlock => "unlocks_fnp_engine",
            Self::SelfDamagePayoffSupported => "self_damage_payoff_has_enabler",
            Self::SelfDamagePayoffUnsupported => "self_damage_payoff_without_enabler",
            Self::StrengthPayoffConvertibleBurstSupported => {
                "strength_payoff_has_convertible_burst_source"
            }
            Self::ConvertibleStrengthRequiresDrawTiming => {
                "convertible_strength_requires_draw_timing"
            }
            Self::StrengthPayoffTemporaryBurstOnly => "strength_payoff_has_temporary_burst_support",
            Self::StrengthPayoffWithoutStableGenerator => {
                "strength_payoff_without_stable_generator"
            }
            Self::StrengthPayoffUnsupported => "strength_payoff_without_generator",
            Self::StrengthPayoffSupported => "strength_payoff_has_generator",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardComponentSignalContextV1 {
    pub same_card_count: usize,
    pub formation_needs: Vec<StrategyDeckFormationNeedV1>,
    pub startup: DeckStartupProfileV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardComponentSignalReportV1 {
    pub card: CardId,
    pub roles: Vec<CardComponentRoleV1>,
    pub positive_signals: Vec<CardComponentSignalKindV1>,
    pub debt_signals: Vec<CardComponentSignalKindV1>,
    pub note_signals: Vec<CardComponentSignalKindV1>,
}

pub fn evaluate_card_component_signals_v1(
    context: &CardComponentSignalContextV1,
    profile: &CardRewardSemanticProfileV1,
) -> CardComponentSignalReportV1 {
    let mut report = CardComponentSignalReportV1 {
        card: profile.card,
        roles: component_roles(profile),
        positive_signals: Vec::new(),
        debt_signals: Vec::new(),
        note_signals: Vec::new(),
    };

    add_generic_components(context, profile, &mut report);
    add_card_specific_components(context, profile.card, &mut report);
    add_unresolved_package_payoff_debts(profile, &mut report);
    report
}

fn add_generic_components(
    context: &CardComponentSignalContextV1,
    profile: &CardRewardSemanticProfileV1,
    report: &mut CardComponentSignalReportV1,
) {
    if fills_current_need(context, profile) {
        push_signal(
            &mut report.positive_signals,
            CardComponentSignalKindV1::FormationNeedCoverage,
        );
    }
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
        || profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
    {
        push_signal(
            &mut report.positive_signals,
            CardComponentSignalKindV1::DamageMitigation,
        );
    }
    if effective_draw_energy_access_component(context, profile) {
        push_signal(
            &mut report.positive_signals,
            CardComponentSignalKindV1::DrawEnergyAccess,
        );
    }
    if effective_exhaust_access_component(context, profile) {
        push_signal(
            &mut report.positive_signals,
            CardComponentSignalKindV1::ExhaustAccess,
        );
    }
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::ExhaustPayoff)
    {
        if context.startup.exhaust_engine_count > 0 {
            push_signal(
                &mut report.positive_signals,
                CardComponentSignalKindV1::ExhaustPayoffSupported,
            );
        } else {
            push_signal(
                &mut report.debt_signals,
                CardComponentSignalKindV1::ExhaustPayoffUnsupported,
            );
        }
    }
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::StrengthPayoff)
    {
        add_strength_payoff_component(context, report);
    }
    if context.same_card_count > 0
        && profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        && card_mechanics_profile_v1(profile.card).applies_no_draw_debuff
    {
        push_signal(
            &mut report.debt_signals,
            CardComponentSignalKindV1::DuplicateNoDrawAccessDebt,
        );
        push_signal(
            &mut report.note_signals,
            CardComponentSignalKindV1::DuplicateAccessRequiresTurnPlanning,
        );
    }
}

fn add_unresolved_package_payoff_debts(
    profile: &CardRewardSemanticProfileV1,
    report: &mut CardComponentSignalReportV1,
) {
    if !profile
        .roles
        .contains(&CardRewardSemanticRoleV1::PackagePayoff)
    {
        return;
    }
    if report
        .positive_signals
        .iter()
        .any(|component| package_payoff_support_signal(*component))
    {
        return;
    }
    push_signal(
        &mut report.debt_signals,
        CardComponentSignalKindV1::PayoffWithoutVisibleGapFill,
    );
}

fn package_payoff_support_signal(signal: CardComponentSignalKindV1) -> bool {
    matches!(
        signal,
        CardComponentSignalKindV1::FormationNeedCoverage
            | CardComponentSignalKindV1::ExhaustPayoffSupported
            | CardComponentSignalKindV1::ExhaustEngineEnabler
            | CardComponentSignalKindV1::FnpEngineUnlock
            | CardComponentSignalKindV1::SelfDamagePayoffSupported
            | CardComponentSignalKindV1::StrengthPayoffConvertibleBurstSupported
            | CardComponentSignalKindV1::StrengthPayoffSupported
    )
}

fn add_card_specific_components(
    context: &CardComponentSignalContextV1,
    card: CardId,
    report: &mut CardComponentSignalReportV1,
) {
    match card {
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting
            if startup_energy_candidate_discounted_by_snecko_v1(&context.startup, card) =>
        {
            push_signal(
                &mut report.debt_signals,
                CardComponentSignalKindV1::SneckoEnergyDiscountDebt,
            );
            if card == CardId::Offering {
                push_signal(
                    &mut report.note_signals,
                    CardComponentSignalKindV1::OfferingEnergyGainLessReliableUnderSnecko,
                );
            } else {
                push_signal(
                    &mut report.note_signals,
                    CardComponentSignalKindV1::EnergyGainLessReliableUnderSnecko,
                );
            }
        }
        CardId::Corruption => {
            if context.startup.has_corruption_duplicate_without_payoff {
                push_signal(
                    &mut report.debt_signals,
                    CardComponentSignalKindV1::CorruptionDuplicateWithoutPayoffDebt,
                );
            } else {
                push_signal(
                    &mut report.positive_signals,
                    CardComponentSignalKindV1::ExhaustEngineEnabler,
                );
            }
            if context.startup.feel_no_pain_count > 0 {
                push_signal(
                    &mut report.positive_signals,
                    CardComponentSignalKindV1::FnpEngineUnlock,
                );
            }
        }
        CardId::FeelNoPain | CardId::FireBreathing => {}
        CardId::Rupture => {
            if context.startup.self_damage_source_count == 0 {
                push_signal(
                    &mut report.debt_signals,
                    CardComponentSignalKindV1::SelfDamagePayoffUnsupported,
                );
            } else {
                push_signal(
                    &mut report.positive_signals,
                    CardComponentSignalKindV1::SelfDamagePayoffSupported,
                );
            }
        }
        _ => {}
    }
}

fn add_strength_payoff_component(
    context: &CardComponentSignalContextV1,
    report: &mut CardComponentSignalReportV1,
) {
    if context.startup.persistent_strength_source_count == 0 {
        if context.startup.convertible_strength_source_count > 0 {
            push_signal(
                &mut report.positive_signals,
                CardComponentSignalKindV1::StrengthPayoffConvertibleBurstSupported,
            );
            push_signal(
                &mut report.note_signals,
                CardComponentSignalKindV1::ConvertibleStrengthRequiresDrawTiming,
            );
        } else if context.startup.temporary_strength_burst_count > 0 {
            push_signal(
                &mut report.note_signals,
                CardComponentSignalKindV1::StrengthPayoffTemporaryBurstOnly,
            );
            push_signal(
                &mut report.debt_signals,
                CardComponentSignalKindV1::StrengthPayoffWithoutStableGenerator,
            );
        } else {
            push_signal(
                &mut report.debt_signals,
                CardComponentSignalKindV1::StrengthPayoffUnsupported,
            );
        }
    } else {
        push_signal(
            &mut report.positive_signals,
            CardComponentSignalKindV1::StrengthPayoffSupported,
        );
    }
}

fn component_roles(profile: &CardRewardSemanticProfileV1) -> Vec<CardComponentRoleV1> {
    let mut roles = Vec::new();
    if profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::ScalingSource
                | CardRewardSemanticRoleV1::ExhaustGenerator
                | CardRewardSemanticRoleV1::StatusGenerator
                | CardRewardSemanticRoleV1::BlockRetention
                | CardRewardSemanticRoleV1::BlockMultiplier
        )
    }) {
        push_role(&mut roles, CardComponentRoleV1::Enabler);
    }
    if profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::StrengthPayoff
                | CardRewardSemanticRoleV1::BlockPayoff
                | CardRewardSemanticRoleV1::StrikePayoff
                | CardRewardSemanticRoleV1::UpgradePayoff
                | CardRewardSemanticRoleV1::ExhaustReuse
                | CardRewardSemanticRoleV1::ExhaustPayoff
                | CardRewardSemanticRoleV1::StatusPayoff
                | CardRewardSemanticRoleV1::SelfDamagePayoff
                | CardRewardSemanticRoleV1::CombatExternalPayoff
                | CardRewardSemanticRoleV1::PackagePayoff
        )
    }) {
        push_role(&mut roles, CardComponentRoleV1::Payoff);
    }
    if profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::CardDraw
                | CardRewardSemanticRoleV1::CycleAccess
                | CardRewardSemanticRoleV1::EnergySource
        )
    }) {
        push_role(&mut roles, CardComponentRoleV1::Lubricant);
    }
    if profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::FrontloadDamage
                | CardRewardSemanticRoleV1::Block
                | CardRewardSemanticRoleV1::TemporaryStrengthBurst
        )
    }) {
        push_role(&mut roles, CardComponentRoleV1::Transition);
    }
    if profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::Weak | CardRewardSemanticRoleV1::EnemyStrengthDown
        )
    }) {
        push_role(&mut roles, CardComponentRoleV1::Mitigation);
    }
    if roles.is_empty() {
        push_role(&mut roles, CardComponentRoleV1::Liability);
    }
    roles
}

fn fills_current_need(
    context: &CardComponentSignalContextV1,
    profile: &CardRewardSemanticProfileV1,
) -> bool {
    context.formation_needs.iter().any(|need| match need {
        StrategyDeckFormationNeedV1::Frontload => {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
                || profile.roles.contains(&CardRewardSemanticRoleV1::AoeDamage)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::Vulnerable)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::TemporaryStrengthBurst)
        }
        StrategyDeckFormationNeedV1::Block => {
            profile.roles.contains(&CardRewardSemanticRoleV1::Block)
                || profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
        }
        StrategyDeckFormationNeedV1::Scaling => {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::ScalingSource)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::PackagePayoff)
        }
        StrategyDeckFormationNeedV1::DrawEnergy | StrategyDeckFormationNeedV1::Consistency => {
            effective_draw_energy_access_component(context, profile)
        }
    })
}

fn effective_draw_energy_access_component(
    context: &CardComponentSignalContextV1,
    profile: &CardRewardSemanticProfileV1,
) -> bool {
    if startup_energy_candidate_discounted_by_snecko_v1(&context.startup, profile.card) {
        return false;
    }
    profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::CycleAccess)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnergySource)
}

fn effective_exhaust_access_component(
    context: &CardComponentSignalContextV1,
    profile: &CardRewardSemanticProfileV1,
) -> bool {
    if !profile
        .roles
        .contains(&CardRewardSemanticRoleV1::ExhaustGenerator)
    {
        return false;
    }
    context.startup.exhaust_payoff_count > 0
        || context.startup.feel_no_pain_count > 0
        || context.startup.exhaust_engine_count > 0
}

fn push_role(roles: &mut Vec<CardComponentRoleV1>, role: CardComponentRoleV1) {
    if !roles.contains(&role) {
        roles.push(role);
    }
}

fn push_signal(values: &mut Vec<CardComponentSignalKindV1>, value: CardComponentSignalKindV1) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
    use crate::state::rewards::RewardCard;
    use CardComponentSignalKindV1::*;

    fn context() -> CardComponentSignalContextV1 {
        CardComponentSignalContextV1 {
            same_card_count: 0,
            formation_needs: vec![StrategyDeckFormationNeedV1::Consistency],
            startup: DeckStartupProfileV1::default(),
        }
    }

    #[test]
    fn disarm_emits_generic_mitigation_signal() {
        let report = evaluate_card_component_signals_v1(
            &context(),
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Disarm, 0)),
        );

        assert!(report.positive_signals.contains(&DamageMitigation));
    }

    #[test]
    fn rupture_without_self_damage_is_skip_preferred_not_package_support() {
        let report = evaluate_card_component_signals_v1(
            &context(),
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Rupture, 0)),
        );

        assert!(report.debt_signals.contains(&SelfDamagePayoffUnsupported));
    }

    #[test]
    fn strength_payoff_distinguishes_temporary_and_convertible_strength() {
        let mut burst_only = context();
        burst_only.startup.temporary_strength_burst_count = 1;
        let burst_report = evaluate_card_component_signals_v1(
            &burst_only,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::HeavyBlade, 0)),
        );

        assert!(burst_report
            .note_signals
            .contains(&StrengthPayoffTemporaryBurstOnly));
        assert!(burst_report
            .debt_signals
            .contains(&StrengthPayoffWithoutStableGenerator));
        assert!(!burst_report
            .positive_signals
            .contains(&StrengthPayoffSupported));

        let mut convertible = context();
        convertible.startup.temporary_strength_burst_count = 1;
        convertible.startup.strength_converter_count = 1;
        convertible.startup.convertible_strength_source_count = 1;
        let convertible_report = evaluate_card_component_signals_v1(
            &convertible,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::HeavyBlade, 0)),
        );

        assert!(convertible_report
            .positive_signals
            .contains(&StrengthPayoffConvertibleBurstSupported));
        assert!(convertible_report
            .note_signals
            .contains(&ConvertibleStrengthRequiresDrawTiming));
    }

    #[test]
    fn duplicate_no_draw_access_card_emits_structural_debt() {
        let mut context = context();
        context.same_card_count = 1;

        let report = evaluate_card_component_signals_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::BattleTrance, 0)),
        );

        assert!(report.debt_signals.contains(&DuplicateNoDrawAccessDebt));
        assert!(report
            .note_signals
            .contains(&DuplicateAccessRequiresTurnPlanning));
    }

    #[test]
    fn offering_under_snecko_low_cost_volatility_is_not_clean_startup_access() {
        let mut context = context();
        context.startup.has_snecko_eye = true;
        context.startup.has_snecko_low_cost_volatility = true;
        context.startup.snecko_random_cost_debt = 1;
        context.startup.has_snecko_offering_reliability_debt = true;

        let report = evaluate_card_component_signals_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Offering, 0)),
        );

        assert!(report.debt_signals.contains(&SneckoEnergyDiscountDebt));
        assert!(report
            .note_signals
            .contains(&OfferingEnergyGainLessReliableUnderSnecko));
    }

    #[test]
    fn exhaust_generator_is_exhaust_access_not_draw_energy_lubricant() {
        let mut context = context();
        context.startup.exhaust_payoff_count = 1;
        context.startup.exhaust_engine_count = 1;

        let report = evaluate_card_component_signals_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::SeverSoul, 1)),
        );

        assert!(report.positive_signals.contains(&ExhaustAccess));
        assert!(!report.positive_signals.contains(&DrawEnergyAccess));
        assert!(!report.roles.contains(&CardComponentRoleV1::Lubricant));
    }

    #[test]
    fn exhaust_payoff_without_generator_emits_structural_debt() {
        let report = evaluate_card_component_signals_v1(
            &context(),
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::DarkEmbrace, 0)),
        );

        assert!(report.debt_signals.contains(&ExhaustPayoffUnsupported));
        assert!(report.debt_signals.contains(&PayoffWithoutVisibleGapFill));
    }

    #[test]
    fn exhaust_payoff_with_generator_clears_unresolved_payoff_debt() {
        let mut context = context();
        context.startup.exhaust_engine_count = 1;

        let report = evaluate_card_component_signals_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::DarkEmbrace, 0)),
        );

        assert!(report.positive_signals.contains(&ExhaustPayoffSupported));
        assert!(!report.debt_signals.contains(&PayoffWithoutVisibleGapFill));
    }

    #[test]
    fn corruption_with_fnp_emits_engine_signal_without_boss_tax_verdict() {
        let mut context = context();
        context.startup.feel_no_pain_count = 1;
        let report = evaluate_card_component_signals_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Corruption, 0)),
        );

        assert!(report.positive_signals.contains(&FnpEngineUnlock));
        assert!(report.note_signals.is_empty());
    }

    #[test]
    fn duplicate_corruption_without_payoff_reports_shape_debt() {
        let mut context = context();
        context.startup.corruption_count = 1;
        context.startup.exhaust_payoff_count = 0;
        context.startup.has_corruption_duplicate_without_payoff = true;

        let report = evaluate_card_component_signals_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Corruption, 0)),
        );

        assert!(!report.positive_signals.contains(&ExhaustEngineEnabler));
        assert!(report
            .debt_signals
            .contains(&CorruptionDuplicateWithoutPayoffDebt));
    }
}
