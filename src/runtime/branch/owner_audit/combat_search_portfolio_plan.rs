use super::combat_search_lanes::{CombatSearchLane, CombatSearchLaneKind, CombatSearchStakes};
use super::combat_search_portfolio_context::CombatSearchPortfolioContext;

pub(super) struct CombatSearchPortfolioPlan {
    lanes: Vec<CombatSearchLane>,
}

pub(super) struct CombatSearchPortfolioSchedule {
    pub(super) lanes: Vec<CombatSearchLane>,
    pub(super) suppressed: Vec<CombatSearchLane>,
}

impl CombatSearchPortfolioPlan {
    pub(super) fn after_primary(context: CombatSearchPortfolioContext) -> Self {
        let lanes = match context.stakes {
            CombatSearchStakes::Hallway => {
                let mut lanes = vec![CombatSearchLane::new(
                    CombatSearchLaneKind::PrimaryImmediateEscalation,
                )];
                if context.nonboss_potion_rescue_signal {
                    lanes.push(CombatSearchLane::new(
                        CombatSearchLaneKind::HallwaySurvivalFallback,
                    ));
                }
                lanes
            }
            CombatSearchStakes::Elite => vec![CombatSearchLane::new(
                CombatSearchLaneKind::EliteSurvivalFallback,
            )],
            CombatSearchStakes::Boss => vec![CombatSearchLane::new(
                CombatSearchLaneKind::BossPotionRescue,
            )],
        };
        Self { lanes }
    }

    pub(super) fn into_schedule_by<K, F>(self, mut key_for: F) -> CombatSearchPortfolioSchedule
    where
        K: Eq,
        F: FnMut(CombatSearchLane) -> K,
    {
        let mut keys = Vec::new();
        let mut lanes = Vec::new();
        let mut suppressed = Vec::new();
        for lane in self.lanes {
            let key = key_for(lane);
            if keys.contains(&key) {
                suppressed.push(lane);
            } else {
                keys.push(key);
                lanes.push(lane);
            }
        }
        CombatSearchPortfolioSchedule { lanes, suppressed }
    }

    #[cfg(test)]
    fn lane_kinds(&self) -> Vec<CombatSearchLaneKind> {
        self.lanes.iter().map(|lane| lane.kind()).collect()
    }

    #[cfg(test)]
    fn lane_labels(&self) -> Vec<&'static str> {
        self.lanes.iter().map(|lane| lane.label()).collect()
    }

    #[cfg(test)]
    fn should_report(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::combat_search_lanes::CombatSearchStakes;
    use super::*;

    #[test]
    fn boss_plan_schedules_only_potion_rescue_after_primary_gap() {
        let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
            stakes: CombatSearchStakes::Boss,
            time_eater_boss: false,
            nonboss_potion_rescue_signal: false,
        });

        assert_eq!(
            plan.lane_kinds(),
            vec![CombatSearchLaneKind::BossPotionRescue]
        );
        assert!(!plan.should_report());
    }

    #[test]
    fn elite_plan_ends_with_one_survival_fallback() {
        let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
            stakes: CombatSearchStakes::Elite,
            time_eater_boss: false,
            nonboss_potion_rescue_signal: true,
        });

        assert_eq!(plan.lane_labels(), vec!["elite_survival_fallback"]);
        assert!(!plan.should_report());
    }

    #[test]
    fn hallway_plan_uses_explicit_primary_immediate_escalation() {
        let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
            stakes: CombatSearchStakes::Hallway,
            time_eater_boss: false,
            nonboss_potion_rescue_signal: false,
        });

        assert_eq!(
            plan.lane_kinds(),
            vec![CombatSearchLaneKind::PrimaryImmediateEscalation]
        );
        assert!(!plan.should_report());
    }

    #[test]
    fn pressured_hallway_plan_ends_with_survival_fallback() {
        let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
            stakes: CombatSearchStakes::Hallway,
            time_eater_boss: false,
            nonboss_potion_rescue_signal: true,
        });

        assert_eq!(
            plan.lane_labels(),
            vec!["primary_immediate_escalation", "hallway_survival_fallback"]
        );
        assert!(!plan.should_report());
    }

    #[test]
    fn duplicate_producer_is_suppressed_in_stable_order() {
        let duplicate = CombatSearchLane::new(CombatSearchLaneKind::HallwaySurvivalFallback);
        let plan = CombatSearchPortfolioPlan {
            lanes: vec![duplicate, duplicate],
        };

        let schedule = plan.into_schedule_by(|lane| lane.kind());

        assert_eq!(schedule.lanes.len(), 1);
        assert_eq!(schedule.lanes[0].kind(), duplicate.kind());
        assert_eq!(schedule.suppressed.len(), 1);
        assert_eq!(schedule.suppressed[0].kind(), duplicate.kind());
    }
}
