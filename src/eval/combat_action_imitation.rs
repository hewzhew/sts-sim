use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use sts_combat_planner::{
    CombatActionPolicy, CombatPolicyChoice, CombatStateGuide, CombatStateGuideRank,
    SharedCombatActionPolicy, UniformCombatActionPolicy,
};

use crate::content::cards::{get_card_definition, java_id, CardId};
use crate::content::monsters::EnemyId;
use crate::content::powers::PowerId;
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use crate::sim::combat_action::combat_action_key;
use crate::sim::combat_action_surface::CombatSelectionActionFamilyV2;
use crate::state::core::{ClientInput, EngineState};

pub const COMBAT_ACTION_IMITATION_SCHEMA_NAME: &str = "CombatActionImitationArtifactV1";
pub const COMBAT_ACTION_IMITATION_SCHEMA_VERSION: u32 = 1;
const COMBAT_ACTION_FEATURE_SCHEMA: &str = "typed-state-and-generation-x-semantic-action/v3";

#[derive(Clone, Copy, Debug)]
pub struct CombatActionImitationTrainingConfigV1 {
    pub epochs: usize,
    pub learning_rate: f64,
    pub l2_penalty: f64,
    pub max_structured_alternatives: usize,
    pub max_engine_steps_per_transition: usize,
    pub logit_scale: f64,
    pub max_abs_log_factor: f64,
    /// Zero lets the learned distribution own action ordering. A positive
    /// value trains and applies the learned logits as residual corrections to
    /// the same base action policy used at runtime.
    pub base_weight_exponent: f64,
}

