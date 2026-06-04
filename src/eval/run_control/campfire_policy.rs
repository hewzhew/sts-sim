use crate::ai::noncombat_strategy_v1::{
    build_run_strategy_snapshot_from_run_state_v2, StrategyPackageIdV2, StrategyPlanSupportV1,
};
use crate::state::core::{CampfireChoice, ClientInput, EngineState};

use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_campfire_policy_rest(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return Ok(None);
    }
    if session.run_state.current_hp >= session.run_state.max_hp {
        return Ok(None);
    }
    if !crate::engine::campfire_handler::get_available_options(&session.run_state)
        .contains(&CampfireChoice::Rest)
    {
        return Ok(None);
    }

    let snapshot = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
    let Some(recovery) = snapshot.package(StrategyPackageIdV2::RecoveryPressure) else {
        return Ok(None);
    };
    if recovery.support != StrategyPlanSupportV1::Strong {
        return Ok(None);
    }

    let summary = format!(
        "campfire policy: rest [RecoveryPressure {:?}]",
        recovery.support
    );
    let outcome = session.apply_input(ClientInput::CampfireOption(CampfireChoice::Rest))?;
    Ok(Some((outcome, summary)))
}
