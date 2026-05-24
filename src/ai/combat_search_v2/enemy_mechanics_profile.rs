use super::*;
use crate::content::powers::{store, PowerId};

const SPLIT_MOVE_ID: u8 = 3;
const SENTRY_BOLT_MOVE_ID: u8 = 3;
const HEXAGHOST_DIVIDER_MOVE_ID: u8 = 1;
const HEXAGHOST_ACTIVATE_MOVE_ID: u8 = 5;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct EnemyMechanicsProfileV1 {
    pub(super) tracked_monsters: usize,
    pub(super) split_pending_count: usize,
    pub(super) guardian_open_count: usize,
    pub(super) guardian_defensive_count: usize,
    pub(super) guardian_mode_shift_pending_count: usize,
    pub(super) guardian_min_mode_shift_remaining: Option<i32>,
    pub(super) lagavulin_sleeping_count: usize,
    pub(super) lagavulin_waking_count: usize,
    pub(super) gremlin_nob_enrage_count: usize,
    pub(super) gremlin_nob_anger_amount_total: i32,
    pub(super) sentry_dazed_pressure_count: usize,
    pub(super) hexaghost_opening_pressure_count: usize,
}

pub(super) fn enemy_mechanics_profile(combat: &CombatState) -> EnemyMechanicsProfileV1 {
    let mut profile = EnemyMechanicsProfileV1::default();
    for monster in combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
    {
        let Some(enemy_id) = EnemyId::from_id(monster.monster_type) else {
            continue;
        };

        match enemy_id {
            EnemyId::AcidSlimeL | EnemyId::SpikeSlimeL | EnemyId::SlimeBoss => {
                profile.tracked_monsters += 1;
                if split_pending_or_triggered(combat, monster) {
                    profile.split_pending_count += 1;
                }
            }
            EnemyId::TheGuardian => {
                profile.tracked_monsters += 1;
                if monster.guardian.is_open {
                    profile.guardian_open_count += 1;
                    if store::has_power(combat, monster.id, PowerId::ModeShift) {
                        let remaining = store::power_amount(combat, monster.id, PowerId::ModeShift);
                        profile.guardian_min_mode_shift_remaining = Some(
                            profile
                                .guardian_min_mode_shift_remaining
                                .map_or(remaining, |old| old.min(remaining)),
                        );
                        if remaining <= 0 || monster.guardian.close_up_triggered {
                            profile.guardian_mode_shift_pending_count += 1;
                        }
                    }
                } else {
                    profile.guardian_defensive_count += 1;
                }
            }
            EnemyId::Lagavulin => {
                profile.tracked_monsters += 1;
                if !monster.lagavulin.is_out {
                    profile.lagavulin_sleeping_count += 1;
                }
                if monster.lagavulin.is_out_triggered {
                    profile.lagavulin_waking_count += 1;
                }
            }
            EnemyId::GremlinNob => {
                profile.tracked_monsters += 1;
                if store::has_power(combat, monster.id, PowerId::Anger) {
                    let amount = store::power_amount(combat, monster.id, PowerId::Anger);
                    profile.gremlin_nob_enrage_count += 1;
                    profile.gremlin_nob_anger_amount_total += amount;
                }
            }
            EnemyId::Sentry => {
                profile.tracked_monsters += 1;
                if monster.planned_move_id() == SENTRY_BOLT_MOVE_ID {
                    profile.sentry_dazed_pressure_count += 1;
                }
            }
            EnemyId::Hexaghost => {
                profile.tracked_monsters += 1;
                if matches!(
                    monster.planned_move_id(),
                    HEXAGHOST_ACTIVATE_MOVE_ID | HEXAGHOST_DIVIDER_MOVE_ID
                ) {
                    profile.hexaghost_opening_pressure_count += 1;
                }
            }
            _ => {}
        }
    }
    profile
}

pub(super) fn enemy_mechanics_profile_report(
    profile: EnemyMechanicsProfileV1,
) -> CombatSearchV2EnemyMechanicsReport {
    CombatSearchV2EnemyMechanicsReport {
        profiling_policy: "typed_act1_enemy_mechanics_fact_profile_no_direct_score",
        tracked_monsters: profile.tracked_monsters,
        split_pending_count: profile.split_pending_count,
        guardian_open_count: profile.guardian_open_count,
        guardian_defensive_count: profile.guardian_defensive_count,
        guardian_mode_shift_pending_count: profile.guardian_mode_shift_pending_count,
        guardian_min_mode_shift_remaining: profile.guardian_min_mode_shift_remaining,
        lagavulin_sleeping_count: profile.lagavulin_sleeping_count,
        lagavulin_waking_count: profile.lagavulin_waking_count,
        gremlin_nob_enrage_count: profile.gremlin_nob_enrage_count,
        gremlin_nob_anger_amount_total: profile.gremlin_nob_anger_amount_total,
        sentry_dazed_pressure_count: profile.sentry_dazed_pressure_count,
        hexaghost_opening_pressure_count: profile.hexaghost_opening_pressure_count,
        notes: vec![
            "enemy mechanics profile exposes phase facts for value/rollout consumers",
            "this profile does not by itself score or prune search branches",
            "split phase debt used by frontier value remains in enemy_phase_value",
        ],
    }
}

fn split_pending_or_triggered(combat: &CombatState, monster: &MonsterEntity) -> bool {
    store::has_power(combat, monster.id, PowerId::Split)
        && (monster.planned_move_id() == SPLIT_MOVE_ID
            || monster.current_hp <= monster.max_hp.saturating_div(2))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::combat::{Power, PowerPayload};
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn guardian_profile_reports_mode_shift_remaining() {
        let mut combat = blank_test_combat();
        let mut guardian = test_monster(EnemyId::TheGuardian);
        guardian.id = 7;
        guardian.guardian.is_open = true;
        combat.entities.monsters = vec![guardian];
        combat.entities.power_db.insert(
            7,
            vec![Power {
                power_type: PowerId::ModeShift,
                instance_id: None,
                amount: 4,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        let profile = enemy_mechanics_profile(&combat);

        assert_eq!(profile.guardian_open_count, 1);
        assert_eq!(profile.guardian_min_mode_shift_remaining, Some(4));
    }

    #[test]
    fn gremlin_nob_profile_reports_anger_amount() {
        let mut combat = blank_test_combat();
        let mut nob = test_monster(EnemyId::GremlinNob);
        nob.id = 9;
        combat.entities.monsters = vec![nob];
        combat.entities.power_db.insert(
            9,
            vec![Power {
                power_type: PowerId::Anger,
                instance_id: None,
                amount: 2,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );

        let profile = enemy_mechanics_profile(&combat);

        assert_eq!(profile.gremlin_nob_enrage_count, 1);
        assert_eq!(profile.gremlin_nob_anger_amount_total, 2);
    }
}
