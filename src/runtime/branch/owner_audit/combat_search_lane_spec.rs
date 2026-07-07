use sts_simulator::ai::combat_search_v2::CombatSearchAcceptancePluginId;

use super::combat_search_lanes::CombatSearchLaneKind;

#[derive(Clone, Copy)]
pub(super) struct CombatSearchLaneSpec {
    pub(super) label: &'static str,
    pub(super) acceptance: CombatSearchAcceptancePluginId,
}

pub(super) fn lane_spec(kind: CombatSearchLaneKind) -> CombatSearchLaneSpec {
    match kind {
        CombatSearchLaneKind::Primary => CombatSearchLaneSpec {
            label: "primary",
            acceptance: CombatSearchAcceptancePluginId::AcceptedLineOrPrimaryChunk,
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
        acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
    }
}

fn dirty_rejecting_spec(label: &'static str) -> CombatSearchLaneSpec {
    CombatSearchLaneSpec {
        label,
        acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
    }
}