impl Default for CombatActionImitationTrainingConfigV1 {
    fn default() -> Self {
        Self {
            epochs: 240,
            learning_rate: 0.08,
            l2_penalty: 1.0e-4,
            max_structured_alternatives: 256,
            max_engine_steps_per_transition: 512,
            logit_scale: 1.0,
            max_abs_log_factor: 3.0,
            base_weight_exponent: 0.0,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatActionImitationCoefficientV1 {
    pub feature: String,
    pub weight: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatActionImitationArtifactV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub feature_schema: String,
    pub training_authority: String,
    #[serde(default = "default_source_trajectory_count")]
    pub source_trajectory_count: usize,
    pub source_action_count: usize,
    pub source_terminal_final_hp: i32,
    pub ranked_decision_count: usize,
    pub pairwise_comparison_count: usize,
    pub skipped_forced_decision_count: usize,
    pub training_top1_correct: usize,
    pub training_top1_total: usize,
    pub logit_scale: f64,
    pub max_abs_log_factor: f64,
    #[serde(default = "default_base_weight_exponent")]
    pub base_weight_exponent: f64,
    pub coefficients: Vec<CombatActionImitationCoefficientV1>,
}

impl CombatActionImitationArtifactV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_name != COMBAT_ACTION_IMITATION_SCHEMA_NAME
            || self.schema_version != COMBAT_ACTION_IMITATION_SCHEMA_VERSION
            || self.feature_schema != COMBAT_ACTION_FEATURE_SCHEMA
        {
            return Err("unsupported combat action imitation schema".to_string());
        }
        if self.source_trajectory_count == 0
            || self.ranked_decision_count == 0
            || self.coefficients.is_empty()
        {
            return Err("combat action imitation artifact has no learned ranking".to_string());
        }
        if !self.logit_scale.is_finite() || self.logit_scale <= 0.0 {
            return Err(
                "combat action imitation logit scale must be positive and finite".to_string(),
            );
        }
        if !self.max_abs_log_factor.is_finite() || self.max_abs_log_factor <= 0.0 {
            return Err(
                "combat action imitation log-factor limit must be positive and finite".to_string(),
            );
        }
        if !self.base_weight_exponent.is_finite()
            || !(0.0..=1.0).contains(&self.base_weight_exponent)
        {
            return Err(
                "combat action imitation base-weight exponent must be in 0..=1".to_string(),
            );
        }
        if self
            .coefficients
            .iter()
            .any(|coefficient| coefficient.feature.is_empty() || !coefficient.weight.is_finite())
        {
            return Err("combat action imitation coefficient is invalid".to_string());
        }
        if self
            .coefficients
            .windows(2)
            .any(|pair| pair[0].feature >= pair[1].feature)
        {
            return Err(
                "combat action imitation coefficients must have unique ascending names".to_string(),
            );
        }
        Ok(())
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let bytes = serde_json::to_vec_pretty(self).map_err(|error| error.to_string())?;
        std::fs::write(path, bytes).map_err(|error| error.to_string())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let artifact = serde_json::from_slice::<Self>(
            &std::fs::read(path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("invalid combat action imitation artifact: {error}"))?;
        artifact.validate()?;
        Ok(artifact)
    }
}

type SparseFeatures = BTreeMap<String, f64>;

#[derive(Clone, Debug)]
struct RankingExample {
    demonstrated_index: usize,
    candidates: Vec<SparseFeatures>,
    base_logits: Vec<f64>,
}

#[derive(Clone, Copy)]
pub struct CombatActionImitationDemonstrationV1<'a> {
    pub root: &'a CombatPosition,
    pub actions: &'a [ClientInput],
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatActionImitationDecisionAuditV1 {
    pub action_index: usize,
    pub player_turn: u32,
    pub candidate_count: usize,
    pub demonstrated_rank: usize,
    pub demonstrated_input: ClientInput,
    pub demonstrated_action_key: String,
    pub best_input: ClientInput,
    pub best_action_key: String,
    pub demonstrated_logit: f64,
    pub best_logit: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatActionImitationAuditV1 {
    pub source_action_count: usize,
    pub ranked_decision_count: usize,
    pub skipped_forced_decision_count: usize,
    pub misses: Vec<CombatActionImitationDecisionAuditV1>,
}

/// Trains a cheap action policy from one exact, terminally verified combat
/// witness. The artifact contains typed state/action features and learned
/// coefficients only: no exact state hash, card UUID, hand index, or witness
/// action is available to the runtime policy.
pub fn train_combat_action_imitation_v1(
    root: &CombatPosition,
    demonstrated_actions: &[ClientInput],
    config: CombatActionImitationTrainingConfigV1,
) -> Result<CombatActionImitationArtifactV1, String> {
    train_combat_action_imitation_from_demonstrations_v1(
        &[CombatActionImitationDemonstrationV1 {
            root,
            actions: demonstrated_actions,
        }],
        config,
    )
}

pub fn train_combat_action_imitation_from_demonstrations_v1(
    demonstrations: &[CombatActionImitationDemonstrationV1<'_>],
    config: CombatActionImitationTrainingConfigV1,
) -> Result<CombatActionImitationArtifactV1, String> {
    train_combat_action_imitation_from_demonstrations_with_base_v1(
        demonstrations,
        config,
        Arc::new(UniformCombatActionPolicy),
    )
}

pub fn train_combat_action_imitation_from_demonstrations_with_base_v1(
    demonstrations: &[CombatActionImitationDemonstrationV1<'_>],
    config: CombatActionImitationTrainingConfigV1,
    base_policy: SharedCombatActionPolicy,
) -> Result<CombatActionImitationArtifactV1, String> {
    validate_training_config(config)?;
    if demonstrations.is_empty() {
        return Err("combat action imitation requires at least one demonstration".to_string());
    }
    let stepper = EngineCombatStepper;
    let mut examples = Vec::new();
    let mut skipped_forced_decision_count = 0usize;
    let mut pairwise_comparison_count = 0usize;
    let mut source_action_count = 0usize;
    let mut source_terminal_final_hp = i32::MAX;

    for (source_index, demonstration) in demonstrations.iter().enumerate() {
        let mut position = demonstration.root.clone();
        source_action_count = source_action_count.saturating_add(demonstration.actions.len());
        for (action_index, demonstrated) in demonstration.actions.iter().enumerate() {
            if !stepper.is_legal_action(&position, demonstrated) {
                return Err(format!(
                    "demonstration {source_index} action {action_index} is not legal at its exact replay state"
                ));
            }
            let candidates = concrete_training_inputs(
                &position,
                demonstrated,
                config.max_structured_alternatives,
            );
            let demonstrated_index = candidates
                .iter()
                .position(|candidate| candidate == demonstrated)
                .ok_or_else(|| {
                    format!(
                        "demonstration {source_index} action {action_index} was absent from its legal surface"
                    )
                })?;
            if candidates.len() > 1 {
                pairwise_comparison_count =
                    pairwise_comparison_count.saturating_add(candidates.len().saturating_sub(1));
                let state = typed_combat_feature_components_v1(&position);
                examples.push(RankingExample {
                    demonstrated_index,
                    base_logits: concrete_base_logits(
                        &position,
                        &candidates,
                        base_policy.as_ref(),
                        config.base_weight_exponent,
                    ),
                    candidates: candidates
                        .iter()
                        .map(|candidate| {
                            action_feature_vector_with_state(&position, candidate, &state)
                        })
                        .collect(),
                });
            } else {
                skipped_forced_decision_count = skipped_forced_decision_count.saturating_add(1);
            }

            let step = stepper.apply_to_stable(
                &position,
                demonstrated.clone(),
                CombatStepLimits {
                    max_engine_steps: config.max_engine_steps_per_transition,
                    deadline: None,
                },
            );
            if step.truncated || step.timed_out {
                return Err(format!(
                    "demonstration {source_index} action {action_index} did not reach a stable exact successor"
                ));
            }
            position = step.position;
        }

        if stepper.terminal(&position) != CombatTerminal::Win
            || position.combat.runtime.combat_smoked
        {
            return Err(format!(
                "combat action imitation demonstration {source_index} is not an exact terminal victory"
            ));
        }
        source_terminal_final_hp =
            source_terminal_final_hp.min(position.combat.entities.player.current_hp);
    }
    if examples.is_empty() {
        return Err("combat action imitation source contains no ranked decisions".to_string());
    }

    let weights = train_sparse_softmax(&examples, config);
    let training_top1_correct = examples
        .iter()
        .filter(|example| {
            runtime_candidate_index(
                &weights,
                example,
                config.logit_scale,
                config.max_abs_log_factor,
            ) == example.demonstrated_index
        })
        .count();
    let coefficients = weights
        .into_iter()
        .filter(|(_, weight)| weight.abs() >= 1.0e-10)
        .map(|(feature, weight)| CombatActionImitationCoefficientV1 { feature, weight })
        .collect::<Vec<_>>();
    let artifact = CombatActionImitationArtifactV1 {
        schema_name: COMBAT_ACTION_IMITATION_SCHEMA_NAME.to_string(),
        schema_version: COMBAT_ACTION_IMITATION_SCHEMA_VERSION,
        feature_schema: COMBAT_ACTION_FEATURE_SCHEMA.to_string(),
        training_authority: "exact_terminal_win_action_demonstrations".to_string(),
        source_trajectory_count: demonstrations.len(),
        source_action_count,
        source_terminal_final_hp,
        ranked_decision_count: examples.len(),
        pairwise_comparison_count,
        skipped_forced_decision_count,
        training_top1_correct,
        training_top1_total: examples.len(),
        logit_scale: config.logit_scale,
        max_abs_log_factor: config.max_abs_log_factor,
        base_weight_exponent: config.base_weight_exponent,
        coefficients,
    };
    artifact.validate()?;
    Ok(artifact)
}

/// Replays one verified demonstration and exposes only decisions the learned
/// policy does not rank first. This is a training-representation diagnostic;
/// it neither changes policy weights nor grants the witness runtime authority.
pub fn audit_combat_action_imitation_v1(
    root: &CombatPosition,
    demonstrated_actions: &[ClientInput],
    artifact: &CombatActionImitationArtifactV1,
    base_policy: &dyn CombatActionPolicy,
    max_structured_alternatives: usize,
    max_engine_steps_per_transition: usize,
) -> Result<CombatActionImitationAuditV1, String> {
    artifact.validate()?;
    let coefficients = artifact
        .coefficients
        .iter()
        .map(|coefficient| (coefficient.feature.clone(), coefficient.weight))
        .collect::<HashMap<_, _>>();
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut misses = Vec::new();
    let mut ranked_decision_count = 0usize;
    let mut skipped_forced_decision_count = 0usize;
    for (action_index, demonstrated) in demonstrated_actions.iter().enumerate() {
        if !stepper.is_legal_action(&position, demonstrated) {
            return Err(format!(
                "action imitation audit action {action_index} is not legal at its exact replay state"
            ));
        }
        let candidates =
            concrete_training_inputs(&position, demonstrated, max_structured_alternatives);
        let demonstrated_index = candidates
            .iter()
            .position(|candidate| candidate == demonstrated)
            .ok_or_else(|| {
                format!(
                    "action imitation audit action {action_index} is absent from its candidates"
                )
            })?;
        if candidates.len() > 1 {
            ranked_decision_count = ranked_decision_count.saturating_add(1);
            let state = typed_combat_feature_components_v1(&position);
            let logits = candidates
                .iter()
                .map(|candidate| {
                    sparse_score(
                        &coefficients,
                        &action_feature_vector_with_state(&position, candidate, &state),
                    ) * artifact.logit_scale
                })
                .collect::<Vec<_>>();
            let base_logits = concrete_base_logits(
                &position,
                &candidates,
                base_policy,
                artifact.base_weight_exponent,
            );
            let logits =
                runtime_combined_logits(&logits, &base_logits, artifact.max_abs_log_factor);
            let demonstrated_logit = logits[demonstrated_index];
            let demonstrated_rank = 1 + logits
                .iter()
                .enumerate()
                .filter(|(candidate_index, candidate)| {
                    candidate.total_cmp(&demonstrated_logit).is_gt()
                        || (candidate.total_cmp(&demonstrated_logit).is_eq()
                            && *candidate_index < demonstrated_index)
                })
                .count();
            let best_index = logits
                .iter()
                .enumerate()
                .max_by(|(left_index, left), (right_index, right)| {
                    left.total_cmp(right)
                        .then_with(|| right_index.cmp(left_index))
                })
                .map(|(index, _)| index)
                .unwrap_or_default();
            if best_index != demonstrated_index {
                misses.push(CombatActionImitationDecisionAuditV1 {
                    action_index,
                    player_turn: position.combat.turn.turn_count,
                    candidate_count: candidates.len(),
                    demonstrated_rank,
                    demonstrated_input: demonstrated.clone(),
                    demonstrated_action_key: combat_action_key(&position.combat, demonstrated),
                    best_input: candidates[best_index].clone(),
                    best_action_key: combat_action_key(&position.combat, &candidates[best_index]),
                    demonstrated_logit,
                    best_logit: logits[best_index],
                });
            }
        } else {
            skipped_forced_decision_count = skipped_forced_decision_count.saturating_add(1);
        }
        let step = stepper.apply_to_stable(
            &position,
            demonstrated.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return Err(format!(
                "action imitation audit action {action_index} did not reach a stable exact successor"
            ));
        }
        position = step.position;
    }
    if stepper.terminal(&position) != CombatTerminal::Win || position.combat.runtime.combat_smoked {
        return Err("action imitation audit source is not an exact terminal victory".to_string());
    }
    Ok(CombatActionImitationAuditV1 {
        source_action_count: demonstrated_actions.len(),
        ranked_decision_count,
        skipped_forced_decision_count,
        misses,
    })
}

fn default_source_trajectory_count() -> usize {
    1
}

fn default_base_weight_exponent() -> f64 {
    1.0
}

pub fn combat_action_imitation_policy_v1(
    base: SharedCombatActionPolicy,
    artifact: CombatActionImitationArtifactV1,
) -> Result<SharedCombatActionPolicy, String> {
    artifact.validate()?;
    let coefficients = artifact
        .coefficients
        .iter()
        .map(|coefficient| (coefficient.feature.clone(), coefficient.weight))
        .collect();
    Ok(Arc::new(CombatActionImitationPolicyV1 {
        base,
        coefficients,
        logit_scale: artifact.logit_scale,
        max_abs_log_factor: artifact.max_abs_log_factor,
        base_weight_exponent: artifact.base_weight_exponent,
    }))
}

/// Applies a specialized proposal policy only while constructing the current
/// root player turn.  Future turns return to the durable continuation policy;
/// their ordering belongs to cross-turn boundary guidance rather than to one
/// demonstrated action trace.
pub fn root_player_turn_action_policy_v1(
    root_player_turn: u32,
    root: SharedCombatActionPolicy,
    continuation: SharedCombatActionPolicy,
) -> SharedCombatActionPolicy {
    Arc::new(RootPlayerTurnActionPolicyV1 {
        root_player_turn,
        root,
        continuation,
    })
}

struct RootPlayerTurnActionPolicyV1 {
    root_player_turn: u32,
    root: SharedCombatActionPolicy,
    continuation: SharedCombatActionPolicy,
}

impl CombatActionPolicy for RootPlayerTurnActionPolicyV1 {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        if position.combat.turn.turn_count == self.root_player_turn {
            self.root.weights(position, choices)
        } else {
            self.continuation.weights(position, choices)
        }
    }

    fn structured_selection_member_weights(
        &self,
        position: &CombatPosition,
        family: &CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        if position.combat.turn.turn_count == self.root_player_turn {
            self.root
                .structured_selection_member_weights(position, family, members)
        } else {
            self.continuation
                .structured_selection_member_weights(position, family, members)
        }
    }

    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.continuation.state_guides(position)
    }

    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.continuation.turn_generation_guides(position)
    }
}

#[derive(Clone)]
struct CombatActionImitationPolicyV1 {
    base: SharedCombatActionPolicy,
    coefficients: HashMap<String, f64>,
    logit_scale: f64,
    max_abs_log_factor: f64,
    base_weight_exponent: f64,
}

impl CombatActionImitationPolicyV1 {
    fn learned_logit(&self, position: &CombatPosition, input: &ClientInput, state: &[i32]) -> f64 {
        sparse_score(
            &self.coefficients,
            &action_feature_vector_with_state(position, input, state),
        ) * self.logit_scale
    }
}

impl CombatActionPolicy for CombatActionImitationPolicyV1 {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        let base = self.base.weights(position, choices);
        let base = (base.len() == choices.len())
            .then_some(base)
            .unwrap_or_else(|| vec![1.0; choices.len()]);
        let state = typed_combat_feature_components_v1(position);
        let logits = choices
            .iter()
            .map(|choice| match choice {
                CombatPolicyChoice::Atomic(input) => {
                    Some(self.learned_logit(position, input, &state))
                }
                CombatPolicyChoice::StructuredSelection(_) => None,
            })
            .collect::<Vec<_>>();
        let atomic_logits = logits.iter().flatten().copied().collect::<Vec<_>>();
        let atomic_factors = normalized_learned_factors(&atomic_logits, self.max_abs_log_factor);
        let mut atomic_factor_index = 0usize;
        choices
            .iter()
            .zip(base)
            .zip(logits)
            .map(|((choice, base), logit)| match choice {
                CombatPolicyChoice::Atomic(_) => {
                    debug_assert!(logit.is_some());
                    let factor = atomic_factors[atomic_factor_index];
                    atomic_factor_index += 1;
                    positive_or_neutral(base).powf(self.base_weight_exponent) * factor
                }
                CombatPolicyChoice::StructuredSelection(_) => {
                    positive_or_neutral(base).powf(self.base_weight_exponent)
                }
            })
            .collect()
    }

    fn structured_selection_member_weights(
        &self,
        position: &CombatPosition,
        family: &CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        let base = self
            .base
            .structured_selection_member_weights(position, family, members);
        let base = (base.len() == members.len())
            .then_some(base)
            .unwrap_or_else(|| vec![1.0; members.len()]);
        let state = typed_combat_feature_components_v1(position);
        let logits = members
            .iter()
            .map(|member| self.learned_logit(position, member, &state))
            .collect::<Vec<_>>();
        let factors = normalized_learned_factors(&logits, self.max_abs_log_factor);
        members
            .iter()
            .zip(base)
            .zip(factors)
            .map(|((_member, base), factor)| {
                positive_or_neutral(base).powf(self.base_weight_exponent) * factor
            })
            .collect()
    }

    fn state_guide_rank(&self, position: &CombatPosition) -> Option<CombatStateGuideRank> {
        self.base.state_guide_rank(position)
    }

    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.base.state_guides(position)
    }

    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.base.turn_generation_guides(position)
    }
}

