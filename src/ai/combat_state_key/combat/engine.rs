use crate::state::core::EngineState;

use super::super::types::CombatEngineKey;
use super::pending_choice::pending_choice_key;

pub(super) fn engine_key(engine: &EngineState) -> CombatEngineKey {
    match engine {
        EngineState::CombatPlayerTurn => CombatEngineKey::CombatPlayerTurn,
        EngineState::CombatProcessing => CombatEngineKey::CombatProcessing,
        EngineState::PendingChoice(choice) => {
            CombatEngineKey::PendingChoice(pending_choice_key(choice))
        }
        EngineState::RewardScreen(value) => CombatEngineKey::RewardScreen(format!("{value:?}")),
        EngineState::TreasureRoom(value) => CombatEngineKey::TreasureRoom(format!("{value:?}")),
        EngineState::Campfire => CombatEngineKey::Campfire,
        EngineState::Shop(value) => CombatEngineKey::Shop(format!("{value:?}")),
        EngineState::MapNavigation => CombatEngineKey::MapNavigation,
        EngineState::EventRoom => CombatEngineKey::EventRoom,
        EngineState::CombatStart(value) => CombatEngineKey::CombatStart(format!("{value:?}")),
        EngineState::RunPendingChoice(value) => {
            CombatEngineKey::RunPendingChoice(format!("{value:?}"))
        }
        EngineState::BossRelicSelect(value) => {
            CombatEngineKey::BossRelicSelect(format!("{value:?}"))
        }
        EngineState::GameOver(value) => CombatEngineKey::GameOver(format!("{value:?}")),
    }
}
