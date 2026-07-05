use super::*;
use crate::content::powers::{store, PowerId};

const SPLIT_MOVE_ID: u8 = 3;
const SENTRY_BOLT_MOVE_ID: u8 = 3;
const HEXAGHOST_DIVIDER_MOVE_ID: u8 = 1;
const HEXAGHOST_ACTIVATE_MOVE_ID: u8 = 5;
const BRONZE_AUTOMATON_HYPER_BEAM_MOVE_ID: u8 = 2;
const BRONZE_AUTOMATON_SPAWN_ORBS_MOVE_ID: u8 = 4;
const BRONZE_ORB_STASIS_MOVE_ID: u8 = 3;

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
    pub(super) fungi_beast_count: usize,
    pub(super) healer_support_count: usize,
    pub(super) hexaghost_opening_pressure_count: usize,
    pub(super) bronze_automaton_count: usize,
    pub(super) bronze_automaton_spawn_orbs_pending_count: usize,
    pub(super) bronze_automaton_hyper_beam_pending_count: usize,
    pub(super) bronze_orb_count: usize,
    pub(super) bronze_orb_stasis_pending_count: usize,
    pub(super) bronze_orb_stasis_card_count: usize,
    pub(super) awakened_one_curiosity_count: usize,
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
            EnemyId::FungiBeast => {
                profile.tracked_monsters += 1;
                profile.fungi_beast_count += 1;
            }
            EnemyId::Healer => {
                profile.tracked_monsters += 1;
                profile.healer_support_count += 1;
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
            EnemyId::BronzeAutomaton => {
                profile.tracked_monsters += 1;
                profile.bronze_automaton_count += 1;
                if monster.bronze_automaton.first_turn
                    || monster.planned_move_id() == BRONZE_AUTOMATON_SPAWN_ORBS_MOVE_ID
                {
                    profile.bronze_automaton_spawn_orbs_pending_count += 1;
                }
                if monster.bronze_automaton.num_turns >= 4
                    || monster.planned_move_id() == BRONZE_AUTOMATON_HYPER_BEAM_MOVE_ID
                {
                    profile.bronze_automaton_hyper_beam_pending_count += 1;
                }
            }
            EnemyId::BronzeOrb => {
                profile.tracked_monsters += 1;
                profile.bronze_orb_count += 1;
                if !monster.bronze_orb.used_stasis
                    && monster.planned_move_id() == BRONZE_ORB_STASIS_MOVE_ID
                {
                    profile.bronze_orb_stasis_pending_count += 1;
                }
                if store::has_power(combat, monster.id, PowerId::Stasis) {
                    profile.bronze_orb_stasis_card_count += 1;
                }
            }
            EnemyId::AwakenedOne => {
                profile.tracked_monsters += 1;
                if store::has_power(combat, monster.id, PowerId::Curiosity) {
                    profile.awakened_one_curiosity_count += 1;
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
        fungi_beast_count: profile.fungi_beast_count,
        healer_support_count: profile.healer_support_count,
        hexaghost_opening_pressure_count: profile.hexaghost_opening_pressure_count,
        bronze_automaton_count: profile.bronze_automaton_count,
        bronze_automaton_spawn_orbs_pending_count: profile
            .bronze_automaton_spawn_orbs_pending_count,
        bronze_automaton_hyper_beam_pending_count: profile
            .bronze_automaton_hyper_beam_pending_count,
        bronze_orb_count: profile.bronze_orb_count,
        bronze_orb_stasis_pending_count: profile.bronze_orb_stasis_pending_count,
        bronze_orb_stasis_card_count: profile.bronze_orb_stasis_card_count,
        awakened_one_curiosity_count: profile.awakened_one_curiosity_count,
        notes: vec![
            "enemy mechanics profile exposes typed phase/support facts for value/rollout consumers",
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
mod tests;