fn normalized_learned_factors(logits: &[f64], max_log_penalty: f64) -> Vec<f64> {
    let max_logit = logits.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if !max_logit.is_finite() {
        return vec![1.0; logits.len()];
    }
    logits
        .iter()
        .map(|logit| (logit - max_logit).clamp(-max_log_penalty, 0.0).exp())
        .collect()
}

fn validate_training_config(config: CombatActionImitationTrainingConfigV1) -> Result<(), String> {
    if config.epochs == 0
        || config.max_structured_alternatives == 0
        || config.max_engine_steps_per_transition == 0
        || !config.learning_rate.is_finite()
        || config.learning_rate <= 0.0
        || !config.l2_penalty.is_finite()
        || config.l2_penalty < 0.0
        || !config.logit_scale.is_finite()
        || config.logit_scale <= 0.0
        || !config.max_abs_log_factor.is_finite()
        || config.max_abs_log_factor <= 0.0
        || !config.base_weight_exponent.is_finite()
        || !(0.0..=1.0).contains(&config.base_weight_exponent)
    {
        return Err("invalid combat action imitation training configuration".to_string());
    }
    Ok(())
}

fn concrete_training_inputs(
    position: &CombatPosition,
    demonstrated: &ClientInput,
    max_structured_alternatives: usize,
) -> Vec<ClientInput> {
    let stepper = EngineCombatStepper;
    let mut candidates = stepper.atomic_actions(position);
    if let EngineState::PendingChoice(choice) = &position.engine {
        if let Some(inputs) =
            crate::ai::combat_search_v2::pending_choice_action_prefix::canonical_pending_choice_inputs(
                choice,
            )
        {
            candidates.extend(inputs.take(max_structured_alternatives));
        }
    }
    if !candidates.contains(demonstrated) {
        candidates.push(demonstrated.clone());
    }
    let mut unique = Vec::with_capacity(candidates.len());
    for candidate in candidates {
        if stepper.is_legal_action(position, &candidate) && !unique.contains(&candidate) {
            unique.push(candidate);
        }
    }
    unique
}

