use serde::{Deserialize, Serialize};

use crate::content::monsters::factory::EncounterId;
use crate::runtime::combat::CombatState;
use crate::state::map::node::RoomType;
use crate::state::rewards::RewardState;

use super::{EngineState, PostCombatReturn};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CombatStartRequest {
    pub encounter_id: EncounterId,
    pub room_type: RoomType,
    pub context: CombatContext,
}

impl CombatStartRequest {
    pub fn room(encounter_id: EncounterId, room_type: RoomType) -> Self {
        Self {
            encounter_id,
            room_type,
            context: CombatContext::Room(RoomCombatContext { room_type }),
        }
    }

    pub fn event(
        encounter_id: EncounterId,
        rewards: RewardState,
        reward_allowed: bool,
        no_cards_in_rewards: bool,
        elite_trigger: bool,
        post_combat_return: PostCombatReturn,
    ) -> Self {
        let room_type = if elite_trigger {
            RoomType::MonsterRoomElite
        } else {
            RoomType::MonsterRoom
        };
        Self {
            encounter_id,
            room_type,
            context: CombatContext::Event(EventCombatContext {
                rewards,
                reward_allowed,
                no_cards_in_rewards,
                elite_trigger,
                post_combat_return,
            }),
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum CombatContext {
    Room(RoomCombatContext),
    Event(EventCombatContext),
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct RoomCombatContext {
    pub room_type: RoomType,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct EventCombatContext {
    /// Pre-populated rewards (gold, relics) added before combat starts.
    pub rewards: RewardState,
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
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ActiveCombat {
    pub engine_state: EngineState,
    pub combat_state: CombatState,
    pub context: CombatContext,
}

impl ActiveCombat {
    pub fn new(
        engine_state: EngineState,
        combat_state: CombatState,
        context: CombatContext,
    ) -> Self {
        Self {
            engine_state,
            combat_state,
            context,
        }
    }
}
