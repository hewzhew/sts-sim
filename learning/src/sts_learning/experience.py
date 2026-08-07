"""Bounded semantic experience segments with exact attempt lineage."""

from __future__ import annotations

import operator
import sys
from collections.abc import Iterable, Iterator, Mapping, Sequence
from dataclasses import dataclass
from enum import Enum
from types import MappingProxyType

import numpy as np

from .policy import BehaviorManifestId, SelectionProbability
from .recovery import (
    RecoverySlotSnapshot,
    RecoverySlotStatus,
    TerminalAttemptRecord,
)
from .semantic_batch import SemanticBatchError, select_semantic_decision_rows


class ExperienceError(ValueError):
    """An experience batch or segment transition is invalid."""


class SegmentCloseReason(Enum):
    EXPLICIT_FLUSH = "explicit_flush"
    DECISION_LIMIT = "decision_limit"
    PAYLOAD_BYTE_LIMIT = "payload_byte_limit"
    DECISION_AND_PAYLOAD_LIMIT = "decision_and_payload_limit"


@dataclass(frozen=True, order=True)
class AttemptKey:
    slot_index: int
    episode_seed: int
    episode_generation: int
    attempt_index: int


@dataclass(frozen=True)
class DecisionLineage:
    key: AttemptKey
    recoveries_used: int

    @classmethod
    def from_snapshot(cls, snapshot: RecoverySlotSnapshot) -> DecisionLineage:
        if not isinstance(snapshot, RecoverySlotSnapshot):
            raise ExperienceError("lineage must come from a RecoverySlotSnapshot")
        if snapshot.status is not RecoverySlotStatus.ACTIVE:
            raise ExperienceError("decision lineage requires an active slot snapshot")
        if snapshot.pending_terminal is not None:
            raise ExperienceError("active decision lineage cannot have a pending terminal")
        slot_index = _nonnegative_integer(snapshot.slot_index, "slot_index")
        episode_seed = _bounded_seed(snapshot.episode_seed)
        episode_generation = _nonnegative_integer(
            snapshot.episode_generation,
            "episode_generation",
        )
        attempt_index = _positive_integer(snapshot.attempt_index, "attempt_index")
        recoveries_used = _nonnegative_integer(
            snapshot.recoveries_used,
            "recoveries_used",
        )
        if attempt_index != recoveries_used + 1:
            raise ExperienceError("attempt index must equal recoveries used plus one")
        return cls(
            key=AttemptKey(
                slot_index=slot_index,
                episode_seed=episode_seed,
                episode_generation=episode_generation,
                attempt_index=attempt_index,
            ),
            recoveries_used=recoveries_used,
        )


@dataclass(frozen=True)
class ExperienceLimits:
    max_decisions: int
    max_payload_bytes: int

    def __post_init__(self) -> None:
        object.__setattr__(
            self,
            "max_decisions",
            _positive_integer(self.max_decisions, "max_decisions"),
        )
        object.__setattr__(
            self,
            "max_payload_bytes",
            _positive_integer(self.max_payload_bytes, "max_payload_bytes"),
        )