fn concrete_base_logits(
    position: &CombatPosition,
    candidates: &[ClientInput],
    base_policy: &dyn CombatActionPolicy,
    exponent: f64,
) -> Vec<f64> {
    if exponent <= 0.0 {
        return vec![0.0; candidates.len()];
    }
    let choices = candidates
        .iter()
        .map(CombatPolicyChoice::Atomic)
        .collect::<Vec<_>>();
    let weights = base_policy.weights(position, &choices);
    if weights.len() != candidates.len() {
        return vec![0.0; candidates.len()];
    }
    weights
        .into_iter()
        .map(|weight| positive_or_neutral(weight).ln() * exponent)
        .collect()
}

fn train_sparse_softmax(
    examples: &[RankingExample],
    config: CombatActionImitationTrainingConfigV1,
) -> BTreeMap<String, f64> {
    let mut weights = BTreeMap::<String, f64>::new();
    for epoch in 0..config.epochs {
        let learning_rate = config.learning_rate / (1.0 + epoch as f64 * 0.05).sqrt();
        let shrink = (1.0 - learning_rate * config.l2_penalty).clamp(0.0, 1.0);
        for weight in weights.values_mut() {
            *weight *= shrink;
        }
        for example in examples {
            let scores = example
                .candidates
                .iter()
                .zip(&example.base_logits)
                .map(|(candidate, base)| {
                    sparse_score(&weights, candidate) * config.logit_scale + base
                })
                .collect::<Vec<_>>();
            let max_score = scores.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let exponentials = scores
                .iter()
                .map(|score| (score - max_score).exp())
                .collect::<Vec<_>>();
            let total = exponentials.iter().sum::<f64>().max(f64::MIN_POSITIVE);
            for (candidate_index, candidate) in example.candidates.iter().enumerate() {
                let target = f64::from(candidate_index == example.demonstrated_index);
                let gradient =
                    (target - exponentials[candidate_index] / total) * config.logit_scale;
                if gradient.abs() < f64::EPSILON {
                    continue;
                }
                for (feature, value) in candidate {
                    *weights.entry(feature.clone()).or_default() +=
                        learning_rate * gradient * value;
                }
            }
        }
    }
    weights
}

