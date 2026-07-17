use sts_core::runtime::combat::CombatState;
use sts_core::sim::combat::CombatPosition;

pub const COMBAT_OUTCOME_FEATURE_SCHEMA_V1: &str = "oracle-combat-outcome-features/v1";
const FEATURE_COUNT: usize = 12;
const PARAMETER_COUNT: usize = FEATURE_COUNT + 1;

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CombatOutcomeFeatureVectorV1(pub [f64; FEATURE_COUNT]);

impl CombatOutcomeFeatureVectorV1 {
    pub fn from_position(position: &CombatPosition) -> Self {
        Self::from_combat(&position.combat)
    }

    pub fn from_combat(combat: &CombatState) -> Self {
        let player = &combat.entities.player;
        let living = combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .collect::<Vec<_>>();
        let enemy_hp = living
            .iter()
            .map(|monster| monster.current_hp.max(0))
            .sum::<i32>();
        let enemy_max_hp = living
            .iter()
            .map(|monster| monster.max_hp.max(1))
            .sum::<i32>()
            .max(1);
        let enemy_block = living
            .iter()
            .map(|monster| monster.block.max(0))
            .sum::<i32>();
        let potion_slots = combat.entities.potions.len().max(1);
        Self([
            ratio(player.current_hp.max(0), player.max_hp.max(1)),
            ratio(player.block.max(0), player.max_hp.max(1)),
            f64::from(combat.turn.energy) / 5.0,
            f64::from(combat.turn.turn_count) / 20.0,
            living.len() as f64 / 5.0,
            ratio(enemy_hp, enemy_max_hp),
            ratio(enemy_block, enemy_max_hp),
            combat.zones.hand.len() as f64 / 10.0,
            combat.zones.draw_pile.len() as f64 / 40.0,
            combat.zones.discard_pile.len() as f64 / 40.0,
            combat.zones.exhaust_pile.len() as f64 / 40.0,
            combat.entities.potions.iter().flatten().count() as f64 / potion_slots as f64,
        ])
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Deserialize, serde::Serialize)]
pub enum CombatOutcomeLabelProvenanceV1 {
    RealizedBehaviorCombat,
    ExactScenarioReplay,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CombatOutcomeTrainingExampleV1 {
    pub features: CombatOutcomeFeatureVectorV1,
    pub victory: bool,
    pub terminal_hp_fraction: f64,
    pub provenance: CombatOutcomeLabelProvenanceV1,
    pub continuation_policy_manifest: String,
}

impl CombatOutcomeTrainingExampleV1 {
    pub fn from_position(
        position: &CombatPosition,
        victory: bool,
        terminal_hp: i32,
        terminal_max_hp: i32,
        provenance: CombatOutcomeLabelProvenanceV1,
        continuation_policy_manifest: impl Into<String>,
    ) -> Self {
        Self {
            features: CombatOutcomeFeatureVectorV1::from_position(position),
            victory,
            terminal_hp_fraction: ratio(terminal_hp.max(0), terminal_max_hp.max(1)),
            provenance,
            continuation_policy_manifest: continuation_policy_manifest.into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CombatOutcomeModelTrainingConfigV1 {
    pub epochs: usize,
    pub learning_rate: f64,
    pub l2_penalty: f64,
    pub minimum_training_examples: usize,
    pub minimum_calibration_examples: usize,
    pub maximum_calibration_brier: f64,
    pub maximum_goal_probability_error_p95: f64,
}

impl Default for CombatOutcomeModelTrainingConfigV1 {
    fn default() -> Self {
        Self {
            epochs: 600,
            learning_rate: 0.1,
            l2_penalty: 0.0001,
            minimum_training_examples: 16,
            minimum_calibration_examples: 8,
            maximum_calibration_brier: 0.24,
            maximum_goal_probability_error_p95: 0.35,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatOutcomeModelErrorV1 {
    EmptyModelId,
    EmptyContinuationPolicyManifest,
    InsufficientTrainingExamples { provided: usize, required: usize },
    InsufficientCalibrationExamples { provided: usize, required: usize },
    TrainingMissingOutcomeClass,
    CalibrationMissingOutcomeClass,
    ContinuationPolicyMismatch,
    NonFiniteExample,
    InvalidTrainingConfig,
}

#[derive(Clone, Debug, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct CombatOutcomeModelV1 {
    model_id: String,
    continuation_policy_manifest: String,
    win_parameters: [f64; PARAMETER_COUNT],
    terminal_hp_parameters: [f64; PARAMETER_COUNT],
    feature_minimums: [f64; FEATURE_COUNT],
    feature_maximums: [f64; FEATURE_COUNT],
    training_examples: usize,
    calibration_examples: usize,
    calibration_brier: f64,
    goal_probability_error_p95: f64,
    terminal_hp_mae: f64,
    calibration_accepted: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatOutcomeModelApplicabilityV1 {
    InDomain,
    CalibrationRejected,
    OutOfDomain { feature_indices: Vec<usize> },
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatOutcomeModelEpistemicV1 {
    pub training_examples: usize,
    pub calibration_examples: usize,
    pub calibration_brier: f64,
    pub goal_probability_error_p95: f64,
    pub terminal_hp_mae: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CombatOutcomeProbabilityIntervalV1 {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatOutcomeEstimateV1 {
    pub model_id: String,
    pub feature_schema_id: &'static str,
    pub continuation_policy_manifest: String,
    pub goal_success_probability: f64,
    pub goal_success_interval: CombatOutcomeProbabilityIntervalV1,
    pub terminal_hp_fraction_mean: f64,
    pub applicability: CombatOutcomeModelApplicabilityV1,
    pub epistemic: CombatOutcomeModelEpistemicV1,
}

impl CombatOutcomeModelV1 {
    pub fn fit(
        model_id: impl Into<String>,
        continuation_policy_manifest: impl Into<String>,
        training: &[CombatOutcomeTrainingExampleV1],
        calibration: &[CombatOutcomeTrainingExampleV1],
        config: CombatOutcomeModelTrainingConfigV1,
    ) -> Result<Self, CombatOutcomeModelErrorV1> {
        let model_id = model_id.into();
        if model_id.trim().is_empty() {
            return Err(CombatOutcomeModelErrorV1::EmptyModelId);
        }
        let continuation_policy_manifest = continuation_policy_manifest.into();
        if continuation_policy_manifest.trim().is_empty() {
            return Err(CombatOutcomeModelErrorV1::EmptyContinuationPolicyManifest);
        }
        validate_config(config)?;
        if training.len() < config.minimum_training_examples {
            return Err(CombatOutcomeModelErrorV1::InsufficientTrainingExamples {
                provided: training.len(),
                required: config.minimum_training_examples,
            });
        }
        if calibration.len() < config.minimum_calibration_examples {
            return Err(CombatOutcomeModelErrorV1::InsufficientCalibrationExamples {
                provided: calibration.len(),
                required: config.minimum_calibration_examples,
            });
        }
        validate_examples(training)?;
        validate_examples(calibration)?;
        if !contains_both_outcomes(training) {
            return Err(CombatOutcomeModelErrorV1::TrainingMissingOutcomeClass);
        }
        if !contains_both_outcomes(calibration) {
            return Err(CombatOutcomeModelErrorV1::CalibrationMissingOutcomeClass);
        }
        if training
            .iter()
            .chain(calibration)
            .any(|example| example.continuation_policy_manifest != continuation_policy_manifest)
        {
            return Err(CombatOutcomeModelErrorV1::ContinuationPolicyMismatch);
        }

        let win_parameters = train_logistic(training, config);
        let terminal_hp_parameters = train_terminal_hp(training, config);
        let (feature_minimums, feature_maximums) = feature_ranges(training);
        let calibration_brier = calibration
            .iter()
            .map(|example| {
                let predicted = sigmoid(dot(win_parameters, example.features));
                let target = f64::from(example.victory);
                (predicted - target).powi(2)
            })
            .sum::<f64>()
            / calibration.len() as f64;
        let mut goal_probability_errors = calibration
            .iter()
            .map(|example| {
                let predicted = sigmoid(dot(win_parameters, example.features));
                let target = f64::from(example.victory);
                (predicted - target).abs()
            })
            .collect::<Vec<_>>();
        goal_probability_errors.sort_by(f64::total_cmp);
        let p95_index = goal_probability_errors
            .len()
            .saturating_mul(95)
            .div_ceil(100)
            .saturating_sub(1);
        let goal_probability_error_p95 = goal_probability_errors[p95_index];
        let victorious = calibration
            .iter()
            .filter(|example| example.victory)
            .collect::<Vec<_>>();
        let terminal_hp_mae = victorious
            .iter()
            .map(|example| {
                (dot(terminal_hp_parameters, example.features).clamp(0.0, 1.0)
                    - example.terminal_hp_fraction)
                    .abs()
            })
            .sum::<f64>()
            / victorious.len().max(1) as f64;
        Ok(Self {
            model_id,
            continuation_policy_manifest,
            win_parameters,
            terminal_hp_parameters,
            feature_minimums,
            feature_maximums,
            training_examples: training.len(),
            calibration_examples: calibration.len(),
            calibration_brier,
            goal_probability_error_p95,
            terminal_hp_mae,
            calibration_accepted: calibration_brier <= config.maximum_calibration_brier
                && goal_probability_error_p95 <= config.maximum_goal_probability_error_p95,
        })
    }

    pub fn evaluate(&self, position: &CombatPosition) -> CombatOutcomeEstimateV1 {
        let features = CombatOutcomeFeatureVectorV1::from_position(position);
        let out_of_domain = (0..FEATURE_COUNT)
            .filter(|index| {
                features.0[*index] < self.feature_minimums[*index] - f64::EPSILON
                    || features.0[*index] > self.feature_maximums[*index] + f64::EPSILON
            })
            .collect::<Vec<_>>();
        let applicability = if !self.calibration_accepted {
            CombatOutcomeModelApplicabilityV1::CalibrationRejected
        } else if out_of_domain.is_empty() {
            CombatOutcomeModelApplicabilityV1::InDomain
        } else {
            CombatOutcomeModelApplicabilityV1::OutOfDomain {
                feature_indices: out_of_domain,
            }
        };
        let goal_success_probability = sigmoid(dot(self.win_parameters, features));
        CombatOutcomeEstimateV1 {
            model_id: self.model_id.clone(),
            feature_schema_id: COMBAT_OUTCOME_FEATURE_SCHEMA_V1,
            continuation_policy_manifest: self.continuation_policy_manifest.clone(),
            goal_success_probability,
            goal_success_interval: CombatOutcomeProbabilityIntervalV1 {
                lower: (goal_success_probability - self.goal_probability_error_p95).clamp(0.0, 1.0),
                upper: (goal_success_probability + self.goal_probability_error_p95).clamp(0.0, 1.0),
            },
            terminal_hp_fraction_mean: dot(self.terminal_hp_parameters, features).clamp(0.0, 1.0),
            applicability,
            epistemic: CombatOutcomeModelEpistemicV1 {
                training_examples: self.training_examples,
                calibration_examples: self.calibration_examples,
                calibration_brier: self.calibration_brier,
                goal_probability_error_p95: self.goal_probability_error_p95,
                terminal_hp_mae: self.terminal_hp_mae,
            },
        }
    }
}

fn validate_config(
    config: CombatOutcomeModelTrainingConfigV1,
) -> Result<(), CombatOutcomeModelErrorV1> {
    if config.epochs == 0
        || !config.learning_rate.is_finite()
        || config.learning_rate <= 0.0
        || !config.l2_penalty.is_finite()
        || config.l2_penalty < 0.0
        || config.minimum_training_examples == 0
        || config.minimum_calibration_examples == 0
        || !config.maximum_calibration_brier.is_finite()
        || !(0.0..=1.0).contains(&config.maximum_calibration_brier)
        || !config.maximum_goal_probability_error_p95.is_finite()
        || !(0.0..=1.0).contains(&config.maximum_goal_probability_error_p95)
    {
        return Err(CombatOutcomeModelErrorV1::InvalidTrainingConfig);
    }
    Ok(())
}

fn validate_examples(
    examples: &[CombatOutcomeTrainingExampleV1],
) -> Result<(), CombatOutcomeModelErrorV1> {
    if examples.iter().any(|example| {
        example.features.0.iter().any(|value| !value.is_finite())
            || !example.terminal_hp_fraction.is_finite()
            || !(0.0..=1.0).contains(&example.terminal_hp_fraction)
    }) {
        return Err(CombatOutcomeModelErrorV1::NonFiniteExample);
    }
    Ok(())
}

fn contains_both_outcomes(examples: &[CombatOutcomeTrainingExampleV1]) -> bool {
    examples.iter().any(|example| example.victory)
        && examples.iter().any(|example| !example.victory)
}

fn train_logistic(
    examples: &[CombatOutcomeTrainingExampleV1],
    config: CombatOutcomeModelTrainingConfigV1,
) -> [f64; PARAMETER_COUNT] {
    let mut parameters = [0.0; PARAMETER_COUNT];
    for _ in 0..config.epochs {
        let mut gradient = [0.0; PARAMETER_COUNT];
        for example in examples {
            let error = sigmoid(dot(parameters, example.features)) - f64::from(example.victory);
            gradient[0] += error;
            for index in 0..FEATURE_COUNT {
                gradient[index + 1] += error * example.features.0[index];
            }
        }
        let scale = config.learning_rate / examples.len() as f64;
        parameters[0] -= scale * gradient[0];
        for index in 1..PARAMETER_COUNT {
            parameters[index] -= scale * (gradient[index] + config.l2_penalty * parameters[index]);
        }
    }
    parameters
}

fn train_terminal_hp(
    examples: &[CombatOutcomeTrainingExampleV1],
    config: CombatOutcomeModelTrainingConfigV1,
) -> [f64; PARAMETER_COUNT] {
    let victories = examples
        .iter()
        .filter(|example| example.victory)
        .collect::<Vec<_>>();
    let mut parameters = [0.0; PARAMETER_COUNT];
    for _ in 0..config.epochs {
        let mut gradient = [0.0; PARAMETER_COUNT];
        for example in &victories {
            let error = dot(parameters, example.features) - example.terminal_hp_fraction;
            gradient[0] += error;
            for index in 0..FEATURE_COUNT {
                gradient[index + 1] += error * example.features.0[index];
            }
        }
        let scale = config.learning_rate / victories.len().max(1) as f64;
        parameters[0] -= scale * gradient[0];
        for index in 1..PARAMETER_COUNT {
            parameters[index] -= scale * (gradient[index] + config.l2_penalty * parameters[index]);
        }
    }
    parameters
}

fn feature_ranges(
    examples: &[CombatOutcomeTrainingExampleV1],
) -> ([f64; FEATURE_COUNT], [f64; FEATURE_COUNT]) {
    let mut minimums = [f64::INFINITY; FEATURE_COUNT];
    let mut maximums = [f64::NEG_INFINITY; FEATURE_COUNT];
    for example in examples {
        for index in 0..FEATURE_COUNT {
            minimums[index] = minimums[index].min(example.features.0[index]);
            maximums[index] = maximums[index].max(example.features.0[index]);
        }
    }
    (minimums, maximums)
}

fn dot(parameters: [f64; PARAMETER_COUNT], features: CombatOutcomeFeatureVectorV1) -> f64 {
    parameters[0]
        + features
            .0
            .iter()
            .enumerate()
            .map(|(index, value)| parameters[index + 1] * value)
            .sum::<f64>()
}

fn sigmoid(value: f64) -> f64 {
    if value >= 0.0 {
        1.0 / (1.0 + (-value).exp())
    } else {
        let exp = value.exp();
        exp / (1.0 + exp)
    }
}

fn ratio(numerator: i32, denominator: i32) -> f64 {
    f64::from(numerator) / f64::from(denominator.max(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_core::content::monsters::EnemyId;
    use sts_core::sim::combat::CombatPosition;
    use sts_core::state::core::EngineState;

    fn position(player_hp: i32, enemy_hp: i32, turn: u32) -> CombatPosition {
        let mut combat = sts_core::test_support::blank_test_combat();
        combat.entities.player.max_hp = 80;
        combat.entities.player.current_hp = player_hp;
        let mut monster = sts_core::test_support::test_monster(EnemyId::JawWorm);
        monster.max_hp = 80;
        monster.current_hp = enemy_hp;
        combat.entities.monsters = vec![monster];
        combat.turn.turn_count = turn;
        CombatPosition::new(EngineState::CombatPlayerTurn, combat)
    }

    fn example(
        player_hp: i32,
        enemy_hp: i32,
        turn: u32,
        victory: bool,
    ) -> CombatOutcomeTrainingExampleV1 {
        CombatOutcomeTrainingExampleV1::from_position(
            &position(player_hp, enemy_hp, turn),
            victory,
            if victory { player_hp - 3 } else { 0 },
            80,
            CombatOutcomeLabelProvenanceV1::ExactScenarioReplay,
            "exact-test-continuation-policy-v1",
        )
    }

    fn fitted_model() -> CombatOutcomeModelV1 {
        let training = (0..24)
            .flat_map(|index| {
                [
                    example(62 + index % 6, 8 + index % 5, (2 + index % 3) as u32, true),
                    example(8 + index % 5, 62 + index % 6, (8 + index % 3) as u32, false),
                ]
            })
            .collect::<Vec<_>>();
        let calibration = (0..8)
            .flat_map(|index| {
                [
                    example(63 + index % 4, 9 + index % 3, (2 + index % 2) as u32, true),
                    example(9 + index % 3, 63 + index % 4, (8 + index % 2) as u32, false),
                ]
            })
            .collect::<Vec<_>>();
        CombatOutcomeModelV1::fit(
            "synthetic-outcome-model-v1",
            "exact-test-continuation-policy-v1",
            &training,
            &calibration,
            CombatOutcomeModelTrainingConfigV1::default(),
        )
        .unwrap()
    }

    #[test]
    fn learned_outcome_estimate_prefers_the_favorable_long_term_state() {
        let model = fitted_model();
        let favorable = model.evaluate(&position(65, 10, 2));
        let dangerous = model.evaluate(&position(10, 65, 9));

        assert_eq!(
            favorable.applicability,
            CombatOutcomeModelApplicabilityV1::InDomain
        );
        assert_eq!(
            dangerous.applicability,
            CombatOutcomeModelApplicabilityV1::InDomain
        );
        assert!(favorable.goal_success_probability > dangerous.goal_success_probability);
        assert!(favorable.terminal_hp_fraction_mean > dangerous.terminal_hp_fraction_mean);
        assert_eq!(favorable.model_id, "synthetic-outcome-model-v1");
    }

    #[test]
    fn evaluation_reports_distribution_shift_instead_of_silently_extrapolating() {
        let model = fitted_model();
        let estimate = model.evaluate(&position(80, 80, 40));

        assert!(matches!(
            estimate.applicability,
            CombatOutcomeModelApplicabilityV1::OutOfDomain { .. }
        ));
    }

    #[test]
    fn training_rejects_single_outcome_labels() {
        let samples = (0..20)
            .map(|index| example(60, 10 + index, 2, true))
            .collect::<Vec<_>>();
        assert_eq!(
            CombatOutcomeModelV1::fit(
                "invalid",
                "policy",
                &samples,
                &samples,
                CombatOutcomeModelTrainingConfigV1::default(),
            ),
            Err(CombatOutcomeModelErrorV1::TrainingMissingOutcomeClass)
        );
    }
}