@dataclass(frozen=True)
class PreparedDecisionBatch:
    """Policy-independent frozen copy prepared before model inference."""

    payload: Mapping[str, object]
    lineages: tuple[DecisionLineage, ...]
    candidate_counts: tuple[int, ...]
    decision_count: int
    payload_bytes: int

    @classmethod
    def capture(
        cls,
        decision_batch: Mapping[str, object],
        snapshots: Sequence[RecoverySlotSnapshot],
    ) -> PreparedDecisionBatch:
        if not isinstance(decision_batch, Mapping):
            raise ExperienceError("decision batch must be a mapping")
        slots = _integer_column(decision_batch, "slot_indices")
        counts = _integer_column(decision_batch, "candidate_counts")
        if not slots:
            raise ExperienceError("experience batch must contain a decision row")
        if len(slots) != len(counts):
            raise ExperienceError("decision slot and candidate columns are misaligned")
        if len(set(slots)) != len(slots):
            raise ExperienceError("decision batch contains duplicate slots")
        if any(count <= 0 for count in counts):
            raise ExperienceError("every decision row must have a legal candidate")
        normalized_snapshots = tuple(snapshots)
        if len(normalized_snapshots) != len(slots):
            raise ExperienceError(
                f"received {len(normalized_snapshots)} snapshots for {len(slots)} rows"
            )
        lineages = tuple(
            DecisionLineage.from_snapshot(snapshot)
            for snapshot in normalized_snapshots
        )
        for slot, lineage in zip(slots, lineages, strict=True):
            if lineage.key.slot_index != slot:
                raise ExperienceError(
                    f"decision slot {slot} is aligned to snapshot slot "
                    f"{lineage.key.slot_index}"
                )
        payload, payload_bytes = _freeze_payload(decision_batch, "decision_batch")
        if not isinstance(payload, Mapping):
            raise ExperienceError("frozen decision batch is not a mapping")
        return cls(
            payload=payload,
            lineages=lineages,
            candidate_counts=counts,
            decision_count=len(slots),
            payload_bytes=payload_bytes,
        )


@dataclass(frozen=True)
class DecisionExperienceBatch:
    payload: Mapping[str, object]
    lineages: tuple[DecisionLineage, ...]
    selected_ordinals: tuple[int, ...]
    selection_probabilities: tuple[SelectionProbability, ...]
    behavior_manifest_id: BehaviorManifestId
    decision_count: int
    payload_bytes: int

    @classmethod
    def from_prepared(
        cls,
        prepared: PreparedDecisionBatch,
        selected_ordinals: Sequence[int],
        selection_probabilities: Sequence[SelectionProbability],
        behavior_manifest_id: BehaviorManifestId,
    ) -> DecisionExperienceBatch:
        if not isinstance(prepared, PreparedDecisionBatch):
            raise ExperienceError("experience input must be a PreparedDecisionBatch")
        ordinals = _integer_sequence(selected_ordinals, "selected ordinals")
        try:
            probabilities = tuple(selection_probabilities)
        except TypeError as error:
            raise ExperienceError(
                "selection probabilities must be a sequence"
            ) from error
        if not isinstance(behavior_manifest_id, BehaviorManifestId):
            raise ExperienceError(
                "decision experience requires a BehaviorManifestId"
            )
        if len(ordinals) != prepared.decision_count:
            raise ExperienceError(
                f"received {len(ordinals)} ordinals for "
                f"{prepared.decision_count} decision rows"
            )
        if len(probabilities) != prepared.decision_count:
            raise ExperienceError(
                "selection probabilities must contain one value per decision row"
            )
        if not all(
            isinstance(probability, SelectionProbability)
            for probability in probabilities
        ):
            raise ExperienceError(
                "selection probabilities must be typed SelectionProbability values"
            )
        for row, (ordinal, count) in enumerate(
            zip(ordinals, prepared.candidate_counts, strict=True)
        ):
            if not 0 <= ordinal < count:
                raise ExperienceError(
                    f"row {row} candidate ordinal {ordinal} is outside 0..{count}"
                )
        return cls(
            payload=prepared.payload,
            lineages=prepared.lineages,
            selected_ordinals=ordinals,
            selection_probabilities=probabilities,
            behavior_manifest_id=behavior_manifest_id,
            decision_count=prepared.decision_count,
            payload_bytes=prepared.payload_bytes,
        )

    def select_rows(self, row_indices: Sequence[int]) -> DecisionExperienceBatch:
        """Own and freeze an exact row subset for attempt-local retention."""

        rows = _integer_sequence(row_indices, "row indices")
        try:
            selected = select_semantic_decision_rows(self.payload, rows)
        except SemanticBatchError as error:
            raise ExperienceError("cannot select semantic decision rows") from error
        payload, payload_bytes = _freeze_payload(selected, "selected_decision_batch")
        if not isinstance(payload, Mapping):
            raise ExperienceError("selected decision payload is not a mapping")
        return DecisionExperienceBatch(
            payload=payload,
            lineages=tuple(self.lineages[row] for row in rows),
            selected_ordinals=tuple(self.selected_ordinals[row] for row in rows),
            selection_probabilities=tuple(
                self.selection_probabilities[row] for row in rows
            ),
            behavior_manifest_id=self.behavior_manifest_id,
            decision_count=len(rows),
            payload_bytes=payload_bytes,
        )


