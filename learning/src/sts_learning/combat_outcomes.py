"""Typed same-root combat outcomes and independent grouped advantage axes."""

from __future__ import annotations

import math
import operator
from collections.abc import Iterable, Mapping
from dataclasses import dataclass


class CombatOutcomeError(ValueError):
    """A combat terminal batch or same-root group is malformed."""


@dataclass(frozen=True)
class CombatTerminalOutcome:
    """Typed terminal facts for one numbered replicate from one combat root."""

    replicate_index: int
    terminal_kind: int
    won: bool
    start_hp: int
    final_hp: int
    hp_loss: int
    turns: int
    potions_used: int
    potions_discarded: int
    cards_played: int

    def __post_init__(self) -> None:
        for name in (
            "replicate_index",
            "terminal_kind",
            "start_hp",
            "final_hp",
            "hp_loss",
            "turns",
            "potions_used",
            "potions_discarded",
            "cards_played",
        ):
            object.__setattr__(self, name, _integer(getattr(self, name), name))
        if self.replicate_index < 0:
            raise CombatOutcomeError("replicate_index must be non-negative")
        if self.terminal_kind < 0:
            raise CombatOutcomeError("terminal_kind must be non-negative")
        if not isinstance(self.won, bool):
            raise CombatOutcomeError("won must be a boolean bridge fact")
        if self.start_hp <= 0:
            raise CombatOutcomeError("start_hp must be positive")
        if self.final_hp < 0:
            raise CombatOutcomeError("final_hp must be non-negative")
        if self.hp_loss != max(self.start_hp - self.final_hp, 0):
            raise CombatOutcomeError("hp_loss disagrees with start_hp and final_hp")
        for name in (
            "turns",
            "potions_used",
            "potions_discarded",
            "cards_played",
        ):
            if getattr(self, name) < 0:
                raise CombatOutcomeError(f"{name} must be non-negative")


@dataclass(frozen=True)
class CombatTerminalStepBatch:
    """Newly terminal combat replicates copied from one bridge transition."""

    root_id: str
    exact_combat_state_hash: str
    outcomes: tuple[CombatTerminalOutcome, ...]

    def __post_init__(self) -> None:
        validate_combat_digest(self.root_id, "root_id")
        validate_combat_digest(
            self.exact_combat_state_hash,
            "exact_combat_state_hash",
        )
        outcomes = tuple(self.outcomes)
        if not all(isinstance(row, CombatTerminalOutcome) for row in outcomes):
            raise CombatOutcomeError(
                "combat terminal batch requires typed outcome rows"
            )
        indices = tuple(row.replicate_index for row in outcomes)
        if len(set(indices)) != len(indices):
            raise CombatOutcomeError("combat terminal batch repeats a replicate")
        object.__setattr__(self, "outcomes", outcomes)

    @classmethod
    def from_bridge_step(
        cls,
        step: Mapping[str, object],
        *,
        replicate_count: int,
    ) -> CombatTerminalStepBatch:
        count = _positive_integer(replicate_count, "replicate_count")
        root_id = _string_field(step, "root_id")
        exact_combat_state_hash = _string_field(step, "exact_combat_state_hash")
        columns: dict[str, tuple[object, ...]] = {
            name: _integer_column(step, name)
            for name in (
                "terminal_slot_indices",
                "terminal_kind",
                "terminal_start_hp",
                "terminal_final_hp",
                "terminal_hp_loss",
                "terminal_turns",
                "terminal_potions_used",
                "terminal_potions_discarded",
                "terminal_cards_played",
            )
        }
        columns["terminal_won"] = _boolean_column(step, "terminal_won")
        row_count = len(columns["terminal_slot_indices"])
        for name, values in columns.items():
            if len(values) != row_count:
                raise CombatOutcomeError(
                    f"combat terminal column {name} has {len(values)} rows, "
                    f"expected {row_count}"
                )
        if row_count > count:
            raise CombatOutcomeError("terminal rows exceed the combat replicate count")
        outcomes = tuple(
            CombatTerminalOutcome(
                replicate_index=columns["terminal_slot_indices"][row],
                terminal_kind=columns["terminal_kind"][row],
                won=columns["terminal_won"][row],
                start_hp=columns["terminal_start_hp"][row],
                final_hp=columns["terminal_final_hp"][row],
                hp_loss=columns["terminal_hp_loss"][row],
                turns=columns["terminal_turns"][row],
                potions_used=columns["terminal_potions_used"][row],
                potions_discarded=columns["terminal_potions_discarded"][row],
                cards_played=columns["terminal_cards_played"][row],
            )
            for row in range(row_count)
        )
        if any(row.replicate_index >= count for row in outcomes):
            raise CombatOutcomeError("terminal replicate is outside the combat group")
        return cls(root_id, exact_combat_state_hash, outcomes)


