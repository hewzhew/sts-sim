use super::context::DeckContext;
use crate::combat::CombatCard;
use crate::content::cards::{get_card_definition, CardId, CardType};
use crate::content::relics::RelicId;

pub enum DeckAction {
    PreventObtain,
    GainGold(i32),
    GainMaxHp(i32),
    LoseMaxHp(i32),
    UpdateRelicCounter(RelicId, i32),
    TriggerObtainCard(CardId),
}

pub struct ObtainResult {
    pub final_cards: Vec<CombatCard>,
    pub actions: Vec<DeckAction>,
}

pub struct RemoveResult {
    pub actions: Vec<DeckAction>,
}

pub struct DeckManager;

impl DeckManager {
    /// Simulates Soul.java/AbstractCard obtain hooks
    pub fn obtain_card(
        ctx: &DeckContext,
        card_id: CardId,
        next_uuid: &mut u32,
        pre_upgrades: u8,
    ) -> ObtainResult {
        let def = get_card_definition(card_id);
        let mut actions = Vec::new();

        let is_curse = def.card_type == CardType::Curse;

        // --- 1. Interception Phase (Omamori) ---
        if is_curse && ctx.has_omamori && ctx.omamori_charges > 0 {
            actions.push(DeckAction::PreventObtain);
            actions.push(DeckAction::UpdateRelicCounter(
                RelicId::Omamori,
                ctx.omamori_charges - 1,
            ));
            return ObtainResult {
                final_cards: Vec::new(),
                actions,
            };
        }

        // --- 2. Side Effect Checks ---
        if is_curse && ctx.has_darkstone_periapt {
            actions.push(DeckAction::GainMaxHp(6));
        }

        if ctx.has_ceramic_fish {
            actions.push(DeckAction::GainGold(9));
        }

        // --- 3. Modification Phase (Eggs) ---
        let mut should_upgrade = false;
        match def.card_type {
            CardType::Attack => {
                if ctx.has_molten_egg {
                    should_upgrade = true;
                }
            }
            CardType::Skill => {
                if ctx.has_toxic_egg {
                    should_upgrade = true;
                }
            }
            CardType::Power => {
                if ctx.has_frozen_egg {
                    should_upgrade = true;
                }
            }
            _ => {}
        }

        let copy_count = if ctx.has_hoarder_mod { 3 } else { 1 };
        let mut final_cards = Vec::new();

        for _ in 0..copy_count {
            let mut card = CombatCard::new(card_id, *next_uuid);
            *next_uuid += 1;

            card.upgrades = pre_upgrades;
            if should_upgrade {
                // Mimics Java: if (!c.upgraded) c.upgrade()
                // Usually prevents double upgrades, but Searing Blow can stack.
                // If it's 0, it becomes 1. If it's already 1 (e.g. from cardUpgradedChance or previously), we leave it or add 1 if Searing Blow.
                if card_id == CardId::SearingBlow {
                    card.upgrades += 1;
                } else {
                    card.upgrades = card.upgrades.max(1);
                }
            }

            final_cards.push(card);
        }

        ObtainResult {
            final_cards,
            actions,
        }
    }

    /// Simulates CardGroup.java removeCard / AbstractCard onRemoveFromMasterDeck hooks
    pub fn remove_card(target_id: CardId) -> RemoveResult {
        let mut actions = Vec::new();

        // 1. Parasite loses max hp
        if target_id == CardId::Parasite {
            actions.push(DeckAction::LoseMaxHp(3));
        }

        // 2. Necronomicurse replicates itself
        if target_id == CardId::Necronomicurse {
            // Note: in Java it also flashes the relic here
            // But since it literally just returns an effect that spawns the card...
            actions.push(DeckAction::TriggerObtainCard(CardId::Necronomicurse));
        }

        RemoveResult {
            actions,
        }
    }
}
