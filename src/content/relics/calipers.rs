
pub fn on_equip() {
    // Note: Engine-level mechanics handled directly via `state.player.block = (state.player.block - 15).max(0);` in `engine.rs`
    // within `Action::EndTurnTrigger`, zeroing out block gracefully without full wipe.
}
