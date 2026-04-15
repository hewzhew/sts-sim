use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardTaxonomy(u32);

impl CardTaxonomy {
    const STRENGTH_ENABLER: u32 = 1 << 0;
    const STRENGTH_PAYOFF: u32 = 1 << 1;
    const MULTI_ATTACK_PAYOFF: u32 = 1 << 2;
    const SETUP_POWER: u32 = 1 << 3;
    const SCALING_POWER: u32 = 1 << 4;
    const ENGINE_PIECE: u32 = 1 << 5;
    const EXHAUST_ENGINE: u32 = 1 << 6;
    const EXHAUST_OUTLET: u32 = 1 << 7;
    const DRAW_CORE: u32 = 1 << 8;
    const BLOCK_CORE: u32 = 1 << 9;
    const RESOURCE_CONVERSION: u32 = 1 << 10;
    const ARMAMENTS_FRONTLOAD_PRIORITY: u32 = 1 << 12;
    const ARMAMENTS_UPGRADE_PRIORITY: u32 = 1 << 13;
    const PANACEA_SELF_COMBO: u32 = 1 << 14;
    const ATTACK_FOLLOWUP_PRIORITY: u32 = 1 << 15;
    const STATUS_ENGINE: u32 = 1 << 16;
    const EXHAUST_RECOVERY: u32 = 1 << 17;
    const AOE: u32 = 1 << 18;
    const SELF_DAMAGE_SOURCE: u32 = 1 << 19;
    const STATUS_PRODUCER: u32 = 1 << 20;
    const DISCARD_CYCLE: u32 = 1 << 21;
    const VULN_PAYOFF: u32 = 1 << 22;
    const CONDITIONAL_FREE: u32 = 1 << 23;
    const COMBAT_HEAL: u32 = 1 << 24;
    const VULN_ENABLER: u32 = 1 << 25;
    const SELF_DAMAGE_PAYOFF: u32 = 1 << 26;
    const BLOCK_PAYOFF: u32 = 1 << 27;
    const WEAK_ENABLER: u32 = 1 << 28;
    const MULTI_HIT: u32 = 1 << 29;
    const DISCARD_RETRIEVAL: u32 = 1 << 30;

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

    pub(crate) const fn is_setup_power(self) -> bool {
        self.contains(Self::SETUP_POWER)
    }

