use super::combat_search_lanes::CombatSearchLane;
#[cfg(test)]
use super::combat_search_lanes::CombatSearchLaneKind;
use super::combat_search_portfolio_context::CombatSearchPortfolioContext;

pub(super) struct CombatSearchPortfolioPlan {
    lanes: Vec<CombatSearchLane>,
}

impl CombatSearchPortfolioPlan {
    pub(super) fn after_primary(_context: CombatSearchPortfolioContext) -> Self {
        Self { lanes: Vec::new() }
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
    fn non_boss_plan_disables_post_primary_lanes() {
        for stakes in [CombatSearchStakes::Elite, CombatSearchStakes::Hallway] {
            let plan = CombatSearchPortfolioPlan::after_primary(CombatSearchPortfolioContext {
                stakes,
                time_eater_boss: false,
                nonboss_potion_rescue_signal: true,
            });

            assert!(plan.lane_kinds().is_empty());
            assert!(!plan.should_report());
        }
    }
}