@dataclass(frozen=True)
class AttemptFragment:
    lineage: DecisionLineage
    terminal: TerminalAttemptRecord | None

    @property
    def censored(self) -> bool:
        return self.terminal is None


@dataclass(frozen=True)
class ExperienceSegment:
    sequence_index: int
    close_reason: SegmentCloseReason
    batches: tuple[DecisionExperienceBatch, ...]
    attempts: tuple[AttemptFragment, ...]
    decision_count: int
    payload_bytes: int

    @property
    def censored(self) -> bool:
        return any(attempt.censored for attempt in self.attempts)


class ExperienceSegmentBuffer:
    """One bounded mutable segment; sealed segments are caller-consumed."""

    def __init__(self, limits: ExperienceLimits) -> None:
        if not isinstance(limits, ExperienceLimits):
            raise ExperienceError("limits must be ExperienceLimits")
        self.limits = limits
        self._next_sequence_index = 0
        self._batches: list[DecisionExperienceBatch] = []
        self._decision_count = 0
        self._payload_bytes = 0
        self._lineages: dict[AttemptKey, DecisionLineage] = {}
        self._terminals: dict[AttemptKey, TerminalAttemptRecord] = {}

    @property
    def decision_count(self) -> int:
        return self._decision_count

    @property
    def payload_bytes(self) -> int:
        return self._payload_bytes

    @property
    def empty(self) -> bool:
        return not self._batches

    def prepare(
        self,
        decision_batch: Mapping[str, object],
        snapshots: Sequence[RecoverySlotSnapshot],
    ) -> PreparedDecisionBatch:
        prepared = PreparedDecisionBatch.capture(decision_batch, snapshots)
        if prepared.decision_count > self.limits.max_decisions:
            raise ExperienceError(
                f"one batch has {prepared.decision_count} decisions, exceeding "
                f"segment limit {self.limits.max_decisions}"
            )
        if prepared.payload_bytes > self.limits.max_payload_bytes:
            raise ExperienceError(
                f"one batch has {prepared.payload_bytes} payload bytes, exceeding "
                f"segment limit {self.limits.max_payload_bytes}"
            )
        return prepared

    def record(
        self,
        prepared: PreparedDecisionBatch,
        selected_ordinals: Sequence[int],
        selection_probabilities: Sequence[SelectionProbability],
        behavior_manifest_id: BehaviorManifestId,
    ) -> tuple[ExperienceSegment, ...]:
        batch = DecisionExperienceBatch.from_prepared(
            prepared,
            selected_ordinals,
            selection_probabilities,
            behavior_manifest_id,
        )
        emitted = self.rotate_before(batch)
        self.commit(batch)
        return emitted

    def rotate_before(
        self,
        batch: DecisionExperienceBatch,
    ) -> tuple[ExperienceSegment, ...]:
        """Seal the old segment, if needed, without admitting this batch."""

        self._validate_batch(batch)
        decision_overflow, payload_overflow = self._would_overflow(batch)
        if self._batches and (decision_overflow or payload_overflow):
            return (
                self._seal(_limit_reason(decision_overflow, payload_overflow)),
            )
        return ()

    def commit(self, batch: DecisionExperienceBatch) -> None:
        """Admit one already-applied choice after any required rotation."""

        self._validate_batch(batch)
        decision_overflow, payload_overflow = self._would_overflow(batch)
        if decision_overflow or payload_overflow:
            raise ExperienceError("experience batch requires rotation before commit")
        self._append(batch)

    def _validate_batch(self, batch: DecisionExperienceBatch) -> None:
        if not isinstance(batch, DecisionExperienceBatch):
            raise ExperienceError("experience input must be DecisionExperienceBatch")
        if not isinstance(batch.behavior_manifest_id, BehaviorManifestId):
            raise ExperienceError("experience batch has no behavior manifest identity")
        if batch.decision_count != len(batch.lineages):
            raise ExperienceError("experience batch lineage rows are misaligned")
        if batch.decision_count != len(batch.selected_ordinals):
            raise ExperienceError("experience batch ordinal rows are misaligned")
        if batch.decision_count != len(batch.selection_probabilities):
            raise ExperienceError(
                "experience batch selection probability rows are misaligned"
            )
        if not all(
            isinstance(probability, SelectionProbability)
            for probability in batch.selection_probabilities
        ):
            raise ExperienceError(
                "experience batch selection probabilities must be typed"
            )
        if batch.decision_count > self.limits.max_decisions:
            raise ExperienceError("prepared batch exceeds the decision limit")
        if batch.payload_bytes > self.limits.max_payload_bytes:
            raise ExperienceError("prepared batch exceeds the payload byte limit")

    def _would_overflow(
        self,
        batch: DecisionExperienceBatch,
    ) -> tuple[bool, bool]:
        return (
            self._decision_count + batch.decision_count
            > self.limits.max_decisions,
            self._payload_bytes + batch.payload_bytes
            > self.limits.max_payload_bytes,
        )

    def record_terminals(
        self,
        attempts: Sequence[TerminalAttemptRecord],
    ) -> None:
        records = tuple(attempts)
        if not all(isinstance(record, TerminalAttemptRecord) for record in records):
            raise ExperienceError(
                "terminal experience must contain TerminalAttemptRecord values"
            )
        prepared: list[tuple[AttemptKey, TerminalAttemptRecord]] = []
        seen = set()
        for record in records:
            key = _terminal_key(record)
            if key in seen:
                raise ExperienceError("terminal batch contains duplicate attempt lineage")
            seen.add(key)
            lineage = self._lineages.get(key)
            if lineage is None:
                raise ExperienceError("terminal attempt is absent from the open segment")
            if lineage.recoveries_used != record.recoveries_used:
                raise ExperienceError("terminal recovery count does not match its decisions")
            if key in self._terminals:
                raise ExperienceError("terminal attempt was already recorded")
            prepared.append((key, record))
        self._terminals.update(prepared)

    def flush(self) -> ExperienceSegment | None:
        if not self._batches:
            return None
        return self._seal(SegmentCloseReason.EXPLICIT_FLUSH)

    def _append(self, batch: DecisionExperienceBatch) -> None:
        for lineage in batch.lineages:
            previous = self._lineages.get(lineage.key)
            if previous is not None and previous != lineage:
                raise ExperienceError("attempt lineage changed within one segment")
        self._batches.append(batch)
        self._decision_count += batch.decision_count
        self._payload_bytes += batch.payload_bytes
        for lineage in batch.lineages:
            self._lineages.setdefault(lineage.key, lineage)
        if self._decision_count > self.limits.max_decisions:
            raise AssertionError("experience decision bound was exceeded")
        if self._payload_bytes > self.limits.max_payload_bytes:
            raise AssertionError("experience payload byte bound was exceeded")

    def _seal(self, reason: SegmentCloseReason) -> ExperienceSegment:
        if not self._batches:
            raise ExperienceError("cannot seal an empty experience segment")
        attempts = tuple(
            AttemptFragment(
                lineage=lineage,
                terminal=self._terminals.get(key),
            )
            for key, lineage in self._lineages.items()
        )
        segment = ExperienceSegment(
            sequence_index=self._next_sequence_index,
            close_reason=reason,
            batches=tuple(self._batches),
            attempts=attempts,
            decision_count=self._decision_count,
            payload_bytes=self._payload_bytes,
        )
        self._next_sequence_index += 1
        self._batches = []
        self._decision_count = 0
        self._payload_bytes = 0
        self._lineages = {}
        self._terminals = {}
        return segment


