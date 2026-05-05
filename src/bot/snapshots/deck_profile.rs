use crate::content::cards::CardId;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::run::RunState;

#[derive(Debug, Default, Clone, Copy)]
pub struct DeckProfile {
    pub attack_count: i32,
    pub skill_count: i32,
    pub power_count: i32,
    pub searing_blow_count: i32,
    pub searing_blow_upgrades: i32,
    pub strength_enablers: i32,
    pub strength_payoffs: i32,
    pub exhaust_engines: i32,
    pub exhaust_outlets: i32,
    pub exhaust_fodder: i32,
    pub block_core: i32,
    pub block_payoffs: i32,
    pub self_damage_sources: i32,
    pub x_cost_payoffs: i32,
    pub draw_sources: i32,
    pub power_scalers: i32,
    pub status_generators: i32,
    pub status_payoffs: i32,
}

pub fn deck_profile(run_state: &RunState) -> DeckProfile {
    deck_profile_from_cards(run_state.master_deck.iter())
}

pub fn combat_zone_profile(combat_state: &CombatState) -> DeckProfile {
    deck_profile_from_cards(
        combat_state
            .zones
            .hand
            .iter()
            .chain(combat_state.zones.draw_pile.iter())
            .chain(combat_state.zones.discard_pile.iter())
            .chain(combat_state.zones.exhaust_pile.iter())
            .chain(combat_state.zones.limbo.iter()),
    )
}

pub(crate) fn deck_profile_from_cards<'a, I>(cards: I) -> DeckProfile
where
    I: IntoIterator<Item = &'a CombatCard>,
{
    let mut profile = DeckProfile::default();

    for card in cards {
        match crate::content::cards::get_card_definition(card.id).card_type {
            crate::content::cards::CardType::Attack => profile.attack_count += 1,
            crate::content::cards::CardType::Skill => profile.skill_count += 1,
            crate::content::cards::CardType::Power => profile.power_count += 1,
            _ => {}
        }

        if matches!(
            card.id,
            CardId::Inflame
                | CardId::SpotWeakness
                | CardId::DemonForm
                | CardId::LimitBreak
                | CardId::Flex
                | CardId::Rupture
        ) {
            profile.strength_enablers += 1;
        }
        if matches!(
            card.id,
            CardId::HeavyBlade
                | CardId::SwordBoomerang
                | CardId::TwinStrike
                | CardId::Whirlwind
                | CardId::Pummel
                | CardId::Reaper
        ) {
            profile.strength_payoffs += 1;
        }
        if card.id == CardId::Whirlwind {
            profile.x_cost_payoffs += 1;
        }
        if card.id == CardId::SearingBlow {
            profile.searing_blow_count += 1;
            profile.searing_blow_upgrades += card.upgrades as i32;
        }
        if matches!(
            card.id,
            CardId::BattleTrance
                | CardId::PommelStrike
                | CardId::ShrugItOff
                | CardId::Offering
                | CardId::BurningPact
                | CardId::Finesse
                | CardId::FlashOfSteel
                | CardId::MasterOfStrategy
                | CardId::Brutality
        ) {
            profile.draw_sources += 1;
        }
        if matches!(
            card.id,
            CardId::Corruption | CardId::FeelNoPain | CardId::DarkEmbrace
        ) {
            profile.exhaust_engines += 2;
            profile.power_scalers += 1;
        }
        if matches!(
            card.id,
            CardId::SecondWind
                | CardId::FiendFire
                | CardId::SeverSoul
                | CardId::BurningPact
                | CardId::TrueGrit
                | CardId::Exhume
        ) {
            profile.exhaust_outlets += 1;
        }
        if matches!(card.id, CardId::WildStrike | CardId::RecklessCharge) {
            profile.exhaust_fodder += 1;
            profile.status_generators += 1;
        }
        if card.id == CardId::PowerThrough {
            profile.exhaust_fodder += 1;
            profile.block_core += 1;
            profile.status_generators += 1;
        }
        if matches!(
            card.id,
            CardId::ShrugItOff
                | CardId::FlameBarrier
                | CardId::Impervious
                | CardId::GhostlyArmor
                | CardId::Entrench
                | CardId::BodySlam
                | CardId::IronWave
        ) {
            profile.block_core += 1;
        }
        if matches!(card.id, CardId::Barricade | CardId::Juggernaut) {
            profile.block_core += 1;
            profile.block_payoffs += 1;
            profile.power_scalers += 1;
        }
        if matches!(
            card.id,
            CardId::Offering
                | CardId::Bloodletting
                | CardId::Hemokinesis
                | CardId::Combust
                | CardId::Brutality
                | CardId::Rupture
        ) {
            profile.self_damage_sources += 1;
        }
        if matches!(
            card.id,
            CardId::DemonForm
                | CardId::Inflame
                | CardId::Panache
                | CardId::Mayhem
                | CardId::Magnetism
                | CardId::Rupture
        ) {
            profile.power_scalers += 1;
        }
        if matches!(card.id, CardId::Evolve | CardId::FireBreathing) {
            profile.status_payoffs += 1;
            profile.power_scalers += 1;
        }
    }

    profile
}
