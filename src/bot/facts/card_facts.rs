use crate::content::cards::{self, CardId, CardRarity, CardType};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CostBand {
    XCost,
    ZeroOne,
    Two,
    ThreePlus,
    Unplayable,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct CardFacts {
    pub cost_band: CostBand,
    pub card_type: CardType,
    pub rarity: CardRarity,
    pub exhausts_self: bool,
    pub ethereal: bool,
    pub innate: bool,
    pub draws_cards: bool,
    pub gains_energy: bool,
    pub applies_weak: bool,
    pub applies_vuln: bool,
    pub applies_frail: bool,
    pub self_damage: bool,
    pub multi_hit: bool,
    pub aoe: bool,
    pub produces_status: bool,
    pub exhausts_other_cards: bool,
    pub target_sensitive: bool,
    pub combat_heal: bool,
    pub conditional_free: bool,
    pub self_replicating: bool,
    pub random_generation: bool,
    pub cost_manipulation_sensitive: bool,
}

pub(crate) fn facts(card_id: CardId) -> CardFacts {
    use CardId::*;

    let def = cards::get_card_definition(card_id);
    let cost_band = match def.cost {
        -1 => CostBand::XCost,
        i8::MIN..=-2 => CostBand::Unplayable,
        0 | 1 => CostBand::ZeroOne,
        2 => CostBand::Two,
        _ => CostBand::ThreePlus,
    };

    CardFacts {
        cost_band,
        card_type: def.card_type,
        rarity: def.rarity,
        exhausts_self: def.exhaust,
        ethereal: def.ethereal,
        innate: def.innate,
        draws_cards: matches!(
            card_id,
            BattleTrance
                | BurningPact
                | DarkEmbrace
                | DeepBreath
                | Dropkick
                | Evolve
                | Finesse
                | FlashOfSteel
                | GoodInstincts
                | MasterOfStrategy
                | Offering
                | PommelStrike
                | ShrugItOff
                | Warcry
        ),
        gains_energy: matches!(
            card_id,
            Bloodletting | Berserk | Offering | SeeingRed | Sentinel
        ),
        applies_weak: matches!(card_id, Clothesline | Intimidate | Shockwave | Uppercut),
        applies_vuln: matches!(card_id, Bash | Shockwave | ThunderClap | Trip | Uppercut),
        applies_frail: matches!(card_id, Shockwave),
        self_damage: matches!(
            card_id,
            Bloodletting | Combust | Hemokinesis | JAX | Offering | Rupture
        ),
        multi_hit: matches!(
            card_id,
            Pummel | SwordBoomerang | TwinStrike | Whirlwind | Reaper
        ),
        aoe: matches!(
            card_id,
            Cleave
                | Combust
                | FireBreathing
                | Immolate
                | Reaper
                | Shockwave
                | ThunderClap
                | Whirlwind
        ),
        produces_status: matches!(card_id, PowerThrough | RecklessCharge | WildStrike),
        exhausts_other_cards: matches!(
            card_id,
            BurningPact | FiendFire | SecondWind | SeverSoul | TrueGrit
        ),
        target_sensitive: matches!(
            card_id,
            Disarm | Shockwave | Uppercut | Headbutt | TrueGrit | SpotWeakness | Trip
        ),
        combat_heal: matches!(card_id, Reaper | Bite | Feed | BandageUp),
        conditional_free: matches!(card_id, BloodForBlood | Dropkick | SeeingRed),
        self_replicating: matches!(card_id, Anger),
        random_generation: matches!(card_id, Discovery | InfernalBlade | Metamorphosis | Mayhem),
        cost_manipulation_sensitive: matches!(
            card_id,
            BloodForBlood | Dropkick | SeverSoul | SeeingRed
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn card_facts_capture_mechanism_level_signals() {
        let shrug = facts(CardId::ShrugItOff);
        assert!(shrug.draws_cards);
        assert!(!shrug.gains_energy);

        let offering = facts(CardId::Offering);
        assert!(offering.draws_cards);
        assert!(offering.gains_energy);
        assert!(offering.self_damage);

        let shockwave = facts(CardId::Shockwave);
        assert!(shockwave.applies_weak);
        assert!(shockwave.applies_vuln);
        assert!(shockwave.target_sensitive);

        let anger = facts(CardId::Anger);
        assert!(anger.self_replicating);

        let discovery = facts(CardId::Discovery);
        assert!(discovery.random_generation);
    }
}
