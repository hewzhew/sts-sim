"""Strictly bounded on-policy attempt batches for one optimizer update."""

from __future__ import annotations

import operator
from dataclasses import dataclass

from .attempts import (
    AttemptAssemblyDelivery,
    CompletedAttemptExperience,
    CompletedAttemptSink,
    DroppedAttemptExperience,
)
from .policy import BehaviorManifestId


class AttemptUpdateBatchError(RuntimeError):
    """A complete-attempt stream cannot form one exact on-policy update."""


@dataclass(frozen=True)
class AttemptUpdateBatchLimits:
    """Exact update size plus hard retained decision and payload bounds."""

    attempts_per_update: int
    max_decisions_per_update: int
    max_payload_bytes_per_update: int

    def __post_init__(self) -> None:
        for name in (
            "attempts_per_update",
            "max_decisions_per_update",
            "max_payload_bytes_per_update",
        ):
            object.__setattr__(self, name, _positive_integer(getattr(self, name), name))


@dataclass(frozen=True)
class AttemptUpdateBatchSnapshot:
    """Compact counters; pending tensor payloads are deliberately absent."""

    deliveries: int
    sink_deliveries: int
    update_batches: int
    completed_attempts: int
    dropped_attempts: int
    pending_attempts: int
    pending_decisions: int
    pending_payload_bytes: int
    pending_behavior_manifest_id: BehaviorManifestId | None
    poisoned: bool


class BoundedAttemptUpdateBatcher:
    """Collect one exact same-behavior attempt batch before training once."""

    def __init__(
        self,
        limits: AttemptUpdateBatchLimits,
        sink: CompletedAttemptSink,
        *,
        resume_snapshot: AttemptUpdateBatchSnapshot | None = None,
    ) -> None:
        if not isinstance(limits, AttemptUpdateBatchLimits):
            raise AttemptUpdateBatchError("limits must be AttemptUpdateBatchLimits")
        if not callable(sink):
            raise AttemptUpdateBatchError("attempt update sink must be callable")
        restored = _validated_resume_snapshot(resume_snapshot)
        self.limits = limits
        self._sink = sink
        self._deliveries = restored.deliveries
        self._sink_deliveries = restored.sink_deliveries
        self._update_batches = restored.update_batches
        self._completed_attempts = restored.completed_attempts
        self._dropped_attempts = restored.dropped_attempts
        self._pending: tuple[CompletedAttemptExperience, ...] = ()
        self._pending_decisions = 0
        self._pending_payload_bytes = 0
        self._pending_behavior_manifest_id: BehaviorManifestId | None = None
        self._poisoned = False

    @property
    def update_sink(self) -> CompletedAttemptSink:
        """Return the exact synchronous owner wired after update batching."""

        return self._sink

    @property
    def snapshot(self) -> AttemptUpdateBatchSnapshot:
        return AttemptUpdateBatchSnapshot(
            deliveries=self._deliveries,
            sink_deliveries=self._sink_deliveries,
            update_batches=self._update_batches,
            completed_attempts=self._completed_attempts,
            dropped_attempts=self._dropped_attempts,
            pending_attempts=len(self._pending),
            pending_decisions=self._pending_decisions,
            pending_payload_bytes=self._pending_payload_bytes,
            pending_behavior_manifest_id=self._pending_behavior_manifest_id,
            poisoned=self._poisoned,
        )

    def __call__(self, delivery: AttemptAssemblyDelivery) -> None:
        if self._poisoned:
            raise AttemptUpdateBatchError("attempt update batcher is poisoned")
        completed, dropped = _validated_delivery(delivery)
        if not completed and not dropped:
            raise AttemptUpdateBatchError("attempt update delivery is empty")

        pending_keys = {attempt.lineage.key for attempt in self._pending}
        incoming_keys = [attempt.lineage.key for attempt in completed]
        if len(set(incoming_keys)) != len(incoming_keys):
            raise AttemptUpdateBatchError("attempt update delivery repeats a lineage")
        if pending_keys.intersection(incoming_keys):
            raise AttemptUpdateBatchError("attempt update batch repeats a pending lineage")
        dropped_keys = [attempt.lineage.key for attempt in dropped]
        if len(set(dropped_keys)) != len(dropped_keys):
            raise AttemptUpdateBatchError("attempt update delivery repeats a dropped lineage")
        if set(incoming_keys).intersection(dropped_keys):
            raise AttemptUpdateBatchError("one lineage is both completed and dropped")

        incoming_manifest_id = _single_behavior_manifest_id(completed)
        pending_count = len(self._pending)
        next_count = pending_count + len(completed)
        target = self.limits.attempts_per_update
        if next_count > target:
            raise AttemptUpdateBatchError(
                "attempt update delivery exceeds the exact attempts_per_update target"
            )
        if (
            incoming_manifest_id is not None
            and self._pending_behavior_manifest_id is not None
            and incoming_manifest_id != self._pending_behavior_manifest_id
        ):
            raise AttemptUpdateBatchError(
                "attempt update batch mixes behavior manifest identities"
            )

        incoming_decisions = sum(attempt.decision_count for attempt in completed)
        incoming_payload_bytes = sum(attempt.payload_bytes for attempt in completed)
        next_decisions = self._pending_decisions + incoming_decisions
        next_payload_bytes = self._pending_payload_bytes + incoming_payload_bytes
        if next_decisions > self.limits.max_decisions_per_update:
            raise AttemptUpdateBatchError(
                "attempt update batch exceeds max_decisions_per_update"
            )
        if next_payload_bytes > self.limits.max_payload_bytes_per_update:
            raise AttemptUpdateBatchError(
                "attempt update batch exceeds max_payload_bytes_per_update"
            )
        next_pending = self._pending + completed

        ready = next_count == target
        sink_delivery = None
        if ready:
            sink_delivery = AttemptAssemblyDelivery(
                completed=next_pending,
                dropped=dropped,
            )
        elif dropped:
            sink_delivery = AttemptAssemblyDelivery(completed=(), dropped=dropped)

        if sink_delivery is not None:
            try:
                self._sink(sink_delivery)
            except Exception:
                self._pending = ()
                self._pending_decisions = 0
                self._pending_payload_bytes = 0
                self._pending_behavior_manifest_id = None
                self._poisoned = True
                raise

        self._deliveries += 1
        self._completed_attempts += len(completed)
        self._dropped_attempts += len(dropped)
        if sink_delivery is not None:
            self._sink_deliveries += 1
        if ready:
            self._update_batches += 1
            self._pending = ()
            self._pending_decisions = 0
            self._pending_payload_bytes = 0
            self._pending_behavior_manifest_id = None
        else:
            self._pending = next_pending
            self._pending_decisions = next_decisions
            self._pending_payload_bytes = next_payload_bytes
            if incoming_manifest_id is not None:
                self._pending_behavior_manifest_id = incoming_manifest_id


