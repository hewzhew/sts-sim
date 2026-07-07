use sts_simulator::eval::run_control::{RunControlAutoStepOptions, RunControlSession};

use super::combat_search_lane_options;
use super::combat_search_portfolio_context::CombatSearchPortfolioContext;
use super::combat_search_portfolio_plan::CombatSearchPortfolioPlan;
use super::Args;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum CombatSearchStakes {
    Hallway,
    Elite,
    Boss,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CombatSearchLaneKind {
    Primary,
    DiagnosticRescue,
    HallwayImmediateRescue,
    NonBossPotionRescue,
    HallwayQualityPotionRescue,
    BossNoPotion,
    BossPotionRescue,
    BossTimeEaterClock,
    QualityRealHp,
}

#[derive(Clone, Copy)]
pub(super) enum CombatSearchLaneCommitPolicy {
    AcceptedLineOnly,
    AcceptedLineOrPrimaryChunk,
}

#[derive(Clone, Copy)]
pub(super) struct CombatSearchLane {
    kind: CombatSearchLaneKind,
}

pub(super) struct CombatSearchRequest {
    pub(super) args: Args,
    context: CombatSearchPortfolioContext,
}

impl CombatSearchRequest {
    pub(super) fn from_session(session: &RunControlSession, args: Args) -> Self {
        Self {
            args,
            context: CombatSearchPortfolioContext::from_session(session),
        }
    }

    pub(super) fn portfolio_after_primary(&self) -> Vec<CombatSearchLane> {
        CombatSearchPortfolioPlan::after_primary(self.context).into_lanes()
    }

    pub(super) fn should_report(&self) -> bool {
        self.context.stakes == CombatSearchStakes::Boss
    }

    pub(super) fn combat_budget_capped(&self) -> bool {
        match self.context.stakes {
            CombatSearchStakes::Boss => self.args.wall_capped_boss_budget,
            CombatSearchStakes::Elite | CombatSearchStakes::Hallway => {
                self.args.wall_capped_search_budget
            }
        }
    }

    pub(super) fn stakes(&self) -> CombatSearchStakes {
        self.context.stakes
    }
}

impl CombatSearchLane {
    pub(super) fn primary() -> Self {
        Self::new(CombatSearchLaneKind::Primary)
    }

    pub(super) fn new(kind: CombatSearchLaneKind) -> Self {
        Self { kind }
    }

    pub(super) fn kind(self) -> CombatSearchLaneKind {
        self.kind
    }

    pub(super) fn label(self) -> &'static str {
        match self.kind {
            CombatSearchLaneKind::Primary => "primary",
            CombatSearchLaneKind::DiagnosticRescue => "diagnostic_rescue",
            CombatSearchLaneKind::HallwayImmediateRescue => "hallway_immediate_rescue",
            CombatSearchLaneKind::NonBossPotionRescue => "nonboss_potion_rescue",
            CombatSearchLaneKind::HallwayQualityPotionRescue => "hallway_quality_potion_rescue",
            CombatSearchLaneKind::BossNoPotion => "no_potion",
            CombatSearchLaneKind::BossPotionRescue => "potion_rescue",
            CombatSearchLaneKind::BossTimeEaterClock => "time_eater_clock",
            CombatSearchLaneKind::QualityRealHp => "quality_real_hp",
        }
    }

    pub(super) fn commit_policy(self) -> CombatSearchLaneCommitPolicy {
        match self.kind {
            CombatSearchLaneKind::Primary => {
                CombatSearchLaneCommitPolicy::AcceptedLineOrPrimaryChunk
            }
            _ => CombatSearchLaneCommitPolicy::AcceptedLineOnly,
        }
    }

    pub(super) fn rejects_new_curses(self) -> bool {
        matches!(
            self.kind,
            CombatSearchLaneKind::NonBossPotionRescue
                | CombatSearchLaneKind::HallwayQualityPotionRescue
        )
    }

    pub(super) fn options(
        self,
        request: &CombatSearchRequest,
        session: &RunControlSession,
    ) -> RunControlAutoStepOptions {
        combat_search_lane_options::lane_options(self, request, session)
    }
}
