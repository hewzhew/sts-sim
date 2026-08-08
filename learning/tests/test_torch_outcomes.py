from __future__ import annotations

from dataclasses import replace
import importlib.util
import math
import unittest

from learning.tests.torch_outcome_fixtures import (
    behavior_manifest_fixture,
    completed_attempt_fixture,
    decision_batch_fixture,
)
from sts_learning import (
    BehaviorManifestRegistry,
    DecisionRunProgress,
    FloorProgressReturnConfig,
    SelectionProbability,
    SemanticBatchConcatLimits,
    TerminalAdvantageMode,
    floor_progress_terminal_return,
    terminal_return_advantages,
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
        self.return_config = FloorProgressReturnConfig(target_floor=100)
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

        attempts = (short, long)
        advantages = terminal_return_advantages(
            tuple(
                floor_progress_terminal_return(attempt.terminal, self.return_config)
                for attempt in attempts
            ),
            TerminalAdvantageMode.LEAVE_ONE_OUT,
        )
        reference_attempt_losses = []
        for attempt, advantage in zip(attempts, advantages, strict=True):
            terms = []
            for batch in attempt.batches:
                logits = reference(batch.payload)
                selected = batch.selected_ordinals[0]
                log_probability = torch.log_softmax(
                    logits.values / self.config.temperature,
                    dim=0,
                )[selected]
                terms.append(-advantage * log_probability)
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
            self.return_config,
            TerminalAdvantageMode.LEAVE_ONE_OUT,
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

    def test_positive_and_negative_returns_move_probability_oppositely(self) -> None:
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
            self.return_config,
            TerminalAdvantageMode.RAW_RETURN,
        )
        result.value.backward()

        self.assertAlmostEqual(
            float(result.value.detach()),
            0.4 * math.log(2.0),
            places=6,
        )
        self.assertEqual(result.attempt_count, 2)
        self.assertEqual(result.decision_count, 4)
        self.assertAlmostEqual(values.grad[0].item(), -0.3125, places=6)
        self.assertAlmostEqual(values.grad[1].item(), 0.3125, places=6)
        for start in (2, 4, 6):
            self.assertAlmostEqual(values.grad[start].item(), 1.0 / 48.0, places=6)
            self.assertAlmostEqual(values.grad[start + 1].item(), -1.0 / 48.0, places=6)

    def test_matched_floor_objective_compares_only_attempts_at_that_floor(self) -> None:
        values = torch.nn.Parameter(torch.zeros(4))

        def scorer(payload):
            return RaggedCandidateLogits(
                values=values,
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            )

        attempts = []
        for slot, terminal_floor, ordinal in ((1, 10, 0), (2, 20, 1)):
            batch = replace(
                decision_batch_fixture(
                    slot=slot,
                    semantic_row=0,
                    selected_ordinal=ordinal,
                    manifest_id=self.manifest_id,
                    selection_probability=SelectionProbability.known(0.5),
                ),
                run_progress=(
                    DecisionRunProgress(
                        episode_seed=100 + slot,
                        act=1,
                        floor=0,
                    ),
                ),
            )
            attempt = completed_attempt_fixture(
                slot=slot,
                batches=(batch,),
                reward=-1,
            )
            attempts.append(
                replace(
                    attempt,
                    terminal=replace(
                        attempt.terminal,
                        terminal=replace(
                            attempt.terminal.terminal,
                            terminal_floor=terminal_floor,
                        ),
                    ),
                )
            )

        result = on_policy_terminal_loss(
            scorer,
            attempts,
            self.registry,
            CONCAT_LIMITS,
            self.config,
            self.return_config,
            TerminalAdvantageMode.MATCHED_FLOOR_LEAVE_ONE_OUT,
        )
        result.value.backward()

        self.assertAlmostEqual(float(result.value.detach()), 0.0, places=6)
        self.assertGreater(values.grad[0].item(), 0.0)
        self.assertLess(values.grad[3].item(), 0.0)

    def test_floor_progress_reserves_the_unique_maximum_for_victory(self) -> None:
        config = FloorProgressReturnConfig(target_floor=52)
        defeat = completed_attempt_fixture(
            slot=1,
            batches=(self._fixed_probability_batch(slot=1),),
            reward=-1,
        ).terminal

        for floor, expected in (
            (0, -1.0),
            (26, 0.0),
            (51, 1.0 - 2.0 / 52.0),
            (999, 1.0 - 2.0 / 52.0),
        ):
            with self.subTest(floor=floor):
                record = replace(
                    defeat,
                    terminal=replace(defeat.terminal, terminal_floor=floor),
                )
                self.assertAlmostEqual(
                    floor_progress_terminal_return(record, config),
                    expected,
                )

        victory = completed_attempt_fixture(
            slot=1,
            batches=(self._fixed_probability_batch(slot=1),),
            reward=1,
        ).terminal
        victory = replace(
            victory,
            terminal=replace(victory.terminal, terminal_floor=0),
        )
        self.assertEqual(floor_progress_terminal_return(victory, config), 1.0)

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
                    self.return_config,
                    TerminalAdvantageMode.RAW_RETURN,
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
                self.return_config,
                TerminalAdvantageMode.RAW_RETURN,
            )
        with self.assertRaisesRegex(TorchOutcomeError, "at least one"):
            on_policy_terminal_loss(
                lambda payload: None,
                (),
                self.registry,
                CONCAT_LIMITS,
                self.config,
                self.return_config,
                TerminalAdvantageMode.RAW_RETURN,
            )
        with self.assertRaisesRegex(TorchOutcomeError, "only complete"):
            on_policy_terminal_loss(
                lambda payload: None,
                (object(),),  # type: ignore[arg-type]
                self.registry,
                CONCAT_LIMITS,
                self.config,
                self.return_config,
                TerminalAdvantageMode.RAW_RETURN,
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
