use sts_simulator::content::monsters::EnemyId;
use sts_simulator::eval::run_control::RunControlSession;

use super::combat_search_lanes::{CombatSearchLane, CombatSearchLaneKind, CombatSearchStakes};

#[derive(Clone, Copy)]
pub(super) struct CombatSearchPortfolioContext {
    pub(super) stakes: CombatSearchStakes,
    pub(super) time_eater_boss: bool,
    pub(super) nonboss_potion_rescue_signal: bool,
}

pub(super) struct CombatSearchPortfolioPlan {
    lanes: Vec<CombatSearchLane>,
}

impl CombatSearchPortfolioContext {
    pub(super) fn from_session(session: &RunControlSession) -> Self {
        Self {
            stakes: combat_search_stakes(session),
            time_eater_boss: is_time_eater_boss(session),
            nonboss_potion_rescue_signal: should_try_nonboss_potion_rescue(session),
        }
    }
}

impl CombatSearchPortfolioPlan {
    pub(super) fn after_primary(context: CombatSearchPortfolioContext) -> Self {
        let mut lanes = Vec::new();
        match context.stakes {
            CombatSearchStakes::Boss => {
                lanes.push(CombatSearchLane::new(CombatSearchLaneKind::BossNoPotion));
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::BossPotionRescue,
                ));
                if context.time_eater_boss {
                    lanes.push(CombatSearchLane::new(
                        CombatSearchLaneKind::BossTimeEaterClock,
                    ));
                }
                lanes.push(CombatSearchLane::new(CombatSearchLaneKind::QualityRealHp));
            }
            CombatSearchStakes::Elite => {
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::DiagnosticRescue,
                ));
                if context.nonboss_potion_rescue_signal {
                    lanes.push(CombatSearchLane::new(
                        CombatSearchLaneKind::NonBossPotionRescue,
                    ));
                }
                lanes.push(CombatSearchLane::new(CombatSearchLaneKind::QualityRealHp));
            }
            CombatSearchStakes::Hallway => {
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::DiagnosticRescue,
                ));
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::HallwayImmediateRescue,
                ));
                if context.nonboss_potion_rescue_signal {
                    lanes.push(CombatSearchLane::new(
                        CombatSearchLaneKind::NonBossPotionRescue,
                    ));
                    lanes.push(CombatSearchLane::new(
                        CombatSearchLaneKind::HallwayQualityPotionRescue,
                    ));
                }
            }
        }
        Self { lanes }
    }

    pub(super) fn into_lanes(self) -> Vec<CombatSearchLane> {
        self.lanes
    }

    #[cfg(test)]
    fn lane_kinds(&self) -> Vec<CombatSearchLaneKind> {
        self.lanes.iter().map(|lane| lane.kind()).collect()
    }

    #[cfg(test)]
    fn should_report(&self) -> bool {
        self.lanes
            .iter()
            .any(|lane| matches!(lane.kind(), CombatSearchLaneKind::BossNoPotion))
    }
}

fn combat_search_stakes(session: &RunControlSession) -> CombatSearchStakes {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            if active.combat_state.meta.is_boss_fight {
                CombatSearchStakes::Boss
            } else if active.combat_state.meta.is_elite_fight {
                CombatSearchStakes::Elite
            } else {
                CombatSearchStakes::Hallway
            }
        })
        .unwrap_or(CombatSearchStakes::Hallway)
}

fn is_time_eater_boss(session: &RunControlSession) -> bool {
    session.active_combat.as_ref().is_some_and(|active| {
        active.combat_state.meta.is_boss_fight
            && active
                .combat_state
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.is_alive_for_action())
                .any(|monster| EnemyId::from_id(monster.monster_type) == Some(EnemyId::TimeEater))
    })
}

fn should_try_nonboss_potion_rescue(session: &RunControlSession) -> bool {
    let Some(active) = session.active_combat.as_ref() else {
        return false;
    };
    let meta = &active.combat_state.meta;
    let player = &active.combat_state.entities.player;
    let has_usable_potion = active
        .combat_state
        .entities
        .potions
        .iter()
        .flatten()
        .any(|potion| potion.can_use);
    !meta.is_boss_fight
        && has_usable_potion
        && (meta.is_elite_fight
            || session.run_state.act_num >= 3
            || player.current_hp * 2 <= player.max_hp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn boss_plan_lists_rescue_lanes_without_session_wiring() {
        let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
            stakes: CombatSearchStakes::Boss,
            time_eater_boss: false,
            nonboss_potion_rescue_signal: false,
        });

        assert_eq!(
            plan.lane_kinds(),
            vec![
                CombatSearchLaneKind::BossNoPotion,
                CombatSearchLaneKind::BossPotionRescue,
                CombatSearchLaneKind::QualityRealHp,
            ]
        );
        assert!(plan.should_report());
    }

    #[test]
    fn hallway_plan_adds_potion_lanes_only_when_context_says_so() {
        let without_potion =
            CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
                stakes: CombatSearchStakes::Hallway,
                time_eater_boss: false,
                nonboss_potion_rescue_signal: false,
            });
        let with_potion = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
            stakes: CombatSearchStakes::Hallway,
            time_eater_boss: false,
            nonboss_potion_rescue_signal: true,
        });

        assert_eq!(
            without_potion.lane_kinds(),
            vec![
                CombatSearchLaneKind::DiagnosticRescue,
                CombatSearchLaneKind::HallwayImmediateRescue,
            ]
        );
        assert_eq!(
            with_potion.lane_kinds(),
            vec![
                CombatSearchLaneKind::DiagnosticRescue,
                CombatSearchLaneKind::HallwayImmediateRescue,
                CombatSearchLaneKind::NonBossPotionRescue,
                CombatSearchLaneKind::HallwayQualityPotionRescue,
            ]
        );
    }
}
