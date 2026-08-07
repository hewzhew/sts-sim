from __future__ import annotations

import unittest

from learning.tests.driver_fixtures import FakeBatchEnv, RecordingPolicy
from learning.tests.policy_fixtures import BEHAVIOR_MANIFEST_ID
from sts_learning import (
    BatchPolicyChoice,
    BehaviorManifestId,
    HeldOutEvaluationError,
    HeldOutEvaluationSpec,
    SeedPartition,
    SeedSchedule,
    evaluate_held_out_behavior,
)


class HeldOutEvaluationTests(unittest.TestCase):
    def test_evaluation_reports_atomic_multi_terminal_outcomes(self) -> None:
        created: list[FakeBatchEnv] = []

        def factory(seeds: list[int]) -> FakeBatchEnv:
            env = FakeBatchEnv(
                seeds,
                terminal_plans=({0: 1, 1: -1}, {0: 1}),
            )
            created.append(env)
            return env

        schedule = SeedSchedule(SeedPartition.HELD_OUT)
        result = evaluate_held_out_behavior(
            factory,
            RecordingPolicy(),
            schedule=schedule,
            spec=HeldOutEvaluationSpec(
                slot_count=2,
                terminal_attempt_target=1,
                max_batch_steps=5,
            ),
        )

        self.assertTrue(result.complete)
        self.assertFalse(result.step_limit_reached)
        self.assertEqual(result.run.summary.batch_steps, 1)
        self.assertEqual(result.run.summary.terminal_attempts, 2)
        self.assertEqual(result.run.summary.victories, 1)
        self.assertEqual(result.run.summary.defeats, 1)
        self.assertEqual(result.run.summary.recoveries, 0)
        self.assertEqual(result.behavior_manifest_id, BEHAVIOR_MANIFEST_ID)
        self.assertEqual(created[0].restore_calls, [])
        self.assertEqual(result.schedule_start, schedule)
        self.assertGreater(result.schedule_end.next_candidate, schedule.next_candidate)

    def test_same_schedule_repeats_the_same_seed_and_outcome_prefix(self) -> None:
        created_seeds: list[tuple[int, ...]] = []

        def factory(seeds: list[int]) -> FakeBatchEnv:
            created_seeds.append(tuple(seeds))
            return FakeBatchEnv(seeds, terminal_plans=({0: -1},))

        schedule = SeedSchedule(SeedPartition.HELD_OUT, next_candidate=17)
        spec = HeldOutEvaluationSpec(
            slot_count=1,
            terminal_attempt_target=1,
            max_batch_steps=3,
        )
        first = evaluate_held_out_behavior(
            factory,
            RecordingPolicy(),
            schedule=schedule,
            spec=spec,
        )
        second = evaluate_held_out_behavior(
            factory,
            RecordingPolicy(),
            schedule=schedule,
            spec=spec,
        )

        self.assertEqual(created_seeds[0], created_seeds[1])
        self.assertEqual(first.schedule_end, second.schedule_end)
        self.assertEqual(first.run.summary.batch_steps, second.run.summary.batch_steps)
        self.assertEqual(first.run.summary.victories, second.run.summary.victories)
        self.assertEqual(first.run.summary.defeats, second.run.summary.defeats)

    def test_budget_exhaustion_and_training_schedule_are_explicit(self) -> None:
        exhausted = evaluate_held_out_behavior(
            FakeBatchEnv,
            RecordingPolicy(),
            schedule=SeedSchedule(SeedPartition.HELD_OUT),
            spec=HeldOutEvaluationSpec(
                slot_count=1,
                terminal_attempt_target=1,
                max_batch_steps=0,
            ),
        )

        self.assertFalse(exhausted.complete)
        self.assertTrue(exhausted.step_limit_reached)
        self.assertEqual(exhausted.run.summary.terminal_attempts, 0)
        with self.assertRaisesRegex(HeldOutEvaluationError, "held-out"):
            evaluate_held_out_behavior(
                FakeBatchEnv,
                RecordingPolicy(),
                schedule=SeedSchedule(SeedPartition.TRAINING),
                spec=HeldOutEvaluationSpec(
                    slot_count=1,
                    terminal_attempt_target=1,
                    max_batch_steps=1,
                ),
            )

    def test_policy_cannot_mix_behavior_manifests_inside_one_evaluation(self) -> None:
        alternative = BehaviorManifestId(b"\x91" * 32)

        class SwitchingPolicy:
            behavior_manifest_id = BEHAVIOR_MANIFEST_ID

            def __init__(self) -> None:
                self.calls = 0

            def choose(self, decision_batch):
                self.calls += 1
                manifest_id = (
                    self.behavior_manifest_id if self.calls == 1 else alternative
                )
                return BatchPolicyChoice.deterministic(
                    [0] * len(decision_batch["slot_indices"]),
                    manifest_id,
                )

        with self.assertRaisesRegex(HeldOutEvaluationError, "changed behavior"):
            evaluate_held_out_behavior(
                FakeBatchEnv,
                SwitchingPolicy(),
                schedule=SeedSchedule(SeedPartition.HELD_OUT),
                spec=HeldOutEvaluationSpec(
                    slot_count=1,
                    terminal_attempt_target=1,
                    max_batch_steps=1,
                ),
            )


if __name__ == "__main__":
    unittest.main()
