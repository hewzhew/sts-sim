use crate::ai::analysis::card_semantics::{CardBurden, Mechanic, PlayEffect};
use crate::ai::strategy::package_transition::{PackageKind, PackageTransitionReport};
use crate::content::cards::CardId;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct RewardQualityReport {
    pub thin_payoff_support: Vec<Mechanic>,
    pub duplicate_burdens: Vec<CardBurden>,
}

pub fn assess_reward_quality(
    deck: &[CardId],
    candidate: CardId,
    transition: &PackageTransitionReport,
) -> RewardQualityReport {
    let mut report = RewardQualityReport::default();
    if uses_mechanic(transition, Mechanic::Block) && support_units(deck, Mechanic::Block) < 2 {
        report.thin_payoff_support.push(Mechanic::Block);
    }
    if has_clutter_burden(transition) && has_existing_clutter_burden(deck, candidate) {
        report
            .duplicate_burdens
            .push(CardBurden::AddsCombatDeckClutter);
    }
    report
}

impl RewardQualityReport {
    pub fn suppresses_support(&self, package: PackageKind) -> bool {
        package == PackageKind::Block && self.thin_payoff_support.contains(&Mechanic::Block)
    }

    pub fn suppresses_payoff_effect(&self, effect: &PlayEffect) -> bool {
        matches!(effect, PlayEffect::DamageUses(mechanic) if self.thin_payoff_support.contains(mechanic))
    }

    pub fn has_duplicate_burden(&self) -> bool {
        !self.duplicate_burdens.is_empty()
    }
}

fn uses_mechanic(transition: &PackageTransitionReport, mechanic: Mechanic) -> bool {
    transition
        .candidate_play_effects
        .contains(&PlayEffect::DamageUses(mechanic))
}

fn has_clutter_burden(transition: &PackageTransitionReport) -> bool {
    transition
        .candidate_burdens
        .contains(&CardBurden::AddsCombatDeckClutter)
}

fn has_existing_clutter_burden(deck: &[CardId], candidate: CardId) -> bool {
    card_adds_clutter(candidate) && deck.iter().copied().any(card_adds_clutter)
}

fn support_units(deck: &[CardId], mechanic: Mechanic) -> u8 {
    deck.iter()
        .copied()
        .map(|card| support_units_for_card(card, mechanic))
        .sum()
}

fn support_units_for_card(card: CardId, mechanic: Mechanic) -> u8 {
    match (mechanic, card) {
        (Mechanic::Block, CardId::FlameBarrier | CardId::Impervious | CardId::PowerThrough) => 2,
        (
            Mechanic::Block,
            CardId::ShrugItOff | CardId::TrueGrit | CardId::SecondWind | CardId::IronWave,
        ) => 1,
        _ => 0,
    }
}

fn card_adds_clutter(card: CardId) -> bool {
    matches!(
        card,
        CardId::WildStrike | CardId::RecklessCharge | CardId::PowerThrough
    )
}