fn runtime_candidate_index(
    weights: &BTreeMap<String, f64>,
    example: &RankingExample,
    logit_scale: f64,
    max_abs_log_factor: f64,
) -> usize {
    let learned = example
        .candidates
        .iter()
        .map(|candidate| sparse_score(weights, candidate) * logit_scale)
        .collect::<Vec<_>>();
    runtime_combined_logits(&learned, &example.base_logits, max_abs_log_factor)
        .iter()
        .enumerate()
        .max_by(|(left_index, left), (right_index, right)| {
            left.total_cmp(right)
                .then_with(|| right_index.cmp(left_index))
        })
        .map(|(index, _)| index)
        .unwrap_or_default()
}

fn runtime_combined_logits(
    learned_logits: &[f64],
    base_logits: &[f64],
    max_abs_log_factor: f64,
) -> Vec<f64> {
    debug_assert_eq!(learned_logits.len(), base_logits.len());
    let max_learned = learned_logits
        .iter()
        .copied()
        .fold(f64::NEG_INFINITY, f64::max);
    learned_logits
        .iter()
        .zip(base_logits)
        .map(|(learned, base)| {
            let residual = if max_learned.is_finite() {
                (learned - max_learned).clamp(-max_abs_log_factor, 0.0)
            } else {
                0.0
            };
            base + residual
        })
        .collect()
}

fn sparse_score<W>(weights: &W, features: &SparseFeatures) -> f64
where
    W: SparseWeightLookup,
{
    features
        .iter()
        .map(|(feature, value)| weights.weight(feature) * value)
        .sum()
}

trait SparseWeightLookup {
    fn weight(&self, feature: &str) -> f64;
}

impl SparseWeightLookup for BTreeMap<String, f64> {
    fn weight(&self, feature: &str) -> f64 {
        self.get(feature).copied().unwrap_or_default()
    }
}

impl SparseWeightLookup for HashMap<String, f64> {
    fn weight(&self, feature: &str) -> f64 {
        self.get(feature).copied().unwrap_or_default()
    }
}

fn positive_or_neutral(weight: f64) -> f64 {
    if weight.is_finite() && weight > 0.0 {
        weight
    } else {
        1.0
    }
}

#[cfg(test)]
fn action_feature_vector(position: &CombatPosition, input: &ClientInput) -> SparseFeatures {
    let state = typed_combat_feature_components_v1(position);
    action_feature_vector_with_state(position, input, &state)
}

fn action_feature_vector_with_state(
    position: &CombatPosition,
    input: &ClientInput,
    state: &[i32],
) -> SparseFeatures {
    let mut features = SparseFeatures::new();
    let tokens = action_semantic_tokens(position, input);
    let token_scale = 1.0 / (tokens.len().max(1) as f64).sqrt();
    for token in tokens {
        add_feature(&mut features, format!("action/{token}"), token_scale);
        for (index, component) in state.iter().copied().enumerate() {
            add_feature(
                &mut features,
                format!("cross/{token}/state/{index}"),
                token_scale * squash_component(component),
            );
        }
    }
    add_numeric_action_features(position, input, &mut features);
    features
}

