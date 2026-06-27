use crate::ai::analysis::card_semantics::{
    card_definition, CardBurden, CombatEvent, InstalledRule, Mechanic, PayoffRequirement,
    PlayEffect,
};
use crate::ai::strategy::package_state::{PackageMaturity, PackageStateReport};
use crate::ai::strategy::package_transition::{assess_package_transition, PackageKind};
use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewardAdmissionClass {
    ClosesRequirement,
    BuildsSupportedPackage,
    EngineSeed,
    ImmediateWork,
    BurdenedImmediateWork,
    OpensUnsupportedPayoff,
    EmptyOrDeferred,
    Skip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewardAdmissionReason {
    Closes(PayoffRequirement),
    Supports(PackageKind),
    Provides(Mechanic),
    FrontloadDamage,
    DamageUses(Mechanic),
    Emits(CombatEvent),
    PlaysTopCardAndExhaust,
    Installs(InstalledRule),
    Opens(PayoffRequirement),
    Burden(CardBurden),
    Empty,
    Skip,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RewardAdmission {
    pub card: Option<CardId>,
    pub class: RewardAdmissionClass,
    pub reasons: Vec<RewardAdmissionReason>,
}

impl RewardAdmissionClass {
    pub fn rank(self) -> u8 {
        match self {
            RewardAdmissionClass::ClosesRequirement => 0,
            RewardAdmissionClass::BuildsSupportedPackage => 1,
            RewardAdmissionClass::ImmediateWork => 2,
            RewardAdmissionClass::EngineSeed => 3,
            RewardAdmissionClass::BurdenedImmediateWork => 4,
            RewardAdmissionClass::Skip => 5,
            RewardAdmissionClass::OpensUnsupportedPayoff => 6,
            RewardAdmissionClass::EmptyOrDeferred => 7,
        }
    }
}

pub fn assess_reward_admission(deck: &[CardId], candidate: CardId) -> RewardAdmission {
    let deck = deck
        .iter()
        .copied()
        .map(card_definition)
        .collect::<Vec<_>>();
    let transition = assess_package_transition(&deck, card_definition(candidate));
    let mut reasons = Vec::new();
    reasons.extend(
        transition
            .newly_closed_requirements
            .iter()
            .copied()
            .map(RewardAdmissionReason::Closes),
    );
    reasons.extend(
        transition
            .package_changes
            .iter()
            .filter(|change| change.to == PackageMaturity::Supported)
            .map(|change| RewardAdmissionReason::Supports(change.package)),
    );
    for effect in &transition.candidate_play_effects {
        let Some(package) = supported_damage_payoff_package(&transition.before, effect) else {
            continue;
        };
        let reason = RewardAdmissionReason::Supports(package);
        if !reasons.contains(&reason) {
            reasons.push(reason);
        }
    }
    reasons.extend(
        transition
            .newly_open_requirements
            .iter()
            .copied()
            .map(RewardAdmissionReason::Opens),
    );
    for effect in &transition.candidate_play_effects {
        match *effect {
            PlayEffect::Provide(mechanic) => {
                reasons.push(RewardAdmissionReason::Provides(mechanic))
            }
            PlayEffect::FrontloadDamage => reasons.push(RewardAdmissionReason::FrontloadDamage),
            PlayEffect::DamageUses(mechanic) => {
                reasons.push(RewardAdmissionReason::DamageUses(mechanic))
            }
            PlayEffect::EmitEvent(event) => reasons.push(RewardAdmissionReason::Emits(event)),
            PlayEffect::PlayTopCardAndExhaust => {
                reasons.push(RewardAdmissionReason::PlaysTopCardAndExhaust)
            }
            PlayEffect::AddCombatDeckClutter => {}
        }
    }
    reasons.extend(
        transition
            .candidate_installed_rules
            .iter()
            .copied()
            .map(RewardAdmissionReason::Installs),
    );
    reasons.extend(
        transition
            .candidate_burdens
            .iter()
            .copied()
            .map(RewardAdmissionReason::Burden),
    );

    let closes = !transition.newly_closed_requirements.is_empty();
    let supports = transition
        .package_changes
        .iter()
        .any(|change| change.to == PackageMaturity::Supported);
    let supported_payoff = transition
        .candidate_play_effects
        .iter()
        .any(|effect| supported_damage_payoff_package(&transition.before, effect).is_some());
    let engine_seed = !transition.candidate_installed_rules.is_empty()
        || !transition.candidate_event_handlers.is_empty()
        || transition
            .candidate_play_effects
            .iter()
            .any(is_engine_seed_effect);
    let immediate = transition
        .candidate_play_effects
        .iter()
        .any(is_immediate_work);
    let payoff_only = transition
        .candidate_play_effects
        .iter()
        .any(|effect| matches!(effect, PlayEffect::DamageUses(_)))
        && !immediate;
    let burdened = transition.candidate_burdens.iter().any(is_admission_burden);
    let opens = !transition.newly_open_requirements.is_empty();

    let class = if closes {
        RewardAdmissionClass::ClosesRequirement
    } else if supports || supported_payoff {
        RewardAdmissionClass::BuildsSupportedPackage
    } else if engine_seed && !immediate {
        RewardAdmissionClass::EngineSeed
    } else if immediate && burdened {
        RewardAdmissionClass::BurdenedImmediateWork
    } else if immediate {
        RewardAdmissionClass::ImmediateWork
    } else if engine_seed {
        RewardAdmissionClass::EngineSeed
    } else if opens || payoff_only {
        RewardAdmissionClass::OpensUnsupportedPayoff
    } else {
        reasons.push(RewardAdmissionReason::Empty);
        RewardAdmissionClass::EmptyOrDeferred
    };

    RewardAdmission {
        card: Some(candidate),
        class,
        reasons,
    }
}

pub fn skip_reward_admission() -> RewardAdmission {
    RewardAdmission {
        card: None,
        class: RewardAdmissionClass::Skip,
        reasons: vec![RewardAdmissionReason::Skip],
    }
}

pub fn render_reward_admission_compact(admission: &RewardAdmission) -> String {
    let reasons = admission
        .reasons
        .iter()
        .take(3)
        .map(|reason| format!("{reason:?}"))
        .collect::<Vec<_>>();
    if reasons.is_empty() {
        format!("{:?}", admission.class)
    } else {
        format!("{:?}:{}", admission.class, reasons.join(","))
    }
}

fn supported_damage_payoff_package(
    before: &PackageStateReport,
    effect: &PlayEffect,
) -> Option<PackageKind> {
    match effect {
        PlayEffect::DamageUses(Mechanic::Strength) if package_has_source(before.strength) => {
            Some(PackageKind::Strength)
        }
        PlayEffect::DamageUses(Mechanic::Block) if package_has_source(before.block) => {
            Some(PackageKind::Block)
        }
        _ => None,
    }
}

fn package_has_source(maturity: PackageMaturity) -> bool {
    matches!(
        maturity,
        PackageMaturity::SourceOnly | PackageMaturity::Supported
    )
}

fn is_immediate_work(effect: &PlayEffect) -> bool {
    match effect {
        PlayEffect::FrontloadDamage => true,
        PlayEffect::Provide(
            Mechanic::CardDraw
            | Mechanic::Energy
            | Mechanic::Block
            | Mechanic::Weak
            | Mechanic::Vulnerable
            | Mechanic::EnemyStrengthDown,
        ) => true,
        PlayEffect::Provide(_)
        | PlayEffect::DamageUses(_)
        | PlayEffect::EmitEvent(_)
        | PlayEffect::AddCombatDeckClutter
        | PlayEffect::PlayTopCardAndExhaust => false,
    }
}

fn is_engine_seed_effect(effect: &PlayEffect) -> bool {
    matches!(
        effect,
        PlayEffect::Provide(Mechanic::Strength | Mechanic::StrengthMultiplier)
            | PlayEffect::PlayTopCardAndExhaust
    )
}

fn is_admission_burden(burden: &CardBurden) -> bool {
    matches!(
        burden,
        CardBurden::AddsCombatDeckClutter | CardBurden::RandomExhaust
    )
}
