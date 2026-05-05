use super::{
    apply_power_action, attack_actions, gain_block_action, remove_power_action,
    set_next_move_action, PLAYER,
};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, ActionInfo, AddTo, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, GuardianRuntimeState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, BlockStep, DamageKind, DebuffSpec, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind, RemovePowerStep,
};

const CLOSE_UP: u8 = 1;
const FIERCE_BASH: u8 = 2;
const ROLL_ATTACK: u8 = 3;
const TWIN_SLAM: u8 = 4;
const WHIRLWIND: u8 = 5;
const CHARGE_UP: u8 = 6;
const VENT_STEAM: u8 = 7;

const THRESHOLD_INCREASE: i32 = 10;
const DEFENSIVE_BLOCK: i32 = 20;
const CHARGE_UP_BLOCK: i32 = 9;
const WHIRLWIND_DAMAGE: i32 = 5;
const WHIRLWIND_HITS: u8 = 4;
const TWIN_SLAM_DAMAGE: i32 = 8;
const TWIN_SLAM_HITS: u8 = 2;
const VENT_DEBUFF: i32 = 2;

pub struct TheGuardian;

enum GuardianTurn<'a> {
    ChargeUp(&'a BlockStep),
    FierceBash(&'a AttackSpec),
    VentSteam {
        weak: &'a ApplyPowerStep,
        vulnerable: &'a ApplyPowerStep,
    },
    Whirlwind(&'a AttackSpec),
    CloseUp(&'a ApplyPowerStep),
    RollAttack(&'a AttackSpec),
    TwinSlam {
        mode_shift: &'a ApplyPowerStep,
        attack: &'a AttackSpec,
        sharp_hide: &'a RemovePowerStep,
    },
}

fn initial_threshold(asc: u8) -> i32 {
    if asc >= 19 {
        40
    } else if asc >= 9 {
        35
    } else {
        30
    }
}

fn fierce_bash_damage(asc: u8) -> i32 {
    if asc >= 4 {
        36
    } else {
        32
    }
}

fn roll_damage(asc: u8) -> i32 {
    if asc >= 4 {
        10
    } else {
        9
    }
}

fn sharp_hide_damage(asc: u8) -> i32 {
    if asc >= 19 {
        4
    } else {
        3
    }
}

fn attack_plan(move_id: u8, damage: i32, hits: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        move_id,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: damage,
                hits,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

fn close_up_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        CLOSE_UP,
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::SelfTarget,
            power_id: PowerId::SharpHide,
            amount: sharp_hide_damage(asc),
            effect: PowerEffectKind::Buff,
            visible_strength: EffectStrength::Normal,
        }),
    )
}

fn fierce_bash_plan(asc: u8) -> MonsterTurnPlan {
    attack_plan(FIERCE_BASH, fierce_bash_damage(asc), 1)
}

fn roll_attack_plan(asc: u8) -> MonsterTurnPlan {
    attack_plan(ROLL_ATTACK, roll_damage(asc), 1)
}

fn whirlwind_plan() -> MonsterTurnPlan {
    attack_plan(WHIRLWIND, WHIRLWIND_DAMAGE, WHIRLWIND_HITS)
}

fn charge_up_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        CHARGE_UP,
        MoveStep::GainBlock(BlockStep {
            target: MoveTarget::SelfTarget,
            amount: CHARGE_UP_BLOCK,
        }),
    )
}

fn vent_steam_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        VENT_STEAM,
        smallvec::smallvec![
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                amount: VENT_DEBUFF,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Vulnerable,
                amount: VENT_DEBUFF,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::StrongDebuff(DebuffSpec {
            power_id: PowerId::Weak,
            amount: VENT_DEBUFF,
            strength: EffectStrength::Strong,
        }),
    )
}