fn action_semantic_tokens(position: &CombatPosition, input: &ClientInput) -> Vec<String> {
    let mut tokens = Vec::new();
    match input {
        ClientInput::PlayCard { card_index, target } => {
            tokens.push("kind/play_card".to_string());
            let mut card_tokens = Vec::new();
            if let Some(card) = position.combat.zones.hand.get(*card_index) {
                let definition = get_card_definition(card.id);
                card_tokens.push(format!("card/{}+{}", java_id(card.id), card.upgrades));
                card_tokens.push(format!("card_type/{:?}", definition.card_type));
                tokens.extend(card_tokens.iter().cloned());
            }
            let target_start = tokens.len();
            push_target_tokens(position, *target, &mut tokens);
            let target_tokens = tokens[target_start..].to_vec();
            for card_token in &card_tokens {
                for target_token in &target_tokens {
                    tokens.push(format!("interaction/{card_token}/{target_token}"));
                }
            }
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => {
            tokens.push("kind/use_potion".to_string());
            if let Some(Some(potion)) = position.combat.entities.potions.get(*potion_index) {
                tokens.push(format!("potion/{:?}", potion.id));
            }
            push_target_tokens(position, *target, &mut tokens);
        }
        ClientInput::DiscardPotion(_) => tokens.push("kind/discard_potion".to_string()),
        ClientInput::EndTurn => tokens.push("kind/end_turn".to_string()),
        ClientInput::SubmitSelection(resolution) => {
            tokens.push(format!("kind/selection/{:?}", resolution.scope));
            for uuid in resolution.selected_card_uuids() {
                if let Some(card) = combat_card_by_uuid(position, uuid) {
                    tokens.push(format!(
                        "selected_card/{}+{}",
                        java_id(card.id),
                        card.upgrades
                    ));
                } else {
                    tokens.push("selected_card/unknown".to_string());
                }
            }
        }
        ClientInput::SubmitScryDiscard(indices) => {
            tokens.push("kind/scry_discard".to_string());
            for index in indices {
                if let Some(card) = position.combat.zones.draw_pile.get(*index) {
                    tokens.push(format!(
                        "selected_card/{}+{}",
                        java_id(card.id),
                        card.upgrades
                    ));
                }
            }
        }
        ClientInput::SubmitDiscoverChoice(index) => {
            tokens.push("kind/discover_choice".to_string());
            push_discover_choice_tokens(position, *index, &mut tokens);
        }
        ClientInput::Cancel => tokens.push("kind/cancel".to_string()),
        ClientInput::Proceed => tokens.push("kind/proceed".to_string()),
        _ => tokens.push("kind/non_combat_input".to_string()),
    }
    tokens.sort();
    tokens.dedup();
    tokens
}

fn push_target_tokens(
    position: &CombatPosition,
    target: Option<crate::EntityId>,
    tokens: &mut Vec<String>,
) {
    match target {
        None => tokens.push("target/none".to_string()),
        Some(entity) => {
            tokens.push("target/enemy".to_string());
            if let Some(monster) = position
                .combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == entity)
            {
                tokens.push(format!("target/slot/{}", monster.slot));
                if let Some(enemy_id) = EnemyId::from_id(monster.monster_type) {
                    tokens.push(format!("target/enemy/{enemy_id:?}"));
                }
                if monster.block > 0 {
                    tokens.push("target/has_block".to_string());
                }
                for power in [
                    PowerId::Artifact,
                    PowerId::Vulnerable,
                    PowerId::Weak,
                    PowerId::Strength,
                    PowerId::Flight,
                    PowerId::SharpHide,
                    PowerId::Malleable,
                    PowerId::Minion,
                ] {
                    let amount = position.combat.get_power(monster.id, power);
                    if amount != 0 {
                        tokens.push(format!("target/power/{power:?}/{}", amount.signum()));
                    }
                }
            }
        }
    }
}

fn push_discover_choice_tokens(position: &CombatPosition, index: usize, tokens: &mut Vec<String>) {
    use crate::state::core::PendingChoice;

    let EngineState::PendingChoice(choice) = &position.engine else {
        return;
    };
    let selected = match choice {
        PendingChoice::DiscoverySelect(choice) => choice.cards.get(index).map(|card| (*card, 0)),
        PendingChoice::CardRewardSelect { cards, .. } => cards.get(index).map(|card| (*card, 0)),
        PendingChoice::ForeignInfluenceSelect { cards, upgraded } => cards
            .get(index)
            .map(|card| (*card, usize::from(*upgraded) as u8)),
        PendingChoice::ChooseOneSelect { choices } => choices
            .get(index)
            .map(|choice| (choice.card_id, choice.upgrades)),
        PendingChoice::StanceChoice => {
            tokens.push(format!("choice/stance/{index}"));
            None
        }
        _ => None,
    };
    if let Some((card, upgrades)) = selected {
        push_choice_card_tokens(card, upgrades, tokens);
    }
}

fn push_choice_card_tokens(card: CardId, upgrades: u8, tokens: &mut Vec<String>) {
    let definition = get_card_definition(card);
    tokens.push(format!("choice/card/{}+{upgrades}", java_id(card)));
    tokens.push(format!("choice/card_type/{:?}", definition.card_type));
}

