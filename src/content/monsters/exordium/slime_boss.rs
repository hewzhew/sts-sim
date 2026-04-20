use crate::content::cards::CardId;
use crate::content::monsters::exordium::{
    add_card_action, attack_actions, set_next_move_action, spawn_action, PLAYER,
};
use crate::content::monsters::{EnemyId, MonsterBehavior};
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, AttackSpec, AttackStep, CardDestination, DamageKind, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, SpawnHpSpec, SpawnHpValue,
    SpawnMonsterStep,
};

const SLAM: u8 = 1;
const PREP_SLAM: u8 = 2;
const SPLIT: u8 = 3;
const STICKY: u8 = 4;

pub struct SlimeBoss;

enum SlimeBossTurn<'a> {
    Sticky(&'a AddCardStep),
    PrepSlam,
    Slam(&'a AttackSpec),
    Split(&'a [MoveStep]),
}

fn slam_damage(asc: u8) -> i32 {
    if asc >= 4 {
        38
    } else {
        35
    }
}

fn sticky_count(asc: u8) -> u8 {
    if asc >= 19 {
        5
    } else {
        3
    }
}

fn sticky_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        STICKY,
        MoveStep::AddCard(AddCardStep {
            card_id: CardId::Slimed,
            amount: sticky_count(asc),
            upgraded: false,
            destination: CardDestination::Discard,
            visible_strength: EffectStrength::Strong,
        }),
    )
}

fn prep_slam_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(PREP_SLAM, MoveStep::Magic)
}

fn slam_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        SLAM,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: slam_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

pub(crate) fn split_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        SPLIT,
        smallvec::smallvec![
            MoveStep::Suicide,
            MoveStep::SpawnMonster(SpawnMonsterStep {
                monster_id: EnemyId::SpikeSlimeL,
                logical_position_offset: -1,
                protocol_draw_x_offset: Some(-385),
                hp: SpawnHpSpec {
                    current: SpawnHpValue::SourceCurrentHp,
                    max: SpawnHpValue::SourceCurrentHp,
                },
                is_minion: false,
            }),
            MoveStep::SpawnMonster(SpawnMonsterStep {
                monster_id: EnemyId::AcidSlimeL,
                logical_position_offset: 1,
                protocol_draw_x_offset: Some(120),
                hp: SpawnHpSpec {
                    current: SpawnHpValue::SourceCurrentHp,
                    max: SpawnHpValue::SourceCurrentHp,
                },
                is_minion: false,
            }),
        ],
        MonsterMoveSpec::Unknown,
    )
}

fn plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        STICKY => sticky_plan(asc),
        PREP_SLAM => prep_slam_plan(),
        SLAM => slam_plan(asc),
        SPLIT => split_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> SlimeBossTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (STICKY, [MoveStep::AddCard(add_card)])
            if add_card.card_id == CardId::Slimed
                && add_card.destination == CardDestination::Discard =>
        {
            SlimeBossTurn::Sticky(add_card)
        }
        (PREP_SLAM, [MoveStep::Magic]) => SlimeBossTurn::PrepSlam,
        (
            SLAM,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => SlimeBossTurn::Slam(attack),
        (SPLIT, steps)
            if steps
                .iter()
                .all(|step| matches!(step, MoveStep::Suicide | MoveStep::SpawnMonster(_))) =>
        {
            SlimeBossTurn::Split(steps)
        }
        (_, []) => panic!("slime boss plan missing locked truth"),
        (move_id, steps) => panic!("slime boss plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for SlimeBoss {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        match entity.move_history().back().copied() {
            None => sticky_plan(ascension_level),
            Some(STICKY) => prep_slam_plan(),
            Some(PREP_SLAM) => slam_plan(ascension_level),
            Some(SLAM) => sticky_plan(ascension_level),
            Some(SPLIT) => split_plan(),
            Some(move_id) => plan_for(move_id, ascension_level),
        }
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

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
            power_id: PowerId::Split,
            amount: 1,
        }]
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match decode_turn(plan) {
            SlimeBossTurn::Sticky(add_card) => vec![
                add_card_action(add_card),
                set_next_move_action(entity, prep_slam_plan()),
            ],
            SlimeBossTurn::PrepSlam => {
                vec![set_next_move_action(
                    entity,
                    slam_plan(state.meta.ascension_level),
                )]
            }
            SlimeBossTurn::Slam(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(set_next_move_action(
                    entity,
                    sticky_plan(state.meta.ascension_level),
                ));
                actions
            }
            SlimeBossTurn::Split(steps) => {
                let mut actions = Vec::new();
                for step in steps {
                    match step {
                        MoveStep::Suicide => actions.push(Action::Suicide { target: entity.id }),
                        MoveStep::SpawnMonster(step) => actions.push(spawn_action(entity, step)),
                        other => panic!("slime boss split step unsupported: {:?}", other),
                    }
                }
                actions
            }
        }
    }
}
