use super::combat_search_lanes::{CombatSearchLane, CombatSearchLaneKind, CombatSearchStakes};
use super::combat_search_portfolio_context::CombatSearchPortfolioContext;

pub(super) struct CombatSearchPortfolioPlan {
    lanes: Vec<CombatSearchLane>,
}

impl CombatSearchPortfolioPlan {
    pub(super) fn after_primary(context: CombatSearchPortfolioContext) -> Self {
        let lanes = match context.stakes {
            CombatSearchStakes::Hallway => {
                vec![CombatSearchLane::new(
                    CombatSearchLaneKind::PrimaryImmediateEscalation,
                )]
            }
            CombatSearchStakes::Elite | CombatSearchStakes::Boss => Vec::new(),
        };
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
        false
    }
}

#[cfg(test)]
mod tests {
    use super::super::combat_search_lanes::CombatSearchStakes;
    use super::*;

    #[test]
    fn boss_plan_disables_post_primary_lanes() {
        let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
            stakes: CombatSearchStakes::Boss,
            time_eater_boss: false,
            nonboss_potion_rescue_signal: false,
        });

        assert!(plan.lane_kinds().is_empty());
        assert!(!plan.should_report());
    }

    #[test]
    fn elite_plan_disables_post_primary_lanes() {
        let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
            stakes: CombatSearchStakes::Elite,
            time_eater_boss: false,
            nonboss_potion_rescue_signal: true,
        });

        assert!(plan.lane_kinds().is_empty());
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
}
