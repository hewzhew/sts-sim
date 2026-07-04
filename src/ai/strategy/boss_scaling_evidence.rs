use crate::ai::analysis::card_semantics::{
    card_definition_with_upgrades, CombatEvent, DamageScalingAxis, InstalledRule, Mechanic,
    PayoffRequirement, TriggeredEffect,
};
use crate::ai::strategy::deck_plan::DeckPlanSnapshot;
use crate::ai::strategy::package_transition::PackageKind;
use crate::ai::strategy::reward_admission::{RewardAdmission, RewardAdmissionReason};
use crate::content::cards::CardId;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BossScalingEvidence {
    pub label: &'static str,
    pub score_delta: i32,
    pub relevant_to_boss_plan: bool,
}

impl BossScalingEvidence {
    const fn relevant(label: &'static str, score_delta: i32) -> Self {
        Self {
            label,
            score_delta,
            relevant_to_boss_plan: true,
        }
    }

    const fn score_only(label: &'static str, score_delta: i32) -> Self {
        Self {
            label,
            score_delta,
            relevant_to_boss_plan: false,
        }
    }

    const fn none() -> Self {
        Self {
            label: "",
            score_delta: 0,
            relevant_to_boss_plan: false,
        }
    }
}

pub fn assess_boss_scaling_evidence(
    deck: DeckPlanSnapshot,
    card: Option<(CardId, u8)>,
    admission: &RewardAdmission,
) -> BossScalingEvidence {
    let card_semantics = card.map(|(id, upgrades)| card_definition_with_upgrades(id, upgrades));

    if admission_provides(admission, Mechanic::Strength)
        || card_grants_strength(card_semantics.as_ref())
    {
        return BossScalingEvidence::relevant("boss-scaling-source", 70);
    }
    if admission_provides(admission, Mechanic::StrengthMultiplier) {
        return if deck.roles.strength_source_units > 0 {
            BossScalingEvidence::relevant("boss-scaling-multiplier", 45)
        } else {
            BossScalingEvidence::score_only("boss-speculative-multiplier", -35)
        };
    }
    if installs_corruption(admission) && deck.roles.exhaust_payoff_units > 0 {
        return BossScalingEvidence::relevant("boss-exhaust-engine-pair", 75);
    }
    if candidate_exhaust_payoff(card_semantics.as_ref()) && deck.roles.corruption_units > 0 {
        return if deck.roles.exhaust_payoff_units == 0 {
            BossScalingEvidence::relevant("boss-exhaust-engine-pair", 75)
        } else {
            BossScalingEvidence::score_only("boss-duplicate-engine-payoff", -25)
        };
    }
    if candidate_exhaust_source(admission) && deck.roles.exhaust_payoff_units > 0 {
        return BossScalingEvidence::relevant("boss-exhaust-engine-piece", 45);
    }
    if candidate_block_payoff(admission) {
        return if deck.roles.block_units >= 4 || deck.roles.cycle_block_units >= 2 {
            BossScalingEvidence::relevant("boss-block-damage-engine", 55)
        } else {
            BossScalingEvidence::score_only("boss-speculative-payoff", -35)
        };
    }
    if candidate_strength_payoff(admission) {
        return if deck.roles.strength_source_units > 0 && deck.roles.strength_payoff_units == 0 {
            BossScalingEvidence::relevant("boss-strength-payoff", 40)
        } else {
            BossScalingEvidence::score_only("boss-speculative-payoff", -35)
        };
    }
    if admission_provides(admission, Mechanic::CardDraw)
        || admission_provides(admission, Mechanic::Energy)
    {
        return BossScalingEvidence::score_only("boss-tempo-access", 25);
    }
    if admission_provides(admission, Mechanic::Weak)
        || admission_provides(admission, Mechanic::EnemyStrengthDown)
        || admission_provides(admission, Mechanic::Vulnerable)
    {
        return BossScalingEvidence::score_only("boss-support-only", 30);
    }
    if admission_provides(admission, Mechanic::Block) {
        return BossScalingEvidence::score_only("boss-premium-survival", 20);
    }
    BossScalingEvidence::none()
}

fn admission_provides(admission: &RewardAdmission, mechanic: Mechanic) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::Provides(mechanic))
}

fn installs_corruption(admission: &RewardAdmission) -> bool {
    admission.reasons.contains(&RewardAdmissionReason::Installs(
        InstalledRule::SkillCardsCostZeroAndExhaust,
    ))
}

fn candidate_exhaust_source(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::Emits(CombatEvent::CardExhausted))
        || admission
            .reasons
            .contains(&RewardAdmissionReason::PlaysTopCardAndExhaust)
}

fn candidate_block_payoff(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::DamageUses(Mechanic::Block))
        || admission
            .reasons
            .contains(&RewardAdmissionReason::Supports(PackageKind::Block))
        || admission.reasons.contains(&RewardAdmissionReason::Closes(
            PayoffRequirement::WantsMechanic(Mechanic::Block),
        ))
}

fn candidate_strength_payoff(admission: &RewardAdmission) -> bool {
    admission
        .reasons
        .contains(&RewardAdmissionReason::DamageUses(Mechanic::Strength))
        || admission
            .reasons
            .contains(&RewardAdmissionReason::DamageScalesWith(
                DamageScalingAxis::PerHitStrength,
            ))
        || admission
            .reasons
            .contains(&RewardAdmissionReason::Supports(PackageKind::Strength))
        || admission.reasons.contains(&RewardAdmissionReason::Closes(
            PayoffRequirement::WantsMechanic(Mechanic::Strength),
        ))
}

fn card_grants_strength(
    semantics: Option<&crate::ai::analysis::card_semantics::CardDefinition>,
) -> bool {
    semantics.is_some_and(|definition| {
        definition
            .event_handlers
            .iter()
            .any(|handler| handler.effect == TriggeredEffect::Provide(Mechanic::Strength))
    })
}

fn candidate_exhaust_payoff(
    semantics: Option<&crate::ai::analysis::card_semantics::CardDefinition>,
) -> bool {
    semantics.is_some_and(|definition| {
        definition.event_handlers.iter().any(|handler| {
            handler.on == CombatEvent::CardExhausted
                && matches!(
                    handler.effect,
                    TriggeredEffect::Provide(Mechanic::Block | Mechanic::CardDraw)
                )
        })
    })
}
