use crate::ai::card_reward_policy_v1::{
    card_reward_semantic_profile_v1, CardRewardSemanticProfileV1, CardRewardSemanticRoleV1,
};
use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyDeckFormationNeedV1,
};
use crate::content::cards::{get_card_definition, CardId, CardType};
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
    pub hp: i32,
    pub max_hp: i32,
    pub deck_size: usize,
    pub powers: usize,
    pub curses: usize,
    pub draw_sources: usize,
    pub exhaust_generators: usize,
    pub formation_needs: Vec<StrategyDeckFormationNeedV1>,
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
        hp: run_state.current_hp,
        max_hp: run_state.max_hp,
        deck_size: run_state.master_deck.len(),
        powers: 0,
        curses: 0,
        draw_sources: 0,
        exhaust_generators: 0,
        formation_needs: strategy.formation_summary().needs,
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
    let draw_one_only = is_draw_one_style_goodstuff(profile);
    let ordinary_frontload = is_ordinary_frontload(profile);

    if fills_missing_job {
        reasons.push("fills_missing_job");
    }
    if boss_answer {
        reasons.push("boss_or_elite_answer");
    }
    if package_enabler {
        reasons.push("package_enabler");
    }
    if strong_access {
        reasons.push("pays_cycle_cost_with_access_or_thinning");
    } else if draw_one_only {
        reasons.push("draw_one_is_not_free_access");
    }
    if ordinary_frontload {
        reasons.push("ordinary_frontload_under_cycle_pressure");
    }

    let verdict = match pressure {
        CardAdmissionPressureV1::Low => CardAdmissionVerdictV1::Admit,
        CardAdmissionPressureV1::Medium => {
            if boss_answer || package_enabler || strong_access || fills_missing_job {
                CardAdmissionVerdictV1::Admit
            } else {
                reasons.push("medium_pressure_requires_clear_job");
                CardAdmissionVerdictV1::AdmitIfNoCleanerAlternative
            }
        }
        CardAdmissionPressureV1::High => {
            if boss_answer || package_enabler || strong_access {
                CardAdmissionVerdictV1::Admit
            } else if fills_missing_job && source != CardAdmissionSourceV1::Shop {
                CardAdmissionVerdictV1::AdmitIfNoCleanerAlternative
            } else {
                reasons.push("high_pressure_rejects_redundant_goodstuff");
                CardAdmissionVerdictV1::Reject
            }
        }
        CardAdmissionPressureV1::Severe => {
            if boss_answer || package_enabler || strong_access {
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

fn effective_cycle_cards(context: &CardAdmissionContextV1) -> f32 {
    let draw_credit = context.draw_sources as f32 * 0.5;
    let exhaust_credit = context.exhaust_generators as f32 * 0.8;
    let power_credit = context.powers as f32 * 0.4;
    let curse_penalty = context.curses as f32 * 1.5;
    (context.deck_size as f32 - draw_credit - exhaust_credit - power_credit + curse_penalty)
        .max(1.0)
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

fn admission_pressure(
    context: &CardAdmissionContextV1,
    effective_cycle_cards: f32,
) -> CardAdmissionPressureV1 {
    let effective_cycle_time = effective_cycle_cards / 5.0;
    if context.deck_size >= 40 || effective_cycle_time > 7.0 {
        CardAdmissionPressureV1::Severe
    } else if context.deck_size >= 35 || effective_cycle_time > 6.0 {
        CardAdmissionPressureV1::High
    } else if context.deck_size >= 30 || effective_cycle_time > 5.0 {
        CardAdmissionPressureV1::Medium
    } else {
        CardAdmissionPressureV1::Low
    }
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
}
