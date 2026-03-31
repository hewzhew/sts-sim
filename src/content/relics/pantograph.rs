// Pantograph: At the start of boss combats, heal 25 HP.

pub fn is_pantograph() -> bool {
    true
}

// NOTE: Since pantograph heals at the start of BOSS combats, we should
// implement a custom logic check inside the `hooks::at_battle_start` 
// dispatch if `state.is_boss_combat` or similar node tracking is present.
