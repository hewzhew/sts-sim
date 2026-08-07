"""Bounded online batch execution without trajectory retention."""

from __future__ import annotations

import operator
import time
from collections.abc import Callable, Iterable, Mapping, Sequence
from dataclasses import dataclass
from typing import Protocol

from .experience import (
    DecisionExperienceBatch,
    ExperienceSegment,
    ExperienceSegmentBuffer,
    PreparedDecisionBatch,
)
from .outcomes import TerminalStepBatch
from .policy import BatchPolicyChoice, SelectionProbability
from .recovery import (
    EpisodeOutcome,
    RecoveryEvent,
    RecoveryLedger,
    RecoveryMode,
    RecoverySlotSnapshot,
    TerminalAccountingBatch,
    TerminalAttemptRecord,
    restore_with_accounting,
)
from .seeds import (
    SeedPartition,
    SeedSchedule,
    reset_scheduled_checkpointed_with_accounting,
)


class BatchDriverError(ValueError):
    """A policy, curriculum, or environment batch violated the driver contract."""


class CheckpointBatch(Protocol):
    """Opaque checkpoint collection; contents remain owned by the bridge."""

    def __len__(self) -> int: ...

    def select(self, slot_indices: list[int]) -> CheckpointBatch: ...

    def updated(self, replacements: CheckpointBatch) -> CheckpointBatch: ...


class BatchEnvironment(Protocol):
    """Narrow structural type implemented by the standalone Rust bridge."""

    @property
    def slot_count(self) -> int: ...

    @property
    def terminal_count(self) -> int: ...

    @property
    def ready(self) -> bool: ...

    def decision_batch(self, *, semantic: bool = False) -> Mapping[str, object]: ...

    def choose(self, ordinals: list[int]) -> None: ...

    def step(self) -> Mapping[str, object]: ...

    def checkpoint_slots(self, slot_indices: list[int]) -> CheckpointBatch: ...

    def restore_slots(
        self,
        slot_indices: list[int],
        checkpoints: object,
    ) -> None: ...

    def reset_slots(self, slot_indices: list[int], seeds: list[int]) -> None: ...

    def reset_slots_checkpointed(
        self,
        slot_indices: list[int],
        seeds: list[int],
    ) -> CheckpointBatch: ...


class BatchPolicy(Protocol):
    """One inference call over every row in one ragged decision batch."""

    def choose(self, decision_batch: Mapping[str, object]) -> BatchPolicyChoice: ...


@dataclass(frozen=True)
class RecoveryPlan:
    """Defeated slots to restore; every other defeat completes."""

    slot_indices: tuple[int, ...] = ()

    def __post_init__(self) -> None:
        slots = _normalize_integer_sequence(self.slot_indices, "recovery slots")
        if len(set(slots)) != len(slots):
            raise BatchDriverError("recovery plan contains duplicate slots")
        if any(slot < 0 for slot in slots):
            raise BatchDriverError("recovery slots must be non-negative")
        object.__setattr__(self, "slot_indices", slots)


class BatchCurriculum(Protocol):
    """Caller-owned retry choice over one complete terminal batch."""

    def plan_recovery(
        self,
        accounting: TerminalAccountingBatch,
        snapshots: tuple[RecoverySlotSnapshot, ...],
    ) -> RecoveryPlan: ...


class ExperienceSegmentSink(Protocol):
    """Synchronous consumer; the driver never queues sealed segments."""

    def __call__(self, segment: ExperienceSegment) -> None: ...


@dataclass(frozen=True)
class InitialPopulation:
    """Seed-aligned environment, ledger, cursor, and episode-root checkpoints."""

    env: BatchEnvironment
    ledger: RecoveryLedger
    schedule: SeedSchedule
    checkpoint_bank: CheckpointBatch


