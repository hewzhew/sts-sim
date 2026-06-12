use crate::ai::card_component_marginal_value_v1::{
    evaluate_card_component_marginal_value_v1, CardComponentMarginalContextV1,
    CardComponentMarginalVerdictV1,
};
use crate::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::ai::deck_startup_profile_v1::{
    deck_startup_profile_v1, startup_liability_for_candidate_v1, startup_support_for_candidate_v1,
    DeckStartupProfileV1,
};
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyDeckFormationNeedV1,
};
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::monsters::factory::EncounterId;
use crate::content::relics::RelicId;
use crate::state::rewards::RewardCard;
use crate::state::run::RunState;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardAdmissionSourceV1 {
    Reward,
    Shop,
    Event,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum CardAdmissionPressureV1 {
    Low,
    Medium,
    High,
    Severe,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CardAdmissionVerdictV1 {
    Admit,
    AdmitIfNoCleanerAlternative,
    Reject,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardAdmissionContextV1 {
    pub act: u8,
    pub floor: i32,
    pub boss: Option<EncounterId>,
    pub hp: i32,
    pub max_hp: i32,
    pub deck_size: usize,
    pub powers: usize,
    pub curses: usize,
    pub draw_sources: usize,
    pub exhaust_generators: usize,
    pub frontload_jobs: usize,
    pub block_jobs: usize,
    pub formation_needs: Vec<StrategyDeckFormationNeedV1>,
    pub startup: DeckStartupProfileV1,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardAdmissionReportV1 {
    pub source: CardAdmissionSourceV1,
    pub pressure: CardAdmissionPressureV1,
    pub verdict: CardAdmissionVerdictV1,
    pub effective_cycle_cards_before: f32,
    pub effective_cycle_cards_after: f32,
    pub shop_priority_adjustment: i32,
    pub reasons: Vec<&'static str>,
}

pub fn card_admission_context_from_run_state_v1(run_state: &RunState) -> CardAdmissionContextV1 {
    let strategy = build_run_strategy_snapshot_from_run_state_v2(run_state);
    let mut context = CardAdmissionContextV1 {
        act: run_state.act_num,
        floor: run_state.floor_num,
        boss: run_state.boss_key,
        hp: run_state.current_hp,
        max_hp: run_state.max_hp,
        deck_size: run_state.master_deck.len(),
        powers: 0,
        curses: 0,
        draw_sources: 0,
        exhaust_generators: 0,
        frontload_jobs: 0,
        block_jobs: 0,
        formation_needs: strategy.formation_summary().needs,
        startup: deck_startup_profile_v1(run_state),
    };

    for card in &run_state.master_deck {
        let def = get_card_definition(card.id);
        match def.card_type {
            CardType::Power => context.powers = context.powers.saturating_add(1),
            CardType::Curse => context.curses = context.curses.saturating_add(1),
            _ => {}
        }
        let profile = card_reward_semantic_profile_v1(&RewardCard::new(card.id, card.upgrades));
        if profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw) {
            context.draw_sources = context.draw_sources.saturating_add(1);
        }
        if profile
            .roles
            .contains(&CardRewardSemanticRoleV1::ExhaustGenerator)
        {
            context.exhaust_generators = context.exhaust_generators.saturating_add(1);
        }
        add_profile_jobs_to_context(&mut context, &profile);
    }
    for relic in &run_state.relics {
        match relic.id {
            RelicId::SneckoEye => context.draw_sources = context.draw_sources.saturating_add(2),
            RelicId::RunicPyramid => context.draw_sources = context.draw_sources.saturating_add(1),
            _ => {}
        }
    }

    context
}

pub fn evaluate_card_admission_v1(
    run_state: &RunState,
    card: RewardCard,
    source: CardAdmissionSourceV1,
) -> CardAdmissionReportV1 {
    let context = card_admission_context_from_run_state_v1(run_state);
    let profile = card_reward_semantic_profile_v1(&card);
    evaluate_card_profile_admission_v1(&context, &profile, source)
}

pub fn card_admission_pressure_v1(context: &CardAdmissionContextV1) -> CardAdmissionPressureV1 {
    admission_pressure(context, effective_cycle_cards(context))
}

pub fn evaluate_card_profile_admission_v1(
    context: &CardAdmissionContextV1,
    profile: &CardRewardSemanticProfileV1,
    source: CardAdmissionSourceV1,
) -> CardAdmissionReportV1 {
    let before = effective_cycle_cards(context);
    let after = before + marginal_cycle_cost(profile);
    let pressure = admission_pressure(context, before);
    let mut reasons = Vec::new();

    let fills_missing_job = fills_missing_job(context, profile);
    let boss_answer = is_boss_or_elite_answer(profile);
    let package_enabler = is_package_enabler(profile);
    let strong_access = is_strong_access_or_thinning(profile);
    let contextual_boss_answer = is_contextual_boss_answer(context, profile);
    let boss_answer = boss_answer || contextual_boss_answer;
    let component = evaluate_card_component_marginal_value_v1(
        &component_marginal_context_from_admission_context(context),
        profile,
    );
    let component_negative = component.is_negative();
    let package_enabler = package_enabler && !component_negative;
    let component_strong_take = component.verdict >= CardComponentMarginalVerdictV1::StrongTake;
    let component_context_take = component.verdict >= CardComponentMarginalVerdictV1::ContextTake;
    let draw_one_only = is_draw_one_style_goodstuff(profile);
    let ordinary_frontload = is_ordinary_frontload(profile);
    let boss_specific_liability = is_contextual_boss_liability(context, profile);
    let startup_liability =
        startup_liability_for_candidate_v1(&context.startup, profile.card, context.act);
    let startup_support = startup_support_for_candidate_v1(&context.startup, profile.card);
    let redundant_saturated_job = redundant_saturated_job(context, profile, fills_missing_job);

    if fills_missing_job {
        reasons.push("fills_missing_job");
    }
    if boss_answer {
        reasons.push("boss_or_elite_answer");
    }
    if contextual_boss_answer {
        reasons.push("boss_specific_answer");
    }
    if boss_specific_liability {
        reasons.push("boss_specific_cycle_or_counter_liability");
    }
    if let Some(reason) = startup_liability {
        reasons.push(reason);
    }
    if let Some(reason) = startup_support {
        reasons.push(reason);
    }
    if package_enabler {
        reasons.push("package_enabler");
    }
    if component_strong_take {
        reasons.push("component_marginal_strong_take");
    } else if component_context_take {
        reasons.push("component_marginal_context_take");
    } else if component.verdict == CardComponentMarginalVerdictV1::SkipPreferred {
        reasons.push("component_marginal_skip_preferred");
    } else if component.verdict == CardComponentMarginalVerdictV1::Reject {
        reasons.push("component_marginal_reject");
    }
    reasons.extend(component.positive_components.iter().copied());
    reasons.extend(component.debts.iter().copied());
    reasons.extend(component.boss_taxes.iter().copied());
    if strong_access {
        reasons.push("pays_cycle_cost_with_access_or_thinning");
    } else if draw_one_only {
        reasons.push("draw_one_is_not_free_access");
    }
    if ordinary_frontload {
        reasons.push("ordinary_frontload_under_cycle_pressure");
    }
    if let Some(reason) = redundant_saturated_job {
        reasons.push(reason);
    }

    let verdict = match pressure {
        CardAdmissionPressureV1::Low => {
            if component.verdict == CardComponentMarginalVerdictV1::Reject && !boss_answer {
                reasons.push("low_pressure_rejects_negative_component_margin");
                CardAdmissionVerdictV1::Reject
            } else if startup_liability.is_some() && startup_support.is_none() && !boss_answer {
                reasons.push("low_pressure_rejects_unpayable_startup_debt");
                CardAdmissionVerdictV1::Reject
            } else {
                CardAdmissionVerdictV1::Admit
            }
        }
        CardAdmissionPressureV1::Medium => {
            if component_strong_take {
                CardAdmissionVerdictV1::Admit
            } else if component.verdict == CardComponentMarginalVerdictV1::Reject && !boss_answer {
                reasons.push("medium_pressure_rejects_negative_component_margin");
                CardAdmissionVerdictV1::Reject
            } else if startup_liability.is_some() && startup_support.is_none() && !boss_answer {
                reasons.push("medium_pressure_rejects_unpayable_startup_debt");
                CardAdmissionVerdictV1::Reject
            } else if boss_answer
                || package_enabler
                || strong_access
                || fills_missing_job
                || component_context_take
            {
                CardAdmissionVerdictV1::Admit
            } else if boss_specific_liability {
                reasons.push("medium_pressure_rejects_known_boss_liability");
                CardAdmissionVerdictV1::Reject
            } else if redundant_saturated_job.is_some() {
                CardAdmissionVerdictV1::Reject
            } else {
                reasons.push("medium_pressure_requires_clear_job");
                CardAdmissionVerdictV1::AdmitIfNoCleanerAlternative
            }
        }
        CardAdmissionPressureV1::High => {
            if component_strong_take {
                CardAdmissionVerdictV1::Admit
            } else if component_negative && !boss_answer {
                reasons.push("high_pressure_rejects_negative_component_margin");
                CardAdmissionVerdictV1::Reject
            } else if startup_liability.is_some() && startup_support.is_none() && !boss_answer {
                reasons.push("high_pressure_rejects_unpayable_startup_debt");
                CardAdmissionVerdictV1::Reject
            } else if boss_answer || package_enabler || strong_access || startup_support.is_some() {
                CardAdmissionVerdictV1::Admit
            } else if boss_specific_liability {
                reasons.push("high_pressure_rejects_known_boss_liability");
                CardAdmissionVerdictV1::Reject
            } else if fills_missing_job && source != CardAdmissionSourceV1::Shop {
                CardAdmissionVerdictV1::AdmitIfNoCleanerAlternative
            } else {
                reasons.push("high_pressure_rejects_redundant_goodstuff");
                CardAdmissionVerdictV1::Reject
            }
        }
        CardAdmissionPressureV1::Severe => {
            if component_strong_take {
                CardAdmissionVerdictV1::Admit
            } else if component_negative && !boss_answer {
                reasons.push("severe_pressure_rejects_negative_component_margin");
                CardAdmissionVerdictV1::Reject
            } else if startup_liability.is_some() && startup_support.is_none() && !boss_answer {
                reasons.push("severe_pressure_rejects_unpayable_startup_debt");
                CardAdmissionVerdictV1::Reject
            } else if boss_answer || package_enabler || strong_access || startup_support.is_some() {
                CardAdmissionVerdictV1::Admit
            } else {
                reasons.push("severe_pressure_rejects_cycle_debt");
                CardAdmissionVerdictV1::Reject
            }
        }
    };

    let shop_priority_adjustment = match verdict {
        CardAdmissionVerdictV1::Admit => 0,
        CardAdmissionVerdictV1::AdmitIfNoCleanerAlternative => -300,
        CardAdmissionVerdictV1::Reject => -800,
    };

    CardAdmissionReportV1 {
        source,
        pressure,
        verdict,
        effective_cycle_cards_before: before,
        effective_cycle_cards_after: after,
        shop_priority_adjustment,
        reasons,
    }
}

fn component_marginal_context_from_admission_context(
    context: &CardAdmissionContextV1,
) -> CardComponentMarginalContextV1 {
    CardComponentMarginalContextV1 {
        act: context.act,
        floor: context.floor,
        boss: context.boss,
        hp: context.hp,
        max_hp: context.max_hp,
        deck_size: context.deck_size,
        powers: context.powers,
        draw_sources: context.draw_sources,
        exhaust_generators: context.exhaust_generators,
        frontload_jobs: context.frontload_jobs,
        block_jobs: context.block_jobs,
        formation_needs: context.formation_needs.clone(),
        startup: context.startup.clone(),
    }
}

fn effective_cycle_cards(context: &CardAdmissionContextV1) -> f32 {
    let draw_credit = context.draw_sources as f32 * 0.5;
    let exhaust_credit = context.exhaust_generators as f32 * 0.8;
    let power_credit = context.powers as f32 * 0.4;
    let curse_penalty = context.curses as f32 * 1.5;
    (context.deck_size as f32 - draw_credit - exhaust_credit - power_credit + curse_penalty)
        .max(1.0)
}

fn add_profile_jobs_to_context(
    context: &mut CardAdmissionContextV1,
    profile: &CardRewardSemanticProfileV1,
) {
    if profile
        .roles
        .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
        || profile.roles.contains(&CardRewardSemanticRoleV1::AoeDamage)
    {
        context.frontload_jobs = context.frontload_jobs.saturating_add(1);
    }
    if profile.roles.contains(&CardRewardSemanticRoleV1::Block)
        || profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
    {
        context.block_jobs = context.block_jobs.saturating_add(1);
    }
}

fn marginal_cycle_cost(profile: &CardRewardSemanticProfileV1) -> f32 {
    if is_strong_access_or_thinning(profile) || is_package_enabler(profile) {
        0.25
    } else if profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw) {
        0.7
    } else {
        1.0
    }
}

fn redundant_saturated_job(
    context: &CardAdmissionContextV1,
    profile: &CardRewardSemanticProfileV1,
    fills_missing_job: bool,
) -> Option<&'static str> {
    if fills_missing_job || is_boss_or_elite_answer(profile) || is_package_enabler(profile) {
        return None;
    }
    if is_ordinary_frontload(profile) && context.frontload_jobs >= 8 {
        return Some("frontload_job_saturated_under_pressure");
    }
    if is_plain_block(profile) && context.block_jobs >= 8 {
        return Some("block_job_saturated_under_pressure");
    }
    None
}

fn admission_pressure(
    context: &CardAdmissionContextV1,
    effective_cycle_cards: f32,
) -> CardAdmissionPressureV1 {
    let effective_cycle_time = effective_cycle_cards / 5.0;
    let mut score: u8 = if context.deck_size >= 40 || effective_cycle_time > 7.0 {
        3
    } else if context.deck_size >= 35 || effective_cycle_time > 6.0 {
        2
    } else if context.deck_size >= 30 || effective_cycle_time > 5.0 {
        1
    } else {
        0
    };

    if context.act >= 3 {
        score = score.max(1);
        if context.floor >= 40 {
            score = score.saturating_add(1);
        }
    } else if context.act == 2 && context.floor >= 28 {
        score = score.saturating_add(1);
    }
    if matches!(context.boss, Some(EncounterId::TimeEater)) {
        score = score.saturating_add(1);
    }
    score = score.saturating_sub(large_deck_license_credit(context));

    match score.min(3) {
        0 => CardAdmissionPressureV1::Low,
        1 => CardAdmissionPressureV1::Medium,
        2 => CardAdmissionPressureV1::High,
        _ => CardAdmissionPressureV1::Severe,
    }
}

fn large_deck_license_credit(context: &CardAdmissionContextV1) -> u8 {
    let mut credit: u8 = 0;
    if context.draw_sources >= 4 {
        credit = credit.saturating_add(1);
    }
    if context.exhaust_generators >= 2 {
        credit = credit.saturating_add(1);
    }
    if context.powers >= 4 && context.exhaust_generators > 0 {
        credit = credit.saturating_add(1);
    }
    credit.min(2)
}

fn fills_missing_job(
    context: &CardAdmissionContextV1,
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

fn is_boss_or_elite_answer(profile: &CardRewardSemanticProfileV1) -> bool {
    profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
        || matches!(
            profile.card,
            CardId::Shockwave
                | CardId::Uppercut
                | CardId::FlameBarrier
                | CardId::Impervious
                | CardId::DemonForm
                | CardId::FiendFire
                | CardId::PowerThrough
        )
}

fn is_contextual_boss_answer(
    context: &CardAdmissionContextV1,
    profile: &CardRewardSemanticProfileV1,
) -> bool {
    match context.boss {
        Some(EncounterId::TimeEater) => {
            is_high_impact_per_card(profile)
                || is_strong_access_or_thinning(profile)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::ScalingSource)
        }
        Some(EncounterId::Collector) => {
            profile.roles.contains(&CardRewardSemanticRoleV1::AoeDamage)
                || profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
                || profile
                    .roles
                    .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
                || matches!(
                    profile.card,
                    CardId::FlameBarrier
                        | CardId::Impervious
                        | CardId::PowerThrough
                        | CardId::Shockwave
                        | CardId::Cleave
                        | CardId::Whirlwind
                        | CardId::Immolate
                )
        }
        Some(EncounterId::Automaton) => {
            is_strong_access_or_thinning(profile)
                || matches!(
                    profile.card,
                    CardId::Impervious
                        | CardId::FlameBarrier
                        | CardId::PowerThrough
                        | CardId::Shockwave
                        | CardId::DemonForm
                        | CardId::Corruption
                )
        }
        Some(EncounterId::TheChamp) => {
            profile
                .roles
                .contains(&CardRewardSemanticRoleV1::ScalingSource)
                || matches!(
                    profile.card,
                    CardId::Impervious
                        | CardId::FlameBarrier
                        | CardId::PowerThrough
                        | CardId::DemonForm
                        | CardId::LimitBreak
                        | CardId::HeavyBlade
                )
        }
        _ => false,
    }
}

fn is_contextual_boss_liability(
    context: &CardAdmissionContextV1,
    profile: &CardRewardSemanticProfileV1,
) -> bool {
    match context.boss {
        Some(EncounterId::TimeEater) => is_low_value_time_eater_card(profile),
        _ => false,
    }
}

fn is_high_impact_per_card(profile: &CardRewardSemanticProfileV1) -> bool {
    profile.roles.contains(&CardRewardSemanticRoleV1::Weak)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::EnemyStrengthDown)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::BlockRetention)
        || profile
            .roles
            .contains(&CardRewardSemanticRoleV1::BlockMultiplier)
        || matches!(
            profile.card,
            CardId::Shockwave
                | CardId::Uppercut
                | CardId::FlameBarrier
                | CardId::Impervious
                | CardId::PowerThrough
                | CardId::FiendFire
                | CardId::Barricade
                | CardId::DemonForm
                | CardId::Corruption
        )
}

