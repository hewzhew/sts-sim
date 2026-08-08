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
from sts_learning import AttemptUpdateBatchLimits


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
    from sts_learning.torch_generation import TorchGenerationError


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class CategoricalOnlineSessionTests(unittest.TestCase):
    def test_compact_session_trains_publishes_restores_and_continues(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            factory = _factory(Path(root))
            initial = factory.new(model_seed=43, behavior_seed=94)
            initial_resume = initial.publish()
            evaluation_left = factory.recover_behavior(
                initial.active_behavior_manifest_id,
                behavior_seed=501,
            )
            evaluation_right = factory.recover_behavior(
                initial.active_behavior_manifest_id,
                behavior_seed=501,
            )
            decision = initial.runner.driver.env.decision_batch(semantic=True)
            self.assertEqual(
                evaluation_left.choose(decision),
                evaluation_right.choose(decision),
            )

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

    def test_partial_attempt_update_batch_stays_live_only_until_full(self) -> None:
        with tempfile.TemporaryDirectory() as root:
            session = _factory(Path(root), attempts_per_update=2).new(
                model_seed=43,
                behavior_seed=94,
            )

            partial = session.advance_generation(max_batch_steps=1)

            self.assertFalse(partial.generation.promoted)
            self.assertEqual(session.runner.trainer.snapshot.optimizer_steps, 0)
            self.assertEqual(
                session.runner.update_batcher.snapshot.pending_attempts,
                1,
            )
            with self.assertRaisesRegex(TorchGenerationError, "pending"):
                session.publish()

            completed = session.advance_generation(max_batch_steps=1)
            self.assertTrue(completed.generation.promoted)
            self.assertEqual(session.runner.trainer.snapshot.optimizer_steps, 1)
            self.assertEqual(
                session.runner.update_batcher.snapshot.pending_attempts,
                0,
            )


def _factory(root: Path, *, attempts_per_update: int = 1):
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
            limits=CategoricalSessionLimits(
                owner_capacity=4,
                attempt_updates=AttemptUpdateBatchLimits(
                    attempts_per_update=attempts_per_update,
                    max_decisions_per_update=64,
                    max_payload_bytes_per_update=1024 * 1024,
                ),
            ),
        ),
        NoRecoveryCurriculum(),
    )


if __name__ == "__main__":
    unittest.main()
