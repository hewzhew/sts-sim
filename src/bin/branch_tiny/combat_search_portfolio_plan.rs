use super::combat_search_lanes::{CombatSearchLane, CombatSearchLaneKind, CombatSearchStakes};
use super::combat_search_portfolio_context::CombatSearchPortfolioContext;

pub(super) struct CombatSearchPortfolioPlan {
    lanes: Vec<CombatSearchLane>,
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
