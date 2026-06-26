use crate::ai::card_analysis_v1::{
    card_analysis_profile_v1, CardAnalysisProfileV1, CardAnalysisStartupKeyV1,
};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeckStartupProfileV1 {
    pub setup_debt: u8,
    pub setup_payment: u8,
    pub immediate_survival: u8,
    pub payoff_engine: u8,
    pub combat_shape_risk: u8,
    pub feel_no_pain_count: u8,
    pub exhaust_engine_count: u8,
    pub exhaust_payoff_count: u8,
    pub corruption_count: u8,
    pub havoc_count: u8,
    pub status_generator_count: u8,
    pub status_digest_count: u8,
    pub strong_draw_count: u8,
    pub effective_setup_payment: u8,
    pub effective_strong_draw_count: u8,
    pub has_snecko_eye: bool,
    pub zero_cost_card_count: u8,
    pub low_cost_card_count: u8,
    pub high_cost_card_count: u8,
    pub snecko_draw_bonus: u8,
    pub snecko_random_cost_debt: u8,
    pub snecko_high_cost_payoff: u8,
    pub persistent_strength_source_count: u8,
    pub temporary_strength_burst_count: u8,
    pub strength_converter_count: u8,
    pub convertible_strength_source_count: u8,
    pub rupture_count: u8,
    pub self_damage_source_count: u8,
    pub strength_payoff_count: u8,
    pub dual_wield_count: u8,
    pub dual_wield_target_count: u8,
    pub anger_count: u8,
    pub armaments_count: u8,
    pub upgraded_armaments_count: u8,
    pub apparition_count: u8,
    pub upgraded_apparition_count: u8,
    pub has_runic_pyramid: bool,
    pub has_setup_debt_high_payment_low: bool,
    pub has_fnp_duplicate_without_exhaust_engine: bool,
    pub has_corruption_duplicate_without_payoff: bool,
    pub has_havoc_duplicate_without_payoff: bool,
    pub has_status_generator_saturation_without_digest: bool,
    pub has_clash_playability_debt: bool,
    pub has_dual_wield_without_target: bool,
    pub has_anger_duplicate_without_digest: bool,
    pub has_strength_payoff_without_strength: bool,
    pub has_rupture_without_self_damage: bool,
    pub has_armaments_unupgraded_duplicate: bool,
    pub has_pyramid_unupgraded_apparition: bool,
    pub has_snecko_low_cost_volatility: bool,
    pub has_snecko_offering_reliability_debt: bool,
}

