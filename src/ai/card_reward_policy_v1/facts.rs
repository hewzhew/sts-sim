use crate::content::cards::{get_card_definition, CardId, CardTarget};
use crate::state::rewards::RewardCard;

use super::types::{
    CardRewardDamageFactsV1, CardRewardFactsV1, CardRewardPickDependencyV1,
    CardRewardScalingSignalV1,
};

pub(crate) fn card_facts(card: &RewardCard) -> CardRewardFactsV1 {
    let def = get_card_definition(card.id);
    let upgrades = i32::from(card.upgrades);
    let damage_per_hit = (def.base_damage + def.upgrade_damage * upgrades).max(0);
    let hit_count = hit_count(card.id, upgrades, damage_per_hit);
    let magic = (def.base_magic + def.upgrade_magic * upgrades).max(0);

    CardRewardFactsV1 {
        card: card.id,
        name: def.name.to_string(),
        card_type: def.card_type,
        rarity: def.rarity,
        cost: def.cost,
        damage: CardRewardDamageFactsV1 {
            damage_per_hit,
            hit_count,
            total_damage: damage_per_hit.saturating_mul(hit_count),
        },
        block: (def.base_block + def.upgrade_block * upgrades).max(0),
        draw_cards: draw_cards(card.id, magic),
        energy_gain: energy_gain(card.id, magic),
        vulnerable: vulnerable(card.id, magic),
        weak: weak(card.id, magic),
        strength_gain: strength_gain(card.id, magic),
        enemy_strength_down: enemy_strength_down(card.id, magic),
        exhausts: def.exhaust,
        exhausts_other_cards: exhausts_other_cards(card.id),
        adds_status_cards: adds_status_cards(card.id),
        upgrades_cards: upgrades_cards(card.id),
        is_random_output: is_random_output(card.id),
        has_conditional_playability: has_conditional_playability(card.id),
        is_aoe: def.target == CardTarget::AllEnemy || def.is_multi_damage,
        pick_dependencies: pick_dependencies(card.id),
        unsupported_mechanics: unsupported_mechanics(card.id),
    }
}

pub(crate) fn scaling_signals(facts: &CardRewardFactsV1) -> Vec<CardRewardScalingSignalV1> {
    let mut signals = Vec::new();
    if facts.strength_gain > 0 {
        signals.push(CardRewardScalingSignalV1::StrengthGain);
    }
    if facts
        .pick_dependencies
        .contains(&CardRewardPickDependencyV1::StrengthScaling)
    {
        signals.push(CardRewardScalingSignalV1::StrengthPayoff);
    }
    if facts.vulnerable > 0 {
        signals.push(CardRewardScalingSignalV1::Vulnerable);
    }
    if facts.weak > 0 {
        signals.push(CardRewardScalingSignalV1::Weak);
    }
    if facts.enemy_strength_down > 0 {
        signals.push(CardRewardScalingSignalV1::EnemyStrengthDown);
    }
    if facts
        .pick_dependencies
        .contains(&CardRewardPickDependencyV1::ExhaustPackage)
    {
        signals.push(CardRewardScalingSignalV1::ExhaustPayoff);
    }
    if facts
        .pick_dependencies
        .contains(&CardRewardPickDependencyV1::StatusPackage)
    {
        signals.push(CardRewardScalingSignalV1::StatusPayoff);
    }
    if facts
        .pick_dependencies
        .contains(&CardRewardPickDependencyV1::BlockDensity)
    {
        signals.push(CardRewardScalingSignalV1::BlockEngine);
    }
    signals
}

fn hit_count(card_id: CardId, upgrades: i32, damage_per_hit: i32) -> i32 {
    if damage_per_hit <= 0 {
        return 0;
    }
    let def = get_card_definition(card_id);
    match card_id {
        CardId::TwinStrike => 2,
        CardId::SwordBoomerang => (def.base_magic + def.upgrade_magic * upgrades).max(0),
        CardId::RiddleWithHoles => 5,
        _ => 1,
    }
}

fn draw_cards(card_id: CardId, magic: i32) -> i32 {
    match card_id {
        CardId::PommelStrike
        | CardId::ShrugItOff
        | CardId::Finesse
        | CardId::FlashOfSteel
        | CardId::DeepBreath
        | CardId::QuickSlash
        | CardId::SweepingBeam => 1,
        CardId::BurningPact
        | CardId::BattleTrance
        | CardId::Offering
        | CardId::Warcry
        | CardId::MasterOfStrategy
        | CardId::Acrobatics
        | CardId::Backflip
        | CardId::Skim
        | CardId::WheelKick => magic,
        _ => 0,
    }
}

fn energy_gain(card_id: CardId, magic: i32) -> i32 {
    match card_id {
        CardId::Offering => 2,
        CardId::SeeingRed | CardId::Bloodletting | CardId::Turbo => magic,
        _ => 0,
    }
}