@dataclass(frozen=True)
class BatchStepResult:
    """Bounded facts from one vector environment transition."""

    decision_rounds: int
    slot_steps: int
    attempts: tuple[TerminalAttemptRecord, ...]
    completed_episodes: tuple[EpisodeOutcome, ...]
    recoveries: tuple[RecoveryEvent, ...]
    emitted_experience_segments: int
    emitted_experience_decisions: int
    emitted_experience_payload_bytes: int


@dataclass(frozen=True)
class BatchRunSummary:
    """Compact aggregate from a bounded number of vector transitions."""

    mode: RecoveryMode
    active_slots: int
    batch_steps: int
    slot_steps: int
    decision_rounds: int
    terminal_attempts: int
    completed_episodes: int
    recoveries: int
    emitted_experience_segments: int
    emitted_experience_decisions: int
    emitted_experience_payload_bytes: int
    open_experience_decisions: int
    open_experience_payload_bytes: int
    elapsed_seconds: float

    @property
    def steps_per_second(self) -> float:
        if self.elapsed_seconds == 0.0:
            return 0.0
        return self.slot_steps / self.elapsed_seconds


@dataclass(frozen=True)
class TerminalTargetRunResult:
    """Exact bounded prefix stopped by a terminal target or a step limit."""

    summary: BatchRunSummary
    terminal_attempt_target: int
    batch_step_limit: int

    @property
    def target_reached(self) -> bool:
        return self.summary.terminal_attempts >= self.terminal_attempt_target

    @property
    def step_limit_reached(self) -> bool:
        return (
            not self.target_reached
            and self.summary.batch_steps == self.batch_step_limit
        )


def initialize_population(
    env_factory: Callable[[list[int]], BatchEnvironment],
    *,
    slot_count: int,
    schedule: SeedSchedule,
    max_recoveries_per_episode: int,
) -> InitialPopulation:
    """Create all initial slot owners from one immutable seed plan."""

    normalized_slot_count = _normalize_count(
        slot_count,
        "slot_count",
        allow_zero=False,
    )
    slots = tuple(range(normalized_slot_count))
    recovery_limit = _normalize_count(
        max_recoveries_per_episode,
        "max_recoveries_per_episode",
        allow_zero=True,
    )
    initial, next_schedule = schedule.plan(slots)
    env = env_factory(list(initial.seeds))
    actual_slot_count = operator.index(env.slot_count)
    if actual_slot_count != len(slots):
        raise BatchDriverError(
            f"environment created {actual_slot_count} slots, expected {len(slots)}"
        )
    if operator.index(env.terminal_count) != 0:
        raise BatchDriverError("initial environment contains terminal slots")

    if schedule.partition is SeedPartition.TRAINING:
        ledger = RecoveryLedger.training(
            initial.seeds,
            max_recoveries_per_episode=recovery_limit,
        )
    else:
        if recovery_limit != 0:
            raise BatchDriverError("held-out population requires zero recoveries")
        ledger = RecoveryLedger.held_out(initial.seeds)

    checkpoint_bank = env.checkpoint_slots(list(slots))
    if len(checkpoint_bank) != len(slots):
        raise BatchDriverError(
            f"checkpoint bank contains {len(checkpoint_bank)} slots, expected {len(slots)}"
        )
    return InitialPopulation(
        env=env,
        ledger=ledger,
        schedule=next_schedule,
        checkpoint_bank=checkpoint_bank,
    )