pub fn deck_startup_profile_v1(run_state: &RunState) -> DeckStartupProfileV1 {
    let strength = crate::ai::strength_profile_v1::strength_profile_v1(run_state);
    let deck_shape = crate::ai::deck_shape_v1::deck_shape_profile_v1(run_state);
    let mut profile = DeckStartupProfileV1 {
        has_runic_pyramid: run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::RunicPyramid),
        has_snecko_eye: run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::SneckoEye),
        ..Default::default()
    };

    for relic in &run_state.relics {
        match relic.id {
            RelicId::RunicPyramid | RelicId::SneckoEye => {
                profile.setup_payment = profile.setup_payment.saturating_add(1);
            }
            RelicId::MedicalKit => {
                profile.exhaust_engine_count = profile.exhaust_engine_count.saturating_add(1);
            }
            _ => {}
        }
    }

    for card in &run_state.master_deck {
        let id = card.id;
        let analysis = card_analysis_profile_v1(id, card.upgrades);
        record_card_cost_shape(&analysis, &mut profile);
        if analysis.is_startup_setup_debt {
            profile.setup_debt = profile.setup_debt.saturating_add(1);
        }
        if analysis.is_startup_setup_payment {
            profile.setup_payment = profile.setup_payment.saturating_add(1);
        }
        if analysis.is_startup_immediate_survival {
            profile.immediate_survival = profile.immediate_survival.saturating_add(1);
        }
        if analysis.is_startup_base_combat_shape_risk
            || (profile.has_runic_pyramid && analysis.is_startup_unupgraded_apparition)
        {
            profile.combat_shape_risk = profile.combat_shape_risk.saturating_add(1);
        }
        if analysis.is_startup_exhaust_engine {
            profile.exhaust_engine_count = profile.exhaust_engine_count.saturating_add(1);
        }
        if analysis.is_startup_strong_draw {
            profile.strong_draw_count = profile.strong_draw_count.saturating_add(1);
        }
        if analysis.startup_key == Some(CardAnalysisStartupKeyV1::Rupture) {
            profile.rupture_count = profile.rupture_count.saturating_add(1);
        }
        if analysis.is_startup_self_damage_source {
            profile.self_damage_source_count = profile.self_damage_source_count.saturating_add(1);
        }
        if analysis.is_startup_dual_wield_target {
            profile.dual_wield_target_count = profile.dual_wield_target_count.saturating_add(1);
        }

        match analysis.startup_key {
            Some(CardAnalysisStartupKeyV1::FeelNoPain) => {
                profile.feel_no_pain_count = profile.feel_no_pain_count.saturating_add(1)
            }
            Some(CardAnalysisStartupKeyV1::DualWield) => {
                profile.dual_wield_count = profile.dual_wield_count.saturating_add(1)
            }
            Some(CardAnalysisStartupKeyV1::Anger) => {
                profile.anger_count = profile.anger_count.saturating_add(1)
            }
            Some(CardAnalysisStartupKeyV1::Armaments) => {
                profile.armaments_count = profile.armaments_count.saturating_add(1);
                if card.upgrades > 0 {
                    profile.upgraded_armaments_count =
                        profile.upgraded_armaments_count.saturating_add(1);
                }
            }
            Some(CardAnalysisStartupKeyV1::Apparition) => {
                profile.apparition_count = profile.apparition_count.saturating_add(1);
                if card.upgrades > 0 {
                    profile.upgraded_apparition_count =
                        profile.upgraded_apparition_count.saturating_add(1);
                }
            }
            _ => {}
        }
    }

    apply_relic_adjusted_startup_v1(&mut profile, run_state);

    profile.persistent_strength_source_count = strength.stable_sources;
    profile.exhaust_payoff_count = deck_shape.exhaust_payoff_count;
    profile.corruption_count = deck_shape.corruption_count;
    profile.havoc_count = deck_shape.havoc_count;
    profile.status_generator_count = deck_shape.status_generator_count;
    profile.status_digest_count = deck_shape.status_digest_count;
    profile.temporary_strength_burst_count = strength.temporary_bursts;
    profile.strength_converter_count = strength.converters;
    profile.convertible_strength_source_count = strength.convertible_potential_count;
    profile.strength_payoff_count = strength.payoffs;
    profile.payoff_engine = profile
        .payoff_engine
        .saturating_add(strength.stable_sources)
        .saturating_add(deck_shape.exhaust_payoff_count)
        .saturating_add(deck_shape.status_digest_count);
    if strength.convertible_potential_count > 0 {
        profile.payoff_engine = profile.payoff_engine.saturating_add(1);
    }

    profile.has_setup_debt_high_payment_low = profile.setup_debt >= 4
        && profile
            .effective_setup_payment
            .saturating_add(profile.effective_strong_draw_count)
            <= 2;
    profile.has_fnp_duplicate_without_exhaust_engine =
        profile.feel_no_pain_count >= 2 && profile.exhaust_engine_count == 0;
    profile.has_corruption_duplicate_without_payoff = deck_shape.risks.iter().any(|risk| {
        matches!(
            risk,
            crate::ai::deck_shape_v1::DeckShapeRiskV1::NonstackingPowerDuplicateWithoutPayoff {
                card,
                ..
            } if card_has_startup_key_v1(*card, CardAnalysisStartupKeyV1::Corruption)
        )
    });
    profile.has_havoc_duplicate_without_payoff = deck_shape.risks.iter().any(|risk| {
        matches!(
            risk,
            crate::ai::deck_shape_v1::DeckShapeRiskV1::RandomExhaustSaturationWithoutPayoff {
                card,
                ..
            } if card_has_startup_key_v1(*card, CardAnalysisStartupKeyV1::Havoc)
        )
    });
    profile.has_status_generator_saturation_without_digest = deck_shape.risks.iter().any(|risk| {
        matches!(
            risk,
            crate::ai::deck_shape_v1::DeckShapeRiskV1::StatusGeneratorSaturationWithoutDigest { .. }
        )
    });
    profile.has_clash_playability_debt = deck_shape.risks.iter().any(|risk| {
        matches!(
            risk,
            crate::ai::deck_shape_v1::DeckShapeRiskV1::ClashPlayabilityDebt { .. }
        )
    });
    profile.has_dual_wield_without_target =
        profile.dual_wield_count > 0 && profile.dual_wield_target_count == 0;
    profile.has_anger_duplicate_without_digest = profile.anger_count >= 2
        && profile
            .strong_draw_count
            .saturating_add(profile.exhaust_engine_count)
            .saturating_add(profile.persistent_strength_source_count)
            <= 1;
    profile.has_strength_payoff_without_strength =
        profile.strength_payoff_count > 0 && profile.persistent_strength_source_count == 0;
    profile.has_rupture_without_self_damage =
        profile.rupture_count > 0 && profile.self_damage_source_count == 0;
    profile.has_armaments_unupgraded_duplicate =
        profile.armaments_count >= 2 && profile.upgraded_armaments_count == 0;
    profile.has_pyramid_unupgraded_apparition =
        profile.has_runic_pyramid && profile.apparition_count > profile.upgraded_apparition_count;

    profile
}

