from __future__ import annotations

import importlib.util
import unittest

from learning.tests.torch_outcome_fixtures import (
    behavior_manifest_fixture,
    completed_attempt_fixture,
    decision_batch_fixture,
)
from sts_learning import (
    BehaviorManifestRegistry,
    SelectionProbability,
    SemanticBatchConcatLimits,
)


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None
if _TORCH_AVAILABLE:
    import torch

    from sts_learning import AttemptAssemblyDelivery
    from sts_learning.torch_policy import RaggedCandidateLogits
    from sts_learning.torch_training import (
        SynchronousValueTrainer,
        TorchTrainingError,
    )


CONCAT_LIMITS = SemanticBatchConcatLimits(
    max_rows=16,
    max_input_array_bytes=1024 * 1024,
)


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class SynchronousValueTrainerTests(unittest.TestCase):
    def test_delivery_updates_once_without_retaining_attempt_payload(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)
        manifest_id = registry.register(behavior_manifest_fixture())
        parameter = torch.nn.Parameter(torch.tensor([0.0, 5.0]))

        def scorer(payload):
            return RaggedCandidateLogits(
                values=parameter,
                row_splits=torch.tensor([0, 2]),
            )

        trainer = SynchronousValueTrainer(
            scorer,
            torch.optim.SGD([parameter], lr=0.1),
            registry,
            CONCAT_LIMITS,
        )
        batch = decision_batch_fixture(
            slot=1,
            semantic_row=0,
            selected_ordinal=0,
            manifest_id=manifest_id,
            selection_probability=SelectionProbability.known(0.3),
        )
        delivery = AttemptAssemblyDelivery(
            completed=(
                completed_attempt_fixture(slot=1, batches=(batch,), reward=1),
            ),
            dropped=(),
        )

        trainer(delivery)

        self.assertAlmostEqual(parameter[0].item(), 0.2, places=6)
        self.assertEqual(parameter[1].item(), 5.0)
        self.assertEqual(trainer.snapshot.deliveries, 1)
        self.assertEqual(trainer.snapshot.optimizer_steps, 1)
        self.assertEqual(trainer.snapshot.completed_attempts, 1)
        self.assertEqual(trainer.snapshot.trained_decisions, 1)
        self.assertEqual(trainer.snapshot.last_loss, 1.0)
        self.assertEqual(
            trainer.snapshot.last_behavior_manifest_ids,
            ((manifest_id,),),
        )
        self.assertEqual(
            trainer.snapshot.last_selection_probabilities,
            ((SelectionProbability.known(0.3),),),
        )
        self.assertGreater(trainer.snapshot.total_training_seconds, 0.0)
        self.assertGreater(trainer.snapshot.last_training_seconds, 0.0)

        restored_parameter = torch.nn.Parameter(torch.tensor([0.2, 5.0]))

        def restored_scorer(payload):
            return RaggedCandidateLogits(
                values=restored_parameter,
                row_splits=torch.tensor([0, 2]),
            )

        restored = SynchronousValueTrainer(
            restored_scorer,
            torch.optim.SGD([restored_parameter], lr=0.1),
            registry,
            CONCAT_LIMITS,
            resume_snapshot=trainer.snapshot,
        )
        self.assertEqual(restored.snapshot, trainer.snapshot)

    def test_unknown_manifest_fails_before_optimizer_and_can_be_retried(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)
        manifest = behavior_manifest_fixture()
        parameter = torch.nn.Parameter(torch.tensor([0.0, 5.0]))

        def scorer(payload):
            return RaggedCandidateLogits(
                values=parameter,
                row_splits=torch.tensor([0, 2]),
            )

        trainer = SynchronousValueTrainer(
            scorer,
            torch.optim.SGD([parameter], lr=0.1),
            registry,
            CONCAT_LIMITS,
        )
        batch = decision_batch_fixture(
            slot=1,
            semantic_row=0,
            selected_ordinal=0,
            manifest_id=manifest.identity,
        )
        delivery = AttemptAssemblyDelivery(
            completed=(
                completed_attempt_fixture(slot=1, batches=(batch,), reward=1),
            ),
            dropped=(),
        )

        with self.assertRaisesRegex(ValueError, "unknown behavior"):
            trainer(delivery)
        self.assertEqual(parameter[0].item(), 0.0)
        self.assertFalse(trainer.snapshot.poisoned)
        self.assertEqual(trainer.snapshot.deliveries, 0)

        registry.register(manifest)
        trainer(delivery)
        self.assertEqual(trainer.snapshot.optimizer_steps, 1)

    def test_optimizer_failure_poison_stops_retrying_mutable_state(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)
        manifest_id = registry.register(behavior_manifest_fixture())
        parameter = torch.nn.Parameter(torch.tensor([0.0, 5.0]))

        def scorer(payload):
            return RaggedCandidateLogits(
                values=parameter,
                row_splits=torch.tensor([0, 2]),
            )

        class FailingSgd(torch.optim.SGD):
            def step(self, closure=None):
                raise RuntimeError("optimizer failed")

        trainer = SynchronousValueTrainer(
            scorer,
            FailingSgd([parameter], lr=0.1),
            registry,
            CONCAT_LIMITS,
        )
        batch = decision_batch_fixture(
            slot=1,
            semantic_row=0,
            selected_ordinal=0,
            manifest_id=manifest_id,
        )
        delivery = AttemptAssemblyDelivery(
            completed=(
                completed_attempt_fixture(slot=1, batches=(batch,), reward=1),
            ),
            dropped=(),
        )

        with self.assertRaisesRegex(RuntimeError, "optimizer failed"):
            trainer(delivery)
        self.assertTrue(trainer.snapshot.poisoned)
        with self.assertRaisesRegex(TorchTrainingError, "poisoned"):
            trainer(delivery)


if __name__ == "__main__":
    unittest.main()
