use super::enemy_phase_transition::EnemyPhaseTransitionHint;
use super::phase_profile::CombatSearchPhaseProfileV1;
use super::CombatSearchV2PhaseGuardPolicy;
use crate::content::cards::CardType;
use crate::content::monsters::EnemyId;

// Kept smaller than the main role gaps in action_priority; phase facts nudge nearby
// ordering decisions without turning this module into an alternate policy.
const PHASE_ROLE_ADJUSTMENT: i32 = 12;
const AWAKENED_POWER_PENALTY: i32 = PHASE_ROLE_ADJUSTMENT * 2;
const TIME_EATER_CLOCK_PENALTY: i32 = PHASE_ROLE_ADJUSTMENT;
const STASIS_TARGET_SETUP_MAX: i32 = 20;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PhaseActionOrderingFacts {
    pub(super) card_type: CardType,
    pub(super) block: i32,
    pub(super) mitigation: i32,
    pub(super) target_progress: i32,
    pub(super) target_lethal: bool,
    pub(super) future_debuff: bool,
    pub(super) access: PhaseActionAccessFacts,
    pub(super) target_enemy_id: Option<EnemyId>,
    pub(super) target_has_stasis_card: bool,
    pub(super) phase_transition: EnemyPhaseTransitionHint,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PhaseActionAccessFacts {
    pub(super) declared_draw_cards: i32,
    pub(super) conditional_draw_cards: i32,
    pub(super) total_draw_cards: i32,
    pub(super) bad_draw_cards: i32,
    pub(super) forced_turn_end: bool,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PhaseActionOrderingHint {
    pub(super) role_rank_adjustment: i32,
    pub(super) phase_setup: i32,
    pub(super) phase_survival: i32,
    pub(super) phase_transition_safety: i32,
}

pub(super) fn phase_action_ordering_hint(
    profile: CombatSearchPhaseProfileV1,
    phase_guard_policy: CombatSearchV2PhaseGuardPolicy,
    facts: PhaseActionOrderingFacts,
) -> PhaseActionOrderingHint {
    let mut hint = PhaseActionOrderingHint {
        phase_transition_safety: -facts.phase_transition.ordering_risk_score(),
        ..PhaseActionOrderingHint::default()
    };

    if profile.enemy_mechanics.lagavulin_sleeping_count > 0 {
        apply_lagavulin_sleep_hint(&mut hint, facts);
    }
    if profile.enemy_mechanics.guardian_defensive_count > 0
        || profile.enemy_mechanics.guardian_mode_shift_pending_count > 0
    {
        hint.phase_survival = facts.block.saturating_add(facts.mitigation);
    }
    if facts.target_enemy_id == Some(EnemyId::BronzeOrb)
        && facts.target_progress > 0
        && (facts.target_has_stasis_card
            || profile.enemy_mechanics.bronze_orb_stasis_pending_count > 0)
    {
        hint.phase_setup = hint
            .phase_setup
            .saturating_add(facts.target_progress.min(STASIS_TARGET_SETUP_MAX));
    }
    if profile.enemy_mechanics.awakened_one_curiosity_count > 0
        && facts.card_type == CardType::Power
    {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_sub(AWAKENED_POWER_PENALTY);
        hint.phase_setup -= 1;
    }
    if phase_guard_policy.guards_time_eater_clock() {
        apply_time_eater_clock_hint(&mut hint, profile, facts);
    }

    hint
}

impl PhaseActionOrderingHint {
    pub(super) fn has_signal(self) -> bool {
        self.role_rank_adjustment != 0
            || self.phase_setup != 0
            || self.phase_survival != 0
            || self.phase_transition_safety != 0
    }
}

fn apply_lagavulin_sleep_hint(hint: &mut PhaseActionOrderingHint, facts: PhaseActionOrderingFacts) {
    if facts.phase_transition.lagavulin_wake_risk_count > 0 {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_sub(PHASE_ROLE_ADJUSTMENT);
        hint.phase_setup -= 1;
    } else if facts.card_type == CardType::Power {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_add(PHASE_ROLE_ADJUSTMENT);
        hint.phase_setup += 1;
    }
}

fn apply_time_eater_clock_hint(
    hint: &mut PhaseActionOrderingHint,
    profile: CombatSearchPhaseProfileV1,
    facts: PhaseActionOrderingFacts,
) {
    let triggers_warp = profile
        .enemy_mechanics
        .time_eater_cards_until_warp
        .is_some_and(|cards| cards <= 1);
    let pending_haste = profile.enemy_mechanics.time_eater_pending_haste_count > 0;
    let access_draw = facts.access.draw_cards();
    let plain_draw = access_draw > 0
        && facts.target_progress <= 0
        && facts.block <= 0
        && facts.mitigation <= 0
        && !facts.future_debuff
        && facts.card_type != CardType::Power;
    let low_impact_spam = access_draw <= 0
        && facts.target_progress <= 0
        && facts.block <= 0
        && facts.mitigation <= 0
        && !facts.future_debuff
        && facts.card_type != CardType::Power;
    let target_time_eater = facts.target_enemy_id == Some(EnemyId::TimeEater)
        || (profile.enemy_mechanics.time_eater_count == 1
            && facts.target_enemy_id.is_none()
            && facts.target_progress > 0);
    let crosses_half = target_time_eater
        && profile
            .enemy_mechanics
            .time_eater_current_hp
            .zip(profile.enemy_mechanics.time_eater_half_hp)
            .is_some_and(|(hp, half)| {
                hp >= half && hp.saturating_sub(facts.target_progress) < half
            });

    if triggers_warp
        && !facts.target_lethal
        && facts.block <= 0
        && facts.mitigation <= 0
        && facts.card_type != CardType::Power
        && (plain_draw || low_impact_spam)
    {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_sub(TIME_EATER_CLOCK_PENALTY);
        hint.phase_transition_safety -= 1 + facts.access.time_warp_access_risk();
    }

    if pending_haste && target_time_eater && !facts.target_lethal {
        if facts.target_progress > 0
            && facts.block <= 0
            && facts.mitigation <= 0
            && facts.card_type != CardType::Power
        {
            hint.role_rank_adjustment = hint
                .role_rank_adjustment
                .saturating_sub(TIME_EATER_CLOCK_PENALTY);
            hint.phase_transition_safety -= facts.target_progress.min(20);
        }
        if facts.future_debuff && facts.mitigation <= 0 {
            hint.role_rank_adjustment = hint
                .role_rank_adjustment
                .saturating_sub(TIME_EATER_CLOCK_PENALTY);
            hint.phase_transition_safety -= 1;
        }
    }

    if crosses_half
        && !facts.target_lethal
        && facts.block <= 0
        && facts.mitigation <= 0
        && facts.card_type != CardType::Power
    {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_sub(TIME_EATER_CLOCK_PENALTY);
        hint.phase_transition_safety -= 1;
    }
}

impl PhaseActionAccessFacts {
    fn draw_cards(self) -> i32 {
        self.total_draw_cards.max(
            self.declared_draw_cards
                .saturating_add(self.conditional_draw_cards),
        )
    }

    fn time_warp_access_risk(self) -> i32 {
        self.bad_draw_cards
            .max(0)
            .saturating_add(i32::from(self.forced_turn_end))
    }
}

#[cfg(test)]
mod tests;
