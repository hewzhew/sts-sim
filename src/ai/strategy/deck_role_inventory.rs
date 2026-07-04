use crate::ai::analysis::card_semantics::{
    card_definition_with_upgrades, CombatEvent, DamageScalingAxis, Mechanic, PlayEffect,
    TriggeredEffect,
};
use crate::runtime::combat::CombatCard;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeckRoleInventory {
    pub frontload_units: u8,
    pub aoe_units: u8,
    pub block_units: u8,
    pub cycle_block_units: u8,
    pub mitigation_units: u8,
    pub debuff_units: u8,
    pub draw_units: u8,
    pub energy_units: u8,
    pub strength_source_units: u8,
    pub exhaust_stream_units: u8,
    pub block_payoff_units: u8,
    pub strength_payoff_units: u8,
    pub upgrade_access_units: u8,
}

impl DeckRoleInventory {
    pub fn from_deck(deck: &[CombatCard]) -> Self {
        let mut inventory = Self::default();
        for card in deck {
            let definition = card_definition_with_upgrades(card.id, card.upgrades);
            let provides_block = definition
                .play_effects
                .contains(&PlayEffect::Provide(Mechanic::Block));
            let provides_draw = definition
                .play_effects
                .contains(&PlayEffect::Provide(Mechanic::CardDraw));
            if provides_block && provides_draw {
                inventory.cycle_block_units += 1;
            }
            for effect in &definition.play_effects {
                inventory.add_play_effect(*effect);
            }
            for handler in &definition.event_handlers {
                inventory.add_event_handler(handler.on, handler.effect);
            }
        }
        inventory
    }

    fn add_play_effect(&mut self, effect: PlayEffect) {
        match effect {
            PlayEffect::FrontloadDamage => self.frontload_units += 1,
            PlayEffect::AreaDamage => self.aoe_units += 1,
            PlayEffect::DamageUses(Mechanic::Block) => self.block_payoff_units += 1,
            PlayEffect::DamageUses(Mechanic::Strength)
            | PlayEffect::DamageScalesWith(DamageScalingAxis::PerHitStrength) => {
                self.strength_payoff_units += 1;
            }
            PlayEffect::Provide(mechanic) => self.add_mechanic(mechanic),
            PlayEffect::EmitEvent(CombatEvent::CardExhausted)
            | PlayEffect::PlayTopCardAndExhaust => self.exhaust_stream_units += 1,
            PlayEffect::CombatUpgradeSingle | PlayEffect::CombatUpgradeAll => {
                self.upgrade_access_units += 1;
            }
            PlayEffect::DamageUses(_)
            | PlayEffect::DamageScalesWith(_)
            | PlayEffect::EmitEvent(_)
            | PlayEffect::ExhaustsSelf
            | PlayEffect::RunReward(_)
            | PlayEffect::RecoverCurrentHp
            | PlayEffect::CostReducedByHpLossThisCombat
            | PlayEffect::AddCombatDeckClutter => {}
        }
    }

    fn add_event_handler(&mut self, event: CombatEvent, effect: TriggeredEffect) {
        if effect != TriggeredEffect::Provide(Mechanic::Strength) {
            return;
        }
        self.strength_source_units += match event {
            CombatEvent::TurnStart => 2,
            CombatEvent::CardSelfDamage => 1,
            _ => 1,
        };
    }

    fn add_mechanic(&mut self, mechanic: Mechanic) {
        match mechanic {
            Mechanic::Block => self.block_units += 1,
            Mechanic::Weak | Mechanic::EnemyStrengthDown => {
                self.mitigation_units += 1;
                self.debuff_units += 1;
            }
            Mechanic::Vulnerable => self.debuff_units += 1,
            Mechanic::CardDraw => self.draw_units += 1,
            Mechanic::Energy => self.energy_units += 1,
            Mechanic::Strength => self.strength_source_units += 1,
            Mechanic::TemporaryStrength
            | Mechanic::StrengthMultiplier
            | Mechanic::TopdeckControl => {}
        }
    }
}
