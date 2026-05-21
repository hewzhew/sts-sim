use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::engine::core::{is_smoke_escape_stable_boundary, tick_engine};
use crate::runtime::combat::CombatState;
use crate::sim::combat_legal_actions::legal_moves_for_audit;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use crate::app::decision_env::{ActionId, DecisionId};

pub const NEUTRAL_ENGINE_QUERY_VERSION: &str = "neutral_engine_query_v0";

#[derive(Clone, Debug)]
pub struct SearchExecutionContext {
    pub decision_id: DecisionId,
    pub engine: EngineState,
    pub combat: CombatState,
    pub candidates: Vec<ClientInput>,
}

impl SearchExecutionContext {
    pub fn new(
        decision_id: DecisionId,
        engine: EngineState,
        combat: CombatState,
        candidates: Vec<ClientInput>,
    ) -> Self {
        Self {
            decision_id,
            engine,
            combat,
            candidates,
        }
    }

    pub fn candidate(&self, action_id: ActionId) -> Option<&ClientInput> {
        self.candidates.get(action_id.0)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NeutralQueryKind {
    OneStepTransition,
    StableTransition,
    CurrentTurnClose,
    AlignedBoundary,
    BranchCompression,
    PairedCompare,
    CommutationProbe,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BoundaryKind {
    OneStep,
    Stable,
    CurrentTurnClose,
    AlignedStable,
    CombatEnd,
    GameOver,
    PendingChoice,
    StepLimit,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Observability {
    PublicTransition,
    EngineOutcome,
    FutureSample,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EngineStepLimit {
    pub max_engine_steps: u32,
}

impl Default for EngineStepLimit {
    fn default() -> Self {
        Self {
            max_engine_steps: 240,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CombatStateSummary {
    pub engine_state: String,
    pub player_hp: i32,
    pub player_block: i32,
    pub energy: u8,
    pub enemy_total_hp: i32,
    pub living_enemy_count: usize,
    pub hand_len: usize,
    pub draw_len: usize,
    pub discard_len: usize,
    pub exhaust_len: usize,
    pub pending_choice: bool,
    pub combat_cleared: bool,
    pub player_dead: bool,
}

impl CombatStateSummary {
    pub fn from_state(engine: &EngineState, combat: &CombatState) -> Self {
        Self {
            engine_state: format!("{engine:?}"),
            player_hp: combat.entities.player.current_hp,
            player_block: combat.entities.player.block,
            energy: combat.turn.energy,
            enemy_total_hp: enemy_total_hp(combat),
            living_enemy_count: living_enemy_count(combat),
            hand_len: combat.zones.hand.len(),
            draw_len: combat.zones.draw_pile.len(),
            discard_len: combat.zones.discard_pile.len(),
            exhaust_len: combat.zones.exhaust_pile.len(),
            pending_choice: matches!(engine, EngineState::PendingChoice(_)),
            combat_cleared: combat_cleared(engine, combat),
            player_dead: matches!(engine, EngineState::GameOver(_))
                || combat.entities.player.current_hp <= 0,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct TransitionDelta {
    pub player_hp_delta: i32,
    pub player_block_delta: i32,
    pub energy_delta: i16,
    pub enemy_total_hp_delta: i32,
    pub living_enemy_delta: i32,
    pub hand_len_delta: i32,
    pub draw_len_delta: i32,
    pub discard_len_delta: i32,
    pub exhaust_len_delta: i32,
}

impl TransitionDelta {
    pub fn between(before: &CombatStateSummary, after: &CombatStateSummary) -> Self {
        Self {
            player_hp_delta: after.player_hp - before.player_hp,
            player_block_delta: after.player_block - before.player_block,
            energy_delta: after.energy as i16 - before.energy as i16,
            enemy_total_hp_delta: after.enemy_total_hp - before.enemy_total_hp,
            living_enemy_delta: after.living_enemy_count as i32 - before.living_enemy_count as i32,
            hand_len_delta: after.hand_len as i32 - before.hand_len as i32,
            draw_len_delta: after.draw_len as i32 - before.draw_len as i32,
            discard_len_delta: after.discard_len as i32 - before.discard_len as i32,
            exhaust_len_delta: after.exhaust_len as i32 - before.exhaust_len as i32,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct BranchEffectSignature {
    pub player_dead: bool,
    pub combat_cleared: bool,
    pub pending_choice: bool,
    pub hp_loss_bucket: i32,
    pub enemy_damage_bucket: i32,
    pub kills: i32,
    pub energy_left: u8,
    pub hand_delta_bucket: i32,
    pub draw_delta_bucket: i32,
    pub exhaust_delta_bucket: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BranchEffectVector {
    pub player_dead: bool,
    pub combat_cleared: bool,
    pub pending_choice_created: bool,
    pub hp_lost: i32,
    pub enemy_hp_removed: i32,
    pub enemies_killed: i32,
    pub energy_left: u8,
    pub hand_len_delta: i32,
    pub draw_len_delta: i32,
    pub discard_len_delta: i32,
    pub exhaust_len_delta: i32,
    pub signature: BranchEffectSignature,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct BranchEffectGroup {
    pub group_id: usize,
    pub signature: BranchEffectSignature,
    pub count: usize,
    pub action_ids: Vec<ActionId>,
    pub representative_action_id: ActionId,
    pub representative_input_debug: String,
}

impl BranchEffectVector {
    pub fn from_transition(before: &CombatStateSummary, after: &CombatStateSummary) -> Self {
        let delta = TransitionDelta::between(before, after);
        let hp_lost = (-delta.player_hp_delta).max(0);
        let enemy_hp_removed = (-delta.enemy_total_hp_delta).max(0);
        let enemies_killed = (-delta.living_enemy_delta).max(0);
        let signature = BranchEffectSignature {
            player_dead: after.player_dead,
            combat_cleared: after.combat_cleared,
            pending_choice: after.pending_choice,
            hp_loss_bucket: bucket(hp_lost, 5),
            enemy_damage_bucket: bucket(enemy_hp_removed, 5),
            kills: enemies_killed,
            energy_left: after.energy,
            hand_delta_bucket: bucket(delta.hand_len_delta, 2),
            draw_delta_bucket: bucket(delta.draw_len_delta, 2),
            exhaust_delta_bucket: bucket(delta.exhaust_len_delta, 1),
        };
        Self {
            player_dead: after.player_dead,
            combat_cleared: after.combat_cleared,
            pending_choice_created: !before.pending_choice && after.pending_choice,
            hp_lost,
            enemy_hp_removed,
            enemies_killed,
            energy_left: after.energy,
            hand_len_delta: delta.hand_len_delta,
            draw_len_delta: delta.draw_len_delta,
            discard_len_delta: delta.discard_len_delta,
            exhaust_len_delta: delta.exhaust_len_delta,
            signature,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct NeutralEngineQueryResult {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub action_id: ActionId,
    pub query_kind: NeutralQueryKind,
    pub input_debug: String,
    pub scenario_debug: Option<String>,
    pub alive: bool,
    pub truncated: bool,
    pub engine_steps: u32,
    pub max_engine_steps: u32,
    pub boundary_kind: BoundaryKind,
    pub observability: Observability,
    pub before: CombatStateSummary,
    pub after: CombatStateSummary,
    pub delta: TransitionDelta,
    pub branch_effect: BranchEffectVector,
}

pub struct NeutralEngineQueryService {
    pub step_limit: EngineStepLimit,
}

impl Default for NeutralEngineQueryService {
    fn default() -> Self {
        Self {
            step_limit: EngineStepLimit::default(),
        }
    }
}

impl NeutralEngineQueryService {
    pub fn force_one_step(
        &self,
        context: &SearchExecutionContext,
        action_id: ActionId,
    ) -> Option<NeutralEngineQueryResult> {
        let input = context.candidate(action_id)?.clone();
        let mut engine = context.engine.clone();
        let mut combat = context.combat.clone();
        let before = CombatStateSummary::from_state(&engine, &combat);
        let alive = tick_engine(&mut engine, &mut combat, Some(input.clone()));
        let after = CombatStateSummary::from_state(&engine, &combat);
        Some(build_result(
            context,
            action_id,
            NeutralQueryKind::OneStepTransition,
            input,
            alive,
            false,
            1,
            1,
            BoundaryKind::OneStep,
            Observability::EngineOutcome,
            before,
            after,
        ))
    }

    pub fn force_to_stable(
        &self,
        context: &SearchExecutionContext,
        action_id: ActionId,
    ) -> Option<NeutralEngineQueryResult> {
        let input = context.candidate(action_id)?.clone();
        let mut engine = context.engine.clone();
        let mut combat = context.combat.clone();
        let before = CombatStateSummary::from_state(&engine, &combat);
        let advance = force_input_to_stable(
            &mut engine,
            &mut combat,
            input.clone(),
            self.step_limit.max_engine_steps,
        );
        let after = CombatStateSummary::from_state(&engine, &combat);
        Some(build_result(
            context,
            action_id,
            NeutralQueryKind::StableTransition,
            input,
            advance.alive,
            advance.truncated,
            advance.engine_steps,
            self.step_limit.max_engine_steps,
            boundary_kind(&engine, &combat, advance.truncated),
            Observability::EngineOutcome,
            before,
            after,
        ))
    }

    pub fn force_to_current_turn_close(
        &self,
        context: &SearchExecutionContext,
        action_id: ActionId,
    ) -> Option<NeutralEngineQueryResult> {
        let mut result = self.force_to_stable(context, action_id)?;
        result.query_kind = NeutralQueryKind::CurrentTurnClose;
        result.boundary_kind = BoundaryKind::CurrentTurnClose;
        Some(result)
    }

    pub fn force_to_aligned_boundary(
        &self,
        context: &SearchExecutionContext,
        action_id: ActionId,
        boundary_kind: BoundaryKind,
    ) -> Option<NeutralEngineQueryResult> {
        let mut result = self.force_to_stable(context, action_id)?;
        result.query_kind = NeutralQueryKind::AlignedBoundary;
        result.boundary_kind = if result.truncated {
            BoundaryKind::StepLimit
        } else {
            boundary_kind
        };
        Some(result)
    }

    pub fn paired_compare(
        &self,
        context: &SearchExecutionContext,
        left: ActionId,
        right: ActionId,
    ) -> Option<PairedCandidateCompare> {
        let left_result = self.force_to_stable(context, left)?;
        let right_result = self.force_to_stable(context, right)?;
        Some(PairedCandidateCompare::from_results(
            left_result,
            right_result,
        ))
    }

    pub fn commutation_probe(
        &self,
        context: &SearchExecutionContext,
        left: ActionId,
        right: ActionId,
    ) -> Option<CommutationProbeResult> {
        let left_then_right = self.force_sequence_to_stable(context, left, right);
        let right_then_left = self.force_sequence_to_stable(context, right, left);
        Some(CommutationProbeResult::from_sequences(
            context.decision_id.clone(),
            left,
            right,
            left_then_right,
            right_then_left,
        ))
    }

    pub fn isolated_enemy_response_public_probe(
        &self,
        context: &SearchExecutionContext,
        left: ActionId,
        right: ActionId,
    ) -> Option<EnemyResponsePublicProbeResult> {
        let left_result = self.force_then_enemy_response_public(context, left)?;
        let right_result = self.force_then_enemy_response_public(context, right)?;
        Some(EnemyResponsePublicProbeResult::from_branches(
            context.decision_id.clone(),
            left,
            right,
            left_result,
            right_result,
        ))
    }

    pub fn enemy_response_public_probe(
        &self,
        context: &SearchExecutionContext,
        left: ActionId,
        right: ActionId,
    ) -> Option<EnemyResponsePublicProbeResult> {
        self.isolated_enemy_response_public_probe(context, left, right)
    }

    pub fn aligned_enemy_response_public_probe(
        &self,
        context: &SearchExecutionContext,
        left: ActionId,
        right: ActionId,
    ) -> Option<AlignedEnemyResponsePublicProbeResult> {
        let left_result = self.force_sequence_then_enemy_response_public(context, left, right)?;
        let right_result = self.force_sequence_then_enemy_response_public(context, right, left)?;
        Some(AlignedEnemyResponsePublicProbeResult::from_branches(
            context.decision_id.clone(),
            left,
            right,
            left_result,
            right_result,
        ))
    }

    fn force_then_enemy_response_public(
        &self,
        context: &SearchExecutionContext,
        action_id: ActionId,
    ) -> Option<EnemyResponseBranchSummary> {
        let input = context.candidate(action_id)?.clone();
        let mut engine = context.engine.clone();
        let mut combat = context.combat.clone();
        let before = CombatStateSummary::from_state(&engine, &combat);
        let first_advance = force_input_to_stable(
            &mut engine,
            &mut combat,
            input,
            self.step_limit.max_engine_steps,
        );
        if first_advance.truncated
            || !first_advance.alive
            || matches!(engine, EngineState::GameOver(_))
            || combat_cleared(&engine, &combat)
            || matches!(engine, EngineState::PendingChoice(_))
        {
            let after = CombatStateSummary::from_state(&engine, &combat);
            return Some(EnemyResponseBranchSummary::from_summary(
                context,
                action_id,
                before,
                after,
                false,
                first_advance.truncated,
                first_advance.engine_steps,
                if matches!(engine, EngineState::PendingChoice(_)) {
                    Some("pending_choice_before_enemy_response")
                } else {
                    None
                },
            ));
        }
        let legal = legal_moves_for_audit(&engine, &combat);
        if !legal.contains(&ClientInput::EndTurn) {
            let after = CombatStateSummary::from_state(&engine, &combat);
            return Some(EnemyResponseBranchSummary::from_summary(
                context,
                action_id,
                before,
                after,
                false,
                false,
                first_advance.engine_steps,
                Some("end_turn_not_legal_after_candidate"),
            ));
        }
        let remaining = self
            .step_limit
            .max_engine_steps
            .saturating_sub(first_advance.engine_steps)
            .max(1);
        let enemy_advance =
            force_input_to_stable(&mut engine, &mut combat, ClientInput::EndTurn, remaining);
        let after = CombatStateSummary::from_state(&engine, &combat);
        Some(EnemyResponseBranchSummary::from_summary(
            context,
            action_id,
            before,
            after,
            true,
            enemy_advance.truncated,
            first_advance
                .engine_steps
                .saturating_add(enemy_advance.engine_steps),
            None,
        ))
    }

    fn force_sequence_then_enemy_response_public(
        &self,
        context: &SearchExecutionContext,
        first: ActionId,
        suffix: ActionId,
    ) -> Option<AlignedEnemyResponseBranchSummary> {
        let first_input = context.candidate(first)?.clone();
        let suffix_input = context.candidate(suffix)?.clone();
        let mut engine = context.engine.clone();
        let mut combat = context.combat.clone();
        let before = CombatStateSummary::from_state(&engine, &combat);
        let first_advance = force_input_to_stable(
            &mut engine,
            &mut combat,
            first_input,
            self.step_limit.max_engine_steps,
        );
        if first_advance.truncated
            || !first_advance.alive
            || matches!(engine, EngineState::GameOver(_))
            || combat_cleared(&engine, &combat)
            || matches!(engine, EngineState::PendingChoice(_))
        {
            let after = CombatStateSummary::from_state(&engine, &combat);
            let failure_reason = if matches!(engine, EngineState::PendingChoice(_)) {
                Some("pending_choice_before_suffix")
            } else if first_advance.truncated {
                Some("first_action_truncated")
            } else {
                Some("first_action_terminal_before_suffix")
            };
            return Some(AlignedEnemyResponseBranchSummary::from_summary(
                context,
                first,
                suffix,
                before,
                after,
                false,
                false,
                false,
                first_advance.truncated,
                first_advance.engine_steps,
                failure_reason,
            ));
        }

        let legal_after_first = legal_moves_for_audit(&engine, &combat);
        let Some(remapped_suffix) =
            remap_input_after_state(&suffix_input, &context.combat, &combat)
        else {
            let after = CombatStateSummary::from_state(&engine, &combat);
            return Some(AlignedEnemyResponseBranchSummary::from_summary(
                context,
                first,
                suffix,
                before,
                after,
                false,
                false,
                false,
                false,
                first_advance.engine_steps,
                Some("suffix_action_remap_failed"),
            ));
        };
        if !legal_after_first.contains(&remapped_suffix) {
            let after = CombatStateSummary::from_state(&engine, &combat);
            return Some(AlignedEnemyResponseBranchSummary::from_summary(
                context,
                first,
                suffix,
                before,
                after,
                false,
                false,
                false,
                false,
                first_advance.engine_steps,
                Some("suffix_action_illegal_after_first"),
            ));
        }

        let remaining_after_first = self
            .step_limit
            .max_engine_steps
            .saturating_sub(first_advance.engine_steps)
            .max(1);
        let suffix_advance = force_input_to_stable(
            &mut engine,
            &mut combat,
            remapped_suffix,
            remaining_after_first,
        );
        let steps_after_suffix = first_advance
            .engine_steps
            .saturating_add(suffix_advance.engine_steps);
        if suffix_advance.truncated
            || !suffix_advance.alive
            || matches!(engine, EngineState::GameOver(_))
            || combat_cleared(&engine, &combat)
            || matches!(engine, EngineState::PendingChoice(_))
        {
            let after = CombatStateSummary::from_state(&engine, &combat);
            let failure_reason = if matches!(engine, EngineState::PendingChoice(_)) {
                Some("pending_choice_before_enemy_response")
            } else if suffix_advance.truncated {
                Some("suffix_action_truncated")
            } else {
                Some("suffix_action_terminal_before_enemy_response")
            };
            return Some(AlignedEnemyResponseBranchSummary::from_summary(
                context,
                first,
                suffix,
                before,
                after,
                true,
                true,
                false,
                suffix_advance.truncated,
                steps_after_suffix,
                failure_reason,
            ));
        }

        let legal_after_suffix = legal_moves_for_audit(&engine, &combat);
        if !legal_after_suffix.contains(&ClientInput::EndTurn) {
            let after = CombatStateSummary::from_state(&engine, &combat);
            return Some(AlignedEnemyResponseBranchSummary::from_summary(
                context,
                first,
                suffix,
                before,
                after,
                true,
                true,
                false,
                false,
                steps_after_suffix,
                Some("end_turn_not_legal_after_suffix"),
            ));
        }
        let remaining = self
            .step_limit
            .max_engine_steps
            .saturating_sub(steps_after_suffix)
            .max(1);
        let enemy_advance =
            force_input_to_stable(&mut engine, &mut combat, ClientInput::EndTurn, remaining);
        let after = CombatStateSummary::from_state(&engine, &combat);
        Some(AlignedEnemyResponseBranchSummary::from_summary(
            context,
            first,
            suffix,
            before,
            after,
            true,
            true,
            true,
            enemy_advance.truncated,
            steps_after_suffix.saturating_add(enemy_advance.engine_steps),
            None,
        ))
    }

    fn force_sequence_to_stable(
        &self,
        context: &SearchExecutionContext,
        first: ActionId,
        second: ActionId,
    ) -> CommutationSequenceSummary {
        let Some(first_input) = context.candidate(first).cloned() else {
            return CommutationSequenceSummary::illegal(first, second, "missing_first_candidate");
        };
        let Some(second_input) = context.candidate(second).cloned() else {
            return CommutationSequenceSummary::illegal(first, second, "missing_second_candidate");
        };
        let mut engine = context.engine.clone();
        let mut combat = context.combat.clone();
        let before = CombatStateSummary::from_state(&engine, &combat);
        let first_advance = force_input_to_stable(
            &mut engine,
            &mut combat,
            first_input,
            self.step_limit.max_engine_steps,
        );
        if first_advance.truncated || !first_advance.alive {
            let after = CombatStateSummary::from_state(&engine, &combat);
            return CommutationSequenceSummary::after_first_only(
                context,
                first,
                second,
                false,
                first_advance.truncated,
                first_advance.engine_steps,
                before,
                after,
                "first_action_terminal_or_truncated",
            );
        }
        let legal = legal_moves_for_audit(&engine, &combat);
        let Some(remapped_second) =
            remap_input_after_state(&second_input, &context.combat, &combat)
        else {
            let after = CombatStateSummary::from_state(&engine, &combat);
            return CommutationSequenceSummary::after_first_only(
                context,
                first,
                second,
                false,
                false,
                first_advance.engine_steps,
                before,
                after,
                "second_action_remap_failed",
            );
        };
        if !legal.contains(&remapped_second) {
            let after = CombatStateSummary::from_state(&engine, &combat);
            return CommutationSequenceSummary::after_first_only(
                context,
                first,
                second,
                false,
                false,
                first_advance.engine_steps,
                before,
                after,
                "second_action_illegal_after_first",
            );
        }
        let remaining = self
            .step_limit
            .max_engine_steps
            .saturating_sub(first_advance.engine_steps)
            .max(1);
        let second_advance =
            force_input_to_stable(&mut engine, &mut combat, remapped_second, remaining);
        let after = CombatStateSummary::from_state(&engine, &combat);
        CommutationSequenceSummary::completed(
            context,
            first,
            second,
            first_advance
                .engine_steps
                .saturating_add(second_advance.engine_steps),
            second_advance.truncated,
            before,
            after,
        )
    }

    pub fn branch_effect_evidence(
        &self,
        context: &SearchExecutionContext,
        action_ids: &[ActionId],
    ) -> Vec<NeutralEngineQueryResult> {
        action_ids
            .iter()
            .filter_map(|action_id| self.force_to_stable(context, *action_id))
            .map(|mut result| {
                result.query_kind = NeutralQueryKind::BranchCompression;
                result
            })
            .collect()
    }

    pub fn draw_top_card_branch_effects(
        &self,
        context: &SearchExecutionContext,
        action_id: ActionId,
        max_branches: usize,
    ) -> Vec<NeutralEngineQueryResult> {
        let branch_count = context.combat.zones.draw_pile.len().min(max_branches);
        (0..branch_count)
            .filter_map(|draw_index| {
                let mut branch_combat = context.combat.clone();
                let card = branch_combat.zones.draw_pile.remove(draw_index);
                let scenario_debug =
                    format!("draw_top_card_sample/index:{draw_index}/card:{:?}", card.id);
                branch_combat.add_card_to_draw_pile_top(card);
                let branch_context = SearchExecutionContext {
                    decision_id: context.decision_id.clone(),
                    engine: context.engine.clone(),
                    combat: branch_combat,
                    candidates: context.candidates.clone(),
                };
                let mut result = self.force_to_stable(&branch_context, action_id)?;
                result.query_kind = NeutralQueryKind::BranchCompression;
                result.observability = Observability::FutureSample;
                result.scenario_debug = Some(scenario_debug);
                Some(result)
            })
            .collect()
    }

    pub fn compress_branch_effects(
        &self,
        results: &[NeutralEngineQueryResult],
    ) -> Vec<BranchEffectGroup> {
        compress_branch_effects(results)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct PairedCandidateCompare {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub left: NeutralEngineQueryResult,
    pub right: NeutralEngineQueryResult,
    pub hp_lost_diff_left_minus_right: i32,
    pub enemy_removed_diff_left_minus_right: i32,
    pub kill_diff_left_minus_right: i32,
    pub left_dead_right_alive: bool,
    pub left_alive_right_dead: bool,
    pub left_clears_right_not: bool,
    pub right_clears_left_not: bool,
}

impl PairedCandidateCompare {
    pub fn from_results(
        mut left: NeutralEngineQueryResult,
        mut right: NeutralEngineQueryResult,
    ) -> Self {
        left.query_kind = NeutralQueryKind::PairedCompare;
        right.query_kind = NeutralQueryKind::PairedCompare;
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id: left.decision_id.clone(),
            hp_lost_diff_left_minus_right: left.branch_effect.hp_lost - right.branch_effect.hp_lost,
            enemy_removed_diff_left_minus_right: left.branch_effect.enemy_hp_removed
                - right.branch_effect.enemy_hp_removed,
            kill_diff_left_minus_right: left.branch_effect.enemies_killed
                - right.branch_effect.enemies_killed,
            left_dead_right_alive: left.branch_effect.player_dead
                && !right.branch_effect.player_dead,
            left_alive_right_dead: !left.branch_effect.player_dead
                && right.branch_effect.player_dead,
            left_clears_right_not: left.branch_effect.combat_cleared
                && !right.branch_effect.combat_cleared,
            right_clears_left_not: !left.branch_effect.combat_cleared
                && right.branch_effect.combat_cleared,
            left,
            right,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CommutationSequenceSummary {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub first_action_id: ActionId,
    pub second_action_id: ActionId,
    pub second_action_legal: bool,
    pub reached_boundary: bool,
    pub truncated: bool,
    pub engine_steps: u32,
    pub boundary_kind: BoundaryKind,
    pub failure_reason: Option<String>,
    pub after: CombatStateSummary,
    pub branch_effect: BranchEffectVector,
}

impl CommutationSequenceSummary {
    fn illegal(first: ActionId, second: ActionId, reason: impl Into<String>) -> Self {
        let empty = CombatStateSummary {
            engine_state: "unavailable".to_string(),
            player_hp: 0,
            player_block: 0,
            energy: 0,
            enemy_total_hp: 0,
            living_enemy_count: 0,
            hand_len: 0,
            draw_len: 0,
            discard_len: 0,
            exhaust_len: 0,
            pending_choice: false,
            combat_cleared: false,
            player_dead: false,
        };
        let effect = BranchEffectVector::from_transition(&empty, &empty);
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id: DecisionId {
                episode_id: "missing_context".to_string(),
                step_index: 0,
                decision_type: "missing_context".to_string(),
            },
            first_action_id: first,
            second_action_id: second,
            second_action_legal: false,
            reached_boundary: false,
            truncated: false,
            engine_steps: 0,
            boundary_kind: BoundaryKind::StepLimit,
            failure_reason: Some(reason.into()),
            after: empty,
            branch_effect: effect,
        }
    }

    fn after_first_only(
        context: &SearchExecutionContext,
        first: ActionId,
        second: ActionId,
        second_action_legal: bool,
        truncated: bool,
        engine_steps: u32,
        before: CombatStateSummary,
        after: CombatStateSummary,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id: context.decision_id.clone(),
            first_action_id: first,
            second_action_id: second,
            second_action_legal,
            reached_boundary: false,
            truncated,
            engine_steps,
            boundary_kind: summary_boundary_kind(&after, truncated),
            failure_reason: Some(reason.into()),
            branch_effect: BranchEffectVector::from_transition(&before, &after),
            after,
        }
    }

    fn completed(
        context: &SearchExecutionContext,
        first: ActionId,
        second: ActionId,
        engine_steps: u32,
        truncated: bool,
        before: CombatStateSummary,
        after: CombatStateSummary,
    ) -> Self {
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id: context.decision_id.clone(),
            first_action_id: first,
            second_action_id: second,
            second_action_legal: true,
            reached_boundary: !truncated,
            truncated,
            engine_steps,
            boundary_kind: summary_boundary_kind(&after, truncated),
            failure_reason: None,
            branch_effect: BranchEffectVector::from_transition(&before, &after),
            after,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct CommutationProbeResult {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub left_action_id: ActionId,
    pub right_action_id: ActionId,
    pub left_then_right_legal: bool,
    pub right_then_left_legal: bool,
    pub both_orders_reached_boundary: bool,
    pub summary_equal: bool,
    pub hp_loss_diff: i32,
    pub enemy_removed_diff: i32,
    pub kill_diff: i32,
    pub terminal_diff: bool,
    pub order_only_equivalent: bool,
    pub left_then_right: CommutationSequenceSummary,
    pub right_then_left: CommutationSequenceSummary,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnemyResponsePublicSummary {
    pub player_hp: i32,
    pub player_block: i32,
    pub enemy_total_hp: i32,
    pub living_enemy_count: usize,
    pub pending_choice: bool,
    pub combat_cleared: bool,
    pub player_dead: bool,
    pub redacted_fields: Vec<String>,
}

impl EnemyResponsePublicSummary {
    fn from_summary(summary: &CombatStateSummary) -> Self {
        Self {
            player_hp: summary.player_hp,
            player_block: summary.player_block,
            enemy_total_hp: summary.enemy_total_hp,
            living_enemy_count: summary.living_enemy_count,
            pending_choice: summary.pending_choice,
            combat_cleared: summary.combat_cleared,
            player_dead: summary.player_dead,
            redacted_fields: vec![
                "hand".to_string(),
                "draw_pile".to_string(),
                "discard_pile".to_string(),
                "exhaust_pile".to_string(),
            ],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnemyResponseBranchSummary {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub action_id: ActionId,
    pub end_turn_applied: bool,
    pub reached_public_boundary: bool,
    pub truncated: bool,
    pub engine_steps: u32,
    pub boundary_kind: BoundaryKind,
    pub failure_reason: Option<String>,
    pub summary: EnemyResponsePublicSummary,
    pub public_delta: EnemyResponsePublicDelta,
}

impl EnemyResponseBranchSummary {
    fn from_summary(
        context: &SearchExecutionContext,
        action_id: ActionId,
        before: CombatStateSummary,
        after: CombatStateSummary,
        end_turn_applied: bool,
        truncated: bool,
        engine_steps: u32,
        failure_reason: Option<&'static str>,
    ) -> Self {
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id: context.decision_id.clone(),
            action_id,
            end_turn_applied,
            reached_public_boundary: !truncated && failure_reason.is_none(),
            truncated,
            engine_steps,
            boundary_kind: summary_boundary_kind(&after, truncated),
            failure_reason: failure_reason.map(str::to_string),
            summary: EnemyResponsePublicSummary::from_summary(&after),
            public_delta: EnemyResponsePublicDelta::from_transition(&before, &after),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnemyResponsePublicDelta {
    pub hp_lost: i32,
    pub enemy_hp_removed: i32,
    pub enemies_killed: i32,
    pub player_dead: bool,
    pub combat_cleared: bool,
}

impl EnemyResponsePublicDelta {
    fn from_transition(before: &CombatStateSummary, after: &CombatStateSummary) -> Self {
        let effect = BranchEffectVector::from_transition(before, after);
        Self {
            hp_lost: effect.hp_lost,
            enemy_hp_removed: effect.enemy_hp_removed,
            enemies_killed: effect.enemies_killed,
            player_dead: effect.player_dead,
            combat_cleared: effect.combat_cleared,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct EnemyResponsePublicProbeResult {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub left_action_id: ActionId,
    pub right_action_id: ActionId,
    pub left: EnemyResponseBranchSummary,
    pub right: EnemyResponseBranchSummary,
    pub hp_lost_diff_left_minus_right: i32,
    pub enemy_removed_diff_left_minus_right: i32,
    pub kill_diff_left_minus_right: i32,
    pub left_dead_right_alive: bool,
    pub left_alive_right_dead: bool,
    pub left_clears_right_not: bool,
    pub right_clears_left_not: bool,
    pub public_safe: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AlignedEnemyResponseBranchSummary {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub first_action_id: ActionId,
    pub suffix_action_id: ActionId,
    pub suffix_action_legal: bool,
    pub suffix_action_applied: bool,
    pub end_turn_applied: bool,
    pub reached_public_boundary: bool,
    pub truncated: bool,
    pub engine_steps: u32,
    pub boundary_kind: BoundaryKind,
    pub failure_reason: Option<String>,
    pub summary: EnemyResponsePublicSummary,
    pub public_delta: EnemyResponsePublicDelta,
}

impl AlignedEnemyResponseBranchSummary {
    fn from_summary(
        context: &SearchExecutionContext,
        first_action_id: ActionId,
        suffix_action_id: ActionId,
        before: CombatStateSummary,
        after: CombatStateSummary,
        suffix_action_legal: bool,
        suffix_action_applied: bool,
        end_turn_applied: bool,
        truncated: bool,
        engine_steps: u32,
        failure_reason: Option<&'static str>,
    ) -> Self {
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id: context.decision_id.clone(),
            first_action_id,
            suffix_action_id,
            suffix_action_legal,
            suffix_action_applied,
            end_turn_applied,
            reached_public_boundary: !truncated && failure_reason.is_none(),
            truncated,
            engine_steps,
            boundary_kind: summary_boundary_kind(&after, truncated),
            failure_reason: failure_reason.map(str::to_string),
            summary: EnemyResponsePublicSummary::from_summary(&after),
            public_delta: EnemyResponsePublicDelta::from_transition(&before, &after),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
pub struct AlignedEnemyResponsePublicProbeResult {
    pub schema_version: String,
    pub decision_id: DecisionId,
    pub left_action_id: ActionId,
    pub right_action_id: ActionId,
    pub left: AlignedEnemyResponseBranchSummary,
    pub right: AlignedEnemyResponseBranchSummary,
    pub hp_lost_diff_left_minus_right: i32,
    pub enemy_removed_diff_left_minus_right: i32,
    pub kill_diff_left_minus_right: i32,
    pub left_dead_right_alive: bool,
    pub left_alive_right_dead: bool,
    pub left_clears_right_not: bool,
    pub right_clears_left_not: bool,
    pub summary_equal: bool,
    pub public_safe: bool,
}

impl AlignedEnemyResponsePublicProbeResult {
    fn from_branches(
        decision_id: DecisionId,
        left_action_id: ActionId,
        right_action_id: ActionId,
        left: AlignedEnemyResponseBranchSummary,
        right: AlignedEnemyResponseBranchSummary,
    ) -> Self {
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id,
            left_action_id,
            right_action_id,
            hp_lost_diff_left_minus_right: left.public_delta.hp_lost - right.public_delta.hp_lost,
            enemy_removed_diff_left_minus_right: left.public_delta.enemy_hp_removed
                - right.public_delta.enemy_hp_removed,
            kill_diff_left_minus_right: left.public_delta.enemies_killed
                - right.public_delta.enemies_killed,
            left_dead_right_alive: left.public_delta.player_dead && !right.public_delta.player_dead,
            left_alive_right_dead: !left.public_delta.player_dead && right.public_delta.player_dead,
            left_clears_right_not: left.public_delta.combat_cleared
                && !right.public_delta.combat_cleared,
            right_clears_left_not: !left.public_delta.combat_cleared
                && right.public_delta.combat_cleared,
            summary_equal: left.summary == right.summary,
            public_safe: left
                .summary
                .redacted_fields
                .iter()
                .any(|field| field == "hand")
                && right
                    .summary
                    .redacted_fields
                    .iter()
                    .any(|field| field == "hand"),
            left,
            right,
        }
    }
}

impl EnemyResponsePublicProbeResult {
    fn from_branches(
        decision_id: DecisionId,
        left_action_id: ActionId,
        right_action_id: ActionId,
        left: EnemyResponseBranchSummary,
        right: EnemyResponseBranchSummary,
    ) -> Self {
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id,
            left_action_id,
            right_action_id,
            hp_lost_diff_left_minus_right: left.public_delta.hp_lost - right.public_delta.hp_lost,
            enemy_removed_diff_left_minus_right: left.public_delta.enemy_hp_removed
                - right.public_delta.enemy_hp_removed,
            kill_diff_left_minus_right: left.public_delta.enemies_killed
                - right.public_delta.enemies_killed,
            left_dead_right_alive: left.public_delta.player_dead && !right.public_delta.player_dead,
            left_alive_right_dead: !left.public_delta.player_dead && right.public_delta.player_dead,
            left_clears_right_not: left.public_delta.combat_cleared
                && !right.public_delta.combat_cleared,
            right_clears_left_not: !left.public_delta.combat_cleared
                && right.public_delta.combat_cleared,
            public_safe: left
                .summary
                .redacted_fields
                .iter()
                .any(|field| field == "hand")
                && right
                    .summary
                    .redacted_fields
                    .iter()
                    .any(|field| field == "hand"),
            left,
            right,
        }
    }
}

impl CommutationProbeResult {
    fn from_sequences(
        decision_id: DecisionId,
        left: ActionId,
        right: ActionId,
        left_then_right: CommutationSequenceSummary,
        right_then_left: CommutationSequenceSummary,
    ) -> Self {
        let left_then_right_legal = left_then_right.second_action_legal;
        let right_then_left_legal = right_then_left.second_action_legal;
        let both_orders_reached_boundary =
            left_then_right.reached_boundary && right_then_left.reached_boundary;
        let summary_equal =
            both_orders_reached_boundary && left_then_right.after == right_then_left.after;
        let terminal_diff = left_then_right.branch_effect.player_dead
            != right_then_left.branch_effect.player_dead
            || left_then_right.branch_effect.combat_cleared
                != right_then_left.branch_effect.combat_cleared;
        Self {
            schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
            decision_id,
            left_action_id: left,
            right_action_id: right,
            left_then_right_legal,
            right_then_left_legal,
            both_orders_reached_boundary,
            summary_equal,
            hp_loss_diff: left_then_right.branch_effect.hp_lost
                - right_then_left.branch_effect.hp_lost,
            enemy_removed_diff: left_then_right.branch_effect.enemy_hp_removed
                - right_then_left.branch_effect.enemy_hp_removed,
            kill_diff: left_then_right.branch_effect.enemies_killed
                - right_then_left.branch_effect.enemies_killed,
            terminal_diff,
            order_only_equivalent: left_then_right_legal
                && right_then_left_legal
                && summary_equal
                && !terminal_diff,
            left_then_right,
            right_then_left,
        }
    }
}

pub fn compress_branch_effects(results: &[NeutralEngineQueryResult]) -> Vec<BranchEffectGroup> {
    let mut groups: BTreeMap<BranchEffectSignature, Vec<&NeutralEngineQueryResult>> =
        BTreeMap::new();
    for result in results {
        groups
            .entry(result.branch_effect.signature.clone())
            .or_default()
            .push(result);
    }
    groups
        .into_iter()
        .enumerate()
        .filter_map(|(group_id, (signature, members))| {
            let representative = members.first()?;
            Some(BranchEffectGroup {
                group_id,
                signature,
                count: members.len(),
                action_ids: members.iter().map(|result| result.action_id).collect(),
                representative_action_id: representative.action_id,
                representative_input_debug: representative.input_debug.clone(),
            })
        })
        .collect()
}

#[derive(Clone, Copy, Debug)]
struct StableAdvance {
    alive: bool,
    truncated: bool,
    engine_steps: u32,
}

fn force_input_to_stable(
    engine: &mut EngineState,
    combat: &mut CombatState,
    input: ClientInput,
    max_engine_steps: u32,
) -> StableAdvance {
    let mut steps = 1u32;
    let alive = tick_engine(engine, combat, Some(input));
    if !alive {
        return StableAdvance {
            alive: false,
            truncated: false,
            engine_steps: steps,
        };
    }
    normalize_player_turn_processing(engine, combat);
    loop {
        if stable_boundary(engine, combat) {
            return StableAdvance {
                alive: !matches!(engine, EngineState::GameOver(_)),
                truncated: false,
                engine_steps: steps,
            };
        }
        if steps >= max_engine_steps.max(1) {
            return StableAdvance {
                alive: true,
                truncated: true,
                engine_steps: steps,
            };
        }
        let alive = tick_engine(engine, combat, None);
        steps = steps.saturating_add(1);
        if !alive {
            return StableAdvance {
                alive: false,
                truncated: false,
                engine_steps: steps,
            };
        }
        normalize_player_turn_processing(engine, combat);
    }
}

fn build_result(
    context: &SearchExecutionContext,
    action_id: ActionId,
    query_kind: NeutralQueryKind,
    input: ClientInput,
    alive: bool,
    truncated: bool,
    engine_steps: u32,
    max_engine_steps: u32,
    boundary_kind: BoundaryKind,
    observability: Observability,
    before: CombatStateSummary,
    after: CombatStateSummary,
) -> NeutralEngineQueryResult {
    let delta = TransitionDelta::between(&before, &after);
    let branch_effect = BranchEffectVector::from_transition(&before, &after);
    NeutralEngineQueryResult {
        schema_version: NEUTRAL_ENGINE_QUERY_VERSION.to_string(),
        decision_id: context.decision_id.clone(),
        action_id,
        query_kind,
        input_debug: format!("{input:?}"),
        scenario_debug: None,
        alive,
        truncated,
        engine_steps,
        max_engine_steps,
        boundary_kind,
        observability,
        before,
        after,
        delta,
        branch_effect,
    }
}

fn remap_input_after_state(
    original: &ClientInput,
    before: &CombatState,
    after: &CombatState,
) -> Option<ClientInput> {
    match original {
        ClientInput::PlayCard { card_index, target } => {
            let uuid = before.zones.hand.get(*card_index)?.uuid;
            let new_index = after.zones.hand.iter().position(|card| card.uuid == uuid)?;
            Some(ClientInput::PlayCard {
                card_index: new_index,
                target: *target,
            })
        }
        other => Some(other.clone()),
    }
}

fn summary_boundary_kind(summary: &CombatStateSummary, truncated: bool) -> BoundaryKind {
    if truncated {
        BoundaryKind::StepLimit
    } else if summary.player_dead {
        BoundaryKind::GameOver
    } else if summary.combat_cleared {
        BoundaryKind::CombatEnd
    } else if summary.pending_choice {
        BoundaryKind::PendingChoice
    } else {
        BoundaryKind::Stable
    }
}

fn normalize_player_turn_processing(engine: &mut EngineState, combat: &CombatState) {
    if *engine == EngineState::CombatPlayerTurn
        && (combat.has_pending_actions() || !combat.zones.queued_cards.is_empty())
    {
        *engine = EngineState::CombatProcessing;
    }
}

fn stable_boundary(engine: &EngineState, combat: &CombatState) -> bool {
    matches!(engine, EngineState::CombatPlayerTurn)
        || matches!(engine, EngineState::PendingChoice(_))
        || matches!(engine, EngineState::RewardScreen(_))
        || matches!(engine, EngineState::GameOver(_))
        || is_smoke_escape_stable_boundary(engine, combat)
}

fn boundary_kind(engine: &EngineState, combat: &CombatState, truncated: bool) -> BoundaryKind {
    if truncated {
        BoundaryKind::StepLimit
    } else if matches!(engine, EngineState::GameOver(_)) {
        BoundaryKind::GameOver
    } else if combat_cleared(engine, combat) {
        BoundaryKind::CombatEnd
    } else if matches!(engine, EngineState::PendingChoice(_)) {
        BoundaryKind::PendingChoice
    } else {
        BoundaryKind::Stable
    }
}

fn enemy_total_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_escaped && !monster.half_dead && monster.current_hp > 0)
        .map(|monster| monster.current_hp)
        .sum()
}

fn living_enemy_count(combat: &CombatState) -> usize {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| {
            !monster.is_escaped && !monster.half_dead && !monster.is_dying && monster.current_hp > 0
        })
        .count()
}

fn combat_cleared(engine: &EngineState, combat: &CombatState) -> bool {
    matches!(engine, EngineState::RewardScreen(_))
        || combat
            .entities
            .monsters
            .iter()
            .all(|monster| monster.is_escaped || monster.current_hp <= 0)
}

fn bucket(value: i32, width: i32) -> i32 {
    if width <= 1 {
        value
    } else {
        value.div_euclid(width)
    }
}
