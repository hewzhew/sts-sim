use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::state::run::RunState;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
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
}

pub fn deck_startup_profile_v1(run_state: &RunState) -> DeckStartupProfileV1 {
    let strength = crate::ai::strength_profile_v1::strength_profile_v1(run_state);
    let deck_shape = crate::ai::deck_shape_v1::deck_shape_profile_v1(run_state);
    let mut profile = DeckStartupProfileV1 {
        has_runic_pyramid: run_state
            .relics
            .iter()
            .any(|relic| relic.id == RelicId::RunicPyramid),
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
        if is_setup_debt_card(id, card.upgrades) {
            profile.setup_debt = profile.setup_debt.saturating_add(1);
        }
        if is_setup_payment_card(id) {
            profile.setup_payment = profile.setup_payment.saturating_add(1);
        }
        if is_immediate_survival_card(id, card.upgrades) {
            profile.immediate_survival = profile.immediate_survival.saturating_add(1);
        }
        if is_combat_shape_risk_card(id, card.upgrades, profile.has_runic_pyramid) {
            profile.combat_shape_risk = profile.combat_shape_risk.saturating_add(1);
        }
        if is_exhaust_engine_card(id) {
            profile.exhaust_engine_count = profile.exhaust_engine_count.saturating_add(1);
        }
        if is_strong_draw_card(id) {
            profile.strong_draw_count = profile.strong_draw_count.saturating_add(1);
        }
        if id == CardId::Rupture {
            profile.rupture_count = profile.rupture_count.saturating_add(1);
        }
        if is_self_damage_source_card(id) {
            profile.self_damage_source_count = profile.self_damage_source_count.saturating_add(1);
        }
        if is_dual_wield_target_card(id) {
            profile.dual_wield_target_count = profile.dual_wield_target_count.saturating_add(1);
        }

        match id {
            CardId::FeelNoPain => {
                profile.feel_no_pain_count = profile.feel_no_pain_count.saturating_add(1)
            }
            CardId::DualWield => {
                profile.dual_wield_count = profile.dual_wield_count.saturating_add(1)
            }
            CardId::Anger => profile.anger_count = profile.anger_count.saturating_add(1),
            CardId::Armaments => {
                profile.armaments_count = profile.armaments_count.saturating_add(1);
                if card.upgrades > 0 {
                    profile.upgraded_armaments_count =
                        profile.upgraded_armaments_count.saturating_add(1);
                }
            }
            CardId::Apparition => {
                profile.apparition_count = profile.apparition_count.saturating_add(1);
                if card.upgrades > 0 {
                    profile.upgraded_apparition_count =
                        profile.upgraded_apparition_count.saturating_add(1);
                }
            }
            _ => {}
        }
    }

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
            .setup_payment
            .saturating_add(profile.strong_draw_count)
            <= 2;
    profile.has_fnp_duplicate_without_exhaust_engine =
        profile.feel_no_pain_count >= 2 && profile.exhaust_engine_count == 0;
    profile.has_corruption_duplicate_without_payoff = deck_shape.risks.iter().any(|risk| {
        matches!(
            risk,
            crate::ai::deck_shape_v1::DeckShapeRiskV1::NonstackingPowerDuplicateWithoutPayoff {
                card: CardId::Corruption,
                ..
            }
        )
    });
    profile.has_havoc_duplicate_without_payoff = deck_shape.risks.iter().any(|risk| {
        matches!(
            risk,
            crate::ai::deck_shape_v1::DeckShapeRiskV1::RandomExhaustSaturationWithoutPayoff {
                card: CardId::Havoc,
                ..
            }
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

pub fn startup_liability_for_candidate_v1(
    startup: &DeckStartupProfileV1,
    candidate: CardId,
    act: u8,
) -> Option<&'static str> {
    match candidate {
        CardId::Corruption
            if startup.exhaust_payoff_count == 0 && startup.corruption_count >= 1 =>
        {
            Some("startup_rejects_corruption_duplicate_without_payoff")
        }
        CardId::Havoc if startup.exhaust_payoff_count == 0 && startup.havoc_count >= 1 => {
            Some("startup_rejects_havoc_duplicate_without_payoff")
        }
        CardId::WildStrike | CardId::RecklessCharge | CardId::PowerThrough | CardId::Immolate
            if startup.status_generator_count >= 1 && startup.status_digest_count == 0 =>
        {
            Some("startup_rejects_status_generator_duplicate_without_digest")
        }
        CardId::Clash if startup.has_clash_playability_debt => {
            Some("startup_rejects_clash_playability_debt")
        }
        CardId::FeelNoPain
            if startup.feel_no_pain_count >= 1 && startup.exhaust_engine_count == 0 =>
        {
            Some("startup_rejects_more_fnp_without_exhaust_engine")
        }
        CardId::FeelNoPain if startup.feel_no_pain_count >= 2 && startup.setup_payment <= 2 => {
            Some("startup_rejects_third_fnp_without_setup_payment")
        }
        CardId::DualWield if startup.dual_wield_target_count == 0 || startup.setup_payment <= 1 => {
            Some("startup_rejects_dual_wield_without_target_or_payment")
        }
        CardId::Anger
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
        CardId::HeavyBlade | CardId::SwordBoomerang | CardId::Pummel
            if startup.persistent_strength_source_count == 0 && act >= 2 =>
        {
            Some("startup_rejects_strength_payoff_without_strength")
        }
        CardId::Rupture if startup.self_damage_source_count == 0 => {
            Some("startup_rejects_rupture_without_self_damage")
        }
        _ => None,
    }
}

pub fn startup_support_for_candidate_v1(
    startup: &DeckStartupProfileV1,
    candidate: CardId,
) -> Option<&'static str> {
    match candidate {
        CardId::Offering | CardId::BattleTrance | CardId::BurningPact
            if startup.has_setup_debt_high_payment_low =>
        {
            Some("startup_supports_setup_payment")
        }
        CardId::BurningPact | CardId::TrueGrit | CardId::SecondWind | CardId::FiendFire
            if startup.feel_no_pain_count > 0 =>
        {
            Some("startup_supports_fnp_exhaust_engine")
        }
        CardId::Inflame | CardId::SpotWeakness | CardId::DemonForm
            if startup.has_strength_payoff_without_strength =>
        {
            Some("startup_supports_strength_source")
        }
        CardId::Rupture
            if startup.has_strength_payoff_without_strength
                && startup.self_damage_source_count > 0 =>
        {
            Some("startup_supports_conditional_strength_source")
        }
        CardId::Bloodletting | CardId::Hemokinesis | CardId::Combust | CardId::Brutality
            if startup.rupture_count > 0 =>
        {
            Some("startup_supports_rupture_self_damage_source")
        }
        CardId::Armaments
            if startup.upgraded_armaments_count == 0 && startup.armaments_count == 0 =>
        {
            Some("startup_supports_upgrade_access")
        }
        _ => None,
    }
}

fn is_setup_debt_card(card: CardId, upgrades: u8) -> bool {
    matches!(
        card,
        CardId::FeelNoPain
            | CardId::DarkEmbrace
            | CardId::DemonForm
            | CardId::Barricade
            | CardId::Metallicize
            | CardId::FireBreathing
            | CardId::Evolve
            | CardId::Rupture
            | CardId::DualWield
            | CardId::LimitBreak
    ) || (card == CardId::Armaments && upgrades == 0)
}

fn is_setup_payment_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Offering
            | CardId::BattleTrance
            | CardId::BurningPact
            | CardId::Bloodletting
            | CardId::SeeingRed
            | CardId::Sentinel
            | CardId::ShrugItOff
            | CardId::PommelStrike
            | CardId::Warcry
    )
}

