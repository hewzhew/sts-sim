// Kept smaller than the main role gaps in action_priority; phase facts nudge nearby
// ordering decisions without turning this module into an alternate policy.
pub(super) const PHASE_ROLE_ADJUSTMENT: i32 = 12;
pub(super) const AWAKENED_POWER_PENALTY: i32 = PHASE_ROLE_ADJUSTMENT * 2;
pub(super) const TIME_EATER_CLOCK_PENALTY: i32 = PHASE_ROLE_ADJUSTMENT;
pub(super) const STASIS_TARGET_SETUP_MAX: i32 = 20;
