const APOTHEOSIS_BASE_SCORE: i32 = 8_500;
const APOTHEOSIS_UPGRADE_TARGET_SCORE: i32 = 1_400;
const APOTHEOSIS_NO_TARGETS_PENALTY: i32 = 6_000;
const APOTHEOSIS_MULTI_TARGET_BONUS: i32 = 4_000;
const APOTHEOSIS_UNDER_PRESSURE_PENALTY: i32 = 1_500;

pub(crate) fn apotheosis_timing_score(
    upgradable_targets: i32,
    imminent_unblocked_damage: i32,
) -> i32 {
    let mut value =
        APOTHEOSIS_BASE_SCORE + upgradable_targets.max(0) * APOTHEOSIS_UPGRADE_TARGET_SCORE;
    if upgradable_targets <= 0 {
        value -= APOTHEOSIS_NO_TARGETS_PENALTY;
    } else if upgradable_targets >= 3 {
        value += APOTHEOSIS_MULTI_TARGET_BONUS;
    }
    if imminent_unblocked_damage > 8 {
        value -= APOTHEOSIS_UNDER_PRESSURE_PENALTY;
    }
    value
}

pub(crate) fn apotheosis_hand_shaping_score(
    upgradable_targets: i32,
    imminent_unblocked_damage: i32,
) -> i32 {
    -(apotheosis_timing_score(upgradable_targets, imminent_unblocked_damage) / 2)
}
