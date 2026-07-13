use crate::ai::analysis::card_semantics::{
    card_definition_with_upgrades, CardBurden, CardDefinition, CombatEvent, DamageScalingAxis,
    DeckMechanicContext, InstalledRule, Mechanic, PlayEffect, TriggeredEffect,
};
use crate::ai::card_analysis_v1::{card_analysis_profile_v1, CardAnalysisAoeSupportV1};
use crate::content::cards::{get_card_definition, CardId};
use crate::runtime::combat::CombatCard;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DeckRoleInventory {
    pub frontload_units: u8,
    pub aoe_units: u8,
    pub strong_aoe_units: u8,
    pub block_units: u8,
    pub cycle_block_units: u8,
    pub mitigation_units: u8,
    pub debuff_units: u8,
    pub vulnerable_units: u8,
    pub draw_units: u8,
    pub energy_units: u8,
    pub x_cost_payoff_units: u8,
    pub strength_source_units: u8,
    pub conditional_strength_source_units: u8,
    pub repeatable_self_damage_supply: bool,
    pub strength_multiplier_units: u8,
    pub corruption_units: u8,
    pub exhaust_stream_units: u8,
    pub exhaust_payoff_units: u8,
    pub block_payoff_units: u8,
    pub strength_payoff_units: u8,
    pub upgrade_access_units: u8,
}

