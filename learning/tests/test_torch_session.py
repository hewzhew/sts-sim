from __future__ import annotations

import importlib.util
import tempfile
import unittest
from dataclasses import replace
from pathlib import Path

from learning.tests.driver_fixtures import (
    FakeCheckpointBatch,
    NumpyWinningBatchEnv,
)
from learning.tests.semantic_fixtures import semantic_schema_fixture


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None

if _TORCH_AVAILABLE:
    from sts_learning.torch_policy import (
        RaggedCategoricalPolicyConfig,
        RaggedScorerConfig,
    )
    from sts_learning.torch_session import (
        CategoricalOnlineSessionFactory,
        NoRecoveryCurriculum,
        TorchSessionError,
    )
    from sts_learning.torch_session_config import (
        CategoricalOnlineProfile,
        CategoricalOnlineSessionConfig,
        CategoricalSessionBridge,
        CategoricalSessionLimits,
    )


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class CategoricalOnlineSessionTests(unittest.TestCase):
    def test_compact_session_trains_publishes_restores_and_continues(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            factory = _factory(Path(root))
            initial = factory.new(model_seed=43, behavior_seed=94)
            initial_resume = initial.publish()

            generation_zero = factory.restore(initial_resume.manifest_id)
            generation_one = generation_zero.advance_generation(max_batch_steps=1)
            self.assertTrue(generation_one.generation.promoted)
            self.assertIsNotNone(generation_one.resume)
            assert generation_one.resume is not None

            restored = factory.restore(generation_one.resume.manifest_id)
            generation_two = restored.advance_generation(max_batch_steps=1)

            self.assertTrue(generation_two.generation.promoted)
            self.assertIsNotNone(generation_two.resume)
            self.assertEqual(
                restored.runner.controller.snapshot.active_training_step,
                2,
            )
            self.assertEqual(restored.runner.trainer.snapshot.optimizer_steps, 2)
            with self.assertRaisesRegex(TorchSessionError, "unused experiment root"):
                factory.new(model_seed=43, behavior_seed=94)
            mismatched = CategoricalOnlineSessionFactory(
                Path(root),
                factory.bridge,
                replace(factory.config, slot_count=2),
                factory.curriculum,
            )
            with self.assertRaisesRegex(TorchSessionError, "slot_count"):
                mismatched.restore(initial_resume.manifest_id)


def _factory(root: Path):
    return CategoricalOnlineSessionFactory(
        root,
        CategoricalSessionBridge(
            environment=NumpyWinningBatchEnv,
            environment_from_checkpoint=(
                NumpyWinningBatchEnv.from_checkpoint_bytes
            ),
            checkpoint_bank_from_checkpoint=(
                FakeCheckpointBatch.from_checkpoint_bytes
            ),
            semantic_schema=semantic_schema_fixture(),
        ),
        CategoricalOnlineSessionConfig(
            profile=CategoricalOnlineProfile(
                scorer=RaggedScorerConfig(hidden_dim=4, relation_layers=0),
                behavior=RaggedCategoricalPolicyConfig(temperature=0.8),
                optimizer_steps_per_generation=1,
            ),
            limits=CategoricalSessionLimits(owner_capacity=4),
        ),
        NoRecoveryCurriculum(),
    )


if __name__ == "__main__":
    unittest.main()
