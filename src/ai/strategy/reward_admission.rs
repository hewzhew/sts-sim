use crate::ai::analysis::card_semantics::{
    card_definition, CardBurden, CombatEvent, InstalledRule, Mechanic, PayoffRequirement,
    PlayEffect,
};
use crate::ai::strategy::package_state::PackageMaturity;
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
            RewardAdmissionClass::EngineSeed => 2,
            RewardAdmissionClass::ImmediateWork => 3,
            RewardAdmissionClass::BurdenedImmediateWork => 4,
            RewardAdmissionClass::OpensUnsupportedPayoff => 5,
            RewardAdmissionClass::EmptyOrDeferred => 6,
            RewardAdmissionClass::Skip => 7,
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
            PlayEffect::AddCombatDeckClutter | PlayEffect::PlayTopCardAndExhaust => {}
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
    let engine_seed = !transition.candidate_installed_rules.is_empty()
        || !transition.candidate_event_handlers.is_empty();
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
    } else if supports {
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

fn is_admission_burden(burden: &CardBurden) -> bool {
    matches!(
        burden,
        CardBurden::HpCost
            | CardBurden::DrawLockout
            | CardBurden::AddsCombatDeckClutter
            | CardBurden::RandomExhaust
    )
}
