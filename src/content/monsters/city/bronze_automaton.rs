use crate::content::monsters::exordium::{attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, BlockStep, BuffSpec, DamageKind, DefendSpec, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind, SpawnHpSpec,
    SpawnHpValue,
};
use smallvec::smallvec;

pub struct BronzeAutomaton;

const FLAIL: u8 = 1;
const HYPER_BEAM: u8 = 2;
const STUNNED: u8 = 3;
const SPAWN_ORBS: u8 = 4;
const BOOST: u8 = 5;
const LEFT_ORB_OFFSET: i32 = -167;
const RIGHT_ORB_OFFSET: i32 = 166;

fn flail_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        8
    } else {
        7
    }
}

fn beam_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        50
    } else {
        45
    }
}

fn block_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 9 {
        12
    } else {
        9
    }
}

fn strength_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        4
    } else {
        3
    }
}

fn current_runtime(entity: &MonsterEntity) -> (bool, u8) {
    assert!(
        entity.bronze_automaton.protocol_seeded,
        "bronze automaton runtime truth must be protocol-seeded or factory-seeded"
    );
    (
        entity.bronze_automaton.first_turn,
        entity.bronze_automaton.num_turns,
    )
}

fn automaton_runtime_update(
    entity: &MonsterEntity,
    first_turn: Option<bool>,
    num_turns: Option<u8>,
) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::BronzeAutomaton {
            first_turn,
            num_turns,
            protocol_seeded: Some(true),
        },
    }
}

fn flail_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        FLAIL,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: flail_damage(ascension_level),
            hits: 2,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn hyper_beam_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        HYPER_BEAM,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: beam_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn stunned_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(STUNNED, smallvec![], MonsterMoveSpec::Stun)
}

fn spawn_orbs_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(SPAWN_ORBS, smallvec![], MonsterMoveSpec::Unknown)
}

fn boost_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        BOOST,
        smallvec![
            MoveStep::GainBlock(BlockStep {
                target: MoveTarget::SelfTarget,
                amount: block_amount(ascension_level),
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Strength,
                amount: strength_amount(ascension_level),
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::DefendBuff(
            DefendSpec {
                block: block_amount(ascension_level),
            },
            BuffSpec {
                power_id: PowerId::Strength,
                amount: strength_amount(ascension_level),
            },
        ),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        FLAIL => flail_plan(ascension_level),
        HYPER_BEAM => hyper_beam_plan(ascension_level),
        STUNNED => stunned_plan(),
        SPAWN_ORBS => spawn_orbs_plan(),
        BOOST => boost_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn automaton_draw_x(state: &CombatState, entity: &MonsterEntity) -> i32 {
    state
        .monster_protocol_identity(entity.id)
        .and_then(|identity| identity.draw_x)
        .unwrap_or(entity.logical_position)
}

fn spawn_orb_action(draw_x: i32) -> Action {
    Action::SpawnMonsterSmart {
        monster_id: crate::content::monsters::EnemyId::BronzeOrb,
        logical_position: draw_x,
        hp: SpawnHpSpec {
            current: SpawnHpValue::Rolled,
            max: SpawnHpValue::Rolled,
        },
        protocol_draw_x: Some(draw_x),
        is_minion: true,
    }
}

impl MonsterBehavior for BronzeAutomaton {
    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_hp_rng, _ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Artifact,
            amount: 3,
        }]
    }

    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        let (first_turn, num_turns) = current_runtime(entity);
        if first_turn {
            return spawn_orbs_plan();
        }
        if num_turns == 4 {
            return hyper_beam_plan(ascension_level);
        }
        if last_move(entity, HYPER_BEAM) {
            if ascension_level >= 19 {
                return boost_plan(ascension_level);
            }
            return stunned_plan();
        }
        if last_move(entity, STUNNED) || last_move(entity, BOOST) || last_move(entity, SPAWN_ORBS) {
            return flail_plan(ascension_level);
        }
        boost_plan(ascension_level)
    }

    fn on_roll_move(
        _ascension_level: u8,
        entity: &MonsterEntity,
        _num: i32,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let (first_turn, num_turns) = current_runtime(entity);
        let next_first_turn = Some(false);
        let next_num_turns = match plan.move_id {
            HYPER_BEAM => Some(0),
            FLAIL | BOOST if !first_turn && !last_move(entity, HYPER_BEAM) => {
                Some(num_turns.saturating_add(1))
            }
            _ => Some(num_turns),
        };
        vec![automaton_runtime_update(
            entity,
            next_first_turn,
            next_num_turns,
        )]
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match plan.move_id {
            SPAWN_ORBS => {
                let center = automaton_draw_x(state, entity);
                vec![
                    spawn_orb_action(center + LEFT_ORB_OFFSET),
                    spawn_orb_action(center + RIGHT_ORB_OFFSET),
                    Action::RollMonsterMove {
                        monster_id: entity.id,
                    },
                ]
            }
            FLAIL => {
                let mut actions = attack_actions(
                    entity.id,
                    PLAYER,
                    &AttackSpec {
                        base_damage: flail_damage(state.meta.ascension_level),
                        hits: 2,
                        damage_kind: DamageKind::Normal,
                    },
                );
                actions.push(Action::RollMonsterMove {
                    monster_id: entity.id,
                });
                actions
            }
            BOOST => vec![
                Action::GainBlock {
                    target: entity.id,
                    amount: block_amount(state.meta.ascension_level),
                },
                Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: strength_amount(state.meta.ascension_level),
                },
                Action::RollMonsterMove {
                    monster_id: entity.id,
                },
            ],
            HYPER_BEAM => {
                let mut actions = attack_actions(
                    entity.id,
                    PLAYER,
                    &AttackSpec {
                        base_damage: beam_damage(state.meta.ascension_level),
                        hits: 1,
                        damage_kind: DamageKind::Normal,
                    },
                );
                actions.push(Action::RollMonsterMove {
                    monster_id: entity.id,
                });
                actions
            }
            STUNNED => vec![Action::RollMonsterMove {
                monster_id: entity.id,
            }],
            _ => panic!(
                "bronze automaton take_turn received unsupported move {}",
                plan.move_id
            ),
        }
    }

    fn on_death(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        state
            .entities
            .monsters
            .iter()
            .filter(|monster| {
                monster.id != entity.id
                    && !monster.is_dying
                    && !monster.is_escaped
                    && monster.current_hp > 0
            })
            .map(|monster| Action::Suicide { target: monster.id })
            .collect()
    }
}