fn apply_relic_adjusted_startup_v1(profile: &mut DeckStartupProfileV1, run_state: &RunState) {
    profile.effective_setup_payment = profile.setup_payment;
    profile.effective_strong_draw_count = profile.strong_draw_count;

    if !profile.has_snecko_eye {
        return;
    }

    profile.snecko_draw_bonus = 2;
    profile.snecko_high_cost_payoff = profile.high_cost_card_count;
    profile.has_snecko_low_cost_volatility =
        profile.low_cost_card_count > profile.high_cost_card_count.saturating_add(3);
    profile.snecko_random_cost_debt = if profile.has_snecko_low_cost_volatility {
        if profile.low_cost_card_count > profile.high_cost_card_count.saturating_add(7) {
            2
        } else {
            1
        }
    } else {
        0
    };

    let has_offering = run_state.master_deck.iter().any(|card| {
        card_analysis_profile_v1(card.id, card.upgrades).startup_key
            == Some(CardAnalysisStartupKeyV1::Offering)
    });
    profile.has_snecko_offering_reliability_debt =
        has_offering && profile.snecko_random_cost_debt > 0;

    if profile.has_snecko_offering_reliability_debt {
        profile.effective_setup_payment = profile.effective_setup_payment.saturating_sub(1);
        profile.effective_strong_draw_count = profile.effective_strong_draw_count.saturating_sub(1);
    }
}

fn record_card_cost_shape(analysis: &CardAnalysisProfileV1, profile: &mut DeckStartupProfileV1) {
    if analysis.cost == 0 {
        profile.zero_cost_card_count = profile.zero_cost_card_count.saturating_add(1);
    }
    if (0..=1).contains(&analysis.cost) {
        profile.low_cost_card_count = profile.low_cost_card_count.saturating_add(1);
    } else if analysis.cost >= 2 {
        profile.high_cost_card_count = profile.high_cost_card_count.saturating_add(1);
    }
}

fn card_has_startup_key_v1(card: CardId, key: CardAnalysisStartupKeyV1) -> bool {
    card_analysis_profile_v1(card, 0).startup_key == Some(key)
}