def iter_payload_arrays(value: object) -> Iterator[np.ndarray]:
    """Yield every NumPy buffer retained by a frozen experience payload."""

    if isinstance(value, np.ndarray):
        yield value
    elif isinstance(value, Mapping):
        for child in value.values():
            yield from iter_payload_arrays(child)


def _freeze_payload(value: object, path: str) -> tuple[object, int]:
    if isinstance(value, np.ndarray):
        if value.dtype.hasobject:
            raise ExperienceError(f"{path} contains an object array")
        copied = np.array(value, copy=True, order="C", subok=False)
        copied.setflags(write=False)
        return copied, sys.getsizeof(copied)
    if isinstance(value, Mapping):
        frozen: dict[str, object] = {}
        payload_bytes = 0
        for key, child in value.items():
            if not isinstance(key, str):
                raise ExperienceError(f"{path} contains a non-string mapping key")
            frozen_child, child_bytes = _freeze_payload(child, f"{path}.{key}")
            frozen[key] = frozen_child
            payload_bytes += sys.getsizeof(key) + child_bytes
        proxy = MappingProxyType(frozen)
        payload_bytes += sys.getsizeof(frozen) + sys.getsizeof(proxy)
        return proxy, payload_bytes
    if isinstance(value, bool):
        return value, sys.getsizeof(value)
    try:
        normalized = operator.index(value)
        return normalized, sys.getsizeof(normalized)
    except TypeError as error:
        raise ExperienceError(
            f"{path} contains unsupported value {type(value).__name__}"
        ) from error


