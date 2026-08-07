from __future__ import annotations

import unittest

from sts_learning import (
    RecoveryLedger,
    RecoveryProtocolError,
    RecoverySlotStatus,
    reset_with_accounting,
    restore_with_accounting,
)


class FakeRestoreEnv:
    def __init__(self, *, fail: bool = False) -> None:
        self.fail = fail
        self.calls: list[tuple[list[int], object]] = []

    def restore_slots(self, slot_indices: list[int], checkpoints: object) -> None:
        self.calls.append((slot_indices, checkpoints))
        if self.fail:
            raise RuntimeError("restore failed")


class FakeResetEnv:
    def __init__(self, *, fail: bool = False) -> None:
        self.fail = fail
        self.calls: list[tuple[list[int], list[int]]] = []

    def reset_slots(self, slot_indices: list[int], seeds: list[int]) -> None:
        self.calls.append((slot_indices, seeds))
        if self.fail:
            raise RuntimeError("reset failed")


class RecoveryLedgerTests(unittest.TestCase):
    def test_training_lifecycle_reports_attempts_and_zero_recovery(self) -> None:
        ledger = RecoveryLedger.training(2, max_recoveries_per_episode=2)

        victories = ledger.record_terminal([0, 1], [-1, 1])
        self.assertEqual(len(victories), 1)
        self.assertEqual(victories[0].slot_index, 1)
        self.assertTrue(victories[0].zero_recovery)

        first = ledger.prepare_recovery([0])
        events = ledger.commit_recovery(first)
        self.assertEqual(events[0].attempt_index, 2)
        self.assertEqual(events[0].recoveries_used, 1)
        ledger.record_terminal([0], [-1])
        ledger.commit_recovery(ledger.prepare_recovery([0]))
        ledger.record_terminal([0], [-1])
        with self.assertRaisesRegex(RecoveryProtocolError, "exhausted"):
            ledger.prepare_recovery([0])

        defeats = ledger.complete_defeats([0])
        self.assertEqual(defeats[0].attempts, 3)
        self.assertEqual(defeats[0].recoveries_used, 2)
        self.assertFalse(defeats[0].zero_recovery)
        reset = ledger.prepare_reset([0, 1])
        ledger.commit_reset(reset)
        self.assertEqual(ledger.snapshot(0).episode_generation, 1)
        self.assertEqual(ledger.snapshot(0).status, RecoverySlotStatus.ACTIVE)

    def test_held_out_mode_cannot_prepare_recovery(self) -> None:
        ledger = RecoveryLedger.held_out(1)
        ledger.record_terminal([0], [-1])

        with self.assertRaisesRegex(RecoveryProtocolError, "forbids recovery"):
            ledger.prepare_recovery([0])
        outcome = ledger.complete_defeats([0])[0]
        self.assertTrue(outcome.zero_recovery)
        self.assertEqual(outcome.attempts, 1)

    def test_invalid_terminal_batch_does_not_mutate_valid_prefix(self) -> None:
        ledger = RecoveryLedger.training(2, max_recoveries_per_episode=1)

        with self.assertRaisesRegex(RecoveryProtocolError, "must be -1 or 1"):
            ledger.record_terminal([0, 1], [-1, 0])
        self.assertTrue(
            all(
                snapshot.status is RecoverySlotStatus.ACTIVE
                for snapshot in ledger.snapshots()
            )
        )

    def test_restore_failure_keeps_defeat_pending_before_commit(self) -> None:
        ledger = RecoveryLedger.training(1, max_recoveries_per_episode=1)
        ledger.record_terminal([0], [-1])
        checkpoints = object()
        failing = FakeRestoreEnv(fail=True)

        with self.assertRaisesRegex(RuntimeError, "restore failed"):
            restore_with_accounting(failing, [0], checkpoints, ledger)
        self.assertEqual(ledger.snapshot(0).status, RecoverySlotStatus.DEFEAT_PENDING)
        self.assertEqual(ledger.snapshot(0).recoveries_used, 0)

        working = FakeRestoreEnv()
        events = restore_with_accounting(working, [0], checkpoints, ledger)
        self.assertEqual(working.calls, [([0], checkpoints)])
        self.assertEqual(events[0].recoveries_used, 1)
        self.assertEqual(ledger.snapshot(0).status, RecoverySlotStatus.ACTIVE)

    def test_reset_failure_keeps_completed_generations_before_commit(self) -> None:
        ledger = RecoveryLedger.held_out(2)
        ledger.record_terminal([0, 1], [1, -1])
        ledger.complete_defeats([1])
        failing = FakeResetEnv(fail=True)

        with self.assertRaisesRegex(RuntimeError, "reset failed"):
            reset_with_accounting(failing, [0, 1], [101, 102], ledger)
        self.assertEqual(ledger.snapshot(0).episode_generation, 0)
        self.assertEqual(ledger.snapshot(1).episode_generation, 0)

        working = FakeResetEnv()
        reset_with_accounting(working, [0, 1], [101, 102], ledger)
        self.assertEqual(working.calls, [([0, 1], [101, 102])])
        self.assertEqual(ledger.snapshot(0).episode_generation, 1)
        self.assertEqual(ledger.snapshot(1).status, RecoverySlotStatus.ACTIVE)


if __name__ == "__main__":
    unittest.main()