pub fn startup_liability_for_candidate_v1(
    startup: &DeckStartupProfileV1,
    candidate: CardId,
    act: u8,
) -> Option<&'static str> {
    let candidate = card_analysis_profile_v1(candidate, 0);
    match candidate.startup_key {
        Some(CardAnalysisStartupKeyV1::Corruption)
            if startup.exhaust_payoff_count == 0 && startup.corruption_count >= 1 =>
        {
            Some("startup_rejects_corruption_duplicate_without_payoff")
        }
        Some(CardAnalysisStartupKeyV1::Havoc)
            if startup.exhaust_payoff_count == 0 && startup.havoc_count >= 1 =>
        {
            Some("startup_rejects_havoc_duplicate_without_payoff")
        }
        _ if candidate.has_status_enabler
            && startup.status_generator_count >= 1
            && startup.status_digest_count == 0 =>
        {
            Some("startup_rejects_status_generator_duplicate_without_digest")
        }
        Some(CardAnalysisStartupKeyV1::Clash) if startup.has_clash_playability_debt => {
            Some("startup_rejects_clash_playability_debt")
        }
        Some(CardAnalysisStartupKeyV1::FeelNoPain)
            if startup.feel_no_pain_count >= 1 && startup.exhaust_engine_count == 0 =>
        {
            Some("startup_rejects_more_fnp_without_exhaust_engine")
        }
        Some(CardAnalysisStartupKeyV1::FeelNoPain)
            if startup.feel_no_pain_count >= 2 && startup.setup_payment <= 2 =>
        {
            Some("startup_rejects_third_fnp_without_setup_payment")
        }
        Some(CardAnalysisStartupKeyV1::DualWield)
            if startup.dual_wield_target_count == 0 || startup.setup_payment <= 1 =>
        {
            Some("startup_rejects_dual_wield_without_target_or_payment")
        }
        Some(CardAnalysisStartupKeyV1::Anger)
            if startup.anger_count >= 1
                && (act >= 2
                    || startup
                        .strong_draw_count
                        .saturating_add(startup.exhaust_engine_count)
                        .saturating_add(startup.persistent_strength_source_count)
                        <= 1) =>
        {
            Some("startup_rejects_more_anger_without_digest")
        }
        _ if candidate.is_startup_strength_payoff_liability_candidate
            && startup.persistent_strength_source_count == 0
            && act >= 2 =>
        {
            Some("startup_rejects_strength_payoff_without_strength")
        }
        Some(CardAnalysisStartupKeyV1::Rupture) if startup.self_damage_source_count == 0 => {
            Some("startup_rejects_rupture_without_self_damage")
        }
        _ => None,
    }
}

pub fn startup_support_for_candidate_v1(
    startup: &DeckStartupProfileV1,
    candidate: CardId,
) -> Option<&'static str> {
    let candidate = card_analysis_profile_v1(candidate, 0);
    match candidate.startup_key {
        Some(CardAnalysisStartupKeyV1::Offering)
            if startup.has_setup_debt_high_payment_low
                && !startup_energy_candidate_discounted_by_snecko_v1(startup, candidate.card) =>
        {
            Some("startup_supports_setup_payment")
        }
        _ if candidate.startup_key != Some(CardAnalysisStartupKeyV1::Offering)
            && candidate.is_startup_strong_setup_support_candidate
            && startup.has_setup_debt_high_payment_low =>
        {
            Some("startup_supports_setup_payment")
        }
        _ if candidate.is_startup_fnp_exhaust_support_candidate
            && startup.feel_no_pain_count > 0 =>
        {
            Some("startup_supports_fnp_exhaust_engine")
        }
        _ if candidate.is_startup_stable_strength_support_candidate
            && startup.has_strength_payoff_without_strength =>
        {
            Some("startup_supports_strength_source")
        }
        Some(CardAnalysisStartupKeyV1::Rupture)
            if startup.has_strength_payoff_without_strength
                && startup.self_damage_source_count > 0 =>
        {
            Some("startup_supports_conditional_strength_source")
        }
        _ if candidate.is_startup_self_damage_support_candidate && startup.rupture_count > 0 => {
            Some("startup_supports_rupture_self_damage_source")
        }
        Some(CardAnalysisStartupKeyV1::Armaments)
            if startup.upgraded_armaments_count == 0 && startup.armaments_count == 0 =>
        {
            Some("startup_supports_upgrade_access")
        }
        _ => None,
    }
}

pub fn startup_energy_candidate_discounted_by_snecko_v1(
    startup: &DeckStartupProfileV1,
    candidate: CardId,
) -> bool {
    let candidate = card_analysis_profile_v1(candidate, 0);
    startup.has_snecko_eye
        && startup.has_snecko_low_cost_volatility
        && candidate.is_startup_snecko_energy_candidate
}

