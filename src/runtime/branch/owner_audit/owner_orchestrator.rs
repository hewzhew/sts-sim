use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlSession, RunProgressStepV1,
};

use super::owner_model::OwnerDecision;
use super::owner_routines::apply_owner_routine;
use super::{BranchStatus, Owner};

pub(super) enum OwnerOrchestration {
    StopAtCandidates,
    Stop(BranchStatus),
    AppliedRoutine(RunProgressStepV1),
}

pub(super) fn orchestrate_owner_boundary(
    session: &mut RunControlSession,
    owner: Owner,
) -> OwnerOrchestration {
    let surface = build_decision_surface(session);
    match super::owners::owner_decision(session, owner, &surface) {
        OwnerDecision::Candidates(choices) if !choices.is_empty() => {
            OwnerOrchestration::StopAtCandidates
        }
        OwnerDecision::Candidates(_) => OwnerOrchestration::Stop(BranchStatus::AdvanceFailed(
            format!("owner {owner:?} produced no candidates"),
        )),
        OwnerDecision::Gap(reason) => OwnerOrchestration::Stop(BranchStatus::AdvanceFailed(
            format!("owner {owner:?} gap: {reason}"),
        )),
        OwnerDecision::Routine(routine) => match apply_owner_routine(session, routine) {
            Ok(outcome) => {
                if !matches!(
                    outcome.progress_steps.as_slice(),
                    [RunProgressStepV1::Decision(_) | RunProgressStepV1::ForcedTransition(_)]
                ) {
                    return OwnerOrchestration::Stop(BranchStatus::AdvanceFailed(format!(
                        "owner routine {owner:?} did not produce exactly one run mutation"
                    )));
                }
                OwnerOrchestration::AppliedRoutine(outcome.progress_steps[0].clone())
            }
            Err(err) => OwnerOrchestration::Stop(BranchStatus::AdvanceFailed(format!(
                "owner routine {owner:?} failed: {err}"
            ))),
        },
    }
}
