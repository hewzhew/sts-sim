from __future__ import annotations

import importlib.util
import math
import unittest

from learning.tests.torch_combat_fixtures import combat_group_experience_fixture
from learning.tests.torch_outcome_fixtures import behavior_manifest_fixture
from sts_learning import (
    BehaviorManifestId,
    BehaviorManifestRegistry,
    CombatAllWinAxis,
    CombatWinObjectiveConfig,
    SelectionProbability,
    SemanticBatchConcatLimits,
)


_TORCH_AVAILABLE = importlib.util.find_spec("torch") is not None
if _TORCH_AVAILABLE:
    import torch

    from sts_learning.torch_outcomes import (
        TorchOutcomeError,
        on_policy_combat_win_loss,
    )
    from sts_learning.torch_policy import (
        RaggedCandidateLogits,
        RaggedCategoricalPolicyConfig,
    )


CONCAT_LIMITS = SemanticBatchConcatLimits(
    max_rows=16,
    max_input_array_bytes=1024 * 1024,
)


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class OnPolicyCombatWinLossTests(unittest.TestCase):
    def setUp(self) -> None:
        self.config = RaggedCategoricalPolicyConfig(temperature=1.0)
        self.objective = CombatWinObjectiveConfig()
        self.registry = BehaviorManifestRegistry(capacity=1)
        self.manifest_id = self.registry.register(
            behavior_manifest_fixture(behavior_rule=self.config.behavior_rule)
        )

    def test_one_forward_weights_replicates_equally_not_decisions(self) -> None:
        values = torch.nn.Parameter(torch.zeros(7))
        calls = 0

        def scorer(payload):
            nonlocal calls
            calls += 1
            return RaggedCandidateLogits(
                values=values,
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            )

        result = on_policy_combat_win_loss(
            scorer,
            (combat_group_experience_fixture(self.manifest_id, wins=(True, False)),),
            self.registry,
            CONCAT_LIMITS,
            self.config,
            self.objective,
        )
        result.value.backward()

        self.assertEqual(calls, 1)
        self.assertEqual(result.group_count, 1)
        self.assertEqual(result.signal_group_count, 1)
        self.assertEqual(result.win_signal_group_count, 1)
        self.assertEqual(result.terminal_hp_signal_group_count, 0)
        self.assertEqual(result.replicate_count, 2)
        self.assertEqual(result.decision_count, 3)
        self.assertAlmostEqual(
            float(result.value.detach()),
            0.25 * (math.log(2.0) - math.log(3.0)),
            places=6,
        )
        expected = torch.tensor(
            [-0.25, 0.25, 1.0 / 6.0, -1.0 / 12.0, -1.0 / 12.0, 0.125, -0.125]
        )
        torch.testing.assert_close(values.grad, expected)

    def test_all_win_group_uses_hp_after_win_axis_is_solved(self) -> None:
        values = torch.nn.Parameter(torch.zeros(7))

        def scorer(payload):
            return RaggedCandidateLogits(
                values=values,
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            )

        result = on_policy_combat_win_loss(
            scorer,
            (combat_group_experience_fixture(self.manifest_id, wins=(True, True)),),
            self.registry,
            CONCAT_LIMITS,
            self.config,
            self.objective,
        )
        result.value.backward()

        self.assertEqual(result.signal_group_count, 1)
        self.assertEqual(result.win_signal_group_count, 0)
        self.assertEqual(result.terminal_hp_signal_group_count, 1)
        self.assertAlmostEqual(
            float(result.value.detach()),
            0.1875 * (math.log(2.0) - math.log(3.0)),
            places=6,
        )

    def test_all_win_axis_can_explicitly_disable_hp_learning(self) -> None:
        values = torch.nn.Parameter(torch.zeros(7))

        result = on_policy_combat_win_loss(
            lambda payload: RaggedCandidateLogits(
                values=values,
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            ),
            (combat_group_experience_fixture(self.manifest_id, wins=(True, True)),),
            self.registry,
            CONCAT_LIMITS,
            self.config,
            CombatWinObjectiveConfig(all_win_axis=CombatAllWinAxis.NONE),
        )
        result.value.backward()

        self.assertEqual(result.signal_group_count, 0)
        self.assertEqual(result.win_signal_group_count, 0)
        self.assertEqual(result.terminal_hp_signal_group_count, 0)
        torch.testing.assert_close(values.grad, torch.zeros_like(values))

    def test_mixed_outcomes_use_win_only_even_when_hp_also_varies(self) -> None:
        result = on_policy_combat_win_loss(
            lambda payload: RaggedCandidateLogits(
                values=torch.zeros(7, requires_grad=True),
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            ),
            (combat_group_experience_fixture(self.manifest_id, wins=(True, False)),),
            self.registry,
            CONCAT_LIMITS,
            self.config,
            self.objective,
        )

        self.assertEqual(result.win_signal_group_count, 1)
        self.assertEqual(result.terminal_hp_signal_group_count, 0)

    def test_all_loss_and_potion_only_variation_have_no_signal(self) -> None:
        for wins, final_hps in (
            ((False, False), (0, 0)),
            ((True, True), (70, 70)),
        ):
            with self.subTest(wins=wins):
                values = torch.nn.Parameter(torch.zeros(7))

                def scorer(payload):
                    return RaggedCandidateLogits(
                        values=values,
                        row_splits=torch.as_tensor(
                            payload["candidate_row_splits"],
                            dtype=torch.long,
                        ),
                    )

                result = on_policy_combat_win_loss(
                    scorer,
                    (
                        combat_group_experience_fixture(
                            self.manifest_id,
                            wins=wins,
                            final_hps=final_hps,
                            potions_used=(0, 1),
                        ),
                    ),
                    self.registry,
                    CONCAT_LIMITS,
                    self.config,
                    self.objective,
                )
                result.value.backward()

                self.assertEqual(result.signal_group_count, 0)
                self.assertEqual(result.win_signal_group_count, 0)
                self.assertEqual(result.terminal_hp_signal_group_count, 0)
                self.assertEqual(float(result.value.detach()), 0.0)
                torch.testing.assert_close(values.grad, torch.zeros_like(values))

    def test_unknown_behavior_and_off_policy_probability_fail_closed(self) -> None:
        def scorer(payload):
            return RaggedCandidateLogits(
                values=torch.zeros(7),
                row_splits=torch.as_tensor(
                    payload["candidate_row_splits"],
                    dtype=torch.long,
                ),
            )

        unknown = BehaviorManifestId(b"\xff" * 32)
        with self.assertRaisesRegex(TorchOutcomeError, "unknown behavior"):
            on_policy_combat_win_loss(
                scorer,
                (combat_group_experience_fixture(unknown, wins=(True, False)),),
                self.registry,
                CONCAT_LIMITS,
                self.config,
                self.objective,
            )
        with self.assertRaisesRegex(TorchOutcomeError, "off-policy"):
            on_policy_combat_win_loss(
                scorer,
                (
                    combat_group_experience_fixture(
                        self.manifest_id,
                        wins=(True, False),
                        first_probability=SelectionProbability.known(0.4),
                    ),
                ),
                self.registry,
                CONCAT_LIMITS,
                self.config,
                self.objective,
            )

if __name__ == "__main__":
    unittest.main()