pub fn startup_snecko_cost_conversion_candidate_v1(
    startup: &DeckStartupProfileV1,
    candidate: CardId,
) -> Option<&'static str> {
    if !startup.has_snecko_eye || !startup.has_snecko_low_cost_volatility {
        return None;
    }

    let candidate = card_analysis_profile_v1(candidate, 0);
    if candidate.cost >= 2 {
        Some("snecko_high_cost_candidate_converts_random_cost_debt")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::potions::{Potion, PotionId};
    use crate::content::relics::RelicState;

    #[test]
    fn flags_slow_pyramid_apparition_deck_shape() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state
            .relics
            .push(RelicState::new(RelicId::RunicPyramid));
        for _ in 0..3 {
            run_state.add_card_to_deck(CardId::FeelNoPain);
        }
        for _ in 0..2 {
            run_state.add_card_to_deck(CardId::Anger);
        }
        run_state.add_card_to_deck(CardId::DualWield);
        run_state.add_card_to_deck(CardId::HeavyBlade);
        run_state.add_card_to_deck(CardId::Armaments);
        run_state.add_card_to_deck(CardId::Armaments);
        run_state.add_card_to_deck(CardId::Apparition);

        let profile = deck_startup_profile_v1(&run_state);

        assert!(profile.has_setup_debt_high_payment_low);
        assert!(profile.has_fnp_duplicate_without_exhaust_engine);
        assert!(profile.has_dual_wield_without_target);
        assert!(profile.has_anger_duplicate_without_digest);
        assert!(profile.has_strength_payoff_without_strength);
        assert!(profile.has_armaments_unupgraded_duplicate);
        assert!(profile.has_pyramid_unupgraded_apparition);
    }

    #[test]
    fn recognizes_payment_and_payoff_repairs() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::FeelNoPain);
        run_state.add_card_to_deck(CardId::BurningPact);
        run_state.add_card_to_deck(CardId::BattleTrance);
        run_state.add_card_to_deck(CardId::Inflame);
        run_state.add_card_to_deck(CardId::HeavyBlade);

        let profile = deck_startup_profile_v1(&run_state);

        assert_eq!(profile.exhaust_engine_count, 1);
        assert!(profile.strong_draw_count >= 2);
        assert_eq!(profile.persistent_strength_source_count, 1);
        assert!(!profile.has_fnp_duplicate_without_exhaust_engine);
        assert!(!profile.has_strength_payoff_without_strength);
    }

    #[test]
    fn rupture_requires_self_damage_before_counting_as_strength_source() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::Rupture);
        run_state.add_card_to_deck(CardId::HeavyBlade);

        let without_self_damage = deck_startup_profile_v1(&run_state);

        assert_eq!(without_self_damage.persistent_strength_source_count, 0);
        assert!(without_self_damage.has_rupture_without_self_damage);
        assert!(without_self_damage.has_strength_payoff_without_strength);

        run_state.add_card_to_deck(CardId::Bloodletting);

        let with_self_damage = deck_startup_profile_v1(&run_state);

        assert_eq!(with_self_damage.persistent_strength_source_count, 1);
        assert!(!with_self_damage.has_rupture_without_self_damage);
        assert!(!with_self_damage.has_strength_payoff_without_strength);
    }

    #[test]
    fn flex_is_temporary_burst_not_persistent_strength() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::Flex);
        run_state.add_card_to_deck(CardId::HeavyBlade);

        let profile = deck_startup_profile_v1(&run_state);

        assert_eq!(profile.persistent_strength_source_count, 0);
        assert_eq!(profile.temporary_strength_burst_count, 1);
        assert_eq!(profile.strength_converter_count, 0);
        assert_eq!(profile.convertible_strength_source_count, 0);
        assert_eq!(profile.strength_payoff_count, 1);
        assert!(profile.has_strength_payoff_without_strength);
    }

    #[test]
    fn flex_with_limit_break_is_convertible_strength_not_stable_source() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::Flex);
        run_state.add_card_to_deck(CardId::LimitBreak);

        let profile = deck_startup_profile_v1(&run_state);

        assert_eq!(profile.persistent_strength_source_count, 0);
        assert_eq!(profile.temporary_strength_burst_count, 1);
        assert_eq!(profile.strength_converter_count, 1);
        assert_eq!(profile.convertible_strength_source_count, 1);
        assert!(profile.has_strength_payoff_without_strength);
    }

    #[test]
    fn flex_potion_with_artifact_access_is_convertible_strength() {
        let mut run_state = RunState::new(1, 0, false, "Ironclad");
        run_state.potions[0] = Some(Potion::new(PotionId::SteroidPotion, 1));
        run_state.potions[1] = Some(Potion::new(PotionId::AncientPotion, 2));

        let profile = deck_startup_profile_v1(&run_state);

        assert_eq!(profile.persistent_strength_source_count, 0);
        assert_eq!(profile.temporary_strength_burst_count, 1);
        assert_eq!(profile.strength_converter_count, 1);
        assert_eq!(profile.convertible_strength_source_count, 1);
    }

    #[test]
    fn corruption_duplicates_are_enabler_saturation_not_exhaust_payoff() {
        let mut run_state = RunState::new(2, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::Corruption);
        run_state.add_card_to_deck(CardId::Corruption);

        let profile = deck_startup_profile_v1(&run_state);

        assert_eq!(profile.exhaust_engine_count, 2);
        assert_eq!(profile.exhaust_payoff_count, 0);
        assert!(profile.has_corruption_duplicate_without_payoff);
        assert_eq!(
            startup_liability_for_candidate_v1(&profile, CardId::Corruption, 2),
            Some("startup_rejects_corruption_duplicate_without_payoff")
        );
    }

    #[test]
    fn repeated_status_generators_need_status_digest_capacity() {
        let mut run_state = RunState::new(2, 0, false, "Ironclad");
        run_state.add_card_to_deck(CardId::WildStrike);

        let profile = deck_startup_profile_v1(&run_state);

        assert_eq!(profile.status_generator_count, 1);
        assert_eq!(profile.status_digest_count, 0);
        assert_eq!(
            startup_liability_for_candidate_v1(&profile, CardId::WildStrike, 2),
            Some("startup_rejects_status_generator_duplicate_without_digest")
        );
    }

    #[test]
    fn snecko_eye_exposes_low_cost_volatility_and_discounts_offering_startup() {
        let mut run_state = RunState::new(2, 0, false, "Ironclad");
        run_state.relics.push(RelicState::new(RelicId::SneckoEye));
        run_state.add_card_to_deck(CardId::Offering);
        run_state.add_card_to_deck(CardId::BattleTrance);
        run_state.add_card_to_deck(CardId::ShrugItOff);
        run_state.add_card_to_deck(CardId::PommelStrike);
        run_state.add_card_to_deck(CardId::SpotWeakness);
        run_state.add_card_to_deck(CardId::Inflame);

        let profile = deck_startup_profile_v1(&run_state);

        assert!(profile.has_snecko_eye);
        assert!(profile.low_cost_card_count >= 5);
        assert!(profile.low_cost_card_count > profile.high_cost_card_count);
        assert!(profile.has_snecko_low_cost_volatility);
        assert!(profile.has_snecko_offering_reliability_debt);
        assert!(profile.effective_setup_payment < profile.setup_payment);
        assert!(profile.effective_strong_draw_count < profile.strong_draw_count);
    }

    #[test]
    fn snecko_low_cost_volatility_prevents_offering_from_clean_setup_support() {
        let mut run_state = RunState::new(2, 0, false, "Ironclad");
        run_state.relics.push(RelicState::new(RelicId::SneckoEye));
        run_state.add_card_to_deck(CardId::FeelNoPain);
        run_state.add_card_to_deck(CardId::DarkEmbrace);
        run_state.add_card_to_deck(CardId::DemonForm);
        run_state.add_card_to_deck(CardId::FireBreathing);

        let profile = deck_startup_profile_v1(&run_state);

        assert!(profile.has_setup_debt_high_payment_low);
        assert!(profile.has_snecko_low_cost_volatility);
        assert_eq!(
            startup_support_for_candidate_v1(&profile, CardId::Offering),
            None
        );
    }
}
