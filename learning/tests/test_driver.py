from __future__ import annotations

import unittest
from collections.abc import Mapping, Sequence

import numpy as np

from sts_learning import (
    BatchDriverError,
    ExperienceLimits,
    ExperienceSegment,
    ExperienceSegmentBuffer,
    OnlineBatchDriver,
    RecoveryPlan,
    RecoverySlotSnapshot,
    SeedPartition,
    SeedPartitionSpec,
    SeedSchedule,
    TerminalAccountingBatch,
    initialize_population,
)


class FakeCheckpointBatch:
    def __init__(self, checkpoints: dict[int, tuple[int, int]]) -> None:
        self.checkpoints = dict(checkpoints)

    def __len__(self) -> int:
        return len(self.checkpoints)

    def select(self, slot_indices: list[int]) -> FakeCheckpointBatch:
        if len(set(slot_indices)) != len(slot_indices):
            raise ValueError("duplicate checkpoint selection")
        return FakeCheckpointBatch(
            {slot: self.checkpoints[slot] for slot in slot_indices}
        )

    def updated(self, replacements: FakeCheckpointBatch) -> FakeCheckpointBatch:
        if not replacements.checkpoints.keys() <= self.checkpoints.keys():
            raise ValueError("replacement slot is missing")
        updated = dict(self.checkpoints)
        updated.update(replacements.checkpoints)
        return FakeCheckpointBatch(updated)


class FakeBatchEnv:
    def __init__(
        self,
        seeds: list[int],
        *,
        terminal_plans: Sequence[Mapping[int, int]] = (),
    ) -> None:
        self.seeds = list(seeds)
        self.generations = [0] * len(seeds)
        self.terminal = [False] * len(seeds)
        self._ready = False
        self._round = 0
        self._terminal_plans = list(terminal_plans)
        self.choose_calls: list[list[int]] = []
        self.restore_calls: list[list[int]] = []
        self.reset_calls: list[tuple[list[int], list[int]]] = []

    @property
    def slot_count(self) -> int:
        return len(self.seeds)

    @property
    def terminal_count(self) -> int:
        return sum(self.terminal)

    @property
    def ready(self) -> bool:
        return self._ready

    def decision_batch(self, *, semantic: bool = False) -> dict[str, object]:
        slots = [slot for slot, terminal in enumerate(self.terminal) if not terminal]
        return {
            "slot_indices": slots,
            "candidate_counts": [2] * len(slots),
            "semantic": {"complete": semantic},
        }

    def choose(self, ordinals: list[int]) -> None:
        self.choose_calls.append(list(ordinals))
        self._round += 1
        self._ready = self._round == 2

    def step(self) -> dict[str, object]:
        if not self._ready:
            raise ValueError("not ready")
        active = [slot for slot, terminal in enumerate(self.terminal) if not terminal]
        plan = self._terminal_plans.pop(0) if self._terminal_plans else {}
        terminal_slots = [slot for slot in active if slot in plan]
        rewards = [plan[slot] for slot in terminal_slots]
        for slot in terminal_slots:
            self.terminal[slot] = True
        self._ready = False
        self._round = 0
        return {
            "slot_indices": active,
            "reward": [plan.get(slot, 0) for slot in active],
            "terminated": [slot in plan for slot in active],
            "terminal_slot_indices": terminal_slots,
            "terminal_reward": rewards,
            "terminal_act": [3] * len(terminal_slots),
            "terminal_floor": [40] * len(terminal_slots),
            "terminal_hp": [20 if reward == 1 else 0 for reward in rewards],
            "terminal_max_hp": [80] * len(terminal_slots),
            "terminal_gold": [50] * len(terminal_slots),
        }

    def checkpoint_slots(self, slot_indices: list[int]) -> FakeCheckpointBatch:
        return FakeCheckpointBatch(
            {
                slot: (self.seeds[slot], self.generations[slot])
                for slot in slot_indices
            }
        )

    def restore_slots(
        self,
        slot_indices: list[int],
        checkpoints: object,
    ) -> None:
        assert isinstance(checkpoints, FakeCheckpointBatch)
        self.restore_calls.append(list(slot_indices))
        for slot in slot_indices:
            seed, generation = checkpoints.checkpoints[slot]
            self.seeds[slot] = seed
            self.generations[slot] = generation
            self.terminal[slot] = False
        self._ready = False
        self._round = 0

    def reset_slots(self, slot_indices: list[int], seeds: list[int]) -> None:
        self.reset_calls.append((list(slot_indices), list(seeds)))
        for slot, seed in zip(slot_indices, seeds, strict=True):
            if not self.terminal[slot]:
                raise ValueError("reset target is not terminal")
            self.seeds[slot] = seed
            self.generations[slot] += 1
            self.terminal[slot] = False
        self._ready = False
        self._round = 0

    def reset_slots_checkpointed(
        self,
        slot_indices: list[int],
        seeds: list[int],
    ) -> FakeCheckpointBatch:
        self.reset_slots(slot_indices, seeds)
        return self.checkpoint_slots(slot_indices)


