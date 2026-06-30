use crate::ai::analysis::card_semantics::PayoffRequirement;
use crate::ai::analysis::card_semantics::{
    card_definition, card_definition_with_upgrades, CombatEvent, DamageScalingAxis, EventHandler,
    InstalledRule, Mechanic, PlayEffect, TriggeredEffect,
};
use crate::ai::strategy::reward_admission::{RewardAdmission, RewardAdmissionReason};
use crate::content::cards::CardId;
use crate::runtime::combat::CombatCard;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PressureLevel {
    Open,
    Thin,
    Present,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FrontloadPressure {
    NeedMore,
    Enough,
    Saturated,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BloatPressure {
    Clean,
    Watch,
    Dense,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum FitLevel {
    None,
    Seed,
    Supports,
    Closes,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConstructionLaneAdjustment {
    None,
    PromoteOneStep,
    PromoteToMainline,
    SoftDemote,
    HardDemote,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AxisEvidence {
    pub has_exhaust_fuel: bool,
    pub has_exhaust_payoff: bool,
    pub has_corruption: bool,
    pub has_strength_source: bool,
    pub has_strength_multiplier: bool,
    pub has_slow_scaling: bool,
    pub has_real_draw: bool,
    pub small_cantrip_count: u8,
    pub has_mitigation: bool,
    pub block_source_count: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct AxisPressure {
    pub level: PressureLevel,
    pub evidence: AxisEvidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckConstructionContext {
    pub act: u8,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckConstructionPressure {
    pub long_fight: AxisPressure,
    pub card_flow: AxisPressure,
    pub defense: AxisPressure,
    pub frontload: FrontloadPressure,
    pub bloat: BloatPressure,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CandidateConstructionFit {
    pub long_fight: FitLevel,
    pub card_flow: FitLevel,
    pub defense: FitLevel,
    pub low_margin_frontload: bool,
    pub duplicate_low_margin: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct DeckConstructionCounts {
    frontload: u8,
    low_margin_frontload: u8,
    block_sources: u8,
    small_cantrips: u8,
    real_draw: u8,
    energy_access: u8,
    mitigation: u8,
    exhaust_fuel: u8,
    exhaust_payoff: u8,
    strength_sources: u8,
    strength_payoffs: u8,
    strength_multipliers: u8,
    slow_scaling: u8,
    corruption: u8,
}

pub fn assess_deck_construction_pressure(
    deck: &[CombatCard],
    context: DeckConstructionContext,
) -> DeckConstructionPressure {
    let counts = deck_counts(deck);
    let evidence = axis_evidence(counts);

    DeckConstructionPressure {
        long_fight: AxisPressure {
            level: long_fight_level(counts, evidence),
            evidence,
        },
        card_flow: AxisPressure {
            level: card_flow_level(counts),
            evidence,
        },
        defense: AxisPressure {
            level: defense_level(counts),
            evidence,
        },
        frontload: frontload_pressure(counts, context),
        bloat: bloat_pressure(deck.len(), context),
    }
}

pub fn assess_candidate_construction_fit(admission: &RewardAdmission) -> CandidateConstructionFit {
    let Some(card) = admission.card else {
        return CandidateConstructionFit {
            long_fight: FitLevel::None,
            card_flow: FitLevel::None,
            defense: FitLevel::None,
            low_margin_frontload: false,
            duplicate_low_margin: false,
        };
    };
    let definition = card_definition(card);
    let mut fit = CandidateConstructionFit {
        long_fight: FitLevel::None,
        card_flow: FitLevel::None,
        defense: FitLevel::None,
        low_margin_frontload: low_margin_frontload_card(card),
        duplicate_low_margin: admission.reasons.iter().any(|reason| {
            matches!(
                reason,
                RewardAdmissionReason::DuplicateBurden(_)
                    | RewardAdmissionReason::DuplicateConcern(_)
            )
        }),
    };

    for reason in &admission.reasons {
        match *reason {
            RewardAdmissionReason::Closes(requirement) => match requirement {
                PayoffRequirement::WantsMechanic(Mechanic::Block) => {
                    fit.defense = fit.defense.max(FitLevel::Closes)
                }
                PayoffRequirement::WantsMechanic(Mechanic::CardDraw | Mechanic::Energy) => {
                    fit.card_flow = fit.card_flow.max(FitLevel::Closes)
                }
                PayoffRequirement::WantsMechanic(Mechanic::Strength)
                | PayoffRequirement::WantsEventStream(
                    CombatEvent::CardExhausted | CombatEvent::CardSelfDamage,
                ) => fit.long_fight = fit.long_fight.max(FitLevel::Closes),
                _ => {}
            },
            RewardAdmissionReason::Supports(package) => {
                use crate::ai::strategy::package_transition::PackageKind;
                match package {
                    PackageKind::Strength | PackageKind::Exhaust | PackageKind::SelfDamage => {
                        fit.long_fight = fit.long_fight.max(FitLevel::Supports)
                    }
                    PackageKind::Block => fit.defense = fit.defense.max(FitLevel::Supports),
                }
            }
            RewardAdmissionReason::Provides(Mechanic::CardDraw | Mechanic::Energy) => {
                fit.card_flow = fit.card_flow.max(FitLevel::Supports)
            }
            RewardAdmissionReason::Provides(
                Mechanic::Block | Mechanic::Weak | Mechanic::EnemyStrengthDown,
            ) => fit.defense = fit.defense.max(FitLevel::Supports),
            RewardAdmissionReason::Installs(InstalledRule::SkillCardsCostZeroAndExhaust) => {
                fit.long_fight = fit.long_fight.max(FitLevel::Seed)
            }
            _ => {}
        }
    }

    for handler in &definition.event_handlers {
        match handler {
            EventHandler {
                on: CombatEvent::TurnStart,
                effect: TriggeredEffect::Provide(Mechanic::Strength),
            } => fit.long_fight = fit.long_fight.max(FitLevel::Seed),
            EventHandler {
                on: CombatEvent::CardExhausted,
                effect: TriggeredEffect::Provide(Mechanic::CardDraw),
            } => fit.card_flow = fit.card_flow.max(FitLevel::Seed),
            EventHandler {
                on: CombatEvent::CardExhausted,
                effect: TriggeredEffect::Provide(Mechanic::Block),
            } => fit.defense = fit.defense.max(FitLevel::Seed),
            _ => {}
        }
    }

    for effect in &definition.play_effects {
        match *effect {
            PlayEffect::DamageScalesWith(
                DamageScalingAxis::EnergySpent
                | DamageScalingAxis::HandSize
                | DamageScalingAxis::PerHitStrength,
            ) => fit.long_fight = fit.long_fight.max(FitLevel::Seed),
            PlayEffect::Provide(Mechanic::Strength) => {
                fit.long_fight = fit.long_fight.max(FitLevel::Seed)
            }
            _ => {}
        }
    }

    if admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::Opens(_) | RewardAdmissionReason::ThinSupport(_)
        )
    }) && fit.long_fight == FitLevel::Seed
    {
        fit.long_fight = FitLevel::None;
    }

    fit
}

pub fn reward_construction_lane_adjustment(
    pressure: DeckConstructionPressure,
    admission: &RewardAdmission,
) -> ConstructionLaneAdjustment {
    if admission.card.is_none() {
        return ConstructionLaneAdjustment::None;
    }
    let fit = assess_candidate_construction_fit(admission);
    if responds_to_open_or_thin_pressure(pressure, fit) {
        if fit.long_fight >= FitLevel::Supports
            || fit.card_flow >= FitLevel::Supports
            || fit.defense >= FitLevel::Supports
        {
            return ConstructionLaneAdjustment::PromoteToMainline;
        }
        return ConstructionLaneAdjustment::PromoteOneStep;
    }
    if should_demote_low_margin_reward(pressure, fit) {
        return if fit.duplicate_low_margin {
            ConstructionLaneAdjustment::HardDemote
        } else {
            ConstructionLaneAdjustment::SoftDemote
        };
    }
    ConstructionLaneAdjustment::None
}

fn responds_to_open_or_thin_pressure(
    pressure: DeckConstructionPressure,
    fit: CandidateConstructionFit,
) -> bool {
    pressure.long_fight.level != PressureLevel::Present && fit.long_fight >= FitLevel::Seed
        || pressure.card_flow.level != PressureLevel::Present && fit.card_flow >= FitLevel::Supports
        || pressure.defense.level != PressureLevel::Present && fit.defense >= FitLevel::Supports
}

fn should_demote_low_margin_reward(
    pressure: DeckConstructionPressure,
    fit: CandidateConstructionFit,
) -> bool {
    if !fit.low_margin_frontload || pressure.frontload == FrontloadPressure::NeedMore {
        return false;
    }
    fit.duplicate_low_margin
        || pressure.frontload == FrontloadPressure::Saturated
        || pressure.bloat == BloatPressure::Dense
}

fn deck_counts(deck: &[CombatCard]) -> DeckConstructionCounts {
    let mut counts = DeckConstructionCounts::default();
    for card in deck {
        let definition = card_definition_with_upgrades(card.id, card.upgrades);
        if low_margin_frontload_card(card.id) {
            counts.low_margin_frontload = counts.low_margin_frontload.saturating_add(1);
        }
        for effect in &definition.play_effects {
            match *effect {
                PlayEffect::FrontloadDamage | PlayEffect::AreaDamage => {
                    counts.frontload = counts.frontload.saturating_add(1)
                }
                PlayEffect::Provide(Mechanic::Block) => {
                    counts.block_sources = counts.block_sources.saturating_add(1)
                }
                PlayEffect::Provide(Mechanic::Weak | Mechanic::EnemyStrengthDown) => {
                    counts.mitigation = counts.mitigation.saturating_add(1)
                }
                PlayEffect::Provide(Mechanic::CardDraw) if real_draw_card(card.id) => {
                    counts.real_draw = counts.real_draw.saturating_add(1)
                }
                PlayEffect::Provide(Mechanic::CardDraw) => {
                    counts.small_cantrips = counts.small_cantrips.saturating_add(1)
                }
                PlayEffect::Provide(Mechanic::Energy) => {
                    counts.energy_access = counts.energy_access.saturating_add(1)
                }
                PlayEffect::Provide(Mechanic::Strength) => {
                    counts.strength_sources = counts.strength_sources.saturating_add(1)
                }
                PlayEffect::Provide(Mechanic::StrengthMultiplier) => {
                    counts.strength_multipliers = counts.strength_multipliers.saturating_add(1)
                }
                PlayEffect::DamageUses(Mechanic::Strength)
                | PlayEffect::DamageScalesWith(DamageScalingAxis::PerHitStrength) => {
                    counts.strength_payoffs = counts.strength_payoffs.saturating_add(1)
                }
                PlayEffect::EmitEvent(CombatEvent::CardExhausted)
                | PlayEffect::PlayTopCardAndExhaust => {
                    counts.exhaust_fuel = counts.exhaust_fuel.saturating_add(1)
                }
                PlayEffect::EmitEvent(_)
                | PlayEffect::DamageUses(_)
                | PlayEffect::DamageScalesWith(_)
                | PlayEffect::ExhaustsSelf
                | PlayEffect::RunReward(_)
                | PlayEffect::RecoverCurrentHp
                | PlayEffect::CostReducedByHpLossThisCombat
                | PlayEffect::AddCombatDeckClutter
                | PlayEffect::CombatUpgradeSingle
                | PlayEffect::CombatUpgradeAll
                | PlayEffect::Provide(
                    Mechanic::TemporaryStrength | Mechanic::Vulnerable | Mechanic::TopdeckControl,
                ) => {}
            }
        }
        for rule in &definition.installed_rules {
            if *rule == InstalledRule::SkillCardsCostZeroAndExhaust {
                counts.corruption = counts.corruption.saturating_add(1);
                counts.exhaust_fuel = counts.exhaust_fuel.saturating_add(1);
            }
        }
        for handler in &definition.event_handlers {
            match handler {
                EventHandler {
                    on: CombatEvent::CardExhausted,
                    effect: TriggeredEffect::Provide(Mechanic::Block | Mechanic::CardDraw),
                } => counts.exhaust_payoff = counts.exhaust_payoff.saturating_add(1),
                EventHandler {
                    on: CombatEvent::TurnStart,
                    effect: TriggeredEffect::Provide(Mechanic::Strength),
                } => counts.slow_scaling = counts.slow_scaling.saturating_add(1),
                _ => {}
            }
        }
    }
    counts
}

fn axis_evidence(counts: DeckConstructionCounts) -> AxisEvidence {
    AxisEvidence {
        has_exhaust_fuel: counts.exhaust_fuel > 0,
        has_exhaust_payoff: counts.exhaust_payoff > 0,
        has_corruption: counts.corruption > 0,
        has_strength_source: counts.strength_sources > 0,
        has_strength_multiplier: counts.strength_multipliers > 0,
        has_slow_scaling: counts.slow_scaling > 0,
        has_real_draw: counts.real_draw > 0,
        small_cantrip_count: counts.small_cantrips,
        has_mitigation: counts.mitigation > 0,
        block_source_count: counts.block_sources,
    }
}

fn long_fight_level(counts: DeckConstructionCounts, evidence: AxisEvidence) -> PressureLevel {
    let exhaust_engine = evidence.has_exhaust_fuel && evidence.has_exhaust_payoff;
    let strength_engine = evidence.has_strength_source && counts.strength_payoffs > 0;
    let slow_scaling_supported =
        evidence.has_slow_scaling && (counts.block_sources >= 2 || counts.real_draw > 0);
    if exhaust_engine || strength_engine || slow_scaling_supported {
        PressureLevel::Present
    } else if evidence.has_corruption
        || evidence.has_slow_scaling
        || evidence.has_strength_source
        || evidence.has_strength_multiplier
        || counts.strength_payoffs > 0
    {
        PressureLevel::Thin
    } else {
        PressureLevel::Open
    }
}

fn card_flow_level(counts: DeckConstructionCounts) -> PressureLevel {
    if counts.real_draw >= 2 || (counts.real_draw >= 1 && counts.energy_access >= 1) {
        PressureLevel::Present
    } else if counts.real_draw >= 1 || counts.small_cantrips >= 2 || counts.energy_access >= 1 {
        PressureLevel::Thin
    } else {
        PressureLevel::Open
    }
}

fn defense_level(counts: DeckConstructionCounts) -> PressureLevel {
    let exhaust_defense_engine = counts.exhaust_fuel > 0 && counts.exhaust_payoff > 0;
    if exhaust_defense_engine || (counts.block_sources >= 3 && counts.mitigation > 0) {
        PressureLevel::Present
    } else if counts.block_sources > 0 || counts.mitigation > 0 {
        PressureLevel::Thin
    } else {
        PressureLevel::Open
    }
}

fn frontload_pressure(
    counts: DeckConstructionCounts,
    context: DeckConstructionContext,
) -> FrontloadPressure {
    let enough = if context.act <= 1 { 3 } else { 2 };
    let saturated = if context.act <= 1 { 6 } else { 5 };
    if counts.frontload < enough {
        FrontloadPressure::NeedMore
    } else if counts.low_margin_frontload >= saturated || counts.frontload >= saturated + 2 {
        FrontloadPressure::Saturated
    } else {
        FrontloadPressure::Enough
    }
}

fn bloat_pressure(deck_size: usize, context: DeckConstructionContext) -> BloatPressure {
    let (watch, dense) = if context.act <= 1 { (28, 34) } else { (22, 28) };
    if deck_size >= dense {
        BloatPressure::Dense
    } else if deck_size >= watch {
        BloatPressure::Watch
    } else {
        BloatPressure::Clean
    }
}

fn real_draw_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::Offering
            | CardId::BattleTrance
            | CardId::BurningPact
            | CardId::DarkEmbrace
            | CardId::MasterOfStrategy
            | CardId::Acrobatics
            | CardId::CalculatedGamble
    )
}

fn low_margin_frontload_card(card: CardId) -> bool {
    matches!(
        card,
        CardId::TwinStrike
            | CardId::SwordBoomerang
            | CardId::WildStrike
            | CardId::RecklessCharge
            | CardId::Rampage
            | CardId::IronWave
            | CardId::Clothesline
            | CardId::ThunderClap
            | CardId::Anger
            | CardId::SwiftStrike
    )
}
