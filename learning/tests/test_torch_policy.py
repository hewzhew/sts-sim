from __future__ import annotations

import importlib.util
import unittest

import numpy as np


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None
if _TORCH_AVAILABLE:
    import torch

    from sts_learning.torch_policy import (
        GreedyTorchPolicy,
        RaggedCandidateScorer,
        RaggedScorerConfig,
        SemanticSchemaDimensions,
        TorchPolicyError,
        ragged_cross_entropy,
    )

try:
    from sts_learning_bridge import LearningBatchEnv, semantic_schema
except ImportError:
    LearningBatchEnv = None  # type: ignore[assignment,misc]
    semantic_schema = None  # type: ignore[assignment]


def _schema() -> dict[str, object]:
    return {
        "version": 2,
        "token_kind": {"Observation": 0, "Candidate": 1, "Entity": 2},
        "categorical_field": {"Kind": 0, "Flag": 1},
        "scalar_field": {"Amount": 0},
        "relation_kind": {"HasCandidate": 0, "Targets": 1},
        "categorical_vocabulary_size": {0: 3, 1: 2},
    }


def _batch() -> dict[str, object]:
    return {
        "slot_indices": np.array([4, 9], dtype=np.uint64),
        "candidate_counts": np.array([2, 3], dtype=np.uint64),
        "candidate_row_splits": np.array([0, 2, 5], dtype=np.uint64),
        "semantic": {
            "schema_version": 2,
            "token": {
                "row_splits": np.array([0, 4, 9], dtype=np.uint64),
                "kind": np.array([0, 1, 1, 2, 0, 1, 1, 1, 2], dtype=np.uint16),
            },
            "categorical": {
                "token_indices": np.array([0, 1, 2, 4, 5, 6, 7], dtype=np.uint64),
                "field": np.array([0, 1, 1, 0, 1, 1, 1], dtype=np.uint16),
                "value": np.array([2, 0, 1, 1, 1, 0, 1], dtype=np.int64),
            },
            "scalar": {
                "token_indices": np.array([0, 3, 4, 8], dtype=np.uint64),
                "field": np.array([0, 0, 0, 0], dtype=np.uint16),
                "value": np.array([0.5, -1.0, 2.0, 0.25], dtype=np.float32),
            },
            "relation": {
                "source_token_indices": np.array(
                    [0, 0, 1, 4, 4, 4, 5], dtype=np.uint64
                ),
                "relation": np.array([0, 0, 1, 0, 0, 0, 1], dtype=np.uint16),
                "target_token_indices": np.array(
                    [1, 2, 3, 5, 6, 7, 8], dtype=np.uint64
                ),
            },
            "candidate_token_indices": np.array([1, 2, 5, 6, 7], dtype=np.uint64),
        },
    }


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class TorchPolicyTests(unittest.TestCase):
    def test_schema_dimensions_come_only_from_bridge_schema(self) -> None:
        dimensions = SemanticSchemaDimensions.from_bridge_schema(_schema())

        self.assertEqual(dimensions.token_kind_size, 3)
        self.assertEqual(dimensions.categorical_field_size, 2)
        self.assertEqual(dimensions.categorical_offsets, (0, 3))
        self.assertEqual(dimensions.categorical_vocabulary_size, 5)

    def test_ragged_logits_loss_and_parameter_update(self) -> None:
        assert _TORCH_AVAILABLE
        torch.manual_seed(7)
        scorer = RaggedCandidateScorer.from_bridge_schema(
            _schema(),
            RaggedScorerConfig(hidden_dim=24, relation_layers=1),
        )
        optimizer = torch.optim.SGD(scorer.parameters(), lr=0.05)

        logits = scorer(_batch())
        self.assertEqual(tuple(logits.values.shape), (5,))
        self.assertEqual(logits.row_splits.tolist(), [0, 2, 5])
        self.assertTrue(bool(torch.all(torch.isfinite(logits.values))))
        self.assertEqual(len(logits.greedy_ordinals()), 2)

        loss = ragged_cross_entropy(logits, [1, 2])
        self.assertTrue(bool(torch.isfinite(loss)))
        before = scorer.scorer[-1].weight.detach().clone()
        optimizer.zero_grad()
        loss.backward()
        self.assertTrue(
            any(
                parameter.grad is not None
                and bool(torch.all(torch.isfinite(parameter.grad)))
                for parameter in scorer.parameters()
            )
        )
        optimizer.step()
        self.assertFalse(torch.equal(before, scorer.scorer[-1].weight.detach()))

    def test_cross_row_relation_is_rejected(self) -> None:
        scorer = RaggedCandidateScorer.from_bridge_schema(_schema())
        batch = _batch()
        relation = batch["semantic"]["relation"]  # type: ignore[index]
        relation["target_token_indices"][0] = 5  # type: ignore[index]

        with self.assertRaisesRegex(TorchPolicyError, "relation escapes"):
            scorer(batch)


@unittest.skipUnless(
    _TORCH_AVAILABLE and LearningBatchEnv is not None,
    "optional PyTorch dependency or standalone bridge wheel is not installed",
)
class RealBridgeTorchPolicyTests(unittest.TestCase):
    def test_real_semantic_batch_trains_without_padding(self) -> None:
        assert LearningBatchEnv is not None
        assert semantic_schema is not None
        env = LearningBatchEnv([11, 12, 13])
        batch = env.decision_batch(semantic=True)
        scorer = RaggedCandidateScorer.from_bridge_schema(
            semantic_schema(),
            RaggedScorerConfig(hidden_dim=16, relation_layers=1),
        )

        logits = scorer(batch)
        targets = torch.zeros(len(batch["slot_indices"]), dtype=torch.long)
        loss = ragged_cross_entropy(logits, targets)
        loss.backward()

        self.assertEqual(logits.row_splits.tolist(), batch["candidate_row_splits"].tolist())
        self.assertTrue(bool(torch.isfinite(loss)))
        choices = GreedyTorchPolicy(scorer).choose(batch)
        self.assertEqual(len(choices), len(batch["slot_indices"]))
        self.assertTrue(
            all(
                0 <= choice < int(count)
                for choice, count in zip(choices, batch["candidate_counts"], strict=True)
            )
        )
        env.choose(list(choices))
        self.assertTrue(env.ready)


if __name__ == "__main__":
    unittest.main()