fn vulnerable(card_id: CardId, magic: i32) -> i32 {
    match card_id {
        CardId::Bash | CardId::Uppercut | CardId::Shockwave | CardId::Terror => magic,
        CardId::ThunderClap | CardId::Trip | CardId::BeamCell => 1,
        _ => 0,
    }
}

fn weak(card_id: CardId, magic: i32) -> i32 {
    match card_id {
        CardId::Clothesline | CardId::Uppercut | CardId::Shockwave => magic,
        CardId::Blind | CardId::SuckerPunch | CardId::GoForTheEyes => magic.max(1),
        _ => 0,
    }
}

fn strength_gain(card_id: CardId, magic: i32) -> i32 {
    match card_id {
        CardId::Inflame | CardId::SpotWeakness | CardId::Flex => magic,
        CardId::DemonForm => magic,
        _ => 0,
    }
}

fn enemy_strength_down(card_id: CardId, magic: i32) -> i32 {
    match card_id {
        CardId::Disarm | CardId::Shockwave | CardId::DarkShackles | CardId::PiercingWail => magic,
        _ => 0,
    }
}

fn exhausts_other_cards(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::BurningPact
            | CardId::TrueGrit
            | CardId::SecondWind
            | CardId::SeverSoul
            | CardId::FiendFire
            | CardId::Recycle
            | CardId::Exhume
    )
}

fn adds_status_cards(card_id: CardId) -> i32 {
    match card_id {
        CardId::WildStrike => 1,
        CardId::RecklessCharge => 1,
        CardId::PowerThrough => 2,
        CardId::Immolate => 1,
        _ => 0,
    }
}

fn upgrades_cards(card_id: CardId) -> bool {
    matches!(card_id, CardId::Armaments | CardId::Apotheosis)
}

fn is_random_output(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::SwordBoomerang
            | CardId::Discovery
            | CardId::InfernalBlade
            | CardId::JackOfAllTrades
            | CardId::WhiteNoise
            | CardId::Chrysalis
            | CardId::Metamorphosis
            | CardId::Transmutation
            | CardId::Magnetism
            | CardId::Mayhem
    )
}

fn has_conditional_playability(card_id: CardId) -> bool {
    matches!(
        card_id,
        CardId::Clash
            | CardId::BodySlam
            | CardId::PerfectedStrike
            | CardId::GrandFinale
            | CardId::Expertise
    )
}

fn pick_dependencies(card_id: CardId) -> Vec<CardRewardPickDependencyV1> {
    let mut dependencies = Vec::new();
    match card_id {
        CardId::SearingBlow => dependencies.push(CardRewardPickDependencyV1::RouteUpgradeDensity),
        CardId::HeavyBlade | CardId::LimitBreak | CardId::Reaper => {
            dependencies.push(CardRewardPickDependencyV1::StrengthScaling)
        }
        CardId::BodySlam | CardId::Barricade | CardId::Entrench | CardId::Juggernaut => {
            dependencies.push(CardRewardPickDependencyV1::BlockDensity)
        }
        CardId::PerfectedStrike => dependencies.push(CardRewardPickDependencyV1::StrikeDensity),
        CardId::FeelNoPain | CardId::DarkEmbrace | CardId::Corruption => {
            dependencies.push(CardRewardPickDependencyV1::ExhaustPackage)
        }
        CardId::Evolve | CardId::FireBreathing => {
            dependencies.push(CardRewardPickDependencyV1::StatusPackage)
        }
        CardId::Rupture => dependencies.push(CardRewardPickDependencyV1::SelfDamagePackage),
        _ => {}
    }
    if is_random_output(card_id) {
        dependencies.push(CardRewardPickDependencyV1::RandomOutputPolicy);
    }
    if has_conditional_playability(card_id) {
        dependencies.push(CardRewardPickDependencyV1::ConditionalPlayabilityPolicy);
    }
    if !unsupported_mechanics(card_id).is_empty() {
        dependencies.push(CardRewardPickDependencyV1::UnsupportedMechanics);
    }
    dependencies
}

fn unsupported_mechanics(card_id: CardId) -> Vec<String> {
    match card_id {
        CardId::Warcry => vec!["hand top-deck selection".to_string()],
        CardId::Rampage => vec!["combat-history damage growth".to_string()],
        CardId::BloodForBlood => vec!["combat-damage cost mutation".to_string()],
        CardId::Dropkick => vec!["conditional draw and energy on vulnerable target".to_string()],
        CardId::Headbutt => vec!["discard-pile top-deck selection".to_string()],
        CardId::Havoc => vec!["top-deck random/hidden execution".to_string()],
        CardId::DualWield => vec!["hand copy selection".to_string()],
        CardId::Exhume => vec!["exhaust-pile selection".to_string()],
        CardId::FiendFire => vec!["hand-size dependent exhaust damage".to_string()],
        _ => Vec::new(),
    }
}
