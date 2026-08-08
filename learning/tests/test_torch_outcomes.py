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

    from learning.tests.semantic_fixtures import semantic_schema_fixture
    from sts_learning.torch_outcomes import (
        TorchOutcomeError,
        _selected_log_probabilities,
        on_policy_terminal_loss,
    )
    from sts_learning.torch_policy import (
        RaggedCandidateLogits,
        RaggedCandidateScorer,
        RaggedCategoricalPolicyConfig,
        RaggedScorerConfig,
    )


CONCAT_LIMITS = SemanticBatchConcatLimits(
    max_rows=16,
    max_input_array_bytes=1024 * 1024,
)


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class OnPolicyTerminalLossTests(unittest.TestCase):
    def setUp(self) -> None:
        self.config = RaggedCategoricalPolicyConfig(temperature=0.8)
        self.registry = BehaviorManifestRegistry(capacity=1)
        self.manifest_id = self.registry.register(
            behavior_manifest_fixture(behavior_rule=self.config.behavior_rule)
        )

    def test_single_forward_loss_and_gradients_equal_per_attempt_reference(self) -> None:
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
        short = completed_attempt_fixture(
            slot=1,
            batches=(
                self._on_policy_batch(reference, slot=1, row=0, ordinal=1),
            ),
            reward=1,
        )
        long = completed_attempt_fixture(
            slot=2,
            batches=(
                self._on_policy_batch(reference, slot=2, row=1, ordinal=2),
                self._on_policy_batch(reference, slot=2, row=0, ordinal=0),
            ),
            reward=-1,
        )

        reference_attempt_losses = []
        for attempt in (short, long):
            terms = []
            for batch in attempt.batches:
                logits = reference(batch.payload)
                selected = batch.selected_ordinals[0]
                log_probability = torch.log_softmax(
                    logits.values / self.config.temperature,
                    dim=0,
                )[selected]
                terms.append(-attempt.terminal.terminal_reward * log_probability)
            reference_attempt_losses.append(torch.stack(terms).mean())
        reference_loss = torch.stack(reference_attempt_losses).mean()
        reference_loss.backward()

        calls = 0

        def counted_scorer(payload):
            nonlocal calls
            calls += 1
            return vectorized(payload)

        result = on_policy_terminal_loss(
            counted_scorer,
            (short, long),
            self.registry,
            CONCAT_LIMITS,
            self.config,
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

    def test_victory_and_defeat_move_selected_relative_probability_oppositely(self) -> None:
        values = torch.nn.Parameter(torch.zeros(8))

        def scorer(payload):
            return RaggedCandidateLogits(
                values=values,
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            )

        short = completed_attempt_fixture(
            slot=1,
            batches=(self._fixed_probability_batch(slot=1),),
            reward=1,
        )
        long = completed_attempt_fixture(
            slot=2,
            batches=tuple(self._fixed_probability_batch(slot=2) for _ in range(3)),
            reward=-1,
        )

        result = on_policy_terminal_loss(
            scorer,
            (short, long),
            self.registry,
            CONCAT_LIMITS,
            self.config,
        )
        result.value.backward()

        self.assertAlmostEqual(float(result.value.detach()), 0.0, places=6)
        self.assertEqual(result.attempt_count, 2)
        self.assertEqual(result.decision_count, 4)
        self.assertAlmostEqual(values.grad[0].item(), -0.3125, places=6)
        self.assertAlmostEqual(values.grad[1].item(), 0.3125, places=6)
        for start in (2, 4, 6):
            self.assertAlmostEqual(values.grad[start].item(), 5.0 / 48.0, places=6)
            self.assertAlmostEqual(values.grad[start + 1].item(), -5.0 / 48.0, places=6)

    def test_forced_candidate_has_exactly_zero_policy_gradient(self) -> None:
        values = torch.nn.Parameter(torch.tensor([2.0, 3.0, 4.0]))
        selected = _selected_log_probabilities(
            RaggedCandidateLogits(
                values=values,
                row_splits=torch.tensor([0, 1, 3], dtype=torch.long),
            ),
            (0, 1),
            self.config,
        )

        selected.sum().backward()

        self.assertEqual(values.grad[0].item(), 0.0)
        self.assertNotEqual(values.grad[1].item(), 0.0)
        self.assertNotEqual(values.grad[2].item(), 0.0)

    def test_unknown_or_mismatched_propensity_is_rejected(self) -> None:
        values = torch.nn.Parameter(torch.zeros(2))

        def scorer(payload):
            return RaggedCandidateLogits(
                values=values,
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            )

        base = decision_batch_fixture(
            slot=1,
            semantic_row=0,
            selected_ordinal=0,
            manifest_id=self.manifest_id,
        )
        for evidence in (
            SelectionProbability.unknown(),
            SelectionProbability.known(0.2),
        ):
            attempt = completed_attempt_fixture(
                slot=1,
                batches=(replace(base, selection_probabilities=(evidence,)),),
                reward=1,
            )
            with self.assertRaisesRegex(
                TorchOutcomeError,
                "known selection|off-policy",
            ):
                on_policy_terminal_loss(
                    scorer,
                    (attempt,),
                    self.registry,
                    CONCAT_LIMITS,
                    self.config,
                )

    def test_unknown_manifest_and_non_complete_input_fail_before_training(self) -> None:
        unregistered = behavior_manifest_fixture(
            behavior_rule=self.config.behavior_rule
        ).identity
        batch = decision_batch_fixture(
            slot=1,
            semantic_row=0,
            selected_ordinal=0,
            manifest_id=unregistered,
            selection_probability=SelectionProbability.known(0.5),
        )

        with self.assertRaisesRegex(TorchOutcomeError, "unknown behavior"):
            on_policy_terminal_loss(
                lambda payload: RaggedCandidateLogits(
                    values=torch.zeros(2),
                    row_splits=torch.tensor([0, 2]),
                ),
                (completed_attempt_fixture(slot=1, batches=(batch,), reward=1),),
                BehaviorManifestRegistry(capacity=1),
                CONCAT_LIMITS,
                self.config,
            )
        with self.assertRaisesRegex(TorchOutcomeError, "at least one"):
            on_policy_terminal_loss(
                lambda payload: None,
                (),
                self.registry,
                CONCAT_LIMITS,
                self.config,
            )
        with self.assertRaisesRegex(TorchOutcomeError, "only complete"):
            on_policy_terminal_loss(
                lambda payload: None,
                (object(),),  # type: ignore[arg-type]
                self.registry,
                CONCAT_LIMITS,
                self.config,
            )

    def _on_policy_batch(self, scorer, *, slot: int, row: int, ordinal: int):
        batch = decision_batch_fixture(
            slot=slot,
            semantic_row=row,
            selected_ordinal=ordinal,
            manifest_id=self.manifest_id,
        )
        logits = scorer(batch.payload)
        probability = float(
            torch.softmax(
                logits.values.detach().to(dtype=torch.float64)
                / self.config.temperature,
                dim=0,
            )[ordinal].item()
        )
        return replace(
            batch,
            selection_probabilities=(SelectionProbability.known(probability),),
        )

    def _fixed_probability_batch(self, *, slot: int):
        return decision_batch_fixture(
            slot=slot,
            semantic_row=0,
            selected_ordinal=0,
            manifest_id=self.manifest_id,
            selection_probability=SelectionProbability.known(0.5),
        )


if __name__ == "__main__":
    unittest.main()
