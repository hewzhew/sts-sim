use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlConfig, RunControlSession,
};
use sts_simulator::state::core::EngineState;
use sts_simulator::state::events::EventState;

use super::owner_model::{OwnerDecision, OwnerRoutine};
use super::{owners, Args, EventOwnerProbeArgs, Owner};

pub(super) fn run(args: Args, probe: EventOwnerProbeArgs) -> Result<(), String> {
    let mut session = RunControlSession::new(RunControlConfig {
        seed: args.seed,
        ascension_level: args.ascension,
        ..Default::default()
    });
    let mut event_state = EventState::new(probe.event_id);
    event_state.current_screen = probe.screen;
    session.run_state.event_state = Some(event_state);
    session.engine_state = EngineState::EventRoom;

    let surface = build_decision_surface(&session);
    println!(
        "event_owner_probe event={:?} screen={} candidates={}",
        probe.event_id,
        probe.screen,
        surface.view.candidates.len()
    );
    for candidate in &surface.view.candidates {
        println!(
            "  candidate id={} key={:?} label={} command={:?}",
            candidate.id,
            candidate.key,
            candidate.label,
            candidate.action.executable_command()
        );
    }

    match owners::owner_decision(&session, Owner::Event(probe.event_id), &surface) {
        OwnerDecision::Routine(OwnerRoutine::Command(command)) => {
            println!("  owner_decision=command {command:?}");
        }
        OwnerDecision::Routine(OwnerRoutine::RewardTinyAutomation) => {
            println!("  owner_decision=unexpected_reward_tiny_automation");
        }
        OwnerDecision::Routine(OwnerRoutine::AdvanceEmptyCampfire) => {
            println!("  owner_decision=unexpected_advance_empty_campfire");
        }
        OwnerDecision::Candidates(choices) => {
            println!("  owner_decision=candidates count={}", choices.len());
            for choice in choices {
                println!(
                    "    choice key={:?} label={} command={:?}",
                    choice.key, choice.label, choice.action
                );
            }
        }
        OwnerDecision::Gap(reason) => {
            println!("  owner_decision=gap {reason}");
        }
    }
    Ok(())
}