fn is_low_value_time_eater_card(profile: &CardRewardSemanticProfileV1) -> bool {
    matches!(
        profile.card,
        CardId::Anger | CardId::Flex | CardId::Warcry | CardId::Bloodletting | CardId::SeeingRed
    ) || (is_draw_one_style_goodstuff(profile) && !is_high_impact_per_card(profile))
}

fn is_package_enabler(profile: &CardRewardSemanticProfileV1) -> bool {
    profile.roles.iter().any(|role| {
        matches!(
            role,
            CardRewardSemanticRoleV1::ScalingSource
                | CardRewardSemanticRoleV1::BlockRetention
                | CardRewardSemanticRoleV1::BlockMultiplier
                | CardRewardSemanticRoleV1::ExhaustGenerator
                | CardRewardSemanticRoleV1::StatusGenerator
        )
    })
}

fn is_strong_access_or_thinning(profile: &CardRewardSemanticProfileV1) -> bool {
    matches!(
        profile.card,
        CardId::Offering
            | CardId::BattleTrance
            | CardId::BurningPact
            | CardId::SecondWind
            | CardId::TrueGrit
            | CardId::Corruption
            | CardId::DarkEmbrace
    )
}

fn is_draw_one_style_goodstuff(profile: &CardRewardSemanticProfileV1) -> bool {
    profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        && !is_strong_access_or_thinning(profile)
        && !is_package_enabler(profile)
        && !is_boss_or_elite_answer(profile)
}

