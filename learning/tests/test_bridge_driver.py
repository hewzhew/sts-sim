from __future__ import annotations

import unittest
from collections.abc import Mapping, Sequence

from sts_learning import (
    ExperienceLimits,
    ExperienceSegment,
    ExperienceSegmentBuffer,
    OnlineBatchDriver,
    RecoveryPlan,
    RecoverySlotSnapshot,
    SeedPartition,
    SeedSchedule,
    TerminalAccountingBatch,
    initialize_population,
    iter_payload_arrays,
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


class CountingSegmentSink:
    def __init__(self, limits: ExperienceLimits) -> None:
        self.limits = limits
        self.segments = 0
        self.decisions = 0
        self.payload_bytes = 0
        self.terminal_attempts = 0

    def __call__(self, segment: ExperienceSegment) -> None:
        assert segment.decision_count <= self.limits.max_decisions
        assert segment.payload_bytes <= self.limits.max_payload_bytes
        for batch in segment.batches:
            assert all(
                not array.flags.writeable
                for array in iter_payload_arrays(batch.payload)
            )
        self.segments += 1
        self.decisions += segment.decision_count
        self.payload_bytes += segment.payload_bytes
        self.terminal_attempts += sum(
            fragment.terminal is not None for fragment in segment.attempts
        )


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
        limits = ExperienceLimits(
            max_decisions=48,
            max_payload_bytes=16 * 1024 * 1024,
        )
        sink = CountingSegmentSink(limits)
        driver = OnlineBatchDriver(
            population,
            policy=FirstLegalPolicy(),
            curriculum=NoRecoveryCurriculum(),
            experience_buffer=ExperienceSegmentBuffer(limits),
            experience_sink=sink,
        )

        summary = driver.run(batch_steps=160)
        flushed = driver.flush_experience()

        self.assertEqual(summary.batch_steps, 160)
        self.assertEqual(summary.slot_steps, 480)
        self.assertGreater(summary.terminal_attempts, 0)
        self.assertEqual(summary.terminal_attempts, summary.completed_episodes)
        self.assertEqual(summary.recoveries, 0)
        self.assertEqual(summary.active_slots, 3)
        self.assertGreater(summary.emitted_experience_segments, 0)
        self.assertLessEqual(
            summary.open_experience_decisions,
            limits.max_decisions,
        )
        self.assertLessEqual(
            summary.open_experience_payload_bytes,
            limits.max_payload_bytes,
        )
        self.assertIsNotNone(flushed)
        self.assertEqual(sink.segments, summary.emitted_experience_segments + 1)
        self.assertGreater(sink.decisions, summary.emitted_experience_decisions)
        self.assertGreater(sink.terminal_attempts, 0)


if __name__ == "__main__":
    unittest.main()
