use super::*;
use crate::content::cards;
use crate::content::powers::{store, PowerId};
use crate::runtime::action::{Action, DamageInfo};
use crate::runtime::combat::{CombatCard, MonsterEntity};
use std::collections::BTreeMap;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct ProjectedMonsterDamage {
    entity_id: usize,
    current_hp: i32,
    max_hp: i32,
    projected_hp: i32,
    projected_block: i32,
    hp_loss: i32,
    split_power: bool,
    large_slime_split_already_triggered: bool,
    planned_move_id: u8,
    guardian_open: bool,
    guardian_close_up_triggered: bool,
    guardian_mode_shift_remaining: Option<i32>,
    lagavulin_sleeping: bool,
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

#[derive(Clone, Debug, Default)]
struct PhaseProjection {
    monsters: BTreeMap<usize, ProjectedMonsterDamage>,
    slot_entity_ids: Vec<usize>,
}

impl PhaseProjection {
    fn from_combat(combat: &CombatState) -> Self {
        let mut monsters = BTreeMap::new();
        let mut slot_entity_ids = Vec::new();
        for monster in &combat.entities.monsters {
            let Some(projected) = projected_monster_from(combat, monster) else {
                continue;
            };
            slot_entity_ids.push(projected.entity_id);
            monsters.insert(projected.entity_id, projected);
        }
        Self {
            monsters,
            slot_entity_ids,
        }
    }

    fn apply_damage_to_entity(&mut self, entity_id: usize, damage: i32) {
        let Some(projected) = self.monsters.get_mut(&entity_id) else {
            return;
        };
        projected.apply_damage(damage);
    }

    fn apply_damage_to_slot(&mut self, slot: usize, damage: i32) {
        let Some(entity_id) = self.slot_entity_ids.get(slot).copied() else {
            return;
        };
        self.apply_damage_to_entity(entity_id, damage);
    }
}

impl ProjectedMonsterDamage {
    fn apply_damage(&mut self, damage: i32) {
        if damage <= 0 || self.projected_hp <= 0 {
            return;
        }
        let unblocked = if self.projected_block > 0 {
            if damage >= self.projected_block {
                let remaining = damage - self.projected_block;
                self.projected_block = 0;
                remaining
            } else {
                self.projected_block -= damage;
                0
            }
        } else {
            damage
        };
        let hp_loss = unblocked.min(self.projected_hp).max(0);
        self.projected_hp -= hp_loss;
        self.hp_loss = self.hp_loss.saturating_add(hp_loss);
    }
}

fn projected_monster_from(
    combat: &CombatState,
    monster: &MonsterEntity,
) -> Option<ProjectedMonsterDamage> {
    if !monster.is_alive_for_action() {
        return None;
    }
    let enemy_id = EnemyId::from_id(monster.monster_type)?;
    Some(ProjectedMonsterDamage {
        entity_id: monster.id,
        current_hp: monster.current_hp,
        max_hp: monster.max_hp,
        projected_hp: monster.current_hp,
        projected_block: monster.block,
        hp_loss: 0,
        split_power: store::has_power(combat, monster.id, PowerId::Split),
        large_slime_split_already_triggered: matches!(
            enemy_id,
            EnemyId::AcidSlimeL | EnemyId::SpikeSlimeL
        ) && monster.large_slime.split_triggered,
        planned_move_id: monster.planned_move_id(),
        guardian_open: enemy_id == EnemyId::TheGuardian && monster.guardian.is_open,
        guardian_close_up_triggered: enemy_id == EnemyId::TheGuardian
            && monster.guardian.close_up_triggered,
        guardian_mode_shift_remaining: if enemy_id == EnemyId::TheGuardian
            && store::has_power(combat, monster.id, PowerId::ModeShift)
        {
            Some(store::power_amount(combat, monster.id, PowerId::ModeShift))
        } else {
            None
        },
        lagavulin_sleeping: enemy_id == EnemyId::Lagavulin && !monster.lagavulin.is_out,
    })
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
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::runtime::combat::{CombatCard, Power, PowerPayload};
    use crate::test_support::{blank_test_combat, test_monster};

    fn test_power(power_type: PowerId, amount: i32) -> Power {
        Power {
            power_type,
            instance_id: None,
            amount,
            extra_data: 0,
            payload: PowerPayload::None,
            just_applied: false,
        }
    }

    #[test]
    fn detects_large_slime_split_trigger_from_card_damage() {
        let mut combat = blank_test_combat();
        let mut slime = test_monster(EnemyId::AcidSlimeL);
        slime.id = 11;
        slime.current_hp = 40;
        slime.max_hp = 65;
        combat.entities.monsters = vec![slime];
        combat
            .entities
            .power_db
            .insert(11, vec![test_power(PowerId::Split, -1)]);
        combat.zones.hand = vec![CombatCard::new(CardId::Carnage, 20)];

        let hint = enemy_phase_transition_hint_for_input(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(11),
            },
        );

        assert_eq!(hint.split_trigger_count, 1);
        assert!(hint.split_debt_hp > 0);
        assert!(hint.ordering_risk_score() > 0);
    }

    #[test]
    fn lethal_split_monster_damage_is_not_split_debt() {
        let mut combat = blank_test_combat();
        let mut slime = test_monster(EnemyId::AcidSlimeL);
        slime.id = 11;
        slime.current_hp = 20;
        slime.max_hp = 65;
        combat.entities.monsters = vec![slime];
        combat
            .entities
            .power_db
            .insert(11, vec![test_power(PowerId::Split, -1)]);
        combat.zones.hand = vec![CombatCard::new(CardId::Carnage, 20)];

        let hint = enemy_phase_transition_hint_for_input(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(11),
            },
        );

        assert_eq!(hint.split_trigger_count, 0);
        assert_eq!(hint.split_debt_hp, 0);
    }

    #[test]
    fn detects_slime_boss_split_trigger_from_card_damage() {
        let mut combat = blank_test_combat();
        let mut slime = test_monster(EnemyId::SlimeBoss);
        slime.id = 14;
        slime.current_hp = 80;
        slime.max_hp = 140;
        combat.entities.monsters = vec![slime];
        combat
            .entities
            .power_db
            .insert(14, vec![test_power(PowerId::Split, -1)]);
        combat.zones.hand = vec![CombatCard::new(CardId::Carnage, 20)];

        let hint = enemy_phase_transition_hint_for_input(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(14),
            },
        );

        assert_eq!(hint.split_trigger_count, 1);
        assert!(hint.split_debt_hp > 0);
    }

    #[test]
    fn detects_guardian_mode_shift_trigger_from_card_damage() {
        let mut combat = blank_test_combat();
        let mut guardian = test_monster(EnemyId::TheGuardian);
        guardian.id = 12;
        guardian.current_hp = 100;
        guardian.max_hp = 240;
        guardian.guardian.is_open = true;
        combat.entities.monsters = vec![guardian];
        combat
            .entities
            .power_db
            .insert(12, vec![test_power(PowerId::ModeShift, 5)]);
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 21)];

        let hint = enemy_phase_transition_hint_for_input(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(12),
            },
        );

        assert_eq!(hint.guardian_mode_shift_trigger_count, 1);
        assert_eq!(hint.guardian_min_threshold_remaining_before_hit, Some(5));
    }

    #[test]
    fn detects_lagavulin_wake_risk_from_card_damage() {
        let mut combat = blank_test_combat();
        let mut lagavulin = test_monster(EnemyId::Lagavulin);
        lagavulin.id = 13;
        lagavulin.lagavulin.is_out = false;
        combat.entities.monsters = vec![lagavulin];
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 22)];

        let hint = enemy_phase_transition_hint_for_input(
            &combat,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(13),
            },
        );

        assert_eq!(hint.lagavulin_wake_risk_count, 1);
    }
}
