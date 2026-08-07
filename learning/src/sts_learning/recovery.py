"""Caller-owned recovery accounting for exact learning checkpoints.

This module does not own environment mechanics, checkpoints, or a rule for
when recovery is desirable. It validates and records explicit caller actions.
"""

from __future__ import annotations

import operator
from dataclasses import dataclass, field
from enum import Enum
from typing import Protocol, Sequence

from .outcomes import TerminalAttemptOutcome, TerminalStepBatch


class RecoveryProtocolError(ValueError):
    """The requested accounting transition is not legal."""


class RecoveryMode(Enum):
    TRAINING = "training"
    HELD_OUT_ZERO_RECOVERY = "held_out_zero_recovery"


class RecoverySlotStatus(Enum):
    ACTIVE = "active"
    DEFEAT_PENDING = "defeat_pending"
    VICTORY_COMPLETE = "victory_complete"
    DEFEAT_COMPLETE = "defeat_complete"


@dataclass(frozen=True)
class RecoverySlotSnapshot:
    slot_index: int
    episode_generation: int
    attempt_index: int
    recoveries_used: int
    status: RecoverySlotStatus
    pending_terminal: TerminalAttemptOutcome | None


@dataclass(frozen=True)
class RecoveryTicket:
    slot_indices: tuple[int, ...]
    episode_generations: tuple[int, ...]
    recoveries_before: tuple[int, ...]
    _owner: object = field(repr=False, compare=False)


@dataclass(frozen=True)
class EpisodeResetTicket:
    slot_indices: tuple[int, ...]
    episode_generations: tuple[int, ...]
    _owner: object = field(repr=False, compare=False)


@dataclass(frozen=True)
class RecoveryEvent:
    slot_index: int
    episode_generation: int
    attempt_index: int
    recoveries_used: int


@dataclass(frozen=True)
class EpisodeOutcome:
    episode_generation: int
    attempts: int
    recoveries_used: int
    terminal: TerminalAttemptOutcome

    @property
    def slot_index(self) -> int:
        return self.terminal.slot_index

    @property
    def terminal_reward(self) -> int:
        return self.terminal.terminal_reward

    @property
    def zero_recovery(self) -> bool:
        return self.recoveries_used == 0


class RecoveryRestoreTarget(Protocol):
    def restore_slots(self, slot_indices: list[int], checkpoints: object) -> None: ...


class EpisodeResetTarget(Protocol):
    def reset_slots(self, slot_indices: list[int], seeds: list[int]) -> None: ...


