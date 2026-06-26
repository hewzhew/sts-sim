use crate::ai::analysis::card_semantics::{
    CardDefinition, CombatEvent, DeckMechanicContext, InstalledRule, Mechanic, PayoffRequirement,
};
use crate::ai::strategy::package_state::{
    assess_package_state, PackageMaturity, PackageStateReport,
};
use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageKind {
    Strength,
    Exhaust,
    SelfDamage,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PackagePromotion {
    pub package: PackageKind,
    pub from: PackageMaturity,
    pub to: PackageMaturity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageTransitionReport {
    pub candidate: CardId,
    pub before: PackageStateReport,
    pub after: PackageStateReport,
    pub promotions: Vec<PackagePromotion>,
    pub newly_closed_requirements: Vec<PayoffRequirement>,
    pub newly_open_requirements: Vec<PayoffRequirement>,
    pub new_mechanics: Vec<Mechanic>,
    pub new_event_streams: Vec<CombatEvent>,
    pub new_installed_rules: Vec<InstalledRule>,
}

pub fn assess_package_transition(
    deck: &[CardDefinition],
    candidate: CardDefinition,
) -> PackageTransitionReport {
    let before_context = DeckMechanicContext::from_definitions(deck);
    let before = assess_package_state(&before_context);

    let mut after_deck = deck.to_vec();
    after_deck.push(candidate.clone());
    let after_context = DeckMechanicContext::from_definitions(&after_deck);
    let after = assess_package_state(&after_context);

    PackageTransitionReport {
        candidate: candidate.card,
        promotions: package_promotions(&before, &after),
        newly_closed_requirements: removed_requirements(
            &before.open_requirements,
            &after.open_requirements,
        ),
        newly_open_requirements: added_requirements(
            &before.open_requirements,
            &after.open_requirements,
        ),
        new_mechanics: added_items(&before_context.mechanics, &after_context.mechanics),
        new_event_streams: added_items(&before_context.event_streams, &after_context.event_streams),
        new_installed_rules: added_items(
            &before_context.installed_rules,
            &after_context.installed_rules,
        ),
        before,
        after,
    }
}

fn package_promotions(
    before: &PackageStateReport,
    after: &PackageStateReport,
) -> Vec<PackagePromotion> {
    let mut promotions = Vec::new();
    push_promotion_if_changed(
        &mut promotions,
        PackageKind::Strength,
        before.strength,
        after.strength,
    );
    push_promotion_if_changed(
        &mut promotions,
        PackageKind::Exhaust,
        before.exhaust,
        after.exhaust,
    );
    push_promotion_if_changed(
        &mut promotions,
        PackageKind::SelfDamage,
        before.self_damage,
        after.self_damage,
    );
    promotions
}

fn push_promotion_if_changed(
    promotions: &mut Vec<PackagePromotion>,
    package: PackageKind,
    from: PackageMaturity,
    to: PackageMaturity,
) {
    if from != to {
        promotions.push(PackagePromotion { package, from, to });
    }
}

fn removed_requirements(
    before: &[PayoffRequirement],
    after: &[PayoffRequirement],
) -> Vec<PayoffRequirement> {
    before
        .iter()
        .copied()
        .filter(|requirement| !after.contains(requirement))
        .collect()
}

fn added_requirements(
    before: &[PayoffRequirement],
    after: &[PayoffRequirement],
) -> Vec<PayoffRequirement> {
    added_items(before, after)
}

fn added_items<T: Copy + Eq>(before: &[T], after: &[T]) -> Vec<T> {
    after
        .iter()
        .copied()
        .filter(|item| !before.contains(item))
        .collect()
}