fn is_immediate_survival_card(card: CardId, upgrades: u8) -> bool {
    matches!(
        card,
        CardId::Impervious
            | CardId::FlameBarrier
            | CardId::PowerThrough
            | CardId::ShrugItOff
            | CardId::Disarm
            | CardId::Shockwave
            | CardId::Uppercut
            | CardId::Clothesline
            | CardId::Intimidate
            | CardId::TrueGrit
            | CardId::SecondWind
    ) || (card == CardId::Apparition && upgrades > 0)
}

fn is_exhaust_engine_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Corruption
            | CardId::BurningPact
            | CardId::TrueGrit
            | CardId::SecondWind
            | CardId::FiendFire
            | CardId::SeverSoul
            | CardId::Havoc
    )
}

fn is_strong_draw_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Offering | CardId::BattleTrance | CardId::BurningPact | CardId::DarkEmbrace
    )
}

fn is_self_damage_source_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Bloodletting
            | CardId::Offering
            | CardId::Hemokinesis
            | CardId::Combust
            | CardId::Brutality
            | CardId::JAX
    )
}

fn is_dual_wield_target_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Feed
            | CardId::Reaper
            | CardId::DemonForm
            | CardId::Barricade
            | CardId::Corruption
            | CardId::LimitBreak
            | CardId::Inflame
            | CardId::SpotWeakness
    )
}

fn is_combat_shape_risk_card(card: CardId, upgrades: u8, has_runic_pyramid: bool) -> bool {
    matches!(
        card,
        CardId::Anger
            | CardId::WildStrike
            | CardId::RecklessCharge
            | CardId::DualWield
            | CardId::Havoc
            | CardId::Clash
    ) || (card == CardId::Apparition && upgrades == 0 && has_runic_pyramid)
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
}