fn add_numeric_action_features(
    position: &CombatPosition,
    input: &ClientInput,
    features: &mut SparseFeatures,
) {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            if let Some(card) = position.combat.zones.hand.get(*card_index) {
                let definition = get_card_definition(card.id);
                add_feature(
                    features,
                    "numeric/card/base_damage".to_string(),
                    squash_component(definition.base_damage),
                );
                add_feature(
                    features,
                    "numeric/card/base_block".to_string(),
                    squash_component(definition.base_block),
                );
                add_feature(
                    features,
                    "numeric/card/base_magic".to_string(),
                    squash_component(definition.base_magic),
                );
                let cost = card
                    .cost_for_turn
                    .map(i32::from)
                    .unwrap_or(i32::from(definition.cost) + i32::from(card.cost_modifier));
                add_feature(
                    features,
                    "numeric/card/cost".to_string(),
                    squash_component(cost),
                );
                add_feature(
                    features,
                    "numeric/card/exhaust".to_string(),
                    f64::from(card.exhaust_override.unwrap_or(definition.exhaust)),
                );
            }
            if let Some(monster) = target.and_then(|entity| {
                position
                    .combat
                    .entities
                    .monsters
                    .iter()
                    .find(|monster| monster.id == entity)
            }) {
                for (name, value) in [
                    ("current_hp", monster.current_hp),
                    ("max_hp", monster.max_hp),
                    ("block", monster.block),
                    (
                        "artifact",
                        position.combat.get_power(monster.id, PowerId::Artifact),
                    ),
                    (
                        "vulnerable",
                        position.combat.get_power(monster.id, PowerId::Vulnerable),
                    ),
                    ("weak", position.combat.get_power(monster.id, PowerId::Weak)),
                    (
                        "strength",
                        position.combat.get_power(monster.id, PowerId::Strength),
                    ),
                ] {
                    add_feature(
                        features,
                        format!("numeric/target/{name}"),
                        squash_component(value),
                    );
                }
            }
        }
        ClientInput::SubmitSelection(resolution) => add_feature(
            features,
            "numeric/selection/count".to_string(),
            squash_component(i32::try_from(resolution.selected.len()).unwrap_or(i32::MAX)),
        ),
        ClientInput::SubmitScryDiscard(indices) => add_feature(
            features,
            "numeric/selection/count".to_string(),
            squash_component(i32::try_from(indices.len()).unwrap_or(i32::MAX)),
        ),
        _ => {}
    }
}

fn combat_card_by_uuid(
    position: &CombatPosition,
    uuid: u32,
) -> Option<&crate::runtime::combat::CombatCard> {
    position
        .combat
        .zones
        .hand
        .iter()
        .chain(&position.combat.zones.draw_pile)
        .chain(&position.combat.zones.discard_pile)
        .chain(&position.combat.zones.exhaust_pile)
        .chain(&position.combat.zones.limbo)
        .find(|card| card.uuid == uuid)
}

fn add_feature(features: &mut SparseFeatures, name: String, value: f64) {
    if value.is_finite() && value.abs() >= 1.0e-12 {
        *features.entry(name).or_default() += value;
    }
}

fn squash_component(value: i32) -> f64 {
    (f64::from(value).asinh() / 8.0).clamp(-1.0, 1.0)
}

