use super::combat_search_lanes::{CombatSearchLaneCommitPolicy, CombatSearchLaneKind};

#[derive(Clone, Copy)]
pub(super) struct CombatSearchLaneSpec {
    pub(super) label: &'static str,
    pub(super) commit_policy: CombatSearchLaneCommitPolicy,
    pub(super) rejects_new_curses: bool,
}

pub(super) fn lane_spec(kind: CombatSearchLaneKind) -> CombatSearchLaneSpec {
    match kind {
        CombatSearchLaneKind::Primary => CombatSearchLaneSpec {
            label: "primary",
            commit_policy: CombatSearchLaneCommitPolicy::AcceptedLineOrPrimaryChunk,
            rejects_new_curses: false,
        },
        CombatSearchLaneKind::DiagnosticRescue => rescue_spec("diagnostic_rescue"),
        CombatSearchLaneKind::HallwayImmediateRescue => rescue_spec("hallway_immediate_rescue"),
        CombatSearchLaneKind::NonBossPotionRescue => dirty_rejecting_spec("nonboss_potion_rescue"),
        CombatSearchLaneKind::HallwayQualityPotionRescue => {
            dirty_rejecting_spec("hallway_quality_potion_rescue")
        }
        CombatSearchLaneKind::BossNoPotion => rescue_spec("no_potion"),
        CombatSearchLaneKind::BossPotionRescue => rescue_spec("potion_rescue"),
        CombatSearchLaneKind::BossTimeEaterClock => rescue_spec("time_eater_clock"),
        CombatSearchLaneKind::QualityRealHp => rescue_spec("quality_real_hp"),
    }
}

fn rescue_spec(label: &'static str) -> CombatSearchLaneSpec {
    CombatSearchLaneSpec {
        label,
        commit_policy: CombatSearchLaneCommitPolicy::AcceptedLineOnly,
        rejects_new_curses: false,
    }
}

fn dirty_rejecting_spec(label: &'static str) -> CombatSearchLaneSpec {
    CombatSearchLaneSpec {
        rejects_new_curses: true,
        ..rescue_spec(label)
    }
}
