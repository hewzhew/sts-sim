from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

from learning.tests.driver_fixtures import NoRecovery, NumpyFakeBatchEnv
from learning.tests.torch_outcome_fixtures import (
    behavior_manifest_template_fixture,
    completed_attempt_fixture,
    decision_batch_fixture,
)
from sts_learning import (
    AttemptAssemblyLimits,
    BehaviorManifestCatalogLimits,
    BehaviorManifestRegistry,
    BoundedAttemptAssembler,
    BoundedBehaviorManifestCatalog,
    ExperienceLimits,
    ExperienceSegmentBuffer,
    OnlineBatchDriver,
    SeedPartition,
    SeedSchedule,
    SemanticBatchConcatLimits,
    initialize_population,
)


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None

if _TORCH_AVAILABLE:
    import torch

    from learning.tests.semantic_fixtures import semantic_schema_fixture
    from sts_learning import AttemptAssemblyDelivery
    from sts_learning.torch_behavior import (
        CategoricalTorchBehaviorController,
        TorchBehaviorPublisher,
    )
    from sts_learning.torch_checkpoints import (
        BoundedTorchCheckpointStore,
        TorchCheckpointLimits,
    )
    from sts_learning.torch_generation import (
        BoundedCategoricalGenerationRunner,
        TorchGenerationError,
    )
    from sts_learning.torch_policy import (
        RaggedCandidateScorer,
        RaggedCategoricalPolicyConfig,
        RaggedScorerConfig,
    )
    from sts_learning.torch_training import SynchronousValueTrainer


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class BoundedCategoricalGenerationRunnerTests(unittest.TestCase):
    def test_miswired_generation_owners_fail_before_environment_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            driver, assembler, trainer, controller, shadow = _components(Path(root))
            disconnected = BoundedAttemptAssembler(
                _attempt_limits(),
                trainer,
            )

            with self.assertRaisesRegex(TorchGenerationError, "not wired"):
                BoundedCategoricalGenerationRunner(
                    driver,
                    disconnected,
                    trainer,
                    controller,
                    shadow,
                    optimizer_steps_per_generation=1,
                )

            self.assertEqual(driver.env.choose_calls, [])  # type: ignore[attr-defined]
            self.assertIs(assembler.completed_attempt_sink, trainer)

    def test_generation_optimizer_must_own_the_exact_shadow_parameters(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            other = _scorer()
            driver, assembler, trainer, controller, shadow = _components(
                Path(root),
                optimizer_model=other,
            )

            with self.assertRaisesRegex(TorchGenerationError, "exactly the shadow"):
                BoundedCategoricalGenerationRunner(
                    driver,
                    assembler,
                    trainer,
                    controller,
                    shadow,
                    optimizer_steps_per_generation=1,
                )

            self.assertEqual(driver.env.choose_calls, [])  # type: ignore[attr-defined]

    def test_generation_target_and_step_limit_require_typed_counts(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            driver, assembler, trainer, controller, shadow = _components(Path(root))
            with self.assertRaisesRegex(TorchGenerationError, "positive"):
                BoundedCategoricalGenerationRunner(
                    driver,
                    assembler,
                    trainer,
                    controller,
                    shadow,
                    optimizer_steps_per_generation=0,
                )
            runner = BoundedCategoricalGenerationRunner(
                driver,
                assembler,
                trainer,
                controller,
                shadow,
                optimizer_steps_per_generation=1,
            )

            with self.assertRaisesRegex(TorchGenerationError, "not bool"):
                runner.advance(max_batch_steps=True)

            driver.policy = object()  # type: ignore[assignment]
            with self.assertRaisesRegex(TorchGenerationError, "driver policy"):
                runner.advance(max_batch_steps=0)

            self.assertEqual(driver.env.choose_calls, [])  # type: ignore[attr-defined]

    def test_partial_optimizer_progress_promotes_without_replaying_environment(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            driver, assembler, trainer, controller, shadow = _components(Path(root))
            runner = BoundedCategoricalGenerationRunner(
                driver,
                assembler,
                trainer,
                controller,
                shadow,
                optimizer_steps_per_generation=2,
            )
            manifest_id = controller.snapshot.active_manifest_id
            assert manifest_id is not None

            trainer(_delivery(slot=1, manifest_id=manifest_id))
            partial = runner.advance(max_batch_steps=0)

            self.assertFalse(partial.promoted)
            self.assertTrue(partial.step_limit_reached)
            self.assertEqual(partial.optimizer_steps_before, 1)
            self.assertEqual(partial.promotion_target_training_step, 2)
            self.assertEqual(controller.snapshot.active_training_step, 0)

            trainer(_delivery(slot=2, manifest_id=manifest_id))
            promoted = runner.advance(max_batch_steps=0)

            self.assertTrue(promoted.promoted)
            self.assertEqual(promoted.batch_steps, 0)
            self.assertEqual(promoted.optimizer_steps_before, 2)
            self.assertEqual(promoted.optimizer_steps_after, 2)
            self.assertEqual(controller.snapshot.active_training_step, 2)
            self.assertEqual(driver.env.choose_calls, [])  # type: ignore[attr-defined]


def _components(
    root: Path,
    *,
    optimizer_model=None,
):
    shadow = _scorer()
    behavior_config = RaggedCategoricalPolicyConfig(temperature=0.8)
    registry = BehaviorManifestRegistry(capacity=2)
    store = BoundedTorchCheckpointStore(
        root / "checkpoints",
        TorchCheckpointLimits(
            max_checkpoints=2,
            max_bytes_per_checkpoint=2 * 1024 * 1024,
            max_total_bytes=4 * 1024 * 1024,
        ),
    )
    catalog = BoundedBehaviorManifestCatalog(
        root / "manifests",
        BehaviorManifestCatalogLimits(
            max_manifests=2,
            max_bytes_per_manifest=1024,
            max_total_bytes=2 * 1024,
        ),
    )

    def scorer_factory():
        return _scorer()

    controller = CategoricalTorchBehaviorController(
        TorchBehaviorPublisher(
            store,
            catalog,
            registry,
            behavior_manifest_template_fixture(
                behavior_rule=behavior_config.behavior_rule,
            ),
        ),
        scorer_factory,
        behavior_config,
        torch.Generator().manual_seed(94),
    )
    controller.publish_and_promote(shadow, training_step=0)
    optimizer_owner = shadow if optimizer_model is None else optimizer_model
    trainer = SynchronousValueTrainer(
        shadow,
        torch.optim.SGD(optimizer_owner.parameters(), lr=0.001),
        registry,
        SemanticBatchConcatLimits(
            max_rows=64,
            max_input_array_bytes=1024 * 1024,
        ),
    )
    assembler = BoundedAttemptAssembler(_attempt_limits(), trainer)
    population = initialize_population(
        NumpyFakeBatchEnv,
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
                max_decisions=8,
                max_payload_bytes=1024 * 1024,
            )
        ),
        experience_sink=assembler,
    )
    return driver, assembler, trainer, controller, shadow


def _attempt_limits() -> AttemptAssemblyLimits:
    return AttemptAssemblyLimits(
        max_open_attempts=1,
        max_decisions_per_attempt=64,
        max_payload_bytes_per_attempt=1024 * 1024,
    )


def _delivery(*, slot: int, manifest_id):
    batch = decision_batch_fixture(
        slot=slot,
        semantic_row=0,
        selected_ordinal=0,
        manifest_id=manifest_id,
    )
    return AttemptAssemblyDelivery(
        completed=(
            completed_attempt_fixture(
                slot=slot,
                batches=(batch,),
                reward=1,
            ),
        ),
        dropped=(),
    )


def _scorer():
    return RaggedCandidateScorer.from_bridge_schema(
        semantic_schema_fixture(),
        RaggedScorerConfig(hidden_dim=4, relation_layers=0),
    )


if __name__ == "__main__":
    unittest.main()
