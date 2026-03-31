/// Magic Flower: Healing is 50% more effective during combat.
/// Handled statically inside engine HP modifications (`Heal` action). 
/// This hook is essentially a no-op but acts as documentation. (A function could be exported to wrap heals).

pub fn modify_heal(amount: i32) -> i32 {
    let new_amount = (amount as f32 * 1.5).round() as i32;
    new_amount
}
