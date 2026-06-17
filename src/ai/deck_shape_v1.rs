use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::relics::RelicId;
use crate::state::run::RunState;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct DeckShapeProfileV1 {
    pub exhaust_enabler_count: u8,
    pub exhaust_payoff_count: u8,
    pub status_generator_count: u8,
    pub status_digest_count: u8,
    pub corruption_count: u8,
    pub havoc_count: u8,
    pub wild_strike_count: u8,
    pub clash_count: u8,
    pub curse_count: u8,
    pub non_attack_count: u8,
    pub risks: Vec<DeckShapeRiskV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub enum DeckShapeRiskV1 {
    NonstackingPowerDuplicateWithoutPayoff { card: CardId, copies: u8 },
    RandomExhaustSaturationWithoutPayoff { card: CardId, copies: u8 },
    StatusGeneratorSaturationWithoutDigest { copies: u8 },
    ClashPlayabilityDebt { copies: u8 },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeckShapeCandidateDeltaV1 {
    pub candidate: CardId,
    pub risks: Vec<DeckShapeRiskV1>,
    pub labels: Vec<&'static str>,
}

impl DeckShapeCandidateDeltaV1 {
    pub fn has_blocking_risk(&self) -> bool {
        !self.risks.is_empty()
    }
}

pub fn deck_shape_profile_v1(run_state: &RunState) -> DeckShapeProfileV1 {
    let mut profile = DeckShapeProfileV1::default();

    for relic in &run_state.relics {
        match relic.id {
            RelicId::DeadBranch | RelicId::CharonsAshes => {
                profile.exhaust_payoff_count = profile.exhaust_payoff_count.saturating_add(1);
            }
            RelicId::MedicalKit => {
                profile.exhaust_enabler_count = profile.exhaust_enabler_count.saturating_add(1);
                profile.status_digest_count = profile.status_digest_count.saturating_add(1);
            }
            _ => {}
        }
    }

    for card in &run_state.master_deck {
        let id = card.id;
        let definition = get_card_definition(id);
        if definition.card_type != CardType::Attack {
            profile.non_attack_count = profile.non_attack_count.saturating_add(1);
        }
        if definition.card_type == CardType::Curse {
            profile.curse_count = profile.curse_count.saturating_add(1);
        }

        if is_exhaust_enabler_card(id) {
            profile.exhaust_enabler_count = profile.exhaust_enabler_count.saturating_add(1);
        }
        if is_exhaust_payoff_card(id) {
            profile.exhaust_payoff_count = profile.exhaust_payoff_count.saturating_add(1);
        }
        if is_status_generator_card(id) {
            profile.status_generator_count = profile.status_generator_count.saturating_add(1);
        }
        if is_status_digest_card(id) {
            profile.status_digest_count = profile.status_digest_count.saturating_add(1);
        }

        match id {
            CardId::Corruption => {
                profile.corruption_count = profile.corruption_count.saturating_add(1)
            }
            CardId::Havoc => profile.havoc_count = profile.havoc_count.saturating_add(1),
            CardId::WildStrike => {
                profile.wild_strike_count = profile.wild_strike_count.saturating_add(1)
            }
            CardId::Clash => profile.clash_count = profile.clash_count.saturating_add(1),
            _ => {}
        }
    }

    if profile.corruption_count >= 2 && profile.exhaust_payoff_count == 0 {
        profile
            .risks
            .push(DeckShapeRiskV1::NonstackingPowerDuplicateWithoutPayoff {
                card: CardId::Corruption,
                copies: profile.corruption_count,
            });
    }
    if profile.havoc_count >= 2 && profile.exhaust_payoff_count == 0 {
        profile
            .risks
            .push(DeckShapeRiskV1::RandomExhaustSaturationWithoutPayoff {
                card: CardId::Havoc,
                copies: profile.havoc_count,
            });
    }
    if profile.status_generator_count >= 2 && profile.status_digest_count == 0 {
        profile
            .risks
            .push(DeckShapeRiskV1::StatusGeneratorSaturationWithoutDigest {
                copies: profile.status_generator_count,
            });
    }
    if profile.clash_count > 0 && clash_debt_from_profile(&profile) {
        profile.risks.push(DeckShapeRiskV1::ClashPlayabilityDebt {
            copies: profile.clash_count,
        });
    }

    profile
}

pub fn deck_shape_candidate_delta_v1(
    profile: &DeckShapeProfileV1,
    candidate: CardId,
) -> DeckShapeCandidateDeltaV1 {
    let mut delta = DeckShapeCandidateDeltaV1 {
        candidate,
        risks: Vec::new(),
        labels: Vec::new(),
    };

    match candidate {
        CardId::Corruption
            if profile.corruption_count >= 1 && profile.exhaust_payoff_count == 0 =>
        {
            delta
                .labels
                .push("deck_shape_nonstacking_power_duplicate_without_payoff");
            delta
                .risks
                .push(DeckShapeRiskV1::NonstackingPowerDuplicateWithoutPayoff {
                    card: CardId::Corruption,
                    copies: profile.corruption_count.saturating_add(1),
                });
        }
        CardId::Havoc if profile.havoc_count >= 1 && profile.exhaust_payoff_count == 0 => {
            delta
                .labels
                .push("deck_shape_random_exhaust_saturation_without_payoff");
            delta
                .risks
                .push(DeckShapeRiskV1::RandomExhaustSaturationWithoutPayoff {
                    card: CardId::Havoc,
                    copies: profile.havoc_count.saturating_add(1),
                });
        }
        CardId::WildStrike | CardId::RecklessCharge | CardId::PowerThrough | CardId::Immolate
            if profile.status_generator_count >= 1 && profile.status_digest_count == 0 =>
        {
            delta
                .labels
                .push("deck_shape_status_generator_duplicate_without_digest");
            delta
                .risks
                .push(DeckShapeRiskV1::StatusGeneratorSaturationWithoutDigest {
                    copies: profile.status_generator_count.saturating_add(1),
                });
        }
        CardId::Clash if clash_debt_after_candidate(profile) => {
            delta.labels.push("deck_shape_clash_playability_debt");
            delta.risks.push(DeckShapeRiskV1::ClashPlayabilityDebt {
                copies: profile.clash_count.saturating_add(1),
            });
        }
        _ => {}
    }

    delta
}

pub fn is_exhaust_enabler_card(card: CardId) -> bool {
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

pub fn is_exhaust_payoff_card(card: CardId) -> bool {
    matches!(card, CardId::FeelNoPain | CardId::DarkEmbrace)
}

pub fn is_status_generator_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::WildStrike | CardId::RecklessCharge | CardId::PowerThrough | CardId::Immolate
    )
}

pub fn is_status_digest_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Evolve
            | CardId::FireBreathing
            | CardId::BurningPact
            | CardId::TrueGrit
            | CardId::SecondWind
            | CardId::FiendFire
            | CardId::SeverSoul
    )
}

fn clash_debt_from_profile(profile: &DeckShapeProfileV1) -> bool {
    profile.curse_count > 0 || profile.non_attack_count >= 8 || profile.status_generator_count > 0
}

fn clash_debt_after_candidate(profile: &DeckShapeProfileV1) -> bool {
    profile.curse_count > 0
        || profile.non_attack_count.saturating_add(1) >= 8
        || profile.status_generator_count > 0
}