class OnlineBatchDriver:
    """Execute aligned policy batches and resolve terminal slot lifecycles."""

    def __init__(
        self,
        population: InitialPopulation,
        *,
        policy: BatchPolicy,
        curriculum: BatchCurriculum,
        max_decision_rounds_per_step: int = 256,
        experience_buffer: ExperienceSegmentBuffer | None = None,
        experience_sink: ExperienceSegmentSink | None = None,
    ) -> None:
        if (experience_buffer is None) != (experience_sink is None):
            raise BatchDriverError(
                "experience_buffer and experience_sink must be configured together"
            )
        self.env = population.env
        self.ledger = population.ledger
        self.schedule = population.schedule
        self._checkpoint_bank = population.checkpoint_bank
        self.policy = policy
        self.curriculum = curriculum
        self._experience_buffer = experience_buffer
        self._experience_sink = experience_sink
        self._experience_sink_failed = False
        self.max_decision_rounds_per_step = _normalize_count(
            max_decision_rounds_per_step,
            "max_decision_rounds_per_step",
            allow_zero=False,
        )

    @property
    def experience_sink(self) -> ExperienceSegmentSink | None:
        """Return the configured synchronous sink owner without exposing a queue."""

        return self._experience_sink

    def advance(self) -> BatchStepResult:
        """Resolve decisions, step once, and immediately refill completed slots."""

        if self._experience_sink_failed:
            raise BatchDriverError("experience sink previously failed")
        decision_rounds = 0
        experience_segments = 0
        experience_decisions = 0
        experience_payload_bytes = 0
        while not self.env.ready:
            if decision_rounds >= self.max_decision_rounds_per_step:
                raise BatchDriverError(
                    "policy did not finish symbolic selection within "
                    f"{self.max_decision_rounds_per_step} rounds"
                )
            decision_batch = self.env.decision_batch(semantic=True)
            slots = _mapping_integer_sequence(
                decision_batch,
                "slot_indices",
                "decision slot indices",
            )
            candidate_counts = _mapping_integer_sequence(
                decision_batch,
                "candidate_counts",
                "candidate counts",
            )
            if len(slots) != len(candidate_counts):
                raise BatchDriverError("decision slot and candidate columns are misaligned")
            prepared_experience: PreparedDecisionBatch | None = None
            if self._experience_buffer is not None:
                prepared_experience = self._experience_buffer.prepare(
                    decision_batch,
                    tuple(self.ledger.snapshot(slot) for slot in slots),
                )
            choice = self.policy.choose(decision_batch)
            if not isinstance(choice, BatchPolicyChoice):
                raise BatchDriverError("policy must return BatchPolicyChoice")
            ordinals = choice.ordinals
            if len(ordinals) != len(slots):
                raise BatchDriverError(
                    f"policy returned {len(ordinals)} ordinals for {len(slots)} rows"
                )
            probabilities = choice.selection_probabilities
            if len(probabilities) != len(slots):
                raise BatchDriverError(
                    "policy selection probabilities must contain one value per row"
                )
            if not all(
                isinstance(probability, SelectionProbability)
                for probability in probabilities
            ):
                raise BatchDriverError(
                    "policy selection probabilities must be typed"
                )
            for slot, ordinal, count in zip(
                slots,
                ordinals,
                candidate_counts,
                strict=True,
            ):
                if count <= 0:
                    raise BatchDriverError(f"slot {slot} has no legal candidates")
                if not 0 <= ordinal < count:
                    raise BatchDriverError(
                        f"slot {slot} candidate ordinal {ordinal} is outside 0..{count}"
                    )
            if prepared_experience is not None:
                experience_batch = DecisionExperienceBatch.from_prepared(
                    prepared_experience,
                    ordinals,
                    probabilities,
                    choice.behavior_manifest_id,
                )
                emitted = self._experience_buffer.rotate_before(experience_batch)
                segments, decisions, payload_bytes = self._consume_experience(emitted)
                experience_segments += segments
                experience_decisions += decisions
                experience_payload_bytes += payload_bytes
            self.env.choose(list(ordinals))
            if prepared_experience is not None:
                self._experience_buffer.commit(experience_batch)
            decision_rounds += 1

        bridge_step = self.env.step()
        slot_steps = len(
            _mapping_integer_sequence(
                bridge_step,
                "slot_indices",
                "step slot indices",
            )
        )
        terminal = TerminalStepBatch.from_bridge_step(
            bridge_step,
            slot_count=self.ledger.slot_count,
        )
        if not terminal.attempts:
            return BatchStepResult(
                decision_rounds=decision_rounds,
                slot_steps=slot_steps,
                attempts=(),
                completed_episodes=(),
                recoveries=(),
                emitted_experience_segments=experience_segments,
                emitted_experience_decisions=experience_decisions,
                emitted_experience_payload_bytes=experience_payload_bytes,
            )

        accounting = self.ledger.record_terminal(terminal)
        if self._experience_buffer is not None:
            self._experience_buffer.record_terminals(accounting.attempts)
        snapshots = tuple(
            self.ledger.snapshot(slot) for slot in terminal.slot_indices
        )
        plan = self.curriculum.plan_recovery(accounting, snapshots)
        if not isinstance(plan, RecoveryPlan):
            raise BatchDriverError("curriculum must return a RecoveryPlan")
        defeat_slots = tuple(
            attempt.slot_index
            for attempt in accounting.attempts
            if attempt.terminal_reward == -1
        )
        unexpected = set(plan.slot_indices).difference(defeat_slots)
        if unexpected:
            raise BatchDriverError(
                "recovery plan contains non-defeat slots: "
                + ", ".join(str(slot) for slot in sorted(unexpected))
            )

        recoveries: tuple[RecoveryEvent, ...] = ()
        if plan.slot_indices:
            checkpoints = self._checkpoint_bank.select(list(plan.slot_indices))
            recoveries = restore_with_accounting(
                self.env,
                plan.slot_indices,
                checkpoints,
                self.ledger,
            )

        recover_set = set(plan.slot_indices)
        completed_defeat_slots = tuple(
            slot for slot in defeat_slots if slot not in recover_set
        )
        completed_defeats = self.ledger.complete_defeats(completed_defeat_slots)
        completed_by_slot = {
            outcome.slot_index: outcome
            for outcome in accounting.completed_episodes + completed_defeats
        }
        completed = tuple(
            completed_by_slot[attempt.slot_index]
            for attempt in accounting.attempts
            if attempt.slot_index in completed_by_slot
        )
        if completed:
            reset_slots = tuple(outcome.slot_index for outcome in completed)
            (
                _,
                next_schedule,
                replacements,
            ) = reset_scheduled_checkpointed_with_accounting(
                self.env,
                reset_slots,
                self.ledger,
                self.schedule,
            )
            self._checkpoint_bank = self._checkpoint_bank.updated(replacements)
            self.schedule = next_schedule

        return BatchStepResult(
            decision_rounds=decision_rounds,
            slot_steps=slot_steps,
            attempts=accounting.attempts,
            completed_episodes=completed,
            recoveries=recoveries,
            emitted_experience_segments=experience_segments,
            emitted_experience_decisions=experience_decisions,
            emitted_experience_payload_bytes=experience_payload_bytes,
        )

    def flush_experience(self) -> ExperienceSegment | None:
        """Seal and synchronously consume the current bounded segment."""

        if self._experience_sink_failed:
            raise BatchDriverError("experience sink previously failed")
        if self._experience_buffer is None:
            return None
        segment = self._experience_buffer.flush()
        if segment is not None:
            self._consume_experience((segment,))
        return segment

    def run(self, *, batch_steps: int) -> BatchRunSummary:
        """Run a fixed number of vector transitions without retaining results."""

        requested_steps = _normalize_count(batch_steps, "batch_steps", allow_zero=True)
        return self._run_normalized(
            batch_step_limit=requested_steps,
            terminal_attempt_target=None,
        )

    def run_until_terminal_attempts(
        self,
        *,
        terminal_attempts: int,
        max_batch_steps: int,
    ) -> TerminalTargetRunResult:
        """Run an atomic step prefix until its terminal target or explicit limit."""

        target = _normalize_count(
            terminal_attempts,
            "terminal_attempts",
            allow_zero=True,
        )
        step_limit = _normalize_count(
            max_batch_steps,
            "max_batch_steps",
            allow_zero=True,
        )
        summary = self._run_normalized(
            batch_step_limit=step_limit,
            terminal_attempt_target=target,
        )
        return TerminalTargetRunResult(
            summary=summary,
            terminal_attempt_target=target,
            batch_step_limit=step_limit,
        )

    def _run_normalized(
        self,
        *,
        batch_step_limit: int,
        terminal_attempt_target: int | None,
    ) -> BatchRunSummary:
        slot_steps = 0
        decision_rounds = 0
        terminal_attempts = 0
        completed_episodes = 0
        recoveries = 0
        experience_segments = 0
        experience_decisions = 0
        experience_payload_bytes = 0
        batch_steps = 0
        started = time.perf_counter()
        for _ in range(batch_step_limit):
            if (
                terminal_attempt_target is not None
                and terminal_attempts >= terminal_attempt_target
            ):
                break
            result = self.advance()
            batch_steps += 1
            slot_steps += result.slot_steps
            decision_rounds += result.decision_rounds
            terminal_attempts += len(result.attempts)
            completed_episodes += len(result.completed_episodes)
            recoveries += len(result.recoveries)
            experience_segments += result.emitted_experience_segments
            experience_decisions += result.emitted_experience_decisions
            experience_payload_bytes += result.emitted_experience_payload_bytes
        elapsed = time.perf_counter() - started
        open_experience_decisions = (
            self._experience_buffer.decision_count
            if self._experience_buffer is not None
            else 0
        )
        open_experience_payload_bytes = (
            self._experience_buffer.payload_bytes
            if self._experience_buffer is not None
            else 0
        )
        return BatchRunSummary(
            mode=self.ledger.mode,
            active_slots=self.ledger.slot_count - operator.index(self.env.terminal_count),
            batch_steps=batch_steps,
            slot_steps=slot_steps,
            decision_rounds=decision_rounds,
            terminal_attempts=terminal_attempts,
            completed_episodes=completed_episodes,
            recoveries=recoveries,
            emitted_experience_segments=experience_segments,
            emitted_experience_decisions=experience_decisions,
            emitted_experience_payload_bytes=experience_payload_bytes,
            open_experience_decisions=open_experience_decisions,
            open_experience_payload_bytes=open_experience_payload_bytes,
            elapsed_seconds=elapsed,
        )

    def _consume_experience(
        self,
        segments: Sequence[ExperienceSegment],
    ) -> tuple[int, int, int]:
        if not segments:
            return 0, 0, 0
        sink = self._experience_sink
        if sink is None:
            raise AssertionError("experience segments exist without a sink")
        decisions = 0
        payload_bytes = 0
        try:
            for segment in segments:
                sink(segment)
                decisions += segment.decision_count
                payload_bytes += segment.payload_bytes
        except Exception:
            self._experience_sink_failed = True
            raise
        return len(segments), decisions, payload_bytes


def _normalize_count(value: int, name: str, *, allow_zero: bool) -> int:
    if isinstance(value, bool):
        raise BatchDriverError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise BatchDriverError(f"{name} must be an integer") from error
    lower = 0 if allow_zero else 1
    if normalized < lower:
        relation = "non-negative" if allow_zero else "positive"
        raise BatchDriverError(f"{name} must be {relation}")
    return normalized


def _mapping_integer_sequence(
    mapping: Mapping[str, object],
    key: str,
    name: str,
) -> tuple[int, ...]:
    try:
        raw = mapping[key]
    except KeyError as error:
        raise BatchDriverError(f"batch is missing {key}") from error
    return _normalize_integer_sequence(raw, name)


def _normalize_integer_sequence(raw: object, name: str) -> tuple[int, ...]:
    if isinstance(raw, (str, bytes)) or not isinstance(raw, Iterable):
        raise BatchDriverError(f"{name} must be an iterable of integers")
    normalized = []
    for value in raw:
        if isinstance(value, bool):
            raise BatchDriverError(f"{name} must not contain bool")
        try:
            normalized.append(operator.index(value))
        except TypeError as error:
            raise BatchDriverError(f"{name} must contain only integers") from error
    return tuple(normalized)
