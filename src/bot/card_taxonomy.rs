use crate::bot::card_facts;
use crate::bot::card_structure::{self, CardStructure};
use crate::content::cards::CardId;

// Compatibility layer: structural signals now come from `card_structure` and mechanism facts
// from `card_facts`. A few combat-facing execution priors remain here temporarily while callers
// are migrated off the older taxonomy surface.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardTaxonomy {
    structure: CardStructure,
    armaments_frontload_priority: bool,
    armaments_upgrade_priority: bool,
    panacea_self_combo: bool,
    attack_followup_priority: bool,
    aoe: bool,
    self_damage_source: bool,
    status_producer: bool,
    conditional_free: bool,
    combat_heal: bool,
    vuln_enabler: bool,
    weak_enabler: bool,
    multi_hit: bool,
}

impl CardTaxonomy {
    pub(crate) const fn is_strength_enabler(self) -> bool {
        self.structure.is_strength_enabler()
    }

    pub(crate) const fn is_strength_payoff(self) -> bool {
        self.structure.is_strength_payoff()
    }

    pub(crate) const fn is_multi_attack_payoff(self) -> bool {
        self.structure.is_multi_attack_payoff()
    }

    pub(crate) const fn is_setup_power(self) -> bool {
        self.structure.is_setup_piece()
    }

    pub(crate) const fn is_scaling_power(self) -> bool {
        self.structure.is_scaling_piece()
    }

    pub(crate) const fn is_engine_piece(self) -> bool {
        self.structure.is_engine_piece()
    }

    pub(crate) const fn is_exhaust_engine(self) -> bool {
        self.structure.is_exhaust_engine()
    }

    pub(crate) const fn is_exhaust_outlet(self) -> bool {
        self.structure.is_exhaust_outlet()
    }

    pub(crate) const fn is_draw_core(self) -> bool {
        self.structure.is_draw_core()
    }

    pub(crate) const fn is_block_core(self) -> bool {
        self.structure.is_block_core()
    }

    pub(crate) const fn is_resource_conversion(self) -> bool {
        self.structure.is_resource_conversion()
    }

    pub(crate) const fn is_armaments_frontload_priority(self) -> bool {
        self.armaments_frontload_priority
    }

    pub(crate) const fn is_armaments_upgrade_priority(self) -> bool {
        self.armaments_upgrade_priority
    }

    pub(crate) const fn is_panacea_self_combo(self) -> bool {
        self.panacea_self_combo
    }

    pub(crate) const fn is_attack_followup_priority(self) -> bool {
        self.attack_followup_priority
    }

    pub(crate) const fn is_status_engine(self) -> bool {
        self.structure.is_status_engine()
    }

    pub(crate) const fn is_exhaust_recovery(self) -> bool {
        self.structure.is_exhaust_recovery()
    }

    pub(crate) const fn is_aoe(self) -> bool {
        self.aoe
    }

    pub(crate) const fn is_self_damage_source(self) -> bool {
        self.self_damage_source
    }

    pub(crate) const fn is_status_producer(self) -> bool {
        self.status_producer
    }

    pub(crate) const fn is_discard_cycle(self) -> bool {
        self.structure.is_discard_cycle()
    }

    #[allow(dead_code)]
    pub(crate) const fn is_vuln_payoff(self) -> bool {
        self.structure.is_vuln_payoff()
    }

    #[allow(dead_code)]
    pub(crate) const fn is_conditional_free(self) -> bool {
        self.conditional_free
    }

    #[allow(dead_code)]
    pub(crate) const fn is_combat_heal(self) -> bool {
        self.combat_heal
    }

    pub(crate) const fn is_vuln_enabler(self) -> bool {
        self.vuln_enabler
    }

    pub(crate) const fn is_self_damage_payoff(self) -> bool {
        self.structure.is_self_damage_payoff()
    }

    #[allow(dead_code)]
    pub(crate) const fn is_block_payoff(self) -> bool {
        self.structure.is_block_payoff()
    }

    pub(crate) const fn is_weak_enabler(self) -> bool {
        self.weak_enabler
    }

    pub(crate) const fn is_multi_hit(self) -> bool {
        self.multi_hit
    }

    pub(crate) const fn is_discard_retrieval(self) -> bool {
        self.structure.is_discard_retrieval()
    }
}

pub(crate) fn taxonomy(card_id: CardId) -> CardTaxonomy {
    use CardId::*;
    let facts = card_facts::facts(card_id);

    CardTaxonomy {
        structure: card_structure::structure(card_id),
        armaments_frontload_priority: matches!(
            card_id,
            BattleTrance
                | Corruption
                | DarkEmbrace
                | DemonForm
                | Evolve
                | HeavyBlade
                | Inflame
                | LimitBreak
                | Shockwave
                | Whirlwind
        ),
        armaments_upgrade_priority: matches!(
            card_id,
            Armaments
                | Bash
                | BattleTrance
                | BloodForBlood
                | BodySlam
                | BurningPact
                | Corruption
                | DarkEmbrace
                | DemonForm
                | Evolve
                | Exhume
                | FlameBarrier
                | GhostlyArmor
                | Havoc
                | HeavyBlade
                | Inflame
                | LimitBreak
                | PommelStrike
                | SecondWind
                | SeeingRed
                | Shockwave
                | ShrugItOff
                | TrueGrit
                | Uppercut
                | Whirlwind
        ),
        panacea_self_combo: matches!(card_id, Berserk | Flex),
        attack_followup_priority: matches!(
            card_id,
            Bash | BloodForBlood
                | Dropkick
                | HeavyBlade
                | Hemokinesis
                | Pummel
                | Rampage
                | Shockwave
                | SwordBoomerang
                | Uppercut
                | Whirlwind
        ),
        aoe: facts.aoe,
        self_damage_source: facts.self_damage,
        status_producer: facts.produces_status,
        conditional_free: facts.conditional_free,
        combat_heal: facts.combat_heal,
        vuln_enabler: facts.applies_vuln,
        weak_enabler: facts.applies_weak,
        multi_hit: facts.multi_hit,
    }
}

pub(crate) fn is_strength_enabler(card_id: CardId) -> bool {
    taxonomy(card_id).is_strength_enabler()
}

pub(crate) fn is_strength_payoff(card_id: CardId) -> bool {
    taxonomy(card_id).is_strength_payoff()
}

pub(crate) fn is_multi_attack_payoff(card_id: CardId) -> bool {
    taxonomy(card_id).is_multi_attack_payoff()
}
