use crate::ai::analysis::card_semantics::Mechanic;
use crate::ai::strategy::reward_admission::{
    RewardAdmission, RewardAdmissionClass, RewardAdmissionReason,
};
use crate::runtime::combat::CombatCard;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeckAdmission {
    Welcome,
    Conditional,
    Discouraged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeckAdmissionContext {
    pub act: u8,
    pub current_hp: i32,
    pub max_hp: i32,
}

impl DeckAdmissionContext {
    pub fn survival_pressure(self) -> bool {
        let max_hp = self.max_hp.max(1);
        self.current_hp.saturating_mul(2) <= max_hp
            || (self.act >= 2 && self.current_hp.saturating_mul(3) <= max_hp.saturating_mul(2))
    }
}

pub fn assess_deck_admission(
    deck: &[CombatCard],
    context: DeckAdmissionContext,
    admission: &RewardAdmission,
) -> DeckAdmission {
    if admission.card.is_none() || structurally_important(admission) {
        return DeckAdmission::Welcome;
    }
    if context.survival_pressure() && survival_relevant(admission) {
        return DeckAdmission::Welcome;
    }
    let (soft_limit, hard_limit) = deck_size_limits(context);
    let deck_size = deck.len();
    if deck_size >= hard_limit && ordinary_addition(admission) {
        return DeckAdmission::Discouraged;
    }
    if deck_size >= soft_limit && ordinary_addition(admission) {
        return DeckAdmission::Conditional;
    }
    DeckAdmission::Welcome
}

fn deck_size_limits(context: DeckAdmissionContext) -> (usize, usize) {
    if context.act >= 2 {
        (24, 30)
    } else {
        (28, 34)
    }
}

fn structurally_important(admission: &RewardAdmission) -> bool {
    matches!(
        admission.class,
        RewardAdmissionClass::ClosesRequirement | RewardAdmissionClass::BuildsSupportedPackage
    ) || admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::Installs(_)
                | RewardAdmissionReason::Provides(Mechanic::CardDraw | Mechanic::Energy)
        )
    })
}

fn survival_relevant(admission: &RewardAdmission) -> bool {
    admission.reasons.iter().any(|reason| {
        matches!(
            reason,
            RewardAdmissionReason::Provides(
                Mechanic::Block | Mechanic::Weak | Mechanic::EnemyStrengthDown
            )
        )
    })
}

fn ordinary_addition(admission: &RewardAdmission) -> bool {
    matches!(
        admission.class,
        RewardAdmissionClass::ImmediateWork | RewardAdmissionClass::BurdenedImmediateWork
    )
}
