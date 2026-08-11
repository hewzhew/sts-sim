use std::time::{Duration, Instant};

use sts_combat_planner::LocalTurnGraphRootActionFamilySnapshot;

use super::combat_search::RunControlCombatWorkAdvanceV1;
use super::oracle_combat_work::OracleRunCombatWorkV1;
use super::oracle_combat_work_contract::OracleRunCombatWorkCheckpointV1;
use super::oracle_resident_combat_job_evidence::OracleResidentCombatJobEvidenceV1;
use super::progress_options::{RunControlCombatSearchQuantum, RunControlSearchCombatOptions};
use super::session::{RunControlSession, RunProgressOutcome};
use crate::eval::combat_guidance_bundle::CombatGuidanceBundleV1;
use crate::state::core::ClientInput;

/// Opaque capability over one resident exact-combat search.
///
/// Analysis and explorer orchestration may grant bounded work, inspect typed
/// evidence, checkpoint, or commit a verified result. They never receive the
/// live local-graph/discrepancy sessions or their private queues.
pub(super) struct OracleResidentCombatJobV1 {
    work: OracleRunCombatWorkV1,
}

impl OracleResidentCombatJobV1 {
    pub(super) fn new(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        OracleRunCombatWorkV1::new_with_guidance(session, options, guidance)
            .map(|work| Self { work })
    }

    pub(super) fn restore(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        checkpoint: OracleRunCombatWorkCheckpointV1,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        OracleRunCombatWorkV1::restart_from_checkpoint_with_guidance(
            session, options, checkpoint, guidance,
        )
        .map(|work| Self { work })
    }

    pub(super) fn promote(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        checkpoint: OracleRunCombatWorkCheckpointV1,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        OracleRunCombatWorkV1::restart_for_higher_fidelity_with_guidance(
            session, options, checkpoint, guidance,
        )
        .map(|work| Self { work })
    }

    pub(super) fn restart(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        OracleRunCombatWorkV1::restart_from_exact_state_with_guidance(session, options, guidance)
            .map(|work| Self { work })
    }

    pub(super) fn for_exact_actions(
        session: &RunControlSession,
        options: RunControlSearchCombatOptions,
        guidance: Option<&CombatGuidanceBundleV1>,
    ) -> Result<Self, String> {
        OracleRunCombatWorkV1::for_exact_action_witness_with_guidance(session, options, guidance)
            .map(|work| Self { work })
    }

    pub(super) fn root_action_families(&self) -> Vec<LocalTurnGraphRootActionFamilySnapshot> {
        self.work.root_action_families()
    }

    pub(super) fn checkpoint(&self) -> OracleRunCombatWorkCheckpointV1 {
        self.work.checkpoint()
    }

    pub(super) fn advance(
        &mut self,
        quantum: &RunControlCombatSearchQuantum,
        deadline: Option<Instant>,
    ) -> RunControlCombatWorkAdvanceV1 {
        self.work.advance(quantum, deadline)
    }

    pub(super) fn advance_improving_incumbent(
        &mut self,
        quantum: &RunControlCombatSearchQuantum,
        deadline: Option<Instant>,
    ) -> RunControlCombatWorkAdvanceV1 {
        self.work.advance_improving_incumbent(quantum, deadline)
    }

    pub(super) fn advance_current_stage_probe(
        &mut self,
        quantum: &RunControlCombatSearchQuantum,
        deadline: Option<Instant>,
    ) -> RunControlCombatWorkAdvanceV1 {
        self.work.advance_current_stage_probe(quantum, deadline)
    }

    pub(super) fn ensure_requested_allowance(
        &mut self,
        requested_nodes: usize,
        requested_wall_time: Option<Duration>,
    ) {
        self.work
            .ensure_requested_allowance(requested_nodes, requested_wall_time);
    }

    pub(super) fn mark_search_resume_exact(&mut self) {
        self.work.mark_search_resume_exact();
    }

    pub(super) fn search_resume_exact(&self) -> bool {
        self.work.search_resume_exact()
    }

    pub(super) fn has_verified_witness(&self) -> bool {
        self.work.has_verified_witness()
    }

    pub(super) fn verified_witness_inputs(&self) -> Option<Vec<ClientInput>> {
        self.work.verified_witness_inputs()
    }

    pub(super) fn incumbent_hp_loss(&self) -> Option<u32> {
        self.work.incumbent_hp_loss()
    }

    pub(super) fn has_refinement_ending_witness(&self) -> bool {
        self.work.has_refinement_ending_witness()
    }

    pub(super) fn verify_and_restore_action_witness(
        &mut self,
        inputs: &[ClientInput],
    ) -> Result<(), String> {
        self.work.verify_and_restore_action_witness(inputs)
    }

    pub(super) fn quantum_count(&self) -> usize {
        self.work.quantum_count()
    }

    pub(super) fn remaining_nodes(&self) -> usize {
        self.work.remaining_nodes()
    }

    pub(super) fn current_search_generation_work(&self) -> u64 {
        self.work.current_search_generation_work()
    }

    pub(super) fn remaining_wall_ms(&self) -> Option<u64> {
        self.work.remaining_wall_ms()
    }

    pub(super) fn max_potions_used(&self) -> Option<u32> {
        self.work.max_potions_used()
    }

    pub(super) fn allowed_potion_slots(&self) -> Option<u64> {
        self.work.allowed_potion_slots()
    }

    pub(super) fn restart_count(&self) -> usize {
        self.work.restart_count()
    }

    pub(super) fn evidence(&self) -> OracleResidentCombatJobEvidenceV1 {
        self.work.progress()
    }

    pub(super) fn finish_and_apply(
        &self,
        session: &mut RunControlSession,
    ) -> Result<RunProgressOutcome, String> {
        self.work.finish_and_apply(session)
    }
}