class RecoveryLedger:
    """Bounded current-episode state for a fixed vector environment."""

    def __init__(
        self,
        slot_count: int,
        *,
        mode: RecoveryMode,
        max_recoveries_per_episode: int,
    ) -> None:
        if slot_count < 0:
            raise RecoveryProtocolError("slot_count must be non-negative")
        if max_recoveries_per_episode < 0:
            raise RecoveryProtocolError(
                "max_recoveries_per_episode must be non-negative"
            )
        if (
            mode is RecoveryMode.HELD_OUT_ZERO_RECOVERY
            and max_recoveries_per_episode != 0
        ):
            raise RecoveryProtocolError("held-out mode requires a zero recovery budget")
        self.mode = mode
        self.max_recoveries_per_episode = max_recoveries_per_episode
        self._ticket_owner = object()
        self._episode_generation = [0] * slot_count
        self._attempt_index = [1] * slot_count
        self._recoveries_used = [0] * slot_count
        self._status = [RecoverySlotStatus.ACTIVE] * slot_count
        self._pending_terminal: list[TerminalAttemptOutcome | None] = [None] * slot_count

    @classmethod
    def training(
        cls, slot_count: int, *, max_recoveries_per_episode: int
    ) -> RecoveryLedger:
        return cls(
            slot_count,
            mode=RecoveryMode.TRAINING,
            max_recoveries_per_episode=max_recoveries_per_episode,
        )

    @classmethod
    def held_out(cls, slot_count: int) -> RecoveryLedger:
        return cls(
            slot_count,
            mode=RecoveryMode.HELD_OUT_ZERO_RECOVERY,
            max_recoveries_per_episode=0,
        )

    @property
    def slot_count(self) -> int:
        return len(self._status)

    def snapshot(self, slot_index: int) -> RecoverySlotSnapshot:
        slot = self._validate_slots([slot_index])[0]
        return RecoverySlotSnapshot(
            slot_index=slot,
            episode_generation=self._episode_generation[slot],
            attempt_index=self._attempt_index[slot],
            recoveries_used=self._recoveries_used[slot],
            status=self._status[slot],
            pending_terminal=self._pending_terminal[slot],
        )

    def snapshots(self) -> tuple[RecoverySlotSnapshot, ...]:
        return tuple(self.snapshot(slot) for slot in range(self.slot_count))

    def record_terminal(self, batch: TerminalStepBatch) -> tuple[EpisodeOutcome, ...]:
        if not isinstance(batch, TerminalStepBatch):
            raise RecoveryProtocolError("terminal input must be a TerminalStepBatch")
        slots = self._validate_slots(batch.slot_indices)
        for slot in slots:
            if self._status[slot] is not RecoverySlotStatus.ACTIVE:
                raise RecoveryProtocolError(f"slot {slot} is not active")
            if self._pending_terminal[slot] is not None:
                raise RecoveryProtocolError(f"slot {slot} already has a pending terminal")

        outcomes = []
        for terminal in batch.attempts:
            slot = terminal.slot_index
            if terminal.terminal_reward == 1:
                self._status[slot] = RecoverySlotStatus.VICTORY_COMPLETE
                outcomes.append(self._outcome(slot, terminal))
            else:
                self._status[slot] = RecoverySlotStatus.DEFEAT_PENDING
                self._pending_terminal[slot] = terminal
        return tuple(outcomes)

    def prepare_recovery(self, slot_indices: Sequence[int]) -> RecoveryTicket:
        slots = self._validate_slots(slot_indices)
        if self.mode is RecoveryMode.HELD_OUT_ZERO_RECOVERY:
            raise RecoveryProtocolError("held-out evaluation forbids recovery")
        for slot in slots:
            if self._status[slot] is not RecoverySlotStatus.DEFEAT_PENDING:
                raise RecoveryProtocolError(f"slot {slot} has no pending defeat")
            if self._pending_terminal[slot] is None:
                raise RecoveryProtocolError(f"slot {slot} is missing its pending terminal")
            if self._recoveries_used[slot] >= self.max_recoveries_per_episode:
                raise RecoveryProtocolError(f"slot {slot} exhausted its recovery budget")
        return RecoveryTicket(
            slot_indices=slots,
            episode_generations=tuple(self._episode_generation[slot] for slot in slots),
            recoveries_before=tuple(self._recoveries_used[slot] for slot in slots),
            _owner=self._ticket_owner,
        )

    def commit_recovery(self, ticket: RecoveryTicket) -> tuple[RecoveryEvent, ...]:
        if ticket._owner is not self._ticket_owner:
            raise RecoveryProtocolError("recovery ticket belongs to another ledger")
        slots = self._validate_slots(ticket.slot_indices)
        if not (
            len(slots)
            == len(ticket.episode_generations)
            == len(ticket.recoveries_before)
        ):
            raise RecoveryProtocolError("recovery ticket columns are misaligned")
        for slot, generation, recoveries_before in zip(
            slots,
            ticket.episode_generations,
            ticket.recoveries_before,
            strict=True,
        ):
            if self._status[slot] is not RecoverySlotStatus.DEFEAT_PENDING:
                raise RecoveryProtocolError(f"slot {slot} no longer has a pending defeat")
            if self._episode_generation[slot] != generation:
                raise RecoveryProtocolError(f"slot {slot} episode generation changed")
            if self._recoveries_used[slot] != recoveries_before:
                raise RecoveryProtocolError(f"slot {slot} recovery count changed")
            if recoveries_before >= self.max_recoveries_per_episode:
                raise RecoveryProtocolError(f"slot {slot} exhausted its recovery budget")

        events = []
        for slot in slots:
            self._recoveries_used[slot] += 1
            self._attempt_index[slot] += 1
            self._status[slot] = RecoverySlotStatus.ACTIVE
            self._pending_terminal[slot] = None
            events.append(
                RecoveryEvent(
                    slot_index=slot,
                    episode_generation=self._episode_generation[slot],
                    attempt_index=self._attempt_index[slot],
                    recoveries_used=self._recoveries_used[slot],
                )
            )
        return tuple(events)

    def complete_defeats(
        self, slot_indices: Sequence[int]
    ) -> tuple[EpisodeOutcome, ...]:
        slots = self._validate_slots(slot_indices)
        terminals = []
        for slot in slots:
            if self._status[slot] is not RecoverySlotStatus.DEFEAT_PENDING:
                raise RecoveryProtocolError(f"slot {slot} has no pending defeat")
            terminal = self._pending_terminal[slot]
            if terminal is None:
                raise RecoveryProtocolError(f"slot {slot} is missing its pending terminal")
            terminals.append(terminal)
        outcomes = tuple(
            self._outcome(slot, terminal)
            for slot, terminal in zip(slots, terminals, strict=True)
        )
        for slot in slots:
            self._status[slot] = RecoverySlotStatus.DEFEAT_COMPLETE
            self._pending_terminal[slot] = None
        return outcomes

    def prepare_reset(self, slot_indices: Sequence[int]) -> EpisodeResetTicket:
        slots = self._validate_slots(slot_indices)
        completed = {
            RecoverySlotStatus.VICTORY_COMPLETE,
            RecoverySlotStatus.DEFEAT_COMPLETE,
        }
        for slot in slots:
            if self._status[slot] not in completed:
                raise RecoveryProtocolError(f"slot {slot} episode is not complete")
        return EpisodeResetTicket(
            slot_indices=slots,
            episode_generations=tuple(self._episode_generation[slot] for slot in slots),
            _owner=self._ticket_owner,
        )

    def commit_reset(self, ticket: EpisodeResetTicket) -> None:
        if ticket._owner is not self._ticket_owner:
            raise RecoveryProtocolError("reset ticket belongs to another ledger")
        slots = self._validate_slots(ticket.slot_indices)
        if len(slots) != len(ticket.episode_generations):
            raise RecoveryProtocolError("reset ticket columns are misaligned")
        completed = {
            RecoverySlotStatus.VICTORY_COMPLETE,
            RecoverySlotStatus.DEFEAT_COMPLETE,
        }
        for slot, generation in zip(
            slots, ticket.episode_generations, strict=True
        ):
            if self._status[slot] not in completed:
                raise RecoveryProtocolError(f"slot {slot} episode is not complete")
            if self._episode_generation[slot] != generation:
                raise RecoveryProtocolError(f"slot {slot} episode generation changed")
        for slot in slots:
            self._episode_generation[slot] += 1
            self._attempt_index[slot] = 1
            self._recoveries_used[slot] = 0
            self._status[slot] = RecoverySlotStatus.ACTIVE
            self._pending_terminal[slot] = None

    def _validate_slots(self, slot_indices: Sequence[int]) -> tuple[int, ...]:
        slots = tuple(operator.index(slot) for slot in slot_indices)
        if len(set(slots)) != len(slots):
            raise RecoveryProtocolError("slot batch contains duplicate indices")
        for slot in slots:
            if not 0 <= slot < self.slot_count:
                raise RecoveryProtocolError(
                    f"slot {slot} is outside 0..{self.slot_count}"
                )
        return slots

    def _outcome(
        self, slot: int, terminal: TerminalAttemptOutcome
    ) -> EpisodeOutcome:
        if terminal.slot_index != slot:
            raise RecoveryProtocolError("terminal outcome belongs to another slot")
        return EpisodeOutcome(
            episode_generation=self._episode_generation[slot],
            attempts=self._attempt_index[slot],
            recoveries_used=self._recoveries_used[slot],
            terminal=terminal,
        )


def restore_with_accounting(
    env: RecoveryRestoreTarget,
    slot_indices: Sequence[int],
    checkpoints: object,
    ledger: RecoveryLedger,
) -> tuple[RecoveryEvent, ...]:
    """Restore a prepared batch, committing accounting only after success."""

    ticket = ledger.prepare_recovery(slot_indices)
    env.restore_slots(list(ticket.slot_indices), checkpoints)
    return ledger.commit_recovery(ticket)


def reset_with_accounting(
    env: EpisodeResetTarget,
    slot_indices: Sequence[int],
    seeds: Sequence[int],
    ledger: RecoveryLedger,
) -> None:
    """Start new episodes, committing ledger generations only after success."""

    ticket = ledger.prepare_reset(slot_indices)
    reset_seeds = [operator.index(seed) for seed in seeds]
    if len(ticket.slot_indices) != len(reset_seeds):
        raise RecoveryProtocolError(
            f"expected {len(ticket.slot_indices)} reset seeds, received {len(reset_seeds)}"
        )
    env.reset_slots(list(ticket.slot_indices), reset_seeds)
    ledger.commit_reset(ticket)
