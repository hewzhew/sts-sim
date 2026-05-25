use super::*;
use crate::content::cards;
#[cfg(test)]
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageInfo};
use crate::runtime::combat::CombatCard;

mod projection;
use projection::{PhaseProjection, ProjectedMonsterDamage};

const SPLIT_MOVE_ID: u8 = 3;
const SPLIT_TRIGGER_RISK_PER_DEBT_HP: i32 = 3;
const GUARDIAN_MODE_SHIFT_TRIGGER_RISK: i32 = 40;
const LAGAVULIN_WAKE_RISK: i32 = 80;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct EnemyPhaseTransitionHint {
    pub(super) split_trigger_count: usize,
    pub(super) split_debt_hp: i32,
    pub(super) guardian_mode_shift_trigger_count: usize,
    pub(super) guardian_min_threshold_remaining_before_hit: Option<i32>,
    pub(super) lagavulin_wake_risk_count: usize,
}

pub(super) fn enemy_phase_transition_hint_for_input(
    combat: &CombatState,
    input: &ClientInput,
) -> EnemyPhaseTransitionHint {
    match input {
        ClientInput::PlayCard { card_index, target } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| play_card_phase_transition_hint(combat, card, *target))
            .unwrap_or_default(),
        _ => EnemyPhaseTransitionHint::default(),
    }
}

impl EnemyPhaseTransitionHint {
    pub(super) fn ordering_risk_score(self) -> i32 {
        self.split_debt_hp
            .saturating_mul(SPLIT_TRIGGER_RISK_PER_DEBT_HP)
            .saturating_add(
                (self.guardian_mode_shift_trigger_count as i32)
                    .saturating_mul(GUARDIAN_MODE_SHIFT_TRIGGER_RISK),
            )
            .saturating_add(
                (self.lagavulin_wake_risk_count as i32).saturating_mul(LAGAVULIN_WAKE_RISK),
            )
    }
}

fn play_card_phase_transition_hint(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> EnemyPhaseTransitionHint {
    let mut hint = EnemyPhaseTransitionHint::default();
    let evaluated = cards::evaluate_card_for_play(card, combat, target);
    let actions = cards::resolve_card_play_with_context(
        evaluated.id,
        combat,
        &evaluated,
        target,
        cards::CardUseContext {
            played_from_hand: true,
        },
    );
    let mut projection = PhaseProjection::from_combat(combat);

    for action in actions {
        observe_action_damage(&mut projection, action.action);
    }

    for projected in projection.monsters.values() {
        observe_split_transition(&mut hint, projected);
        observe_guardian_transition(&mut hint, projected);
        observe_lagavulin_transition(&mut hint, projected);
    }

    hint
}

fn observe_action_damage(projection: &mut PhaseProjection, action: Action) {
    match action {
        Action::Damage(info)
        | Action::PummelDamage(info)
        | Action::BaneDamage(info)
        | Action::WallopDamage(info)
        | Action::DamagePerAttackPlayed(info)
        | Action::HeelHook(info)
        | Action::Flechettes(info)
        | Action::DropkickDamageAndEffect {
            damage_info: info, ..
        }
        | Action::Ftl {
            damage_info: info, ..
        }
        | Action::Skewer {
            damage_info: info, ..
        }
        | Action::Sunder {
            damage_info: info, ..
        }
        | Action::FearNoEvil {
            damage_info: info, ..
        }
        | Action::FiendFire {
            damage_info: info, ..
        }
        | Action::Feed {
            damage_info: info, ..
        }
        | Action::LessonLearned {
            damage_info: info, ..
        }
        | Action::HandOfGreed {
            damage_info: info, ..
        }
        | Action::RitualDagger {
            damage_info: info, ..
        }
        | Action::VampireDamage(info)
        | Action::Barrage { damage: info } => observe_damage_info(projection, &info),
        Action::DamageAllEnemies { damages, .. }
        | Action::VampireDamageAllEnemies { damages, .. } => {
            for (slot, damage) in damages.iter().copied().enumerate() {
                projection.apply_damage_to_slot(slot, damage);
            }
        }
        Action::Whirlwind { damages, .. } => {
            for (slot, damage) in damages.iter().copied().enumerate() {
                projection.apply_damage_to_slot(slot, damage);
            }
        }
        _ => {}
    }
}

fn observe_damage_info(projection: &mut PhaseProjection, info: &DamageInfo) {
    projection.apply_damage_to_entity(info.target, info.output);
}

fn observe_split_transition(
    hint: &mut EnemyPhaseTransitionHint,
    projected: &ProjectedMonsterDamage,
) {
    if !projected.split_power || projected.large_slime_split_already_triggered {
        return;
    }
    if projected.planned_move_id == SPLIT_MOVE_ID {
        return;
    }
    let threshold = projected.max_hp.saturating_div(2);
    if projected.current_hp > threshold
        && projected.projected_hp <= threshold
        && projected.projected_hp > 0
    {
        hint.split_trigger_count += 1;
        hint.split_debt_hp = hint.split_debt_hp.saturating_add(projected.projected_hp);
    }
}

fn observe_guardian_transition(
    hint: &mut EnemyPhaseTransitionHint,
    projected: &ProjectedMonsterDamage,
) {
    if !projected.guardian_open || projected.guardian_close_up_triggered || projected.hp_loss <= 0 {
        return;
    }
    let Some(remaining) = projected.guardian_mode_shift_remaining else {
        return;
    };
    hint.guardian_min_threshold_remaining_before_hit = Some(
        hint.guardian_min_threshold_remaining_before_hit
            .map_or(remaining, |old| old.min(remaining)),
    );
    if projected.hp_loss >= remaining.max(0) {
        hint.guardian_mode_shift_trigger_count += 1;
    }
}

fn observe_lagavulin_transition(
    hint: &mut EnemyPhaseTransitionHint,
    projected: &ProjectedMonsterDamage,
) {
    if projected.lagavulin_sleeping && projected.hp_loss > 0 {
        hint.lagavulin_wake_risk_count += 1;
    }
}

#[cfg(test)]
mod tests;
