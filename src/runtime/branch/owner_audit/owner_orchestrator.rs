use sts_simulator::eval::run_control::{
    build_decision_surface, RunControlAutoAppliedKindV1, RunControlAutoAppliedStepV1,
    RunControlSession,
};

use super::owner_model::OwnerDecision;
use super::owner_routines::apply_owner_routine;
use super::{BranchStatus, Owner};

const OWNER_ROUTINE_STEP_LIMIT: usize = 16;

pub(super) enum OwnerOrchestration {
    StopAtCandidates,
    Stop(BranchStatus),
    AppliedRoutine(RunControlAutoAppliedStepV1),
}

pub(super) fn orchestrate_owner_boundary(
    session: &mut RunControlSession,
    owner: Owner,
    policy_steps: &mut usize,
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
        OwnerDecision::Routine(routine) => {
            *policy_steps += 1;
            if *policy_steps > OWNER_ROUTINE_STEP_LIMIT {
                return OwnerOrchestration::Stop(BranchStatus::BudgetGap {
                    boundary: surface.view.header.title.clone(),
                    reason: "owner routine step budget exhausted".to_string(),
                });
            }
            match apply_owner_routine(session, routine) {
                Ok(outcome) => OwnerOrchestration::AppliedRoutine(RunControlAutoAppliedStepV1 {
                    kind: RunControlAutoAppliedKindV1::OwnerRoutine,
                    label: format!("owner routine {owner:?}"),
                    action_result: outcome.action_result,
                    route_decision_packet: None,
                }),
                Err(err) => OwnerOrchestration::Stop(BranchStatus::AdvanceFailed(format!(
                    "owner routine {owner:?} failed: {err}"
                ))),
            }
        }
    }
}
