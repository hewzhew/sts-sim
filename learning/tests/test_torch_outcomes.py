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
)


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None
if _TORCH_AVAILABLE:
    import torch

    from sts_learning.torch_outcomes import (
        TorchOutcomeError,
        realized_outcome_value_loss,
    )
    from sts_learning.torch_policy import RaggedCandidateLogits

@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class RealizedOutcomeValueLossTests(unittest.TestCase):
    def test_only_selected_candidates_are_targeted_and_attempts_are_equal_weight(self) -> None:
        manifest = behavior_manifest_fixture()
        registry = BehaviorManifestRegistry(capacity=1)
        manifest_id = registry.register(manifest)
        values = torch.nn.Parameter(
            torch.tensor([-1.0, 90.0, 0.0, 80.0, 0.0, 70.0, 0.0, 60.0])
        )

        def scorer(payload):
            indices = torch.as_tensor(payload["value_indices"], dtype=torch.long)
            return RaggedCandidateLogits(
                values=values[indices],
                row_splits=torch.arange(0, len(indices) + 1, 2),
            )

        short = completed_attempt_fixture(
            slot=1,
            batches=(
                decision_batch_fixture(
                    slot=1,
                    value_indices=(0, 1),
                    selected_ordinals=(0,),
                    manifest_id=manifest_id,
                ),
            ),
            reward=1,
        )
        long = completed_attempt_fixture(
            slot=2,
            batches=(
                decision_batch_fixture(
                    slot=2,
                    value_indices=(2, 3),
                    selected_ordinals=(0,),
                    manifest_id=manifest_id,
                ),
                decision_batch_fixture(
                    slot=2,
                    value_indices=(4, 5),
                    selected_ordinals=(0,),
                    manifest_id=manifest_id,
                ),
                decision_batch_fixture(
                    slot=2,
                    value_indices=(6, 7),
                    selected_ordinals=(0,),
                    manifest_id=manifest_id,
                ),
            ),
            reward=-1,
        )

        result = realized_outcome_value_loss(scorer, (short, long), registry)
        result.value.backward()

        self.assertEqual(float(result.value.detach()), 2.5)
        self.assertEqual(result.attempt_count, 2)
        self.assertEqual(result.decision_count, 4)
        self.assertEqual(
            result.behavior_manifest_ids,
            ((manifest_id,), (manifest_id, manifest_id, manifest_id)),
        )
        self.assertEqual(values.grad[0].item(), -2.0)
        for index in (1, 3, 5, 7):
            self.assertEqual(values.grad[index].item(), 0.0)
        for index in (2, 4, 6):
            self.assertAlmostEqual(values.grad[index].item(), 1.0 / 3.0, places=6)

    def test_unknown_behavior_manifest_fails_before_training(self) -> None:
        manifest = behavior_manifest_fixture()
        unregistered_id = manifest.identity
        registry = BehaviorManifestRegistry(capacity=1)
        batch = decision_batch_fixture(
            slot=1,
            value_indices=(0, 1),
            selected_ordinals=(0,),
            manifest_id=unregistered_id,
        )

        with self.assertRaisesRegex(TorchOutcomeError, "unknown behavior"):
            realized_outcome_value_loss(
                lambda payload: RaggedCandidateLogits(
                    values=torch.zeros(2),
                    row_splits=torch.tensor([0, 2]),
                ),
                (completed_attempt_fixture(slot=1, batches=(batch,), reward=1),),
                registry,
            )

    def test_empty_or_non_complete_input_cannot_create_a_loss(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)

        with self.assertRaisesRegex(TorchOutcomeError, "at least one"):
            realized_outcome_value_loss(lambda payload: None, (), registry)
        with self.assertRaisesRegex(TorchOutcomeError, "only complete"):
            realized_outcome_value_loss(
                lambda payload: None,
                (object(),),  # type: ignore[arg-type]
                registry,
            )


if __name__ == "__main__":
    unittest.main()
