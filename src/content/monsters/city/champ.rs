use crate::content::monsters::exordium::{attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, BlockStep, BuffSpec, DamageKind, DebuffSpec,
    DefendSpec, EffectStrength, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget,
    PowerEffectKind,
};
use smallvec::smallvec;

pub struct Champ;

const HEAVY_SLASH: u8 = 1;
const DEFENSIVE_STANCE: u8 = 2;
const EXECUTE: u8 = 3;
const FACE_SLAP: u8 = 4;
const GLOAT: u8 = 5;
const TAUNT: u8 = 6;
const ANGER: u8 = 7;

fn slash_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        18
    } else {
        16
    }
}

fn slap_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        14
    } else {
        12
    }
}

fn execute_damage() -> i32 {
    10
}

fn block_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 19 {
        20
    } else if ascension_level >= 9 {
        18
    } else {
        15
    }
}

fn forge_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 19 {
        7
    } else if ascension_level >= 9 {
        6
    } else {
        5
    }
}

fn strength_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 19 {
        4
    } else if ascension_level >= 4 {
        3
    } else {
        2
    }
}

fn current_runtime(entity: &MonsterEntity) -> (bool, u8, u8, bool) {
    assert!(
        entity.champ.protocol_seeded,
        "champ runtime truth must be protocol-seeded or factory-seeded"
    );
    (
        entity.champ.first_turn,
        entity.champ.num_turns,
        entity.champ.forge_times,
        entity.champ.threshold_reached,
    )
}

fn champ_runtime_update(
    entity: &MonsterEntity,
    first_turn: Option<bool>,
    num_turns: Option<u8>,
    forge_times: Option<u8>,
    threshold_reached: Option<bool>,
) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::Champ {
            first_turn,
            num_turns,
            forge_times,
            threshold_reached,
            protocol_seeded: Some(true),
        },
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn last_move_before(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().len() >= 2
        && entity.move_history()[entity.move_history().len() - 2] == move_id
}

fn heavy_slash_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        HEAVY_SLASH,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: slash_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn defensive_stance_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        DEFENSIVE_STANCE,
        smallvec![
            MoveStep::GainBlock(BlockStep {
                target: MoveTarget::SelfTarget,
                amount: block_amount(ascension_level),
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Metallicize,
                amount: forge_amount(ascension_level),
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::DefendBuff(
            DefendSpec {
                block: block_amount(ascension_level),
            },
            BuffSpec {
                power_id: PowerId::Metallicize,
                amount: forge_amount(ascension_level),
            },
        ),
    )
}

fn execute_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        EXECUTE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: execute_damage(),
            hits: 2,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn face_slap_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        FACE_SLAP,
        smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: slap_damage(ascension_level),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Frail,
                amount: 2,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Vulnerable,
                amount: 2,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::AttackDebuff(
            AttackSpec {
                base_damage: slap_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DebuffSpec {
                power_id: PowerId::Frail,
                amount: 2,
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn gloat_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        GLOAT,
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Strength,
            amount: strength_amount(ascension_level),
        }),
    )
}

fn taunt_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        TAUNT,
        smallvec![
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                amount: 2,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Vulnerable,
                amount: 2,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::Debuff(DebuffSpec {
            power_id: PowerId::Weak,
            amount: 2,
            strength: EffectStrength::Normal,
        }),
    )
}

