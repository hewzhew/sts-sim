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
        CategoricalTorchBehaviorController,
        TorchBehaviorPublisher,
    )
    from sts_learning.torch_checkpoints import (
        BoundedTorchCheckpointStore,
        TorchCheckpointLimits,
    )
    from sts_learning.torch_generation import BoundedCategoricalGenerationRunner
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

    def test_categorical_behavior_promotes_one_bounded_online_generation(self) -> None:
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
        registry = BehaviorManifestRegistry(capacity=2)
        with tempfile.TemporaryDirectory() as root:
            store = BoundedTorchCheckpointStore(
                Path(root, "checkpoints"),
                TorchCheckpointLimits(
                    max_checkpoints=2,
                    max_bytes_per_checkpoint=2 * 1024 * 1024,
                    max_total_bytes=4 * 1024 * 1024,
                ),
            )
            catalog = BoundedBehaviorManifestCatalog(
                Path(root, "manifests"),
                BehaviorManifestCatalogLimits(
                    max_manifests=2,
                    max_bytes_per_manifest=1024,
                    max_total_bytes=2 * 1024,
                ),
            )
            publisher = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(
                    semantic_schema_version=int(schema["version"]),
                    behavior_rule=behavior_config.behavior_rule,
                ),
            )
            behavior_generator = torch.Generator().manual_seed(94)
            controller = CategoricalTorchBehaviorController(
                publisher,
                scorer_factory,
                behavior_config,
                behavior_generator,
            )
            generation_zero = controller.publish_and_promote(
                shadow,
                training_step=0,
            )
            trainer = SynchronousValueTrainer(
                shadow,
                torch.optim.SGD(shadow.parameters(), lr=0.001),
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
            population = initialize_population(
                LearningBatchEnv,
                slot_count=1,
                schedule=SeedSchedule(SeedPartition.HELD_OUT),
                max_recoveries_per_episode=0,
            )
            driver = OnlineBatchDriver(
                population,
                policy=controller,
                curriculum=NoRecovery(),
                experience_buffer=ExperienceSegmentBuffer(
                    ExperienceLimits(
                        max_decisions=16,
                        max_payload_bytes=4 * 1024 * 1024,
                    )
                ),
                experience_sink=assembler,
            )
            generation_runner = BoundedCategoricalGenerationRunner(
                driver,
                assembler,
                trainer,
                controller,
                shadow,
                optimizer_steps_per_generation=1,
            )

            with warnings.catch_warnings():
                warnings.simplefilter("error")
                partial_generation = generation_runner.advance(max_batch_steps=1)
                self.assertFalse(partial_generation.promoted)
                self.assertTrue(partial_generation.step_limit_reached)
                self.assertEqual(partial_generation.optimizer_steps_after, 0)
                self.assertEqual(
                    controller.snapshot.active_manifest_id,
                    generation_zero.manifest_id,
                )
                promoted_generation = generation_runner.advance(max_batch_steps=159)
                self.assertTrue(promoted_generation.promoted)
                self.assertEqual(
                    promoted_generation.active_manifest_id_before,
                    generation_zero.manifest_id,
                )
                self.assertEqual(promoted_generation.active_training_step_before, 0)
                self.assertEqual(promoted_generation.optimizer_steps_before, 0)
                self.assertEqual(promoted_generation.optimizer_steps_after, 1)
                self.assertEqual(
                    promoted_generation.promotion_target_training_step,
                    1,
                )
                self.assertEqual(promoted_generation.terminal_attempts, 1)
                self.assertEqual(promoted_generation.terminal_flushes, 1)
                generation_one = promoted_generation.publication
                assert generation_one is not None
                generation_after_first = driver.ledger.snapshot(0).episode_generation
                first_training = trainer.snapshot
                first_manifest_evidence = first_training.last_behavior_manifest_ids
                first_probability_evidence = first_training.last_selection_probabilities

                self.assertEqual(first_training.optimizer_steps, 1)
                self.assertIsNotNone(first_manifest_evidence)
                self.assertIsNotNone(first_probability_evidence)
                assert first_manifest_evidence is not None
                assert first_probability_evidence is not None
                self.assertTrue(
                    all(
                        manifest_id == generation_zero.manifest_id
                        for attempt in first_manifest_evidence
                        for manifest_id in attempt
                    )
                )
                self.assertTrue(
                    all(
                        probability.value is not None
                        for attempt in first_probability_evidence
                        for probability in attempt
                    )
                )

                second_run = driver.run_until_terminal_attempts(
                    terminal_attempts=1,
                    max_batch_steps=(
                        160
                        - partial_generation.batch_steps
                        - promoted_generation.batch_steps
                    ),
                )
                self.assertTrue(second_run.target_reached)
                self.assertEqual(second_run.summary.terminal_attempts, 1)
                self.assertLessEqual(
                    partial_generation.batch_steps
                    + promoted_generation.batch_steps
                    + second_run.summary.batch_steps,
                    160,
                )
                driver.flush_experience()

            second_training = trainer.snapshot
            second_manifest_evidence = second_training.last_behavior_manifest_ids
            second_probability_evidence = second_training.last_selection_probabilities

            self.assertNotEqual(generation_zero.manifest_id, generation_one.manifest_id)
            self.assertEqual(
                driver.ledger.snapshot(0).episode_generation,
                generation_after_first + 1,
            )
            self.assertEqual(
                first_manifest_evidence,
                ((generation_zero.manifest_id,) * len(first_manifest_evidence[0]),),
            )
            self.assertEqual(second_training.optimizer_steps, 2)
            self.assertIsNotNone(second_manifest_evidence)
            self.assertIsNotNone(second_probability_evidence)
            assert second_manifest_evidence is not None
            assert second_probability_evidence is not None
            self.assertTrue(
                all(
                    manifest_id == generation_one.manifest_id
                    for attempt in second_manifest_evidence
                    for manifest_id in attempt
                )
            )
            self.assertTrue(
                all(
                    probability.value is not None
                    for attempt in second_probability_evidence
                    for probability in attempt
                )
            )
            self.assertEqual(
                controller.snapshot.active_manifest_id,
                generation_one.manifest_id,
            )
            self.assertEqual(controller.snapshot.active_training_step, 1)
            self.assertEqual(controller.snapshot.successful_promotions, 2)
            self.assertEqual(store.snapshot.checkpoints, 2)
            self.assertEqual(catalog.snapshot.manifests, 2)
            self.assertEqual(registry.snapshot.registered_manifests, 2)

            recovered_generator = torch.Generator()
            recovered_generator.set_state(behavior_generator.get_state())
            recovered = CategoricalTorchBehaviorController(
                TorchBehaviorPublisher(
                    store,
                    catalog,
                    BehaviorManifestRegistry(capacity=1),
                    behavior_manifest_template_fixture(
                        semantic_schema_version=int(schema["version"]),
                        behavior_rule=behavior_config.behavior_rule,
                    ),
                ),
                scorer_factory,
                behavior_config,
                recovered_generator,
            )
            recovered_publication = recovered.recover_and_promote(
                generation_one.manifest_id
            )
            next_decision = driver.env.decision_batch(semantic=True)

            self.assertEqual(recovered_publication, generation_one)
            self.assertEqual(
                controller.choose(next_decision),
                recovered.choose(next_decision),
            )

        self.assertEqual(assembler.snapshot.completed_attempts, 2)
        self.assertEqual(assembler.snapshot.dropped_attempts, 0)
        self.assertTrue(
            any(
                probability.value != 1.0
                for evidence in (
                    first_probability_evidence,
                    second_probability_evidence,
                )
                for attempt in evidence
                for probability in attempt
            )
        )
        self.assertFalse(trainer.snapshot.poisoned)


if __name__ == "__main__":
    unittest.main()
