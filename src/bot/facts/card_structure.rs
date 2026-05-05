use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardStructure(u32);

impl CardStructure {
    const STRENGTH_ENABLER: u32 = 1 << 0;
    const STRENGTH_PAYOFF: u32 = 1 << 1;
    const MULTI_ATTACK_PAYOFF: u32 = 1 << 2;
    const SETUP_PIECE: u32 = 1 << 3;
    const SCALING_PIECE: u32 = 1 << 4;
    const ENGINE_PIECE: u32 = 1 << 5;
    const EXHAUST_ENGINE: u32 = 1 << 6;
    const EXHAUST_OUTLET: u32 = 1 << 7;
    const DRAW_CORE: u32 = 1 << 8;
    const BLOCK_CORE: u32 = 1 << 9;
    const RESOURCE_CONVERSION: u32 = 1 << 10;
    const STATUS_ENGINE: u32 = 1 << 11;
    const VULN_PAYOFF: u32 = 1 << 14;
    const BLOCK_PAYOFF: u32 = 1 << 16;

    const fn new(bits: u32) -> Self {
        Self(bits)
    }

    const fn contains(self, flag: u32) -> bool {
        self.0 & flag != 0
    }

    pub(crate) const fn is_strength_enabler(self) -> bool {
        self.contains(Self::STRENGTH_ENABLER)
    }

    pub(crate) const fn is_strength_payoff(self) -> bool {
        self.contains(Self::STRENGTH_PAYOFF)
    }

    pub(crate) const fn is_multi_attack_payoff(self) -> bool {
        self.contains(Self::MULTI_ATTACK_PAYOFF)
    }

    pub(crate) const fn is_setup_piece(self) -> bool {
        self.contains(Self::SETUP_PIECE)
    }

    pub(crate) const fn is_scaling_piece(self) -> bool {
        self.contains(Self::SCALING_PIECE)
    }

    pub(crate) const fn is_engine_piece(self) -> bool {
        self.contains(Self::ENGINE_PIECE)
    }

    pub(crate) const fn is_exhaust_engine(self) -> bool {
        self.contains(Self::EXHAUST_ENGINE)
    }

    pub(crate) const fn is_exhaust_outlet(self) -> bool {
        self.contains(Self::EXHAUST_OUTLET)
    }

    pub(crate) const fn is_draw_core(self) -> bool {
        self.contains(Self::DRAW_CORE)
    }

    pub(crate) const fn is_block_core(self) -> bool {
        self.contains(Self::BLOCK_CORE)
    }

    pub(crate) const fn is_resource_conversion(self) -> bool {
        self.contains(Self::RESOURCE_CONVERSION)
    }

    pub(crate) const fn is_status_engine(self) -> bool {
        self.contains(Self::STATUS_ENGINE)
    }

    pub(crate) const fn is_vuln_payoff(self) -> bool {
        self.contains(Self::VULN_PAYOFF)
    }

    pub(crate) const fn is_block_payoff(self) -> bool {
        self.contains(Self::BLOCK_PAYOFF)
    }
}

pub(crate) const fn structure(card_id: CardId) -> CardStructure {
    use CardId::*;
    use CardStructure as S;

    let bits = match card_id {
        Corruption | FeelNoPain => S::SETUP_PIECE | S::ENGINE_PIECE | S::EXHAUST_ENGINE,
        DarkEmbrace => S::SETUP_PIECE | S::ENGINE_PIECE | S::EXHAUST_ENGINE | S::DRAW_CORE,
        Evolve => S::SETUP_PIECE | S::ENGINE_PIECE | S::STATUS_ENGINE | S::DRAW_CORE,
        Barricade => S::SETUP_PIECE | S::BLOCK_CORE,
        DemonForm => S::STRENGTH_ENABLER | S::SETUP_PIECE | S::SCALING_PIECE,
        Inflame => S::STRENGTH_ENABLER | S::SETUP_PIECE,
        Metallicize => S::SETUP_PIECE | S::SCALING_PIECE,
        Berserk => S::SETUP_PIECE,
        Rupture => S::STRENGTH_ENABLER | S::SETUP_PIECE | S::SCALING_PIECE,
        Combust => S::SETUP_PIECE,
        FireBreathing | Panache | Mayhem | Magnetism => S::SETUP_PIECE,
        SpotWeakness | Flex => S::STRENGTH_ENABLER,
        HeavyBlade => S::STRENGTH_PAYOFF,
        SwordBoomerang | TwinStrike | Pummel => S::STRENGTH_PAYOFF | S::MULTI_ATTACK_PAYOFF,
        Whirlwind => S::STRENGTH_PAYOFF | S::MULTI_ATTACK_PAYOFF,
        LimitBreak => S::STRENGTH_PAYOFF | S::STRENGTH_ENABLER,
        Reaper => S::STRENGTH_PAYOFF | S::MULTI_ATTACK_PAYOFF,
        SecondWind | FiendFire | SeverSoul | TrueGrit => S::EXHAUST_OUTLET,
        BurningPact => S::EXHAUST_OUTLET | S::DRAW_CORE,
        Offering | Bloodletting | SeeingRed => S::RESOURCE_CONVERSION,
        BattleTrance | PommelStrike | DeepBreath | Warcry | MasterOfStrategy | Finesse
        | FlashOfSteel | GoodInstincts => S::DRAW_CORE,
        Dropkick => S::DRAW_CORE | S::VULN_PAYOFF,
        ShrugItOff => S::DRAW_CORE | S::BLOCK_CORE,
        Defend | DefendG | Apparition | GhostlyArmor | FlameBarrier | Impervious => S::BLOCK_CORE,
        PowerThrough => S::BLOCK_CORE | S::STATUS_ENGINE,
        Entrench | BodySlam => S::BLOCK_PAYOFF,
        WildStrike | RecklessCharge => S::STATUS_ENGINE,
        _ => 0,
    };

    CardStructure::new(bits)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structure_tags_are_descriptive_not_value_labels() {
        let corruption = structure(CardId::Corruption);
        assert!(corruption.is_exhaust_engine());
        assert!(corruption.is_engine_piece());
        assert!(!corruption.is_strength_payoff());

        let pummel = structure(CardId::Pummel);
        assert!(pummel.is_strength_payoff());
        assert!(pummel.is_multi_attack_payoff());
    }
}
