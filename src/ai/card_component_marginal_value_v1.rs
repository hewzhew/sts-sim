use crate::ai::card_reward_policy_v1::{CardRewardSemanticProfileV1, CardRewardSemanticRoleV1};
use crate::ai::card_semantics_v1::card_mechanics_profile_v1;
use crate::ai::deck_startup_profile_v1::DeckStartupProfileV1;
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
    add_boss_components(context, profile, &mut report);
    report.verdict = marginal_verdict(context, &report);
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
    if profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnergySource)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::ExhaustGenerator)
    {
        push_str(
            &mut report.positive_components,
            "improves_access_or_conversion",
        );
    }
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::PackagePayoff)
        && report
            .positive_components
            .iter()
            .all(|component| *component != "fills_current_formation_need")
    {
        push_str(&mut report.debts, "payoff_without_visible_gap_fill");
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

fn add_card_specific_components(
    context: &CardComponentMarginalContextV1,
    card: CardId,
    report: &mut CardComponentMarginalReportV1,
) {
    match card {
        CardId::Disarm => {
            push_str(
                &mut report.positive_components,
                "direct_strength_down_answer",
            );
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
        CardId::FeelNoPain => {
            if context.startup.exhaust_engine_count > 0 {
                push_str(
                    &mut report.positive_components,
                    "exhaust_payoff_has_generator",
                );
            } else {
                push_str(&mut report.debts, "exhaust_payoff_without_generator");
            }
        }
        CardId::FireBreathing => {
            if context.startup.self_damage_source_count == 0
                && context.startup.exhaust_engine_count == 0
                && context.draw_sources <= 2
            {
                push_str(&mut report.debts, "status_payoff_low_trigger_or_access");
            } else {
                push_str(&mut report.notes, "status_payoff_needs_trigger_density");
            }
        }
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
        CardId::BodySlam => {
            if context.block_jobs >= 5 || context.startup.immediate_survival >= 3 {
                push_str(
                    &mut report.positive_components,
                    "block_payoff_has_block_density",
                );
            } else {
                push_str(&mut report.debts, "block_payoff_without_block_engine");
            }
        }
        CardId::GhostlyArmor => {
            if context.startup.exhaust_engine_count > 0 || context.startup.feel_no_pain_count > 0 {
                push_str(
                    &mut report.positive_components,
                    "big_block_doubles_as_exhaust_material",
                );
            } else if context.block_jobs >= 8 {
                push_str(&mut report.debts, "plain_block_redundancy");
            }
        }
        CardId::BloodForBlood => {
            if context.startup.self_damage_source_count > 0 || context.max_hp_buffer_available() {
                push_str(
                    &mut report.positive_components,
                    "hp_loss_payoff_has_support",
                );
            } else if context.act >= 2 {
                push_str(
                    &mut report.debts,
                    "hp_loss_payoff_relies_on_accidental_damage",
                );
            }
        }
        _ => {}
    }
}

fn add_boss_components(
    context: &CardComponentMarginalContextV1,
    profile: &CardRewardSemanticProfileV1,
    report: &mut CardComponentMarginalReportV1,
) {
    match context.boss {
        Some(EncounterId::AwakenedOne) => {
            if profile
                .roles
                .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
            {
                push_str(
                    &mut report.positive_components,
                    "awakened_one_multi_hit_strength_answer",
                );
            }
            if profile.card == CardId::Corruption {
                push_str(&mut report.notes, "awakened_one_power_but_core_engine");
            } else if profile_is_minor_power(profile) {
                push_str(&mut report.boss_taxes, "awakened_one_minor_power_tax");
            }
        }
        Some(EncounterId::Automaton) => {
            if matches!(
                profile.card,
                CardId::Impervious
                    | CardId::FlameBarrier
                    | CardId::PowerThrough
                    | CardId::Disarm
                    | CardId::Shockwave
            ) {
                push_str(
                    &mut report.positive_components,
                    "automaton_big_turn_or_multi_hit_answer",
                );
            }
        }
        Some(EncounterId::TheChamp) => {
            if matches!(
                profile.card,
                CardId::Impervious
                    | CardId::FlameBarrier
                    | CardId::PowerThrough
                    | CardId::Disarm
                    | CardId::DemonForm
            ) {
                push_str(
                    &mut report.positive_components,
                    "champ_execute_or_scaling_answer",
                );
            }
        }
        Some(EncounterId::TimeEater) => {
            if profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
                || profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
            {
                push_str(
                    &mut report.positive_components,
                    "time_eater_high_impact_or_access",
                );
            }
        }
        _ => {}
    }
}

fn marginal_verdict(
    context: &CardComponentMarginalContextV1,
    report: &CardComponentMarginalReportV1,
) -> CardComponentMarginalVerdictV1 {
    if report
        .positive_components
        .iter()
        .any(|component| component.contains("strength_answer"))
        && matches!(context.boss, Some(EncounterId::AwakenedOne))
    {
        return CardComponentMarginalVerdictV1::MustTake;
    }
    if report
        .positive_components
        .iter()
        .any(|component| component.contains("answer"))
    {
        return CardComponentMarginalVerdictV1::StrongTake;
    }
    if report
        .positive_components
        .iter()
        .any(|component| component.contains("unlocks") || component.contains("improves_access"))
    {
        return CardComponentMarginalVerdictV1::ContextTake;
    }
    if report.boss_taxes.len() >= 1 && report.positive_components.is_empty() {
        return CardComponentMarginalVerdictV1::SkipPreferred;
    }
    if report.debts.len() >= 2 {
        return CardComponentMarginalVerdictV1::Reject;
    }
    if report.debts.len() == 1 && context.act >= 2 {
        return CardComponentMarginalVerdictV1::SkipPreferred;
    }
    if report.positive_components.is_empty()
        && context.act >= 3
        && context.deck_size >= 35
        && !report.roles.contains(&CardComponentRoleV1::Lubricant)
    {
        return CardComponentMarginalVerdictV1::SkipPreferred;
    }
    if !report.positive_components.is_empty() {
        CardComponentMarginalVerdictV1::ContextTake
    } else {
        CardComponentMarginalVerdictV1::Speculative
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
                | CardRewardSemanticRoleV1::ExhaustPayoff
                | CardRewardSemanticRoleV1::StatusPayoff
                | CardRewardSemanticRoleV1::SelfDamagePayoff
                | CardRewardSemanticRoleV1::PackagePayoff
        )
    }) {
        push_role(&mut roles, CardComponentRoleV1::Payoff);
    }
    if profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::CardDraw
                | CardRewardSemanticRoleV1::EnergySource
                | CardRewardSemanticRoleV1::ExhaustGenerator
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
            profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::EnergySource)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::ExhaustGenerator)
        }
    })
}