def _validated_delivery(
    delivery: AttemptAssemblyDelivery,
) -> tuple[
    tuple[CompletedAttemptExperience, ...],
    tuple[DroppedAttemptExperience, ...],
]:
    if not isinstance(delivery, AttemptAssemblyDelivery):
        raise AttemptUpdateBatchError(
            "attempt update batcher requires AttemptAssemblyDelivery input"
        )
    if not all(
        isinstance(attempt, CompletedAttemptExperience)
        for attempt in delivery.completed
    ):
        raise AttemptUpdateBatchError("completed attempt delivery rows are malformed")
    if not all(
        isinstance(attempt, DroppedAttemptExperience) for attempt in delivery.dropped
    ):
        raise AttemptUpdateBatchError("dropped attempt delivery rows are malformed")
    for attempt in delivery.completed:
        if not attempt.batches:
            raise AttemptUpdateBatchError("completed attempt has no policy decisions")
        if attempt.decision_count != sum(batch.decision_count for batch in attempt.batches):
            raise AttemptUpdateBatchError("completed attempt decision count is inconsistent")
        if attempt.payload_bytes != sum(batch.payload_bytes for batch in attempt.batches):
            raise AttemptUpdateBatchError("completed attempt payload bytes are inconsistent")
    return delivery.completed, delivery.dropped


def _single_behavior_manifest_id(
    attempts: tuple[CompletedAttemptExperience, ...],
) -> BehaviorManifestId | None:
    manifest_ids = {
        batch.behavior_manifest_id
        for attempt in attempts
        for batch in attempt.batches
    }
    if len(manifest_ids) > 1:
        raise AttemptUpdateBatchError(
            "attempt update delivery mixes behavior manifest identities"
        )
    return next(iter(manifest_ids), None)


def _validated_resume_snapshot(
    snapshot: AttemptUpdateBatchSnapshot | None,
) -> AttemptUpdateBatchSnapshot:
    if snapshot is None:
        return AttemptUpdateBatchSnapshot(
            deliveries=0,
            sink_deliveries=0,
            update_batches=0,
            completed_attempts=0,
            dropped_attempts=0,
            pending_attempts=0,
            pending_decisions=0,
            pending_payload_bytes=0,
            pending_behavior_manifest_id=None,
            poisoned=False,
        )
    if not isinstance(snapshot, AttemptUpdateBatchSnapshot):
        raise AttemptUpdateBatchError("attempt update resume snapshot must be typed")
    if snapshot.poisoned:
        raise AttemptUpdateBatchError("cannot resume a poisoned attempt update batcher")
    for name in (
        "deliveries",
        "sink_deliveries",
        "update_batches",
        "completed_attempts",
        "dropped_attempts",
        "pending_attempts",
        "pending_decisions",
        "pending_payload_bytes",
    ):
        _nonnegative_integer(getattr(snapshot, name), f"resume {name}")
    if (
        snapshot.pending_attempts != 0
        or snapshot.pending_decisions != 0
        or snapshot.pending_payload_bytes != 0
        or snapshot.pending_behavior_manifest_id is not None
    ):
        raise AttemptUpdateBatchError(
            "resume snapshot contains pending attempt update payload"
        )
    if snapshot.sink_deliveries > snapshot.deliveries:
        raise AttemptUpdateBatchError("resume sink deliveries exceed input deliveries")
    if snapshot.update_batches > snapshot.sink_deliveries:
        raise AttemptUpdateBatchError("resume update batches exceed sink deliveries")
    return snapshot


def _positive_integer(value: object, name: str) -> int:
    normalized = _nonnegative_integer(value, name)
    if normalized == 0:
        raise AttemptUpdateBatchError(f"{name} must be positive")
    return normalized


def _nonnegative_integer(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise AttemptUpdateBatchError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise AttemptUpdateBatchError(f"{name} must be an integer") from error
    if normalized < 0:
        raise AttemptUpdateBatchError(f"{name} must be non-negative")
    return normalized
