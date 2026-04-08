use crate::action::{DamageInfo, DamageType};

pub fn on_attacked_to_change_damage(info: &DamageInfo, damage_amount: i32) -> i32 {
    if damage_amount > 1
        && damage_amount <= 5
        && info.damage_type != DamageType::HpLoss
        && info.damage_type != DamageType::Thorns
        && info.source != 0
    {
        1
    } else {
        damage_amount
    }
}
