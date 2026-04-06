use crate::content::cards::{CardId, CardType, get_card_definition};
use crate::content::relics::RelicId;
use crate::combat::CombatCard;
use super::context::DeckContext;

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
    pub removed_successfully: bool,
    pub actions: Vec<DeckAction>,
}

pub struct DeckManager;

impl DeckManager {
    /// Simulates Soul.java/AbstractCard obtain hooks
    pub fn obtain_card(ctx: &DeckContext, card_id: CardId, next_uuid: &mut u32) -> ObtainResult {
        let def = get_card_definition(card_id);
        let mut actions = Vec::new();
        
        let is_curse = def.card_type == CardType::Curse;
        
        // --- 1. Interception Phase (Omamori) ---
        if is_curse && ctx.has_omamori && ctx.omamori_charges > 0 {
            actions.push(DeckAction::PreventObtain);
            actions.push(DeckAction::UpdateRelicCounter(RelicId::Omamori, ctx.omamori_charges - 1));
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
                if ctx.has_molten_egg { should_upgrade = true; }
            }
            CardType::Skill => {
                if ctx.has_toxic_egg { should_upgrade = true; }
            }
            CardType::Power => {
                if ctx.has_frozen_egg { should_upgrade = true; }
            }
            _ => {}
        }
        
        // --- 4. Hoarder Checks & Instantiation ---
        let copy_count = if ctx.has_hoarder_mod { 3 } else { 1 };
        let mut final_cards = Vec::new();
        
        for _ in 0..copy_count {
            let mut card = CombatCard::new(card_id, *next_uuid);
            *next_uuid += 1;
            
            if should_upgrade {
                card.upgrades += 1;
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
            removed_successfully: true,
            actions,
        }
    }
}
