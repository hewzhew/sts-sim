use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;

#[derive(Debug, Clone)]
pub struct ShopConfig {
    pub ascension_level: i32,
    pub player_class: &'static str,
    pub has_courier: bool,
    pub has_membership_card: bool,
    pub has_smiling_mask: bool,
    pub previous_purge_count: i32,
    pub potion_class: crate::content::potions::PotionClass,
    pub card_blizz_randomizer: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShopCard {
    pub card_id: CardId,
    pub price: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShopRelic {
    pub relic_id: RelicId,
    pub price: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShopPotion {
    pub potion_id: PotionId,
    pub price: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShopState {
    pub cards: Vec<ShopCard>,
    pub relics: Vec<ShopRelic>,
    pub potions: Vec<ShopPotion>,
    pub purge_cost: i32,
    pub purge_available: bool,
}

impl ShopState {
    pub fn new() -> Self {
        ShopState {
            cards: Vec::new(),
            relics: Vec::new(),
            potions: Vec::new(),
            purge_cost: 75,
            purge_available: true,
        }
    }
}
