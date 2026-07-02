use serde::Serialize;

use crate::ai::analysis::card_semantics::{card_definition_with_upgrades, Mechanic, PlayEffect};
use crate::ai::strategy::deck_role_inventory::DeckRoleInventory;
use crate::ai::strategy::exhaust_corruption_assessment::{
    assess_exhaust_corruption, ExhaustCorruptionAssessment, ExhaustCorruptionRisk,
    ExhaustCorruptionState,
};
use crate::ai::strategy::run_strategic_facts::RunStrategicFacts;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::runtime::combat::CombatCard;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicDeficitLevel {
    Missing,
    Thin,
    Adequate,
    Surplus,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicBurdenLevel {
    Clean,
    Watch,
    Heavy,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicPackageEvidence {
    ExhaustEngine,
    StrengthScaling,
    BlockEngine,
    #[serde(rename = "aoe_package")]
    AoEPackage,
    DrawEngine,
    EnergyEngine,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategicRisk {
    TooManyLowImpactAttacks,
    TooManyConditionalPayoffs,
    NoEnablerForExploiters,
    DeckTooThickForAccess,
    OpeningHandPollution,
    SevereCurseBurden,
    ReliesOnPowers,
    ReliesOnLowImpactCardSpam,
    CorruptionWithoutExhaustPayoff,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct DeckStrategicDeficit {
    pub frontload_damage: StrategicDeficitLevel,
    pub aoe_or_minion_control: StrategicDeficitLevel,
    pub block_or_mitigation: StrategicDeficitLevel,
    pub boss_scaling_plan: StrategicDeficitLevel,
    pub deck_access: StrategicDeficitLevel,
    pub energy_or_playability: StrategicDeficitLevel,
    pub deck_burden: StrategicBurdenLevel,
    pub exhaust_corruption: ExhaustCorruptionAssessment,
    pub package_evidence: Vec<StrategicPackageEvidence>,
    pub risks: Vec<StrategicRisk>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckStrategicDeficitSummary {
    pub frontload_damage: StrategicDeficitLevel,
    pub aoe_or_minion_control: StrategicDeficitLevel,
    pub block_or_mitigation: StrategicDeficitLevel,
    pub boss_scaling_plan: StrategicDeficitLevel,
    pub deck_access: StrategicDeficitLevel,
    pub energy_or_playability: StrategicDeficitLevel,
    pub deck_burden: StrategicBurdenLevel,
    pub too_many_low_impact_attacks: bool,
    pub opening_hand_pollution: bool,
    pub severe_curse_burden: bool,
}

impl DeckStrategicDeficit {
    pub fn summary(&self) -> DeckStrategicDeficitSummary {
        DeckStrategicDeficitSummary {
            frontload_damage: self.frontload_damage,
            aoe_or_minion_control: self.aoe_or_minion_control,
            block_or_mitigation: self.block_or_mitigation,
            boss_scaling_plan: self.boss_scaling_plan,
            deck_access: self.deck_access,
            energy_or_playability: self.energy_or_playability,
            deck_burden: self.deck_burden,
            too_many_low_impact_attacks: self
                .risks
                .contains(&StrategicRisk::TooManyLowImpactAttacks),
            opening_hand_pollution: self.risks.contains(&StrategicRisk::OpeningHandPollution),
            severe_curse_burden: self.risks.contains(&StrategicRisk::SevereCurseBurden),
        }
    }
}

pub fn assess_deck_strategic_deficit(
    deck: &[CombatCard],
    facts: RunStrategicFacts,
) -> DeckStrategicDeficit {
    let inventory = DeckRoleInventory::from_deck(deck);
    let counts = StrategicCounts::from_deck(deck, facts);
    let exhaust_corruption = assess_exhaust_corruption(deck);
    let frontload_units = inventory
        .frontload_units
        .saturating_add(counts.strike_count);
    let block_or_mitigation_units = block_or_mitigation_units(&inventory, &counts);
    let package_evidence = package_evidence(&inventory, &counts, &exhaust_corruption);
    let boss_scaling_plan = boss_scaling_level(&package_evidence, &counts);
    let deck_access = access_level(inventory.draw_units, counts.deck_size);
    let energy_or_playability = energy_level(&inventory, &counts);
    let risks = risks(&inventory, &counts, deck_access, &exhaust_corruption);

    DeckStrategicDeficit {
        frontload_damage: frontload_level(frontload_units, counts.act),
        aoe_or_minion_control: unit_level(inventory.aoe_units, 1, 4),
        block_or_mitigation: block_or_mitigation_level(block_or_mitigation_units, counts.act),
        boss_scaling_plan,
        deck_access,
        energy_or_playability,
        deck_burden: burden_level(&counts, deck_access),
        exhaust_corruption,
        package_evidence,
        risks,
    }
}

pub fn assess_deck_strategic_deficit_summary(
    deck: &[CombatCard],
    facts: RunStrategicFacts,
) -> DeckStrategicDeficitSummary {
    assess_deck_strategic_deficit(deck, facts).summary()
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct StrategicCounts {
    deck_size: usize,
    act: u8,
    curse_count: usize,
    starter_basic_count: usize,
    power_count: u8,
    strike_count: u8,
    defend_count: u8,
    iron_wave_count: u8,
    writhe_count: u8,
    normality_count: u8,
    severe_curse_count: u8,
    low_impact_attacks: u8,
    conditional_payoffs: u8,
    strength_sources: u8,
    status_clutter_sources: u8,
    has_energy_relic: bool,
    has_corruption: bool,
    has_dark_embrace: bool,
    has_ritual_dagger: bool,
}

impl StrategicCounts {
    fn from_deck(deck: &[CombatCard], facts: RunStrategicFacts) -> Self {
        let mut counts = Self {
            deck_size: deck.len(),
            act: facts.entering_act,
            curse_count: facts.curse_count,
            starter_basic_count: facts.starter_basic_count,
            has_energy_relic: facts.has_energy_relic,
            ..Self::default()
        };
        for card in deck {
            counts.add_card(card);
        }
        counts
    }

    fn add_card(&mut self, card: &CombatCard) {
        let definition = get_card_definition(card.id);
        if definition.card_type == CardType::Power {
            self.power_count += 1;
        }
        if card.id == CardId::Strike {
            self.strike_count += 1;
        }
        if card.id == CardId::Defend {
            self.defend_count += 1;
        }
        if card.id == CardId::IronWave {
            self.iron_wave_count += 1;
        }
        if card.id == CardId::Writhe {
            self.writhe_count += 1;
        }
        if card.id == CardId::Normality {
            self.normality_count += 1;
        }
        if is_severe_curse(card.id) {
            self.severe_curse_count += 1;
        }
        if is_low_impact_attack(card.id) {
            self.low_impact_attacks += 1;
        }
        if is_conditional_payoff(card.id) {
            self.conditional_payoffs += 1;
        }
        if card.id == CardId::Corruption {
            self.has_corruption = true;
        }
        if card.id == CardId::DarkEmbrace {
            self.has_dark_embrace = true;
        }
        if card.id == CardId::RitualDagger {
            self.has_ritual_dagger = true;
        }
        let semantics = card_definition_with_upgrades(card.id, card.upgrades);
        for effect in semantics.play_effects {
            match effect {
                PlayEffect::Provide(
                    Mechanic::Strength | Mechanic::TemporaryStrength | Mechanic::StrengthMultiplier,
                ) => self.strength_sources += 1,
                PlayEffect::AddCombatDeckClutter => self.status_clutter_sources += 1,
                _ => {}
            }
        }
    }
}

fn package_evidence(
    inventory: &DeckRoleInventory,
    counts: &StrategicCounts,
    exhaust_corruption: &ExhaustCorruptionAssessment,
) -> Vec<StrategicPackageEvidence> {
    let mut evidence = Vec::new();
    if matches!(
        exhaust_corruption.state,
        ExhaustCorruptionState::SupportedButSlow | ExhaustCorruptionState::EngineOnline
    ) || (counts.has_dark_embrace && inventory.exhaust_stream_units >= 1)
    {
        evidence.push(StrategicPackageEvidence::ExhaustEngine);
    }
    if counts.strength_sources > 0
        || (inventory.strength_payoff_units > 0 && counts.strength_sources > 0)
    {
        evidence.push(StrategicPackageEvidence::StrengthScaling);
    }
    if inventory.block_units >= 3 && inventory.block_payoff_units > 0 {
        evidence.push(StrategicPackageEvidence::BlockEngine);
    }
    if inventory.aoe_units > 0 {
        evidence.push(StrategicPackageEvidence::AoEPackage);
    }
    if inventory.draw_units >= 2 || (counts.has_dark_embrace && inventory.exhaust_stream_units > 0)
    {
        evidence.push(StrategicPackageEvidence::DrawEngine);
    }
    if counts.has_energy_relic || inventory.energy_units > 0 || counts.has_corruption {
        evidence.push(StrategicPackageEvidence::EnergyEngine);
    }
    evidence
}

fn risks(
    inventory: &DeckRoleInventory,
    counts: &StrategicCounts,
    deck_access: StrategicDeficitLevel,
    exhaust_corruption: &ExhaustCorruptionAssessment,
) -> Vec<StrategicRisk> {
    let mut risks = Vec::new();
    if counts.low_impact_attacks >= 4 {
        risks.push(StrategicRisk::TooManyLowImpactAttacks);
    }
    if counts.conditional_payoffs >= 3 {
        risks.push(StrategicRisk::TooManyConditionalPayoffs);
    }
    if unsupported_payoffs(inventory, counts) {
        risks.push(StrategicRisk::NoEnablerForExploiters);
    }
    if counts.deck_size >= 26
        && matches!(
            deck_access,
            StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
        )
    {
        risks.push(StrategicRisk::DeckTooThickForAccess);
    }
    if counts.writhe_count > 0 {
        risks.push(StrategicRisk::OpeningHandPollution);
    }
    if counts.severe_curse_count > 0 {
        risks.push(StrategicRisk::SevereCurseBurden);
    }
    if counts.power_count >= 4 {
        risks.push(StrategicRisk::ReliesOnPowers);
    }
    if exhaust_corruption
        .risks
        .contains(&ExhaustCorruptionRisk::CorruptionWithoutExhaustPayoff)
    {
        risks.push(StrategicRisk::CorruptionWithoutExhaustPayoff);
    }
    if inventory.frontload_units >= 5
        && counts.low_impact_attacks >= 4
        && matches!(
            deck_access,
            StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
        )
    {
        risks.push(StrategicRisk::ReliesOnLowImpactCardSpam);
    }
    risks
}

fn unsupported_payoffs(inventory: &DeckRoleInventory, counts: &StrategicCounts) -> bool {
    (inventory.block_payoff_units > 0 && inventory.block_units <= 1)
        || (inventory.strength_payoff_units > 0 && counts.strength_sources == 0)
        || (counts.conditional_payoffs > 1
            && counts.strength_sources == 0
            && inventory.block_units <= 1
            && counts.status_clutter_sources == 0)
}

fn block_or_mitigation_units(inventory: &DeckRoleInventory, counts: &StrategicCounts) -> u8 {
    let raw_nonstarter = inventory
        .block_units
        .saturating_add(inventory.cycle_block_units)
        .saturating_add(inventory.mitigation_units);
    let low_quality_block = counts.defend_count.saturating_add(counts.iron_wave_count);
    let strong = raw_nonstarter.saturating_sub(low_quality_block);
    let low_quality_cap = counts
        .defend_count
        .saturating_add(counts.iron_wave_count)
        .min(3);
    strong.saturating_add(low_quality_cap)
}

fn block_or_mitigation_level(units: u8, act: u8) -> StrategicDeficitLevel {
    let surplus_at = if act >= 3 { 16 } else { 10 };
    unit_level(units, 2, surplus_at)
}

fn frontload_level(units: u8, act: u8) -> StrategicDeficitLevel {
    if act <= 2 {
        unit_level(units, 2, 8)
    } else {
        unit_level(units, 3, 8)
    }
}

fn boss_scaling_level(
    packages: &[StrategicPackageEvidence],
    counts: &StrategicCounts,
) -> StrategicDeficitLevel {
    let strong = packages
        .iter()
        .filter(|package| {
            matches!(
                package,
                StrategicPackageEvidence::ExhaustEngine
                    | StrategicPackageEvidence::StrengthScaling
                    | StrategicPackageEvidence::BlockEngine
            )
        })
        .count();
    if strong >= 2 {
        StrategicDeficitLevel::Adequate
    } else if strong == 1 || counts.has_ritual_dagger {
        StrategicDeficitLevel::Thin
    } else {
        StrategicDeficitLevel::Missing
    }
}

fn access_level(draw_units: u8, deck_size: usize) -> StrategicDeficitLevel {
    match draw_units {
        0 => StrategicDeficitLevel::Missing,
        1 if deck_size >= 18 => StrategicDeficitLevel::Thin,
        1 => StrategicDeficitLevel::Adequate,
        2 if deck_size >= 28 => StrategicDeficitLevel::Thin,
        2..=4 => StrategicDeficitLevel::Adequate,
        _ if deck_size >= 30 => StrategicDeficitLevel::Adequate,
        _ => StrategicDeficitLevel::Surplus,
    }
}

fn energy_level(inventory: &DeckRoleInventory, counts: &StrategicCounts) -> StrategicDeficitLevel {
    if counts.has_energy_relic || counts.has_corruption || inventory.energy_units >= 2 {
        StrategicDeficitLevel::Adequate
    } else if inventory.energy_units == 1 || counts.act <= 2 {
        StrategicDeficitLevel::Thin
    } else {
        StrategicDeficitLevel::Missing
    }
}

fn burden_level(counts: &StrategicCounts, access: StrategicDeficitLevel) -> StrategicBurdenLevel {
    if counts.curse_count >= 2
        || counts.normality_count > 0
        || counts.severe_curse_count >= 2
        || (counts.writhe_count > 0 && counts.deck_size >= 18)
        || counts.deck_size >= 34
        || (counts.deck_size >= 30
            && matches!(
                access,
                StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
            ))
    {
        StrategicBurdenLevel::Heavy
    } else if counts.curse_count > 0
        || counts.severe_curse_count > 0
        || counts.starter_basic_count >= 6
        || counts.deck_size >= 25
        || (counts.deck_size >= 22
            && matches!(
                access,
                StrategicDeficitLevel::Missing | StrategicDeficitLevel::Thin
            ))
    {
        StrategicBurdenLevel::Watch
    } else {
        StrategicBurdenLevel::Clean
    }
}

fn unit_level(units: u8, thin_under: u8, surplus_at: u8) -> StrategicDeficitLevel {
    if units == 0 {
        StrategicDeficitLevel::Missing
    } else if units <= thin_under {
        StrategicDeficitLevel::Thin
    } else if units < surplus_at {
        StrategicDeficitLevel::Adequate
    } else {
        StrategicDeficitLevel::Surplus
    }
}

fn is_low_impact_attack(card: CardId) -> bool {
    matches!(
        card,
        CardId::Strike
            | CardId::SwiftStrike
            | CardId::TwinStrike
            | CardId::WildStrike
            | CardId::RecklessCharge
            | CardId::SwordBoomerang
            | CardId::Clash
    )
}

fn is_conditional_payoff(card: CardId) -> bool {
    matches!(
        card,
        CardId::BodySlam
            | CardId::Entrench
            | CardId::Barricade
            | CardId::Juggernaut
            | CardId::HeavyBlade
            | CardId::LimitBreak
            | CardId::Pummel
            | CardId::FireBreathing
            | CardId::Evolve
    )
}

fn is_severe_curse(card: CardId) -> bool {
    matches!(
        card,
        CardId::Writhe | CardId::Normality | CardId::Regret | CardId::Pain | CardId::Parasite
    )
}
