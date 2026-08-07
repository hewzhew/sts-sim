from __future__ import annotations

import importlib.util
import tempfile
import unittest
import warnings
from collections.abc import Mapping
from dataclasses import replace
from pathlib import Path

from learning.tests.driver_fixtures import NoRecovery
from learning.tests.torch_outcome_fixtures import (
    behavior_manifest_fixture,
    behavior_manifest_template_fixture,
)
from sts_learning import (
    AttemptAssemblyLimits,
    BatchPolicyChoice,
    BehaviorManifestCatalogLimits,
    BehaviorManifestId,
    BehaviorManifestRegistry,
    BoundedAttemptAssembler,
    BoundedBehaviorManifestCatalog,
    ExperienceLimits,
    ExperienceSegmentBuffer,
    OnlineBatchDriver,
    SemanticBatchConcatLimits,
    SeedPartition,
    SeedSchedule,
    initialize_population,
)


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None
try:
    from sts_learning_bridge import LearningBatchEnv, semantic_schema
except ImportError:
    LearningBatchEnv = None  # type: ignore[assignment,misc]
    semantic_schema = None  # type: ignore[assignment]

if _TORCH_AVAILABLE:
    import torch

    from sts_learning.torch_behavior import (
        CheckpointedCategoricalTorchPolicy,
        TorchBehaviorPublisher,
    )
    from sts_learning.torch_checkpoints import (
        BoundedTorchCheckpointStore,
        TorchCheckpointLimits,
    )
    from sts_learning.torch_policy import (
        RaggedCandidateScorer,
        RaggedCategoricalPolicyConfig,
        RaggedScorerConfig,
    )
    from sts_learning.torch_training import SynchronousValueTrainer


class RegisteredFirstLegalPolicy:
    def __init__(self, manifest_id: BehaviorManifestId) -> None:
        self.manifest_id = manifest_id

    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        return BatchPolicyChoice.deterministic(
            [0] * len(decision_batch["slot_indices"]),  # type: ignore[arg-type]
            self.manifest_id,
        )


class CountingScorer:
    def __init__(self, scorer) -> None:
        self.scorer = scorer
        self.calls = 0

    def __call__(self, decision_batch):
        self.calls += 1
        return self.scorer(decision_batch)


class ProbabilityObservingTrainer:
    """Retain only aggregate propensity facts before synchronous training."""

    def __init__(self, trainer) -> None:
        self.trainer = trainer
        self.observed_probabilities = 0
        self.saw_non_unit_probability = False
        self.saw_unknown_probability = False

    def __call__(self, delivery) -> None:
        for attempt in delivery.completed:
            for batch in attempt.batches:
                for probability in batch.selection_probabilities:
                    self.observed_probabilities += 1
                    if probability.value is None:
                        self.saw_unknown_probability = True
                    elif probability.value != 1.0:
                        self.saw_non_unit_probability = True
        self.trainer(delivery)


