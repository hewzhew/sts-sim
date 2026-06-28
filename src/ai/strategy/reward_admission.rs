use crate::ai::analysis::card_semantics::{
    card_definition, card_definition_with_upgrades, CardBurden, CombatEvent, DamageScalingAxis,
    InstalledRule, Mechanic, PayoffRequirement, PlayEffect, RunRewardKind,
};
use crate::ai::strategy::package_state::{PackageMaturity, PackageStateReport};
use crate::ai::strategy::package_transition::{assess_package_transition, PackageKind};
use crate::ai::strategy::reward_quality::{assess_reward_quality, RewardDuplicateConcern};
use crate::content::cards::CardId;
use crate::runtime::combat::CombatCard;

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
    pub priority: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RewardAdmissionReason {
    Closes(PayoffRequirement),
    Supports(PackageKind),
    ThinSupport(Mechanic),
    Provides(Mechanic),
    FrontloadDamage,
    AreaDamage,
    DamageScalesWith(DamageScalingAxis),
    CombatUpgrade,
    DamageUses(Mechanic),
    Emits(CombatEvent),
    ExhaustsSelf,
    RunReward(RunRewardKind),
    PlaysTopCardAndExhaust,
    Installs(InstalledRule),
    Opens(PayoffRequirement),
    Burden(CardBurden),
    DuplicateBurden(CardBurden),
    DuplicateConcern(RewardDuplicateConcern),
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
            priority: u8::MAX,
        }
    }

    pub fn opens_unsupported_payoff() -> Self {
        Self {
            tier: RewardAdmissionOrderTierV1::OpensUnsupportedPayoff,
            priority: 0,
        }
    }

    pub fn unscored_optional_reward() -> Self {
        Self {
            tier: RewardAdmissionOrderTierV1::OpensUnsupportedPayoff,
            priority: 0,
        }
    }

    pub fn static_skip_boundary() -> Self {
        Self {
            tier: RewardAdmissionOrderTierV1::StaticSkipBoundary,
            priority: 0,
        }
    }
}

pub fn reward_admission_order_key_v1(admission: &RewardAdmission) -> RewardAdmissionOrderKeyV1 {
    RewardAdmissionOrderKeyV1 {
        tier: admission.class.static_order_tier(),
        priority: admission_reason_priority(admission),
    }
}

pub fn assess_reward_admission(deck: &[CardId], candidate: CardId) -> RewardAdmission {
    assess_reward_admission_with_upgrades(deck, candidate, 0)
}

pub fn assess_reward_admission_with_upgrades(
    deck: &[CardId],
    candidate: CardId,
    candidate_upgrades: u8,
) -> RewardAdmission {
    let deck_defs = deck
        .iter()
        .copied()
        .map(card_definition)
        .collect::<Vec<_>>();
    assess_reward_admission_from_definitions(&deck_defs, candidate, candidate_upgrades)
}

pub fn assess_reward_admission_from_master_deck(
    deck: &[CombatCard],
    candidate: CardId,
    candidate_upgrades: u8,
) -> RewardAdmission {
    let deck_defs = deck
        .iter()
        .map(|card| card_definition_with_upgrades(card.id, card.upgrades))
        .collect::<Vec<_>>();
    assess_reward_admission_from_definitions(&deck_defs, candidate, candidate_upgrades)
}