pub fn typed_combat_feature_components_v1(position: &CombatPosition) -> Vec<i32> {
    let mut features =
        crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(
            position,
        );
    features.extend(
        crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(
            position,
        ),
    );
    features.extend(
        crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(
            position,
        ),
    );
    features.extend(
        crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(
            position,
        ),
    );
    features.extend(
        crate::ai::combat_search_v2::oracle_action_policy::oracle_combat_turn_generation_guide_components(
            position,
        ),
    );
    features
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::powers::store;
    use crate::runtime::combat::CombatCard;
    use crate::runtime::combat::{Power, PowerPayload};
    use crate::state::core::{DiscoveryChoiceState, PendingChoice};
    use crate::testing::support::{blank_test_combat, test_monster};
    use sts_combat_planner::UniformCombatActionPolicy;

    struct ConstantPolicy(f64);

    impl CombatActionPolicy for ConstantPolicy {
        fn weights(
            &self,
            _position: &CombatPosition,
            choices: &[CombatPolicyChoice<'_>],
        ) -> Vec<f64> {
            vec![self.0; choices.len()]
        }
    }

    fn synthetic_example(positive: f64, negative: f64) -> RankingExample {
        RankingExample {
            demonstrated_index: 0,
            base_logits: vec![0.0; 2],
            candidates: vec![
                BTreeMap::from([("signal".to_string(), positive)]),
                BTreeMap::from([("signal".to_string(), negative)]),
            ],
        }
    }

    #[test]
    fn sparse_softmax_learns_demonstrated_ranking() {
        let examples = vec![synthetic_example(1.0, -1.0)];
        let config = CombatActionImitationTrainingConfigV1::default();
        let weights = train_sparse_softmax(&examples, config);
        assert_eq!(
            runtime_candidate_index(
                &weights,
                &examples[0],
                config.logit_scale,
                config.max_abs_log_factor,
            ),
            0
        );
        assert!(weights["signal"] > 0.0);
    }

    #[test]
    fn runtime_ranking_applies_base_and_bounded_residual_together() {
        let learned = vec![10.0, 0.0];
        let base = vec![0.0, 4.0];
        let combined = runtime_combined_logits(&learned, &base, 3.0);

        assert_eq!(combined, vec![0.0, 1.0]);
    }

    #[test]
    fn card_semantics_ignore_hand_index_and_uuid() {
        let mut left = blank_test_combat();
        left.zones.hand = vec![
            CombatCard::new(CardId::Warcry, 11),
            CombatCard::new(CardId::Defend, 12),
        ];
        let mut right = left.clone();
        right.zones.hand.swap(0, 1);
        right.zones.hand[1].uuid = 99;
        let left = CombatPosition::new(EngineState::CombatPlayerTurn, left);
        let right = CombatPosition::new(EngineState::CombatPlayerTurn, right);
        let left_features = action_feature_vector(
            &left,
            &ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
        );
        let right_features = action_feature_vector(
            &right,
            &ClientInput::PlayCard {
                card_index: 1,
                target: None,
            },
        );
        assert_eq!(left_features, right_features);
        assert!(left_features.keys().all(|feature| !feature.contains("99")));
    }

    #[test]
    fn discovery_choices_expose_selected_card_semantics() {
        let combat = blank_test_combat();
        let engine =
            EngineState::PendingChoice(PendingChoice::DiscoverySelect(DiscoveryChoiceState {
                cards: vec![CardId::Bash, CardId::Defend],
                colorless: false,
                card_type: None,
                amount: 1,
                can_skip: false,
            }));
        let position = CombatPosition::new(engine, combat);

        let bash = action_feature_vector(&position, &ClientInput::SubmitDiscoverChoice(0));
        let defend = action_feature_vector(&position, &ClientInput::SubmitDiscoverChoice(1));

        assert_ne!(bash, defend);
        assert!(bash.contains_key("action/choice/card/Bash+0"));
        assert!(defend.contains_key("action/choice/card/Defend_R+0"));
    }

    #[test]
    fn targeted_card_semantics_include_target_local_state() {
        let mut combat = blank_test_combat();
        let artifact = test_monster(EnemyId::Cultist);
        let mut exposed = test_monster(EnemyId::Cultist);
        exposed.id = 2;
        exposed.slot = 1;
        combat.entities.monsters = vec![artifact, exposed];
        store::set_powers_for(
            &mut combat,
            1,
            vec![Power {
                power_type: PowerId::Artifact,
                instance_id: None,
                amount: 1,
                extra_data: 0,
                payload: PowerPayload::None,
                just_applied: false,
            }],
        );
        combat.zones.hand = vec![CombatCard::new(CardId::Bash, 11)];
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let into_artifact = action_feature_vector(
            &position,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(1),
            },
        );
        let into_exposed = action_feature_vector(
            &position,
            &ClientInput::PlayCard {
                card_index: 0,
                target: Some(2),
            },
        );

        assert_ne!(into_artifact, into_exposed);
        assert!(
            into_artifact.contains_key("action/interaction/card/Bash+0/target/power/Artifact/1")
        );
        assert!(
            !into_exposed.contains_key("action/interaction/card/Bash+0/target/power/Artifact/1")
        );
    }

    #[test]
    fn learned_policy_preserves_positive_weights() {
        let artifact = CombatActionImitationArtifactV1 {
            schema_name: COMBAT_ACTION_IMITATION_SCHEMA_NAME.to_string(),
            schema_version: COMBAT_ACTION_IMITATION_SCHEMA_VERSION,
            feature_schema: COMBAT_ACTION_FEATURE_SCHEMA.to_string(),
            training_authority: "test".to_string(),
            source_trajectory_count: 1,
            source_action_count: 1,
            source_terminal_final_hp: 1,
            ranked_decision_count: 1,
            pairwise_comparison_count: 1,
            skipped_forced_decision_count: 0,
            training_top1_correct: 1,
            training_top1_total: 1,
            logit_scale: 1.0,
            max_abs_log_factor: 3.0,
            base_weight_exponent: 0.0,
            coefficients: vec![CombatActionImitationCoefficientV1 {
                feature: "action/kind/end_turn".to_string(),
                weight: -100.0,
            }],
        };
        let policy =
            combat_action_imitation_policy_v1(Arc::new(UniformCombatActionPolicy), artifact)
                .expect("valid learned policy");
        let position = CombatPosition::new(EngineState::CombatPlayerTurn, blank_test_combat());
        let input = ClientInput::EndTurn;
        let weights = policy.weights(&position, &[CombatPolicyChoice::Atomic(&input)]);
        assert_eq!(weights.len(), 1);
        assert!(weights[0].is_finite() && weights[0] > 0.0);
    }

    #[test]
    fn specialized_action_prior_stops_after_the_root_player_turn() {
        let mut position = CombatPosition::new(EngineState::CombatPlayerTurn, blank_test_combat());
        let root_turn = position.combat.turn.turn_count;
        let policy = root_player_turn_action_policy_v1(
            root_turn,
            Arc::new(ConstantPolicy(7.0)),
            Arc::new(ConstantPolicy(2.0)),
        );
        let input = ClientInput::EndTurn;
        let choices = [CombatPolicyChoice::Atomic(&input)];
        assert_eq!(policy.weights(&position, &choices), vec![7.0]);

        position.combat.turn.turn_count = root_turn.saturating_add(1);
        assert_eq!(policy.weights(&position, &choices), vec![2.0]);
    }

    #[test]
    fn artifact_rejects_nonfinite_coefficients() {
        let artifact = CombatActionImitationArtifactV1 {
            schema_name: COMBAT_ACTION_IMITATION_SCHEMA_NAME.to_string(),
            schema_version: COMBAT_ACTION_IMITATION_SCHEMA_VERSION,
            feature_schema: COMBAT_ACTION_FEATURE_SCHEMA.to_string(),
            training_authority: "test".to_string(),
            source_trajectory_count: 1,
            source_action_count: 1,
            source_terminal_final_hp: 1,
            ranked_decision_count: 1,
            pairwise_comparison_count: 1,
            skipped_forced_decision_count: 0,
            training_top1_correct: 0,
            training_top1_total: 1,
            logit_scale: 1.0,
            max_abs_log_factor: 3.0,
            base_weight_exponent: 0.0,
            coefficients: vec![CombatActionImitationCoefficientV1 {
                feature: "broken".to_string(),
                weight: f64::NAN,
            }],
        };
        assert!(artifact.validate().is_err());
    }
}
