use serde::{Deserialize, Serialize};

use super::{PendingChoice, RunPendingChoiceState};

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
    PendingChoice(PendingChoice),
    RunPendingChoice(RunPendingChoiceState),
    /// Event-triggered combat: carries pre-populated rewards and post-combat return info.
    /// Combat proceeds normally (CombatPlayerTurn), and when it ends, the engine
    /// checks this state to determine how to handle rewards and where to return.
    EventCombat(EventCombatState),
    BossRelicSelect(crate::state::rewards::BossRelicChoiceState),
    GameOver(RunResult),
}

/// State for event-triggered combat.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EventCombatState {
    /// Pre-populated rewards (gold, relics) added before combat starts.
    pub rewards: crate::state::rewards::RewardState,
    /// If false, skip the reward screen entirely after combat (e.g., Colosseum fight 1).
    pub reward_allowed: bool,
    /// If true, suppress card rewards in the reward screen.
    pub no_cards_in_rewards: bool,
    /// Java `AbstractRoom.eliteTrigger` for event combats. This is a combat
    /// semantics flag for relics/powers, not permission to generate normal
    /// elite rewards.
    pub elite_trigger: bool,
    /// Where to transition after combat + rewards are done.
    pub post_combat_return: PostCombatReturn,
    /// Monster encounter key (e.g., "2 Orb Walkers") for identification.
    pub encounter_key: String,
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