fn profile_is_minor_power(profile: &CardRewardSemanticProfileV1) -> bool {
    matches!(
        profile.card,
        CardId::FireBreathing
            | CardId::Rupture
            | CardId::Metallicize
            | CardId::Combust
            | CardId::Evolve
            | CardId::Inflame
            | CardId::FeelNoPain
    )
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

impl CardComponentMarginalContextV1 {
    fn max_hp_buffer_available(&self) -> bool {
        self.max_hp >= 90 && self.hp_ratio_percent() >= 55
    }

    fn hp_ratio_percent(&self) -> i32 {
        if self.max_hp <= 0 {
            return 0;
        }
        self.hp.saturating_mul(100) / self.max_hp
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
    fn disarm_is_must_take_against_awakened_one() {
        let report = evaluate_card_component_marginal_value_v1(
            &context(),
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Disarm, 0)),
        );

        assert_eq!(report.verdict, CardComponentMarginalVerdictV1::MustTake);
        assert!(report
            .positive_components
            .contains(&"awakened_one_multi_hit_strength_answer"));
    }

    #[test]
    fn rupture_without_self_damage_is_skip_preferred_not_package_support() {
        let report = evaluate_card_component_marginal_value_v1(
            &context(),
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Rupture, 0)),
        );

        assert_eq!(
            report.verdict,
            CardComponentMarginalVerdictV1::SkipPreferred
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
    fn corruption_with_fnp_is_context_take_despite_awakened_power_tax() {
        let mut context = context();
        context.startup.feel_no_pain_count = 1;
        let report = evaluate_card_component_marginal_value_v1(
            &context,
            &card_reward_semantic_profile_v1(&RewardCard::new(CardId::Corruption, 0)),
        );

        assert_eq!(report.verdict, CardComponentMarginalVerdictV1::ContextTake);
        assert!(report.positive_components.contains(&"unlocks_fnp_engine"));
        assert!(report.notes.contains(&"awakened_one_power_but_core_engine"));
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
