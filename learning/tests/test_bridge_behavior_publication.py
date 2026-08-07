from __future__ import annotations

import importlib.util
import tempfile
import unittest
from pathlib import Path

from learning.tests.torch_outcome_fixtures import (
    behavior_manifest_template_fixture,
)
from sts_learning import (
    BehaviorManifestCatalogLimits,
    BehaviorManifestRegistry,
    BoundedBehaviorManifestCatalog,
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
        CheckpointedGreedyTorchPolicy,
        TorchBehaviorPublisher,
    )
    from sts_learning.torch_checkpoints import (
        BoundedTorchCheckpointStore,
        TorchCheckpointLimits,
    )
    from sts_learning.torch_policy import RaggedCandidateScorer, RaggedScorerConfig


@unittest.skipUnless(
    _TORCH_AVAILABLE and LearningBatchEnv is not None,
    "optional PyTorch dependency or standalone bridge wheel is not installed",
)
class RealBridgeBehaviorPublicationTests(unittest.TestCase):
    def test_reopened_checkpoint_preserves_logits_and_switches_manifest(self) -> None:
        assert LearningBatchEnv is not None
        assert semantic_schema is not None
        schema = semantic_schema()
        config = RaggedScorerConfig(hidden_dim=8, relation_layers=0)

        def scorer_factory():
            return RaggedCandidateScorer.from_bridge_schema(schema, config)

        torch.manual_seed(31)
        shadow = scorer_factory()
        env = LearningBatchEnv([991])
        batch = env.decision_batch(semantic=True)
        expected = shadow(batch).values.detach().clone()
        registry = BehaviorManifestRegistry(capacity=1)
        limits = TorchCheckpointLimits(
            max_checkpoints=1,
            max_bytes_per_checkpoint=2 * 1024 * 1024,
            max_total_bytes=2 * 1024 * 1024,
        )

        with tempfile.TemporaryDirectory() as root:
            checkpoint_root = Path(root, "checkpoints")
            manifest_root = Path(root, "manifests")
            store = BoundedTorchCheckpointStore(checkpoint_root, limits)
            catalog_limits = BehaviorManifestCatalogLimits(
                max_manifests=1,
                max_bytes_per_manifest=1024,
                max_total_bytes=1024,
            )
            catalog = BoundedBehaviorManifestCatalog(
                manifest_root,
                catalog_limits,
            )
            publication = TorchBehaviorPublisher(
                store,
                catalog,
                registry,
                behavior_manifest_template_fixture(
                    semantic_schema_version=int(schema["version"]),
                ),
            ).publish(shadow, training_step=7)
            reopened_store = BoundedTorchCheckpointStore(checkpoint_root, limits)
            reopened_catalog = BoundedBehaviorManifestCatalog(
                manifest_root,
                catalog_limits,
            )
            recovered_registry = BehaviorManifestRegistry(capacity=1)
            policy = CheckpointedGreedyTorchPolicy.recover(
                publication.manifest_id,
                reopened_store,
                reopened_catalog,
                recovered_registry,
                scorer_factory,
            )

            torch.testing.assert_close(policy.score(batch).values, expected)
            choice = policy.choose(batch)
            self.assertEqual(choice.behavior_manifest_id, publication.manifest_id)
            self.assertEqual(
                recovered_registry.resolve(publication.manifest_id),
                publication.manifest,
            )
            env.choose(list(choice.ordinals))
            self.assertTrue(env.ready)

            with torch.no_grad():
                shadow.scorer[-1].bias.add_(100.0)
            torch.testing.assert_close(policy.score(batch).values, expected)


if __name__ == "__main__":
    unittest.main()