impl DeckRoleInventory {
    pub fn from_deck(deck: &[CombatCard]) -> Self {
        let mut inventory = Self::default();
        let definitions = deck
            .iter()
            .map(|card| card_definition_with_upgrades(card.id, card.upgrades))
            .collect::<Vec<_>>();
        let context = DeckMechanicContext::from_definitions(&definitions);
        inventory.repeatable_self_damage_supply = context
            .repeatable_event_streams
            .contains(&CombatEvent::CardSelfDamage);

        for (card, definition) in deck.iter().zip(&definitions) {
            if card_is_strong_aoe(card.id, card.upgrades) {
                inventory.strong_aoe_units += 1;
            }
            if get_card_definition(card.id).cost == -1 {
                inventory.x_cost_payoff_units += 1;
            }
            let provides_block = definition
                .play_effects
                .contains(&PlayEffect::Provide(Mechanic::Block));
            let provides_draw = definition
                .play_effects
                .contains(&PlayEffect::Provide(Mechanic::CardDraw));
            if provides_block && provides_draw {
                inventory.cycle_block_units += 1;
            }
            let conditional_strength_provider =
                definition_is_conditional_strength_source(definition);
            if conditional_strength_provider {
                inventory.conditional_strength_source_units += 1;
            }
            for effect in &definition.play_effects {
                if conditional_strength_provider
                    && *effect == PlayEffect::Provide(Mechanic::Strength)
                {
                    continue;
                }
                inventory.add_play_effect(*effect);
            }
            for rule in &definition.installed_rules {
                inventory.add_installed_rule(*rule);
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
        if event == CombatEvent::CardExhausted
            && matches!(
                effect,
                TriggeredEffect::Provide(Mechanic::Block | Mechanic::CardDraw)
            )
        {
            self.exhaust_payoff_units += 1;
        }
        if effect != TriggeredEffect::Provide(Mechanic::Strength) {
            return;
        }
        if event == CombatEvent::CardSelfDamage && !self.repeatable_self_damage_supply {
            self.conditional_strength_source_units += 1;
            return;
        }
        self.strength_source_units += match event {
            CombatEvent::TurnStart => 2,
            CombatEvent::CardSelfDamage => 1,
            _ => 1,
        };
    }

    fn add_installed_rule(&mut self, rule: InstalledRule) {
        match rule {
            InstalledRule::SkillCardsCostZeroAndExhaust => self.corruption_units += 1,
        }
    }

    fn add_mechanic(&mut self, mechanic: Mechanic) {
        match mechanic {
            Mechanic::Block => self.block_units += 1,
            Mechanic::Weak | Mechanic::EnemyStrengthDown => {
                self.mitigation_units += 1;
                self.debuff_units += 1;
            }
            Mechanic::Vulnerable => {
                self.debuff_units += 1;
                self.vulnerable_units += 1;
            }
            Mechanic::CardDraw => self.draw_units += 1,
            Mechanic::Energy => self.energy_units += 1,
            Mechanic::Strength => self.strength_source_units += 1,
            Mechanic::StrengthMultiplier => self.strength_multiplier_units += 1,
            Mechanic::TemporaryStrength | Mechanic::TopdeckControl => {}
        }
    }
}

pub(super) fn card_is_strong_aoe(card: CardId, upgrades: u8) -> bool {
    card == CardId::Combust
        || card_analysis_profile_v1(card, upgrades).aoe_support == CardAnalysisAoeSupportV1::Strong
}

pub(super) fn card_is_stable_strength_source(
    card: CardId,
    upgrades: u8,
    repeatable_self_damage_supply: bool,
) -> bool {
    let definition = card_definition_with_upgrades(card, upgrades);
    (definition
        .play_effects
        .contains(&PlayEffect::Provide(Mechanic::Strength))
        && !definition_is_conditional_strength_source(&definition))
        || definition.event_handlers.iter().any(|handler| {
            handler.effect == TriggeredEffect::Provide(Mechanic::Strength)
                && (handler.on != CombatEvent::CardSelfDamage || repeatable_self_damage_supply)
        })
}

fn definition_is_conditional_strength_source(definition: &CardDefinition) -> bool {
    definition
        .play_effects
        .contains(&PlayEffect::Provide(Mechanic::Strength))
        && definition
            .burdens
            .contains(&CardBurden::RequiresEnemyAttackIntent)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(id: CardId, uuid: u32) -> CombatCard {
        CombatCard::new(id, uuid)
    }

    #[test]
    fn spot_weakness_counts_as_conditional_strength_not_stable_strength() {
        let inventory = DeckRoleInventory::from_deck(&[
            card(CardId::SpotWeakness, 1),
            card(CardId::SpotWeakness, 2),
        ]);

        assert_eq!(inventory.strength_source_units, 0);
        assert_eq!(inventory.conditional_strength_source_units, 2);
    }

    #[test]
    fn inflame_and_demon_form_count_as_stable_strength_sources() {
        let inventory =
            DeckRoleInventory::from_deck(&[card(CardId::Inflame, 1), card(CardId::DemonForm, 2)]);

        assert_eq!(inventory.strength_source_units, 3);
        assert_eq!(inventory.conditional_strength_source_units, 0);
    }

    #[test]
    fn limit_break_counts_as_multiplier_without_becoming_a_strength_source() {
        let inventory =
            DeckRoleInventory::from_deck(&[card(CardId::Inflame, 1), card(CardId::LimitBreak, 2)]);

        assert_eq!(inventory.strength_source_units, 1);
        assert_eq!(inventory.strength_multiplier_units, 1);
    }

    #[test]
    fn offering_backed_rupture_is_conditional_not_stable_strength() {
        let inventory =
            DeckRoleInventory::from_deck(&[card(CardId::Offering, 1), card(CardId::Rupture, 2)]);

        assert!(!inventory.repeatable_self_damage_supply);
        assert_eq!(inventory.strength_source_units, 0);
        assert_eq!(inventory.conditional_strength_source_units, 1);
    }

    #[test]
    fn repeatable_self_damage_makes_rupture_stable_strength() {
        let inventory = DeckRoleInventory::from_deck(&[
            card(CardId::Bloodletting, 1),
            card(CardId::Rupture, 2),
        ]);

        assert!(inventory.repeatable_self_damage_supply);
        assert_eq!(inventory.strength_source_units, 1);
    }

    #[test]
    fn role_inventory_distinguishes_weak_and_strong_aoe() {
        let inventory = DeckRoleInventory::from_deck(&[
            card(CardId::Cleave, 1),
            card(CardId::Cleave, 2),
            card(CardId::Whirlwind, 3),
        ]);

        assert_eq!(inventory.aoe_units, 3);
        assert_eq!(inventory.strong_aoe_units, 1);
    }
}
