use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};
use crate::content::cards::CardId;
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct Taskmaster;

impl MonsterBehavior for Taskmaster {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        // A18+: Gives an ATTACK_DEBUFF intention but the actual debuff includes a Buff to himself.
        (2, Intent::AttackDebuff { damage: 7, hits: 1 })
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();

        let wound_count = if state.meta.ascension_level >= 18 {
            3
        } else if state.meta.ascension_level >= 3 {
            2
        } else {
            1
        };

        if entity.next_move_byte == 2 {
            actions.push(Action::Damage(DamageInfo {
                source: entity.id,
                target: 0,
                base: 7,
                output: 7,
                damage_type: DamageType::Normal,
                is_modified: false,
            }));

            actions.push(Action::MakeTempCardInDiscard {
                card_id: CardId::Wound,
                amount: wound_count,
                upgraded: false,
            });

            if state.meta.ascension_level >= 18 {
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: 1,
                });
            }
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });

        actions
    }
}
