"""Bounded held-out behavior evaluation without trajectory retention."""

from __future__ import annotations

import operator
from collections.abc import Callable
from dataclasses import dataclass, field

from .driver import (
    BatchEnvironment,
    BatchPolicy,
    OnlineBatchDriver,
    RecoveryPlan,
    TerminalTargetRunResult,
    initialize_population,
)
from .decision_progress import BridgeDecisionProgressProvider
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


@dataclass(frozen=True)
class PairedHeldOutEvaluationSpec:
    """One immutable held-out contract shared by two frozen behaviors."""

    schedule: SeedSchedule
    evaluation: HeldOutEvaluationSpec

    def __post_init__(self) -> None:
        if not isinstance(self.schedule, SeedSchedule):
            raise HeldOutEvaluationError(
                "paired evaluation schedule must be a SeedSchedule"
            )
        if self.schedule.partition is not SeedPartition.HELD_OUT:
            raise HeldOutEvaluationError(
                "paired evaluation requires a held-out seed schedule"
            )
        if not isinstance(self.evaluation, HeldOutEvaluationSpec):
            raise HeldOutEvaluationError(
                "paired evaluation spec must contain a typed evaluation"
            )


@dataclass(frozen=True)
class HeldOutEvaluationDelta:
    """Pure ``right - left`` arithmetic without a quality interpretation."""

    terminal_attempts: int
    victories: int
    defeats: int
    terminal_floor_sum: int
    batch_steps: int


@dataclass(frozen=True)
class PairedHeldOutEvaluationResult:
    """Two manifest-owned evaluations under one exact held-out contract."""

    left: HeldOutEvaluationResult
    right: HeldOutEvaluationResult
    right_minus_left: HeldOutEvaluationDelta = field(init=False)
    comparable: bool = field(init=False)

    def __post_init__(self) -> None:
        if not isinstance(self.left, HeldOutEvaluationResult):
            raise HeldOutEvaluationError(
                "paired evaluation left result must be typed"
            )
        if not isinstance(self.right, HeldOutEvaluationResult):
            raise HeldOutEvaluationError(
                "paired evaluation right result must be typed"
            )
        if self.left.behavior_manifest_id == self.right.behavior_manifest_id:
            raise HeldOutEvaluationError(
                "paired evaluation requires distinct behavior manifest identities"
            )
        if self.left.schedule_start != self.right.schedule_start:
            raise HeldOutEvaluationError(
                "paired evaluation results use different seed schedules"
            )
        if (
            self.left.run.terminal_attempt_target
            != self.right.run.terminal_attempt_target
        ):
            raise HeldOutEvaluationError(
                "paired evaluation results use different terminal targets"
            )
        if self.left.run.batch_step_limit != self.right.run.batch_step_limit:
            raise HeldOutEvaluationError(
                "paired evaluation results use different batch-step limits"
            )
        if self.left.run.summary.active_slots != self.right.run.summary.active_slots:
            raise HeldOutEvaluationError(
                "paired evaluation results use different slot counts"
            )

        left_summary = self.left.run.summary
        right_summary = self.right.run.summary
        object.__setattr__(
            self,
            "right_minus_left",
            HeldOutEvaluationDelta(
                terminal_attempts=(
                    right_summary.terminal_attempts
                    - left_summary.terminal_attempts
                ),
                victories=right_summary.victories - left_summary.victories,
                defeats=right_summary.defeats - left_summary.defeats,
                terminal_floor_sum=(
                    right_summary.terminal_progress.floor_sum
                    - left_summary.terminal_progress.floor_sum
                ),
                batch_steps=right_summary.batch_steps - left_summary.batch_steps,
            ),
        )
        object.__setattr__(
            self,
            "comparable",
            self.left.complete
            and self.right.complete
            and (
                self.left.run.terminal_attempt_target
                == self.right.run.terminal_attempt_target
            ),
        )


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
    bind_progress = getattr(policy, "bind_progress_provider", None)
    active_policy = (
        bind_progress(BridgeDecisionProgressProvider(population.env))
        if callable(bind_progress)
        else policy
    )
    if getattr(active_policy, "behavior_manifest_id", None) != manifest_id:
        raise HeldOutEvaluationError(
            "environment binding changed behavior manifest identity"
        )
    driver = OnlineBatchDriver(
        population,
        policy=_ManifestLockedPolicy(active_policy, manifest_id),
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


def evaluate_paired_held_out_behaviors(
    env_factory: Callable[[list[int]], BatchEnvironment],
    left_policy: BatchPolicy,
    right_policy: BatchPolicy,
    *,
    spec: PairedHeldOutEvaluationSpec,
) -> PairedHeldOutEvaluationResult:
    """Evaluate two distinct frozen behaviors under one held-out contract."""

    if not isinstance(spec, PairedHeldOutEvaluationSpec):
        raise HeldOutEvaluationError("paired evaluation spec must be typed")
    if not callable(env_factory):
        raise HeldOutEvaluationError(
            "paired evaluation environment factory must be callable"
        )
    if left_policy is right_policy:
        raise HeldOutEvaluationError(
            "paired evaluation requires two distinct frozen policy objects"
        )
    left_manifest_id = _policy_manifest_id(left_policy, "left")
    right_manifest_id = _policy_manifest_id(right_policy, "right")
    if left_manifest_id == right_manifest_id:
        raise HeldOutEvaluationError(
            "paired evaluation requires distinct behavior manifest identities"
        )

    left = evaluate_held_out_behavior(
        env_factory,
        left_policy,
        schedule=spec.schedule,
        spec=spec.evaluation,
    )
    right = evaluate_held_out_behavior(
        env_factory,
        right_policy,
        schedule=spec.schedule,
        spec=spec.evaluation,
    )
    return PairedHeldOutEvaluationResult(left=left, right=right)


def _policy_manifest_id(policy: BatchPolicy, side: str) -> BehaviorManifestId:
    if not callable(getattr(policy, "choose", None)):
        raise HeldOutEvaluationError(
            f"paired evaluation {side} policy must provide choose()"
        )
    manifest_id = getattr(policy, "behavior_manifest_id", None)
    if not isinstance(manifest_id, BehaviorManifestId):
        raise HeldOutEvaluationError(
            f"paired evaluation {side} policy must expose one typed "
            "behavior manifest identity"
        )
    return manifest_id


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
