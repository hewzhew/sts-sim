#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PotionGateDecision {
    pub(super) allowed: bool,
    pub(super) reason: PotionGateReason,
}

impl PotionGateDecision {
    pub(super) fn allow(reason: PotionGateReason) -> Self {
        Self {
            allowed: true,
            reason,
        }
    }

    pub(super) fn reject(reason: PotionGateReason) -> Self {
        Self {
            allowed: false,
            reason,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum PotionGateReason {
    DirectDamageCanKill,
    VisibleIncomingHpLoss,
    NoVisibleHandLethal,
    PlayerWounded,
    InvalidPotionAction,
    PotionSlotMissing,
    PassiveOnly,
    InvalidTarget,
    NoLivingEnemy,
    NoVisibleIncomingHpLoss,
    NoTacticalPressure,
    NotWounded,
    EscapeNotWin,
    RandomPotionGenerationUnsupported,
}
