use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

pub struct SlimeBoss;

const SPIKE_SLIME_L_SPLIT_OFFSET_X: i32 = -385;
const ACID_SLIME_L_SPLIT_OFFSET_X: i32 = 120;

impl MonsterBehavior for SlimeBoss {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        // Starts with Split power
        vec![
            Action::ApplyPower {
                target: entity.id,
                source: entity.id,
                power_id: PowerId::Split,
                amount: -1, // Java sentinel amount
            }, // we omit the music/unlock actions for the simulator
        ]
    }

    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let slam_dmg = if ascension_level >= 4 { 38 } else { 35 };

        if entity.move_history.is_empty() {
            return (4, Intent::StrongDebuff); // STICKY
        }

        let last_move = *entity.move_history.back().unwrap();
        match last_move {
            4 | 3 => (2, Intent::Unknown), // After STICKY or SPLIT
            2 => (
                1,
                Intent::Attack {
                    damage: slam_dmg,
                    hits: 1,
                },
            ), // After PREP
            1 => (4, Intent::StrongDebuff), // After SLAM
            _ => (4, Intent::StrongDebuff),
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let slam_dmg = if state.meta.ascension_level >= 4 {
            38
        } else {
            35
        };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            4 => {
                // STICKY
                actions.push(Action::MakeTempCardInDiscard {
                    card_id: crate::content::cards::CardId::Slimed,
                    amount: if state.meta.ascension_level >= 19 {
                        5
                    } else {
                        3
                    },
                    upgraded: false,
                });
            }
            2 => { // PREP_SLAM
                 // Just shouts, no logical effect in simulator other than intent.
            }
            1 => {
                // SLAM
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: slam_dmg,
                    output: slam_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            3 => {
                // SPLIT
                // Java order: SuicideAction (Boss dies), THEN SpawnMonster actions.
                // Java uses SpawnMonsterAction(m, false) → useSmartPositioning=true → drawX sort.
                // SpikeSlime_L(drawX=-385) → leftmost → before dead Boss(drawX=0).
                // AcidSlime_L(drawX=120)   → rightmost → after dead Boss.
                let base_draw_x = entity
                    .protocol_identity
                    .draw_x
                    .unwrap_or(entity.logical_position);
                let spike_draw_x = base_draw_x + SPIKE_SLIME_L_SPLIT_OFFSET_X;
                let acid_draw_x = base_draw_x + ACID_SLIME_L_SPLIT_OFFSET_X;

                // 1. Boss suicides first
                actions.push(Action::Suicide { target: entity.id });
                // 2. Spawn SpikeSlime_L on the left
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: crate::content::monsters::EnemyId::SpikeSlimeL,
                    logical_position: spike_draw_x,
                    current_hp: entity.current_hp,
                    max_hp: entity.current_hp,
                    protocol_draw_x: Some(spike_draw_x),
                    is_minion: false,
                });
                // 3. Spawn AcidSlime_L on the right
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: crate::content::monsters::EnemyId::AcidSlimeL,
                    logical_position: acid_draw_x,
                    current_hp: entity.current_hp,
                    max_hp: entity.current_hp,
                    protocol_draw_x: Some(acid_draw_x),
                    is_minion: false,
                });
                // Don't roll next move — Boss is dead
                return actions;
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}