@unittest.skipUnless(
    _TORCH_AVAILABLE and LearningBatchEnv is not None,
    "optional PyTorch dependency or standalone bridge wheel is not installed",
)
class RealBridgeOnlineTrainingTests(unittest.TestCase):
    def test_complete_attempt_stream_performs_bounded_shadow_value_updates(self) -> None:
        assert LearningBatchEnv is not None
        assert semantic_schema is not None
        schema = semantic_schema()
        registry = BehaviorManifestRegistry(capacity=1)
        behavior_manifest = replace(
            behavior_manifest_fixture(),
            semantic_schema_version=int(schema["version"]),
        )
        behavior_manifest_id = registry.register(behavior_manifest)
        scorer = RaggedCandidateScorer.from_bridge_schema(
            schema,
            RaggedScorerConfig(hidden_dim=8, relation_layers=0),
        )
        counting_scorer = CountingScorer(scorer)
        trainer = SynchronousValueTrainer(
            counting_scorer,
            torch.optim.SGD(scorer.parameters(), lr=0.001),
            registry,
            SemanticBatchConcatLimits(
                max_rows=1_024,
                max_input_array_bytes=32 * 1024 * 1024,
            ),
        )
        assembler = BoundedAttemptAssembler(
            AttemptAssemblyLimits(
                max_open_attempts=1,
                max_decisions_per_attempt=1_024,
                max_payload_bytes_per_attempt=32 * 1024 * 1024,
            ),
            trainer,
        )
        experience_limits = ExperienceLimits(
            max_decisions=16,
            max_payload_bytes=4 * 1024 * 1024,
        )
        population = initialize_population(
            LearningBatchEnv,
            slot_count=1,
            schedule=SeedSchedule(SeedPartition.HELD_OUT),
            max_recoveries_per_episode=0,
        )
        driver = OnlineBatchDriver(
            population,
            policy=RegisteredFirstLegalPolicy(behavior_manifest_id),
            curriculum=NoRecovery(),
            experience_buffer=ExperienceSegmentBuffer(experience_limits),
            experience_sink=assembler,
        )

        with warnings.catch_warnings():
            warnings.simplefilter("error")
            summary = driver.run(batch_steps=160)
            driver.flush_experience()

        self.assertGreater(summary.terminal_attempts, 0)
        self.assertEqual(assembler.snapshot.dropped_attempts, 0)
        self.assertGreater(trainer.snapshot.optimizer_steps, 0)
        self.assertEqual(
            trainer.snapshot.completed_attempts,
            assembler.snapshot.completed_attempts,
        )
        self.assertGreater(trainer.snapshot.trained_decisions, 0)
        self.assertEqual(counting_scorer.calls, trainer.snapshot.optimizer_steps)
        self.assertGreater(
            trainer.snapshot.trained_decisions,
            counting_scorer.calls,
        )
        probabilities = trainer.snapshot.last_selection_probabilities
        self.assertIsNotNone(probabilities)
        assert probabilities is not None
        self.assertTrue(
            all(
                probability.value == 1.0
                for attempt in probabilities
                for probability in attempt
            )
        )
        self.assertGreater(trainer.snapshot.total_training_seconds, 0.0)
        self.assertFalse(trainer.snapshot.poisoned)

    def test_categorical_behavior_reaches_bounded_online_value_training(self) -> None:
        assert LearningBatchEnv is not None
        assert semantic_schema is not None
        # This is deliberately end-to-end: publication, bridge decoding,
        # bounded retention, attempt closure, and training are one contract.
        schema = semantic_schema()
        scorer_config = RaggedScorerConfig(hidden_dim=8, relation_layers=0)
        behavior_config = RaggedCategoricalPolicyConfig(temperature=0.8)

        def scorer_factory():
            return RaggedCandidateScorer.from_bridge_schema(schema, scorer_config)

        torch.manual_seed(43)
        shadow = scorer_factory()
        registry = BehaviorManifestRegistry(capacity=1)
        with tempfile.TemporaryDirectory() as root:
            store = BoundedTorchCheckpointStore(
                Path(root, "checkpoints"),
                TorchCheckpointLimits(
                    max_checkpoints=1,
                    max_bytes_per_checkpoint=2 * 1024 * 1024,
                    max_total_bytes=2 * 1024 * 1024,
                ),
            )
            catalog = BoundedBehaviorManifestCatalog(
                Path(root, "manifests"),
                BehaviorManifestCatalogLimits(
                    max_manifests=1,
                    max_bytes_per_manifest=1024,
                    max_total_bytes=1024,
                ),
            )
            publication = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(
                    semantic_schema_version=int(schema["version"]),
                    behavior_rule=behavior_config.behavior_rule,
                ),
            ).publish(shadow, training_step=0)
            policy = CheckpointedCategoricalTorchPolicy.promote(
                publication,
                store,
                catalog,
                registry,
                scorer_factory,
                behavior_config,
                torch.Generator().manual_seed(94),
            )
            counting_scorer = CountingScorer(shadow)
            trainer = SynchronousValueTrainer(
                counting_scorer,
                torch.optim.SGD(shadow.parameters(), lr=0.001),
                registry,
                SemanticBatchConcatLimits(
                    max_rows=1_024,
                    max_input_array_bytes=32 * 1024 * 1024,
                ),
            )
            observing_trainer = ProbabilityObservingTrainer(trainer)
            assembler = BoundedAttemptAssembler(
                AttemptAssemblyLimits(
                    max_open_attempts=1,
                    max_decisions_per_attempt=1_024,
                    max_payload_bytes_per_attempt=32 * 1024 * 1024,
                ),
                observing_trainer,
            )
            population = initialize_population(
                LearningBatchEnv,
                slot_count=1,
                schedule=SeedSchedule(SeedPartition.HELD_OUT),
                max_recoveries_per_episode=0,
            )
            driver = OnlineBatchDriver(
                population,
                policy=policy,
                curriculum=NoRecovery(),
                experience_buffer=ExperienceSegmentBuffer(
                    ExperienceLimits(
                        max_decisions=16,
                        max_payload_bytes=4 * 1024 * 1024,
                    )
                ),
                experience_sink=assembler,
            )

            with warnings.catch_warnings():
                warnings.simplefilter("error")
                summary = driver.run(batch_steps=160)
                driver.flush_experience()

        self.assertGreater(summary.terminal_attempts, 0)
        self.assertGreater(trainer.snapshot.optimizer_steps, 0)
        self.assertEqual(assembler.snapshot.dropped_attempts, 0)
        self.assertEqual(counting_scorer.calls, trainer.snapshot.optimizer_steps)
        self.assertGreater(observing_trainer.observed_probabilities, 0)
        self.assertTrue(observing_trainer.saw_non_unit_probability)
        self.assertFalse(observing_trainer.saw_unknown_probability)
        self.assertFalse(trainer.snapshot.poisoned)


if __name__ == "__main__":
    unittest.main()
