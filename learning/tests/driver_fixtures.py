from __future__ import annotations

from collections.abc import Mapping, Sequence

import numpy as np

from learning.tests.policy_fixtures import BEHAVIOR_MANIFEST_ID
from sts_learning import (
    BatchPolicyChoice,
    DETERMINISTIC_SELECTION,
    RecoveryPlan,
    RecoverySlotSnapshot,
    TerminalAccountingBatch,
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

    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        slots = decision_batch["slot_indices"]
        assert isinstance(slots, Sequence)
        self.batch_sizes.append(len(slots))
        return BatchPolicyChoice.deterministic(
            [0] * len(slots),
            BEHAVIOR_MANIFEST_ID,
        )


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
    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        return BatchPolicyChoice.deterministic(
            [2] * len(decision_batch["slot_indices"]),  # type: ignore[arg-type]
            BEHAVIOR_MANIFEST_ID,
        )


class ArrayFirstPolicy:
    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        return BatchPolicyChoice.deterministic(
            [0] * len(decision_batch["slot_indices"]),  # type: ignore[arg-type]
            BEHAVIOR_MANIFEST_ID,
        )


class UntypedPolicy:
    def choose(self, decision_batch: Mapping[str, object]) -> Sequence[int]:
        return [0] * len(decision_batch["slot_indices"])  # type: ignore[arg-type]


def _forged_choice(
    row_count: int,
    probabilities: tuple[object, ...],
) -> BatchPolicyChoice:
    choice = object.__new__(BatchPolicyChoice)
    object.__setattr__(choice, "ordinals", (0,) * row_count)
    object.__setattr__(choice, "behavior_manifest_id", BEHAVIOR_MANIFEST_ID)
    object.__setattr__(choice, "selection_probabilities", probabilities)
    return choice


class MisalignedProbabilityPolicy:
    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        row_count = len(decision_batch["slot_indices"])  # type: ignore[arg-type]
        return _forged_choice(row_count, (DETERMINISTIC_SELECTION,))


class UntypedProbabilityPolicy:
    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice:
        row_count = len(decision_batch["slot_indices"])  # type: ignore[arg-type]
        return _forged_choice(row_count, (1.0,) * row_count)


class NoRecovery:
    def plan_recovery(
        self,
        accounting: TerminalAccountingBatch,
        snapshots: tuple[RecoverySlotSnapshot, ...],
    ) -> RecoveryPlan:
        return RecoveryPlan()