fn twin_slam_plan(mode_shift_amount: i32) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        TWIN_SLAM,
        smallvec::smallvec![
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::ModeShift,
                amount: mode_shift_amount,
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            }),
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: TWIN_SLAM_DAMAGE,
                    hits: TWIN_SLAM_HITS,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::RemovePower(RemovePowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::SharpHide,
            }),
        ],
        MonsterMoveSpec::AttackBuff(
            AttackSpec {
                base_damage: TWIN_SLAM_DAMAGE,
                hits: TWIN_SLAM_HITS,
                damage_kind: DamageKind::Normal,
            },
            crate::semantics::combat::BuffSpec {
                power_id: PowerId::ModeShift,
                amount: mode_shift_amount,
            },
        ),
    )
}

fn plan_for(move_id: u8, asc: u8, mode_shift_amount: i32) -> MonsterTurnPlan {
    match move_id {
        CLOSE_UP => close_up_plan(asc),
        FIERCE_BASH => fierce_bash_plan(asc),
        ROLL_ATTACK => roll_attack_plan(asc),
        TWIN_SLAM => twin_slam_plan(mode_shift_amount),
        WHIRLWIND => whirlwind_plan(),
        CHARGE_UP => charge_up_plan(),
        VENT_STEAM => vent_steam_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> GuardianTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (CHARGE_UP, [MoveStep::GainBlock(block)]) if block.target == MoveTarget::SelfTarget => {
            GuardianTurn::ChargeUp(block)
        }
        (
            FIERCE_BASH,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => GuardianTurn::FierceBash(attack),
        (VENT_STEAM, [MoveStep::ApplyPower(weak), MoveStep::ApplyPower(vulnerable)])
            if weak.target == MoveTarget::Player
                && weak.power_id == PowerId::Weak
                && weak.effect == PowerEffectKind::Debuff
                && vulnerable.target == MoveTarget::Player
                && vulnerable.power_id == PowerId::Vulnerable
                && vulnerable.effect == PowerEffectKind::Debuff =>
        {
            GuardianTurn::VentSteam { weak, vulnerable }
        }
        (
            WHIRLWIND,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => GuardianTurn::Whirlwind(attack),
        (CLOSE_UP, [MoveStep::ApplyPower(sharp_hide)])
            if sharp_hide.target == MoveTarget::SelfTarget
                && sharp_hide.power_id == PowerId::SharpHide
                && sharp_hide.effect == PowerEffectKind::Buff =>
        {
            GuardianTurn::CloseUp(sharp_hide)
        }
        (
            ROLL_ATTACK,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => GuardianTurn::RollAttack(attack),
        (
            TWIN_SLAM,
            [MoveStep::ApplyPower(mode_shift), MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }), MoveStep::RemovePower(sharp_hide)],
        ) if mode_shift.target == MoveTarget::SelfTarget
            && mode_shift.power_id == PowerId::ModeShift
            && mode_shift.effect == PowerEffectKind::Buff
            && sharp_hide.target == MoveTarget::SelfTarget
            && sharp_hide.power_id == PowerId::SharpHide =>
        {
            GuardianTurn::TwinSlam {
                mode_shift,
                attack,
                sharp_hide,
            }
        }
        (_, []) => panic!("guardian plan missing locked truth"),
        (move_id, steps) => panic!("guardian plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

pub fn initialize_runtime_state(monster: &mut MonsterEntity, ascension_level: u8) {
    monster.guardian = GuardianRuntimeState {
        damage_threshold: initial_threshold(ascension_level),
        damage_taken: 0,
        is_open: true,
        close_up_triggered: false,
    };
}

fn guardian_runtime_update(
    entity: &MonsterEntity,
    damage_threshold: Option<i32>,
    damage_taken: Option<i32>,
    is_open: Option<bool>,
    close_up_triggered: Option<bool>,
) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::Guardian {
            damage_threshold,
            damage_taken,
            is_open,
            close_up_triggered,
        },
    }
}

impl MonsterBehavior for TheGuardian {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        if entity.guardian.is_open {
            charge_up_plan()
        } else {
            roll_attack_plan(ascension_level)
        }
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(
            entity.planned_move_id(),
            state.meta.ascension_level,
            entity.guardian.damage_threshold,
        )
    }

    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_hp_rng, ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        let threshold = initial_threshold(ascension_level);
        vec![
            guardian_runtime_update(entity, Some(threshold), Some(0), Some(true), Some(false)),
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::ModeShift,
                amount: threshold,
            },
        ]
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match decode_turn(plan) {
            GuardianTurn::ChargeUp(block) => vec![
                gain_block_action(entity, block),
                set_next_move_action(entity, fierce_bash_plan(state.meta.ascension_level)),
            ],
            GuardianTurn::FierceBash(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(set_next_move_action(entity, vent_steam_plan()));
                actions
            }
            GuardianTurn::VentSteam { weak, vulnerable } => vec![
                apply_power_action(entity, weak),
                apply_power_action(entity, vulnerable),
                set_next_move_action(entity, whirlwind_plan()),
            ],
            GuardianTurn::Whirlwind(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(set_next_move_action(entity, charge_up_plan()));
                actions
            }
            GuardianTurn::CloseUp(sharp_hide) => vec![
                apply_power_action(entity, sharp_hide),
                set_next_move_action(entity, roll_attack_plan(state.meta.ascension_level)),
            ],
            GuardianTurn::RollAttack(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(set_next_move_action(
                    entity,
                    twin_slam_plan(entity.guardian.damage_threshold),
                ));
                actions
            }
            GuardianTurn::TwinSlam {
                mode_shift,
                attack,
                sharp_hide,
            } => {
                let mut actions = vec![
                    guardian_runtime_update(entity, None, Some(0), Some(true), Some(false)),
                    apply_power_action(entity, mode_shift),
                ];
                if entity.block > 0 {
                    actions.push(Action::LoseBlock {
                        target: entity.id,
                        amount: entity.block,
                    });
                }
                actions.extend(attack_actions(entity.id, PLAYER, attack));
                actions.push(remove_power_action(entity, sharp_hide));
                actions.push(set_next_move_action(entity, whirlwind_plan()));
                actions
            }
        }
    }

    fn on_damaged(
        state: &mut CombatState,
        entity: &MonsterEntity,
        amount: i32,
    ) -> smallvec::SmallVec<[ActionInfo; 4]> {
        if amount <= 0
            || !entity.guardian.is_open
            || entity.guardian.close_up_triggered
            || entity.is_dying
            || entity.half_dead
        {
            return smallvec::smallvec![];
        }

        let new_damage_taken = entity.guardian.damage_taken + amount;
        if new_damage_taken >= entity.guardian.damage_threshold {
            let next_threshold = entity.guardian.damage_threshold + THRESHOLD_INCREASE;
            smallvec::smallvec![
                ActionInfo {
                    action: guardian_runtime_update(entity, None, Some(0), None, Some(true)),
                    insertion_mode: AddTo::Top,
                },
                ActionInfo {
                    action: guardian_runtime_update(
                        entity,
                        Some(next_threshold),
                        None,
                        Some(false),
                        None,
                    ),
                    insertion_mode: AddTo::Bottom,
                },
                ActionInfo {
                    action: Action::RemovePower {
                        target: entity.id,
                        power_id: PowerId::ModeShift,
                    },
                    insertion_mode: AddTo::Bottom,
                },
                ActionInfo {
                    action: Action::GainBlock {
                        target: entity.id,
                        amount: DEFENSIVE_BLOCK,
                    },
                    insertion_mode: AddTo::Bottom,
                },
                ActionInfo {
                    action: set_next_move_action(entity, close_up_plan(state.meta.ascension_level)),
                    insertion_mode: AddTo::Bottom,
                },
            ]
        } else {
            smallvec::smallvec![
                ActionInfo {
                    action: guardian_runtime_update(
                        entity,
                        None,
                        Some(new_damage_taken),
                        None,
                        None
                    ),
                    insertion_mode: AddTo::Top,
                },
                ActionInfo {
                    action: Action::ReducePower {
                        target: entity.id,
                        power_id: PowerId::ModeShift,
                        amount,
                    },
                    insertion_mode: AddTo::Top,
                },
            ]
        }
    }
}
