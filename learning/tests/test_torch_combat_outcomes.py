from __future__ import annotations

import importlib.util
import math
import unittest

from learning.tests.semantic_fixtures import semantic_batch_fixture
from learning.tests.torch_outcome_fixtures import behavior_manifest_fixture
from sts_learning import (
    BehaviorManifestId,
    BehaviorManifestRegistry,
    CombatDecisionExperienceBatch,
    CombatTerminalOutcome,
    CompletedCombatGroup,
    CompletedCombatGroupExperience,
    SelectionProbability,
    SemanticBatchConcatLimits,
    select_semantic_decision_rows,
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


ROOT_ID = "12" * 32
COMBAT_HASH = "ab" * 32
CONCAT_LIMITS = SemanticBatchConcatLimits(
    max_rows=16,
    max_input_array_bytes=1024 * 1024,
)


@unittest.skipUnless(_TORCH_AVAILABLE, "optional PyTorch dependency is not installed")
class OnPolicyCombatWinLossTests(unittest.TestCase):
    def setUp(self) -> None:
        self.config = RaggedCategoricalPolicyConfig(temperature=1.0)
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
            (self._group(wins=(True, False)),),
            self.registry,
            CONCAT_LIMITS,
            self.config,
        )
        result.value.backward()

        self.assertEqual(calls, 1)
        self.assertEqual(result.group_count, 1)
        self.assertEqual(result.signal_group_count, 1)
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

    def test_hp_and_potion_signal_do_not_enter_win_axis(self) -> None:
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
            (self._group(wins=(True, True)),),
            self.registry,
            CONCAT_LIMITS,
            self.config,
        )
        result.value.backward()

        self.assertEqual(result.signal_group_count, 0)
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
                (self._group(wins=(True, False), manifest_id=unknown),),
                self.registry,
                CONCAT_LIMITS,
                self.config,
            )
        with self.assertRaisesRegex(TorchOutcomeError, "off-policy"):
            on_policy_combat_win_loss(
                scorer,
                (
                    self._group(
                        wins=(True, False),
                        first_probability=SelectionProbability.known(0.4),
                    ),
                ),
                self.registry,
                CONCAT_LIMITS,
                self.config,
            )

    def _group(
        self,
        *,
        wins: tuple[bool, bool],
        manifest_id: BehaviorManifestId | None = None,
        first_probability: SelectionProbability | None = None,
    ) -> CompletedCombatGroupExperience:
        manifest = manifest_id if manifest_id is not None else self.manifest_id
        first = CombatDecisionExperienceBatch(
            sequence_index=0,
            root_id=ROOT_ID,
            exact_combat_state_hash=COMBAT_HASH,
            replicate_indices=(0, 1),
            payload=semantic_batch_fixture(),
            selected_ordinals=(0, 0),
            selection_probabilities=(
                first_probability
                if first_probability is not None
                else SelectionProbability.known(0.5),
                SelectionProbability.known(1.0 / 3.0),
            ),
            behavior_manifest_id=manifest,
            decision_count=2,
            payload_bytes=1,
        )
        second = CombatDecisionExperienceBatch(
            sequence_index=1,
            root_id=ROOT_ID,
            exact_combat_state_hash=COMBAT_HASH,
            replicate_indices=(1,),
            payload=select_semantic_decision_rows(semantic_batch_fixture(), [0]),
            selected_ordinals=(0,),
            selection_probabilities=(SelectionProbability.known(0.5),),
            behavior_manifest_id=manifest,
            decision_count=1,
            payload_bytes=1,
        )
        outcomes = CompletedCombatGroup(
            root_id=ROOT_ID,
            exact_combat_state_hash=COMBAT_HASH,
            outcomes=(
                _outcome(0, wins[0], final_hp=70, potions_used=0),
                _outcome(1, wins[1], final_hp=10 if wins[1] else 0, potions_used=1),
            ),
        )
        return CompletedCombatGroupExperience(
            root_id=ROOT_ID,
            exact_combat_state_hash=COMBAT_HASH,
            behavior_manifest_id=manifest,
            batches=(first, second),
            outcomes=outcomes,
            decision_count=3,
            payload_bytes=2,
        )


def _outcome(
    replicate_index: int,
    won: bool,
    *,
    final_hp: int,
    potions_used: int,
) -> CombatTerminalOutcome:
    return CombatTerminalOutcome(
        replicate_index=replicate_index,
        terminal_kind=0 if won else 1,
        won=won,
        start_hp=80,
        final_hp=final_hp,
        hp_loss=80 - final_hp,
        turns=3,
        potions_used=potions_used,
        potions_discarded=0,
        cards_played=8,
    )


if __name__ == "__main__":
    unittest.main()
