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
            "  candidate id={} key={:?} label={} action={:?}",
            candidate.id,
            candidate.key,
            candidate.label,
            candidate.action.executable_action()
        );
    }

    match owners::owner_decision(&session, Owner::Event(probe.event_id), &surface) {
        OwnerDecision::Routine(OwnerRoutine::Candidate {
            candidate_id,
            action,
        }) => {
            println!("  owner_decision=candidate id={candidate_id} action={action:?}");
        }
        OwnerDecision::Routine(OwnerRoutine::RewardPolicyStep) => {
            println!("  owner_decision=unexpected_reward_policy_step");
        }
        OwnerDecision::Routine(OwnerRoutine::ForcedTransition(kind)) => {
            println!("  owner_decision=unexpected_forced_transition kind={kind:?}");
        }
        OwnerDecision::Candidates(choices) => {
            println!("  owner_decision=candidates count={}", choices.len());
            for choice in choices {
                println!(
                    "    choice id={} key={:?} label={} action={:?}",
                    choice.candidate_id, choice.key, choice.label, choice.action
                );
            }
        }
        OwnerDecision::Gap(reason) => {
            println!("  owner_decision=gap {reason}");
        }
    }
    Ok(())
}
