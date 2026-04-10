use crate::combat::CombatCard;

pub fn modify_attack_damage_for_card(card: &CombatCard, damage: f32) -> f32 {
    let def = crate::content::cards::get_card_definition(card.id);
    if def.card_type == crate::content::cards::CardType::Attack
        && def.tags.contains(&crate::content::cards::CardTag::Strike)
    {
        damage + 3.0
    } else {
        damage
    }
}
