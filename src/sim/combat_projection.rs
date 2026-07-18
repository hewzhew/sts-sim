use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::runtime::monster_move::{DamageKind, EffectStrength, MonsterMoveSpec, MonsterTurnPlan};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VisibleIntentKind {
    Attack,
    AttackBuff,
    AttackDebuff,
    AttackDefend,
    Buff,
    Debuff,
    StrongDebuff,
    Defend,
    DefendDebuff,
    DefendBuff,
    Escape,
    Magic,
    Sleep,
    Stun,
    Debug,
    None,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MonsterMovePreview {
    pub damage_per_hit: Option<i32>,
    pub hits: u8,
    pub total_damage: Option<i32>,
    pub visible_intent: VisibleIntentKind,
}

impl MonsterMovePreview {
    pub fn from_plan(plan: &MonsterTurnPlan, damage_per_hit: Option<i32>) -> Self {
        let hits = plan.attack().map(|attack| attack.hits.max(1)).unwrap_or(0);
        let total_damage = damage_per_hit.map(|damage| damage.saturating_mul(hits as i32));

        Self {
            damage_per_hit,
            hits,
            total_damage,
            visible_intent: visible_intent_kind(plan),
        }
    }
}

pub fn project_monster_move_preview(monster: &MonsterEntity) -> MonsterMovePreview {
    let plan = monster.turn_plan();
    MonsterMovePreview::from_plan(&plan, plan.attack().map(|attack| attack.base_damage.max(0)))
}

pub fn project_monster_move_preview_in_combat(
    combat: &CombatState,
    monster: &MonsterEntity,
) -> MonsterMovePreview {
    let plan = crate::content::monsters::resolve_monster_turn_plan(combat, monster);
    project_monster_move_preview_from_plan(combat, monster, &plan)
}

pub fn project_monster_move_preview_from_plan(
    combat: &CombatState,
    monster: &MonsterEntity,
    plan: &MonsterTurnPlan,
) -> MonsterMovePreview {
    let protocol_damage =
        if plan.attack().is_some() && combat.monster_has_protocol_visible_intent(monster.id) {
            let observed = combat
                .monster_protocol_preview_damage_per_hit(monster.id)
                .max(0);
            (observed > 0).then_some(observed)
        } else {
            None
        };
    let semantic_damage = plan.attack().map(|attack| {
        if attack.damage_kind == DamageKind::Normal {
            crate::content::powers::calculate_monster_damage(
                attack.base_damage,
                monster.id,
                combat.entities.player.id,
                combat,
            )
        } else {
            attack.base_damage.max(0)
        }
    });
    let damage_per_hit = protocol_damage.or(semantic_damage);
    MonsterMovePreview::from_plan(plan, damage_per_hit)
}

pub fn monster_has_visible_attack(monster: &MonsterEntity) -> bool {
    project_monster_move_preview(monster)
        .damage_per_hit
        .is_some()
}

pub fn monster_has_visible_attack_in_combat(combat: &CombatState, monster: &MonsterEntity) -> bool {
    project_monster_move_preview_in_combat(combat, monster)
        .damage_per_hit
        .is_some()
}

pub fn monster_preview_total_damage(monster: &MonsterEntity) -> i32 {
    project_monster_move_preview(monster)
        .total_damage
        .unwrap_or(0)
}

pub fn monster_preview_total_damage_in_combat(
    combat: &CombatState,
    monster: &MonsterEntity,
) -> i32 {
    project_monster_move_preview_in_combat(combat, monster)
        .total_damage
        .unwrap_or(0)
}

fn visible_intent_kind(plan: &MonsterTurnPlan) -> VisibleIntentKind {
    match plan.summary_spec() {
        MonsterMoveSpec::Attack(_) => VisibleIntentKind::Attack,
        MonsterMoveSpec::AttackAddCard(_, _) => VisibleIntentKind::AttackDebuff,
        MonsterMoveSpec::AttackUpgradeCards(_, _) => VisibleIntentKind::AttackDebuff,
        MonsterMoveSpec::AttackBuff(_, _) => VisibleIntentKind::AttackBuff,
        MonsterMoveSpec::AttackSustain(_) => VisibleIntentKind::AttackBuff,
        MonsterMoveSpec::AttackDebuff(_, _) => VisibleIntentKind::AttackDebuff,
        MonsterMoveSpec::AttackDefend(_, _) => VisibleIntentKind::AttackDefend,
        MonsterMoveSpec::AddCard(add_card) => match add_card.visible_strength {
            EffectStrength::Strong => VisibleIntentKind::StrongDebuff,
            EffectStrength::Normal => VisibleIntentKind::Debuff,
        },
        MonsterMoveSpec::Buff(_) => VisibleIntentKind::Buff,
        MonsterMoveSpec::Heal(_) => VisibleIntentKind::Buff,
        MonsterMoveSpec::Debuff(_) => VisibleIntentKind::Debuff,
        MonsterMoveSpec::StrongDebuff(_) => VisibleIntentKind::StrongDebuff,
        MonsterMoveSpec::Defend(_) => VisibleIntentKind::Defend,
        MonsterMoveSpec::DefendDebuff(_, _) => VisibleIntentKind::DefendDebuff,
        MonsterMoveSpec::DefendBuff(_, _) => VisibleIntentKind::DefendBuff,
        MonsterMoveSpec::Escape => VisibleIntentKind::Escape,
        MonsterMoveSpec::Magic => VisibleIntentKind::Magic,
        MonsterMoveSpec::Sleep => VisibleIntentKind::Sleep,
        MonsterMoveSpec::Stun => VisibleIntentKind::Stun,
        MonsterMoveSpec::Debug => VisibleIntentKind::Debug,
        MonsterMoveSpec::None => VisibleIntentKind::None,
        MonsterMoveSpec::Unknown => VisibleIntentKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::content::powers::{store, PowerId};
    use crate::runtime::combat::{Power, PowerPayload};
    use crate::test_support::{blank_test_combat, planned_monster};

    fn power(power_type: PowerId, amount: i32) -> Power {
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
    fn semantic_monster_preview_uses_the_execution_damage_pipeline() {
        let mut combat = blank_test_combat();
        let monster = planned_monster(EnemyId::TimeEater, 2);
        let monster_id = monster.id;
        combat.entities.monsters = vec![monster.clone()];
        store::set_powers_for(&mut combat, monster_id, vec![power(PowerId::Strength, 5)]);
        let player_id = combat.entities.player.id;
        store::set_powers_for(&mut combat, player_id, vec![power(PowerId::Vulnerable, 1)]);

        let plan = crate::content::monsters::resolve_monster_turn_plan(&combat, &monster);
        let attack = plan.attack().expect("test move should attack");
        let expected = crate::content::powers::calculate_monster_damage(
            attack.base_damage,
            monster_id,
            player_id,
            &combat,
        );
        let preview = project_monster_move_preview_in_combat(&combat, &monster);

        assert!(expected > attack.base_damage);
        assert_eq!(preview.damage_per_hit, Some(expected));
        assert_eq!(
            preview.total_damage,
            Some(expected.saturating_mul(attack.hits.max(1) as i32))
        );
    }
}
