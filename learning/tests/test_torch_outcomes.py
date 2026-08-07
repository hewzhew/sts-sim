from __future__ import annotations

from dataclasses import replace
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

    from sts_learning.torch_outcomes import (
        TorchOutcomeError,
        realized_outcome_value_loss,
    )
    from sts_learning.torch_policy import RaggedCandidateLogits
    from sts_learning.torch_policy import RaggedCandidateScorer, RaggedScorerConfig
    from learning.tests.semantic_fixtures import semantic_schema_fixture


CONCAT_LIMITS = SemanticBatchConcatLimits(
    max_rows=16,
    max_input_array_bytes=1024 * 1024,
)

@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class RealizedOutcomeValueLossTests(unittest.TestCase):
    def test_single_forward_loss_and_gradients_equal_per_batch_reference(self) -> None:
        torch.manual_seed(42)
        reference = RaggedCandidateScorer.from_bridge_schema(
            semantic_schema_fixture(),
            RaggedScorerConfig(hidden_dim=10, relation_layers=1),
        )
        vectorized = RaggedCandidateScorer.from_bridge_schema(
            semantic_schema_fixture(),
            RaggedScorerConfig(hidden_dim=10, relation_layers=1),
        )
        vectorized.load_state_dict(reference.state_dict())
        registry = BehaviorManifestRegistry(capacity=1)
        manifest_id = registry.register(behavior_manifest_fixture())
        short = completed_attempt_fixture(
            slot=1,
            batches=(
                decision_batch_fixture(
                    slot=1,
                    semantic_row=0,
                    selected_ordinal=1,
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
                    semantic_row=1,
                    selected_ordinal=2,
                    manifest_id=manifest_id,
                ),
                decision_batch_fixture(
                    slot=2,
                    semantic_row=0,
                    selected_ordinal=0,
                    manifest_id=manifest_id,
                ),
            ),
            reward=-1,
        )

        reference_attempt_losses = []
        for attempt in (short, long):
            errors = []
            for batch in attempt.batches:
                logits = reference(batch.payload)
                ordinal = torch.tensor(batch.selected_ordinals, dtype=torch.long)
                selected = logits.values[logits.row_splits[:-1] + ordinal]
                errors.append(
                    (selected - float(attempt.terminal.terminal_reward)).square()
                )
            reference_attempt_losses.append(torch.cat(errors).mean())
        reference_loss = torch.stack(reference_attempt_losses).mean()
        reference_loss.backward()

        calls = 0

        def counted_scorer(payload):
            nonlocal calls
            calls += 1
            return vectorized(payload)

        result = realized_outcome_value_loss(
            counted_scorer,
            (short, long),
            registry,
            CONCAT_LIMITS,
        )
        result.value.backward()

        self.assertEqual(calls, 1)
        torch.testing.assert_close(result.value, reference_loss)
        for actual, expected in zip(
            vectorized.parameters(),
            reference.parameters(),
            strict=True,
        ):
            self.assertIsNotNone(actual.grad)
            self.assertIsNotNone(expected.grad)
            torch.testing.assert_close(actual.grad, expected.grad)

    def test_only_selected_candidates_are_targeted_and_attempts_are_equal_weight(self) -> None:
        manifest = behavior_manifest_fixture()
        registry = BehaviorManifestRegistry(capacity=1)
        manifest_id = registry.register(manifest)
        values = torch.nn.Parameter(
            torch.tensor([-1.0, 90.0, 0.0, 80.0, 0.0, 70.0, 0.0, 60.0])
        )
        scorer_calls = 0

        def scorer(payload):
            nonlocal scorer_calls
            scorer_calls += 1
            return RaggedCandidateLogits(
                values=values,
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            )

        short = completed_attempt_fixture(
            slot=1,
            batches=(
                decision_batch_fixture(
                    slot=1,
                    semantic_row=0,
                    selected_ordinal=0,
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
                    semantic_row=0,
                    selected_ordinal=0,
                    manifest_id=manifest_id,
                ),
                decision_batch_fixture(
                    slot=2,
                    semantic_row=0,
                    selected_ordinal=0,
                    manifest_id=manifest_id,
                ),
                decision_batch_fixture(
                    slot=2,
                    semantic_row=0,
                    selected_ordinal=0,
                    manifest_id=manifest_id,
                ),
            ),
            reward=-1,
        )

        result = realized_outcome_value_loss(
            scorer,
            (short, long),
            registry,
            CONCAT_LIMITS,
        )
        result.value.backward()

        self.assertAlmostEqual(float(result.value.detach()), 2.5, places=6)
        self.assertEqual(result.attempt_count, 2)
        self.assertEqual(result.decision_count, 4)
        self.assertEqual(scorer_calls, 1)
        self.assertEqual(
            result.behavior_manifest_ids,
            ((manifest_id,), (manifest_id, manifest_id, manifest_id)),
        )
        self.assertEqual(values.grad[0].item(), -2.0)
        for index in (1, 3, 5, 7):
            self.assertEqual(values.grad[index].item(), 0.0)
        for index in (2, 4, 6):
            self.assertAlmostEqual(values.grad[index].item(), 1.0 / 3.0, places=6)

    def test_probability_evidence_is_preserved_without_weighting_current_loss(
        self,
    ) -> None:
        registry = BehaviorManifestRegistry(capacity=1)
        manifest_id = registry.register(behavior_manifest_fixture())
        deterministic_batches = (
            decision_batch_fixture(
                slot=1,
                semantic_row=0,
                selected_ordinal=0,
                manifest_id=manifest_id,
            ),
            decision_batch_fixture(
                slot=1,
                semantic_row=0,
                selected_ordinal=1,
                manifest_id=manifest_id,
            ),
        )
        evidence_batches = (
            replace(
                deterministic_batches[0],
                selection_probabilities=(SelectionProbability.known(0.2),),
            ),
            replace(
                deterministic_batches[1],
                selection_probabilities=(SelectionProbability.unknown(),),
            ),
        )
        deterministic = completed_attempt_fixture(
            slot=1,
            batches=deterministic_batches,
            reward=1,
        )
        evidence = completed_attempt_fixture(
            slot=1,
            batches=evidence_batches,
            reward=1,
        )
        values = torch.nn.Parameter(torch.tensor([0.0, 1.0, 2.0, 3.0]))

        def scorer(payload):
            return RaggedCandidateLogits(
                values=values,
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            )

        baseline = realized_outcome_value_loss(
            scorer,
            (deterministic,),
            registry,
            CONCAT_LIMITS,
        )
        baseline.value.backward()
        baseline_gradient = values.grad.detach().clone()
        values.grad = None

        observed = realized_outcome_value_loss(
            scorer,
            (evidence,),
            registry,
            CONCAT_LIMITS,
        )
        observed.value.backward()

        torch.testing.assert_close(observed.value, baseline.value)
        torch.testing.assert_close(values.grad, baseline_gradient)
        self.assertEqual(
            tuple(
                probability.value
                for probability in observed.selection_probabilities[0]
            ),
            (0.2, None),
        )

    def test_unknown_behavior_manifest_fails_before_training(self) -> None:
        manifest = behavior_manifest_fixture()
        unregistered_id = manifest.identity
        registry = BehaviorManifestRegistry(capacity=1)
        batch = decision_batch_fixture(
            slot=1,
            semantic_row=0,
            selected_ordinal=0,
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
                CONCAT_LIMITS,
            )

    def test_empty_or_non_complete_input_cannot_create_a_loss(self) -> None:
        registry = BehaviorManifestRegistry(capacity=1)

        with self.assertRaisesRegex(TorchOutcomeError, "at least one"):
            realized_outcome_value_loss(
                lambda payload: None,
                (),
                registry,
                CONCAT_LIMITS,
            )
        with self.assertRaisesRegex(TorchOutcomeError, "only complete"):
            realized_outcome_value_loss(
                lambda payload: None,
                (object(),),  # type: ignore[arg-type]
                registry,
                CONCAT_LIMITS,
            )


if __name__ == "__main__":
    unittest.main()
