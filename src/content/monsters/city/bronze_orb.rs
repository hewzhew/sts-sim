use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{DefendSpec, MonsterMoveSpec, MonsterTurnPlan};

pub struct BronzeOrb;

const BEAM: u8 = 1;
const SUPPORT_BEAM: u8 = 2;
const STASIS: u8 = 3;
const BLOCK_AMOUNT: i32 = 12;
const BEAM_DAMAGE: i32 = 8;

fn current_used_stasis(entity: &MonsterEntity) -> bool {
    assert!(
        entity.bronze_orb.protocol_seeded,
        "bronze orb runtime truth must be protocol-seeded or factory-seeded"
    );
    entity.bronze_orb.used_stasis
}

fn orb_runtime_update(entity: &MonsterEntity, used_stasis: Option<bool>) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::BronzeOrb {
            used_stasis,
            protocol_seeded: Some(true),
        },
    }
}

fn beam_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        BEAM,
        MonsterMoveSpec::Attack(crate::semantics::combat::AttackSpec {
            base_damage: BEAM_DAMAGE,
            hits: 1,
            damage_kind: crate::semantics::combat::DamageKind::Normal,
        }),
    )
}

fn support_beam_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        SUPPORT_BEAM,
        smallvec::smallvec![],
        MonsterMoveSpec::Defend(DefendSpec {
            block: BLOCK_AMOUNT,
        }),
    )
}

fn stasis_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        STASIS,
        smallvec::smallvec![],
        MonsterMoveSpec::StrongDebuff(crate::semantics::combat::DebuffSpec {
            power_id: crate::content::powers::PowerId::Stasis,
            amount: 1,
            strength: crate::semantics::combat::EffectStrength::Strong,
        }),
    )
}

fn plan_for(move_id: u8) -> MonsterTurnPlan {
    match move_id {
        BEAM => beam_plan(),
        SUPPORT_BEAM => support_beam_plan(),
        STASIS => stasis_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    let mut history = entity.move_history().iter().rev();
    matches!(
        (history.next().copied(), history.next().copied()),
        (Some(a), Some(b)) if a == move_id && b == move_id
    )
}

fn living_automaton_id(state: &CombatState) -> Option<usize> {
    state.entities.monsters.iter().find_map(|monster| {
        (monster.monster_type == crate::content::monsters::EnemyId::BronzeAutomaton as usize
            && !monster.is_dying
            && !monster.is_escaped
            && monster.current_hp > 0)
            .then_some(monster.id)
    })
}

impl MonsterBehavior for BronzeOrb {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        _ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if !current_used_stasis(entity) && num >= 25 {
            return stasis_plan();
        }
        if num >= 70 && !last_two_moves(entity, SUPPORT_BEAM) {
            return support_beam_plan();
        }
        if !last_two_moves(entity, BEAM) {
            return beam_plan();
        }
        support_beam_plan()
    }

    fn on_roll_move(
        _ascension_level: u8,
        entity: &MonsterEntity,
        _num: i32,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        if plan.move_id == STASIS {
            vec![orb_runtime_update(entity, Some(true))]
        } else {
            Vec::new()
        }
    }

    fn turn_plan(_state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id())
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match plan.move_id {
            BEAM => vec![Action::MonsterAttack {
                source: entity.id,
                target: 0,
                base_damage: BEAM_DAMAGE,
                damage_kind: crate::semantics::combat::DamageKind::Normal,
            }],
            SUPPORT_BEAM => living_automaton_id(state)
                .map(|target| {
                    vec![Action::GainBlock {
                        target,
                        amount: BLOCK_AMOUNT,
                    }]
                })
                .unwrap_or_default(),
            STASIS => vec![Action::ApplyStasis {
                target_id: entity.id,
            }],
            _ => panic!(
                "bronze orb take_turn received unsupported move {}",
                plan.move_id
            ),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