class NumpyFakeBatchEnv(FakeBatchEnv):
    def decision_batch(self, *, semantic: bool = False) -> dict[str, object]:
        raw = super().decision_batch(semantic=semantic)
        slots = np.array(raw["slot_indices"], dtype=np.uint64)
        counts = np.array(raw["candidate_counts"], dtype=np.uint64)
        splits = np.zeros(len(slots) + 1, dtype=np.uint64)
        splits[1:] = np.cumsum(counts)
        return {
            "slot_indices": slots,
            "phase": np.zeros(len(slots), dtype=np.uint8),
            "candidate_counts": counts,
            "candidate_row_splits": splits,
            "semantic": {
                "schema_version": 2,
                "completeness": np.ones(len(slots), dtype=np.uint8),
                "token": {
                    "row_splits": np.arange(
                        len(slots) + 1,
                        dtype=np.uint64,
                    ),
                    "kind": np.zeros(len(slots), dtype=np.uint16),
                },
                "candidate_token_indices": np.arange(
                    int(splits[-1]),
                    dtype=np.uint64,
                ),
            },
        }


class OneRejectedChoiceEnv(NumpyFakeBatchEnv):
    def __init__(self, seeds: list[int]) -> None:
        super().__init__(seeds)
        self.rejected = False

    def choose(self, ordinals: list[int]) -> None:
        if len(self.choose_calls) == 1 and not self.rejected:
            self.rejected = True
            raise RuntimeError("choice rejected")
        super().choose(ordinals)


class RecordingPolicy:
    def __init__(self) -> None:
        self.batch_sizes: list[int] = []

    def choose(self, decision_batch: Mapping[str, object]) -> Sequence[int]:
        slots = decision_batch["slot_indices"]
        assert isinstance(slots, Sequence)
        self.batch_sizes.append(len(slots))
        return [0] * len(slots)


class FirstAttemptRecovery:
    def __init__(self) -> None:
        self.calls = 0

    def plan_recovery(
        self,
        accounting: TerminalAccountingBatch,
        snapshots: tuple[RecoverySlotSnapshot, ...],
    ) -> RecoveryPlan:
        self.calls += 1
        rewards = {
            attempt.slot_index: attempt.terminal_reward
            for attempt in accounting.attempts
        }
        return RecoveryPlan(
            tuple(
                snapshot.slot_index
                for snapshot in snapshots
                if rewards[snapshot.slot_index] == -1
                and snapshot.attempt_index == 1
            )
        )


class InvalidPolicy:
    def choose(self, decision_batch: Mapping[str, object]) -> Sequence[int]:
        return [2] * len(decision_batch["slot_indices"])  # type: ignore[arg-type]


class ArrayFirstPolicy:
    def choose(self, decision_batch: Mapping[str, object]) -> Sequence[int]:
        return [0] * len(decision_batch["slot_indices"])  # type: ignore[arg-type]


class NoRecovery:
    def plan_recovery(
        self,
        accounting: TerminalAccountingBatch,
        snapshots: tuple[RecoverySlotSnapshot, ...],
    ) -> RecoveryPlan:
        return RecoveryPlan()


