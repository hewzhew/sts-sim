pub fn on_calculate_block(mut block: f32, amount: i32) -> f32 {
    block += amount as f32;
    block
}
