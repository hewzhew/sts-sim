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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum RewardAdmissionOrderTierV1 {
    ClosesRequirement,
    BuildsSupportedPackage,
    ImmediateWork,
    EngineSeed,
    BurdenedImmediateWork,
    StaticSkipBoundary,
    OpensUnsupportedPayoff,
    EmptyOrDeferred,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RewardAdmissionOrderKeyV1 {
    pub tier: RewardAdmissionOrderTierV1,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewardAdmissionReason {
    Closes(PayoffRequirement),
    Supports(PackageKind),
    ThinSupport(Mechanic),
    Provides(Mechanic),
    FrontloadDamage,
    DamageUses(Mechanic),
    Emits(CombatEvent),
    PlaysTopCardAndExhaust,
    Installs(InstalledRule),
    Opens(PayoffRequirement),
    Burden(CardBurden),
    DuplicateBurden(CardBurden),
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
    fn static_order_tier(self) -> RewardAdmissionOrderTierV1 {
        match self {
            RewardAdmissionClass::ClosesRequirement => {
                RewardAdmissionOrderTierV1::ClosesRequirement
            }
            RewardAdmissionClass::BuildsSupportedPackage => {
                RewardAdmissionOrderTierV1::BuildsSupportedPackage
            }
            RewardAdmissionClass::ImmediateWork => RewardAdmissionOrderTierV1::ImmediateWork,
            RewardAdmissionClass::EngineSeed => RewardAdmissionOrderTierV1::EngineSeed,
            RewardAdmissionClass::BurdenedImmediateWork => {
                RewardAdmissionOrderTierV1::BurdenedImmediateWork
            }
            RewardAdmissionClass::Skip => RewardAdmissionOrderTierV1::StaticSkipBoundary,
            RewardAdmissionClass::OpensUnsupportedPayoff => {
                RewardAdmissionOrderTierV1::OpensUnsupportedPayoff
            }
            RewardAdmissionClass::EmptyOrDeferred => RewardAdmissionOrderTierV1::EmptyOrDeferred,
        }
    }
}

impl RewardAdmissionOrderKeyV1 {
    pub fn empty_or_deferred() -> Self {
        Self {
            tier: RewardAdmissionOrderTierV1::EmptyOrDeferred,
        }
    }

    pub fn opens_unsupported_payoff() -> Self {
        Self {
            tier: RewardAdmissionOrderTierV1::OpensUnsupportedPayoff,
        }
    }

    pub fn unscored_optional_reward() -> Self {
        Self {
            tier: RewardAdmissionOrderTierV1::OpensUnsupportedPayoff,
        }
    }

    pub fn static_skip_boundary() -> Self {
        Self {
            tier: RewardAdmissionOrderTierV1::StaticSkipBoundary,
        }
    }
}

pub fn reward_admission_order_key_v1(admission: &RewardAdmission) -> RewardAdmissionOrderKeyV1 {
    RewardAdmissionOrderKeyV1 {
        tier: admission.class.static_order_tier(),
    }
}

pub fn assess_reward_admission(deck: &[CardId], candidate: CardId) -> RewardAdmission {
    let thin_block_payoff = thin_block_payoff_support(deck, candidate);
    let duplicate_clutter = duplicate_clutter_frontload(deck, candidate);
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
            .filter(|change| {
                change.to == PackageMaturity::Supported
                    && !(thin_block_payoff && change.package == PackageKind::Block)
            })
            .map(|change| RewardAdmissionReason::Supports(change.package)),
    );
    for effect in &transition.candidate_play_effects {
        if thin_block_payoff && matches!(effect, PlayEffect::DamageUses(Mechanic::Block)) {
            continue;
        }
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
    if thin_block_payoff {
        reasons.push(RewardAdmissionReason::ThinSupport(Mechanic::Block));
    }
    if duplicate_clutter {
        reasons.push(RewardAdmissionReason::DuplicateBurden(
            CardBurden::AddsCombatDeckClutter,
        ));
    }

    let closes = !transition.newly_closed_requirements.is_empty();
    let supports = transition.package_changes.iter().any(|change| {
        change.to == PackageMaturity::Supported
            && !(thin_block_payoff && change.package == PackageKind::Block)
    });
    let supported_payoff = transition.candidate_play_effects.iter().any(|effect| {
        supported_damage_payoff_package(&transition.before, effect).is_some()
            && !(thin_block_payoff && matches!(effect, PlayEffect::DamageUses(Mechanic::Block)))
    });
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

    let class = if duplicate_clutter {
        RewardAdmissionClass::EmptyOrDeferred
    } else if closes {
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
        .take(4)
        .map(reason_tag)
        .collect::<Vec<_>>();
    if reasons.is_empty() {
        admission_class_label(admission.class).to_string()
    } else {
        format!(
            "{} | {}",
            admission_class_label(admission.class),
            reasons.join(" ")
        )
    }
}

fn admission_class_label(class: RewardAdmissionClass) -> &'static str {
    match class {
        RewardAdmissionClass::ClosesRequirement => "Closes",
        RewardAdmissionClass::BuildsSupportedPackage => "Supported",
        RewardAdmissionClass::EngineSeed => "Seed",
        RewardAdmissionClass::ImmediateWork => "Immediate",
        RewardAdmissionClass::BurdenedImmediateWork => "Burdened",
        RewardAdmissionClass::OpensUnsupportedPayoff => "Unsupported",
        RewardAdmissionClass::EmptyOrDeferred => "Empty",
        RewardAdmissionClass::Skip => "Skip",
    }
}

fn reason_tag(reason: &RewardAdmissionReason) -> String {
    match reason {
        RewardAdmissionReason::Closes(req) => format!("closes:{}", requirement_tag(*req)),
        RewardAdmissionReason::Supports(package) => format!("pkg:{}", package_tag(*package)),
        RewardAdmissionReason::ThinSupport(mechanic) => format!("thin:{}", mechanic_tag(*mechanic)),
        RewardAdmissionReason::Provides(mechanic) => format!("+{}", mechanic_tag(*mechanic)),
        RewardAdmissionReason::FrontloadDamage => "+damage".to_string(),
        RewardAdmissionReason::DamageUses(mechanic) => format!("uses:{}", mechanic_tag(*mechanic)),
        RewardAdmissionReason::Emits(event) => format!("emits:{}", event_tag(*event)),
        RewardAdmissionReason::PlaysTopCardAndExhaust => "top-card-exhaust".to_string(),
        RewardAdmissionReason::Installs(rule) => format!("installs:{}", rule_tag(*rule)),
        RewardAdmissionReason::Opens(req) => format!("wants:{}", requirement_tag(*req)),
        RewardAdmissionReason::Burden(burden) => format!("risk:{}", burden_tag(*burden)),
        RewardAdmissionReason::DuplicateBurden(burden) => {
            format!("dup-risk:{}", burden_tag(*burden))
        }
        RewardAdmissionReason::Empty => "no-model".to_string(),
        RewardAdmissionReason::Skip => "skip-boundary".to_string(),
    }
}

fn mechanic_tag(mechanic: Mechanic) -> &'static str {
    match mechanic {
        Mechanic::Strength => "strength",
        Mechanic::TemporaryStrength => "temp-strength",
        Mechanic::StrengthMultiplier => "strength-mult",
        Mechanic::CardDraw => "draw",
        Mechanic::Energy => "energy",
        Mechanic::Block => "block",
        Mechanic::Weak => "weak",
        Mechanic::Vulnerable => "vuln",
        Mechanic::EnemyStrengthDown => "str-down",
        Mechanic::TopdeckControl => "topdeck",
    }
}

fn event_tag(event: CombatEvent) -> &'static str {
    match event {
        CombatEvent::CardExhausted => "exhaust",
        CombatEvent::CardSelfDamage => "self-damage",
        CombatEvent::TurnStart => "turn-start",
        CombatEvent::TurnEnd => "turn-end",
    }
}