class BatchDriverTests(unittest.TestCase):
    def test_initial_population_uses_one_seed_plan_for_every_owner(self) -> None:
        schedule = SeedSchedule(
            SeedPartition.TRAINING,
            SeedPartitionSpec(held_out_numerator=1, denominator=4),
        )
        created: list[FakeBatchEnv] = []

        def factory(seeds: list[int]) -> FakeBatchEnv:
            env = FakeBatchEnv(seeds)
            created.append(env)
            return env

        population = initialize_population(
            factory,
            slot_count=3,
            schedule=schedule,
            max_recoveries_per_episode=2,
        )

        self.assertEqual(len(created), 1)
        env = created[0]
        self.assertEqual(
            tuple(env.seeds),
            tuple(snapshot.episode_seed for snapshot in population.ledger.snapshots()),
        )
        self.assertEqual(len(population.checkpoint_bank), 3)
        self.assertGreater(population.schedule.next_candidate, schedule.next_candidate)

    def test_driver_batches_policy_and_resolves_recovery_completion_and_reset(self) -> None:
        envs: list[FakeBatchEnv] = []

        def factory(seeds: list[int]) -> FakeBatchEnv:
            env = FakeBatchEnv(
                seeds,
                terminal_plans=({0: -1, 1: 1}, {0: -1}),
            )
            envs.append(env)
            return env

        population = initialize_population(
            factory,
            slot_count=2,
            schedule=SeedSchedule(SeedPartition.TRAINING),
            max_recoveries_per_episode=1,
        )
        initial_seeds = tuple(envs[0].seeds)
        policy = RecordingPolicy()
        curriculum = FirstAttemptRecovery()
        driver = OnlineBatchDriver(
            population,
            policy=policy,
            curriculum=curriculum,
        )

        summary = driver.run(batch_steps=2)

        env = envs[0]
        self.assertEqual(policy.batch_sizes, [2, 2, 2, 2])
        self.assertEqual(curriculum.calls, 2)
        self.assertEqual(env.restore_calls, [[0]])
        self.assertEqual([call[0] for call in env.reset_calls], [[1], [0]])
        self.assertEqual(summary.batch_steps, 2)
        self.assertEqual(summary.slot_steps, 4)
        self.assertEqual(summary.decision_rounds, 4)
        self.assertEqual(summary.terminal_attempts, 3)
        self.assertEqual(summary.completed_episodes, 2)
        self.assertEqual(summary.recoveries, 1)
        self.assertEqual(summary.active_slots, 2)
        self.assertGreater(summary.steps_per_second, 0.0)
        self.assertEqual(driver.ledger.snapshot(0).episode_generation, 1)
        self.assertEqual(driver.ledger.snapshot(1).episode_generation, 1)
        self.assertNotEqual(tuple(env.seeds), initial_seeds)

    def test_invalid_policy_is_rejected_before_environment_mutation(self) -> None:
        envs: list[FakeBatchEnv] = []

        def factory(seeds: list[int]) -> FakeBatchEnv:
            env = FakeBatchEnv(seeds)
            envs.append(env)
            return env

        population = initialize_population(
            factory,
            slot_count=2,
            schedule=SeedSchedule(SeedPartition.HELD_OUT),
            max_recoveries_per_episode=0,
        )
        driver = OnlineBatchDriver(
            population,
            policy=InvalidPolicy(),
            curriculum=NoRecovery(),
        )

        with self.assertRaisesRegex(BatchDriverError, "outside"):
            driver.advance()
        self.assertEqual(envs[0].choose_calls, [])
        self.assertEqual(envs[0].terminal_count, 0)

    def test_population_rejects_environment_slot_mismatch(self) -> None:
        with self.assertRaisesRegex(BatchDriverError, "created 1 slots"):
            initialize_population(
                lambda seeds: FakeBatchEnv(seeds[:-1]),
                slot_count=2,
                schedule=SeedSchedule(SeedPartition.TRAINING),
                max_recoveries_per_episode=0,
            )

    def test_driver_consumes_bounded_segments_without_retaining_a_queue(self) -> None:
        envs: list[NumpyFakeBatchEnv] = []

        def factory(seeds: list[int]) -> NumpyFakeBatchEnv:
            env = NumpyFakeBatchEnv(
                seeds,
                terminal_plans=({0: 1, 1: -1},),
            )
            envs.append(env)
            return env

        population = initialize_population(
            factory,
            slot_count=2,
            schedule=SeedSchedule(SeedPartition.HELD_OUT),
            max_recoveries_per_episode=0,
        )
        buffer = ExperienceSegmentBuffer(
            ExperienceLimits(
                max_decisions=2,
                max_payload_bytes=1_000_000,
            )
        )
        consumed: list[ExperienceSegment] = []
        driver = OnlineBatchDriver(
            population,
            policy=ArrayFirstPolicy(),
            curriculum=NoRecovery(),
            experience_buffer=buffer,
            experience_sink=consumed.append,
        )

        result = driver.advance()

        self.assertEqual(result.emitted_experience_segments, 1)
        self.assertEqual(result.emitted_experience_decisions, 2)
        self.assertEqual(len(consumed), 1)
        self.assertTrue(consumed[0].censored)
        self.assertEqual(buffer.decision_count, 2)
        final = driver.flush_experience()
        assert final is not None
        self.assertEqual(len(consumed), 2)
        self.assertFalse(final.censored)
        self.assertTrue(all(fragment.terminal for fragment in final.attempts))
        self.assertTrue(buffer.empty)

    def test_experience_buffer_and_sink_must_be_paired(self) -> None:
        population = initialize_population(
            lambda seeds: FakeBatchEnv(seeds),
            slot_count=1,
            schedule=SeedSchedule(SeedPartition.TRAINING),
            max_recoveries_per_episode=0,
        )
        with self.assertRaisesRegex(BatchDriverError, "configured together"):
            OnlineBatchDriver(
                population,
                policy=RecordingPolicy(),
                curriculum=NoRecovery(),
                experience_buffer=ExperienceSegmentBuffer(
                    ExperienceLimits(
                        max_decisions=1,
                        max_payload_bytes=1024,
                    )
                ),
            )

    def test_failing_experience_sink_stops_before_the_current_choice(self) -> None:
        envs: list[NumpyFakeBatchEnv] = []

        def factory(seeds: list[int]) -> NumpyFakeBatchEnv:
            env = NumpyFakeBatchEnv(seeds)
            envs.append(env)
            return env

        population = initialize_population(
            factory,
            slot_count=2,
            schedule=SeedSchedule(SeedPartition.TRAINING),
            max_recoveries_per_episode=0,
        )

        def fail_sink(segment: ExperienceSegment) -> None:
            raise RuntimeError("sink failed")

        driver = OnlineBatchDriver(
            population,
            policy=ArrayFirstPolicy(),
            curriculum=NoRecovery(),
            experience_buffer=ExperienceSegmentBuffer(
                ExperienceLimits(
                    max_decisions=2,
                    max_payload_bytes=1_000_000,
                )
            ),
            experience_sink=fail_sink,
        )

        with self.assertRaisesRegex(RuntimeError, "sink failed"):
            driver.advance()
        self.assertEqual(len(envs[0].choose_calls), 1)
        with self.assertRaisesRegex(BatchDriverError, "previously failed"):
            driver.advance()

    def test_rejected_choice_is_not_committed_as_experience(self) -> None:
        envs: list[OneRejectedChoiceEnv] = []

        def factory(seeds: list[int]) -> OneRejectedChoiceEnv:
            env = OneRejectedChoiceEnv(seeds)
            envs.append(env)
            return env

        population = initialize_population(
            factory,
            slot_count=2,
            schedule=SeedSchedule(SeedPartition.TRAINING),
            max_recoveries_per_episode=0,
        )
        buffer = ExperienceSegmentBuffer(
            ExperienceLimits(
                max_decisions=2,
                max_payload_bytes=1_000_000,
            )
        )
        consumed: list[ExperienceSegment] = []
        driver = OnlineBatchDriver(
            population,
            policy=ArrayFirstPolicy(),
            curriculum=NoRecovery(),
            experience_buffer=buffer,
            experience_sink=consumed.append,
        )

        with self.assertRaisesRegex(RuntimeError, "choice rejected"):
            driver.advance()
        self.assertEqual(len(consumed), 1)
        self.assertTrue(buffer.empty)

        result = driver.advance()
        self.assertEqual(result.emitted_experience_segments, 0)
        self.assertEqual(buffer.decision_count, 2)
        self.assertEqual(len(envs[0].choose_calls), 2)


if __name__ == "__main__":
    unittest.main()
