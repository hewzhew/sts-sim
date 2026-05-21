use crate::runtime::action::{ActionInfo, AddTo};
use crate::runtime::combat::{CombatPhase, CombatState};
use crate::state::core::EngineState;

pub(super) fn resolve_victory_hooks_immediately(combat_state: &mut CombatState) {
    let mut actions = crate::content::relics::hooks::on_victory(combat_state);
    for power in crate::content::powers::store::powers_snapshot_for(combat_state, 0) {
        let power_actions = crate::content::powers::resolve_power_on_victory(
            power.power_type,
            combat_state,
            0,
            power.amount,
        );
        for action in power_actions {
            actions.push(ActionInfo {
                action,
                insertion_mode: AddTo::Bottom,
            });
        }
    }
    if actions.is_empty() {
        return;
    }

    combat_state.queue_actions(actions);
    while let Some(action) = combat_state.pop_next_action() {
        crate::engine::action_handlers::execute_action(action, combat_state);
        combat_state.ensure_flush_next_queued_card();
    }
}

pub(super) fn settle_victory_if_ready(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
) -> Option<bool> {
    if !combat_state.are_monsters_basically_dead_java() {
        return None;
    }

    if !combat_state.turn.counters.victory_triggered {
        combat_state.turn.mark_victory_triggered();
        resolve_victory_hooks_immediately(combat_state);
    }

    if !combat_state.has_pending_actions()
        && combat_state.zones.queued_cards.is_empty()
        && !combat_state.zones.limbo.is_empty()
    {
        let limbo_cards = std::mem::take(&mut combat_state.zones.limbo);
        for card in limbo_cards {
            combat_state.add_card_to_discard_pile_top(card);
        }
    }

    // Java does not cut off queued onUseCard / onDeath aftermath when the last monster dies.
    // Finish draining any already-queued actions (e.g. Rage block, relic hooks, death hooks)
    // before transitioning to rewards.
    if !combat_state.has_pending_actions()
        && combat_state.zones.limbo.is_empty()
        && combat_state.zones.queued_cards.is_empty()
    {
        *engine_state = EngineState::RewardScreen(crate::state::rewards::RewardState::new());
        return Some(false);
    }
    *engine_state = EngineState::CombatProcessing;
    Some(true)
}

pub fn is_smoke_escape_stable_boundary(
    engine_state: &EngineState,
    combat_state: &CombatState,
) -> bool {
    matches!(engine_state, EngineState::CombatProcessing)
        && matches!(combat_state.turn.current_phase, CombatPhase::TurnTransition)
        && combat_state.runtime.combat_smoked
        && combat_state.turn.counters.player_escaping
        && combat_state.turn.counters.victory_triggered
        && combat_state.turn.counters.escape_pending_reward
        && !combat_state.has_pending_actions()
        && combat_state.zones.queued_cards.is_empty()
        && combat_state.zones.limbo.is_empty()
}
