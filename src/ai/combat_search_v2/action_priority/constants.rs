// These ranks only decide child-generation order inside the same legal action set.
// They never merge, prune, or claim that two actions are equivalent.
pub(super) const ROLE_LETHAL_CARD: i32 = 130;
pub(super) const ROLE_PREVENT_VISIBLE_LETHAL: i32 = 120;
pub(super) const ROLE_SUSTAINED_MITIGATION: i32 = 95;
pub(super) const ROLE_KEY_SETUP_CARD: i32 = 90;
pub(super) const ROLE_TACTICAL_POTION_BASE: i32 = 60;
pub(super) const ROLE_PREVENT_HP_LOSS: i32 = 85;
pub(super) const ROLE_CURRENT_TURN_RETALIATION_PROTECTION: i32 = ROLE_PREVENT_HP_LOSS;
pub(super) const ROLE_CURRENT_TURN_ATTACK_SETUP: i32 = 80;
pub(super) const ROLE_DEFERRED_SETUP: i32 = 75;
pub(super) const ROLE_DAMAGE_PROGRESS: i32 = 60;
pub(super) const ROLE_REACTIVE_RISK_PREVENT_HP_LOSS: i32 = 55;
pub(super) const ROLE_BLOCK: i32 = 45;
pub(super) const ROLE_UTILITY_PLAY: i32 = 35;
pub(super) const ROLE_END_TURN: i32 = 0;
pub(super) const ROLE_PENDING_VALUE_SELECTION: i32 = 70;
pub(super) const ROLE_PENDING_REMOVAL_SELECTION: i32 = 65;
pub(super) const ROLE_PENDING_NEUTRAL_SELECTION: i32 = 20;
pub(super) const ROLE_PENDING_CANCEL: i32 = -10;
pub(super) const ROLE_DISCARD_POTION: i32 = -20;