fn is_ordinary_frontload(profile: &CardRewardSemanticProfileV1) -> bool {
    profile
        .roles
        .contains(&CardRewardSemanticRoleV1::FrontloadDamage)
        && !is_boss_or_elite_answer(profile)
        && !is_package_enabler(profile)
}

fn is_plain_block(profile: &CardRewardSemanticProfileV1) -> bool {
    profile.roles.contains(&CardRewardSemanticRoleV1::Block)
        && !profile.roles.contains(&CardRewardSemanticRoleV1::CardDraw)
        && !is_boss_or_elite_answer(profile)
        && !is_package_enabler(profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn severe_pressure_rejects_transition_draw_one_goodstuff() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 3;
        run_state.floor_num = 46;
        for _ in 0..34 {
            run_state.add_card_to_deck(CardId::Strike);
        }

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::PommelStrike, 0),
            CardAdmissionSourceV1::Shop,
        );

        assert_eq!(report.pressure, CardAdmissionPressureV1::Severe);
        assert_eq!(report.verdict, CardAdmissionVerdictV1::Reject);
        assert!(report.reasons.contains(&"draw_one_is_not_free_access"));
        assert!(report.shop_priority_adjustment <= -800);
        assert!(report.effective_cycle_cards_after > report.effective_cycle_cards_before);
    }

    #[test]
    fn severe_pressure_admits_boss_answer() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 3;
        run_state.floor_num = 46;
        for _ in 0..34 {
            run_state.add_card_to_deck(CardId::Strike);
        }

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::Shockwave, 0),
            CardAdmissionSourceV1::Shop,
        );

        assert_eq!(report.pressure, CardAdmissionPressureV1::Severe);
        assert_eq!(report.verdict, CardAdmissionVerdictV1::Admit);
        assert!(report.reasons.contains(&"boss_or_elite_answer"));
        assert_eq!(report.shop_priority_adjustment, 0);
    }

    #[test]
    fn medium_pressure_rejects_redundant_frontload_when_frontload_job_is_saturated() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.floor_num = 24;
        for _ in 0..17 {
            run_state.add_card_to_deck(CardId::Strike);
        }

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::TwinStrike, 0),
            CardAdmissionSourceV1::Reward,
        );

        assert_eq!(report.pressure, CardAdmissionPressureV1::Medium);
        assert_eq!(report.verdict, CardAdmissionVerdictV1::Reject);
        assert!(report
            .reasons
            .contains(&"frontload_job_saturated_under_pressure"));
    }

    #[test]
    fn time_eater_pressure_rejects_low_value_card_spam() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 3;
        run_state.floor_num = 44;
        run_state.boss_key = Some(EncounterId::TimeEater);

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::Anger, 0),
            CardAdmissionSourceV1::Reward,
        );

        assert_eq!(report.verdict, CardAdmissionVerdictV1::Reject);
        assert!(report
            .reasons
            .contains(&"boss_specific_cycle_or_counter_liability"));
    }

    #[test]
    fn collector_pressure_admits_aoe_even_under_late_pressure() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.floor_num = 31;
        run_state.boss_key = Some(EncounterId::Collector);
        for _ in 0..26 {
            run_state.add_card_to_deck(CardId::Strike);
        }

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::Cleave, 0),
            CardAdmissionSourceV1::Reward,
        );

        assert_eq!(report.verdict, CardAdmissionVerdictV1::Admit);
        assert!(report.reasons.contains(&"boss_specific_answer"));
    }

    #[test]
    fn startup_profile_rejects_more_fnp_without_exhaust_engine() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 1;
        run_state.add_card_to_deck(CardId::FeelNoPain);

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::FeelNoPain, 0),
            CardAdmissionSourceV1::Reward,
        );

        assert_eq!(report.verdict, CardAdmissionVerdictV1::Reject);
        assert!(report
            .reasons
            .contains(&"startup_rejects_more_fnp_without_exhaust_engine"));
    }

    #[test]
    fn startup_profile_supports_exhaust_engine_for_fnp_deck() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.add_card_to_deck(CardId::FeelNoPain);

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::BurningPact, 0),
            CardAdmissionSourceV1::Reward,
        );

        assert_eq!(report.verdict, CardAdmissionVerdictV1::Admit);
        assert!(report
            .reasons
            .contains(&"startup_supports_fnp_exhaust_engine"));
    }

    #[test]
    fn startup_profile_rejects_act2_strength_payoff_without_strength() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::HeavyBlade, 0),
            CardAdmissionSourceV1::Reward,
        );

        assert_eq!(report.verdict, CardAdmissionVerdictV1::Reject);
        assert!(report
            .reasons
            .contains(&"startup_rejects_strength_payoff_without_strength"));
    }

    #[test]
    fn startup_profile_does_not_treat_unpaid_rupture_as_strength_source() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.act_num = 2;
        run_state.add_card_to_deck(CardId::Rupture);

        let report = evaluate_card_admission_v1(
            &run_state,
            RewardCard::new(CardId::Pummel, 0),
            CardAdmissionSourceV1::Reward,
        );

        assert_eq!(report.verdict, CardAdmissionVerdictV1::Reject);
        assert!(report
            .reasons
            .contains(&"startup_rejects_strength_payoff_without_strength"));
    }
}