fn requirement_tag(requirement: PayoffRequirement) -> String {
    match requirement {
        PayoffRequirement::WantsMechanic(mechanic) => mechanic_tag(mechanic).to_string(),
        PayoffRequirement::WantsEventStream(event) => event_tag(event).to_string(),
    }
}

fn package_tag(package: PackageKind) -> &'static str {
    match package {
        PackageKind::Strength => "strength",
        PackageKind::Exhaust => "exhaust",
        PackageKind::SelfDamage => "self-damage",
        PackageKind::Block => "block",
    }
}

fn burden_tag(burden: CardBurden) -> &'static str {
    match burden {
        CardBurden::PowerSetup => "setup",
        CardBurden::HpCost => "hp-cost",
        CardBurden::DrawLockout => "draw-lock",
        CardBurden::AddsCombatDeckClutter => "deck-clutter",
        CardBurden::RandomExhaust => "random-exhaust",
        CardBurden::RequiresEnemyAttackIntent => "needs-attack",
    }
}

fn rule_tag(rule: InstalledRule) -> &'static str {
    match rule {
        InstalledRule::SkillCardsCostZeroAndExhaust => "skills-free-exhaust",
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

fn thin_block_payoff_support(deck: &[CardId], candidate: CardId) -> bool {
    candidate == CardId::BodySlam && block_support_units(deck) < 2
}

fn block_support_units(deck: &[CardId]) -> u8 {
    deck.iter().copied().map(block_source_units).sum()
}

fn block_source_units(card: CardId) -> u8 {
    match card {
        CardId::FlameBarrier | CardId::Impervious | CardId::PowerThrough => 2,
        CardId::ShrugItOff | CardId::TrueGrit | CardId::SecondWind | CardId::IronWave => 1,
        _ => 0,
    }
}

fn duplicate_clutter_frontload(deck: &[CardId], candidate: CardId) -> bool {
    matches!(candidate, CardId::WildStrike | CardId::RecklessCharge)
        && deck
            .iter()
            .any(|card| matches!(card, CardId::WildStrike | CardId::RecklessCharge))
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
