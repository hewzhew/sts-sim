use super::{attack_actions, gain_block_action, set_next_move_action, steal_gold_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::rewards::state::RewardItem;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, AttackStep, BlockStep, DamageKind, MonsterMoveSpec, MonsterTurnPlan, MoveStep,
    MoveTarget, StealGoldStep,
};

const MUG: u8 = 1;
const SMOKE_BOMB: u8 = 2;
const ESCAPE: u8 = 3;
const LUNGE: u8 = 4;
const ESCAPE_BLOCK: i32 = 6;

pub struct Looter;

enum LooterTurn<'a> {
    Mug(&'a StealGoldStep, &'a AttackSpec),
    SmokeBomb(&'a BlockStep),
    Escape,
    Lunge(&'a StealGoldStep, &'a AttackSpec),
}

fn gold_amount(asc: u8) -> i32 {
    if asc >= 17 {
        20
    } else {
        15
    }
}

fn swipe_damage(asc: u8) -> i32 {
    if asc >= 2 {
        11
    } else {
        10
    }
}

fn lunge_damage(asc: u8) -> i32 {
    if asc >= 2 {
        14
    } else {
        12
    }
}

fn mug_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        MUG,
        smallvec::smallvec![
            MoveStep::StealGold(StealGoldStep {
                amount: gold_amount(asc),
            }),
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: swipe_damage(asc),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
        ],
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: swipe_damage(asc),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn lunge_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        LUNGE,
        smallvec::smallvec![
            MoveStep::StealGold(StealGoldStep {
                amount: gold_amount(asc),
            }),
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: lunge_damage(asc),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
        ],
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: lunge_damage(asc),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn smoke_bomb_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        SMOKE_BOMB,
        MoveStep::GainBlock(BlockStep {
            target: MoveTarget::SelfTarget,
            amount: ESCAPE_BLOCK,
        }),
    )
}

fn escape_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(ESCAPE, MoveStep::Escape)
}

fn plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        MUG => mug_plan(asc),
        SMOKE_BOMB => smoke_bomb_plan(),
        ESCAPE => escape_plan(),
        LUNGE => lunge_plan(asc),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn current_slash_count(entity: &MonsterEntity) -> u8 {
    assert!(
        entity.thief.protocol_seeded,
        "thief runtime truth must be protocol-seeded or factory-seeded"
    );
    entity.thief.slash_count
}

fn current_stolen_gold(entity: &MonsterEntity) -> i32 {
    assert!(
        entity.thief.protocol_seeded,
        "thief runtime truth must be protocol-seeded or factory-seeded"
    );
    entity.thief.stolen_gold
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> LooterTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            MUG,
            [MoveStep::StealGold(steal), MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => LooterTurn::Mug(steal, attack),
        (SMOKE_BOMB, [MoveStep::GainBlock(block)]) if block.target == MoveTarget::SelfTarget => {
            LooterTurn::SmokeBomb(block)
        }
        (ESCAPE, [MoveStep::Escape]) => LooterTurn::Escape,
        (
            LUNGE,
            [MoveStep::StealGold(steal), MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => LooterTurn::Lunge(steal, attack),
        (_, []) => panic!("looter plan missing locked truth"),
        (move_id, steps) => panic!("looter plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for Looter {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        mug_plan(ascension_level)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_hp_rng, ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Thievery,
            amount: gold_amount(ascension_level),
        }]
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match decode_turn(plan) {
            LooterTurn::Mug(steal, attack) => {
                let next_slash_count = current_slash_count(entity).saturating_add(1);
                let mut actions = vec![steal_gold_action(entity, steal)];
                actions.extend(attack_actions(entity.id, PLAYER, attack));
                let next_plan = if next_slash_count == 2 {
                    if state.rng.ai_rng.random_boolean() {
                        smoke_bomb_plan()
                    } else {
                        lunge_plan(state.meta.ascension_level)
                    }
                } else {
                    mug_plan(state.meta.ascension_level)
                };
                actions.push(set_next_move_action(entity, next_plan));
                actions
            }
            LooterTurn::SmokeBomb(block) => vec![
                gain_block_action(entity, block),
                set_next_move_action(entity, escape_plan()),
            ],
            LooterTurn::Escape => vec![Action::Escape { target: entity.id }],
            LooterTurn::Lunge(steal, attack) => {
                let mut actions = vec![steal_gold_action(entity, steal)];
                actions.extend(attack_actions(entity.id, PLAYER, attack));
                actions.push(set_next_move_action(entity, smoke_bomb_plan()));
                actions
            }
        }
    }

    fn on_death(_state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let stolen_gold = current_stolen_gold(entity);
        if stolen_gold <= 0 {
            Vec::new()
        } else {
            vec![Action::AddCombatReward {
                item: RewardItem::StolenGold {
                    amount: stolen_gold,
                },
            }]
        }
    }
}
