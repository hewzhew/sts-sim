use sts_simulator::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};

use super::focus::{focus_witness_line, CombatReviewFocus};

#[path = "champ_phase/resources.rs"]
mod resources;
#[path = "champ_phase/snapshot.rs"]
mod snapshot;
#[path = "champ_phase/types.rs"]
mod types;
#[path = "champ_phase/verdict.rs"]
mod verdict;

pub(super) use types::ChampPhaseAudit;

use resources::note_champ_resource_before_split;
use snapshot::{champ_phase_snapshot, crossed_below_champ_half_hp};
use types::{ChampHpCrossing, ChampResourceTiming, ChampSplitTrigger};
use verdict::{champ_phase_flags, champ_phase_verdict};

pub(super) fn champ_phase_audit(
    root: &CombatPosition,
    focus: &CombatReviewFocus,
) -> Option<ChampPhaseAudit> {
    let initial_snapshot = champ_phase_snapshot(0, &root.combat)?;
    let witness = focus_witness_line(focus);
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut resources_before_split = ChampResourceTiming::default();
    let mut first_below_half_hp = None;
    let mut split_trigger = None;
    let mut post_split_snapshot = None;
    let mut replayed_actions = 0usize;
    let mut truncated = false;
    let mut timed_out = false;

    for (index, action) in witness.actions.iter().cloned().enumerate() {
        if stepper.terminal(&position) != CombatTerminal::Unresolved {
            break;
        }
        let step_index = index + 1;
        let before = champ_phase_snapshot(step_index - 1, &position.combat)?;
        if !before.champ_threshold_reached {
            note_champ_resource_before_split(
                &position,
                &action.input,
                step_index,
                &mut resources_before_split,
            );
        }
        let step = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: 250,
                deadline: None,
            },
        );
        replayed_actions = replayed_actions.saturating_add(1);
        truncated |= step.truncated;
        timed_out |= step.timed_out;
        if let Some(after) = champ_phase_snapshot(step_index, &step.position.combat) {
            if first_below_half_hp.is_none() && crossed_below_champ_half_hp(&before, &after) {
                first_below_half_hp = Some(ChampHpCrossing {
                    step_index,
                    action_key: action.action_key.clone(),
                    input: action.input.clone(),
                    before_champ_hp: before.champ_hp,
                    after_champ_hp: after.champ_hp,
                });
            }
            if split_trigger.is_none()
                && !before.champ_threshold_reached
                && after.champ_threshold_reached
            {
                post_split_snapshot = Some(after.clone());
                split_trigger = Some(ChampSplitTrigger {
                    step_index,
                    action_key: action.action_key,
                    input: action.input,
                    before,
                    after,
                });
            }
        }
        position = step.position;
        if truncated || timed_out || step.terminal != CombatTerminal::Unresolved {
            break;
        }
    }

    let truncated_by_preview = witness
        .action_count
        .is_some_and(|count| count > witness.actions.len());
    let flags = champ_phase_flags(
        split_trigger.as_ref(),
        post_split_snapshot.as_ref(),
        &resources_before_split,
        truncated || timed_out || truncated_by_preview,
    );
    let verdict = champ_phase_verdict(
        &flags,
        split_trigger.is_some(),
        truncated || timed_out || truncated_by_preview,
    );

    Some(ChampPhaseAudit {
        schema: "champ_phase_audit_v0",
        contract: "exact_replay_timing_snapshot_only_no_search_policy_change_no_strategy_verdict",
        basis_line: focus.selected_review,
        witness_action_count: witness.action_count,
        replayed_actions,
        truncated_by_preview,
        truncated,
        timed_out,
        initial_snapshot,
        first_below_half_hp,
        split_trigger,
        post_split_snapshot,
        resources_before_split,
        flags,
        verdict,
    })
}
