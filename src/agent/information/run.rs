//! Public run context carried into combat decisions.

use serde::{Deserialize, Serialize};

use crate::ai::planner_core::{PlannerPublicMap, PlannerRunGoal};
use crate::content::monsters::factory::EncounterId;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum PublicCombatRunContextV1 {
    Available {
        run_goal: PlannerRunGoal,
        act: u8,
        floor: i32,
        keys: [bool; 3],
        public_map: PlannerPublicMap,
        encounter_id: Option<EncounterId>,
    },
    Unavailable {
        reason: PublicCombatRunContextGapV1,
    },
}

impl PublicCombatRunContextV1 {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available { .. })
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PublicCombatRunContextGapV1 {
    DetachedExactCombatPosition,
}
