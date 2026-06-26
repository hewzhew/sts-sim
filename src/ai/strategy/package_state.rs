use crate::ai::analysis::card_semantics::{
    CombatEvent, DeckMechanicContext, InstalledRule, Mechanic, PayoffRequirement,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PackageMaturity {
    None,
    SourceOnly,
    PayoffOnly,
    Seeded,
    Supported,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PackageStateReport {
    pub strength: PackageMaturity,
    pub exhaust: PackageMaturity,
    pub self_damage: PackageMaturity,
    pub open_requirements: Vec<PayoffRequirement>,
}

pub fn assess_package_state(ctx: &DeckMechanicContext) -> PackageStateReport {
    PackageStateReport {
        strength: assess_strength_package(ctx),
        exhaust: assess_exhaust_package(ctx),
        self_damage: assess_self_damage_package(ctx),
        open_requirements: ctx.open_payoff_requirements.clone(),
    }
}

fn assess_strength_package(ctx: &DeckMechanicContext) -> PackageMaturity {
    let has_source = ctx.mechanics.contains(&Mechanic::Strength);
    let has_payoff = ctx
        .payoff_requirements
        .contains(&PayoffRequirement::WantsMechanic(Mechanic::Strength));
    maturity_from_source_and_payoff(has_source, has_payoff, false)
}

fn assess_exhaust_package(ctx: &DeckMechanicContext) -> PackageMaturity {
    let has_direct_stream = ctx.event_streams.contains(&CombatEvent::CardExhausted);
    let has_installed_seed = ctx
        .installed_rules
        .contains(&InstalledRule::SkillCardsCostZeroAndExhaust);
    let has_payoff = ctx
        .payoff_requirements
        .contains(&PayoffRequirement::WantsEventStream(
            CombatEvent::CardExhausted,
        ));
    maturity_from_source_and_payoff(has_direct_stream, has_payoff, has_installed_seed)
}

fn assess_self_damage_package(ctx: &DeckMechanicContext) -> PackageMaturity {
    let has_source = ctx.event_streams.contains(&CombatEvent::CardSelfDamage);
    let has_payoff = ctx
        .payoff_requirements
        .contains(&PayoffRequirement::WantsEventStream(
            CombatEvent::CardSelfDamage,
        ));
    maturity_from_source_and_payoff(has_source, has_payoff, false)
}

fn maturity_from_source_and_payoff(
    has_source: bool,
    has_payoff: bool,
    has_seed: bool,
) -> PackageMaturity {
    match (has_source, has_payoff, has_seed) {
        (true, true, _) => PackageMaturity::Supported,
        (false, true, true) => PackageMaturity::Seeded,
        (true, false, _) => PackageMaturity::SourceOnly,
        (false, true, false) => PackageMaturity::PayoffOnly,
        (false, false, true) => PackageMaturity::SourceOnly,
        (false, false, false) => PackageMaturity::None,
    }
}