    pub(crate) const fn is_scaling_power(self) -> bool {
        self.contains(Self::SCALING_POWER)
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

    pub(crate) const fn is_armaments_frontload_priority(self) -> bool {
        self.contains(Self::ARMAMENTS_FRONTLOAD_PRIORITY)
    }

    pub(crate) const fn is_armaments_upgrade_priority(self) -> bool {
        self.contains(Self::ARMAMENTS_UPGRADE_PRIORITY)
    }

    pub(crate) const fn is_panacea_self_combo(self) -> bool {
        self.contains(Self::PANACEA_SELF_COMBO)
    }

    pub(crate) const fn is_attack_followup_priority(self) -> bool {
        self.contains(Self::ATTACK_FOLLOWUP_PRIORITY)
    }

    pub(crate) const fn is_status_engine(self) -> bool {
        self.contains(Self::STATUS_ENGINE)
    }

    pub(crate) const fn is_exhaust_recovery(self) -> bool {
        self.contains(Self::EXHAUST_RECOVERY)
    }

    pub(crate) const fn is_aoe(self) -> bool {
        self.contains(Self::AOE)
    }

    pub(crate) const fn is_self_damage_source(self) -> bool {
        self.contains(Self::SELF_DAMAGE_SOURCE)
    }

    pub(crate) const fn is_status_producer(self) -> bool {
        self.contains(Self::STATUS_PRODUCER)
    }

    pub(crate) const fn is_discard_cycle(self) -> bool {
        self.contains(Self::DISCARD_CYCLE)
    }

    pub(crate) const fn is_vuln_payoff(self) -> bool {
        self.contains(Self::VULN_PAYOFF)
    }

    pub(crate) const fn is_conditional_free(self) -> bool {
        self.contains(Self::CONDITIONAL_FREE)
    }

    pub(crate) const fn is_combat_heal(self) -> bool {
        self.contains(Self::COMBAT_HEAL)
    }

    pub(crate) const fn is_vuln_enabler(self) -> bool {
        self.contains(Self::VULN_ENABLER)
    }

    pub(crate) const fn is_self_damage_payoff(self) -> bool {
        self.contains(Self::SELF_DAMAGE_PAYOFF)
    }

    pub(crate) const fn is_block_payoff(self) -> bool {
        self.contains(Self::BLOCK_PAYOFF)
    }

    pub(crate) const fn is_weak_enabler(self) -> bool {
        self.contains(Self::WEAK_ENABLER)
    }

    pub(crate) const fn is_multi_hit(self) -> bool {
        self.contains(Self::MULTI_HIT)
    }

    pub(crate) const fn is_discard_retrieval(self) -> bool {
        self.contains(Self::DISCARD_RETRIEVAL)
    }
}

pub(crate) const fn taxonomy(card_id: CardId) -> CardTaxonomy {
    use CardId::*;
    use CardTaxonomy as T;

    let bits = match card_id {
        Corruption | FeelNoPain => {
            T::SETUP_POWER
                | T::ENGINE_PIECE
                | T::EXHAUST_ENGINE
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
        }
        DarkEmbrace => {
            T::SETUP_POWER
                | T::ENGINE_PIECE
                | T::EXHAUST_ENGINE
                | T::DRAW_CORE
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
        }
        Evolve => {
            T::SETUP_POWER
                | T::ENGINE_PIECE
                | T::STATUS_ENGINE
                | T::DRAW_CORE
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
        }
        Barricade => T::SETUP_POWER | T::BLOCK_CORE,
        DemonForm => {
            T::STRENGTH_ENABLER
                | T::SETUP_POWER
                | T::SCALING_POWER
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
        }
        Inflame => {
            T::STRENGTH_ENABLER
                | T::SETUP_POWER
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
        }
        Metallicize => {
            T::SETUP_POWER
                | T::SCALING_POWER
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
        }
        Berserk => {
            T::SETUP_POWER
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
                | T::PANACEA_SELF_COMBO
        }
        Rupture => T::STRENGTH_ENABLER | T::SETUP_POWER | T::SCALING_POWER | T::SELF_DAMAGE_PAYOFF,
        Combust => T::SETUP_POWER | T::AOE | T::SELF_DAMAGE_SOURCE,
        FireBreathing => T::SETUP_POWER | T::AOE,
        Panache => T::SETUP_POWER | T::SCALING_POWER,
        Mayhem | Magnetism => T::SETUP_POWER,
        SpotWeakness => T::STRENGTH_ENABLER,
        Flex => T::STRENGTH_ENABLER | T::PANACEA_SELF_COMBO,
        HeavyBlade => {
            T::STRENGTH_PAYOFF
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
                | T::ATTACK_FOLLOWUP_PRIORITY
        }
        SwordBoomerang => {
            T::STRENGTH_PAYOFF | T::MULTI_ATTACK_PAYOFF | T::ATTACK_FOLLOWUP_PRIORITY | T::MULTI_HIT
        }
        TwinStrike => T::STRENGTH_PAYOFF | T::MULTI_ATTACK_PAYOFF | T::MULTI_HIT,
        Pummel => {
            T::STRENGTH_PAYOFF | T::MULTI_ATTACK_PAYOFF | T::ATTACK_FOLLOWUP_PRIORITY | T::MULTI_HIT
        }
        Whirlwind => {
            T::STRENGTH_PAYOFF
                | T::MULTI_ATTACK_PAYOFF
                | T::AOE
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
                | T::ATTACK_FOLLOWUP_PRIORITY
        }
        LimitBreak => {
            T::STRENGTH_PAYOFF
                | T::STRENGTH_ENABLER
                | T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
        }
        Reaper => T::STRENGTH_PAYOFF | T::MULTI_ATTACK_PAYOFF | T::AOE | T::COMBAT_HEAL,
        SecondWind => T::EXHAUST_OUTLET | T::BLOCK_CORE | T::ARMAMENTS_UPGRADE_PRIORITY,
        BurningPact => T::EXHAUST_OUTLET | T::DRAW_CORE | T::ARMAMENTS_UPGRADE_PRIORITY,
        FiendFire | SeverSoul => T::EXHAUST_OUTLET,
        TrueGrit => T::EXHAUST_OUTLET | T::BLOCK_CORE | T::ARMAMENTS_UPGRADE_PRIORITY,
        Exhume => T::EXHAUST_RECOVERY | T::ARMAMENTS_UPGRADE_PRIORITY,
        Offering => T::DRAW_CORE | T::RESOURCE_CONVERSION | T::SELF_DAMAGE_SOURCE,
        BattleTrance => {
            T::DRAW_CORE | T::ARMAMENTS_FRONTLOAD_PRIORITY | T::ARMAMENTS_UPGRADE_PRIORITY
        }
        ShrugItOff => T::DRAW_CORE | T::BLOCK_CORE | T::ARMAMENTS_UPGRADE_PRIORITY,
        PommelStrike => T::DRAW_CORE | T::ARMAMENTS_UPGRADE_PRIORITY,
        DeepBreath => T::DRAW_CORE | T::DISCARD_CYCLE,
        Headbutt => T::DISCARD_RETRIEVAL,
        Warcry | MasterOfStrategy | Finesse | FlashOfSteel | GoodInstincts => T::DRAW_CORE,
        Defend | DefendG | Apparition => T::BLOCK_CORE,
        Entrench => T::BLOCK_PAYOFF,
        GhostlyArmor => T::BLOCK_CORE | T::ARMAMENTS_UPGRADE_PRIORITY,
        FlameBarrier => T::BLOCK_CORE | T::ARMAMENTS_UPGRADE_PRIORITY,
        Impervious => T::BLOCK_CORE,
        PowerThrough => T::BLOCK_CORE | T::STATUS_PRODUCER,
        BodySlam => T::BLOCK_PAYOFF | T::ARMAMENTS_UPGRADE_PRIORITY,
        Bloodletting => T::RESOURCE_CONVERSION | T::SELF_DAMAGE_SOURCE,
        SeeingRed => T::RESOURCE_CONVERSION | T::ARMAMENTS_UPGRADE_PRIORITY,
        Bash => T::ARMAMENTS_UPGRADE_PRIORITY | T::ATTACK_FOLLOWUP_PRIORITY | T::VULN_ENABLER,
        ThunderClap => T::VULN_ENABLER | T::AOE,
        Shockwave => {
            T::ARMAMENTS_FRONTLOAD_PRIORITY
                | T::ARMAMENTS_UPGRADE_PRIORITY
                | T::VULN_ENABLER
                | T::WEAK_ENABLER
        }
        Disarm | Apotheosis => 0,
        Uppercut => {
            T::ARMAMENTS_UPGRADE_PRIORITY
                | T::ATTACK_FOLLOWUP_PRIORITY
                | T::VULN_ENABLER
                | T::WEAK_ENABLER
        }
        Clothesline | Intimidate => T::WEAK_ENABLER,
        Havoc => T::ARMAMENTS_UPGRADE_PRIORITY,
        BloodForBlood => {
            T::ARMAMENTS_UPGRADE_PRIORITY | T::ATTACK_FOLLOWUP_PRIORITY | T::CONDITIONAL_FREE
        }
        Hemokinesis => T::ATTACK_FOLLOWUP_PRIORITY | T::SELF_DAMAGE_SOURCE,
        Rampage => T::ATTACK_FOLLOWUP_PRIORITY,
        Dropkick => {
            T::ATTACK_FOLLOWUP_PRIORITY | T::DRAW_CORE | T::VULN_PAYOFF | T::CONDITIONAL_FREE
        }
        Trip => T::VULN_ENABLER,
        WildStrike | RecklessCharge => T::STATUS_PRODUCER,
        _ => 0,
    };

    CardTaxonomy::new(bits)
}

pub(crate) const fn is_strength_enabler(card_id: CardId) -> bool {
    taxonomy(card_id).is_strength_enabler()
}

pub(crate) const fn is_strength_payoff(card_id: CardId) -> bool {
    taxonomy(card_id).is_strength_payoff()
}

pub(crate) const fn is_multi_attack_payoff(card_id: CardId) -> bool {
    taxonomy(card_id).is_multi_attack_payoff()
}
