use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

use crate::engine::core::{is_smoke_escape_stable_boundary, tick_engine};
use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;
use crate::state::EngineState;

use super::decision_env::{ActionId, DecisionId, PolicyInput};
use super::search_policy::{Exactness, SearchEvidence, SearchKind};

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

    pub fn from_policy_input(
        policy_input: &PolicyInput,
        engine: EngineState,
        combat: CombatState,
        candidates: Vec<ClientInput>,
    ) -> Self {
        Self::new(policy_input.decision_id.clone(), engine, combat, candidates)
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
    BranchCompression,
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
    pub alive: bool,
    pub truncated: bool,
    pub engine_steps: u32,
    pub max_engine_steps: u32,
    pub before: CombatStateSummary,
    pub after: CombatStateSummary,
    pub delta: TransitionDelta,
    pub branch_effect: BranchEffectVector,
}

impl NeutralEngineQueryResult {
    pub fn to_search_evidence(&self, evidence_id: impl Into<String>) -> SearchEvidence {
        let search_kind = match self.query_kind {
            NeutralQueryKind::OneStepTransition => SearchKind::NeutralOneStepTransition,
            NeutralQueryKind::StableTransition => SearchKind::NeutralStableTransition {
                max_engine_steps: self.max_engine_steps,
            },
            NeutralQueryKind::BranchCompression => SearchKind::NeutralBranchCompression {
                max_engine_steps: self.max_engine_steps,
            },
        };
        SearchEvidence {
            evidence_id: evidence_id.into(),
            decision_id: self.decision_id.clone(),
            candidate_id: Some(self.action_id),
            request_id: None,
            search_kind,
            exactness: if self.truncated {
                Exactness::BoundedExact
            } else {
                Exactness::Exact
            },
            truncated: self.truncated,
            payload: serde_json::to_value(self).unwrap_or_else(|_| Value::Null),
        }
    }
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
            before,
            after,
        ))
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

    pub fn compress_branch_effects(
        &self,
        results: &[NeutralEngineQueryResult],
    ) -> Vec<BranchEffectGroup> {
        compress_branch_effects(results)
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
        alive,
        truncated,
        engine_steps,
        max_engine_steps,
        before,
        after,
        delta,
        branch_effect,
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
