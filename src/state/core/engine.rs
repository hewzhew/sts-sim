use serde::{Deserialize, Serialize};

use super::{CombatStartRequest, PendingChoice, RunPendingChoiceState};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum EngineState {
    CombatPlayerTurn,
    CombatProcessing,
    RewardScreen(crate::state::rewards::RewardState),
    TreasureRoom(crate::state::rewards::TreasureChestState),
    Campfire,
    Shop(crate::state::shop::ShopState),
    MapNavigation,
    EventRoom,
    /// Request to construct a concrete active combat from RunState. This is a
    /// transient run boundary, not a capture/search combat decision boundary.
    CombatStart(CombatStartRequest),
    PendingChoice(PendingChoice),
    RunPendingChoice(RunPendingChoiceState),
    BossRelicSelect(crate::state::rewards::BossRelicChoiceState),
    GameOver(RunResult),
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
