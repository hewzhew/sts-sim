use crate::content::cards::CardId;

pub fn on_calculate_damage_to_enemy(card_id: CardId, base_magic_num: i32, mut damage: f32, amount: i32) -> f32 {
    let strength_multiplier = if card_id == CardId::HeavyBlade { base_magic_num } else { 1 };
    damage -= (amount * strength_multiplier) as f32;
    damage
}
