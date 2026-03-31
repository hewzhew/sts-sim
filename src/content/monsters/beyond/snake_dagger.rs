use crate::action::{Action, DamageType, DamageInfo};
use crate::combat::{CombatState, Intent};
use crate::content::cards::CardId;
use crate::content::monsters::MonsterBehavior;

pub struct SnakeDagger;

impl MonsterBehavior for SnakeDagger {
    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &crate::combat::MonsterEntity, _ascension_level: u8, _num: i32) -> (u8, Intent) {
        if entity.move_history.is_empty() {
             return (1, Intent::AttackDebuff { damage: 9, hits: 1 });
        }
        (2, Intent::Attack { damage: 25, hits: 1 })
    }

    fn use_pre_battle_action(entity: &crate::combat::MonsterEntity, _hp_rng: &mut crate::rng::StsRng, _ascension_level: u8) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: crate::content::powers::PowerId::Minion,
            amount: 1,
        }]
    }

    fn take_turn(_state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => { // WOUND
                 actions.push(Action::Damage(DamageInfo {
                     source: entity.id,
                     target: 0,
                     base: 9,
                     output: 9,
                     damage_type: DamageType::Normal,
                     is_modified: false,
                 }));
                 actions.push(Action::MakeTempCardInDiscard { card_id: CardId::Wound, amount: 1 , upgraded: false });
            },
            2 => { // EXPLODE
                 actions.push(Action::Damage(DamageInfo {
                     source: entity.id,
                     target: 0,
                     base: 25,
                     output: 25,
                     damage_type: DamageType::Normal,
                     is_modified: false,
                 }));
                 actions.push(Action::Suicide { target: entity.id });
            },
            _ => {}
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