@dataclass(frozen=True)
class CombatGroupedAdvantages:
    """Same-root leave-one-out advantages kept separate by semantic axis."""

    win: tuple[float, ...]
    terminal_hp: tuple[float, ...]
    potion_retention: tuple[float, ...]

    def __post_init__(self) -> None:
        lengths = {len(self.win), len(self.terminal_hp), len(self.potion_retention)}
        if len(lengths) != 1 or next(iter(lengths), 0) < 2:
            raise CombatOutcomeError(
                "grouped advantage axes require the same two-or-more replicates"
            )
        for axis in (self.win, self.terminal_hp, self.potion_retention):
            if not all(math.isfinite(value) for value in axis):
                raise CombatOutcomeError("grouped advantages must be finite")

    @property
    def win_has_signal(self) -> bool:
        return any(value != 0.0 for value in self.win)

    @property
    def terminal_hp_has_signal(self) -> bool:
        return any(value != 0.0 for value in self.terminal_hp)

    @property
    def potion_retention_has_signal(self) -> bool:
        return any(value != 0.0 for value in self.potion_retention)


@dataclass(frozen=True)
class CompletedCombatGroup:
    """Exactly one terminal row for every replicate from one exact root."""

    root_id: str
    exact_combat_state_hash: str
    outcomes: tuple[CombatTerminalOutcome, ...]

    def __post_init__(self) -> None:
        validate_combat_digest(self.root_id, "root_id")
        validate_combat_digest(
            self.exact_combat_state_hash,
            "exact_combat_state_hash",
        )
        outcomes = tuple(self.outcomes)
        if len(outcomes) < 2:
            raise CombatOutcomeError("completed combat group requires two replicates")
        if not all(isinstance(row, CombatTerminalOutcome) for row in outcomes):
            raise CombatOutcomeError("completed combat group requires typed outcomes")
        indices = tuple(row.replicate_index for row in outcomes)
        if indices != tuple(range(len(outcomes))):
            raise CombatOutcomeError(
                "completed combat outcomes must be ordered contiguous replicates"
            )
        if len({row.start_hp for row in outcomes}) != 1:
            raise CombatOutcomeError("same-root outcomes disagree on start_hp")
        object.__setattr__(self, "outcomes", outcomes)

    def grouped_advantages(self) -> CombatGroupedAdvantages:
        start_hp = self.outcomes[0].start_hp
        return CombatGroupedAdvantages(
            win=_leave_one_out(tuple(1.0 if row.won else 0.0 for row in self.outcomes)),
            terminal_hp=_leave_one_out(
                tuple(row.final_hp / start_hp for row in self.outcomes)
            ),
            potion_retention=_leave_one_out(
                tuple(
                    -float(row.potions_used + row.potions_discarded)
                    for row in self.outcomes
                )
            ),
        )


