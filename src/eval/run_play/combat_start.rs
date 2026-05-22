use crate::content::monsters::factory::EncounterId;
use crate::runtime::combat::CombatState;
use crate::sim::combat_start::{build_natural_combat_start, encounter_id_from_event_key};
use crate::state::core::EngineState;
use crate::state::map::node::RoomType;
use crate::state::run::RunState;

pub(super) fn ensure_combat_started_if_needed(
    engine_state: &mut EngineState,
    run_state: &mut RunState,
    combat_state: &mut Option<CombatState>,
) -> Result<(), String> {
    match engine_state {
        EngineState::CombatPlayerTurn
        | EngineState::CombatProcessing
        | EngineState::PendingChoice(_)
            if combat_state.is_none() =>
        {
            let room_type = run_state
                .map
                .get_current_room_type()
                .ok_or_else(|| "combat engine state without current map room".to_string())?;
            let encounter = encounter_for_current_room(run_state, room_type)?;
            let (engine, combat) = build_natural_combat_start(run_state, encounter, room_type)?;
            *engine_state = engine;
            *combat_state = Some(combat);
        }
        EngineState::EventCombat(event_combat) if combat_state.is_none() => {
            let encounter = encounter_id_from_event_key(&event_combat.encounter_key)?;
            let room_type = if event_combat.elite_trigger {
                RoomType::MonsterRoomElite
            } else {
                RoomType::MonsterRoom
            };
            let (_engine, combat) = build_natural_combat_start(run_state, encounter, room_type)?;
            *combat_state = Some(combat);
        }
        _ => {}
    }
    Ok(())
}

fn encounter_for_current_room(
    run_state: &mut RunState,
    room_type: RoomType,
) -> Result<EncounterId, String> {
    match room_type {
        RoomType::MonsterRoom => run_state
            .peek_next_encounter()
            .ok_or_else(|| "normal encounter queue is empty".to_string()),
        RoomType::MonsterRoomElite => run_state
            .peek_next_elite()
            .ok_or_else(|| "elite encounter queue is empty".to_string()),
        RoomType::MonsterRoomBoss => run_state
            .next_boss()
            .ok_or_else(|| "boss encounter queue is empty".to_string()),
        other => Err(format!("room type {other:?} is not combat")),
    }
}