fn assess_reward_admission_from_definitions(
    deck_defs: &[crate::ai::analysis::card_semantics::CardDefinition],
    candidate: CardId,
    candidate_upgrades: u8,
) -> RewardAdmission {
    let deck_ids = deck_defs
        .iter()
        .map(|definition| definition.card)
        .collect::<Vec<_>>();
    let transition = assess_package_transition(
        deck_defs,
        card_definition_with_upgrades(candidate, candidate_upgrades),
    );
    let quality = assess_reward_quality(&deck_ids, candidate, &transition);
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
                    && !quality.suppresses_support(change.package)
            })
            .map(|change| RewardAdmissionReason::Supports(change.package)),
    );
    for effect in &transition.candidate_play_effects {
        if quality.suppresses_payoff_effect(effect) {
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
            PlayEffect::AreaDamage => reasons.push(RewardAdmissionReason::AreaDamage),
            PlayEffect::DamageUses(mechanic) => {
                reasons.push(RewardAdmissionReason::DamageUses(mechanic))
            }
            PlayEffect::DamageScalesWith(axis) => {
                reasons.push(RewardAdmissionReason::DamageScalesWith(axis))
            }
            PlayEffect::EmitEvent(event) => reasons.push(RewardAdmissionReason::Emits(event)),
            PlayEffect::ExhaustsSelf => reasons.push(RewardAdmissionReason::ExhaustsSelf),
            PlayEffect::RunReward(reward) => reasons.push(RewardAdmissionReason::RunReward(reward)),
            PlayEffect::PlayTopCardAndExhaust => {
                reasons.push(RewardAdmissionReason::PlaysTopCardAndExhaust)
            }
            PlayEffect::CombatUpgradeSingle | PlayEffect::CombatUpgradeAll => {
                reasons.push(RewardAdmissionReason::CombatUpgrade)
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
    for mechanic in &quality.thin_payoff_support {
        reasons.push(RewardAdmissionReason::ThinSupport(*mechanic));
    }
    for burden in &quality.duplicate_burdens {
        reasons.push(RewardAdmissionReason::DuplicateBurden(*burden));
    }
    for concern in &quality.duplicate_concerns {
        reasons.push(RewardAdmissionReason::DuplicateConcern(*concern));
    }

    let closes = !transition.newly_closed_requirements.is_empty();
    let supports = transition.package_changes.iter().any(|change| {
        change.to == PackageMaturity::Supported && !quality.suppresses_support(change.package)
    });
    let supported_payoff = transition.candidate_play_effects.iter().any(|effect| {
        supported_damage_payoff_package(&transition.before, effect).is_some()
            && !quality.suppresses_payoff_effect(effect)
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

    let class = if quality.has_duplicate_penalty() {
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
        RewardAdmissionReason::AreaDamage => "+aoe".to_string(),
        RewardAdmissionReason::DamageScalesWith(axis) => {
            format!("scales:{}", damage_scaling_axis_tag(*axis))
        }
        RewardAdmissionReason::CombatUpgrade => "+upgrade".to_string(),
        RewardAdmissionReason::DamageUses(mechanic) => format!("uses:{}", mechanic_tag(*mechanic)),
        RewardAdmissionReason::Emits(event) => format!("emits:{}", event_tag(*event)),
        RewardAdmissionReason::ExhaustsSelf => "self-exhaust".to_string(),
        RewardAdmissionReason::RunReward(reward) => format!("run:{}", run_reward_tag(*reward)),
        RewardAdmissionReason::PlaysTopCardAndExhaust => "top-card-exhaust".to_string(),
        RewardAdmissionReason::Installs(rule) => format!("installs:{}", rule_tag(*rule)),
        RewardAdmissionReason::Opens(req) => format!("wants:{}", requirement_tag(*req)),
        RewardAdmissionReason::Burden(burden) => format!("risk:{}", burden_tag(*burden)),
        RewardAdmissionReason::DuplicateBurden(burden) => {
            format!("dup-risk:{}", burden_tag(*burden))
        }
        RewardAdmissionReason::DuplicateConcern(concern) => {
            format!("dup:{}", duplicate_concern_tag(*concern))
        }
        RewardAdmissionReason::Empty => "no-model".to_string(),
        RewardAdmissionReason::Skip => "skip-boundary".to_string(),
    }
}

fn admission_reason_priority(admission: &RewardAdmission) -> u8 {
    if admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::Closes(_) | RewardAdmissionReason::Supports(_)
        )
    }) {
        return 0;
    }
    if admission
        .reasons
        .contains(&RewardAdmissionReason::RunReward(
            RunRewardKind::MaxHpOnFatal,
        ))
    {
        return 1;
    }
    if admission
        .reasons
        .contains(&RewardAdmissionReason::CombatUpgrade)
    {
        return 2;
    }
    if admission
        .reasons
        .contains(&RewardAdmissionReason::AreaDamage)
    {
        return 3;
    }
    if admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::Provides(Mechanic::CardDraw | Mechanic::Energy)
        )
    }) {
        return 4;
    }
    if admission
        .reasons
        .contains(&RewardAdmissionReason::FrontloadDamage)
    {
        return 5;
    }
    if admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::Provides(
                Mechanic::Weak | Mechanic::Vulnerable | Mechanic::EnemyStrengthDown
            )
        )
    }) {
        return 6;
    }
    if admission
        .reasons
        .contains(&RewardAdmissionReason::Provides(Mechanic::Block))
    {
        return 7;
    }
    10
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
        CombatEvent::StatusDrawn => "status-drawn",
        CombatEvent::TurnStart => "turn-start",
        CombatEvent::TurnEnd => "turn-end",
    }
}

fn damage_scaling_axis_tag(axis: DamageScalingAxis) -> &'static str {
    match axis {
        DamageScalingAxis::EnergySpent => "energy",
        DamageScalingAxis::HandSize => "hand",
        DamageScalingAxis::PerHitStrength => "per-hit-strength",
    }
}

fn run_reward_tag(reward: RunRewardKind) -> &'static str {
    match reward {
        RunRewardKind::MaxHpOnFatal => "max-hp-kill",
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
        CardBurden::ExhaustsHand => "exhausts-hand",
        CardBurden::RequiresEnemyAttackIntent => "needs-attack",
    }
}

fn duplicate_concern_tag(concern: RewardDuplicateConcern) -> &'static str {
    match concern {
        RewardDuplicateConcern::LowMarginalFrontload => "low-marginal-damage",
        RewardDuplicateConcern::RedundantDebuff => "redundant-debuff",
        RewardDuplicateConcern::RedundantCombatUpgrade => "redundant-upgrade",
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

fn package_has_source(maturity: PackageMaturity) -> bool {
    matches!(
        maturity,
        PackageMaturity::SourceOnly | PackageMaturity::Supported
    )
}

fn is_immediate_work(effect: &PlayEffect) -> bool {
    match effect {
        PlayEffect::FrontloadDamage | PlayEffect::AreaDamage => true,
        PlayEffect::Provide(
            Mechanic::CardDraw
            | Mechanic::Energy
            | Mechanic::Strength
            | Mechanic::Block
            | Mechanic::Weak
            | Mechanic::Vulnerable
            | Mechanic::EnemyStrengthDown,
        ) => true,
        PlayEffect::CombatUpgradeSingle | PlayEffect::CombatUpgradeAll => true,
        PlayEffect::Provide(_)
        | PlayEffect::DamageUses(_)
        | PlayEffect::DamageScalesWith(_)
        | PlayEffect::EmitEvent(_)
        | PlayEffect::ExhaustsSelf
        | PlayEffect::RunReward(_)
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
