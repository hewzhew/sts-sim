use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use crate::ai::card_semantics_v1::card_mechanics_profile_v1;
use crate::ai::deck_startup_profile_v1::{
    startup_energy_candidate_discounted_by_snecko_v1, DeckStartupProfileV1,
};
use crate::ai::noncombat_strategy_v1::StrategyDeckFormationNeedV1;
use crate::content::cards::CardId;
use crate::content::monsters::factory::EncounterId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardComponentRoleV1 {
    Enabler,
    Payoff,
    Lubricant,
    Transition,
    BossAnswer,
    Liability,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum CardComponentMarginalVerdictV1 {
    Reject,
    SkipPreferred,
    Speculative,
    ContextTake,
    StrongTake,
    MustTake,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardComponentMarginalContextV1 {
    pub act: u8,
    pub floor: i32,
    pub boss: Option<EncounterId>,
    pub hp: i32,
    pub max_hp: i32,
    pub deck_size: usize,
    pub powers: usize,
    pub draw_sources: usize,
    pub exhaust_generators: usize,
    pub frontload_jobs: usize,
    pub block_jobs: usize,
    pub same_card_count: usize,
    pub formation_needs: Vec<StrategyDeckFormationNeedV1>,
    pub startup: DeckStartupProfileV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardComponentMarginalReportV1 {
    pub card: CardId,
    pub roles: Vec<CardComponentRoleV1>,
    pub verdict: CardComponentMarginalVerdictV1,
    pub positive_components: Vec<&'static str>,
    pub debts: Vec<&'static str>,
    pub boss_taxes: Vec<&'static str>,
    pub notes: Vec<&'static str>,
}

impl CardComponentMarginalReportV1 {
    pub fn has_role(&self, role: CardComponentRoleV1) -> bool {
        self.roles.contains(&role)
    }

    pub fn is_negative(&self) -> bool {
        matches!(
            self.verdict,
            CardComponentMarginalVerdictV1::Reject | CardComponentMarginalVerdictV1::SkipPreferred
        )
    }
}

pub fn evaluate_card_component_marginal_value_v1(
    context: &CardComponentMarginalContextV1,
    profile: &CardRewardSemanticProfileV1,
) -> CardComponentMarginalReportV1 {
    let mut report = CardComponentMarginalReportV1 {
        card: profile.card,
        roles: component_roles(profile),
        verdict: CardComponentMarginalVerdictV1::Speculative,
        positive_components: Vec::new(),
        debts: Vec::new(),
        boss_taxes: Vec::new(),
        notes: Vec::new(),
    };

    add_generic_components(context, profile, &mut report);
    add_card_specific_components(context, profile.card, &mut report);
    add_unresolved_package_payoff_debts(profile, &mut report);
    report
}

fn add_generic_components(
    context: &CardComponentMarginalContextV1,
    profile: &CardRewardSemanticProfileV1,
    report: &mut CardComponentMarginalReportV1,
) {
    if fills_current_need(context, profile) {
        push_str(
            &mut report.positive_components,
            "fills_current_formation_need",
        );
    }
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
        || profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
    {
        push_str(&mut report.positive_components, "mitigates_enemy_damage");
    }
    if effective_draw_energy_access_component(context, profile) {
        push_str(
            &mut report.positive_components,
            "improves_access_or_conversion",
        );
    }
    if effective_exhaust_access_component(context, profile) {
        push_str(&mut report.positive_components, "improves_exhaust_access");
    }
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::ExhaustPayoff)
    {
        if context.startup.exhaust_engine_count > 0 {
            push_str(
                &mut report.positive_components,
                "exhaust_payoff_has_generator",
            );
        } else {
            push_str(&mut report.debts, "exhaust_payoff_without_generator");
        }
    }
    if context.same_card_count > 0
        && profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        && card_mechanics_profile_v1(profile.card).applies_no_draw_debuff
    {
        push_str(
            &mut report.debts,
            "duplicate_draw_access_applies_no_draw_debuff",
        );
        push_str(&mut report.notes, "duplicate_access_requires_turn_planning");
    }
}

fn add_unresolved_package_payoff_debts(
    profile: &CardRewardSemanticProfileV1,
    report: &mut CardComponentMarginalReportV1,
) {
    if !profile
        .roles
        .contains(&CardRewardSemanticRoleV1::PackagePayoff)
    {
        return;
    }
    if report
        .positive_components
        .iter()
        .any(|component| package_payoff_support_component(component))
    {
        return;
    }
    push_str(&mut report.debts, "payoff_without_visible_gap_fill");
}

fn package_payoff_support_component(component: &str) -> bool {
    matches!(
        component,
        "fills_current_formation_need"
            | "exhaust_payoff_has_generator"
            | "exhaust_engine_enabler"
            | "unlocks_fnp_engine"
            | "self_damage_payoff_has_enabler"
            | "strength_payoff_has_convertible_burst_source"
            | "strength_payoff_has_generator"
    )
}

fn add_card_specific_components(
    context: &CardComponentMarginalContextV1,
    card: CardId,
    report: &mut CardComponentMarginalReportV1,
) {
    match card {
        CardId::Offering | CardId::SeeingRed | CardId::Bloodletting
            if startup_energy_candidate_discounted_by_snecko_v1(&context.startup, card) =>
        {
            push_str(
                &mut report.debts,
                "snecko_random_cost_discounts_energy_startup",
            );
            if card == CardId::Offering {
                push_str(
                    &mut report.notes,
                    "offering_energy_gain_is_less_reliable_under_snecko",
                );
            } else {
                push_str(
                    &mut report.notes,
                    "energy_gain_is_less_reliable_under_snecko",
                );
            }
        }
        CardId::Corruption => {
            if context.startup.has_corruption_duplicate_without_payoff {
                push_str(
                    &mut report.debts,
                    "deck_shape_nonstacking_power_duplicate_without_payoff",
                );
            } else {
                push_str(&mut report.positive_components, "exhaust_engine_enabler");
            }
            if context.startup.feel_no_pain_count > 0 {
                push_str(&mut report.positive_components, "unlocks_fnp_engine");
            }
        }
        CardId::FeelNoPain | CardId::FireBreathing => {}
        CardId::Rupture => {
            if context.startup.self_damage_source_count == 0 {
                push_str(&mut report.debts, "self_damage_payoff_without_enabler");
            } else {
                push_str(
                    &mut report.positive_components,
                    "self_damage_payoff_has_enabler",
                );
            }
        }
        CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Pummel | CardId::LimitBreak => {
            if context.startup.persistent_strength_source_count == 0 {
                if context.startup.convertible_strength_source_count > 0 {
                    push_str(
                        &mut report.positive_components,
                        "strength_payoff_has_convertible_burst_source",
                    );
                    push_str(
                        &mut report.notes,
                        "convertible_strength_requires_draw_timing",
                    );
                } else if context.startup.temporary_strength_burst_count > 0 {
                    push_str(
                        &mut report.notes,
                        "strength_payoff_has_temporary_burst_support",
                    );
                    push_str(
                        &mut report.debts,
                        "strength_payoff_without_stable_generator",
                    );
                } else {
                    push_str(&mut report.debts, "strength_payoff_without_generator");
                }
            } else {
                push_str(
                    &mut report.positive_components,
                    "strength_payoff_has_generator",
                );
            }
        }
        _ => {}
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
        push_role(&mut roles, CardComponentRoleV1::BossAnswer);
    }
    if roles.is_empty() {
        push_role(&mut roles, CardComponentRoleV1::Liability);
    }
    roles
}

fn fills_current_need(
    context: &CardComponentMarginalContextV1,
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
    context: &CardComponentMarginalContextV1,
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
    context: &CardComponentMarginalContextV1,
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

fn push_str(values: &mut Vec<&'static str>, value: &'static str) {
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
    use crate::state::rewards::RewardCard;

    fn context() -> CardComponentMarginalContextV1 {
        CardComponentMarginalContextV1 {
            act: 3,
            floor: 42,
            boss: Some(EncounterId::AwakenedOne),
            hp: 80,
            max_hp: 100,
            deck_size: 40,
            powers: 4,
            draw_sources: 2,
            exhaust_generators: 1,
            frontload_jobs: 7,
            block_jobs: 7,
            same_card_count: 0,
            formation_needs: vec![StrategyDeckFormationNeedV1::Consistency],
            startup: DeckStartupProfileV1::default(),
        }
    }

    #[test]
    fn disarm_emits_generic_mitigation_signal() {
        let report = evaluate_card_component_marginal_value_v1(
            &context(),
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Disarm, 0)),
        );

        assert_eq!(report.verdict, CardComponentMarginalVerdictV1::Speculative);
        assert!(report
            .positive_components
            .contains(&"mitigates_enemy_damage"));
    }

    #[test]
    fn rupture_without_self_damage_is_skip_preferred_not_package_support() {
        let report = evaluate_card_component_marginal_value_v1(
            &context(),
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Rupture, 0)),
        );

        assert!(report.debts.contains(&"self_damage_payoff_without_enabler"));
    }

    #[test]
    fn strength_payoff_distinguishes_temporary_and_convertible_strength() {
        let mut burst_only = context();
        burst_only.startup.temporary_strength_burst_count = 1;
        let burst_report = evaluate_card_component_marginal_value_v1(
            &burst_only,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::HeavyBlade, 0)),
        );

        assert!(burst_report
            .notes
            .contains(&"strength_payoff_has_temporary_burst_support"));
        assert!(burst_report
            .debts
            .contains(&"strength_payoff_without_stable_generator"));
        assert!(!burst_report
            .positive_components
            .contains(&"strength_payoff_has_generator"));

        let mut convertible = context();
        convertible.startup.temporary_strength_burst_count = 1;
        convertible.startup.strength_converter_count = 1;
        convertible.startup.convertible_strength_source_count = 1;
        let convertible_report = evaluate_card_component_marginal_value_v1(
            &convertible,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::HeavyBlade, 0)),
        );

        assert!(convertible_report
            .positive_components
            .contains(&"strength_payoff_has_convertible_burst_source"));
        assert!(convertible_report
            .notes
            .contains(&"convertible_strength_requires_draw_timing"));
    }

    #[test]
    fn duplicate_no_draw_access_card_emits_structural_debt() {
        let mut context = context();
        context.same_card_count = 1;

        let report = evaluate_card_component_marginal_value_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::BattleTrance, 0)),
        );

        assert!(report
            .debts
            .contains(&"duplicate_draw_access_applies_no_draw_debuff"));
        assert!(report
            .notes
            .contains(&"duplicate_access_requires_turn_planning"));
    }

    #[test]
    fn offering_under_snecko_low_cost_volatility_is_not_clean_startup_access() {
        let mut context = context();
        context.startup.has_snecko_eye = true;
        context.startup.has_snecko_low_cost_volatility = true;
        context.startup.snecko_random_cost_debt = 1;
        context.startup.has_snecko_offering_reliability_debt = true;

        let report = evaluate_card_component_marginal_value_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Offering, 0)),
        );

        assert!(report
            .debts
            .contains(&"snecko_random_cost_discounts_energy_startup"));
        assert!(report
            .notes
            .contains(&"offering_energy_gain_is_less_reliable_under_snecko"));
    }

    #[test]
    fn exhaust_generator_is_exhaust_access_not_draw_energy_lubricant() {
        let mut context = context();
        context.startup.exhaust_payoff_count = 1;
        context.startup.exhaust_engine_count = 1;

        let report = evaluate_card_component_marginal_value_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::SeverSoul, 1)),
        );

        assert!(report
            .positive_components
            .contains(&"improves_exhaust_access"));
        assert!(!report
            .positive_components
            .contains(&"improves_access_or_conversion"));
        assert!(!report.roles.contains(&CardComponentRoleV1::Lubricant));
    }

    #[test]
    fn exhaust_payoff_without_generator_emits_structural_debt() {
        let report = evaluate_card_component_marginal_value_v1(
            &context(),
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::DarkEmbrace, 0)),
        );

        assert!(report.debts.contains(&"exhaust_payoff_without_generator"));
        assert!(report.debts.contains(&"payoff_without_visible_gap_fill"));
    }

    #[test]
    fn exhaust_payoff_with_generator_clears_unresolved_payoff_debt() {
        let mut context = context();
        context.startup.exhaust_engine_count = 1;

        let report = evaluate_card_component_marginal_value_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::DarkEmbrace, 0)),
        );

        assert!(report
            .positive_components
            .contains(&"exhaust_payoff_has_generator"));
        assert!(!report.debts.contains(&"payoff_without_visible_gap_fill"));
    }

    #[test]
    fn corruption_with_fnp_emits_engine_signal_without_boss_tax_verdict() {
        let mut context = context();
        context.startup.feel_no_pain_count = 1;
        let report = evaluate_card_component_marginal_value_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Corruption, 0)),
        );

        assert_eq!(report.verdict, CardComponentMarginalVerdictV1::Speculative);
        assert!(report.positive_components.contains(&"unlocks_fnp_engine"));
        assert!(report.notes.is_empty());
    }

    #[test]
    fn duplicate_corruption_without_payoff_reports_shape_debt() {
        let mut context = context();
        context.startup.corruption_count = 1;
        context.startup.exhaust_payoff_count = 0;
        context.startup.has_corruption_duplicate_without_payoff = true;

        let report = evaluate_card_component_marginal_value_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Corruption, 0)),
        );

        assert!(!report
            .positive_components
            .contains(&"exhaust_engine_enabler"));
        assert!(report
            .debts
            .contains(&"deck_shape_nonstacking_power_duplicate_without_payoff"));
    }
}
