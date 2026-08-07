from __future__ import annotations

import importlib.util
import unittest

from learning.tests.semantic_fixtures import (
    semantic_batch_fixture,
    semantic_schema_fixture,
)
from sts_learning import select_semantic_decision_rows


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


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class TorchPolicyTests(unittest.TestCase):
    def test_schema_dimensions_come_only_from_bridge_schema(self) -> None:
        dimensions = SemanticSchemaDimensions.from_bridge_schema(
            semantic_schema_fixture()
        )

        self.assertEqual(dimensions.token_kind_size, 3)
        self.assertEqual(dimensions.categorical_field_size, 2)
        self.assertEqual(dimensions.categorical_offsets, (0, 3))
        self.assertEqual(dimensions.categorical_vocabulary_size, 5)

    def test_ragged_logits_loss_and_parameter_update(self) -> None:
        assert _TORCH_AVAILABLE
        torch.manual_seed(7)
        scorer = RaggedCandidateScorer.from_bridge_schema(
            semantic_schema_fixture(),
            RaggedScorerConfig(hidden_dim=24, relation_layers=1),
        )
        optimizer = torch.optim.SGD(scorer.parameters(), lr=0.05)

        logits = scorer(semantic_batch_fixture())
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
        scorer = RaggedCandidateScorer.from_bridge_schema(semantic_schema_fixture())
        batch = semantic_batch_fixture()
        relation = batch["semantic"]["relation"]  # type: ignore[index]
        relation["target_token_indices"][0] = 5  # type: ignore[index]

        with self.assertRaisesRegex(TorchPolicyError, "relation escapes"):
            scorer(batch)

    def test_row_selection_preserves_logits_without_cross_row_leakage(self) -> None:
        torch.manual_seed(9)
        scorer = RaggedCandidateScorer.from_bridge_schema(
            semantic_schema_fixture(),
            RaggedScorerConfig(hidden_dim=20, relation_layers=2),
        )
        batch = semantic_batch_fixture()

        original = scorer(batch)
        selected = scorer(select_semantic_decision_rows(batch, [1, 0]))
        expected = torch.cat((original.values[2:5], original.values[0:2]))

        self.assertEqual(selected.row_splits.tolist(), [0, 3, 5])
        torch.testing.assert_close(selected.values, expected)


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

        selected_batch = select_semantic_decision_rows(batch, [2, 0])
        selected_logits = scorer(selected_batch)
        original_splits = logits.row_splits.tolist()
        expected = torch.cat(
            tuple(
                logits.values[original_splits[row] : original_splits[row + 1]]
                for row in (2, 0)
            )
        )

        self.assertEqual(logits.row_splits.tolist(), batch["candidate_row_splits"].tolist())
        torch.testing.assert_close(selected_logits.values, expected)
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
