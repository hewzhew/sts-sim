use serde::{Deserialize, Serialize};

use super::{CombatStartRequest, PendingChoice, RunPendingChoiceState};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum EngineState {
    CombatPlayerTurn,
    CombatProcessing,
    RewardScreen(crate::state::rewards::RewardState),
    /// Java-style reward overlay opened from another screen. Closing or
    /// exhausting it returns to the stashed screen; opening a card choice stays
    /// inside the overlay reward surface.
    RewardOverlay {
        reward_state: crate::state::rewards::RewardState,
        return_state: Box<EngineState>,
    },
    TreasureRoom(crate::state::rewards::TreasureChestState),
    Campfire,
    Shop(crate::state::shop::ShopState),
    MapNavigation,
    /// Java-style map overlay opened from another screen. Closing it returns to
    /// the stashed screen; selecting a map node commits travel and drops the
    /// overlay return path.
    MapOverlay {
        return_state: Box<EngineState>,
    },
    EventRoom,
    /// Request to construct a concrete active combat from RunState. This is a
    /// transient run boundary, not a capture/search combat decision boundary.
    CombatStart(CombatStartRequest),
    PendingChoice(PendingChoice),
    RunPendingChoice(RunPendingChoiceState),
    BossRelicSelect(crate::state::rewards::BossRelicChoiceState),
    GameOver(RunResult),
}

impl EngineState {
    pub fn map_overlay(return_state: EngineState) -> Self {
        Self::MapOverlay {
            return_state: Box::new(return_state),
        }
    }

    pub fn reward_overlay(
        reward_state: crate::state::rewards::RewardState,
        return_state: EngineState,
    ) -> Self {
        Self::RewardOverlay {
            reward_state,
            return_state: Box::new(return_state),
        }
    }

    pub fn is_map_surface(&self) -> bool {
        matches!(self, Self::MapNavigation | Self::MapOverlay { .. })
    }
}

/// Where to go after event combat finishes.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum PostCombatReturn {
    /// Return to the event dialog (e.g., Colosseum between fights).
    EventRoom,
    /// Standard: combat done -> rewards -> map navigation.
    MapNavigation,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum RunResult {
    Victory,
    Defeat,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub enum TopLevelState {
    InCombat,
    OnMap,
    AtCampfire,
    InShop,
    OnRewardScreen,
    OnEvent,
}
