use sts_simulator::eval::run_control::{RunControlAutoStepOptions, RunControlSession};

use super::combat_search_lane_options;
use super::Args;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum CombatSearchStakes {
    Hallway,
    Elite,
    Boss,
}

#[derive(Clone, Copy)]
pub(super) enum CombatSearchLaneKind {
    Primary,
    DiagnosticRescue,
    HallwayImmediateRescue,
    NonBossPotionRescue,
    BossNoPotion,
    BossPotionRescue,
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
    pub(super) stakes: CombatSearchStakes,
}

impl CombatSearchRequest {
    pub(super) fn from_session(session: &RunControlSession, args: Args) -> Self {
        Self {
            args,
            stakes: combat_search_stakes(session),
        }
    }

    pub(super) fn portfolio_after_primary(
        &self,
        session: &RunControlSession,
    ) -> Vec<CombatSearchLane> {
        let mut lanes = Vec::new();
        match self.stakes {
            CombatSearchStakes::Boss => {
                lanes.push(CombatSearchLane::new(CombatSearchLaneKind::BossNoPotion));
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::BossPotionRescue,
                ));
                lanes.push(CombatSearchLane::new(CombatSearchLaneKind::QualityRealHp));
            }
            CombatSearchStakes::Elite => {
                lanes.push(CombatSearchLane::new(
                    CombatSearchLaneKind::DiagnosticRescue,
                ));
                if should_try_nonboss_potion_rescue(session) {
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
                if should_try_nonboss_potion_rescue(session) {
                    lanes.push(CombatSearchLane::new(
                        CombatSearchLaneKind::NonBossPotionRescue,
                    ));
                }
            }
        }
        lanes
    }

    pub(super) fn should_report(&self) -> bool {
        self.stakes == CombatSearchStakes::Boss
    }

    pub(super) fn combat_budget_capped(&self) -> bool {
        match self.stakes {
            CombatSearchStakes::Boss => self.args.wall_capped_boss_budget,
            CombatSearchStakes::Elite | CombatSearchStakes::Hallway => {
                self.args.wall_capped_search_budget
            }
        }
    }
}

impl CombatSearchLane {
    pub(super) fn primary() -> Self {
        Self::new(CombatSearchLaneKind::Primary)
    }

    fn new(kind: CombatSearchLaneKind) -> Self {
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
            CombatSearchLaneKind::BossNoPotion => "no_potion",
            CombatSearchLaneKind::BossPotionRescue => "potion_rescue",
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
        matches!(self.kind, CombatSearchLaneKind::NonBossPotionRescue)
    }

    pub(super) fn options(
        self,
        request: &CombatSearchRequest,
        session: &RunControlSession,
    ) -> RunControlAutoStepOptions {
        combat_search_lane_options::lane_options(self, request, session)
    }
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