fn anger_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        ANGER,
        smallvec![MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::SelfTarget,
            power_id: PowerId::Strength,
            amount: strength_amount(ascension_level) * 3,
            effect: PowerEffectKind::Buff,
            visible_strength: EffectStrength::Strong,
        }),],
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Strength,
            amount: strength_amount(ascension_level) * 3,
        }),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        HEAVY_SLASH => heavy_slash_plan(ascension_level),
        DEFENSIVE_STANCE => defensive_stance_plan(ascension_level),
        EXECUTE => execute_plan(),
        FACE_SLAP => face_slap_plan(ascension_level),
        GLOAT => gloat_plan(ascension_level),
        TAUNT => taunt_plan(),
        ANGER => anger_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for Champ {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        let (_, num_turns, forge_times, threshold_reached) = current_runtime(entity);

        if entity.current_hp < entity.max_hp / 2 && !threshold_reached {
            return anger_plan(ascension_level);
        }

        if threshold_reached && !last_move(entity, EXECUTE) && !last_move_before(entity, EXECUTE) {
            return execute_plan();
        }

        if num_turns.saturating_add(1) == 4 && !threshold_reached {
            return taunt_plan();
        }

        let forge_roll_cap = if ascension_level >= 19 { 30 } else { 15 };
        if !last_move(entity, DEFENSIVE_STANCE) && forge_times < 2 && num <= forge_roll_cap {
            return defensive_stance_plan(ascension_level);
        }

        if !last_move(entity, GLOAT) && !last_move(entity, DEFENSIVE_STANCE) && num <= 30 {
            return gloat_plan(ascension_level);
        }

        if !last_move(entity, FACE_SLAP) && num <= 55 {
            return face_slap_plan(ascension_level);
        }

        if !last_move(entity, HEAVY_SLASH) {
            heavy_slash_plan(ascension_level)
        } else {
            face_slap_plan(ascension_level)
        }
    }

    fn on_roll_move(
        _ascension_level: u8,
        entity: &MonsterEntity,
        _num: i32,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let (_, num_turns, forge_times, threshold_reached) = current_runtime(entity);
        let crossed_threshold = entity.current_hp < entity.max_hp / 2 && !threshold_reached;
        let next_threshold_reached = threshold_reached || crossed_threshold;
        let mut next_num_turns = num_turns.saturating_add(1);
        let mut next_forge_times = forge_times;

        if plan.move_id == DEFENSIVE_STANCE {
            next_forge_times = next_forge_times.saturating_add(1);
        }
        if plan.move_id == TAUNT {
            next_num_turns = 0;
        }

        vec![champ_runtime_update(
            entity,
            None,
            Some(next_num_turns),
            Some(next_forge_times),
            Some(next_threshold_reached),
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
        let (first_turn, _, _, _) = current_runtime(entity);
        let mut actions = Vec::new();
        if first_turn {
            actions.push(champ_runtime_update(entity, Some(false), None, None, None));
        }

        match plan.move_id {
            HEAVY_SLASH => actions.extend(attack_actions(
                entity.id,
                PLAYER,
                &AttackSpec {
                    base_damage: slash_damage(state.meta.ascension_level),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            )),
            DEFENSIVE_STANCE => {
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: block_amount(state.meta.ascension_level),
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Metallicize,
                    amount: forge_amount(state.meta.ascension_level),
                });
            }
            EXECUTE => actions.extend(attack_actions(
                entity.id,
                PLAYER,
                &AttackSpec {
                    base_damage: execute_damage(),
                    hits: 2,
                    damage_kind: DamageKind::Normal,
                },
            )),
            FACE_SLAP => {
                actions.extend(attack_actions(
                    entity.id,
                    PLAYER,
                    &AttackSpec {
                        base_damage: slap_damage(state.meta.ascension_level),
                        hits: 1,
                        damage_kind: DamageKind::Normal,
                    },
                ));
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: PLAYER,
                    power_id: PowerId::Frail,
                    amount: 2,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: PLAYER,
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                });
            }
            GLOAT => actions.push(Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Strength,
                amount: strength_amount(state.meta.ascension_level),
            }),
            TAUNT => {
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: PLAYER,
                    power_id: PowerId::Weak,
                    amount: 2,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: PLAYER,
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                });
            }
            ANGER => {
                actions.push(Action::RemoveAllDebuffs { target: entity.id });
                actions.push(Action::RemovePower {
                    target: entity.id,
                    power_id: PowerId::Shackled,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: strength_amount(state.meta.ascension_level) * 3,
                });
            }
            _ => panic!("champ take_turn received unsupported move {}", plan.move_id),
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
