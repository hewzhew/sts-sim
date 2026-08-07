from __future__ import annotations

import unittest

import numpy as np

from sts_learning import (
    TerminalAttemptOutcome,
    TerminalBatchError,
    TerminalStepBatch,
)


def bridge_step(**overrides: object) -> dict[str, object]:
    step: dict[str, object] = {
        "terminal_slot_indices": [2, 0],
        "terminal_reward": [-1, 1],
        "terminal_act": [2, 3],
        "terminal_floor": [28, 51],
        "terminal_hp": [0, 37],
        "terminal_max_hp": [84, 90],
        "terminal_gold": [42, 101],
        "unrelated_large_tensor": object(),
    }
    step.update(overrides)
    return step


class TerminalStepBatchTests(unittest.TestCase):
    def test_numpy_bridge_columns_are_copied_to_plain_integers(self) -> None:
        step = bridge_step(
            terminal_slot_indices=np.array([2, 0], dtype=np.uint64),
            terminal_reward=np.array([-1, 1], dtype=np.int8),
            terminal_act=np.array([2, 3], dtype=np.uint8),
            terminal_floor=np.array([28, 51], dtype=np.int32),
            terminal_hp=np.array([0, 37], dtype=np.int32),
            terminal_max_hp=np.array([84, 90], dtype=np.int32),
            terminal_gold=np.array([42, 101], dtype=np.int32),
        )

        batch = TerminalStepBatch.from_bridge_step(step, slot_count=3)

        self.assertEqual(batch.slot_indices, (2, 0))
        self.assertTrue(
            all(
                type(value) is int
                for value in batch.attempts[0].__dict__.values()
            )
        )

    def test_bridge_step_copies_only_compact_aligned_terminal_rows(self) -> None:
        step = bridge_step()

        batch = TerminalStepBatch.from_bridge_step(step, slot_count=3)

        self.assertEqual(batch.slot_indices, (2, 0))
        self.assertEqual(batch.attempts[0].terminal_floor, 28)
        self.assertEqual(batch.attempts[1].terminal_hp, 37)
        self.assertNotIn(step["unrelated_large_tensor"], batch.__dict__.values())

    def test_missing_or_misaligned_columns_fail_before_producing_a_batch(self) -> None:
        missing = bridge_step()
        del missing["terminal_gold"]
        with self.assertRaisesRegex(TerminalBatchError, "missing terminal_gold"):
            TerminalStepBatch.from_bridge_step(missing, slot_count=3)

        with self.assertRaisesRegex(TerminalBatchError, "has 1 rows, expected 2"):
            TerminalStepBatch.from_bridge_step(
                bridge_step(terminal_hp=[0]),
                slot_count=3,
            )

    def test_batch_is_bounded_by_pool_and_rejects_duplicate_or_invalid_rows(self) -> None:
        with self.assertRaisesRegex(TerminalBatchError, "outside 0..2"):
            TerminalStepBatch.from_bridge_step(bridge_step(), slot_count=2)
        with self.assertRaisesRegex(TerminalBatchError, "duplicate slots"):
            TerminalStepBatch.from_bridge_step(
                bridge_step(terminal_slot_indices=[0, 0]),
                slot_count=3,
            )
        with self.assertRaisesRegex(TerminalBatchError, "must be -1 or 1"):
            TerminalStepBatch.from_bridge_step(
                bridge_step(terminal_reward=[0, 1]),
                slot_count=3,
            )

    def test_direct_rows_reject_boolean_and_non_integral_values(self) -> None:
        with self.assertRaisesRegex(TerminalBatchError, "not bool"):
            TerminalAttemptOutcome(True, 1, 3, 51, 20, 80, 50)
        with self.assertRaisesRegex(TerminalBatchError, "must be an integer"):
            TerminalAttemptOutcome(0, 1, 3, 51.5, 20, 80, 50)


if __name__ == "__main__":
    unittest.main()
