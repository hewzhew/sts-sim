"""Bounded held-out behavior evaluation without trajectory retention."""

from __future__ import annotations

import operator
from collections.abc import Callable
from dataclasses import dataclass

from .driver import (
    BatchEnvironment,
    BatchPolicy,
    OnlineBatchDriver,
    RecoveryPlan,
    TerminalTargetRunResult,
    initialize_population,
)
from .policy import BatchPolicyChoice, BehaviorManifestId
from .recovery import RecoverySlotSnapshot, TerminalAccountingBatch
from .seeds import SeedPartition, SeedSchedule


class HeldOutEvaluationError(ValueError):
    """A held-out evaluation request is malformed or crosses ownership bounds."""


@dataclass(frozen=True)
class HeldOutEvaluationSpec:
    slot_count: int
    terminal_attempt_target: int
    max_batch_steps: int

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "slot_count",
            _count(self.slot_count, "slot_count", positive=True),
        )
        object.__setattr__(
            self,
            "terminal_attempt_target",
            _count(
                self.terminal_attempt_target,
                "terminal_attempt_target",
                positive=False,
            ),
        )
        object.__setattr__(
            self,
            "max_batch_steps",
            _count(self.max_batch_steps, "max_batch_steps", positive=False),
        )


@dataclass(frozen=True)
class HeldOutEvaluationResult:
    behavior_manifest_id: BehaviorManifestId
    schedule_start: SeedSchedule
    schedule_end: SeedSchedule
    run: TerminalTargetRunResult

    @property
    def complete(self) -> bool:
        return self.run.target_reached

    @property
    def step_limit_reached(self) -> bool:
        return self.run.step_limit_reached


class _NoHeldOutRecovery:
    def plan_recovery(
        self,
        accounting: TerminalAccountingBatch,
        snapshots: tuple[RecoverySlotSnapshot, ...],
    ) -> RecoveryPlan:
        return RecoveryPlan()


class _ManifestLockedPolicy:
    def __init__(self, policy: BatchPolicy, manifest_id: BehaviorManifestId) -> None:
        self.policy = policy
        self.behavior_manifest_id = manifest_id

    def choose(self, decision_batch) -> BatchPolicyChoice:
        choice = self.policy.choose(decision_batch)
        if not isinstance(choice, BatchPolicyChoice):
            raise HeldOutEvaluationError(
                "evaluation policy must return BatchPolicyChoice"
            )
        if choice.behavior_manifest_id != self.behavior_manifest_id:
            raise HeldOutEvaluationError(
                "evaluation policy changed behavior manifest identity"
            )
        return choice


def evaluate_held_out_behavior(
    env_factory: Callable[[list[int]], BatchEnvironment],
    policy: BatchPolicy,
    *,
    schedule: SeedSchedule,
    spec: HeldOutEvaluationSpec,
) -> HeldOutEvaluationResult:
    """Evaluate one policy on a reproducible held-out seed prefix."""

    if not callable(env_factory):
        raise HeldOutEvaluationError("evaluation environment factory must be callable")
    if not callable(getattr(policy, "choose", None)):
        raise HeldOutEvaluationError("evaluation policy must provide choose()")
    manifest_id = getattr(policy, "behavior_manifest_id", None)
    if not isinstance(manifest_id, BehaviorManifestId):
        raise HeldOutEvaluationError(
            "evaluation policy must expose one typed behavior manifest identity"
        )
    if not isinstance(schedule, SeedSchedule):
        raise HeldOutEvaluationError("evaluation schedule must be a SeedSchedule")
    if schedule.partition is not SeedPartition.HELD_OUT:
        raise HeldOutEvaluationError("evaluation requires a held-out seed schedule")
    if not isinstance(spec, HeldOutEvaluationSpec):
        raise HeldOutEvaluationError("evaluation spec must be typed")

    population = initialize_population(
        env_factory,
        slot_count=spec.slot_count,
        schedule=schedule,
        max_recoveries_per_episode=0,
    )
    driver = OnlineBatchDriver(
        population,
        policy=_ManifestLockedPolicy(policy, manifest_id),
        curriculum=_NoHeldOutRecovery(),
    )
    run = driver.run_until_terminal_attempts(
        terminal_attempts=spec.terminal_attempt_target,
        max_batch_steps=spec.max_batch_steps,
    )
    return HeldOutEvaluationResult(
        behavior_manifest_id=manifest_id,
        schedule_start=schedule,
        schedule_end=driver.schedule,
        run=run,
    )


def _count(value: int, name: str, *, positive: bool) -> int:
    if isinstance(value, bool):
        raise HeldOutEvaluationError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise HeldOutEvaluationError(f"{name} must be an integer") from error
    lower = 1 if positive else 0
    if normalized < lower:
        relation = "positive" if positive else "non-negative"
        raise HeldOutEvaluationError(f"{name} must be {relation}")
    return normalized