class CombatGroupOutcomeAccumulator:
    """Bounded terminal-row owner for one fixed exact combat group."""

    def __init__(
        self,
        *,
        root_id: str,
        exact_combat_state_hash: str,
        replicate_count: int,
    ) -> None:
        validate_combat_digest(root_id, "root_id")
        validate_combat_digest(
            exact_combat_state_hash,
            "exact_combat_state_hash",
        )
        self.root_id = root_id
        self.exact_combat_state_hash = exact_combat_state_hash
        self.replicate_count = _positive_integer(replicate_count, "replicate_count")
        if self.replicate_count < 2:
            raise CombatOutcomeError("combat group requires at least two replicates")
        self._outcomes: dict[int, CombatTerminalOutcome] = {}

    @property
    def terminal_count(self) -> int:
        return len(self._outcomes)

    def record(self, batch: CombatTerminalStepBatch) -> None:
        if not isinstance(batch, CombatTerminalStepBatch):
            raise CombatOutcomeError("combat accumulator requires a typed step batch")
        if (
            batch.root_id != self.root_id
            or batch.exact_combat_state_hash != self.exact_combat_state_hash
        ):
            raise CombatOutcomeError("combat terminal batch belongs to a different root")
        additions: dict[int, CombatTerminalOutcome] = {}
        for outcome in batch.outcomes:
            index = outcome.replicate_index
            if index >= self.replicate_count:
                raise CombatOutcomeError("terminal replicate is outside the combat group")
            if index in self._outcomes or index in additions:
                raise CombatOutcomeError("combat replicate terminated more than once")
            additions[index] = outcome
        self._outcomes.update(additions)

    def finish(self) -> CompletedCombatGroup:
        if len(self._outcomes) != self.replicate_count:
            raise CombatOutcomeError(
                f"combat group has {len(self._outcomes)} terminal rows, "
                f"expected {self.replicate_count}"
            )
        return CompletedCombatGroup(
            root_id=self.root_id,
            exact_combat_state_hash=self.exact_combat_state_hash,
            outcomes=tuple(self._outcomes[index] for index in range(self.replicate_count)),
        )


def _leave_one_out(values: tuple[float, ...]) -> tuple[float, ...]:
    if len(values) < 2:
        raise CombatOutcomeError("leave-one-out requires at least two values")
    total = sum(values)
    denominator = len(values) - 1
    return tuple(value - ((total - value) / denominator) for value in values)


def _integer_column(step: Mapping[str, object], name: str) -> tuple[int, ...]:
    try:
        raw = step[name]
    except KeyError as error:
        raise CombatOutcomeError(f"bridge combat step is missing {name}") from error
    if isinstance(raw, (str, bytes)) or not isinstance(raw, Iterable):
        raise CombatOutcomeError(f"combat terminal column {name} is not iterable")
    try:
        return tuple(operator.index(value) for value in raw)
    except TypeError as error:
        raise CombatOutcomeError(f"combat terminal column {name} is not integral") from error


def _boolean_column(step: Mapping[str, object], name: str) -> tuple[bool, ...]:
    try:
        raw = step[name]
    except KeyError as error:
        raise CombatOutcomeError(f"bridge combat step is missing {name}") from error
    if isinstance(raw, (str, bytes)) or not isinstance(raw, Iterable):
        raise CombatOutcomeError(f"combat terminal column {name} is not iterable")
    return tuple(_boolean(value, name) for value in raw)


def _string_field(step: Mapping[str, object], name: str) -> str:
    try:
        value = step[name]
    except KeyError as error:
        raise CombatOutcomeError(f"bridge combat step is missing {name}") from error
    validate_combat_digest(value, name)
    return value


def _integer(value: object, name: str) -> int:
    if isinstance(value, bool):
        raise CombatOutcomeError(f"{name} must be an integer, not bool")
    try:
        return operator.index(value)
    except TypeError as error:
        raise CombatOutcomeError(f"{name} must be an integer") from error


def _boolean(value: object, name: str) -> bool:
    if isinstance(value, bool):
        return value
    if type(value).__module__ == "numpy" and type(value).__name__ == "bool":
        return bool(value)
    raise CombatOutcomeError(f"{name} must be boolean")


def _positive_integer(value: object, name: str) -> int:
    normalized = _integer(value, name)
    if normalized <= 0:
        raise CombatOutcomeError(f"{name} must be positive")
    return normalized


def validate_combat_digest(value: object, name: str) -> None:
    if not isinstance(value, str) or len(value) != 64:
        raise CombatOutcomeError(f"{name} must be a 64-character lowercase hex digest")
    if any(character not in "0123456789abcdef" for character in value):
        raise CombatOutcomeError(f"{name} must be a 64-character lowercase hex digest")
