from __future__ import annotations

import unittest
from collections.abc import Mapping, Sequence

from sts_learning import (
    OnlineBatchDriver,
    RecoveryPlan,
    RecoverySlotSnapshot,
    SeedPartition,
    SeedSchedule,
    TerminalAccountingBatch,
    initialize_population,
)

try:
    from sts_learning_bridge import LearningBatchEnv
except ImportError:
    LearningBatchEnv = None  # type: ignore[assignment,misc]


class FirstLegalPolicy:
    def choose(self, decision_batch: Mapping[str, object]) -> Sequence[int]:
        return [0] * len(decision_batch["slot_indices"])  # type: ignore[arg-type]


class NoRecoveryCurriculum:
    def plan_recovery(
        self,
        accounting: TerminalAccountingBatch,
        snapshots: tuple[RecoverySlotSnapshot, ...],
    ) -> RecoveryPlan:
        return RecoveryPlan()


@unittest.skipIf(LearningBatchEnv is None, "standalone bridge wheel is not installed")
class BridgeDriverIntegrationTests(unittest.TestCase):
    def test_real_bridge_runs_bounded_continuous_population(self) -> None:
        assert LearningBatchEnv is not None
        population = initialize_population(
            LearningBatchEnv,
            slot_count=3,
            schedule=SeedSchedule(SeedPartition.HELD_OUT),
            max_recoveries_per_episode=0,
        )
        driver = OnlineBatchDriver(
            population,
            policy=FirstLegalPolicy(),
            curriculum=NoRecoveryCurriculum(),
        )

        summary = driver.run(batch_steps=160)

        self.assertEqual(summary.batch_steps, 160)
        self.assertEqual(summary.slot_steps, 480)
        self.assertGreater(summary.terminal_attempts, 0)
        self.assertEqual(summary.terminal_attempts, summary.completed_episodes)
        self.assertEqual(summary.recoveries, 0)
        self.assertEqual(summary.active_slots, 3)


if __name__ == "__main__":
    unittest.main()