def _integer_column(mapping: Mapping[str, object], name: str) -> tuple[int, ...]:
    try:
        raw = mapping[name]
    except KeyError as error:
        raise ExperienceError(f"decision batch is missing {name}") from error
    return _integer_sequence(raw, name)


def _integer_sequence(raw: object, name: str) -> tuple[int, ...]:
    if isinstance(raw, (str, bytes)) or not isinstance(raw, Iterable):
        raise ExperienceError(f"{name} must be an iterable of integers")
    normalized = []
    for value in raw:
        if isinstance(value, bool):
            raise ExperienceError(f"{name} must not contain bool")
        try:
            normalized.append(operator.index(value))
        except TypeError as error:
            raise ExperienceError(f"{name} must contain only integers") from error
    return tuple(normalized)


def _terminal_key(record: TerminalAttemptRecord) -> AttemptKey:
    return AttemptKey(
        slot_index=_nonnegative_integer(record.slot_index, "terminal slot_index"),
        episode_seed=_bounded_seed(record.episode_seed),
        episode_generation=_nonnegative_integer(
            record.episode_generation,
            "terminal episode_generation",
        ),
        attempt_index=_positive_integer(
            record.attempt_index,
            "terminal attempt_index",
        ),
    )


def _limit_reason(
    decision_overflow: bool,
    payload_overflow: bool,
) -> SegmentCloseReason:
    if decision_overflow and payload_overflow:
        return SegmentCloseReason.DECISION_AND_PAYLOAD_LIMIT
    if decision_overflow:
        return SegmentCloseReason.DECISION_LIMIT
    if payload_overflow:
        return SegmentCloseReason.PAYLOAD_BYTE_LIMIT
    raise AssertionError("segment limit reason requires an exceeded bound")


def _positive_integer(value: int, name: str) -> int:
    if isinstance(value, bool):
        raise ExperienceError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise ExperienceError(f"{name} must be an integer") from error
    if normalized <= 0:
        raise ExperienceError(f"{name} must be positive")
    return normalized


def _nonnegative_integer(value: int, name: str) -> int:
    if isinstance(value, bool):
        raise ExperienceError(f"{name} must be an integer, not bool")
    try:
        normalized = operator.index(value)
    except TypeError as error:
        raise ExperienceError(f"{name} must be an integer") from error
    if normalized < 0:
        raise ExperienceError(f"{name} must be non-negative")
    return normalized


def _bounded_seed(value: int) -> int:
    seed = _nonnegative_integer(value, "episode_seed")
    if seed >= 1 << 64:
        raise ExperienceError("episode_seed must be in 0..2^64-1")
    return seed
